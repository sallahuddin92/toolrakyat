use std::collections::{BTreeMap, HashSet};

use crate::document::object_store::ObjectStore;
use crate::error::{PdfError, PdfResult};
use crate::forms::field::{ChoiceOption, FieldType, FieldValue, FormField};
use crate::forms::widget::WidgetAnnotation;
use crate::syntax::object::{ObjectRef, PdfObject};

const MAX_FIELD_TREE_DEPTH: usize = 32;
const MAX_ACROFORM_FIELDS: usize = 1000;
const MAX_OPTIONS_COUNT: usize = 5000;

/// Information parsed from the document Catalog `/AcroForm` entry.
#[derive(Debug, Clone, PartialEq)]
pub struct AcroForm {
    pub object_ref: Option<ObjectRef>,
    pub fields: Vec<FormField>,
    pub need_appearances: bool,
    pub signature_flags: u32,
    pub default_appearance: Option<String>,
}

#[derive(Default, Clone)]
struct InheritedAttributes {
    field_type: Option<String>,
    flags: Option<u32>,
    default_appearance: Option<String>,
    quadding: Option<i32>,
    options: Option<Vec<ChoiceOption>>,
}

pub struct AcroFormParser<'a, 'b> {
    store: &'a mut ObjectStore<'b>,
    page_ref_to_index: BTreeMap<ObjectRef, usize>,
    visited_nodes: HashSet<ObjectRef>,
    fields_collected: Vec<FormField>,
}

impl<'a, 'b> AcroFormParser<'a, 'b> {
    pub fn new(store: &'a mut ObjectStore<'b>, page_refs: &[ObjectRef]) -> Self {
        let mut page_ref_to_index = BTreeMap::new();
        for (idx, &p_ref) in page_refs.iter().enumerate() {
            page_ref_to_index.insert(p_ref, idx);
        }
        Self {
            store,
            page_ref_to_index,
            visited_nodes: HashSet::new(),
            fields_collected: Vec::new(),
        }
    }

    /// Parses the AcroForm tree from the document catalog dictionary.
    pub fn parse_catalog_acroform(
        mut self,
        catalog_dict: &BTreeMap<String, PdfObject>,
    ) -> PdfResult<Option<AcroForm>> {
        let acroform_obj = match catalog_dict.get("AcroForm") {
            Some(obj) => obj,
            None => return Ok(None),
        };

        let (acroform_ref, acroform_dict) = match acroform_obj {
            PdfObject::Reference(r) => {
                let resolved = self.store.resolve(*r)?.clone();
                let dict = resolved
                    .as_dict()
                    .ok_or_else(|| PdfError::TypeMismatch {
                        expected: "dictionary",
                        actual: resolved.type_name(),
                    })?
                    .clone();
                (Some(*r), dict)
            }
            PdfObject::Dictionary(dict) => (None, dict.clone()),
            _ => return Ok(None),
        };

        let need_appearances = acroform_dict
            .get("NeedAppearances")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let signature_flags = acroform_dict
            .get("SigFlags")
            .and_then(|v| v.as_integer())
            .map_or(0, |i| i.max(0) as u32);

        let default_appearance = acroform_dict.get("DA").and_then(|v| v.as_string_lossy());

        let fields_arr = match acroform_dict.get("Fields") {
            Some(obj) => self.resolve_array(obj)?,
            None => {
                return Ok(Some(AcroForm {
                    object_ref: acroform_ref,
                    fields: Vec::new(),
                    need_appearances,
                    signature_flags,
                    default_appearance,
                }))
            }
        };

        let default_inherited = InheritedAttributes {
            default_appearance: default_appearance.clone(),
            ..Default::default()
        };

        for field_item in fields_arr {
            if self.fields_collected.len() >= MAX_ACROFORM_FIELDS {
                break;
            }
            if let Some(r) = field_item.as_reference() {
                self.parse_field_node(r, None, "", &default_inherited, 0)?;
            } else if let Some(dict) = field_item.as_dict() {
                // Direct dictionary field
                let pseudo_ref = ObjectRef {
                    number: 0,
                    generation: 0,
                };
                self.process_field_dict(pseudo_ref, dict, None, "", &default_inherited, 0)?;
            }
        }

        Ok(Some(AcroForm {
            object_ref: acroform_ref,
            fields: self.fields_collected,
            need_appearances,
            signature_flags,
            default_appearance,
        }))
    }

    fn parse_field_node(
        &mut self,
        field_ref: ObjectRef,
        parent_ref: Option<ObjectRef>,
        parent_name_prefix: &str,
        inherited: &InheritedAttributes,
        depth: usize,
    ) -> PdfResult<()> {
        if depth > MAX_FIELD_TREE_DEPTH {
            return Err(PdfError::RecursionLimitExceeded);
        }

        if !self.visited_nodes.insert(field_ref) {
            // Prevent cyclic loops in malformed PDFs
            return Ok(());
        }

        if self.fields_collected.len() >= MAX_ACROFORM_FIELDS {
            return Ok(());
        }

        let field_obj = self.store.resolve(field_ref)?.clone();
        let field_dict = match field_obj.as_dict() {
            Some(d) => d.clone(),
            None => return Ok(()),
        };

        self.process_field_dict(
            field_ref,
            &field_dict,
            parent_ref,
            parent_name_prefix,
            inherited,
            depth,
        )
    }

    fn process_field_dict(
        &mut self,
        field_ref: ObjectRef,
        field_dict: &BTreeMap<String, PdfObject>,
        parent_ref: Option<ObjectRef>,
        parent_name_prefix: &str,
        inherited: &InheritedAttributes,
        depth: usize,
    ) -> PdfResult<()> {
        // Collect current level inherited values
        let current_ft = field_dict
            .get("FT")
            .and_then(|v| v.as_name())
            .map(|s| s.to_string())
            .or_else(|| inherited.field_type.clone());

        let current_flags = field_dict
            .get("Ff")
            .and_then(|v| v.as_integer())
            .map(|i| i.max(0) as u32)
            .or(inherited.flags)
            .unwrap_or(0);

        let current_da = field_dict
            .get("DA")
            .and_then(|v| v.as_string_lossy())
            .or_else(|| inherited.default_appearance.clone());

        let current_quadding = field_dict
            .get("Q")
            .and_then(|v| v.as_integer())
            .map(|i| i as i32)
            .or(inherited.quadding);

        let current_options = self
            .parse_options(field_dict)
            .or_else(|| inherited.options.clone());

        let current_inherited = InheritedAttributes {
            field_type: current_ft.clone(),
            flags: Some(current_flags),
            default_appearance: current_da.clone(),
            quadding: current_quadding,
            options: current_options.clone(),
        };

        // Partial name /T
        let partial_name = field_dict
            .get("T")
            .and_then(|v| v.as_string_lossy())
            .unwrap_or_default();

        let fully_qualified_name = if parent_name_prefix.is_empty() {
            partial_name.clone()
        } else if partial_name.is_empty() {
            parent_name_prefix.to_string()
        } else {
            format!("{parent_name_prefix}.{partial_name}")
        };

        // Check for /Kids
        let kids_arr = if let Some(kids_obj) = field_dict.get("Kids") {
            Some(self.resolve_array(kids_obj)?)
        } else {
            None
        };

        if let Some(kids) = kids_arr {
            // Determine if kids are child fields (have /T) or widget annotations (no /T)
            let mut subfield_refs = Vec::new();
            let mut widget_refs = Vec::new();

            for kid_obj in kids {
                let kid_ref = match kid_obj.as_reference() {
                    Some(r) => r,
                    None => continue,
                };
                let kid_resolved = self.store.resolve(kid_ref)?.clone();
                if let Some(kid_dict) = kid_resolved.as_dict() {
                    if kid_dict.contains_key("T") {
                        subfield_refs.push(kid_ref);
                    } else {
                        widget_refs.push(kid_ref);
                    }
                }
            }

            if !subfield_refs.is_empty() {
                // Non-terminal field node with sub-fields
                for child_ref in subfield_refs {
                    self.parse_field_node(
                        child_ref,
                        Some(field_ref),
                        &fully_qualified_name,
                        &current_inherited,
                        depth + 1,
                    )?;
                }
                return Ok(());
            }

            // Terminal field with multiple widget annotations
            let mut widgets = Vec::new();
            for w_ref in widget_refs {
                let w_resolved = self.store.resolve(w_ref)?.clone();
                if let Some(w_dict) = w_resolved.as_dict() {
                    if let Some(w) = self.parse_widget(w_ref, w_dict, Some(field_ref))? {
                        widgets.push(w);
                    }
                }
            }

            self.emit_form_field(
                field_ref,
                parent_ref,
                partial_name,
                fully_qualified_name,
                field_dict,
                &current_inherited,
                widgets,
            )?;
        } else {
            // Terminal field with single / direct widget
            let mut widgets = Vec::new();
            if let Some(w) = self.parse_widget(field_ref, field_dict, parent_ref)? {
                widgets.push(w);
            }

            self.emit_form_field(
                field_ref,
                parent_ref,
                partial_name,
                fully_qualified_name,
                field_dict,
                &current_inherited,
                widgets,
            )?;
        }

        Ok(())
    }

    fn emit_form_field(
        &mut self,
        object_ref: ObjectRef,
        parent_ref: Option<ObjectRef>,
        partial_name: String,
        fully_qualified_name: String,
        dict: &BTreeMap<String, PdfObject>,
        inherited: &InheritedAttributes,
        widgets: Vec<WidgetAnnotation>,
    ) -> PdfResult<()> {
        let flags = inherited.flags.unwrap_or(0);
        let field_type = self.resolve_field_type(inherited.field_type.as_deref(), flags);

        let alternate_name = dict.get("TU").and_then(|v| v.as_string_lossy());
        let mapping_name = dict.get("TM").and_then(|v| v.as_string_lossy());

        let value = self.parse_field_value(dict.get("V"), &field_type)?;
        let default_value = self.parse_field_value(dict.get("DV"), &field_type)?;

        let is_read_only = (flags & (1 << 0)) != 0; // Bit 1
        let is_required = (flags & (1 << 1)) != 0; // Bit 2

        self.fields_collected.push(FormField {
            object_ref,
            parent_ref,
            field_type,
            partial_name,
            fully_qualified_name,
            alternate_name,
            mapping_name,
            value,
            default_value,
            flags,
            default_appearance: inherited.default_appearance.clone(),
            quadding: inherited.quadding,
            options: inherited.options.clone().unwrap_or_default(),
            widgets,
            is_read_only,
            is_required,
        });

        Ok(())
    }

    fn resolve_field_type(&self, ft_opt: Option<&str>, flags: u32) -> FieldType {
        match ft_opt {
            Some("Tx") => {
                let multiline = (flags & (1 << 12)) != 0; // Bit 13
                let password = (flags & (1 << 13)) != 0; // Bit 14
                FieldType::Text {
                    multiline,
                    password,
                }
            }
            Some("Btn") => {
                if (flags & (1 << 16)) != 0 {
                    // Bit 17: Radio
                    FieldType::RadioButtonGroup
                } else if (flags & (1 << 15)) != 0 {
                    // Bit 16: Pushbutton
                    FieldType::PushButton
                } else {
                    FieldType::Checkbox
                }
            }
            Some("Ch") => {
                let combo = (flags & (1 << 17)) != 0; // Bit 18: Combo
                let multi_select = (flags & (1 << 21)) != 0; // Bit 22: MultiSelect
                FieldType::Choice {
                    combo,
                    multi_select,
                }
            }
            Some("Sig") => FieldType::Signature,
            Some(other) => FieldType::Unknown(other.to_string()),
            None => FieldType::Unknown("Unknown".to_string()),
        }
    }

    fn parse_field_value(
        &mut self,
        val_obj: Option<&PdfObject>,
        field_type: &FieldType,
    ) -> PdfResult<FieldValue> {
        let val_obj = match val_obj {
            Some(v) => v,
            None => return Ok(FieldValue::None),
        };

        let direct_obj = match val_obj {
            PdfObject::Reference(r) => self.store.resolve(*r)?.clone(),
            other => other.clone(),
        };

        match (field_type, &direct_obj) {
            (FieldType::Text { .. }, PdfObject::String(bytes)) => {
                Ok(FieldValue::Text(String::from_utf8_lossy(bytes).to_string()))
            }
            (FieldType::Text { .. }, PdfObject::Name(n)) => Ok(FieldValue::Text(n.clone())),
            (FieldType::Checkbox, PdfObject::Name(n)) => {
                let is_checked = n != "Off";
                Ok(FieldValue::Boolean(is_checked))
            }
            (FieldType::Checkbox, PdfObject::Bool(b)) => Ok(FieldValue::Boolean(*b)),
            (FieldType::RadioButtonGroup, PdfObject::Name(n)) => Ok(FieldValue::Name(n.clone())),
            (
                FieldType::Choice {
                    multi_select: true, ..
                },
                PdfObject::Array(arr),
            ) => {
                let mut opts = Vec::new();
                for item in arr {
                    if let Some(s) = item.as_string_lossy() {
                        opts.push(s);
                    }
                }
                Ok(FieldValue::Choice(opts))
            }
            (FieldType::Choice { .. }, PdfObject::String(bytes)) => Ok(FieldValue::Choice(vec![
                String::from_utf8_lossy(bytes).to_string(),
            ])),
            (FieldType::Choice { .. }, PdfObject::Name(n)) => {
                Ok(FieldValue::Choice(vec![n.clone()]))
            }
            (_, PdfObject::Name(n)) => Ok(FieldValue::Name(n.clone())),
            (_, PdfObject::String(bytes)) => {
                Ok(FieldValue::Text(String::from_utf8_lossy(bytes).to_string()))
            }
            _ => Ok(FieldValue::None),
        }
    }

    fn parse_options(&mut self, dict: &BTreeMap<String, PdfObject>) -> Option<Vec<ChoiceOption>> {
        let opt_obj = dict.get("Opt")?;
        let opt_arr = match opt_obj {
            PdfObject::Array(arr) => arr.clone(),
            PdfObject::Reference(r) => {
                if let Ok(resolved) = self.store.resolve(*r) {
                    if let Some(arr) = resolved.as_array() {
                        arr.to_vec()
                    } else {
                        return None;
                    }
                } else {
                    return None;
                }
            }
            _ => return None,
        };

        let mut options = Vec::new();
        for item in opt_arr.iter().take(MAX_OPTIONS_COUNT) {
            match item {
                PdfObject::String(bytes) => {
                    let s = String::from_utf8_lossy(bytes).to_string();
                    options.push(ChoiceOption {
                        export_value: s.clone(),
                        display_value: s,
                    });
                }
                PdfObject::Name(n) => {
                    options.push(ChoiceOption {
                        export_value: n.clone(),
                        display_value: n.clone(),
                    });
                }
                PdfObject::Array(pair) if pair.len() >= 2 => {
                    let export_val = pair[0].as_string_lossy().unwrap_or_default();
                    let display_val = pair[1]
                        .as_string_lossy()
                        .unwrap_or_else(|| export_val.clone());
                    options.push(ChoiceOption {
                        export_value: export_val,
                        display_value: display_val,
                    });
                }
                _ => {}
            }
        }
        Some(options)
    }

    fn parse_widget(
        &mut self,
        widget_ref: ObjectRef,
        dict: &BTreeMap<String, PdfObject>,
        parent_ref: Option<ObjectRef>,
    ) -> PdfResult<Option<WidgetAnnotation>> {
        // If dict specifies /Subtype, ensure it is /Widget if present
        if let Some(st) = dict.get("Subtype").and_then(|v| v.as_name()) {
            if st != "Widget" {
                return Ok(None);
            }
        }

        // Rect [x1, y1, x2, y2]
        let rect = match dict.get("Rect") {
            Some(obj) => self.parse_rect(obj)?,
            None => [0.0, 0.0, 0.0, 0.0],
        };

        // Page association /P
        let page_index = dict
            .get("P")
            .and_then(|v| v.as_reference())
            .and_then(|p_ref| self.page_ref_to_index.get(&p_ref).copied());

        let appearance_state = dict
            .get("AS")
            .and_then(|v| v.as_name())
            .map(|s| s.to_string());

        let flags = dict
            .get("F")
            .and_then(|v| v.as_integer())
            .map_or(0, |i| i.max(0) as u32);

        // Normal appearance states (/AP /N keys)
        let normal_appearance_states = self.extract_ap_normal_states(dict.get("AP"))?;

        Ok(Some(WidgetAnnotation {
            object_ref: widget_ref,
            page_index,
            rect,
            appearance_state,
            normal_appearance_states,
            flags,
            parent_ref,
        }))
    }

    fn extract_ap_normal_states(&mut self, ap_obj: Option<&PdfObject>) -> PdfResult<Vec<String>> {
        let ap_obj = match ap_obj {
            Some(v) => v,
            None => return Ok(Vec::new()),
        };

        let ap_dict = match ap_obj {
            PdfObject::Reference(r) => {
                let resolved = self.store.resolve(*r)?;
                match resolved.as_dict() {
                    Some(d) => d.clone(),
                    None => return Ok(Vec::new()),
                }
            }
            PdfObject::Dictionary(d) => d.clone(),
            _ => return Ok(Vec::new()),
        };

        let n_obj = match ap_dict.get("N") {
            Some(v) => v,
            None => return Ok(Vec::new()),
        };

        let n_dict = match n_obj {
            PdfObject::Reference(r) => {
                let resolved = self.store.resolve(*r)?;
                match resolved.as_dict() {
                    Some(d) => d.clone(),
                    None => return Ok(Vec::new()),
                }
            }
            PdfObject::Dictionary(d) => d.clone(),
            _ => return Ok(Vec::new()),
        };

        let mut states = Vec::new();
        for key in n_dict.keys() {
            states.push(key.clone());
        }
        Ok(states)
    }

    fn parse_rect(&mut self, obj: &PdfObject) -> PdfResult<[f64; 4]> {
        let arr = match obj {
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

        let x1 = arr[0].as_real().unwrap_or(0.0);
        let y1 = arr[1].as_real().unwrap_or(0.0);
        let x2 = arr[2].as_real().unwrap_or(0.0);
        let y2 = arr[3].as_real().unwrap_or(0.0);

        Ok([x1, y1, x2, y2])
    }

    fn resolve_array(&mut self, obj: &PdfObject) -> PdfResult<Vec<PdfObject>> {
        match obj {
            PdfObject::Array(arr) => Ok(arr.clone()),
            PdfObject::Reference(r) => {
                let resolved = self.store.resolve(*r)?;
                Ok(resolved.as_array().map(|s| s.to_vec()).unwrap_or_default())
            }
            _ => Ok(Vec::new()),
        }
    }
}
