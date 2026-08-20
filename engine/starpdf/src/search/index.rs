use crate::search::matcher::TextMatcher;
use crate::search::result::{SearchOptions, SearchResult};
use crate::text::span::PageText;

#[derive(Debug, Clone)]
pub struct PageSearchIndex {
    pub page_text: PageText,
}

impl PageSearchIndex {
    pub fn new(page_text: PageText) -> Self {
        Self { page_text }
    }

    pub fn search(&self, query: &str, options: &SearchOptions) -> Vec<SearchResult> {
        TextMatcher::search_page(&self.page_text, query, options)
    }
}

#[derive(Debug, Clone)]
pub struct DocumentSearchIndex {
    pub pages: Vec<PageSearchIndex>,
}

impl DocumentSearchIndex {
    pub fn new(pages: Vec<PageText>) -> Self {
        let indices = pages.into_iter().map(PageSearchIndex::new).collect();
        Self { pages: indices }
    }

    pub fn search(&self, query: &str, options: &SearchOptions) -> Vec<SearchResult> {
        let mut results = Vec::new();
        for page_idx in &self.pages {
            results.extend(page_idx.search(query, options));
        }
        results
    }
}
