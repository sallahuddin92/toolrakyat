use std::collections::{BTreeMap, BTreeSet};

use crate::annotation::generator::MAX_ANNOTATION_CONTENTS_LEN;
use crate::annotation::generator::{AnnotationAppearance, AnnotationGenerator};
use crate::annotation::types::{AnnotationSpec, AnnotationUpdateSpec};
use crate::appearance::choice::{ChoiceAppearance, MAX_LIST_OPTIONS, MAX_MULTI_SELECT_INDEXES};
use crate::appearance::da_parser::DefaultAppearance;
use crate::appearance::generator::AppearanceGenerator;
use crate::appearance::rotation::WidgetRotation;
use crate::appearance::status::AppearanceStatus;
use crate::appearance::text_field::{TextFieldAppearance, TextLayoutOptions, MAX_COMB_CELLS};
use crate::document::object_store::ObjectStore;
use crate::error::{PdfError, PdfResult};
use crate::font::appearance::{AppearanceFont, AppearanceFontResolver, GlyphMappingQuality};
use crate::font::subset::TrueTypeSubsetter;
use crate::font::SfntFont;
use crate::forms::field::FieldType;
use crate::mutation::change::PdfChange;
use crate::mutation::result::MutationPlan;
use crate::syntax::object::{ObjectRef, PdfObject, StreamObject};

const MAX_MUTATIONS_PER_BATCH: usize = 500;
const MAX_FIELD_VALUE_LEN: usize = 1_048_576; // 1 MB
const MAX_GENERATED_OBJECTS: usize = 2_000;
const MAX_WIDGET_REFS_PER_CHANGE: usize = 2_000;
const MAX_STATE_NAME_LEN: usize = 256;
const MAX_PDF_OBJECT_NUMBER: u64 = 9_999_999_999;
const MAX_SUBSET_FONT_RESOURCES_PER_MUTATION: usize = 64;
const MAX_PAGE_ROTATION_ANCESTORS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SubsetCacheKey {
    source_ref: Option<ObjectRef>,
    source_checksum: u64,
    resource_name: String,
    base_font: String,
    composite: bool,
    glyph_ids: Vec<u16>,
}

pub struct MutationEngine<'a, 'b> {
    store: &'a mut ObjectStore<'b>,
    page_refs: Vec<ObjectRef>,
    next_alloc_obj_num: u64,
    subset_cache: BTreeMap<SubsetCacheKey, AppearanceFont>,
    mapping_quality: Option<GlyphMappingQuality>,
}

impl<'a, 'b> MutationEngine<'a, 'b> {
    pub fn new(store: &'a mut ObjectStore<'b>, page_refs: &[ObjectRef]) -> Self {
        let max_num = store.xref().entries.keys().copied().max().unwrap_or(0);
        Self {
            store,
            page_refs: page_refs.to_vec(),
            next_alloc_obj_num: max_num.saturating_add(1),
            subset_cache: BTreeMap::new(),
            mapping_quality: None,
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
                    self.mutate_choice(
                        *field_ref,
                        std::slice::from_ref(value),
                        &mut modified_objects,
                    )?;
                    overall_status =
                        overall_status.combine(AppearanceStatus::AppearanceRegenerated);
                }
                PdfChange::SetChoiceValues { field_ref, values } => {
                    if values.is_empty() || values.len() > MAX_MULTI_SELECT_INDEXES {
                        return Err(PdfError::InvalidOperation(format!(
                            "Choice selection must contain 1..={MAX_MULTI_SELECT_INDEXES} values"
                        )));
                    }
                    if values.iter().any(|value| value.len() > MAX_FIELD_VALUE_LEN) {
                        return Err(PdfError::InvalidOperation(
                            "Choice field value exceeds maximum permitted length".into(),
                        ));
                    }
                    self.mutate_choice(*field_ref, values, &mut modified_objects)?;
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
                    let status =
                        self.mutate_update_annotation(*annot_ref, update, &mut modified_objects)?;
                    overall_status = overall_status.combine(status);
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
            glyph_mapping_quality: self.mapping_quality,
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
        let password = (flags & (1 << 13)) != 0;
        let comb = (flags & (1 << 24)) != 0;
        if comb && (multiline || password) {
            return Err(PdfError::InvalidOperation(
                "Comb text fields cannot be multiline or password fields".into(),
            ));
        }
        let comb_max_len = if comb {
            let raw = dict
                .get("MaxLen")
                .and_then(PdfObject::as_integer)
                .ok_or_else(|| {
                    PdfError::InvalidOperation("Comb text field requires a positive /MaxLen".into())
                })?;
            let value = usize::try_from(raw).map_err(|_| {
                PdfError::InvalidOperation("Comb /MaxLen must be a positive integer".into())
            })?;
            if value == 0 || value > MAX_COMB_CELLS {
                return Err(PdfError::InvalidOperation(format!(
                    "Comb /MaxLen must be within 1..={MAX_COMB_CELLS}"
                )));
            }
            Some(value)
        } else {
            None
        };

        let rendered_value = if password {
            "*".repeat(value.chars().count())
        } else {
            value.to_string()
        };

        let widget_refs = self.widget_refs_from_field(&dict)?;
        let appearance_targets = if widget_refs.is_empty() {
            vec![field_ref]
        } else {
            widget_refs
        };
        let field_properties = dict.clone();
        modified.insert(field_ref, PdfObject::Dictionary(dict));

        // 3. Regenerate each widget's /AP. Terminal fields can either be their own
        // widget or own separate widget annotations through /Kids.
        for widget_ref in appearance_targets {
            let mut widget_dict = self.get_dict_for_modification(widget_ref, modified)?;
            let rect = self.extract_rect_from_dict(&widget_dict)?;
            if rect[2] <= rect[0] || rect[3] <= rect[1] {
                continue;
            }
            let font_context = self.font_context_for_widget(&field_properties, &widget_dict)?;
            let glyph_text = Self::appearance_glyph_text(&rendered_value);
            let resolved_font = AppearanceFontResolver::resolve(
                self.store,
                &font_context,
                &self.page_refs,
                &da.font_name,
                &glyph_text,
            )?;
            let resolved_font =
                self.prepare_appearance_font(resolved_font, &glyph_text, modified)?;
            let rotation = self.resolve_widget_rotation(&widget_dict)?;
            let layout_rect = rotation.layout_rect(rect)?;
            let mut stream = TextFieldAppearance::generate_stream_with_font(
                layout_rect,
                &rendered_value,
                &da,
                quadding,
                TextLayoutOptions {
                    multiline,
                    comb_max_len,
                },
                Some(&resolved_font),
            )?;
            rotation.apply_to_stream(rect, &mut stream)?;
            widget_dict.insert(
                "AP".to_string(),
                PdfObject::Dictionary(BTreeMap::from([(
                    "N".to_string(),
                    PdfObject::Stream(stream),
                )])),
            );
            modified.insert(widget_ref, PdfObject::Dictionary(widget_dict));
        }
        Ok(())
    }

    fn mutate_checkbox(
        &mut self,
        field_ref: ObjectRef,
        widget_refs: &[ObjectRef],
        checked: bool,
        modified: &mut BTreeMap<ObjectRef, PdfObject>,
    ) -> PdfResult<()> {
        let mut field_dict = self.get_dict_for_modification(field_ref, modified)?;
        let discovered_widgets = self.widget_refs_from_field(&field_dict)?;
        let effective_widgets: Vec<ObjectRef> = if !discovered_widgets.is_empty()
            && (widget_refs.is_empty() || (widget_refs.len() == 1 && widget_refs[0] == field_ref))
        {
            discovered_widgets
        } else {
            widget_refs.to_vec()
        };
        let on_state = self.determine_widget_on_state(field_ref, &effective_widgets)?;
        let state_name = if checked { on_state.as_str() } else { "Off" };
        field_dict.insert("V".to_string(), PdfObject::Name(state_name.to_string()));

        let rect = self.extract_rect_from_dict(&field_dict)?;
        if rect[2] > rect[0] && rect[3] > rect[1] {
            let da = DefaultAppearance::default();
            let rotation = self.resolve_widget_rotation(&field_dict)?;
            let layout_rect = rotation.layout_rect(rect)?;
            let (mut ap_obj, _) = AppearanceGenerator::generate_widget_ap(
                &FieldType::Checkbox,
                layout_rect,
                state_name,
                &da,
                0,
                Some(&on_state),
                checked,
            )?;
            Self::rotate_appearance_object(&mut ap_obj, rect, rotation)?;
            field_dict.insert("AP".to_string(), ap_obj);
            field_dict.insert("AS".to_string(), PdfObject::Name(state_name.to_string()));
        } else if field_dict.contains_key("AS")
            || effective_widgets.is_empty()
            || effective_widgets.contains(&field_ref)
        {
            field_dict.insert("AS".to_string(), PdfObject::Name(state_name.to_string()));
        }
        modified.insert(field_ref, PdfObject::Dictionary(field_dict));

        // Mutate associated widget annotations
        for &w_ref in &effective_widgets {
            if w_ref == field_ref {
                continue;
            }
            let mut w_dict = self.get_dict_for_modification(w_ref, modified)?;
            w_dict.insert("AS".to_string(), PdfObject::Name(state_name.to_string()));

            let w_rect = self.extract_rect_from_dict(&w_dict)?;
            if w_rect[2] > w_rect[0] && w_rect[3] > w_rect[1] {
                let da = DefaultAppearance::default();
                let rotation = self.resolve_widget_rotation(&w_dict)?;
                let layout_rect = rotation.layout_rect(w_rect)?;
                let (mut ap_obj, _) = AppearanceGenerator::generate_widget_ap(
                    &FieldType::Checkbox,
                    layout_rect,
                    state_name,
                    &da,
                    0,
                    Some(&on_state),
                    checked,
                )?;
                Self::rotate_appearance_object(&mut ap_obj, w_rect, rotation)?;
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

                let k_rect = self.extract_rect_from_dict(&kid_dict)?;
                if k_rect[2] > k_rect[0] && k_rect[3] > k_rect[1] {
                    let da = DefaultAppearance::default();
                    let rotation = self.resolve_widget_rotation(&kid_dict)?;
                    let layout_rect = rotation.layout_rect(k_rect)?;
                    let (mut ap_obj, _) = AppearanceGenerator::generate_widget_ap(
                        &FieldType::RadioButtonGroup,
                        layout_rect,
                        state_to_set,
                        &da,
                        0,
                        Some(on_state),
                        is_selected,
                    )?;
                    Self::rotate_appearance_object(&mut ap_obj, k_rect, rotation)?;
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
        values: &[String],
        modified: &mut BTreeMap<ObjectRef, PdfObject>,
    ) -> PdfResult<()> {
        let mut dict = self.get_dict_for_modification(field_ref, modified)?;
        let flags = dict
            .get("Ff")
            .and_then(PdfObject::as_integer)
            .map_or(0, |value| value.max(0) as u32);
        let combo = (flags & (1 << 17)) != 0;
        let multi_select = (flags & (1 << 21)) != 0;
        if values.len() > 1 && (!multi_select || combo) {
            return Err(PdfError::InvalidOperation(
                "Multiple values require a multi-select list box".into(),
            ));
        }
        let options = self.resolve_choice_options(&dict)?;
        if !combo && options.is_empty() {
            return Err(PdfError::InvalidOperation(
                "List box is missing required /Opt entries".into(),
            ));
        }
        let mut selected = Vec::with_capacity(values.len());
        let mut display_values = Vec::with_capacity(values.len());
        for value in values {
            let matched = options
                .iter()
                .enumerate()
                .find_map(|(index, (export, display))| {
                    (export == value).then(|| (index, display.clone()))
                });
            if let Some((index, display)) = matched {
                selected.push(index);
                display_values.push(display);
            } else if combo {
                display_values.push(value.clone());
            } else {
                return Err(PdfError::InvalidOperation(format!(
                    "Choice value is not present in /Opt: {value}"
                )));
            }
        }
        selected.sort_unstable();
        selected.dedup();
        if !combo && selected.len() != values.len() {
            return Err(PdfError::InvalidOperation(
                "Choice selection contains duplicate values".into(),
            ));
        }
        let value_object = if values.len() == 1 {
            PdfObject::String(values[0].as_bytes().to_vec())
        } else {
            PdfObject::Array(
                values
                    .iter()
                    .map(|value| PdfObject::String(value.as_bytes().to_vec()))
                    .collect(),
            )
        };
        dict.insert("V".to_string(), value_object);
        if selected.is_empty() {
            dict.remove("I");
        } else {
            dict.insert(
                "I".to_string(),
                PdfObject::Array(
                    selected
                        .iter()
                        .map(|index| PdfObject::Integer(*index as i64))
                        .collect(),
                ),
            );
        }

        let da_str = dict
            .get("DA")
            .and_then(|v| v.as_string_lossy())
            .unwrap_or_else(|| "/Helv 12 Tf 0 g".to_string());
        let da = DefaultAppearance::parse(&da_str)?;

        let quadding = dict
            .get("Q")
            .and_then(|v| v.as_integer())
            .map_or(0, |i| i as i32);

        let coverage_text = if combo {
            display_values[0].clone()
        } else {
            options
                .iter()
                .map(|(_, display)| display.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        };
        let widget_refs = self.widget_refs_from_field(&dict)?;
        let appearance_targets = if widget_refs.is_empty() {
            vec![field_ref]
        } else {
            widget_refs
        };
        let field_properties = dict.clone();
        modified.insert(field_ref, PdfObject::Dictionary(dict));

        for widget_ref in appearance_targets {
            let mut widget_dict = self.get_dict_for_modification(widget_ref, modified)?;
            let rect = self.extract_rect_from_dict(&widget_dict)?;
            if rect[2] > rect[0] && rect[3] > rect[1] {
                let font_context = self.font_context_for_widget(&field_properties, &widget_dict)?;
                let glyph_text = Self::appearance_glyph_text(&coverage_text);
                let resolved_font = AppearanceFontResolver::resolve(
                    self.store,
                    &font_context,
                    &self.page_refs,
                    &da.font_name,
                    &glyph_text,
                )?;
                let resolved_font =
                    self.prepare_appearance_font(resolved_font, &glyph_text, modified)?;
                let rotation = self.resolve_widget_rotation(&widget_dict)?;
                let layout_rect = rotation.layout_rect(rect)?;
                let mut stream = if combo {
                    TextFieldAppearance::generate_stream_with_font(
                        layout_rect,
                        &display_values[0],
                        &da,
                        quadding,
                        TextLayoutOptions::default(),
                        Some(&resolved_font),
                    )?
                } else {
                    let top_index = field_properties
                        .get("TI")
                        .and_then(PdfObject::as_integer)
                        .and_then(|value| usize::try_from(value).ok())
                        .unwrap_or(0);
                    let labels: Vec<String> =
                        options.iter().map(|(_, display)| display.clone()).collect();
                    ChoiceAppearance::generate_list_stream_with_font(
                        layout_rect,
                        &labels,
                        &selected,
                        top_index,
                        &da,
                        Some(&resolved_font),
                    )?
                };
                rotation.apply_to_stream(rect, &mut stream)?;
                widget_dict.insert(
                    "AP".to_string(),
                    PdfObject::Dictionary(BTreeMap::from([(
                        "N".to_string(),
                        PdfObject::Stream(stream),
                    )])),
                );
            }
            modified.insert(widget_ref, PdfObject::Dictionary(widget_dict));
        }
        Ok(())
    }

    fn resolve_choice_options(
        &mut self,
        dict: &BTreeMap<String, PdfObject>,
    ) -> PdfResult<Vec<(String, String)>> {
        let Some(opt) = dict.get("Opt") else {
            return Ok(Vec::new());
        };
        let resolved = self.store.resolve_object(opt)?;
        let array = resolved.as_array().ok_or_else(|| PdfError::TypeMismatch {
            expected: "array",
            actual: resolved.type_name(),
        })?;
        if array.len() > MAX_LIST_OPTIONS {
            return Err(PdfError::InvalidOperation(format!(
                "Choice options exceed maximum of {MAX_LIST_OPTIONS}"
            )));
        }
        let mut result = Vec::with_capacity(array.len());
        let mut total_bytes = 0usize;
        for item in array {
            match item {
                PdfObject::Array(pair) if pair.len() >= 2 => {
                    let export = pair[0].as_string_lossy().ok_or_else(|| {
                        PdfError::InvalidOperation("Choice export value must be a string".into())
                    })?;
                    let display = pair[1].as_string_lossy().ok_or_else(|| {
                        PdfError::InvalidOperation("Choice display value must be a string".into())
                    })?;
                    total_bytes = total_bytes
                        .checked_add(export.len())
                        .and_then(|value| value.checked_add(display.len()))
                        .ok_or_else(|| {
                            PdfError::InvalidOperation("Choice option bytes overflow".into())
                        })?;
                    result.push((export, display));
                }
                PdfObject::String(_) | PdfObject::Name(_) => {
                    let value = item.as_string_lossy().ok_or_else(|| {
                        PdfError::InvalidOperation("Choice option must be text".into())
                    })?;
                    total_bytes = total_bytes.checked_add(value.len()).ok_or_else(|| {
                        PdfError::InvalidOperation("Choice option bytes overflow".into())
                    })?;
                    result.push((value.clone(), value));
                }
                _ => {
                    return Err(PdfError::InvalidOperation(
                        "Malformed choice /Opt entry".into(),
                    ))
                }
            }
            if total_bytes > MAX_FIELD_VALUE_LEN {
                return Err(PdfError::InvalidOperation(
                    "Choice option text exceeds maximum aggregate byte limit".into(),
                ));
            }
        }
        Ok(result)
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
    ) -> PdfResult<AppearanceStatus> {
        let mut dict = self.get_dict_for_modification(annot_ref, modified)?;
        Self::validate_annotation_dictionary(&dict)?;
        let subtype = dict
            .get("Subtype")
            .and_then(PdfObject::as_name)
            .unwrap_or("")
            .to_string();

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

        if let Some(fill_color) = &update.fill_color {
            if crate::appearance::PdfColor::parse_from_slice(fill_color).is_none() {
                return Err(PdfError::InvalidOperation(
                    "Annotation interior color must contain 1, 3, or 4 finite components".into(),
                ));
            }
            dict.insert(
                "IC".to_string(),
                PdfObject::Array(fill_color.iter().map(|v| PdfObject::Real(*v)).collect()),
            );
        }

        if let Some(width) = update.border_width {
            if !width.is_finite() || !(0.0..=20.0).contains(&width) {
                return Err(PdfError::InvalidOperation(
                    "Annotation border width must be finite and within 0..=20".into(),
                ));
            }
            dict.insert(
                "BS".to_string(),
                PdfObject::Dictionary(BTreeMap::from([("W".to_string(), PdfObject::Real(width))])),
            );
            dict.insert(
                "Border".to_string(),
                PdfObject::Array(vec![
                    PdfObject::Integer(0),
                    PdfObject::Integer(0),
                    PdfObject::Real(width),
                ]),
            );
        }

        if let Some(points) = update.line_points {
            if !points.iter().all(|value| value.is_finite())
                || ((points[0] - points[2]).abs() < f64::EPSILON
                    && (points[1] - points[3]).abs() < f64::EPSILON)
            {
                return Err(PdfError::InvalidOperation(
                    "Line endpoints must be finite and distinct".into(),
                ));
            }
            dict.insert(
                "L".to_string(),
                PdfObject::Array(points.iter().map(|value| PdfObject::Real(*value)).collect()),
            );
            if update.rect.is_none() {
                let pad = update.border_width.unwrap_or(1.0).clamp(0.1, 20.0) + 8.0;
                dict.insert(
                    "Rect".to_string(),
                    PdfObject::Array(vec![
                        PdfObject::Real(points[0].min(points[2]) - pad),
                        PdfObject::Real(points[1].min(points[3]) - pad),
                        PdfObject::Real(points[0].max(points[2]) + pad),
                        PdfObject::Real(points[1].max(points[3]) + pad),
                    ]),
                );
            }
        }

        if let Some(endings) = update.line_endings {
            dict.insert(
                "LE".to_string(),
                PdfObject::Array(
                    endings
                        .iter()
                        .map(|ending| PdfObject::Name(ending.as_name().to_string()))
                        .collect(),
                ),
            );
        }

        if let Some(quad_points) = &update.quad_points {
            dict.insert(
                "QuadPoints".to_string(),
                PdfObject::Array(
                    quad_points
                        .iter()
                        .map(|value| PdfObject::Real(*value))
                        .collect(),
                ),
            );
        }

        if let Some(ink_list) = &update.ink_list {
            let paths = ink_list
                .iter()
                .map(|path| {
                    PdfObject::Array(
                        path.iter()
                            .flat_map(|point| point.iter())
                            .map(|value| PdfObject::Real(*value))
                            .collect(),
                    )
                })
                .collect();
            dict.insert("InkList".to_string(), PdfObject::Array(paths));
        }

        let visual_change = update.rect.is_some()
            || update.color.is_some()
            || update.fill_color.is_some()
            || update.border_width.is_some()
            || update.line_points.is_some()
            || update.line_endings.is_some()
            || update.quad_points.is_some()
            || update.ink_list.is_some()
            || (subtype == "FreeText" && update.contents.is_some());

        let status = if visual_change {
            match AnnotationGenerator::regenerate_from_dictionary(&dict)? {
                AnnotationAppearance::Regenerated(stream) => {
                    let stream_ref = self.allocate_object_ref()?;
                    modified.insert(stream_ref, PdfObject::Stream(stream));
                    dict.insert(
                        "AP".to_string(),
                        PdfObject::Dictionary(BTreeMap::from([(
                            "N".to_string(),
                            PdfObject::Reference(stream_ref),
                        )])),
                    );
                    AppearanceStatus::AppearanceRegenerated
                }
                AnnotationAppearance::NotRequired => AppearanceStatus::ValueUpdated,
                AnnotationAppearance::Unsupported => {
                    return Err(PdfError::InvalidOperation(format!(
                        "Unsupported appearance regeneration for annotation subtype /{subtype}"
                    )))
                }
            }
        } else if dict.contains_key("AP") {
            AppearanceStatus::AppearancePreserved
        } else {
            AppearanceStatus::ValueUpdated
        };

        modified.insert(annot_ref, PdfObject::Dictionary(dict));
        Ok(status)
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

    fn prepare_appearance_font(
        &mut self,
        resolved: AppearanceFont,
        text: &str,
        modified: &mut BTreeMap<ObjectRef, PdfObject>,
    ) -> PdfResult<AppearanceFont> {
        resolved.verify_text(text)?;
        self.mapping_quality = Some(self.mapping_quality.map_or(resolved.quality, |quality| {
            quality.combine(resolved.quality)
        }));
        let Some(source) = resolved.embedded_source.as_ref() else {
            return Ok(resolved);
        };
        let sfnt = resolved.font.embedded_sfnt.as_ref().ok_or_else(|| {
            PdfError::InvalidOperation("Embedded font source is missing validated SFNT data".into())
        })?;
        let cmap = sfnt.cmap.as_ref().ok_or_else(|| {
            PdfError::InvalidOperation(
                "Embedded TrueType appearance font is missing a usable cmap".into(),
            )
        })?;
        let mut glyph_ids = Vec::with_capacity(text.chars().count());
        for character in text.chars() {
            let glyph = cmap.map_char_to_glyph(character as u32).ok_or_else(|| {
                PdfError::InvalidOperation(format!(
                    "UNREPRESENTABLE glyph U+{:04X} in embedded appearance font",
                    character as u32
                ))
            })?;
            glyph_ids.push(glyph);
        }
        glyph_ids.sort_unstable();
        glyph_ids.dedup();
        let key = SubsetCacheKey {
            source_ref: source.source_ref,
            source_checksum: Self::font_checksum(&sfnt.data),
            resource_name: resolved.resource_name.clone(),
            base_font: resolved.font.base_font.clone(),
            composite: resolved.font.is_composite,
            glyph_ids: glyph_ids.clone(),
        };
        if let Some(cached) = self.subset_cache.get(&key) {
            return Ok(cached.clone());
        }
        if self.subset_cache.len() >= MAX_SUBSET_FONT_RESOURCES_PER_MUTATION {
            return Err(PdfError::InvalidOperation(format!(
                "Font resources per mutation exceed maximum of {MAX_SUBSET_FONT_RESOURCES_PER_MUTATION}"
            )));
        }

        let subset = TrueTypeSubsetter::subset(&sfnt.data, &glyph_ids)?;
        let stream_ref = self.allocate_object_ref()?;
        let descriptor_ref = self.allocate_object_ref()?;
        let descendant_ref = if source.descendant_dictionary.is_some() {
            Some(self.allocate_object_ref()?)
        } else {
            None
        };
        let font_ref = self.allocate_object_ref()?;
        let subset_tag = Self::subset_tag(font_ref.number);
        let base_name = Self::unprefixed_font_name(&resolved.font.base_font);
        let subset_name = format!("{subset_tag}+{base_name}");

        let mut stream_dict = source.font_stream_dictionary.clone();
        stream_dict.remove("Filter");
        stream_dict.remove("DecodeParms");
        stream_dict.insert(
            "Length".to_string(),
            PdfObject::Integer(subset.bytes.len() as i64),
        );
        stream_dict.insert(
            "Length1".to_string(),
            PdfObject::Integer(subset.bytes.len() as i64),
        );
        modified.insert(
            stream_ref,
            PdfObject::Stream(StreamObject {
                stream_offset: 0,
                stream_length: subset.bytes.len(),
                dict: stream_dict,
                data: subset.bytes.clone(),
            }),
        );

        let mut descriptor = source.descriptor_dictionary.clone();
        descriptor.remove("FontFile");
        descriptor.remove("FontFile2");
        descriptor.remove("FontFile3");
        descriptor.insert(
            source.font_file_key.clone(),
            PdfObject::Reference(stream_ref),
        );
        descriptor.insert("FontName".to_string(), PdfObject::Name(subset_name.clone()));
        modified.insert(descriptor_ref, PdfObject::Dictionary(descriptor));

        let mut top_dictionary = source.top_dictionary.clone();
        top_dictionary.insert("BaseFont".to_string(), PdfObject::Name(subset_name.clone()));
        if let (Some(mut descendant), Some(descendant_ref)) =
            (source.descendant_dictionary.clone(), descendant_ref)
        {
            descendant.insert("BaseFont".to_string(), PdfObject::Name(subset_name.clone()));
            descendant.insert(
                "FontDescriptor".to_string(),
                PdfObject::Reference(descriptor_ref),
            );
            modified.insert(descendant_ref, PdfObject::Dictionary(descendant));
            top_dictionary.insert(
                "DescendantFonts".to_string(),
                PdfObject::Array(vec![PdfObject::Reference(descendant_ref)]),
            );
        } else {
            top_dictionary.insert(
                "FontDescriptor".to_string(),
                PdfObject::Reference(descriptor_ref),
            );
        }
        modified.insert(font_ref, PdfObject::Dictionary(top_dictionary));

        let mut subset_font = resolved.font.clone();
        subset_font.base_font = subset_name;
        subset_font.embedded_sfnt = Some(SfntFont::parse(&subset.bytes)?);
        let prepared = AppearanceFont {
            resource_name: format!("SPF{subset_tag}"),
            resource_object: PdfObject::Reference(font_ref),
            font: subset_font,
            quality: resolved.quality,
            embedded_source: None,
        };
        prepared.verify_text(text)?;
        self.subset_cache.insert(key, prepared.clone());
        Ok(prepared)
    }

    fn font_context_for_widget(
        &mut self,
        field: &BTreeMap<String, PdfObject>,
        widget: &BTreeMap<String, PdfObject>,
    ) -> PdfResult<BTreeMap<String, PdfObject>> {
        let mut context = field.clone();
        if context.contains_key("DR") {
            return Ok(context);
        }
        let Some(appearance) = widget.get("AP") else {
            return Ok(context);
        };
        let appearance = self.store.resolve_object(appearance)?;
        let Some(normal) = appearance.as_dict().and_then(|dict| dict.get("N")) else {
            return Ok(context);
        };
        let normal = self.store.resolve_object(normal)?;
        if let Some(resources) = normal
            .as_stream()
            .and_then(|stream| stream.dict.get("Resources"))
        {
            context.insert("DR".to_string(), resources.clone());
            return Ok(context);
        }
        let Some(states) = normal.as_dict() else {
            return Ok(context);
        };
        if states.len() > crate::font::appearance::MAX_APPEARANCE_RESOURCES {
            return Err(PdfError::InvalidOperation(format!(
                "Appearance states exceed maximum of {}",
                crate::font::appearance::MAX_APPEARANCE_RESOURCES
            )));
        }
        for state in states.values() {
            let state = self.store.resolve_object(state)?;
            if let Some(resources) = state
                .as_stream()
                .and_then(|stream| stream.dict.get("Resources"))
            {
                context.insert("DR".to_string(), resources.clone());
                break;
            }
        }
        Ok(context)
    }

    fn appearance_glyph_text(text: &str) -> String {
        text.chars()
            .filter(|character| !matches!(character, '\r' | '\n'))
            .collect()
    }

    fn font_checksum(bytes: &[u8]) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        hash
    }

    fn subset_tag(mut number: u64) -> String {
        let mut tag = [b'A'; 6];
        for byte in tag.iter_mut().rev() {
            *byte = b'A' + (number % 26) as u8;
            number /= 26;
        }
        String::from_utf8_lossy(&tag).into_owned()
    }

    fn unprefixed_font_name(name: &str) -> &str {
        match name.split_once('+') {
            Some((prefix, suffix)) if prefix.len() == 6 => suffix,
            _ => name,
        }
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

    fn resolve_widget_rotation(
        &mut self,
        widget_dict: &BTreeMap<String, PdfObject>,
    ) -> PdfResult<WidgetRotation> {
        if let Some(page_ref) = widget_dict.get("P").and_then(PdfObject::as_reference) {
            self.validate_inherited_page_rotation(page_ref)?;
        }
        let Some(mk_object) = widget_dict.get("MK") else {
            return Ok(WidgetRotation::Degrees0);
        };
        let mk = self.store.resolve_object(mk_object)?;
        let mk = mk.as_dict().ok_or_else(|| PdfError::TypeMismatch {
            expected: "widget appearance characteristics dictionary",
            actual: mk.type_name(),
        })?;
        let rotation = mk.get("R").map_or(Ok(0), |value| {
            value.as_integer().ok_or_else(|| {
                PdfError::InvalidOperation("Widget /MK /R must be an integer".into())
            })
        })?;
        WidgetRotation::from_degrees(rotation)
    }

    fn validate_inherited_page_rotation(&mut self, page_ref: ObjectRef) -> PdfResult<()> {
        let mut current = Some(page_ref);
        let mut visited = BTreeSet::new();
        for _ in 0..MAX_PAGE_ROTATION_ANCESTORS {
            let Some(reference) = current else {
                return Ok(());
            };
            if !visited.insert(reference) {
                return Err(PdfError::CircularReference(
                    "Cycle while resolving inherited page rotation".into(),
                ));
            }
            let object = self.store.resolve(reference)?.clone();
            let dict = object.as_dict().ok_or_else(|| PdfError::TypeMismatch {
                expected: "page or page-tree dictionary",
                actual: object.type_name(),
            })?;
            if let Some(rotation) = dict.get("Rotate") {
                let degrees = rotation.as_integer().ok_or_else(|| {
                    PdfError::InvalidOperation("Page /Rotate must be an integer".into())
                })?;
                let _ = WidgetRotation::from_degrees(degrees)?;
                return Ok(());
            }
            current = dict.get("Parent").and_then(PdfObject::as_reference);
        }
        Err(PdfError::RecursionLimitExceeded)
    }

    fn rotate_appearance_object(
        appearance: &mut PdfObject,
        rect: [f64; 4],
        rotation: WidgetRotation,
    ) -> PdfResult<()> {
        let Some(ap_dict) = appearance.as_dict_mut() else {
            return Err(PdfError::TypeMismatch {
                expected: "appearance dictionary",
                actual: appearance.type_name(),
            });
        };
        let normal = ap_dict.get_mut("N").ok_or_else(|| {
            PdfError::InvalidOperation("Generated appearance is missing /N".into())
        })?;
        match normal {
            PdfObject::Stream(stream) => rotation.apply_to_stream(rect, stream),
            PdfObject::Dictionary(states) => {
                for state in states.values_mut() {
                    let actual = state.type_name();
                    let stream = state.as_stream_mut().ok_or(PdfError::TypeMismatch {
                        expected: "appearance state stream",
                        actual,
                    })?;
                    rotation.apply_to_stream(rect, stream)?;
                }
                Ok(())
            }
            other => Err(PdfError::TypeMismatch {
                expected: "normal appearance stream or state dictionary",
                actual: other.type_name(),
            }),
        }
    }

    fn widget_refs_from_field(
        &mut self,
        dict: &BTreeMap<String, PdfObject>,
    ) -> PdfResult<Vec<ObjectRef>> {
        let Some(kids_object) = dict.get("Kids") else {
            return Ok(Vec::new());
        };
        let resolved = self.store.resolve_object(kids_object)?;
        let kids = resolved.as_array().ok_or_else(|| PdfError::TypeMismatch {
            expected: "array",
            actual: resolved.type_name(),
        })?;
        if kids.len() > MAX_WIDGET_REFS_PER_CHANGE {
            return Err(PdfError::InvalidOperation(format!(
                "Field widget count exceeds maximum of {MAX_WIDGET_REFS_PER_CHANGE}"
            )));
        }
        let mut refs = Vec::with_capacity(kids.len());
        for kid in kids {
            let kid_ref = kid.as_reference().ok_or_else(|| {
                PdfError::InvalidOperation("Field /Kids entry must be an indirect reference".into())
            })?;
            refs.push(kid_ref);
        }
        Ok(refs)
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
