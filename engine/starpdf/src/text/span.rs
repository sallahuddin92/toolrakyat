#[derive(Debug, Clone, PartialEq)]
pub struct TextSpan {
    pub page_index: usize,
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub rotation: f64,
    pub font_name: String,
    pub font_size: f64,
    pub confidence: f64,
    pub source_object: Option<u64>,
}

impl TextSpan {
    pub fn new(
        page_index: usize,
        text: String,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        rotation: f64,
        font_name: String,
        font_size: f64,
        confidence: f64,
    ) -> Self {
        Self {
            page_index,
            text,
            x,
            y,
            width,
            height,
            rotation,
            font_name,
            font_size,
            confidence,
            source_object: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PageText {
    pub page_index: usize,
    pub spans: Vec<TextSpan>,
}

impl PageText {
    pub fn new(page_index: usize) -> Self {
        Self {
            page_index,
            spans: Vec::new(),
        }
    }

    /// Concatenates all text spans on the page into a plain text string.
    pub fn plain_text(&self) -> String {
        let mut full_text = String::new();
        let mut prev_y: Option<f64> = None;

        for span in &self.spans {
            if let Some(py) = prev_y {
                // If vertical baseline shifted significantly, add a newline
                if (span.y - py).abs() > (span.height.max(4.0) * 0.8) {
                    if !full_text.ends_with('\n') {
                        full_text.push('\n');
                    }
                } else if !full_text.ends_with(' ') && !span.text.starts_with(' ') {
                    full_text.push(' ');
                }
            }

            full_text.push_str(&span.text);
            prev_y = Some(span.y);
        }

        full_text
    }

    /// Searches for a query string in this page text.
    pub fn search(
        &self,
        query: &str,
        options: &crate::search::SearchOptions,
    ) -> Vec<crate::search::SearchResult> {
        crate::search::TextMatcher::search_page(self, query, options)
    }
}
