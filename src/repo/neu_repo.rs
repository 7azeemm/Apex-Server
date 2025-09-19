use crate::repo::essence_costs::load_essence_costs;
use crate::repo::gemstone_slots_cost::load_gemstone_slot_costs;
use crate::repo::items::{load_accessories, load_items};
use crate::repo::talisman_upgrades::load_talisman_upgrades;
use git2::Repository;
use serde_json::Value;
use std::error::Error;
use std::path::Path;
use std::time::Duration;
use tokio::fs;
use tokio::sync::Notify;
use tokio::time::{interval_at, Instant};

static DATA_WAITER: Notify = Notify::const_new();
const REPO_PATH: &str = "neu_repo";
const REPO_URL: &str = "https://github.com/NotEnoughUpdates/NotEnoughUpdates-REPO.git";
const THRESHOLD: u64 = 1800; // 30 min

pub async fn schedule() {
    tokio::spawn(async {
        let mut ticker = interval_at(Instant::now(), Duration::from_secs(THRESHOLD));
        let mut force_update = true;
        loop {
            ticker.tick().await;
            fetch_repo(force_update).await;
            DATA_WAITER.notify_waiters();
            force_update = false;
        }
    });
    DATA_WAITER.notified().await;
}

async fn fetch_repo(force_update: bool) {
    let path = Path::new(REPO_PATH);
    let result = match path.exists() {
        true => update_repo(path),
        false => clone_repo(path)
    };

    match result {
        Err(err) => eprintln!("[NEU-Repo] Operation failed: {err}"),
        Ok(updated) => {
            if updated || force_update {
                load_items(&format!("{REPO_PATH}/items")).await;
                load_accessories().await;
                load_gemstone_slot_costs().await;
                load_essence_costs().await;
                load_talisman_upgrades().await;
            }
        }
    }
}

fn update_repo(path: &Path) -> Result<bool, git2::Error> {
    println!("[NEU-Repo] Fetching...");
    let repo = Repository::open(path)?;

    {
        let mut remote = repo.find_remote("origin")?;
        remote.fetch(&["master"], None, None)?;
    }

    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;
    let (analysis, _) = repo.merge_analysis(&[&fetch_commit])?;

    if analysis.is_fast_forward() {
        let ref_name = "refs/heads/master";
        let mut reference = repo.find_reference(ref_name)?;
        reference.set_target(fetch_commit.id(), "Fast-forward")?;
        repo.set_head(ref_name)?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
        println!("[NEU-Repo] Repository updated successfully.");
    } else {
        println!("[NEU-Repo] Repository is already up to date.");
    }
    Ok(analysis.is_fast_forward())
}

fn clone_repo(path: &Path) -> Result<bool, git2::Error> {
    println!("[NEU-Repo] No repository found. Cloning...");
    Repository::clone(REPO_URL, path)?;
    println!("[NEU-Repo] Clone completed successfully.");
    Ok(true)
}

pub async fn load_repo_file(file_path: &str) -> Result<Value, Box<dyn Error + Send + Sync>> {
    let data = fs::read_to_string(&format!("{REPO_PATH}/{file_path}")).await?;
    let value: Value = serde_json::from_str(&data)?;
    Ok(value)
}