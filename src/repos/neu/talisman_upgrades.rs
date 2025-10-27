use crate::repos::neu::neu_repo::load_file;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;
use tokio::sync::RwLock;
use tokio::time::Instant;

static TALISMAN_UPGRADES: LazyLock<RwLock<Vec<Vec<String>>>> = LazyLock::new(|| RwLock::new(Vec::default()));
pub static IGNORED_TALISMANS: LazyLock<RwLock<HashSet<String>>> = LazyLock::new(|| RwLock::new(HashSet::default()));

pub async fn load_talisman_upgrades() {
    let start_time = Instant::now();

    match load_file("constants/misc.json").await {
        Ok(Value::Object(items)) => {
            if let Some(talisman_upgrades) = items.get("talisman_upgrades").and_then(|m| m.as_object()) {
                let talisman_upgrades: HashMap<String, Vec<String>> = talisman_upgrades.iter()
                    .map(|(k, v)| {
                        let upgrades = v.as_array()
                            .expect("expected array in value")
                            .iter()
                            .map(|val| val.as_str().expect("expected string").to_owned())
                            .collect::<Vec<_>>();
                        (k.clone(), upgrades)
                    })
                    .collect();

                let mut current_map = TALISMAN_UPGRADES.write().await;
                *current_map = build_upgrade_lines(&talisman_upgrades);
            }

            if let Some(ignored_talismans) = items.get("ignored_talisman").and_then(|m| m.as_array()) {
                let ignored_talismans: HashSet<String> = ignored_talismans
                    .iter()
                    .filter_map(|m| m.as_str())
                    .map(|s| s.to_owned())
                    .collect();
                let mut list = IGNORED_TALISMANS.write().await;
                *list = ignored_talismans;
            }

            println!("[NEU-Repo] Loaded talisman upgrades in {:.2?}", start_time.elapsed());
        }
        Err(err) => println!("[NEU-Repo] Error occurred while loading talisman upgrades: {:?}", err),
        _ => println!("[NEU-Repo] Error occurred while loading talisman upgrades: Invalid JSON format")
    }
}

fn build_upgrade_lines(map: &HashMap<String, Vec<String>>) -> Vec<Vec<String>> {
    let mut ordered_upgrades: Vec<Vec<String>> = Vec::new();

    for (item_id, upgrades) in map {
        if let Some(matching_upgrade) = ordered_upgrades.iter_mut().find(|ordered_upgrade| {
            ordered_upgrade.iter()
                .any(|x| x == item_id) || ordered_upgrade
                .iter()
                .any(|x| upgrades.iter().any(|u| u == x))
        }) {
            // Replace if new chain is longer (or equal length)
            if matching_upgrade.len() <= upgrades.len() {
                matching_upgrade.clear();
                matching_upgrade.push(item_id.to_string());
                matching_upgrade.extend_from_slice(upgrades);
            }
        } else {
            // Otherwise create new upgrade chain
            let mut new_upgrade = Vec::with_capacity(1 + upgrades.len());
            new_upgrade.push(item_id.to_string());
            new_upgrade.extend_from_slice(upgrades);
            ordered_upgrades.push(new_upgrade);
        }
    }

    // println!("{:#?}", ordered_upgrades);

    ordered_upgrades
}

pub async fn get_talisman_upgrades(item_id: &str) -> Option<Vec<String>> {
    for line in TALISMAN_UPGRADES.read().await.iter() {
        if line.iter().any(|t| t == item_id) {
            return Some(line.clone());
        }
    }

    None
}