use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::sync::LazyLock;
use derive_new::new;
use getset::Getters;
use tokio::sync::RwLock;
use crate::extensions::json_ext::JsonExt;
use crate::repos::neu::neu_repo::load_file;

pub static DONATIONS: LazyLock<RwLock<HashMap<String, Donation>>> = LazyLock::new(|| RwLock::new(HashMap::default()));
pub static UPGRADES: LazyLock<RwLock<Vec<Vec<String>>>> = LazyLock::new(|| RwLock::new(Vec::default()));
pub static SET_EXCEPTIONS: LazyLock<RwLock<HashMap<String, String>>> = LazyLock::new(|| RwLock::new(HashMap::default()));

#[derive(Clone, new, Getters, Debug)]
#[getset(get = "pub")]
pub struct Donation {
    id: String,
    xp: u64,
    set: Option<Vec<String>>,
    children: Option<String>
}

impl Donation {
    pub fn is_set(&self) -> bool { self.set.is_some() }
}

pub async fn load_museum_donations() -> Result<(), Box<dyn Error + Send + Sync>> {
    let json = load_file("constants/museum.json").await?;
    let items = json.as_object().ok_or("Json is not an object")?;

    let required_keys = ["weapons", "armor", "rarities", "children", "itemToXp", "sets_to_items", "set_exceptions"];

    for key in required_keys {
        if !items.contains_key(key) {
            return Err("Museum data is missing from repo!".into())
        }
    }

    let weapons = items.get("weapons").unwrap().as_array().unwrap();
    let armor = items.get("armor").unwrap().as_array().unwrap();
    let rarities = items.get("rarities").unwrap().as_array().unwrap();
    let children_list = items.get("children").unwrap();
    let item_to_xp = items.get("itemToXp").unwrap();
    let sets_to_items = items.get("sets_to_items").unwrap();
    let set_exceptions = items.get("set_exceptions").unwrap().as_object().unwrap();

    let mut donations = HashMap::new();

    for item_id in weapons.iter().chain(rarities) {
        let item_id = item_id.as_str().unwrap();
        let xp = item_to_xp.get_u64(item_id).unwrap_or_default();
        let children = children_list.get_str(item_id).map(String::from);
        let donation = Donation::new(item_id.to_string(), xp, None, children);
        donations.insert(item_id.to_string(), donation);
    }

    for set_id in armor {
        let set_id = set_id.as_str().unwrap();
        let xp = item_to_xp.get_u64(set_id).unwrap_or_default();
        let item_ids = sets_to_items.get_array(set_id)
            .map(|a| a.iter().filter_map(|i| i.as_str().map(String::from)).collect::<Vec<String>>());
        let children = children_list.get_str(set_id).map(String::from);
        let donation = Donation::new(set_id.to_string(), xp, item_ids, children);
        donations.insert(set_id.to_string(), donation);
    }

    build_upgrade_lines(&donations).await;

    let mut current_map = DONATIONS.write().await;
    *current_map = donations;

    // Store set_exceptions
    let mut set_exceptions_lock = SET_EXCEPTIONS.write().await;
    *set_exceptions_lock = set_exceptions.iter().filter_map(|(k, v)| v.as_str().map(|s| (s.to_string(), k.clone()))).collect();

    Ok(())
}

async fn build_upgrade_lines(donations: &HashMap<String, Donation>) {
    let mut children_set = HashSet::new();
    for donation in donations.values() {
        if let Some(child) = &donation.children {
            children_set.insert(child.clone());
        }
    }

    let mut upgrades = Vec::new();
    for (id, _) in donations {
        if !children_set.contains(id) {
            let mut chain = Vec::new();
            let mut current = id.clone();
            loop {
                chain.push(current.clone());
                match donations.get(&current) {
                    Some(donation) => match &donation.children {
                        Some(child) => current = child.clone(),
                        None => break
                    }
                    None => break
                }
            }
            if chain.len() > 1 {
                upgrades.push(chain);
            }
        }
    }

    let mut upgrades_lock = UPGRADES.write().await;
    *upgrades_lock = upgrades;
}
