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
    /// Replaces a contiguous group of native content-stream text operations atomically.
    ReplaceTextGroup {
        page_index: usize,
        targets: Vec<crate::mutation::text_edit::TextEditTarget>,
        replacement: String,
    },
    /// Moves native existing content-stream text by (dx, dy) in PDF default user space.
    MoveText {
        page_index: usize,
        target: crate::mutation::text_edit::TextEditTarget,
        dx: f64,
        dy: f64,
    },
    /// Moves a group of native existing content-stream text spans atomically by (dx, dy).
    MoveTextGroup {
        page_index: usize,
        targets: Vec<crate::mutation::text_edit::TextEditTarget>,
        dx: f64,
        dy: f64,
    },

    /// Replaces an existing Image XObject in-stream or with shared clone.
    ReplaceImage {
        spec: crate::image::ReplaceImageSpec,
    },
    /// Adds a new Image XObject to a page with content-stream draw operator.
    AddImage { spec: crate::image::AddImageSpec },
    /// Moves and/or resizes an existing Image XObject occurrence on a page.
    UpdateImage { spec: crate::image::UpdateImageSpec },
    /// Removes an Image XObject draw operation from a page.
    RemoveImage { spec: crate::image::RemoveImageSpec },

    /// Updates an existing vector graphic or path object in a page content stream.
    UpdateVectorGraphic {
        spec: crate::vector::UpdateVectorGraphicSpec,
    },
    /// Adds a new vector graphic to a page content stream.
    AddVectorGraphic {
        spec: crate::vector::AddVectorGraphicSpec,
    },
    /// Deletes an existing vector graphic from a page content stream.
    DeleteVectorGraphic {
        spec: crate::vector::DeleteVectorGraphicSpec,
    },
}
