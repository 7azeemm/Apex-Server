use crate::extensions::json_ext::JsonExt;
use crate::repos::wiki::wiki_searcher::update_index;
use crate::structs::repo_structs::Repo;
use crate::structs::wiki_structs::{Section, WikiPage};
use rustc_hash::FxHashMap;
use serde_json::Value;
use std::path::Path;
use std::sync::LazyLock;
use tokio::fs;
use tokio::sync::RwLock;
use tokio::time::Instant;

const REPO_PATH: &str = "wiki_repo";
const REPO_URL: &str = "https://github.com/7azeemm/SkyBlock-Wiki.git";

static WIKI_PAGES: LazyLock<RwLock<FxHashMap<String, WikiPage>>> = LazyLock::new(|| RwLock::new(FxHashMap::default()));

pub async fn schedule() {
    let repo = Repo {
        name: "Wiki",
        url: REPO_URL,
        branch: "main",
        path: REPO_PATH,
        threshold: 3600,
    };

    repo.schedule(|| async {
        process().await;
        update_index(get_pages().await);
    }).await;
}

async fn process() {
    let start = Instant::now();

    let mut dir_entries = match fs::read_dir(format!("{REPO_PATH}/pages")).await {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("[WIKI-Repo] Failed to read directory {}: {}", REPO_PATH, e);
            return;
        }
    };

    let mut wiki_pages = WIKI_PAGES.write().await;
    wiki_pages.clear();

    while let Ok(Some(entry)) = dir_entries.next_entry().await {
        let path = entry.path();
        if path.is_file() && path.extension().map(|e| e == "json").unwrap_or(false) {
            if let Some((title, page)) = process_file(&path).await {
                wiki_pages.insert(title, page);
            }
        }
    }

    println!("[Wiki-Repo] Processed all files in {:.2?}", start.elapsed());
}

async fn process_file(path: &Path) -> Option<(String, WikiPage)> {
    let content = match fs::read_to_string(path).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[Wiki-Repo] Failed to read file {}: {}", path.display(), e);
            return None;
        }
    };

    let json: Value = match serde_json::from_str(&content) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("[Wiki-Repo] Failed to parse JSON from {}: {}", path.display(), e);
            return None;
        }
    };

    let Some(title) = json.get_str("title") else { return None };
    let introduction = json.get("Introduction").map(json_to_str);

    let mut sections = Vec::new();
    if let Some(map) = json.as_object() {
        for (key, value) in map.iter() {
            if key == "title" || key == "Introduction" || key == "tags" { continue; }
            sections.push(Section::new(key.to_owned(), json_to_str(value)));
        }
    }

    let page = WikiPage::new(title.to_owned(), introduction, sections);
    Some((title.to_owned(), page))
}

fn json_to_str(value: &Value) -> String {
    fn inner(value: &Value, indent: usize) -> String {
        let pad = "  ".repeat(indent);

        match value {
            Value::String(s) => format!("{pad}{s}"),
            Value::Number(n) => format!("{pad}{n}"),
            Value::Bool(b) => format!("{pad}{b}"),
            Value::Null => format!("{pad}null"),

            Value::Array(arr) => arr
                .iter()
                .map(|v| format!("{pad}- {}", inner(v, indent + 1).trim_start()))
                .collect::<Vec<_>>()
                .join("\n"),

            Value::Object(map) => map
                .iter()
                .map(|(k, v)| {
                    let k = if k.ends_with(':') { k } else { &format!("{k}:") };
                    match v {
                        // Inline simple types
                        Value::String(s) => format!("{pad}{k} {s}"),
                        Value::Number(n) => format!("{pad}{k} {n}"),
                        Value::Bool(b) => format!("{pad}{k} {b}"),
                        Value::Null => format!("{pad}{k} null"),
                        // Nested types -> new line
                        _ => format!("{pad}{k}\n{}", inner(v, indent + 1)),
                    }
                })
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }

    inner(value, 0)
}

pub async fn get_pages() -> Vec<WikiPage> {
    WIKI_PAGES.read().await.values().cloned().collect::<Vec<WikiPage>>()
}

pub async fn get_pages_by_title(names: Vec<String>) -> Vec<Option<WikiPage>> {
    let pages_map = WIKI_PAGES.read().await;
    names.iter().map(|n| pages_map.get(n).cloned()).collect()
}