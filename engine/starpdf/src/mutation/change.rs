use crate::annotation::types::{AnnotationSpec, AnnotationUpdateSpec};
use crate::syntax::object::ObjectRef;

/// Strongly-typed discrete document mutation operation.
#[derive(Debug, Clone, PartialEq)]
pub enum PdfChange {
    /// Mutates the text value (`/V`) of a text field (`/Tx`) and regenerates its `/AP /N` stream.
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
    /// Mutates the selected value (`/V`) of a choice (combo/list) field and regenerates its `/AP /N` stream.
    SetChoice { field_ref: ObjectRef, value: String },
    /// Mutates one or more list-box values and synchronizes `/V`, `/I`, and `/AP`.
    SetChoiceValues {
        field_ref: ObjectRef,
        values: Vec<String>,
    },
    /// Mutates the active appearance state (`/AS`) of a widget annotation directly.
    SetAppearanceState {
        widget_ref: ObjectRef,
        state_name: String,
    },
    /// Adds a new annotation to a specified page.
    AddAnnotation {
        page_index: usize,
        spec: AnnotationSpec,
    },
    /// Updates an existing annotation.
    UpdateAnnotation {
        annot_ref: ObjectRef,
        update: AnnotationUpdateSpec,
    },
    /// Removes an existing annotation from a specified page.
    RemoveAnnotation {
        page_index: usize,
        annot_ref: ObjectRef,
    },
    /// Replaces native existing content-stream text within the bounded v0.13 safety envelope.
    ReplaceText {
        page_index: usize,
        target: crate::mutation::text_edit::TextEditTarget,
        replacement: String,
    },
    /// Replaces an existing Image XObject in-stream or with shared clone.
    ReplaceImage {
        spec: crate::image::ReplaceImageSpec,
    },
    /// Adds a new Image XObject to a page with content-stream draw operator.
    AddImage { spec: crate::image::AddImageSpec },
    /// Removes an Image XObject draw operation from a page.
    RemoveImage { spec: crate::image::RemoveImageSpec },
}
