use crate::syntax::object::ObjectRef;

/// Line ending styles implemented by the v0.8 appearance generator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineEndingStyle {
    #[default]
    None,
    Square,
    Circle,
    Diamond,
    OpenArrow,
    ClosedArrow,
}

impl LineEndingStyle {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "None" => Some(Self::None),
            "Square" => Some(Self::Square),
            "Circle" => Some(Self::Circle),
            "Diamond" => Some(Self::Diamond),
            "OpenArrow" => Some(Self::OpenArrow),
            "ClosedArrow" => Some(Self::ClosedArrow),
            _ => None,
        }
    }

    pub const fn as_name(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Square => "Square",
            Self::Circle => "Circle",
            Self::Diamond => "Diamond",
            Self::OpenArrow => "OpenArrow",
            Self::ClosedArrow => "ClosedArrow",
        }
    }
}

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
#[allow(clippy::struct_excessive_bools)]
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
    pub interior_color: Option<Vec<f64>>,
    pub border_width: Option<f64>,
    pub line_points: Option<[f64; 4]>,
    pub line_endings: Option<[LineEndingStyle; 2]>,
    pub quad_points: Vec<f64>,
    pub ink_list: Vec<Vec<[f64; 2]>>,
    pub uri: Option<String>,
    pub has_normal_appearance: bool,
    pub has_rollover_appearance: bool,
    pub has_down_appearance: bool,
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
        fill_color: Option<Vec<f64>>,
        stroke_width: Option<f64>,
        line_endings: [LineEndingStyle; 2],
        contents: Option<String>,
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
            Self::Line {
                line_points,
                stroke_width,
                line_endings,
                ..
            } => {
                let x1 = line_points[0].min(line_points[2]);
                let y1 = line_points[1].min(line_points[3]);
                let x2 = line_points[0].max(line_points[2]);
                let y2 = line_points[1].max(line_points[3]);
                let ending_pad = if line_endings
                    .iter()
                    .any(|ending| *ending != LineEndingStyle::None)
                {
                    8.0
                } else {
                    2.0
                };
                let pad = stroke_width.unwrap_or(1.0).clamp(0.1, 20.0) + ending_pad;
                [x1 - pad, y1 - pad, x2 + pad, y2 + pad]
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
    pub fill_color: Option<Vec<f64>>,
    pub border_width: Option<f64>,
    pub line_points: Option<[f64; 4]>,
    pub line_endings: Option<[LineEndingStyle; 2]>,
    pub quad_points: Option<Vec<f64>>,
    pub ink_list: Option<Vec<Vec<[f64; 2]>>>,
    pub font_family: Option<String>,
    pub font_size: Option<f64>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub text_color: Option<[f64; 3]>,
}
