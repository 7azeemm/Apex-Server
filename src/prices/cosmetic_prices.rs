use crate::http::send_http_request;
use crate::item_utils::get_pet_level;
use crate::prices::auctions::get_lowest_bin;
use crate::structs::player_data_structs::Pet;
use rustc_hash::FxHashMap;
use std::collections::HashMap;
use std::error::Error;
use std::sync::LazyLock;
use std::time::Duration;
use tokio::sync::{Notify, RwLock};
use tokio::time::{interval_at, Instant};

static DATA_WAITER: Notify = Notify::const_new();
static DATA: LazyLock<RwLock<FxHashMap<String, u64>>> = LazyLock::new(|| RwLock::new(HashMap::default()));
const ENDPOINT: &str = "https://raw.githubusercontent.com/SkyHelperBot/Prices/main/pricesV2.json";
const THRESHOLD: u64 = 300;

pub async fn schedule() {
    tokio::spawn(async {
        let mut ticker = interval_at(Instant::now(), Duration::from_secs(THRESHOLD));
        loop {
            ticker.tick().await;
            match update().await {
                Ok(()) => println!("[Cosmetic-Prices] Next update in {} seconds", THRESHOLD),
                Err(err) => eprintln!("[Cosmetic-Prices] Error: {:?}", err)
            }
            DATA_WAITER.notify_waiters()
        }
    });
    DATA_WAITER.notified().await;
}

async fn update() -> Result<(), Box<dyn Error>> {
    let json = send_http_request(ENDPOINT).await?;

    if let Some(map) = json.as_object() {
        let mut data = DATA.write().await;
        data.clear();
        data.extend(map.iter().filter_map(|(k, v)| v.as_u64().map(|val| (k.clone(), val))));
    }

    Ok(())
}

pub async fn get_cosmetic_price(id: &str) -> Option<u64> {
    DATA.read().await.get(id).map(|p| *p)
}

pub async fn get_pet_networth(pet: &Pet) -> u64 {
    let (level, _) = get_pet_level(pet.name(), pet.tier(), *pet.xp() as u64);
    let level = match level {
        0..100 => 1,
        100..200 => 100,
        _ => level
    };
    let id = format!("LVL_{level}_{}_{}", pet.tier(), pet.name());
    let base_id = format!("{}_{}", pet.tier(), pet.name());

    if let Some(skin) = pet.skin() {
        let id_with_skin = format!("{id}_SKINNED_{skin}");
        if let Some(price) = get_cosmetic_price(&id_with_skin).await {
            return price;
        }

        let mut pet_value = 0;
        pet_value += match get_cosmetic_price(&id).await {
            None => get_lowest_bin(&base_id).await.unwrap_or(0),
            Some(price) => price
        };

        let skin_id = format!("PET_SKIN_{skin}");
        pet_value += match get_cosmetic_price(&skin_id).await {
            None => get_lowest_bin(&skin_id).await.unwrap_or(0),
            Some(price) => price
        };
        return pet_value;
    }

    if let Some(price) = get_cosmetic_price(&id).await {
        return price;
    }

    get_lowest_bin(&base_id).await.unwrap_or(0)
}