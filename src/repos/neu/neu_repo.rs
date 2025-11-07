use crate::repos::neu::essence_costs::load_essence_costs;
use crate::repos::neu::gemstone_slots_cost::load_gemstone_slot_costs;
use crate::repos::neu::items::{load_accessories, load_items};
use crate::repos::neu::museum_donations::load_museum_donations;
use crate::repos::neu::reforge_stones::load_reforge_stones;
use crate::repos::neu::talisman_upgrades::load_talisman_upgrades;
use crate::repos::repo_manager;
use crate::structs::repo_structs::Repo;
use serde_json::Value;
use std::error::Error;
use std::pin::Pin;
use std::time::Instant;

pub const REPO_PATH: &str = "neu_repo";
const REPO_URL: &str = "https://github.com/NotEnoughUpdates/NotEnoughUpdates-REPO.git";

pub async fn schedule() {
    let repo = Repo {
        name: "NEU",
        url: REPO_URL,
        branch: "master",
        path: REPO_PATH,
        threshold: 3600,
    };

    repo.schedule(|| async {
        let loaders: Vec<(&str, Pin<Box<dyn Future<Output = Result<(), Box<dyn Error + Send + Sync>>> + Send>>)> = vec![
            ("Items", Box::pin(load_items())),
            ("Accessories", Box::pin(load_accessories())),
            ("Gemstone-Slot-Costs", Box::pin(load_gemstone_slot_costs())),
            ("Essence-Costs", Box::pin(load_essence_costs())),
            ("Talisman-Upgrades", Box::pin(load_talisman_upgrades())),
            ("Reforge-Stones", Box::pin(load_reforge_stones())),
            ("Museum-Donations", Box::pin(load_museum_donations()))
        ];

        for (name, func) in loaders {
            let start_time = Instant::now();
            match func.await {
                Ok(()) => println!("[NEU-Repo/{name}] Loaded in {:.2?}", start_time.elapsed()),
                Err(err) => eprintln!("[NEU-Repo/{name}] Failed to load, err: {err}")
            }
        }
    }).await;
}

pub async fn load_file(file_path: &str) -> Result<Value, Box<dyn Error + Send + Sync>> {
    repo_manager::load_repo_file(&format!("{REPO_PATH}/{file_path}")).await
}