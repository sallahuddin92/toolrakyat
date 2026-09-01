#[cfg(feature = "wasm")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmDocumentInfo {
    pub page_count: usize,
    pub pdf_version: String,
    pub is_valid: bool,
    pub recovery_status: String,
    pub recovery_events: Vec<String>,
    pub xref_status: String,
}

#[cfg(feature = "wasm")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmSecurityInfo {
    pub xref_status: String,
    pub signature_state: String,
    pub signature_count: usize,
    pub byte_range_count: usize,
    pub encryption_state: String,
    pub encryption_filter: Option<String>,
    pub encryption_subfilter: Option<String>,
    pub permission_raw: Option<i32>,
    pub permission_printing: Option<bool>,
    pub permission_modification: Option<bool>,
    pub permission_copying: Option<bool>,
    pub permission_annotation_and_forms: Option<bool>,
    pub mutation_allowed: bool,
    pub mutation_reason_code: Option<String>,
    pub signed_mutation_state: String,
    pub cryptographic_verification: String,
    pub document_id_valid: bool,
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
    pub span_id: String,
    pub stream_index: usize,
    pub instruction_index: usize,
    pub operand_index: usize,
    pub operator_name: String,
    pub font_family: Option<String>,
    pub is_bold: Option<bool>,
    pub is_italic: Option<bool>,
    pub is_monospace: Option<bool>,
    pub fill_color: Option<[f64; 3]>,
    pub underline: Option<bool>,
    pub strikethrough: Option<bool>,
    pub highlight_color: Option<[f64; 3]>,
    pub base_font: Option<String>,
    pub is_editable: bool,
    pub editability_code: String,
    pub refusal_reason: Option<String>,
}

#[cfg(feature = "wasm")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmReplaceTextResult {
    pub success: bool,
    pub layout_result: String,
    pub modified_object_count: usize,
}

#[cfg(feature = "wasm")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmTextReplacementPlan {
    pub is_executable: bool,
    pub strategy: String,
    pub direction: String,
    pub font_resource_name: String,
    pub predicted_width: f64,
    pub available_width: f64,
    pub layout_safety: String,
    pub refusal_reason: Option<String>,
}

#[cfg(feature = "wasm")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmTextStylePlan {
    pub is_executable: bool,
    pub computed: crate::font::ComputedTextStyle,
    pub requested: crate::font::ComputedTextStyle,
    pub strategy: String,
    pub layout_safety: String,
    pub refusal_reason: Option<String>,
}

#[cfg(feature = "wasm")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmMoveTextResult {
    pub success: bool,
    pub modified_object_count: usize,
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
#[allow(clippy::struct_excessive_bools)]
pub struct WasmWidget {
    pub object_num: u64,
    pub object_gen: u16,
    pub page_index: Option<usize>,
    pub rect: [f64; 4],
    pub appearance_state: Option<String>,
    pub normal_appearance_states: Vec<String>,
    pub has_normal_appearance: bool,
    pub has_rollover_appearance: bool,
    pub has_down_appearance: bool,
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
    pub graph_classification: String,
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
    pub uri: Option<String>,
    pub font_family: Option<String>,
    pub font_size: Option<f64>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub text_color: Option<[f64; 3]>,
    pub underline: Option<bool>,
    pub strikethrough: Option<bool>,
    pub highlight_color: Option<[f64; 3]>,
    pub has_normal_appearance: bool,
    pub has_rollover_appearance: bool,
    pub has_down_appearance: bool,
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
    pub font_family: Option<String>,
    pub font_size: Option<f64>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub text_color: Option<[f64; 3]>,
    pub underline: Option<bool>,
    pub strikethrough: Option<bool>,
    pub highlight_enabled: Option<bool>,
    pub highlight_color: Option<[f64; 3]>,
}

#[cfg(feature = "wasm")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmImageInfo {
    pub image_id: String,
    pub page_index: usize,
    pub stream_index: usize,
    pub instruction_index: usize,
    pub resource_name: String,
    pub width: u32,
    pub height: u32,
    pub color_space: String,
    pub bits_per_component: u32,
    pub filter: Option<String>,
    pub transform: [f64; 6],
    pub rect: [f64; 4],
    pub is_nested_form: bool,
    pub is_shared: bool,
}

#[cfg(feature = "wasm")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmReplaceImageInput {
    pub page_index: usize,
    pub image_id: String,
    pub new_image_bytes: Vec<u8>,
    pub clone_if_shared: bool,
}

#[cfg(feature = "wasm")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmAddImageInput {
    pub page_index: usize,
    pub image_bytes: Vec<u8>,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[cfg(feature = "wasm")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmRemoveImageInput {
    pub page_index: usize,
    pub image_id: String,
}

#[cfg(feature = "wasm")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmImageMutationResult {
    pub success: bool,
    pub modified_object_count: usize,
}

#[cfg(feature = "wasm")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmVectorGraphicInfo {
    pub graphic_id: String,
    pub page_index: usize,
    pub stream_index: usize,
    pub start_instruction_index: usize,
    pub end_instruction_index: usize,
    pub graphic_type: String,
    pub bounds: [f64; 4],
    pub local_bounds: [f64; 4],
    pub transform: [f64; 6],
    pub stroke_color_rgb: Option<[f64; 3]>,
    pub fill_color_rgb: Option<[f64; 3]>,
    pub stroke_color_hex: Option<String>,
    pub fill_color_hex: Option<String>,
    pub line_width: f64,
    pub is_stroked: bool,
    pub is_filled: bool,
    pub is_shared: bool,
    pub is_editable: bool,
    pub editability_code: String,
    pub refusal_reason: Option<String>,
    pub rect_x: Option<f64>,
    pub rect_y: Option<f64>,
    pub rect_w: Option<f64>,
    pub rect_h: Option<f64>,
    pub line_x1: Option<f64>,
    pub line_y1: Option<f64>,
    pub line_x2: Option<f64>,
    pub line_y2: Option<f64>,
}

#[cfg(feature = "wasm")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmUpdateVectorGraphicInput {
    pub page_index: usize,
    pub graphic_id: String,
    pub rect_x: Option<f64>,
    pub rect_y: Option<f64>,
    pub rect_w: Option<f64>,
    pub rect_h: Option<f64>,
    pub line_x1: Option<f64>,
    pub line_y1: Option<f64>,
    pub line_x2: Option<f64>,
    pub line_y2: Option<f64>,
    pub stroke_color_rgb: Option<[f64; 3]>,
    pub fill_color_rgb: Option<[f64; 3]>,
    pub line_width: Option<f64>,
    pub is_stroked: Option<bool>,
    pub is_filled: Option<bool>,
    #[serde(default)]
    pub clone_if_shared: bool,
}

#[cfg(feature = "wasm")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmAddRectangleInput {
    pub page_index: usize,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub stroke_color_rgb: Option<[f64; 3]>,
    pub fill_color_rgb: Option<[f64; 3]>,
    pub line_width: f64,
    pub is_stroked: bool,
    pub is_filled: bool,
}

#[cfg(feature = "wasm")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmAddLineInput {
    pub page_index: usize,
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub stroke_color_rgb: Option<[f64; 3]>,
    pub line_width: f64,
}

#[cfg(feature = "wasm")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmDeleteVectorGraphicInput {
    pub page_index: usize,
    pub graphic_id: String,
    pub clone_if_shared: bool,
}

#[cfg(feature = "wasm")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmVectorMutationResult {
    pub success: bool,
    pub modified_object_count: usize,
}
