use std::collections::HashMap;
use crate::repos::wiki::wiki_repo::get_pages_by_title;
use std::sync::{Arc, LazyLock, RwLock};
use derive_new::new;
use getset::Getters;
use serde::Serialize;
use serde_json::json;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{doc, Index};
use tracing::error;
use crate::structs::player_data_structs::StringBuilder;

const TOP_N: usize = 5;
static WIKI_SEARCHER: LazyLock<RwLock<Arc<WikiSearcher>>> = LazyLock::new(|| RwLock::new(Arc::new(WikiSearcher::new(Vec::new()).unwrap())));

#[derive(Debug, Serialize, Clone, new, Getters)]
#[getset(get = "pub")]
pub struct WikiPage {
    title: String,
    introduction: String,
    sections: HashMap<String, String>,
}

pub fn update_index(pages: Vec<WikiPage>) {
    match WikiSearcher::new(pages) {
        Err(err) => error!(?err, "[WIKI-Repo] Failed to build wiki index"),
        Ok(searcher) => {
            if let Ok(mut writer) = WIKI_SEARCHER.write() {
                *writer = Arc::new(searcher);
            }
        }
    }
}

#[derive(Clone)]
pub struct WikiSearcher {
    index: Arc<Index>,
    title_field: Field,
    intro_field: Field,
    section_title_field: Field,
    section_content_field: Field,
}

impl WikiSearcher {
    pub fn new(pages: Vec<WikiPage>) -> tantivy::Result<Self> {
        let mut schema_builder = Schema::builder();
        let title_field = schema_builder.add_text_field("title", TEXT | STORED);
        let intro_field = schema_builder.add_text_field("introduction", TEXT | STORED);
        let section_title_field = schema_builder.add_text_field("section_title", TEXT | STORED);
        let section_content_field = schema_builder.add_text_field("section_content", TEXT | STORED);
        let schema = schema_builder.build();

        let index = Index::create_in_ram(schema.clone());
        let mut writer = index.writer(50_000_000)?;

        for page in pages {
            let mut document = doc!(
                title_field => *page.title(),
                intro_field => *page.introduction(),
            );

            for (title, content) in page.sections() {
                document.add_text(
                    section_title_field,
                    format!("{} {}", page.title(), title),
                );
                document.add_text(section_content_field, content);
            }

            writer.add_document(document)?;
        }

        writer.commit()?;

        Ok(Self {
            index: Arc::new(index),
            title_field,
            intro_field,
            section_title_field,
            section_content_field,
        })
    }

    fn create_parser(&self) -> QueryParser {
        let mut parser = QueryParser::for_index(
            &self.index,
            vec![
                self.title_field,
                self.intro_field,
                self.section_title_field,
                self.section_content_field,
            ],
        );
        parser.set_field_boost(self.title_field, 5.0);
        parser.set_field_boost(self.intro_field, 3.0);
        parser.set_field_boost(self.section_title_field, 2.0);
        parser.set_field_boost(self.section_content_field, 0.8);
        parser
    }

    pub fn search(&self, query: &str) -> tantivy::Result<Vec<(String, f32)>> {
        let reader = self.index.reader()?;
        let searcher = reader.searcher();
        let parser = self.create_parser();

        let query_parsed = parser.parse_query(query)?;
        let top_docs = searcher.search(&query_parsed, &TopDocs::with_limit(TOP_N))?;

        let results: Vec<(String, f32)> = top_docs
            .into_iter()
            .map(|(score, doc_address)| {
                let retrieved_doc = searcher
                    .doc::<TantivyDocument>(doc_address)
                    .unwrap_or_default();

                let title = retrieved_doc
                    .get_first(self.title_field)
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();

                (title, score)
            })
            .collect();

        Ok(results)
    }
}

pub async fn search_skyblock_wiki(sb: &mut StringBuilder, query: &str) {
    let searcher = WIKI_SEARCHER.read().unwrap().clone();
    let query_results = searcher.search(query).unwrap_or_default();

    let titles: Vec<String> = query_results.iter().map(|(t, _)| t.clone()).collect();
    let pages = get_pages_by_title(titles).await;

    let mut map = HashMap::new();
    for ((title, score), page) in query_results.iter().zip(pages) {
        if let Some(page) = page {
            map.insert(title.clone(), json!({
                "introduction": page.introduction,
                "sections": page.sections,
                "score": *score as u64
            }));
        }
    }

    sb.push(json!(map).to_string())
}