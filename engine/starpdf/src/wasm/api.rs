#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "wasm")]
use crate::annotation::types::{AnnotationSpec, AnnotationUpdateSpec, LineEndingStyle};
#[cfg(feature = "wasm")]
use crate::mutation::PdfChange;
#[cfg(feature = "wasm")]
use crate::search::SearchOptions;
#[cfg(feature = "wasm")]
use crate::syntax::object::ObjectRef;
#[cfg(feature = "wasm")]
use crate::validate::StructuralValidator;
#[cfg(feature = "wasm")]
use crate::wasm::dto::{
    WasmAddAnnotationInput, WasmAnnotation, WasmChoiceOption, WasmDocumentInfo, WasmFormField,
    WasmPageText, WasmSearchBoundingBox, WasmSearchResult, WasmTextSpan, WasmUpdateAnnotationInput,
    WasmWidget,
};
#[cfg(feature = "wasm")]
use crate::wasm::registry::REGISTRY;
#[cfg(feature = "wasm")]
use crate::writer::MinimalWriter;

#[cfg(feature = "wasm")]
fn to_js_error<E: std::fmt::Display>(err: E) -> JsValue {
    JsValue::from_str(&err.to_string())
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_version() -> String {
    "0.8.0".to_string()
}

#[cfg(feature = "wasm")]
fn parse_line_endings(values: Option<&[String; 2]>) -> Result<[LineEndingStyle; 2], JsValue> {
    let Some(values) = values else {
        return Ok([LineEndingStyle::None, LineEndingStyle::None]);
    };
    let first = LineEndingStyle::from_name(&values[0]).ok_or_else(|| {
        JsValue::from_str(&format!("Unsupported line ending style: {}", values[0]))
    })?;
    let second = LineEndingStyle::from_name(&values[1]).ok_or_else(|| {
        JsValue::from_str(&format!("Unsupported line ending style: {}", values[1]))
    })?;
    Ok([first, second])
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_open(bytes: &[u8]) -> Result<u32, JsValue> {
    REGISTRY.insert(bytes.to_vec()).map_err(to_js_error)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_get_info(handle: u32) -> Result<JsValue, JsValue> {
    REGISTRY
        .with_doc(handle, |doc| {
            let page_count = doc.page_count()?;
            let is_valid = StructuralValidator::validate(doc).is_ok();
            let info = WasmDocumentInfo {
                page_count,
                pdf_version: doc.version().to_string(),
                is_valid,
            };
            serde_wasm_bindgen::to_value(&info)
                .map_err(|e| crate::error::PdfError::InvalidOperation(e.to_string()))
        })
        .map_err(to_js_error)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_get_page_count(handle: u32) -> Result<u32, JsValue> {
    REGISTRY
        .with_doc(handle, |doc| {
            let count = doc.page_count()?;
            Ok(count as u32)
        })
        .map_err(to_js_error)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_extract_page_text(handle: u32, page_index: u32) -> Result<JsValue, JsValue> {
    REGISTRY
        .with_doc(handle, |doc| {
            let page_text = doc.extract_page_text(page_index as usize)?;
            let spans = page_text
                .spans
                .iter()
                .map(|s| WasmTextSpan {
                    page_index: s.page_index,
                    text: s.text.clone(),
                    x: s.x,
                    y: s.y,
                    width: s.width,
                    height: s.height,
                    rotation: s.rotation,
                    font_name: s.font_name.clone(),
                    font_size: s.font_size,
                    confidence: s.confidence,
                })
                .collect();

            let result = WasmPageText {
                page_index: page_text.page_index,
                plain_text: page_text.plain_text(),
                spans,
            };

            serde_wasm_bindgen::to_value(&result)
                .map_err(|e| crate::error::PdfError::InvalidOperation(e.to_string()))
        })
        .map_err(to_js_error)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_extract_all_text(handle: u32) -> Result<JsValue, JsValue> {
    REGISTRY
        .with_doc(handle, |doc| {
            let all_pages = doc.extract_all_text()?;
            let results: Vec<WasmPageText> = all_pages
                .into_iter()
                .map(|page_text| {
                    let spans = page_text
                        .spans
                        .iter()
                        .map(|s| WasmTextSpan {
                            page_index: s.page_index,
                            text: s.text.clone(),
                            x: s.x,
                            y: s.y,
                            width: s.width,
                            height: s.height,
                            rotation: s.rotation,
                            font_name: s.font_name.clone(),
                            font_size: s.font_size,
                            confidence: s.confidence,
                        })
                        .collect();
                    WasmPageText {
                        page_index: page_text.page_index,
                        plain_text: page_text.plain_text(),
                        spans,
                    }
                })
                .collect();

            serde_wasm_bindgen::to_value(&results)
                .map_err(|e| crate::error::PdfError::InvalidOperation(e.to_string()))
        })
        .map_err(to_js_error)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_search(handle: u32, query: &str, case_sensitive: bool) -> Result<JsValue, JsValue> {
    REGISTRY
        .with_doc(handle, |doc| {
            let options = SearchOptions { case_sensitive };
            let hits = doc.search(query, &options)?;
            let results: Vec<WasmSearchResult> = hits
                .into_iter()
                .map(|h| WasmSearchResult {
                    page_index: h.page_index,
                    matched_text: h.matched_text,
                    start_span_index: h.start_span_index,
                    end_span_index: h.end_span_index,
                    boxes: h
                        .boxes
                        .into_iter()
                        .map(|b| WasmSearchBoundingBox {
                            page_index: b.page_index,
                            x: b.x,
                            y: b.y,
                            width: b.width,
                            height: b.height,
                            rotation: b.rotation,
                        })
                        .collect(),
                    confidence: h.confidence,
                })
                .collect();

            serde_wasm_bindgen::to_value(&results)
                .map_err(|e| crate::error::PdfError::InvalidOperation(e.to_string()))
        })
        .map_err(to_js_error)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_validate(handle: u32) -> Result<bool, JsValue> {
    REGISTRY
        .with_doc(handle, |doc| Ok(StructuralValidator::validate(doc).is_ok()))
        .map_err(to_js_error)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_get_form_fields(handle: u32) -> Result<JsValue, JsValue> {
    REGISTRY
        .with_doc(handle, |doc| {
            let fields = doc.form_fields()?;
            let wasm_fields: Vec<WasmFormField> = fields
                .into_iter()
                .map(|f| {
                    let field_type_str = match &f.field_type {
                        crate::forms::FieldType::Text { .. } => "text",
                        crate::forms::FieldType::Checkbox => "checkbox",
                        crate::forms::FieldType::RadioButtonGroup => "radio",
                        crate::forms::FieldType::PushButton => "button",
                        crate::forms::FieldType::Choice { combo: true, .. } => "combobox",
                        crate::forms::FieldType::Choice { combo: false, .. } => "listbox",
                        crate::forms::FieldType::Signature => "signature",
                        crate::forms::FieldType::Unknown(s) => s.as_str(),
                    }
                    .to_string();

                    let value_str = match &f.value {
                        crate::forms::FieldValue::Text(t) => t.clone(),
                        crate::forms::FieldValue::Boolean(b) => {
                            if *b { "true" } else { "false" }.to_string()
                        }
                        crate::forms::FieldValue::Name(n) => n.clone(),
                        crate::forms::FieldValue::Choice(opts) => opts.join(", "),
                        crate::forms::FieldValue::None => String::new(),
                    };

                    let widgets = f
                        .widgets
                        .into_iter()
                        .map(|w| {
                            let is_checked = w.is_checked();
                            WasmWidget {
                                object_num: w.object_ref.number,
                                object_gen: w.object_ref.generation,
                                page_index: w.page_index,
                                rect: w.rect,
                                appearance_state: w.appearance_state,
                                normal_appearance_states: w.normal_appearance_states,
                                is_checked,
                            }
                        })
                        .collect();

                    let options = f
                        .options
                        .into_iter()
                        .map(|opt| WasmChoiceOption {
                            export_value: opt.export_value,
                            display_value: opt.display_value,
                        })
                        .collect();

                    WasmFormField {
                        object_num: f.object_ref.number,
                        object_gen: f.object_ref.generation,
                        parent_num: f.parent_ref.map(|r| r.number),
                        parent_gen: f.parent_ref.map(|r| r.generation),
                        field_type: field_type_str,
                        name: f.fully_qualified_name,
                        alternate_name: f.alternate_name,
                        mapping_name: f.mapping_name,
                        value: value_str,
                        is_read_only: f.is_read_only,
                        is_required: f.is_required,
                        max_len: f.max_len,
                        is_comb: f.is_comb,
                        options,
                        selected_indices: f.selected_indices,
                        widgets,
                    }
                })
                .collect();

            serde_wasm_bindgen::to_value(&wasm_fields)
                .map_err(|e| crate::error::PdfError::InvalidOperation(e.to_string()))
        })
        .map_err(to_js_error)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_get_annotations(handle: u32, page_index: u32) -> Result<JsValue, JsValue> {
    REGISTRY
        .with_doc(handle, |doc| {
            let annots = doc.page_annotations(page_index as usize)?;
            let wasm_annots: Vec<WasmAnnotation> = annots
                .into_iter()
                .map(|a| WasmAnnotation {
                    object_num: a.object_ref.number,
                    object_gen: a.object_ref.generation,
                    page_index: a.page_index,
                    subtype: a.subtype.as_name().to_string(),
                    rect: a.rect,
                    contents: a.contents,
                    name: a.name,
                    appearance_state: a.appearance_state,
                    color: a.color,
                    interior_color: a.interior_color,
                    border_width: a.border_width,
                    line_points: a.line_points,
                    line_endings: a.line_endings.map(|endings| {
                        [
                            endings[0].as_name().to_string(),
                            endings[1].as_name().to_string(),
                        ]
                    }),
                    quad_points: a.quad_points,
                    ink_list: a.ink_list,
                })
                .collect();

            serde_wasm_bindgen::to_value(&wasm_annots)
                .map_err(|e| crate::error::PdfError::InvalidOperation(e.to_string()))
        })
        .map_err(to_js_error)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_set_text_field(
    handle: u32,
    obj_num: u64,
    obj_gen: u16,
    value: &str,
) -> Result<bool, JsValue> {
    let field_ref = ObjectRef::new(obj_num, obj_gen);
    let change = PdfChange::SetTextField {
        field_ref,
        value: value.to_string(),
    };
    REGISTRY.add_change(handle, change).map_err(to_js_error)?;
    Ok(true)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_set_checkbox(
    handle: u32,
    obj_num: u64,
    obj_gen: u16,
    checked: bool,
) -> Result<bool, JsValue> {
    let field_ref = ObjectRef::new(obj_num, obj_gen);
    let change = PdfChange::SetCheckbox {
        field_ref,
        widget_refs: vec![field_ref],
        checked,
    };
    REGISTRY.add_change(handle, change).map_err(to_js_error)?;
    Ok(true)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_set_radio(
    handle: u32,
    parent_num: u64,
    parent_gen: u16,
    widget_num: u64,
    widget_gen: u16,
    on_state: &str,
) -> Result<bool, JsValue> {
    let parent_ref = ObjectRef::new(parent_num, parent_gen);
    let selected_widget_ref = ObjectRef::new(widget_num, widget_gen);
    let change = PdfChange::SetRadio {
        parent_ref,
        selected_widget_ref,
        on_state: on_state.to_string(),
    };
    REGISTRY.add_change(handle, change).map_err(to_js_error)?;
    Ok(true)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_set_choice(
    handle: u32,
    obj_num: u64,
    obj_gen: u16,
    value: &str,
) -> Result<bool, JsValue> {
    let field_ref = ObjectRef::new(obj_num, obj_gen);
    let change = PdfChange::SetChoice {
        field_ref,
        value: value.to_string(),
    };
    REGISTRY.add_change(handle, change).map_err(to_js_error)?;
    Ok(true)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_set_choice_values(
    handle: u32,
    obj_num: u64,
    obj_gen: u16,
    values_val: JsValue,
) -> Result<bool, JsValue> {
    let values: Vec<String> = serde_wasm_bindgen::from_value(values_val)
        .map_err(|error| JsValue::from_str(&format!("Invalid choice values: {error}")))?;
    let change = PdfChange::SetChoiceValues {
        field_ref: ObjectRef::new(obj_num, obj_gen),
        values,
    };
    REGISTRY.add_change(handle, change).map_err(to_js_error)?;
    Ok(true)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_add_annotation(
    handle: u32,
    page_index: u32,
    annotation_val: JsValue,
) -> Result<bool, JsValue> {
    let input: WasmAddAnnotationInput = serde_wasm_bindgen::from_value(annotation_val)
        .map_err(|e| JsValue::from_str(&format!("Invalid annotation input: {e}")))?;

    let line_endings = parse_line_endings(input.line_endings.as_ref())?;
    let spec = match input.subtype.as_str() {
        "FreeText" => AnnotationSpec::FreeText {
            rect: input.rect,
            text: input.contents.unwrap_or_default(),
            font_size: input.font_size,
            color: input.color,
        },
        "Square" => AnnotationSpec::Square {
            rect: input.rect,
            stroke_color: input.color,
            fill_color: input.fill_color,
            border_width: input.border_width,
        },
        "Circle" => AnnotationSpec::Circle {
            rect: input.rect,
            stroke_color: input.color,
            fill_color: input.fill_color,
            border_width: input.border_width,
        },
        "Line" => AnnotationSpec::Line {
            line_points: input.line_points.unwrap_or([
                input.rect[0],
                input.rect[1],
                input.rect[2],
                input.rect[3],
            ]),
            stroke_color: input.color,
            fill_color: input.fill_color,
            stroke_width: input.border_width,
            line_endings,
            contents: input.contents,
        },
        "Highlight" => AnnotationSpec::Highlight {
            rect: input.rect,
            quad_points: input.quad_points.unwrap_or_default(),
            color: input.color,
        },
        "Underline" => AnnotationSpec::Underline {
            rect: input.rect,
            quad_points: input.quad_points.unwrap_or_default(),
            color: input.color,
        },
        "StrikeOut" => AnnotationSpec::StrikeOut {
            rect: input.rect,
            quad_points: input.quad_points.unwrap_or_default(),
            color: input.color,
        },
        "Ink" => AnnotationSpec::Ink {
            rect: input.rect,
            ink_list: input.ink_list.unwrap_or_default(),
            stroke_color: input.color,
            stroke_width: input.border_width,
        },
        "Link" => AnnotationSpec::Link {
            rect: input.rect,
            uri: input.uri.unwrap_or_default(),
        },
        other => {
            return Err(JsValue::from_str(&format!(
                "Unsupported annotation subtype: {other}"
            )));
        }
    };

    let change = PdfChange::AddAnnotation {
        page_index: page_index as usize,
        spec,
    };
    REGISTRY.add_change(handle, change).map_err(to_js_error)?;
    Ok(true)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_update_annotation(
    handle: u32,
    obj_num: u64,
    obj_gen: u16,
    update_val: JsValue,
) -> Result<bool, JsValue> {
    let input: WasmUpdateAnnotationInput = serde_wasm_bindgen::from_value(update_val)
        .map_err(|e| JsValue::from_str(&format!("Invalid update input: {e}")))?;

    let line_endings = input
        .line_endings
        .as_ref()
        .map(|values| parse_line_endings(Some(values)))
        .transpose()?;
    let update = AnnotationUpdateSpec {
        rect: input.rect,
        contents: input.contents,
        color: input.color,
        fill_color: input.fill_color,
        border_width: input.border_width,
        line_points: input.line_points,
        line_endings,
        quad_points: input.quad_points,
        ink_list: input.ink_list,
    };

    let change = PdfChange::UpdateAnnotation {
        annot_ref: ObjectRef::new(obj_num, obj_gen),
        update,
    };
    REGISTRY.add_change(handle, change).map_err(to_js_error)?;
    Ok(true)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_remove_annotation(
    handle: u32,
    page_index: u32,
    obj_num: u64,
    obj_gen: u16,
) -> Result<bool, JsValue> {
    let change = PdfChange::RemoveAnnotation {
        page_index: page_index as usize,
        annot_ref: ObjectRef::new(obj_num, obj_gen),
    };
    REGISTRY.add_change(handle, change).map_err(to_js_error)?;
    Ok(true)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_export_incremental(handle: u32) -> Result<Vec<u8>, JsValue> {
    REGISTRY
        .export_and_apply_changes(handle)
        .map_err(to_js_error)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_get_appearance_status(handle: u32) -> Result<String, JsValue> {
    REGISTRY
        .last_appearance_status(handle)
        .map(|status| status.as_str().to_string())
        .map_err(to_js_error)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_close(handle: u32) -> Result<bool, JsValue> {
    REGISTRY.close(handle).map_err(to_js_error)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_create_minimal_pdf(text: &str) -> Result<Vec<u8>, JsValue> {
    MinimalWriter::create_minimal_pdf(text).map_err(to_js_error)
}
