#[cfg(feature = "wasm")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmDocumentInfo {
    pub page_count: usize,
    pub pdf_version: String,
    pub is_valid: bool,
}

#[cfg(feature = "wasm")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmTextSpan {
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
}

#[cfg(feature = "wasm")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmPageText {
    pub page_index: usize,
    pub plain_text: String,
    pub spans: Vec<WasmTextSpan>,
}

#[cfg(feature = "wasm")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmSearchBoundingBox {
    pub page_index: usize,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub rotation: f64,
}

#[cfg(feature = "wasm")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmSearchResult {
    pub page_index: usize,
    pub matched_text: String,
    pub start_span_index: usize,
    pub end_span_index: usize,
    pub boxes: Vec<WasmSearchBoundingBox>,
    pub confidence: f64,
}
