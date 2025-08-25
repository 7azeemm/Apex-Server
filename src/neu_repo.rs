use std::path::Path;
use git2::Repository;

const REPO_URL: &str = "https://github.com/NotEnoughUpdates/NotEnoughUpdates-REPO.git";
const PATH: &str = "neu_repo";

pub fn fetch_repo() {
    let path = Path::new(PATH);
    let result = if path.exists() {
        update_repo(path)
    } else {
        clone_repo(path)
    };

    if let Err(err) = result {
        eprintln!("[NEU Repo] Operation failed: {err}");
    }
}

fn update_repo(path: &Path) -> Result<(), git2::Error> {
    println!("[NEU Repo] Updating...");
    let repo = Repository::open(path)?;

    let mut remote = repo.find_remote("origin")?;
    remote.connect(git2::Direction::Fetch)?;
    remote.fetch(&["master"], None, None)?;

    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;
    let (analysis, _) = repo.merge_analysis(&[&fetch_commit])?;

    if analysis.is_fast_forward() {
        let ref_name = "refs/heads/master";
        let mut reference = repo.find_reference(ref_name)?;
        reference.set_target(fetch_commit.id(), "Fast-forward")?;
        repo.set_head(ref_name)?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
        println!("[NEU Repo] Repository updated successfully.");
    } else {
        println!("[NEU Repo] Repository is already up to date.");
    }
    Ok(())
}

fn clone_repo(path: &Path) -> Result<(), git2::Error> {
    println!("[NEU Repo] No repository found. Cloning...");
    Repository::clone(REPO_URL, path)?;
    println!("[NEU Repo] Clone completed successfully.");
    Ok(())
}