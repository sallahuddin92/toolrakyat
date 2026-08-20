use std::collections::BTreeMap;

use crate::document::object_store::ObjectStore;
use crate::error::{PdfError, PdfResult};
use crate::mutation::change::PdfChange;
use crate::mutation::result::{AppearanceStatus, MutationPlan};
use crate::syntax::object::{ObjectRef, PdfObject};

const MAX_MUTATIONS_PER_PLAN: usize = 1000;
const MAX_FIELD_VALUE_LEN: usize = 1_048_576; // 1 MB limit

pub struct MutationEngine<'a, 'b> {
    store: &'a mut ObjectStore<'b>,
}

impl<'a, 'b> MutationEngine<'a, 'b> {
    pub fn new(store: &'a mut ObjectStore<'b>) -> Self {
        Self { store }
    }

    /// Evaluates a batch of changes and produces a validated MutationPlan containing modified objects.
    pub fn prepare_plan(&mut self, changes: &[PdfChange]) -> PdfResult<MutationPlan> {
        if changes.len() > MAX_MUTATIONS_PER_PLAN {
            return Err(PdfError::InvalidOperation(format!(
                "Exceeded maximum batch mutation limit of {MAX_MUTATIONS_PER_PLAN}"
            )));
        }

        let mut modified_objects = BTreeMap::new();
        let mut overall_status = AppearanceStatus::AppearanceStreamPreserved;

        for change in changes {
            match change {
                PdfChange::SetTextField { field_ref, value } => {
                    if value.len() > MAX_FIELD_VALUE_LEN {
                        return Err(PdfError::InvalidOperation(
                            "Text field value exceeds maximum permitted length".into(),
                        ));
                    }
                    self.mutate_text_field(*field_ref, value, &mut modified_objects)?;
                    overall_status = AppearanceStatus::LogicalOnlyUpdated;
                }
                PdfChange::SetCheckbox {
                    field_ref,
                    widget_refs,
                    checked,
                } => {
                    self.mutate_checkbox(*field_ref, widget_refs, *checked, &mut modified_objects)?;
                    overall_status = AppearanceStatus::AppearanceStateUpdated;
                }
                PdfChange::SetRadio {
                    parent_ref,
                    selected_widget_ref,
                    on_state,
                } => {
                    self.mutate_radio(
                        *parent_ref,
                        *selected_widget_ref,
                        on_state,
                        &mut modified_objects,
                    )?;
                    overall_status = AppearanceStatus::AppearanceStateUpdated;
                }
                PdfChange::SetChoice { field_ref, value } => {
                    if value.len() > MAX_FIELD_VALUE_LEN {
                        return Err(PdfError::InvalidOperation(
                            "Choice field value exceeds maximum permitted length".into(),
                        ));
                    }
                    self.mutate_choice(*field_ref, value, &mut modified_objects)?;
                    overall_status = AppearanceStatus::LogicalOnlyUpdated;
                }
                PdfChange::SetAppearanceState {
                    widget_ref,
                    state_name,
                } => {
                    self.mutate_appearance_state(*widget_ref, state_name, &mut modified_objects)?;
                    overall_status = AppearanceStatus::AppearanceStateUpdated;
                }
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
        dict.insert(
            "V".to_string(),
            PdfObject::String(value.as_bytes().to_vec()),
        );
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
        // Determine on-state from first widget's /AP /N dictionary if available
        let on_state = self.determine_widget_on_state(field_ref, widget_refs)?;
        let state_name = if checked { on_state.as_str() } else { "Off" };

        // 1. Mutate field dictionary /V
        let mut field_dict = self.get_dict_for_modification(field_ref, modified)?;
        field_dict.insert("V".to_string(), PdfObject::Name(state_name.to_string()));

        // If field dict itself contains /AS, update it
        if field_dict.contains_key("AS")
            || widget_refs.is_empty()
            || widget_refs.contains(&field_ref)
        {
            field_dict.insert("AS".to_string(), PdfObject::Name(state_name.to_string()));
        }
        modified.insert(field_ref, PdfObject::Dictionary(field_dict));

        // 2. Mutate associated widget annotations /AS
        for &w_ref in widget_refs {
            if w_ref == field_ref {
                continue;
            }
            let mut w_dict = self.get_dict_for_modification(w_ref, modified)?;
            w_dict.insert("AS".to_string(), PdfObject::Name(state_name.to_string()));
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
        // 1. Update parent /V to on_state
        let mut parent_dict = self.get_dict_for_modification(parent_ref, modified)?;
        parent_dict.insert("V".to_string(), PdfObject::Name(on_state.to_string()));

        // Resolve sibling kids to turn them "Off"
        let kids_arr: Vec<PdfObject> = parent_dict
            .get("Kids")
            .and_then(|v| v.as_array())
            .map(|arr| arr.to_vec())
            .unwrap_or_default();

        modified.insert(parent_ref, PdfObject::Dictionary(parent_dict));

        // 2. Update all sibling widgets
        for kid_obj in kids_arr {
            if let Some(kid_ref) = kid_obj.as_reference() {
                let is_selected = kid_ref == selected_widget_ref;
                let state_to_set = if is_selected { on_state } else { "Off" };

                let mut kid_dict = self.get_dict_for_modification(kid_ref, modified)?;
                kid_dict.insert("AS".to_string(), PdfObject::Name(state_to_set.to_string()));
                modified.insert(kid_ref, PdfObject::Dictionary(kid_dict));
            }
        }

        // If selected_widget_ref is not in /Kids, ensure it is still updated
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
