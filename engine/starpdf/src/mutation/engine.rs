use std::collections::BTreeMap;

use crate::annotation::generator::AnnotationGenerator;
use crate::annotation::generator::MAX_ANNOTATION_CONTENTS_LEN;
use crate::annotation::types::{AnnotationSpec, AnnotationUpdateSpec};
use crate::appearance::da_parser::DefaultAppearance;
use crate::appearance::generator::AppearanceGenerator;
use crate::appearance::status::AppearanceStatus;
use crate::document::object_store::ObjectStore;
use crate::error::{PdfError, PdfResult};
use crate::forms::field::FieldType;
use crate::mutation::change::PdfChange;
use crate::mutation::result::MutationPlan;
use crate::syntax::object::{ObjectRef, PdfObject};

const MAX_MUTATIONS_PER_BATCH: usize = 500;
const MAX_FIELD_VALUE_LEN: usize = 1_048_576; // 1 MB
const MAX_GENERATED_OBJECTS: usize = 2_000;
const MAX_WIDGET_REFS_PER_CHANGE: usize = 2_000;
const MAX_STATE_NAME_LEN: usize = 256;
const MAX_PDF_OBJECT_NUMBER: u64 = 9_999_999_999;

pub struct MutationEngine<'a, 'b> {
    store: &'a mut ObjectStore<'b>,
    page_refs: Vec<ObjectRef>,
    next_alloc_obj_num: u64,
}

impl<'a, 'b> MutationEngine<'a, 'b> {
    pub fn new(store: &'a mut ObjectStore<'b>, page_refs: &[ObjectRef]) -> Self {
        let max_num = store.xref().entries.keys().copied().max().unwrap_or(0);
        Self {
            store,
            page_refs: page_refs.to_vec(),
            next_alloc_obj_num: max_num.saturating_add(1),
        }
    }

    /// Evaluates a batch of changes atomically and produces a validated MutationPlan.
    /// If ANY change in the batch fails validation or generation, the entire transaction is aborted.
    pub fn prepare_plan(&mut self, changes: &[PdfChange]) -> PdfResult<MutationPlan> {
        if changes.len() > MAX_MUTATIONS_PER_BATCH {
            return Err(PdfError::InvalidOperation(format!(
                "Exceeded maximum batch mutation limit of {MAX_MUTATIONS_PER_BATCH}"
            )));
        }

        let mut modified_objects = BTreeMap::new();
        let mut overall_status = AppearanceStatus::AppearancePreserved;

        for change in changes {
            match change {
                PdfChange::SetTextField { field_ref, value } => {
                    if value.len() > MAX_FIELD_VALUE_LEN {
                        return Err(PdfError::InvalidOperation(
                            "Text field value exceeds maximum permitted length".into(),
                        ));
                    }
                    self.mutate_text_field(*field_ref, value, &mut modified_objects)?;
                    overall_status =
                        overall_status.combine(AppearanceStatus::AppearanceRegenerated);
                }
                PdfChange::SetCheckbox {
                    field_ref,
                    widget_refs,
                    checked,
                } => {
                    if widget_refs.len() > MAX_WIDGET_REFS_PER_CHANGE {
                        return Err(PdfError::InvalidOperation(format!(
                            "Checkbox widget count exceeds maximum of {MAX_WIDGET_REFS_PER_CHANGE}"
                        )));
                    }
                    self.mutate_checkbox(*field_ref, widget_refs, *checked, &mut modified_objects)?;
                    overall_status =
                        overall_status.combine(AppearanceStatus::AppearanceRegenerated);
                }
                PdfChange::SetRadio {
                    parent_ref,
                    selected_widget_ref,
                    on_state,
                } => {
                    Self::validate_state_name(on_state)?;
                    self.mutate_radio(
                        *parent_ref,
                        *selected_widget_ref,
                        on_state,
                        &mut modified_objects,
                    )?;
                    overall_status =
                        overall_status.combine(AppearanceStatus::AppearanceRegenerated);
                }
                PdfChange::SetChoice { field_ref, value } => {
                    if value.len() > MAX_FIELD_VALUE_LEN {
                        return Err(PdfError::InvalidOperation(
                            "Choice field value exceeds maximum permitted length".into(),
                        ));
                    }
                    self.mutate_choice(*field_ref, value, &mut modified_objects)?;
                    overall_status =
                        overall_status.combine(AppearanceStatus::AppearanceRegenerated);
                }
                PdfChange::SetAppearanceState {
                    widget_ref,
                    state_name,
                } => {
                    Self::validate_state_name(state_name)?;
                    self.mutate_appearance_state(*widget_ref, state_name, &mut modified_objects)?;
                    overall_status = overall_status.combine(AppearanceStatus::StateUpdated);
                }
                PdfChange::AddAnnotation { page_index, spec } => {
                    let status =
                        self.mutate_add_annotation(*page_index, spec, &mut modified_objects)?;
                    overall_status = overall_status.combine(status);
                }
                PdfChange::UpdateAnnotation { annot_ref, update } => {
                    self.mutate_update_annotation(*annot_ref, update, &mut modified_objects)?;
                    overall_status = overall_status.combine(AppearanceStatus::AppearancePreserved);
                }
                PdfChange::RemoveAnnotation {
                    page_index,
                    annot_ref,
                } => {
                    self.mutate_remove_annotation(*page_index, *annot_ref, &mut modified_objects)?;
                    overall_status = overall_status.combine(AppearanceStatus::ValueUpdated);
                }
            }

            if modified_objects.len() > MAX_GENERATED_OBJECTS {
                return Err(PdfError::InvalidOperation(format!(
                    "Exceeded maximum generated objects limit of {MAX_GENERATED_OBJECTS} in single transaction"
                )));
            }
        }

        Ok(MutationPlan {
            modified_objects,
            appearance_status: overall_status,
        })
    }

    fn mutate_text_field(
        &mut self,
        field_ref: ObjectRef,
        value: &str,
        modified: &mut BTreeMap<ObjectRef, PdfObject>,
    ) -> PdfResult<()> {
        let mut dict = self.get_dict_for_modification(field_ref, modified)?;

        // 1. Update /V
        dict.insert(
            "V".to_string(),
            PdfObject::String(value.as_bytes().to_vec()),
        );

        // 2. Parse field properties
        let rect = self.extract_rect_from_dict(&dict)?;
        let da_str = dict
            .get("DA")
            .and_then(|v| v.as_string_lossy())
            .unwrap_or_else(|| "/Helv 12 Tf 0 g".to_string());
        let da = DefaultAppearance::parse(&da_str)?;

        let quadding = dict
            .get("Q")
            .and_then(|v| v.as_integer())
            .map_or(0, |i| i as i32);

        let flags = dict
            .get("Ff")
            .and_then(|v| v.as_integer())
            .map_or(0, |i| i.max(0) as u32);
        let multiline = (flags & (1 << 12)) != 0;

        let field_type = FieldType::Text {
            multiline,
            password: (flags & (1 << 13)) != 0,
        };

        // 3. Regenerate /AP Form XObject if widget dimensions are present
        if rect[2] > rect[0] && rect[3] > rect[1] {
            let (ap_obj, _) = AppearanceGenerator::generate_widget_ap(
                &field_type,
                rect,
                value,
                &da,
                quadding,
                None,
                false,
            )?;
            dict.insert("AP".to_string(), ap_obj);
        }

        modified.insert(field_ref, PdfObject::Dictionary(dict));
        Ok(())
    }

    fn mutate_checkbox(
        &mut self,
        field_ref: ObjectRef,
        widget_refs: &[ObjectRef],
        checked: bool,
        modified: &mut BTreeMap<ObjectRef, PdfObject>,
    ) -> PdfResult<()> {
        let on_state = self.determine_widget_on_state(field_ref, widget_refs)?;
        let state_name = if checked { on_state.as_str() } else { "Off" };

        let mut field_dict = self.get_dict_for_modification(field_ref, modified)?;
        field_dict.insert("V".to_string(), PdfObject::Name(state_name.to_string()));

        let rect = self
            .extract_rect_from_dict(&field_dict)
            .unwrap_or([0.0, 0.0, 0.0, 0.0]);
        if rect[2] > rect[0] && rect[3] > rect[1] {
            let da = DefaultAppearance::default();
            let (ap_obj, _) = AppearanceGenerator::generate_widget_ap(
                &FieldType::Checkbox,
                rect,
                state_name,
                &da,
                0,
                Some(&on_state),
                checked,
            )?;
            field_dict.insert("AP".to_string(), ap_obj);
            field_dict.insert("AS".to_string(), PdfObject::Name(state_name.to_string()));
        } else if field_dict.contains_key("AS")
            || widget_refs.is_empty()
            || widget_refs.contains(&field_ref)
        {
            field_dict.insert("AS".to_string(), PdfObject::Name(state_name.to_string()));
        }
        modified.insert(field_ref, PdfObject::Dictionary(field_dict));

        // Mutate associated widget annotations
        for &w_ref in widget_refs {
            if w_ref == field_ref {
                continue;
            }
            let mut w_dict = self.get_dict_for_modification(w_ref, modified)?;
            w_dict.insert("AS".to_string(), PdfObject::Name(state_name.to_string()));

            let w_rect = self
                .extract_rect_from_dict(&w_dict)
                .unwrap_or([0.0, 0.0, 0.0, 0.0]);
            if w_rect[2] > w_rect[0] && w_rect[3] > w_rect[1] {
                let da = DefaultAppearance::default();
                let (ap_obj, _) = AppearanceGenerator::generate_widget_ap(
                    &FieldType::Checkbox,
                    w_rect,
                    state_name,
                    &da,
                    0,
                    Some(&on_state),
                    checked,
                )?;
                w_dict.insert("AP".to_string(), ap_obj);
            }
            modified.insert(w_ref, PdfObject::Dictionary(w_dict));
        }

        Ok(())
    }

    fn mutate_radio(
        &mut self,
        parent_ref: ObjectRef,
        selected_widget_ref: ObjectRef,
        on_state: &str,
        modified: &mut BTreeMap<ObjectRef, PdfObject>,
    ) -> PdfResult<()> {
        let mut parent_dict = self.get_dict_for_modification(parent_ref, modified)?;
        parent_dict.insert("V".to_string(), PdfObject::Name(on_state.to_string()));

        let kids_arr: Vec<PdfObject> = parent_dict
            .get("Kids")
            .and_then(|v| v.as_array())
            .map(|arr| arr.to_vec())
            .unwrap_or_default();

        modified.insert(parent_ref, PdfObject::Dictionary(parent_dict));

        for kid_obj in kids_arr {
            if let Some(kid_ref) = kid_obj.as_reference() {
                let is_selected = kid_ref == selected_widget_ref;
                let state_to_set = if is_selected { on_state } else { "Off" };

                let mut kid_dict = self.get_dict_for_modification(kid_ref, modified)?;
                kid_dict.insert("AS".to_string(), PdfObject::Name(state_to_set.to_string()));

                let k_rect = self
                    .extract_rect_from_dict(&kid_dict)
                    .unwrap_or([0.0, 0.0, 0.0, 0.0]);
                if k_rect[2] > k_rect[0] && k_rect[3] > k_rect[1] {
                    let da = DefaultAppearance::default();
                    let (ap_obj, _) = AppearanceGenerator::generate_widget_ap(
                        &FieldType::RadioButtonGroup,
                        k_rect,
                        state_to_set,
                        &da,
                        0,
                        Some(on_state),
                        is_selected,
                    )?;
                    kid_dict.insert("AP".to_string(), ap_obj);
                }
                modified.insert(kid_ref, PdfObject::Dictionary(kid_dict));
            }
        }

        if !modified.contains_key(&selected_widget_ref) {
            let mut w_dict = self.get_dict_for_modification(selected_widget_ref, modified)?;
            w_dict.insert("AS".to_string(), PdfObject::Name(on_state.to_string()));
            modified.insert(selected_widget_ref, PdfObject::Dictionary(w_dict));
        }

        Ok(())
    }

    fn mutate_choice(
        &mut self,
        field_ref: ObjectRef,
        value: &str,
        modified: &mut BTreeMap<ObjectRef, PdfObject>,
    ) -> PdfResult<()> {
        let mut dict = self.get_dict_for_modification(field_ref, modified)?;
        dict.insert(
            "V".to_string(),
            PdfObject::String(value.as_bytes().to_vec()),
        );

        let rect = self.extract_rect_from_dict(&dict)?;
        let da_str = dict
            .get("DA")
            .and_then(|v| v.as_string_lossy())
            .unwrap_or_else(|| "/Helv 12 Tf 0 g".to_string());
        let da = DefaultAppearance::parse(&da_str)?;

        let quadding = dict
            .get("Q")
            .and_then(|v| v.as_integer())
            .map_or(0, |i| i as i32);

        if rect[2] > rect[0] && rect[3] > rect[1] {
            let (ap_obj, _) = AppearanceGenerator::generate_widget_ap(
                &FieldType::Choice {
                    combo: true,
                    multi_select: false,
                },
                rect,
                value,
                &da,
                quadding,
                None,
                false,
            )?;
            dict.insert("AP".to_string(), ap_obj);
        }

        modified.insert(field_ref, PdfObject::Dictionary(dict));
        Ok(())
    }

    fn mutate_appearance_state(
        &mut self,
        widget_ref: ObjectRef,
        state_name: &str,
        modified: &mut BTreeMap<ObjectRef, PdfObject>,
    ) -> PdfResult<()> {
        let mut dict = self.get_dict_for_modification(widget_ref, modified)?;
        dict.insert("AS".to_string(), PdfObject::Name(state_name.to_string()));
        modified.insert(widget_ref, PdfObject::Dictionary(dict));
        Ok(())
    }

    fn mutate_add_annotation(
        &mut self,
        page_index: usize,
        spec: &AnnotationSpec,
        modified: &mut BTreeMap<ObjectRef, PdfObject>,
    ) -> PdfResult<AppearanceStatus> {
        let page_ref = self.page_refs.get(page_index).copied().ok_or_else(|| {
            PdfError::InvalidOperation(format!(
                "Page index {page_index} out of bounds (document has {} pages)",
                self.page_refs.len()
            ))
        })?;

        // 1. Generate annotation dictionary & optional appearance stream
        let (mut annot_dict, stream_opt) = AnnotationGenerator::generate_annotation_objects(spec)?;
        annot_dict.insert("P".to_string(), PdfObject::Reference(page_ref));

        let annot_ref = self.allocate_object_ref()?;

        let appearance_status;
        if let Some(stream) = stream_opt {
            let stream_ref = self.allocate_object_ref()?;
            let ap_dict = BTreeMap::from([("N".to_string(), PdfObject::Reference(stream_ref))]);
            annot_dict.insert("AP".to_string(), PdfObject::Dictionary(ap_dict));
            modified.insert(stream_ref, PdfObject::Stream(stream));
            appearance_status = AppearanceStatus::AppearanceRegenerated;
        } else {
            appearance_status = AppearanceStatus::ValueUpdated;
        }

        modified.insert(annot_ref, PdfObject::Dictionary(annot_dict));

        // 2. Append annotation reference to page's /Annots array
        let mut page_dict = self.get_dict_for_modification(page_ref, modified)?;
        let mut annots_arr = match page_dict.get("Annots") {
            Some(PdfObject::Array(arr)) => arr.clone(),
            Some(PdfObject::Reference(r)) => {
                let resolved = self.store.resolve(*r)?;
                resolved.as_array().map(|s| s.to_vec()).unwrap_or_default()
            }
            _ => Vec::new(),
        };

        annots_arr.push(PdfObject::Reference(annot_ref));
        page_dict.insert("Annots".to_string(), PdfObject::Array(annots_arr));
        modified.insert(page_ref, PdfObject::Dictionary(page_dict));

        Ok(appearance_status)
    }

    fn mutate_update_annotation(
        &mut self,
        annot_ref: ObjectRef,
        update: &AnnotationUpdateSpec,
        modified: &mut BTreeMap<ObjectRef, PdfObject>,
    ) -> PdfResult<()> {
        let mut dict = self.get_dict_for_modification(annot_ref, modified)?;
        Self::validate_annotation_dictionary(&dict)?;

        if let Some(rect) = update.rect {
            Self::validate_rect(rect, "Annotation update rectangle")?;
            dict.insert(
                "Rect".to_string(),
                PdfObject::Array(vec![
                    PdfObject::Real(rect[0]),
                    PdfObject::Real(rect[1]),
                    PdfObject::Real(rect[2]),
                    PdfObject::Real(rect[3]),
                ]),
            );
        }

        if let Some(contents) = &update.contents {
            if contents.len() > MAX_ANNOTATION_CONTENTS_LEN {
                return Err(PdfError::InvalidOperation(format!(
                    "Annotation contents exceed maximum length of {MAX_ANNOTATION_CONTENTS_LEN} bytes"
                )));
            }
            dict.insert(
                "Contents".to_string(),
                PdfObject::String(contents.as_bytes().to_vec()),
            );
        }

        if let Some(color) = &update.color {
            if crate::appearance::PdfColor::parse_from_slice(color).is_none() {
                return Err(PdfError::InvalidOperation(
                    "Annotation color must contain 1, 3, or 4 finite components".into(),
                ));
            }
            let col_objs: Vec<PdfObject> = color.iter().map(|v| PdfObject::Real(*v)).collect();
            dict.insert("C".to_string(), PdfObject::Array(col_objs));
        }

        modified.insert(annot_ref, PdfObject::Dictionary(dict));
        Ok(())
    }

    fn mutate_remove_annotation(
        &mut self,
        page_index: usize,
        annot_ref: ObjectRef,
        modified: &mut BTreeMap<ObjectRef, PdfObject>,
    ) -> PdfResult<()> {
        let page_ref = self.page_refs.get(page_index).copied().ok_or_else(|| {
            PdfError::InvalidOperation(format!(
                "Page index {page_index} out of bounds (document has {} pages)",
                self.page_refs.len()
            ))
        })?;

        let mut page_dict = self.get_dict_for_modification(page_ref, modified)?;
        let mut annots_arr = match page_dict.get("Annots") {
            Some(PdfObject::Array(arr)) => arr.clone(),
            Some(PdfObject::Reference(r)) => {
                let resolved = self.store.resolve(*r)?;
                resolved.as_array().map(|s| s.to_vec()).unwrap_or_default()
            }
            _ => Vec::new(),
        };

        if !annots_arr
            .iter()
            .any(|item| item.as_reference() == Some(annot_ref))
        {
            return Err(PdfError::InvalidOperation(format!(
                "Annotation {} {} is not associated with page {page_index}",
                annot_ref.number, annot_ref.generation
            )));
        }
        let annot_dict = self.get_dict_for_modification(annot_ref, modified)?;
        Self::validate_annotation_dictionary(&annot_dict)?;
        annots_arr.retain(|item| item.as_reference() != Some(annot_ref));
        page_dict.insert("Annots".to_string(), PdfObject::Array(annots_arr));
        modified.insert(page_ref, PdfObject::Dictionary(page_dict));

        Ok(())
    }

    fn allocate_object_ref(&mut self) -> PdfResult<ObjectRef> {
        let num = self.next_alloc_obj_num;
        if num > MAX_PDF_OBJECT_NUMBER {
            return Err(PdfError::InvalidOperation(
                "PDF object number allocation limit exceeded".into(),
            ));
        }
        self.next_alloc_obj_num = num.checked_add(1).ok_or_else(|| {
            PdfError::InvalidOperation("PDF object number allocation overflow".into())
        })?;
        Ok(ObjectRef::new(num, 0))
    }

    fn get_dict_for_modification(
        &mut self,
        obj_ref: ObjectRef,
        modified: &BTreeMap<ObjectRef, PdfObject>,
    ) -> PdfResult<BTreeMap<String, PdfObject>> {
        if let Some(existing) = modified.get(&obj_ref) {
            if let Some(d) = existing.as_dict() {
                return Ok(d.clone());
            }
        }

        let resolved = self.store.resolve(obj_ref)?.clone();
        match resolved.as_dict() {
            Some(d) => Ok(d.clone()),
            None => Err(PdfError::TypeMismatch {
                expected: "dictionary",
                actual: resolved.type_name(),
            }),
        }
    }

    fn extract_rect_from_dict(
        &mut self,
        dict: &BTreeMap<String, PdfObject>,
    ) -> PdfResult<[f64; 4]> {
        let rect_obj = match dict.get("Rect") {
            Some(r) => r,
            None => return Ok([0.0, 0.0, 0.0, 0.0]),
        };

        let arr = match rect_obj {
            PdfObject::Array(a) => a.clone(),
            PdfObject::Reference(r) => {
                let resolved = self.store.resolve(*r)?;
                resolved.as_array().map(|s| s.to_vec()).unwrap_or_default()
            }
            _ => return Ok([0.0, 0.0, 0.0, 0.0]),
        };

        if arr.len() < 4 {
            return Ok([0.0, 0.0, 0.0, 0.0]);
        }

        let rect = [
            arr[0].as_real().unwrap_or(0.0),
            arr[1].as_real().unwrap_or(0.0),
            arr[2].as_real().unwrap_or(0.0),
            arr[3].as_real().unwrap_or(0.0),
        ];
        if !rect.iter().all(|value| value.is_finite()) {
            return Err(PdfError::InvalidOperation(
                "Widget rectangle contains non-finite coordinates".into(),
            ));
        }
        Ok(rect)
    }

    fn validate_state_name(state_name: &str) -> PdfResult<()> {
        if state_name.is_empty() || state_name.len() > MAX_STATE_NAME_LEN {
            return Err(PdfError::InvalidOperation(format!(
                "Appearance state name must contain 1..={MAX_STATE_NAME_LEN} bytes"
            )));
        }
        Ok(())
    }

    fn validate_rect(rect: [f64; 4], label: &str) -> PdfResult<()> {
        if !rect.iter().all(|value| value.is_finite()) || rect[2] <= rect[0] || rect[3] <= rect[1] {
            return Err(PdfError::InvalidOperation(format!(
                "{label} must contain finite coordinates with positive width and height"
            )));
        }
        Ok(())
    }

    fn validate_annotation_dictionary(dict: &BTreeMap<String, PdfObject>) -> PdfResult<()> {
        if dict.get("Subtype").and_then(PdfObject::as_name).is_none() {
            return Err(PdfError::InvalidOperation(
                "Target object is not an annotation dictionary".into(),
            ));
        }
        Ok(())
    }

    fn determine_widget_on_state(
        &mut self,
        field_ref: ObjectRef,
        widget_refs: &[ObjectRef],
    ) -> PdfResult<String> {
        let candidates = if widget_refs.is_empty() {
            vec![field_ref]
        } else {
            widget_refs.to_vec()
        };

        for w_ref in candidates {
            if let Ok(resolved) = self.store.resolve(w_ref) {
                let resolved_obj = resolved.clone();
                if let Some(w_dict) = resolved_obj.as_dict() {
                    if let Some(ap_obj) = w_dict.get("AP") {
                        if let Ok(ap_resolved) = self.store.resolve_object(ap_obj) {
                            if let Some(ap_dict) = ap_resolved.as_dict() {
                                if let Some(n_obj) = ap_dict.get("N") {
                                    if let Ok(n_resolved) = self.store.resolve_object(n_obj) {
                                        if let Some(n_dict) = n_resolved.as_dict() {
                                            for key in n_dict.keys() {
                                                if key != "Off" {
                                                    return Ok(key.clone());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok("Yes".to_string())
    }
}
