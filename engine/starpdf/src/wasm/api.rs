#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "wasm")]
use crate::annotation::types::{AnnotationSpec, AnnotationUpdateSpec, LineEndingStyle};
#[cfg(feature = "wasm")]
use crate::mutation::PdfChange;
#[cfg(feature = "wasm")]
use crate::page_ops::{PageRange, PageSource};
#[cfg(feature = "wasm")]
use crate::search::SearchOptions;
#[cfg(feature = "wasm")]
use crate::syntax::object::ObjectRef;
#[cfg(feature = "wasm")]
use crate::validate::StructuralValidator;
#[cfg(feature = "wasm")]
use crate::wasm::dto::{
    WasmAddAnnotationInput, WasmAnnotation, WasmChoiceOption, WasmDocumentInfo, WasmFormField,
    WasmImageInfo, WasmImageMutationResult, WasmPageText, WasmReplaceTextResult,
    WasmSearchBoundingBox, WasmSearchResult, WasmSecurityInfo, WasmTextSpan,
    WasmUpdateAnnotationInput, WasmWidget,
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
    "0.12.1".to_string()
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_get_security_info(handle: u32) -> Result<JsValue, JsValue> {
    REGISTRY
        .with_doc(handle, |doc| {
            let info = doc.security_info()?;
            let dto = WasmSecurityInfo {
                signature_state: info.signature_state.as_str().to_string(),
                signature_count: info.signature_count,
                byte_range_count: info.byte_ranges.len(),
                encryption_state: info.encryption_state.as_str().to_string(),
                encryption_filter: info.encryption_filter,
                encryption_subfilter: info.encryption_subfilter,
                permission_raw: info.permissions.raw,
                permission_printing: info.permissions.printing,
                permission_modification: info.permissions.modification,
                permission_copying: info.permissions.copying,
                permission_annotation_and_forms: info.permissions.annotation_and_forms,
                mutation_allowed: info.mutation_allowed,
                mutation_reason_code: info.mutation_reason_code,
                signed_mutation_state: info.signed_mutation_state.as_str().to_string(),
                cryptographic_verification: "NOT_PERFORMED".to_string(),
                document_id_valid: info.document_id_valid,
            };
            serde_wasm_bindgen::to_value(&dto)
                .map_err(|error| crate::error::PdfError::InvalidOperation(error.to_string()))
        })
        .map_err(to_js_error)
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
                    span_id: s.span_id.clone(),
                    stream_index: s.stream_index,
                    instruction_index: s.instruction_index,
                    operand_index: s.operand_index,
                    operator_name: s.operator_name.clone(),
                    is_editable: s.is_editable,
                    editability_code: s.editability_status.code().to_string(),
                    refusal_reason: s.refusal_reason.clone(),
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
                            span_id: s.span_id.clone(),
                            stream_index: s.stream_index,
                            instruction_index: s.instruction_index,
                            operand_index: s.operand_index,
                            operator_name: s.operator_name.clone(),
                            is_editable: s.is_editable,
                            editability_code: s.editability_status.code().to_string(),
                            refusal_reason: s.refusal_reason.clone(),
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
pub fn starpdf_replace_text(
    handle: u32,
    page_index: u32,
    span_id: &str,
    new_text: &str,
) -> Result<JsValue, JsValue> {
    let mut layout_str = String::from("EXACT_FIT");
    let mut mod_count = 0;

    let _ = REGISTRY
        .transform_and_replace(handle, |doc| {
            let target = crate::mutation::text_edit::TextEditTarget::from_span_id(span_id)?;
            let plan = doc.replace_text(page_index as usize, &target, new_text)?;
            if let Some(l) = &plan.layout_policy_result {
                layout_str = l.as_str().to_string();
            }
            mod_count = plan.modified_objects.len();
            doc.export_incremental(&plan)
        })
        .map_err(to_js_error)?;

    let dto = WasmReplaceTextResult {
        success: true,
        layout_result: layout_str,
        modified_object_count: mod_count,
    };

    serde_wasm_bindgen::to_value(&dto)
        .map_err(|e| to_js_error(crate::error::PdfError::InvalidOperation(e.to_string())))
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_get_text_editability(
    handle: u32,
    page_index: u32,
    span_id: &str,
) -> Result<JsValue, JsValue> {
    REGISTRY
        .with_doc(handle, |doc| {
            let page_text = doc.extract_page_text(page_index as usize)?;
            let span = page_text
                .spans
                .iter()
                .find(|s| s.span_id == span_id)
                .ok_or_else(|| {
                    crate::error::PdfError::TargetTextNotFound(format!(
                        "Span ID '{span_id}' not found on page {page_index}"
                    ))
                })?;

            let dto = WasmTextSpan {
                page_index: span.page_index,
                text: span.text.clone(),
                x: span.x,
                y: span.y,
                width: span.width,
                height: span.height,
                rotation: span.rotation,
                font_name: span.font_name.clone(),
                font_size: span.font_size,
                confidence: span.confidence,
                span_id: span.span_id.clone(),
                stream_index: span.stream_index,
                instruction_index: span.instruction_index,
                operand_index: span.operand_index,
                operator_name: span.operator_name.clone(),
                is_editable: span.is_editable,
                editability_code: span.editability_status.code().to_string(),
                refusal_reason: span.refusal_reason.clone(),
            };

            serde_wasm_bindgen::to_value(&dto)
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
                                has_normal_appearance: w.has_normal_appearance,
                                has_rollover_appearance: w.has_rollover_appearance,
                                has_down_appearance: w.has_down_appearance,
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
                        graph_classification: f.graph_classification.as_str().to_string(),
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
                    uri: a.uri,
                    has_normal_appearance: a.has_normal_appearance,
                    has_rollover_appearance: a.has_rollover_appearance,
                    has_down_appearance: a.has_down_appearance,
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
pub fn starpdf_delete_page(handle: u32, page_index: u32) -> Result<Vec<u8>, JsValue> {
    REGISTRY
        .transform_and_replace(handle, |document| document.delete_page(page_index as usize))
        .map_err(to_js_error)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_move_page(handle: u32, from_index: u32, to_index: u32) -> Result<Vec<u8>, JsValue> {
    REGISTRY
        .transform_and_replace(handle, |document| {
            document.move_page(from_index as usize, to_index as usize)
        })
        .map_err(to_js_error)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_duplicate_page(
    handle: u32,
    page_index: u32,
    destination_index: u32,
) -> Result<Vec<u8>, JsValue> {
    REGISTRY
        .transform_and_replace(handle, |document| {
            document.duplicate_page(page_index as usize, destination_index as usize)
        })
        .map_err(to_js_error)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_insert_blank_page(
    handle: u32,
    page_index: u32,
    width: f64,
    height: f64,
    rotation: i32,
) -> Result<Vec<u8>, JsValue> {
    REGISTRY
        .transform_and_replace(handle, |document| {
            document.insert_blank_page(page_index as usize, width, height, rotation)
        })
        .map_err(to_js_error)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_extract_pages(handle: u32, page_indices: JsValue) -> Result<Vec<u8>, JsValue> {
    let indices: Vec<u32> = serde_wasm_bindgen::from_value(page_indices)
        .map_err(|error| JsValue::from_str(&format!("Invalid page selection: {error}")))?;
    let indices = indices
        .into_iter()
        .map(|index| index as usize)
        .collect::<Vec<_>>();
    REGISTRY
        .with_doc(handle, |document| document.extract_pages(&indices))
        .map_err(to_js_error)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_insert_imported_page(
    handle: u32,
    imported_bytes: &[u8],
    imported_page_index: u32,
    insert_at: u32,
) -> Result<Vec<u8>, JsValue> {
    let imported = crate::document::PdfDocument::from_bytes(imported_bytes).map_err(to_js_error)?;
    REGISTRY
        .transform_and_replace(handle, |document| {
            document.insert_page_from(&imported, imported_page_index as usize, insert_at as usize)
        })
        .map_err(to_js_error)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_merge_documents(documents: JsValue) -> Result<Vec<u8>, JsValue> {
    let documents: Vec<Vec<u8>> = serde_wasm_bindgen::from_value(documents)
        .map_err(|error| JsValue::from_str(&format!("Invalid merge documents: {error}")))?;
    let inputs = documents.iter().map(Vec::as_slice).collect::<Vec<_>>();
    crate::document::PdfDocument::merge_documents(&inputs).map_err(to_js_error)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_merge_selected(
    documents: JsValue,
    page_sources: JsValue,
) -> Result<Vec<u8>, JsValue> {
    let documents: Vec<Vec<u8>> = serde_wasm_bindgen::from_value(documents)
        .map_err(|error| JsValue::from_str(&format!("Invalid merge documents: {error}")))?;
    let page_sources: Vec<[u32; 2]> = serde_wasm_bindgen::from_value(page_sources)
        .map_err(|error| JsValue::from_str(&format!("Invalid merge page selection: {error}")))?;
    let page_sources = page_sources
        .into_iter()
        .map(|source| PageSource::new(source[0] as usize, source[1] as usize))
        .collect::<Vec<_>>();
    let inputs = documents.iter().map(Vec::as_slice).collect::<Vec<_>>();
    crate::document::PdfDocument::merge_selected(&inputs, &page_sources).map_err(to_js_error)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_split_document(handle: u32, ranges: JsValue) -> Result<JsValue, JsValue> {
    let ranges: Vec<[u32; 2]> = serde_wasm_bindgen::from_value(ranges)
        .map_err(|error| JsValue::from_str(&format!("Invalid split ranges: {error}")))?;
    let ranges = ranges
        .into_iter()
        .map(|range| PageRange::new(range[0] as usize, range[1] as usize))
        .collect::<Vec<_>>();
    let outputs = REGISTRY
        .with_doc(handle, |document| document.split_document(&ranges))
        .map_err(to_js_error)?;
    serde_wasm_bindgen::to_value(&outputs)
        .map_err(|error| JsValue::from_str(&format!("Failed to encode split outputs: {error}")))
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
pub fn starpdf_get_glyph_mapping_quality(handle: u32) -> Result<String, JsValue> {
    REGISTRY
        .last_glyph_mapping_quality(handle)
        .map(|quality| {
            quality
                .map_or("NOT_APPLICABLE", |value| value.as_str())
                .to_string()
        })
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

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_enumerate_images(handle: u32, page_index: u32) -> Result<JsValue, JsValue> {
    REGISTRY
        .with_doc(handle, |doc| {
            let images = doc.enumerate_images(page_index as usize)?;
            let dtos: Vec<WasmImageInfo> = images
                .into_iter()
                .map(|img| WasmImageInfo {
                    image_id: img.image_id,
                    page_index: img.page_index,
                    stream_index: img.stream_index,
                    instruction_index: img.instruction_index,
                    resource_name: img.resource_name,
                    width: img.width,
                    height: img.height,
                    color_space: img.color_space,
                    bits_per_component: img.bits_per_component,
                    filter: img.filter,
                    transform: img.transform,
                    rect: img.rect,
                    is_nested_form: img.is_nested_form,
                    is_shared: img.is_shared,
                })
                .collect();
            serde_wasm_bindgen::to_value(&dtos)
                .map_err(|e| crate::error::PdfError::InvalidOperation(e.to_string()))
        })
        .map_err(to_js_error)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_replace_image(
    handle: u32,
    page_index: u32,
    image_id: &str,
    new_image_bytes: &[u8],
    clone_if_shared: bool,
) -> Result<JsValue, JsValue> {
    let mut mod_count = 0;
    let _ = REGISTRY
        .transform_and_replace(handle, |doc| {
            let spec = crate::image::ReplaceImageSpec {
                page_index: page_index as usize,
                image_id: image_id.to_string(),
                new_image_bytes: new_image_bytes.to_vec(),
                format: crate::image::ImageFormat::AutoDetect,
                clone_if_shared,
            };
            let plan = doc.replace_image(&spec)?;
            mod_count = plan.modified_objects.len();
            doc.export_incremental(&plan)
        })
        .map_err(to_js_error)?;

    let dto = WasmImageMutationResult {
        success: true,
        modified_object_count: mod_count,
    };
    serde_wasm_bindgen::to_value(&dto)
        .map_err(|e| to_js_error(crate::error::PdfError::InvalidOperation(e.to_string())))
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_add_image(
    handle: u32,
    page_index: u32,
    image_bytes: &[u8],
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<JsValue, JsValue> {
    let mut mod_count = 0;
    let _ = REGISTRY
        .transform_and_replace(handle, |doc| {
            let spec = crate::image::AddImageSpec {
                page_index: page_index as usize,
                image_bytes: image_bytes.to_vec(),
                format: crate::image::ImageFormat::AutoDetect,
                x,
                y,
                width,
                height,
            };
            let plan = doc.add_image(&spec)?;
            mod_count = plan.modified_objects.len();
            doc.export_incremental(&plan)
        })
        .map_err(to_js_error)?;

    let dto = WasmImageMutationResult {
        success: true,
        modified_object_count: mod_count,
    };
    serde_wasm_bindgen::to_value(&dto)
        .map_err(|e| to_js_error(crate::error::PdfError::InvalidOperation(e.to_string())))
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_remove_image(
    handle: u32,
    page_index: u32,
    image_id: &str,
) -> Result<JsValue, JsValue> {
    let mut mod_count = 0;
    let _ = REGISTRY
        .transform_and_replace(handle, |doc| {
            let spec = crate::image::RemoveImageSpec {
                page_index: page_index as usize,
                image_id: image_id.to_string(),
            };
            let plan = doc.remove_image(&spec)?;
            mod_count = plan.modified_objects.len();
            doc.export_incremental(&plan)
        })
        .map_err(to_js_error)?;

    let dto = WasmImageMutationResult {
        success: true,
        modified_object_count: mod_count,
    };
    serde_wasm_bindgen::to_value(&dto)
        .map_err(|e| to_js_error(crate::error::PdfError::InvalidOperation(e.to_string())))
}
