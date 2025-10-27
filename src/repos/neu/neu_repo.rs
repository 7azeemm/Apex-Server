use crate::repos::neu::essence_costs::load_essence_costs;
use crate::repos::neu::gemstone_slots_cost::load_gemstone_slot_costs;
use crate::repos::neu::items::{load_accessories, load_items};
use crate::repos::neu::talisman_upgrades::load_talisman_upgrades;
use crate::repos::repo_manager;
use crate::structs::repo_structs::Repo;
use serde_json::Value;
use std::error::Error;

const REPO_PATH: &str = "neu_repo";
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
        load_items(&format!("{REPO_PATH}/items")).await;
        load_accessories().await;
        load_gemstone_slot_costs().await;
        load_essence_costs().await;
        load_talisman_upgrades().await;
    }).await;
}

pub async fn load_file(file_path: &str) -> Result<Value, Box<dyn Error + Send + Sync>> {
    repo_manager::load_repo_file(&format!("{REPO_PATH}/{file_path}")).await
}