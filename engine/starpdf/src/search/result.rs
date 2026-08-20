#[derive(Debug, Clone, PartialEq)]
pub struct SearchBoundingBox {
    pub page_index: usize,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub rotation: f64,
}

impl SearchBoundingBox {
    pub const fn new(
        page_index: usize,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        rotation: f64,
    ) -> Self {
        Self {
            page_index,
            x,
            y,
            width,
            height,
            rotation,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    pub page_index: usize,
    pub matched_text: String,
    pub start_span_index: usize,
    pub end_span_index: usize,
    pub boxes: Vec<SearchBoundingBox>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchOptions {
    pub case_sensitive: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            case_sensitive: false,
        }
    }
}
