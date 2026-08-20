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
