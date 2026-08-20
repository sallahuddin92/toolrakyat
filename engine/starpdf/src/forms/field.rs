use crate::forms::widget::WidgetAnnotation;
use crate::syntax::object::ObjectRef;

/// Specific category of AcroForm field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldType {
    Text { multiline: bool, password: bool },
    Checkbox,
    RadioButtonGroup,
    PushButton,
    Choice { combo: bool, multi_select: bool },
    Signature,
    Unknown(String),
}

/// Logical value held by a form field.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    Text(String),
    Boolean(bool),
    Choice(Vec<String>),
    Name(String),
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldGraphClassification {
    CanonicalField,
    MultiWidgetField,
    OrphanWidget,
    AmbiguousWidgetGroup,
    MalformedFieldGraph,
}

impl FieldGraphClassification {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CanonicalField => "CANONICAL_FIELD",
            Self::MultiWidgetField => "MULTI_WIDGET_FIELD",
            Self::OrphanWidget => "ORPHAN_WIDGET",
            Self::AmbiguousWidgetGroup => "AMBIGUOUS_WIDGET_GROUP",
            Self::MalformedFieldGraph => "MALFORMED_FIELD_GRAPH",
        }
    }
}

/// Represents an option within a Choice (Combo/List) field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceOption {
    pub export_value: String,
    pub display_value: String,
}

/// Strongly-typed AcroForm field model preserving underlying PDF object references.
#[derive(Debug, Clone, PartialEq)]
pub struct FormField {
    pub object_ref: ObjectRef,
    pub parent_ref: Option<ObjectRef>,
    pub field_type: FieldType,
    pub partial_name: String,
    pub fully_qualified_name: String,
    pub alternate_name: Option<String>,
    pub mapping_name: Option<String>,
    pub value: FieldValue,
    pub default_value: FieldValue,
    pub flags: u32,
    pub default_appearance: Option<String>,
    pub quadding: Option<i32>,
    pub max_len: Option<usize>,
    pub is_comb: bool,
    pub options: Vec<ChoiceOption>,
    pub selected_indices: Vec<usize>,
    pub widgets: Vec<WidgetAnnotation>,
    pub is_read_only: bool,
    pub is_required: bool,
    pub graph_classification: FieldGraphClassification,
}

impl FormField {
    #[inline]
    pub fn is_text(&self) -> bool {
        matches!(self.field_type, FieldType::Text { .. })
    }

    #[inline]
    pub fn is_checkbox(&self) -> bool {
        matches!(self.field_type, FieldType::Checkbox)
    }

    #[inline]
    pub fn is_radio(&self) -> bool {
        matches!(self.field_type, FieldType::RadioButtonGroup)
    }

    #[inline]
    pub fn is_choice(&self) -> bool {
        matches!(self.field_type, FieldType::Choice { .. })
    }

    #[inline]
    pub fn is_signature(&self) -> bool {
        matches!(self.field_type, FieldType::Signature)
    }
}
