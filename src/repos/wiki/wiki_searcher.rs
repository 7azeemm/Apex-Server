use crate::repos::wiki::wiki_repo::get_pages_by_title;
use crate::structs::wiki_structs::WikiPage;
use std::sync::{Arc, LazyLock, RwLock};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{doc, Index};

const TOP_N: usize = 5;
static WIKI_SEARCHER: LazyLock<RwLock<Arc<WikiSearcher>>> = LazyLock::new(|| {
    RwLock::new(Arc::new(WikiSearcher::new(Vec::new()).unwrap()))
});

pub fn update_index(pages: Vec<WikiPage>) {
    match WikiSearcher::new(pages) {
        Err(e) => println!("Error while building wiki index, {:?}", e),
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
                intro_field => page.introduction().clone().unwrap_or_default(),
            );

            for section in page.sections() {
                document.add_text(section_title_field, format!("{} {}", page.title(), section.title()));
                document.add_text(section_content_field, section.content());
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
            vec![self.title_field, self.intro_field, self.section_title_field, self.section_content_field],
        );
        parser.set_field_boost(self.title_field, 4.0);
        parser.set_field_boost(self.section_title_field, 2.5);
        parser.set_field_boost(self.intro_field, 1.5);
        parser.set_field_boost(self.section_content_field, 0.8);
        parser
    }

    pub fn search(&self, query: &str) -> tantivy::Result<Vec<(String, f32)>> {
        let reader = self.index.reader()?;
        let searcher = reader.searcher();
        let parser = self.create_parser();

        let query_parsed = parser.parse_query(query)?;
        let top_docs = searcher.search(&query_parsed, &TopDocs::with_limit(TOP_N))?;

        let mut results: Vec<(String, f32)> = top_docs.into_iter().map(|(score, doc_address)| {
            let retrieved_doc = searcher.doc::<TantivyDocument>(doc_address).unwrap_or_default();

            let title = retrieved_doc
                .get_first(self.title_field)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            (title, score)
        }).collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(results)
    }
}

pub fn get_searcher() -> Arc<WikiSearcher> {
    WIKI_SEARCHER.read().unwrap().clone()
}

pub async fn search_wiki(query: &str) -> Vec<(WikiPage, f32)> {
    let searcher = WIKI_SEARCHER.read().unwrap().clone();
    let results = searcher.search(query).unwrap_or_default();

    let titles: Vec<String> = results.iter().map(|(t, _)| t.clone()).collect();
    let pages = get_pages_by_title(titles).await;

    results
        .into_iter()
        .zip(pages)
        .filter_map(|((_, score), page_opt)| page_opt.map(|p| (p, score)))
        .collect()
}