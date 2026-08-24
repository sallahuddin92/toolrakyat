#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextEditability {
    EditableNativeText,
    ReadOnlyNativeText(String),
    UnsupportedFontEncoding(String),
    UnsupportedComplexScript(String),
    UnsupportedVerticalWriting,
    UnsupportedOperator(String),
    UnsupportedLayout(String),
}

impl TextEditability {
    pub const fn is_editable(&self) -> bool {
        matches!(self, Self::EditableNativeText)
    }

    pub const fn code(&self) -> &'static str {
        match self {
            Self::EditableNativeText => "EDITABLE_NATIVE_TEXT",
            Self::ReadOnlyNativeText(_) => "READ_ONLY_NATIVE_TEXT",
            Self::UnsupportedFontEncoding(_) => "UNSUPPORTED_FONT_ENCODING",
            Self::UnsupportedComplexScript(_) => "UNSUPPORTED_COMPLEX_SCRIPT",
            Self::UnsupportedVerticalWriting => "UNSUPPORTED_VERTICAL_WRITING",
            Self::UnsupportedOperator(_) => "UNSUPPORTED_OPERATOR",
            Self::UnsupportedLayout(_) => "UNSUPPORTED_LAYOUT",
        }
    }

    pub fn reason(&self) -> Option<String> {
        match self {
            Self::EditableNativeText => None,
            Self::ReadOnlyNativeText(r)
            | Self::UnsupportedFontEncoding(r)
            | Self::UnsupportedComplexScript(r)
            | Self::UnsupportedOperator(r)
            | Self::UnsupportedLayout(r) => Some(r.clone()),
            Self::UnsupportedVerticalWriting => {
                Some("Vertical text writing mode is not supported for editing".to_string())
            }
        }
    }
}

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
    pub span_id: String,
    pub stream_index: usize,
    pub instruction_index: usize,
    pub operand_index: usize,
    pub operator_name: String,
    pub font_resource_name: String,
    pub font_base_name: String,
    pub font_family: String,
    pub is_bold: bool,
    pub is_italic: bool,
    pub is_monospace: bool,
    pub fill_color: [f64; 3],
    pub original_bytes: Vec<u8>,
    pub is_editable: bool,
    pub editability_status: TextEditability,
    pub refusal_reason: Option<String>,
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
        let span_id = format!("p{page_index}_x{x:.1}_y{y:.1}");
        Self {
            page_index,
            text,
            x,
            y,
            width,
            height,
            rotation,
            font_name: font_name.clone(),
            font_size,
            confidence,
            source_object: None,
            span_id,
            stream_index: 0,
            instruction_index: 0,
            operand_index: 0,
            operator_name: "Tj".to_string(),
            font_resource_name: font_name,
            font_base_name: String::new(),
            font_family: "SansSerif".to_string(),
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            fill_color: [0.0, 0.0, 0.0],
            original_bytes: Vec::new(),
            is_editable: false,
            editability_status: TextEditability::ReadOnlyNativeText(
                "Unresolved source identity".to_string(),
            ),
            refusal_reason: Some("Unresolved source identity".to_string()),
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
