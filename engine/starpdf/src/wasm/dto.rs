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

#[cfg(feature = "wasm")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmChoiceOption {
    pub export_value: String,
    pub display_value: String,
}

#[cfg(feature = "wasm")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmWidget {
    pub object_num: u64,
    pub object_gen: u16,
    pub page_index: Option<usize>,
    pub rect: [f64; 4],
    pub appearance_state: Option<String>,
    pub normal_appearance_states: Vec<String>,
    pub is_checked: bool,
}

#[cfg(feature = "wasm")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmFormField {
    pub object_num: u64,
    pub object_gen: u16,
    pub parent_num: Option<u64>,
    pub parent_gen: Option<u16>,
    pub field_type: String,
    pub name: String,
    pub alternate_name: Option<String>,
    pub mapping_name: Option<String>,
    pub value: String,
    pub is_read_only: bool,
    pub is_required: bool,
    pub max_len: Option<usize>,
    pub is_comb: bool,
    pub options: Vec<WasmChoiceOption>,
    pub selected_indices: Vec<usize>,
    pub widgets: Vec<WasmWidget>,
}

#[cfg(feature = "wasm")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmAnnotation {
    pub object_num: u64,
    pub object_gen: u16,
    pub page_index: usize,
    pub subtype: String,
    pub rect: [f64; 4],
    pub contents: Option<String>,
    pub name: Option<String>,
    pub appearance_state: Option<String>,
    pub color: Option<Vec<f64>>,
    pub interior_color: Option<Vec<f64>>,
    pub border_width: Option<f64>,
    pub line_points: Option<[f64; 4]>,
    pub line_endings: Option<[String; 2]>,
    pub quad_points: Vec<f64>,
    pub ink_list: Vec<Vec<[f64; 2]>>,
}

#[cfg(feature = "wasm")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmAddAnnotationInput {
    pub subtype: String,
    pub rect: [f64; 4],
    pub contents: Option<String>,
    pub font_size: Option<f64>,
    pub color: Option<Vec<f64>>,
    pub fill_color: Option<Vec<f64>>,
    pub border_width: Option<f64>,
    pub quad_points: Option<Vec<f64>>,
    pub line_points: Option<[f64; 4]>,
    pub line_endings: Option<[String; 2]>,
    pub ink_list: Option<Vec<Vec<[f64; 2]>>>,
    pub uri: Option<String>,
}

#[cfg(feature = "wasm")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmUpdateAnnotationInput {
    pub rect: Option<[f64; 4]>,
    pub contents: Option<String>,
    pub color: Option<Vec<f64>>,
    pub fill_color: Option<Vec<f64>>,
    pub border_width: Option<f64>,
    pub line_points: Option<[f64; 4]>,
    pub line_endings: Option<[String; 2]>,
    pub quad_points: Option<Vec<f64>>,
    pub ink_list: Option<Vec<Vec<[f64; 2]>>>,
}
