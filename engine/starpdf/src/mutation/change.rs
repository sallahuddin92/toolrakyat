use crate::syntax::object::ObjectRef;

/// Strongly-typed discrete document mutation operation.
#[derive(Debug, Clone, PartialEq)]
pub enum PdfChange {
    /// Mutates the text value (`/V`) of a text field (`/Tx`).
    SetTextField { field_ref: ObjectRef, value: String },
    /// Mutates the boolean value (`/V`) and widget appearance state (`/AS`) of a checkbox.
    SetCheckbox {
        field_ref: ObjectRef,
        widget_refs: Vec<ObjectRef>,
        checked: bool,
    },
    /// Mutates the selected radio button within a radio group.
    SetRadio {
        parent_ref: ObjectRef,
        selected_widget_ref: ObjectRef,
        on_state: String,
    },
    /// Mutates the selected value (`/V`) of a choice (combo/list) field.
    SetChoice { field_ref: ObjectRef, value: String },
    /// Mutates the active appearance state (`/AS`) of a widget annotation directly.
    SetAppearanceState {
        widget_ref: ObjectRef,
        state_name: String,
    },
}
