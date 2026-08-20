use crate::syntax::object::ObjectRef;

/// Standard annotation subtypes defined in ISO 32000-1 §12.5.6.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnnotationSubtype {
    Text,
    Link,
    FreeText,
    Line,
    Square,
    Circle,
    Highlight,
    Underline,
    StrikeOut,
    Stamp,
    Ink,
    Widget,
    Popup,
    FileAttachment,
    Unknown(String),
}

impl AnnotationSubtype {
    pub fn from_name(name: &str) -> Self {
        match name {
            "Text" => Self::Text,
            "Link" => Self::Link,
            "FreeText" => Self::FreeText,
            "Line" => Self::Line,
            "Square" => Self::Square,
            "Circle" => Self::Circle,
            "Highlight" => Self::Highlight,
            "Underline" => Self::Underline,
            "StrikeOut" => Self::StrikeOut,
            "Stamp" => Self::Stamp,
            "Ink" => Self::Ink,
            "Widget" => Self::Widget,
            "Popup" => Self::Popup,
            "FileAttachment" => Self::FileAttachment,
            other => Self::Unknown(other.to_string()),
        }
    }

    pub fn as_name(&self) -> &str {
        match self {
            Self::Text => "Text",
            Self::Link => "Link",
            Self::FreeText => "FreeText",
            Self::Line => "Line",
            Self::Square => "Square",
            Self::Circle => "Circle",
            Self::Highlight => "Highlight",
            Self::Underline => "Underline",
            Self::StrikeOut => "StrikeOut",
            Self::Stamp => "Stamp",
            Self::Ink => "Ink",
            Self::Widget => "Widget",
            Self::Popup => "Popup",
            Self::FileAttachment => "FileAttachment",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

/// Generic annotation structural model preserving source object reference and layout.
#[derive(Debug, Clone, PartialEq)]
pub struct Annotation {
    pub object_ref: ObjectRef,
    pub page_index: usize,
    pub subtype: AnnotationSubtype,
    pub rect: [f64; 4],
    pub contents: Option<String>,
    pub name: Option<String>,
    pub flags: u32,
    pub appearance_state: Option<String>,
    pub color: Option<Vec<f64>>,
    pub is_hidden: bool,
    pub is_invisible: bool,
    pub is_print: bool,
}

/// Specification for adding a new supported annotation.
#[derive(Debug, Clone, PartialEq)]
pub enum AnnotationSpec {
    FreeText {
        rect: [f64; 4],
        text: String,
        font_size: Option<f64>,
        color: Option<Vec<f64>>,
    },
    Highlight {
        rect: [f64; 4],
        quad_points: Vec<f64>,
        color: Option<Vec<f64>>,
    },
    Underline {
        rect: [f64; 4],
        quad_points: Vec<f64>,
        color: Option<Vec<f64>>,
    },
    StrikeOut {
        rect: [f64; 4],
        quad_points: Vec<f64>,
        color: Option<Vec<f64>>,
    },
    Square {
        rect: [f64; 4],
        stroke_color: Option<Vec<f64>>,
        fill_color: Option<Vec<f64>>,
        border_width: Option<f64>,
    },
    Circle {
        rect: [f64; 4],
        stroke_color: Option<Vec<f64>>,
        fill_color: Option<Vec<f64>>,
        border_width: Option<f64>,
    },
    Line {
        line_points: [f64; 4],
        stroke_color: Option<Vec<f64>>,
        stroke_width: Option<f64>,
    },
    Ink {
        rect: [f64; 4],
        ink_list: Vec<Vec<[f64; 2]>>,
        stroke_color: Option<Vec<f64>>,
        stroke_width: Option<f64>,
    },
    Link {
        rect: [f64; 4],
        uri: String,
    },
}

impl AnnotationSpec {
    pub fn rect(&self) -> [f64; 4] {
        match self {
            Self::FreeText { rect, .. }
            | Self::Highlight { rect, .. }
            | Self::Underline { rect, .. }
            | Self::StrikeOut { rect, .. }
            | Self::Square { rect, .. }
            | Self::Circle { rect, .. }
            | Self::Ink { rect, .. }
            | Self::Link { rect, .. } => *rect,
            Self::Line { line_points, .. } => {
                let x1 = line_points[0].min(line_points[2]);
                let y1 = line_points[1].min(line_points[3]);
                let x2 = line_points[0].max(line_points[2]);
                let y2 = line_points[1].max(line_points[3]);
                [x1, y1, x2, y2]
            }
        }
    }
}

/// Specification for updating an existing annotation.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AnnotationUpdateSpec {
    pub rect: Option<[f64; 4]>,
    pub contents: Option<String>,
    pub color: Option<Vec<f64>>,
}
