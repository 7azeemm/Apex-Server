use crate::repos::neu::neu_repo;
use crate::repos::wiki::wiki_repo;
use crate::structs::repo_structs::Repo;
use git2::{Cred, FetchOptions, RemoteCallbacks, Repository};
use serde_json::Value;
use std::error::Error;
use std::path::Path;
use tokio::fs;

pub async fn schedule() {
    neu_repo::schedule().await;
    wiki_repo::schedule().await;
}

fn auth_callbacks() -> RemoteCallbacks<'static> {
    let token = std::env::var("GITHUB_TOKEN").expect("Github token is not set in .env file");

    let mut callbacks = RemoteCallbacks::new();
    if !token.is_empty() {
        callbacks.credentials(move |_url, username_from_url, _| {
            Cred::userpass_plaintext(username_from_url.unwrap_or("git"), &token)
        });
    }
    callbacks
}

pub async fn fetch_repo(repo: &Repo) -> bool {
    let path = Path::new(repo.path);
    let result = match path.exists() {
        true => update(repo.name, repo.branch, path),
        false => clone(repo.name, repo.url),
    };

    match result {
        Ok(updated) => updated,
        Err(err) => {
            eprintln!("[{}-Repo] Operation failed: {err}", repo.name);
            false
        }
    }
}

pub fn update(name: &str, branch: &str, path: &Path) -> Result<bool, git2::Error> {
    println!("[{name}-Repo] Fetching...");
    let repo = Repository::open(path)?;

    let mut fetch_opts = FetchOptions::new();
    fetch_opts.remote_callbacks(auth_callbacks());

    let mut remote = repo.find_remote("origin")?;
    remote.fetch(&[branch], Some(&mut fetch_opts), None)?;

    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;
    let (analysis, _) = repo.merge_analysis(&[&fetch_commit])?;

    if analysis.is_fast_forward() {
        let ref_name = format!("refs/heads/{branch}");
        let mut reference = repo.find_reference(&ref_name)?;
        reference.set_target(fetch_commit.id(), "Fast-forward")?;
        repo.set_head(&ref_name)?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
        println!("[{name}-Repo] Repository updated successfully.");
        Ok(true)
    } else {
        println!("[{name}-Repo] Repository is already up to date.");
        Ok(false)
    }
}

pub fn clone(name: &str, url: &str) -> Result<bool, git2::Error> {
    let mut fetch_opts = FetchOptions::new();
    fetch_opts.remote_callbacks(auth_callbacks());

    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fetch_opts);

    builder.clone(url, Path::new(&format!("{}_repo", name.to_lowercase())))?;
    println!("[{name}-Repo] Clone completed successfully.");
    Ok(true)
}

pub async fn load_repo_file(path: &str) -> Result<Value, Box<dyn Error + Send + Sync>> {
    let data = fs::read_to_string(path).await?;
    let value: Value = serde_json::from_str(&data)?;
    Ok(value)
}
