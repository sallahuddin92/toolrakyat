use std::collections::{BTreeMap, BTreeSet};

use crate::annotation::generator::{AnnotationAppearance, AnnotationGenerator};
use crate::annotation::generator::{MAX_ANNOTATION_APPEARANCE_BYTES, MAX_ANNOTATION_CONTENTS_LEN};
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
use crate::font::planner::ShapedFallbackData;
use crate::font::subset::TrueTypeSubsetter;
use crate::font::{
    plan_adaptive_text, FontFamily, FontProgramKind, FontStyle, SfntFont, Type0FontEmbedder,
};
use crate::forms::field::FieldType;
use crate::mutation::change::PdfChange;
use crate::mutation::result::MutationPlan;
use crate::mutation::text_edit::{ContentStreamEditor, LayoutPolicyResult, TextEditTarget};
use crate::syntax::object::{ObjectRef, PdfObject, StreamObject};

const MAX_MUTATIONS_PER_BATCH: usize = 500;
const MAX_FIELD_VALUE_LEN: usize = 1_048_576; // 1 MB
const MAX_GENERATED_OBJECTS: usize = 2_000;
const MAX_WIDGET_REFS_PER_CHANGE: usize = 2_000;
const MAX_STATE_NAME_LEN: usize = 256;
const MAX_PDF_OBJECT_NUMBER: u64 = 9_999_999_999;
const MAX_SUBSET_FONT_RESOURCES_PER_MUTATION: usize = 64;
const MAX_PAGE_ROTATION_ANCESTORS: usize = 64;
const MAX_FIELD_PROPERTY_ANCESTORS: usize = 32;

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
        let mut regenerated_form_appearance = false;
        let mut last_layout_result = None;

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
                    regenerated_form_appearance = true;
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
                    regenerated_form_appearance = true;
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
                    regenerated_form_appearance = true;
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
                    regenerated_form_appearance = true;
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
                    regenerated_form_appearance = true;
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
                PdfChange::ReplaceText {
                    page_index,
                    target,
                    replacement,
                } => {
                    let (status, layout) = self.mutate_replace_text(
                        *page_index,
                        target,
                        replacement,
                        &mut modified_objects,
                    )?;
                    overall_status = overall_status.combine(status);
                    if last_layout_result.is_none() {
                        last_layout_result = Some(layout);
                    }
                }
                PdfChange::ReplaceTextGroup {
                    page_index,
                    targets,
                    replacement,
                } => {
                    let (status, layout) = self.mutate_replace_text_group(
                        *page_index,
                        targets,
                        replacement,
                        &mut modified_objects,
                    )?;
                    overall_status = overall_status.combine(status);
                    if last_layout_result.is_none() {
                        last_layout_result = Some(layout);
                    }
                }
                PdfChange::MoveText {
                    page_index,
                    target,
                    dx,
                    dy,
                } => {
                    let status = self.mutate_move_text_group(
                        *page_index,
                        std::slice::from_ref(target),
                        *dx,
                        *dy,
                        &mut modified_objects,
                    )?;
                    overall_status = overall_status.combine(status);
                }
                PdfChange::MoveTextGroup {
                    page_index,
                    targets,
                    dx,
                    dy,
                } => {
                    let status = self.mutate_move_text_group(
                        *page_index,
                        targets,
                        *dx,
                        *dy,
                        &mut modified_objects,
                    )?;
                    overall_status = overall_status.combine(status);
                }

                PdfChange::ReplaceImage { spec } => {
                    let plan = crate::image::ImageEditor::replace_image(
                        self.store,
                        &self.page_refs,
                        &mut self.next_alloc_obj_num,
                        spec,
                    )?;
                    modified_objects.extend(plan.modified_objects);
                    overall_status = overall_status.combine(plan.appearance_status);
                }
                PdfChange::AddImage { spec } => {
                    let plan = crate::image::ImageEditor::add_image(
                        self.store,
                        &self.page_refs,
                        &mut self.next_alloc_obj_num,
                        spec,
                    )?;
                    modified_objects.extend(plan.modified_objects);
                    overall_status = overall_status.combine(plan.appearance_status);
                }
                PdfChange::UpdateImage { spec } => {
                    let plan = crate::image::ImageEditor::update_image(
                        self.store,
                        &self.page_refs,
                        &mut self.next_alloc_obj_num,
                        spec,
                    )?;
                    modified_objects.extend(plan.modified_objects);
                    overall_status = overall_status.combine(plan.appearance_status);
                }
                PdfChange::RemoveImage { spec } => {
                    let plan =
                        crate::image::ImageEditor::remove_image(self.store, &self.page_refs, spec)?;
                    modified_objects.extend(plan.modified_objects);
                    overall_status = overall_status.combine(plan.appearance_status);
                }

                PdfChange::UpdateVectorGraphic { spec } => {
                    let plan = crate::vector::VectorEditor::update_graphic(
                        self.store,
                        &self.page_refs,
                        &mut self.next_alloc_obj_num,
                        spec,
                    )?;
                    modified_objects.extend(plan.modified_objects);
                    overall_status = overall_status.combine(plan.appearance_status);
                }
                PdfChange::AddVectorGraphic { spec } => {
                    let plan = crate::vector::VectorEditor::add_graphic(
                        self.store,
                        &self.page_refs,
                        spec,
                    )?;
                    modified_objects.extend(plan.modified_objects);
                    overall_status = overall_status.combine(plan.appearance_status);
                }
                PdfChange::DeleteVectorGraphic { spec } => {
                    let plan = crate::vector::VectorEditor::delete_graphic(
                        self.store,
                        &self.page_refs,
                        &mut self.next_alloc_obj_num,
                        spec,
                    )?;
                    modified_objects.extend(plan.modified_objects);
                    overall_status = overall_status.combine(plan.appearance_status);
                }
            }

            if modified_objects.len() > MAX_GENERATED_OBJECTS {
                return Err(PdfError::InvalidOperation(format!(
                    "Exceeded maximum generated objects limit of {MAX_GENERATED_OBJECTS} in single transaction"
                )));
            }
        }

        if regenerated_form_appearance {
            self.reconcile_need_appearances(&mut modified_objects)?;
        }

        Ok(MutationPlan {
            modified_objects,
            appearance_status: overall_status,
            glyph_mapping_quality: self.mapping_quality,
            layout_policy_result: last_layout_result,
        })
    }

    fn mutate_replace_text(
        &mut self,
        page_index: usize,
        target: &TextEditTarget,
        replacement: &str,
        modified: &mut BTreeMap<ObjectRef, PdfObject>,
    ) -> PdfResult<(AppearanceStatus, LayoutPolicyResult)> {
        if page_index >= self.page_refs.len() {
            return Err(PdfError::PageNotFound(page_index));
        }
        let page_ref = self.page_refs[page_index];
        let page_obj = self.store.resolve(page_ref)?.clone();
        let mut page_dict = page_obj
            .as_dict()
            .ok_or_else(|| PdfError::TypeMismatch {
                expected: "page dictionary",
                actual: page_obj.type_name(),
            })?
            .clone();

        // 1. Resolve page resources and fonts
        let mut resources =
            crate::font::resource::PageResources::resolve_for_page(&page_dict, self.store)?;

        // 2. Resolve target content stream
        let contents_obj = page_dict
            .get("Contents")
            .ok_or_else(|| PdfError::TargetTextNotFound("Page missing /Contents".to_string()))?;
        let resolved_contents = self.store.resolve_object(contents_obj)?;

        let (target_stream_ref, mut target_stream_obj, is_direct_page_stream) =
            match resolved_contents {
                PdfObject::Stream(s) => {
                    if target.stream_index != 0 {
                        return Err(PdfError::TargetTextNotFound(format!(
                            "Page has 1 content stream, requested stream index {}",
                            target.stream_index
                        )));
                    }
                    if let Some(r) = contents_obj.as_reference() {
                        (Some(r), s, false)
                    } else {
                        (None, s, true)
                    }
                }
                PdfObject::Array(arr) => {
                    if target.stream_index >= arr.len() {
                        return Err(PdfError::TargetTextNotFound(format!(
                            "Page has {} content streams, requested stream index {}",
                            arr.len(),
                            target.stream_index
                        )));
                    }
                    let stream_item = &arr[target.stream_index];
                    let stream_ref = stream_item.as_reference().ok_or_else(|| {
                        PdfError::InvalidOperation(
                            "Page /Contents array items must be indirect references".into(),
                        )
                    })?;
                    let resolved_stream = self.store.resolve_object(stream_item)?;
                    let s = resolved_stream
                        .as_stream()
                        .ok_or_else(|| PdfError::TypeMismatch {
                            expected: "stream",
                            actual: resolved_stream.type_name(),
                        })?;
                    (Some(stream_ref), s.clone(), false)
                }
                other => {
                    return Err(PdfError::TypeMismatch {
                        expected: "stream or array of streams",
                        actual: other.type_name(),
                    });
                }
            };

        // 3. Decompress original stream data
        let decompress_limits = crate::filter::limits::DecompressLimits::default();
        let decompressed_data = match target_stream_obj
            .dict
            .get("Filter")
            .and_then(PdfObject::as_name)
        {
            Some("FlateDecode") => crate::filter::flate::FlateDecoder::decode(
                &target_stream_obj.data,
                &decompress_limits,
            )?,
            Some(other) => {
                return Err(PdfError::InvalidOperation(format!(
                    "Unsupported stream filter /{other}"
                )));
            }
            None => target_stream_obj.data.clone(),
        };

        // 4. Parse content stream instructions and find font at target instruction
        let mut parser = crate::content::ContentParser::from_bytes(&decompressed_data);
        let instructions = parser.parse_instructions()?;
        if target.instruction_index >= instructions.len() {
            return Err(PdfError::TargetTextNotFound(format!(
                "Instruction index {} out of bounds (stream has {} instructions)",
                target.instruction_index,
                instructions.len()
            )));
        }

        // Find active font and text state in content stream up to instruction_index
        let mut active_font_name: Option<String> = None;
        let mut active_font_size = 12.0;
        let mut char_spacing = 0.0;
        let mut word_spacing = 0.0;
        let mut horiz_scaling = 100.0;

        for instr in &instructions[0..=target.instruction_index] {
            match instr.operator {
                crate::content::ContentOperator::Tf if instr.operands.len() >= 2 => {
                    active_font_name = instr.operands[0].as_name().map(ToString::to_string);
                    active_font_size = instr.operands[1].as_f64().unwrap_or(12.0);
                }
                crate::content::ContentOperator::Tc if !instr.operands.is_empty() => {
                    char_spacing = instr.operands[0].as_f64().unwrap_or(0.0);
                }
                crate::content::ContentOperator::Tw if !instr.operands.is_empty() => {
                    word_spacing = instr.operands[0].as_f64().unwrap_or(0.0);
                }
                crate::content::ContentOperator::Tz if !instr.operands.is_empty() => {
                    horiz_scaling = instr.operands[0].as_f64().unwrap_or(100.0);
                }
                _ => {}
            }
        }

        let fallback_font = crate::font::font::Font::standard_fallback("Helvetica");
        let orig_font = active_font_name
            .as_deref()
            .and_then(|name| resources.get_font(name))
            .cloned()
            .unwrap_or(fallback_font);
        let orig_style = orig_font.style;

        // Check if complex script shaping is required
        for ch in replacement.chars() {
            if crate::font::font::is_complex_script_char(ch) {
                return Err(PdfError::UnsupportedComplexScript(format!(
                    "Character U+{:04X} requires complex script shaping",
                    ch as u32
                )));
            }
        }

        // Determine effective font for replacement:
        // Priority 1: Keep original font if it can encode the requested text
        // Priority 2: Use a compatible existing font from page resources matching style
        // Priority 3: Inject and use a standard style-matched Type1 font
        let mut font_switch: Option<(String, f64, String)> = None;
        let effective_font = if orig_font.can_encode_text(replacement) {
            orig_font.clone()
        } else if let Some((comp_name, comp_font)) =
            resources.find_compatible_font(&orig_style, replacement)
        {
            let orig_name = active_font_name
                .as_deref()
                .unwrap_or(&orig_font.name)
                .to_string();
            let comp_name_str = comp_name.to_string();
            let comp_font_clone = comp_font.clone();
            font_switch = Some((comp_name_str, active_font_size, orig_name));
            comp_font_clone
        } else {
            let font_ref = self.allocate_object_ref()?;
            let std_res_name = resources.ensure_standard_font(
                &orig_style,
                &mut page_dict,
                font_ref,
                self.store,
                modified,
            )?;
            let std_font = resources
                .get_font(&std_res_name)
                .cloned()
                .unwrap_or_else(|| {
                    crate::font::font::Font::standard_with_style(&std_res_name, &orig_style)
                });
            if !std_font.can_encode_text(replacement) {
                let missing = std_font.missing_chars(replacement);
                return Err(PdfError::UnsupportedFontEncoding(format!(
                    "Character(s) {:?} cannot be encoded by font /{} or compatible fallback",
                    missing, orig_font.base_font
                )));
            }
            let orig_name = active_font_name
                .as_deref()
                .unwrap_or(&orig_font.name)
                .to_string();
            font_switch = Some((std_res_name, active_font_size, orig_name));
            std_font
        };

        let encoded_replacement = effective_font.encode_text(replacement)?;

        // 6. Evaluate layout / width policy & downstream dependencies
        let target_instr = &instructions[target.instruction_index];
        let original_bytes = match target_instr.operator {
            crate::content::ContentOperator::Tj => target_instr
                .operands
                .first()
                .and_then(crate::content::ContentOperand::as_bytes)
                .unwrap_or(&[]),
            crate::content::ContentOperator::TJ => {
                let items = target_instr
                    .operands
                    .first()
                    .and_then(crate::content::ContentOperand::as_array);
                if let Some(arr) = items {
                    if target.operand_index < arr.len() {
                        arr[target.operand_index].as_bytes().unwrap_or(&[])
                    } else {
                        &[]
                    }
                } else {
                    &[]
                }
            }
            _ => &[],
        };

        let decoded_original = orig_font.decode_bytes(original_bytes);
        let orig_text: String = decoded_original.into_iter().map(|(s, _)| s).collect();

        let orig_width = orig_font.calculate_text_width(
            &orig_text,
            active_font_size,
            char_spacing,
            word_spacing,
            horiz_scaling,
        )?;
        let new_width = effective_font.calculate_text_width(
            replacement,
            active_font_size,
            char_spacing,
            word_spacing,
            horiz_scaling,
        )?;

        // Check if downstream text in the same text block depends on this text advance
        let has_downstream_tj_elements =
            if target_instr.operator == crate::content::ContentOperator::TJ {
                target_instr
                    .operands
                    .first()
                    .and_then(crate::content::ContentOperand::as_array)
                    .is_some_and(|arr| target.operand_index + 1 < arr.len())
            } else {
                false
            };

        let mut has_dependent_downstream_instructions = false;
        for instr in &instructions[(target.instruction_index + 1)..] {
            match instr.operator {
                crate::content::ContentOperator::Et
                | crate::content::ContentOperator::Bt
                | crate::content::ContentOperator::Tm
                | crate::content::ContentOperator::Td
                | crate::content::ContentOperator::TD
                | crate::content::ContentOperator::TStar => {
                    break;
                }
                crate::content::ContentOperator::Tj
                | crate::content::ContentOperator::TJ
                | crate::content::ContentOperator::Quote
                | crate::content::ContentOperator::DoubleQuote => {
                    has_dependent_downstream_instructions = true;
                    break;
                }
                _ => {}
            }
        }

        let has_dependent_downstream =
            has_downstream_tj_elements || has_dependent_downstream_instructions;
        let mut compensation = None;

        let layout_result = if (orig_width - new_width).abs() < 0.01 {
            LayoutPolicyResult::ExactFit
        } else if has_dependent_downstream {
            if new_width > orig_width + 0.5 {
                return Err(PdfError::UnsupportedLayout(
                    "Other text in this PDF depends on the spacing of this text run.".into(),
                ));
            }
            // Compensate exact difference using TJ numeric displacement so downstream text does not move
            let scale_factor = active_font_size * (horiz_scaling / 100.0);
            if scale_factor > 0.0001 {
                let n_comp = ((new_width - orig_width) / scale_factor) * 1000.0;
                compensation = Some(n_comp);
            }
            LayoutPolicyResult::FitWithinOriginalBox {
                original_width: orig_width,
                new_width,
            }
        } else if new_width <= orig_width + 0.5 {
            LayoutPolicyResult::FitWithinOriginalBox {
                original_width: orig_width,
                new_width,
            }
        } else {
            if new_width > orig_width * 3.0 && (new_width - orig_width) > 150.0 {
                return Err(PdfError::UnsupportedLayout(format!(
                    "Replacement text advance ({new_width:.1}pt) significantly exceeds original width ({orig_width:.1}pt) and would require complex line reflow"
                )));
            }
            LayoutPolicyResult::WidthChanged {
                original_width: orig_width,
                new_width,
            }
        };

        // 7. Mutate content stream with optional font switch
        let modified_decompressed =
            ContentStreamEditor::replace_multiple_in_stream_with_font_switch(
                &decompressed_data,
                &[(target, &encoded_replacement)],
                compensation,
                font_switch
                    .as_ref()
                    .map(|(n, s, o)| (n.as_str(), *s, o.as_str())),
            )?;

        // 8. Re-compress or update stream data
        let final_stream_data = if target_stream_obj.dict.contains_key("Filter") {
            miniz_oxide::deflate::compress_to_vec_zlib(&modified_decompressed, 6)
        } else {
            modified_decompressed
        };

        target_stream_obj.data = final_stream_data;
        target_stream_obj.stream_length = target_stream_obj.data.len();
        target_stream_obj.dict.insert(
            "Length".to_string(),
            PdfObject::Integer(target_stream_obj.data.len() as i64),
        );

        // Check if stream is shared with other pages
        let mut count_referencing_pages = 0;
        if let Some(r) = target_stream_ref {
            for &other_page_ref in &self.page_refs {
                if let Ok(other_obj) = self.store.resolve(other_page_ref) {
                    if let Some(other_dict) = other_obj.as_dict() {
                        if let Some(c) = other_dict.get("Contents") {
                            if let Some(cr) = c.as_reference() {
                                if cr == r {
                                    count_referencing_pages += 1;
                                }
                            } else if let Some(arr) = c.as_array() {
                                for item in arr {
                                    if item.as_reference() == Some(r) {
                                        count_referencing_pages += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if count_referencing_pages > 1 {
            // Stream is shared with other pages: clone into a new indirect stream for this page
            let new_stream_ref = ObjectRef::new(self.next_alloc_obj_num, 0);
            self.next_alloc_obj_num = self.next_alloc_obj_num.saturating_add(1);

            modified.insert(new_stream_ref, PdfObject::Stream(target_stream_obj));

            // Update page Contents to point to new_stream_ref
            if let Some(contents_entry) = page_dict.get_mut("Contents") {
                match contents_entry {
                    PdfObject::Reference(_) => {
                        *contents_entry = PdfObject::Reference(new_stream_ref);
                    }
                    PdfObject::Array(arr) => {
                        if target.stream_index < arr.len() {
                            arr[target.stream_index] = PdfObject::Reference(new_stream_ref);
                        }
                    }
                    _ => {}
                }
            }
            modified.insert(page_ref, PdfObject::Dictionary(page_dict));
        } else if let Some(r) = target_stream_ref {
            modified.insert(r, PdfObject::Stream(target_stream_obj));
        } else if is_direct_page_stream {
            page_dict.insert("Contents".to_string(), PdfObject::Stream(target_stream_obj));
            modified.insert(page_ref, PdfObject::Dictionary(page_dict));
        }

        Ok((AppearanceStatus::ValueUpdated, layout_result))
    }

    fn mutate_replace_text_group(
        &mut self,
        page_index: usize,
        targets: &[TextEditTarget],
        replacement: &str,
        modified: &mut BTreeMap<ObjectRef, PdfObject>,
    ) -> PdfResult<(AppearanceStatus, LayoutPolicyResult)> {
        if targets.is_empty() {
            return Err(PdfError::TargetTextNotFound(
                "Empty text targets for group replacement".into(),
            ));
        }
        if targets.len() == 1 {
            return self.mutate_replace_text(page_index, &targets[0], replacement, modified);
        }

        if page_index >= self.page_refs.len() {
            return Err(PdfError::PageNotFound(page_index));
        }
        let page_ref = self.page_refs[page_index];
        let page_obj = self.store.resolve(page_ref)?.clone();
        let mut page_dict = page_obj
            .as_dict()
            .ok_or_else(|| PdfError::TypeMismatch {
                expected: "page dictionary",
                actual: page_obj.type_name(),
            })?
            .clone();

        // 1. Resolve page resources and fonts
        let mut resources =
            crate::font::resource::PageResources::resolve_for_page(&page_dict, self.store)?;

        // 2. Validate all targets share same stream_index
        let stream_index = targets[0].stream_index;
        for t in targets {
            if t.stream_index != stream_index {
                return Err(PdfError::InvalidOperation(
                    "MULTI_SPAN_ACROSS_DIFFERENT_STREAMS".into(),
                ));
            }
        }

        // 3. Resolve target content stream
        let contents_obj = page_dict
            .get("Contents")
            .ok_or_else(|| PdfError::TargetTextNotFound("Page missing /Contents".to_string()))?;
        let resolved_contents = self.store.resolve_object(contents_obj)?;

        let (target_stream_ref, mut target_stream_obj, is_direct_page_stream) =
            match resolved_contents {
                PdfObject::Stream(s) => {
                    if stream_index != 0 {
                        return Err(PdfError::TargetTextNotFound(format!(
                            "Page has 1 content stream, requested stream index {stream_index}"
                        )));
                    }
                    if let Some(r) = contents_obj.as_reference() {
                        (Some(r), s, false)
                    } else {
                        (None, s, true)
                    }
                }
                PdfObject::Array(arr) => {
                    if stream_index >= arr.len() {
                        return Err(PdfError::TargetTextNotFound(format!(
                            "Page has {} content streams, requested stream index {stream_index}",
                            arr.len()
                        )));
                    }
                    let stream_item = &arr[stream_index];
                    let stream_ref = stream_item.as_reference().ok_or_else(|| {
                        PdfError::InvalidOperation(
                            "Page /Contents array items must be indirect references".into(),
                        )
                    })?;
                    let resolved_stream = self.store.resolve_object(stream_item)?;
                    let s = resolved_stream
                        .as_stream()
                        .ok_or_else(|| PdfError::TypeMismatch {
                            expected: "stream",
                            actual: resolved_stream.type_name(),
                        })?;
                    (Some(stream_ref), s.clone(), false)
                }
                other => {
                    return Err(PdfError::TypeMismatch {
                        expected: "stream or array of streams",
                        actual: other.type_name(),
                    });
                }
            };

        // 4. Decompress original stream data
        let decompress_limits = crate::filter::limits::DecompressLimits::default();
        let decompressed_data = match target_stream_obj
            .dict
            .get("Filter")
            .and_then(PdfObject::as_name)
        {
            Some("FlateDecode") => crate::filter::flate::FlateDecoder::decode(
                &target_stream_obj.data,
                &decompress_limits,
            )?,
            Some(other) => {
                return Err(PdfError::InvalidOperation(format!(
                    "Unsupported stream filter /{other}"
                )));
            }
            None => target_stream_obj.data.clone(),
        };

        // 5. Parse content stream instructions and find font at target instructions
        let mut parser = crate::content::ContentParser::from_bytes(&decompressed_data);
        let instructions = parser.parse_instructions()?;

        for t in targets {
            if t.instruction_index >= instructions.len() {
                return Err(PdfError::TargetTextNotFound(format!(
                    "Instruction index {} out of bounds (stream has {} instructions)",
                    t.instruction_index,
                    instructions.len()
                )));
            }
        }

        // Find active font and text state in content stream up to first target instruction
        let first_target = &targets[0];
        let mut active_font_name: Option<String> = None;
        let mut active_font_size = 12.0;
        let mut char_spacing = 0.0;
        let mut word_spacing = 0.0;
        let mut horiz_scaling = 100.0;

        for instr in &instructions[0..=first_target.instruction_index] {
            match instr.operator {
                crate::content::ContentOperator::Tf if instr.operands.len() >= 2 => {
                    active_font_name = instr.operands[0].as_name().map(ToString::to_string);
                    active_font_size = instr.operands[1].as_f64().unwrap_or(12.0);
                }
                crate::content::ContentOperator::Tc if !instr.operands.is_empty() => {
                    char_spacing = instr.operands[0].as_f64().unwrap_or(0.0);
                }
                crate::content::ContentOperator::Tw if !instr.operands.is_empty() => {
                    word_spacing = instr.operands[0].as_f64().unwrap_or(0.0);
                }
                crate::content::ContentOperator::Tz if !instr.operands.is_empty() => {
                    horiz_scaling = instr.operands[0].as_f64().unwrap_or(100.0);
                }
                _ => {}
            }
        }

        let fallback_font = crate::font::font::Font::standard_fallback("Helvetica");
        let orig_font = active_font_name
            .as_deref()
            .and_then(|name| resources.get_font(name))
            .cloned()
            .unwrap_or(fallback_font);
        let orig_style = orig_font.style;

        // Check if complex script shaping is required
        for ch in replacement.chars() {
            if crate::font::font::is_complex_script_char(ch) {
                return Err(PdfError::UnsupportedComplexScript(format!(
                    "Character U+{:04X} requires complex script shaping",
                    ch as u32
                )));
            }
        }

        // Determine effective font for replacement:
        let mut font_switch: Option<(String, f64, String)> = None;
        let effective_font = if orig_font.can_encode_text(replacement) {
            orig_font.clone()
        } else if let Some((comp_name, comp_font)) =
            resources.find_compatible_font(&orig_style, replacement)
        {
            let orig_name = active_font_name
                .as_deref()
                .unwrap_or(&orig_font.name)
                .to_string();
            let comp_name_str = comp_name.to_string();
            let comp_font_clone = comp_font.clone();
            font_switch = Some((comp_name_str, active_font_size, orig_name));
            comp_font_clone
        } else {
            let font_ref = self.allocate_object_ref()?;
            let std_res_name = resources.ensure_standard_font(
                &orig_style,
                &mut page_dict,
                font_ref,
                self.store,
                modified,
            )?;
            let std_font = resources
                .get_font(&std_res_name)
                .cloned()
                .unwrap_or_else(|| {
                    crate::font::font::Font::standard_with_style(&std_res_name, &orig_style)
                });
            if !std_font.can_encode_text(replacement) {
                let missing = std_font.missing_chars(replacement);
                return Err(PdfError::UnsupportedFontEncoding(format!(
                    "Character(s) {:?} cannot be encoded by font /{} or compatible fallback",
                    missing, orig_font.base_font
                )));
            }
            let orig_name = active_font_name
                .as_deref()
                .unwrap_or(&orig_font.name)
                .to_string();
            font_switch = Some((std_res_name, active_font_size, orig_name));
            std_font
        };

        let encoded_replacement = effective_font.encode_text(replacement)?;

        // 7. Calculate combined original text advance across all targets including intermediate TJ adjustments
        let mut orig_total_advance = 0.0;
        let mut last_instr_index = 0;
        let mut last_operand_index = 0;

        for t in targets {
            last_instr_index = last_instr_index.max(t.instruction_index);
            last_operand_index = last_operand_index.max(t.operand_index);

            let target_instr = &instructions[t.instruction_index];
            let original_bytes = match target_instr.operator {
                crate::content::ContentOperator::Tj => target_instr
                    .operands
                    .first()
                    .and_then(crate::content::ContentOperand::as_bytes)
                    .unwrap_or(&[]),
                crate::content::ContentOperator::TJ => {
                    let items = target_instr
                        .operands
                        .first()
                        .and_then(crate::content::ContentOperand::as_array);
                    if let Some(arr) = items {
                        if t.operand_index < arr.len() {
                            arr[t.operand_index].as_bytes().unwrap_or(&[])
                        } else {
                            &[]
                        }
                    } else {
                        &[]
                    }
                }
                _ => &[],
            };
            let decoded = orig_font.decode_bytes(original_bytes);
            let mut span_text = String::new();
            for (s, _) in decoded {
                span_text.push_str(&s);
            }
            let span_width = orig_font.calculate_text_width(
                &span_text,
                active_font_size,
                char_spacing,
                word_spacing,
                horiz_scaling,
            )?;
            orig_total_advance += span_width;
        }

        // Account for any TJ numeric adjustments between the first and last target in the same TJ instruction
        if first_target.instruction_index == last_instr_index {
            let instr = &instructions[first_target.instruction_index];
            if instr.operator == crate::content::ContentOperator::TJ {
                if let Some(arr) = instr
                    .operands
                    .first()
                    .and_then(crate::content::ContentOperand::as_array)
                {
                    let start_op = first_target.operand_index;
                    let end_op = last_operand_index.min(arr.len().saturating_sub(1));
                    for op_idx in start_op..=end_op {
                        match &arr[op_idx] {
                            crate::content::ContentOperand::Integer(n) => {
                                let adj = -(*n as f64 / 1000.0)
                                    * active_font_size
                                    * (horiz_scaling / 100.0);
                                orig_total_advance += adj;
                            }
                            crate::content::ContentOperand::Real(r) => {
                                let adj =
                                    -(*r / 1000.0) * active_font_size * (horiz_scaling / 100.0);
                                orig_total_advance += adj;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        let new_width = effective_font.calculate_text_width(
            replacement,
            active_font_size,
            char_spacing,
            word_spacing,
            horiz_scaling,
        )?;

        // Check if downstream text in the same text block depends on this text advance
        let last_target = targets.last().unwrap();
        let last_target_instr = &instructions[last_target.instruction_index];
        let has_downstream_tj_elements =
            if last_target_instr.operator == crate::content::ContentOperator::TJ {
                last_target_instr
                    .operands
                    .first()
                    .and_then(crate::content::ContentOperand::as_array)
                    .is_some_and(|arr| last_target.operand_index + 1 < arr.len())
            } else {
                false
            };

        let mut has_dependent_downstream_instructions = false;
        for instr in &instructions[(last_instr_index + 1)..] {
            match instr.operator {
                crate::content::ContentOperator::Et
                | crate::content::ContentOperator::Bt
                | crate::content::ContentOperator::Tm
                | crate::content::ContentOperator::Td
                | crate::content::ContentOperator::TD
                | crate::content::ContentOperator::TStar => {
                    break;
                }
                crate::content::ContentOperator::Tj
                | crate::content::ContentOperator::TJ
                | crate::content::ContentOperator::Quote
                | crate::content::ContentOperator::DoubleQuote => {
                    has_dependent_downstream_instructions = true;
                    break;
                }
                _ => {}
            }
        }

        let has_dependent_downstream =
            has_downstream_tj_elements || has_dependent_downstream_instructions;
        let mut compensation = None;

        let layout_result = if (orig_total_advance - new_width).abs() < 0.01 {
            LayoutPolicyResult::ExactFit
        } else if has_dependent_downstream {
            if new_width > orig_total_advance + 0.5 {
                return Err(PdfError::UnsupportedLayout(
                    "Other text in this PDF depends on the spacing of this text run.".into(),
                ));
            }
            // Compensate exact difference using TJ numeric displacement so downstream text does not move
            let scale_factor = active_font_size * (horiz_scaling / 100.0);
            if scale_factor > 0.0001 {
                let n_comp = ((new_width - orig_total_advance) / scale_factor) * 1000.0;
                compensation = Some(n_comp);
            }
            LayoutPolicyResult::FitWithinOriginalBox {
                original_width: orig_total_advance,
                new_width,
            }
        } else if new_width <= orig_total_advance + 0.5 {
            LayoutPolicyResult::FitWithinOriginalBox {
                original_width: orig_total_advance,
                new_width,
            }
        } else {
            if new_width > orig_total_advance * 3.0 && (new_width - orig_total_advance) > 150.0 {
                return Err(PdfError::UnsupportedLayout(format!(
                    "Replacement text advance ({new_width:.1}pt) significantly exceeds original width ({orig_total_advance:.1}pt) and would require complex line reflow"
                )));
            }
            LayoutPolicyResult::WidthChanged {
                original_width: orig_total_advance,
                new_width,
            }
        };

        // 8. Prepare batch edits: first target gets encoded_replacement, remaining targets get empty bytes
        let mut edits = Vec::with_capacity(targets.len());
        edits.push((&targets[0], encoded_replacement.as_slice()));
        for t in &targets[1..] {
            edits.push((t, &[] as &[u8]));
        }

        let modified_decompressed =
            ContentStreamEditor::replace_multiple_in_stream_with_font_switch(
                &decompressed_data,
                &edits,
                compensation,
                font_switch
                    .as_ref()
                    .map(|(n, s, o)| (n.as_str(), *s, o.as_str())),
            )?;

        // 9. Re-compress or update stream data
        let final_stream_data = if target_stream_obj.dict.contains_key("Filter") {
            miniz_oxide::deflate::compress_to_vec_zlib(&modified_decompressed, 6)
        } else {
            modified_decompressed
        };

        target_stream_obj.data = final_stream_data;
        target_stream_obj.stream_length = target_stream_obj.data.len();
        target_stream_obj.dict.insert(
            "Length".to_string(),
            PdfObject::Integer(target_stream_obj.data.len() as i64),
        );

        // Check if stream is shared with other pages
        let mut count_referencing_pages = 0;
        if let Some(r) = target_stream_ref {
            for &other_page_ref in &self.page_refs {
                if let Ok(other_obj) = self.store.resolve(other_page_ref) {
                    if let Some(other_dict) = other_obj.as_dict() {
                        if let Some(c) = other_dict.get("Contents") {
                            if let Some(cr) = c.as_reference() {
                                if cr == r {
                                    count_referencing_pages += 1;
                                }
                            } else if let Some(arr) = c.as_array() {
                                for item in arr {
                                    if item.as_reference() == Some(r) {
                                        count_referencing_pages += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if count_referencing_pages > 1 {
            let new_stream_ref = ObjectRef::new(self.next_alloc_obj_num, 0);
            self.next_alloc_obj_num = self.next_alloc_obj_num.saturating_add(1);

            modified.insert(new_stream_ref, PdfObject::Stream(target_stream_obj));

            if let Some(contents_entry) = page_dict.get_mut("Contents") {
                match contents_entry {
                    PdfObject::Reference(_) => {
                        *contents_entry = PdfObject::Reference(new_stream_ref);
                    }
                    PdfObject::Array(arr) => {
                        if stream_index < arr.len() {
                            arr[stream_index] = PdfObject::Reference(new_stream_ref);
                        }
                    }
                    _ => {}
                }
            }
            modified.insert(page_ref, PdfObject::Dictionary(page_dict));
        } else if let Some(r) = target_stream_ref {
            modified.insert(r, PdfObject::Stream(target_stream_obj));
        } else if is_direct_page_stream {
            page_dict.insert("Contents".to_string(), PdfObject::Stream(target_stream_obj));
            modified.insert(page_ref, PdfObject::Dictionary(page_dict));
        }

        Ok((AppearanceStatus::ValueUpdated, layout_result))
    }

    fn mutate_move_text_group(
        &mut self,
        page_index: usize,
        targets: &[TextEditTarget],
        dx: f64,
        dy: f64,
        modified: &mut BTreeMap<ObjectRef, PdfObject>,
    ) -> PdfResult<AppearanceStatus> {
        if targets.is_empty() {
            return Err(PdfError::TargetTextNotFound(
                "Empty text targets for group move".into(),
            ));
        }

        if page_index >= self.page_refs.len() {
            return Err(PdfError::PageNotFound(page_index));
        }
        let page_ref = self.page_refs[page_index];
        let page_obj = self.store.resolve(page_ref)?.clone();
        let mut page_dict = page_obj
            .as_dict()
            .ok_or_else(|| PdfError::TypeMismatch {
                expected: "page dictionary",
                actual: page_obj.type_name(),
            })?
            .clone();

        // 1. Validate all targets share same stream_index
        let stream_index = targets[0].stream_index;
        for t in targets {
            if t.stream_index != stream_index {
                return Err(PdfError::InvalidOperation(
                    "MULTI_SPAN_ACROSS_DIFFERENT_STREAMS".into(),
                ));
            }
        }

        // 2. Resolve target content stream
        let contents_obj = page_dict
            .get("Contents")
            .ok_or_else(|| PdfError::TargetTextNotFound("Page missing /Contents".to_string()))?;
        let resolved_contents = self.store.resolve_object(contents_obj)?;

        let (target_stream_ref, mut target_stream_obj, is_direct_page_stream) =
            match resolved_contents {
                PdfObject::Stream(s) => {
                    if stream_index != 0 {
                        return Err(PdfError::TargetTextNotFound(format!(
                            "Page has 1 content stream, requested stream index {stream_index}"
                        )));
                    }
                    if let Some(r) = contents_obj.as_reference() {
                        (Some(r), s, false)
                    } else {
                        (None, s, true)
                    }
                }
                PdfObject::Array(arr) => {
                    if stream_index >= arr.len() {
                        return Err(PdfError::TargetTextNotFound(format!(
                            "Page has {} content streams, requested stream index {stream_index}",
                            arr.len()
                        )));
                    }
                    let stream_item = &arr[stream_index];
                    let stream_ref = stream_item.as_reference().ok_or_else(|| {
                        PdfError::InvalidOperation(
                            "Page /Contents array items must be indirect references".into(),
                        )
                    })?;
                    let resolved_stream = self.store.resolve_object(stream_item)?;
                    let s = resolved_stream
                        .as_stream()
                        .ok_or_else(|| PdfError::TypeMismatch {
                            expected: "stream",
                            actual: resolved_stream.type_name(),
                        })?;
                    (Some(stream_ref), s.clone(), false)
                }
                other => {
                    return Err(PdfError::TypeMismatch {
                        expected: "stream or array of streams",
                        actual: other.type_name(),
                    });
                }
            };

        // 3. Decompress original stream data
        let decompress_limits = crate::filter::limits::DecompressLimits::default();
        let decompressed_data = match target_stream_obj
            .dict
            .get("Filter")
            .and_then(PdfObject::as_name)
        {
            Some("FlateDecode") => crate::filter::flate::FlateDecoder::decode(
                &target_stream_obj.data,
                &decompress_limits,
            )?,
            Some(other) => {
                return Err(PdfError::InvalidOperation(format!(
                    "Unsupported stream filter /{other}"
                )));
            }
            None => target_stream_obj.data.clone(),
        };

        let target_refs: Vec<&TextEditTarget> = targets.iter().collect();
        let modified_decompressed =
            ContentStreamEditor::move_multiple_in_stream(&decompressed_data, &target_refs, dx, dy)?;

        // 4. Re-compress or update stream data
        let final_stream_data = if target_stream_obj.dict.contains_key("Filter") {
            miniz_oxide::deflate::compress_to_vec_zlib(&modified_decompressed, 6)
        } else {
            modified_decompressed
        };

        target_stream_obj.data = final_stream_data;
        target_stream_obj.stream_length = target_stream_obj.data.len();
        target_stream_obj.dict.insert(
            "Length".to_string(),
            PdfObject::Integer(target_stream_obj.data.len() as i64),
        );

        // Check if stream is shared with other pages
        let mut count_referencing_pages = 0;
        if let Some(r) = target_stream_ref {
            for &other_page_ref in &self.page_refs {
                if let Ok(other_obj) = self.store.resolve(other_page_ref) {
                    if let Some(other_dict) = other_obj.as_dict() {
                        if let Some(c) = other_dict.get("Contents") {
                            if let Some(cr) = c.as_reference() {
                                if cr == r {
                                    count_referencing_pages += 1;
                                }
                            } else if let Some(arr) = c.as_array() {
                                for item in arr {
                                    if item.as_reference() == Some(r) {
                                        count_referencing_pages += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if count_referencing_pages > 1 {
            let new_stream_ref = ObjectRef::new(self.next_alloc_obj_num, 0);
            self.next_alloc_obj_num = self.next_alloc_obj_num.saturating_add(1);

            modified.insert(new_stream_ref, PdfObject::Stream(target_stream_obj));

            if let Some(contents_entry) = page_dict.get_mut("Contents") {
                match contents_entry {
                    PdfObject::Reference(_) => {
                        *contents_entry = PdfObject::Reference(new_stream_ref);
                    }
                    PdfObject::Array(arr) => {
                        if stream_index < arr.len() {
                            arr[stream_index] = PdfObject::Reference(new_stream_ref);
                        }
                    }
                    _ => {}
                }
            }
            modified.insert(page_ref, PdfObject::Dictionary(page_dict));
        } else if let Some(r) = target_stream_ref {
            modified.insert(r, PdfObject::Stream(target_stream_obj));
        } else if is_direct_page_stream {
            page_dict.insert("Contents".to_string(), PdfObject::Stream(target_stream_obj));
            modified.insert(page_ref, PdfObject::Dictionary(page_dict));
        }

        Ok(AppearanceStatus::ValueUpdated)
    }

    fn mutate_text_field(
        &mut self,
        field_ref: ObjectRef,
        value: &str,
        modified: &mut BTreeMap<ObjectRef, PdfObject>,
    ) -> PdfResult<()> {
        let mut dict = self.get_dict_for_modification(field_ref, modified)?;
        let mut effective = self.effective_field_dictionary(field_ref, modified)?;

        // 1. Update /V
        dict.insert(
            "V".to_string(),
            PdfObject::String(value.as_bytes().to_vec()),
        );
        effective.insert(
            "V".to_string(),
            PdfObject::String(value.as_bytes().to_vec()),
        );

        // 2. Parse field properties
        let da_str = effective
            .get("DA")
            .and_then(|v| v.as_string_lossy())
            .unwrap_or_else(|| "/Helv 12 Tf 0 g".to_string());
        let da = DefaultAppearance::parse(&da_str)?;

        let quadding = effective
            .get("Q")
            .and_then(|v| v.as_integer())
            .map_or(0, |i| i as i32);

        let flags = effective
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
            let raw = effective
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

        let widget_refs = self.widget_refs_from_field(&effective)?;
        let appearance_targets = if widget_refs.is_empty() {
            vec![field_ref]
        } else {
            widget_refs
        };
        let field_properties = effective;
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
            let generated = PdfObject::Dictionary(BTreeMap::from([(
                "N".to_string(),
                PdfObject::Stream(stream),
            )]));
            let merged = self.merge_generated_appearance(widget_dict.get("AP"), &generated)?;
            widget_dict.insert("AP".to_string(), merged);
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
        let effective_field = self.effective_field_dictionary(field_ref, modified)?;
        let discovered_widgets = self.widget_refs_from_field(&effective_field)?;
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
            let merged = self.merge_generated_appearance(field_dict.get("AP"), &ap_obj)?;
            field_dict.insert("AP".to_string(), merged);
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
                let merged = self.merge_generated_appearance(w_dict.get("AP"), &ap_obj)?;
                w_dict.insert("AP".to_string(), merged);
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
                    let merged = self.merge_generated_appearance(kid_dict.get("AP"), &ap_obj)?;
                    kid_dict.insert("AP".to_string(), merged);
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
        let mut effective = self.effective_field_dictionary(field_ref, modified)?;
        let flags = effective
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
        let options = self.resolve_choice_options(&effective)?;
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
        effective.insert(
            "V".to_string(),
            dict.get("V").cloned().ok_or_else(|| {
                PdfError::InvalidOperation("Choice value was not constructed".into())
            })?,
        );
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

        let da_str = effective
            .get("DA")
            .and_then(|v| v.as_string_lossy())
            .unwrap_or_else(|| "/Helv 12 Tf 0 g".to_string());
        let da = DefaultAppearance::parse(&da_str)?;

        let quadding = effective
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
        let widget_refs = self.widget_refs_from_field(&effective)?;
        let appearance_targets = if widget_refs.is_empty() {
            vec![field_ref]
        } else {
            widget_refs
        };
        let field_properties = effective;
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
                let generated = PdfObject::Dictionary(BTreeMap::from([(
                    "N".to_string(),
                    PdfObject::Stream(stream),
                )]));
                let merged = self.merge_generated_appearance(widget_dict.get("AP"), &generated)?;
                widget_dict.insert("AP".to_string(), merged);
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
        let (mut annot_dict, mut stream_opt) =
            AnnotationGenerator::generate_annotation_objects(spec)?;
        annot_dict.insert("P".to_string(), PdfObject::Reference(page_ref));

        if stream_opt.is_none()
            && annot_dict.get("Subtype").and_then(PdfObject::as_name) == Some("FreeText")
        {
            stream_opt = Some(self.generate_adaptive_freetext_appearance(&annot_dict, modified)?);
        }

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
            dict.insert("Contents".to_string(), PdfObject::text_string(contents));
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
                    let generated = PdfObject::Dictionary(BTreeMap::from([(
                        "N".to_string(),
                        PdfObject::Reference(stream_ref),
                    )]));
                    let merged = self.merge_generated_appearance(dict.get("AP"), &generated)?;
                    dict.insert("AP".to_string(), merged);
                    AppearanceStatus::AppearanceRegenerated
                }
                AnnotationAppearance::NotRequired => AppearanceStatus::ValueUpdated,
                AnnotationAppearance::Unsupported => {
                    if subtype == "FreeText" {
                        let stream = self.generate_adaptive_freetext_appearance(&dict, modified)?;
                        let stream_ref = self.allocate_object_ref()?;
                        modified.insert(stream_ref, PdfObject::Stream(stream));
                        let generated = PdfObject::Dictionary(BTreeMap::from([(
                            "N".to_string(),
                            PdfObject::Reference(stream_ref),
                        )]));
                        let merged = self.merge_generated_appearance(dict.get("AP"), &generated)?;
                        dict.insert("AP".to_string(), merged);
                        AppearanceStatus::AppearanceRegenerated
                    } else {
                        return Err(PdfError::InvalidOperation(format!(
                            "Unsupported appearance regeneration for annotation subtype /{subtype}"
                        )));
                    }
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

    fn generate_adaptive_freetext_appearance(
        &mut self,
        dict: &BTreeMap<String, PdfObject>,
        modified: &mut BTreeMap<ObjectRef, PdfObject>,
    ) -> PdfResult<StreamObject> {
        let text = dict
            .get("Contents")
            .and_then(PdfObject::as_string_lossy)
            .unwrap_or_default();
        if text.is_empty() {
            return Err(PdfError::UnsupportedFontEncoding(
                "Adaptive FreeText appearance requires non-empty Unicode text".into(),
            ));
        }
        let rect = dict
            .get("Rect")
            .and_then(PdfObject::as_array)
            .filter(|values| values.len() == 4)
            .ok_or_else(|| {
                PdfError::InvalidOperation("FreeText is missing a valid /Rect".into())
            })?;
        let number = |value: &PdfObject| match value {
            PdfObject::Integer(v) => Some(*v as f64),
            PdfObject::Real(v) => Some(*v),
            _ => None,
        };
        let coords: Vec<f64> = rect.iter().filter_map(number).collect();
        if coords.len() != 4 || !coords.iter().all(|value| value.is_finite()) {
            return Err(PdfError::InvalidOperation(
                "FreeText /Rect must contain four finite numbers".into(),
            ));
        }
        let width = coords[2] - coords[0];
        let height = coords[3] - coords[1];
        if width <= 0.0 || height <= 0.0 {
            return Err(PdfError::InvalidOperation(
                "FreeText /Rect must have positive bounds".into(),
            ));
        }

        let da = dict
            .get("DA")
            .and_then(PdfObject::as_string_lossy)
            .and_then(|value| DefaultAppearance::parse(&value).ok())
            .unwrap_or_default();
        let font_size = da.font_size.clamp(6.0, 72.0);
        let style = FontStyle {
            family: FontFamily::SansSerif,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
        };
        let plan = plan_adaptive_text(&text, &style, font_size)?.ok_or_else(|| {
            PdfError::UnsupportedFontEncoding(
                "No qualified adaptive font covers the complete FreeText value".into(),
            )
        })?;
        if plan.fallback_runs.len() != plan.runs.len() {
            return Err(PdfError::InvalidOperation(
                "Adaptive FreeText run metadata is inconsistent".into(),
            ));
        }

        let mut merged_by_font: BTreeMap<String, ShapedFallbackData> = BTreeMap::new();
        for fallback in &plan.fallback_runs {
            if let Some(existing) = merged_by_font.get_mut(&fallback.font_id) {
                existing
                    .requested_glyphs
                    .extend_from_slice(&fallback.requested_glyphs);
                existing.requested_glyphs.sort_unstable();
                existing.requested_glyphs.dedup();
                existing.cid_to_gid.extend(fallback.cid_to_gid.clone());
                existing
                    .cid_to_unicode
                    .extend(fallback.cid_to_unicode.clone());
                existing.glyph_widths.extend(fallback.glyph_widths.clone());
            } else {
                merged_by_font.insert(fallback.font_id.clone(), fallback.clone());
            }
        }
        if merged_by_font.len() > MAX_SUBSET_FONT_RESOURCES_PER_MUTATION {
            return Err(PdfError::InvalidOperation(
                "Adaptive FreeText exceeds the font resource limit".into(),
            ));
        }
        let required_objects = (merged_by_font.len() as u64)
            .checked_mul(6)
            .ok_or_else(|| PdfError::InvalidOperation("Font object count overflow".into()))?;
        if self.next_alloc_obj_num.saturating_add(required_objects) > MAX_PDF_OBJECT_NUMBER {
            return Err(PdfError::InvalidOperation(
                "PDF object number allocation limit exceeded".into(),
            ));
        }

        let mut font_tags = BTreeMap::new();
        let mut font_resources = BTreeMap::new();
        for (index, (font_id, fallback)) in merged_by_font.iter().enumerate() {
            let embedded = Type0FontEmbedder::embed_type0_font_resource(
                &fallback.font_bytes,
                &fallback.font_name,
                &fallback.requested_glyphs,
                &fallback.cid_to_gid,
                &fallback.cid_to_unicode,
                &fallback.glyph_widths,
                modified,
                &mut self.next_alloc_obj_num,
            )?;
            let mut tag = embedded.resource_name;
            if font_resources.contains_key(&tag) {
                tag = format!("{tag}_{}", index + 1);
            }
            font_resources.insert(tag.clone(), embedded.font_object);
            font_tags.insert(font_id.clone(), tag);
        }

        let mut content = format!(
            "q\n0 0 {:.4} {:.4} re W n\n{}\nBT\n",
            width,
            height,
            da.color.to_fill_ops()
        );
        let mut cursor_x = if plan.direction == crate::font::TextDirection::RightToLeft {
            (width - 2.0 - plan.predicted_width).max(2.0)
        } else {
            2.0
        };
        let baseline = ((height - font_size) / 2.0).max(2.0);
        for (run, fallback) in plan.runs.iter().zip(&plan.fallback_runs) {
            let tag = font_tags.get(&fallback.font_id).ok_or_else(|| {
                PdfError::InvalidOperation("Missing adaptive FreeText font resource".into())
            })?;
            if fallback.raw_bytes.len() != run.glyphs.len().saturating_mul(2) {
                return Err(PdfError::InvalidOperation(
                    "Adaptive FreeText CID sequence is inconsistent".into(),
                ));
            }
            use std::fmt::Write as _;
            writeln!(&mut content, "/{tag} {:.4} Tf", font_size).map_err(|_| {
                PdfError::InvalidOperation("Appearance serialization failed".into())
            })?;
            for (glyph, cid) in run.glyphs.iter().zip(fallback.raw_bytes.chunks_exact(2)) {
                let x = cursor_x + glyph.x_offset * font_size / 1000.0;
                let y = baseline + glyph.y_offset * font_size / 1000.0;
                writeln!(
                    &mut content,
                    "1 0 0 1 {:.4} {:.4} Tm <{:02X}{:02X}> Tj",
                    x, y, cid[0], cid[1]
                )
                .map_err(|_| {
                    PdfError::InvalidOperation("Appearance serialization failed".into())
                })?;
                cursor_x += glyph.advance * font_size / 1000.0;
            }
        }
        content.push_str("ET\nQ\n");
        if content.len() > MAX_ANNOTATION_APPEARANCE_BYTES {
            return Err(PdfError::InvalidOperation(format!(
                "Annotation appearance exceeds maximum length of {MAX_ANNOTATION_APPEARANCE_BYTES} bytes"
            )));
        }

        let data = content.into_bytes();
        let resources =
            BTreeMap::from([("Font".to_string(), PdfObject::Dictionary(font_resources))]);
        let stream_dict = BTreeMap::from([
            ("Type".to_string(), PdfObject::Name("XObject".into())),
            ("Subtype".to_string(), PdfObject::Name("Form".into())),
            ("FormType".to_string(), PdfObject::Integer(1)),
            (
                "BBox".to_string(),
                PdfObject::Array(vec![
                    PdfObject::Real(0.0),
                    PdfObject::Real(0.0),
                    PdfObject::Real(width),
                    PdfObject::Real(height),
                ]),
            ),
            ("Resources".to_string(), PdfObject::Dictionary(resources)),
            ("Length".to_string(), PdfObject::Integer(data.len() as i64)),
        ]);
        Ok(StreamObject {
            dict: stream_dict,
            stream_offset: 0,
            stream_length: data.len(),
            data,
        })
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
        let sfnt =
            resolved
                .font
                .embedded_sfnt
                .as_ref()
                .ok_or(match resolved.font.font_program_kind {
                    FontProgramKind::CffDetectedUnsupported => PdfError::CffDetectedUnsupported,
                    FontProgramKind::Cff2DetectedUnsupported => PdfError::Cff2DetectedUnsupported,
                    FontProgramKind::TrueTypeSupported | FontProgramKind::UnknownFontProgram => {
                        PdfError::UnknownFontProgram
                    }
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

    fn effective_field_dictionary(
        &mut self,
        field_ref: ObjectRef,
        modified: &BTreeMap<ObjectRef, PdfObject>,
    ) -> PdfResult<BTreeMap<String, PdfObject>> {
        const INHERITED_KEYS: [&str; 9] = ["FT", "Ff", "DA", "Q", "Opt", "V", "DV", "MaxLen", "DR"];
        let mut effective = self.get_dict_for_modification(field_ref, modified)?;
        for key in INHERITED_KEYS {
            if let Some(value) = effective.get(key).cloned() {
                effective.insert(key.to_string(), self.store.resolve_object(&value)?);
            }
        }

        let mut current = effective.get("Parent").and_then(PdfObject::as_reference);
        let mut visited = BTreeSet::from([field_ref]);
        for _ in 0..MAX_FIELD_PROPERTY_ANCESTORS {
            let Some(parent_ref) = current else {
                self.inherit_acroform_defaults(&mut effective)?;
                return Ok(effective);
            };
            if !visited.insert(parent_ref) {
                return Err(PdfError::CircularReference(
                    "Cycle while resolving inherited field properties".into(),
                ));
            }
            let parent = self.get_dict_for_modification(parent_ref, modified)?;
            for key in INHERITED_KEYS {
                if !effective.contains_key(key) {
                    if let Some(value) = parent.get(key) {
                        effective.insert(key.to_string(), self.store.resolve_object(value)?);
                    }
                }
            }
            current = parent.get("Parent").and_then(PdfObject::as_reference);
        }
        Err(PdfError::RecursionLimitExceeded)
    }

    fn reconcile_need_appearances(
        &mut self,
        modified: &mut BTreeMap<ObjectRef, PdfObject>,
    ) -> PdfResult<()> {
        let Some(root) = self.store.trailer().get("Root").cloned() else {
            // NeedAppearances reconciliation is optional when no catalog/AcroForm exists.
            return Ok(());
        };
        let catalog = self.store.resolve_object(&root)?;
        let Some(catalog_dict) = catalog.as_dict().cloned() else {
            return Ok(());
        };
        let Some(acroform_object) = catalog_dict.get("AcroForm").cloned() else {
            return Ok(());
        };
        let Some(acroform_ref) = acroform_object.as_reference() else {
            return Ok(());
        };
        let acroform = self.store.resolve_object(&acroform_object)?;
        let Some(acroform_dict) = acroform.as_dict() else {
            return Ok(());
        };
        if acroform_dict
            .get("NeedAppearances")
            .and_then(PdfObject::as_bool)
            != Some(true)
        {
            return Ok(());
        }

        let parser = crate::forms::AcroFormParser::new(self.store, &self.page_refs);
        let Some(parsed) = parser.parse_catalog_acroform(&catalog_dict)? else {
            return Ok(());
        };
        if parsed.fields.is_empty() {
            return Ok(());
        }
        for field in parsed.fields {
            let targets: Vec<ObjectRef> = if field.widgets.is_empty() {
                vec![field.object_ref]
            } else {
                field
                    .widgets
                    .iter()
                    .map(|widget| widget.object_ref)
                    .collect()
            };
            for target in targets {
                if target.number == 0 || !self.has_normal_appearance(target, modified)? {
                    return Ok(());
                }
            }
        }

        let mut updated = self.get_dict_for_modification(acroform_ref, modified)?;
        updated.insert("NeedAppearances".to_string(), PdfObject::Bool(false));
        modified.insert(acroform_ref, PdfObject::Dictionary(updated));
        Ok(())
    }

    fn has_normal_appearance(
        &mut self,
        reference: ObjectRef,
        modified: &BTreeMap<ObjectRef, PdfObject>,
    ) -> PdfResult<bool> {
        let dict = self.get_dict_for_modification(reference, modified)?;
        let Some(appearance) = dict.get("AP") else {
            return Ok(false);
        };
        let appearance = self.store.resolve_object(appearance)?;
        let Some(normal) = appearance.as_dict().and_then(|dict| dict.get("N")) else {
            return Ok(false);
        };
        let normal = self.store.resolve_object(normal)?;
        if normal.as_stream().is_some() {
            return Ok(true);
        }
        let Some(states) = normal.as_dict() else {
            return Ok(false);
        };
        if states.len() > crate::font::appearance::MAX_APPEARANCE_RESOURCES {
            return Err(PdfError::InvalidOperation(format!(
                "Appearance states exceed maximum of {}",
                crate::font::appearance::MAX_APPEARANCE_RESOURCES
            )));
        }
        Ok(!states.is_empty())
    }

    fn merge_generated_appearance(
        &mut self,
        existing: Option<&PdfObject>,
        generated: &PdfObject,
    ) -> PdfResult<PdfObject> {
        let mut generated_dict =
            generated
                .as_dict()
                .cloned()
                .ok_or_else(|| PdfError::TypeMismatch {
                    expected: "appearance dictionary",
                    actual: generated.type_name(),
                })?;
        let generated_normal = generated_dict.remove("N").ok_or_else(|| {
            PdfError::InvalidOperation("Generated appearance is missing /N".into())
        })?;
        let mut merged = if let Some(existing) = existing {
            self.store
                .resolve_object(existing)?
                .as_dict()
                .cloned()
                .ok_or_else(|| PdfError::TypeMismatch {
                    expected: "appearance dictionary",
                    actual: existing.type_name(),
                })?
        } else {
            BTreeMap::new()
        };
        if merged.len() > crate::font::appearance::MAX_APPEARANCE_RESOURCES {
            return Err(PdfError::InvalidOperation(format!(
                "Appearance dictionary exceeds maximum of {} entries",
                crate::font::appearance::MAX_APPEARANCE_RESOURCES
            )));
        }

        let normal = if let Some(existing_normal) = merged.get("N").cloned() {
            let existing_resolved = self.store.resolve_object(&existing_normal)?;
            if let (Some(existing_states), Some(generated_states)) =
                (existing_resolved.as_dict(), generated_normal.as_dict())
            {
                let combined_len = existing_states
                    .len()
                    .checked_add(generated_states.len())
                    .ok_or_else(|| {
                        PdfError::InvalidOperation("Appearance state count overflow".into())
                    })?;
                if combined_len > crate::font::appearance::MAX_APPEARANCE_RESOURCES {
                    return Err(PdfError::InvalidOperation(format!(
                        "Appearance states exceed maximum of {}",
                        crate::font::appearance::MAX_APPEARANCE_RESOURCES
                    )));
                }
                let mut states = existing_states.clone();
                states.extend(generated_states.clone());
                PdfObject::Dictionary(states)
            } else {
                generated_normal
            }
        } else {
            generated_normal
        };
        merged.insert("N".to_string(), normal);
        for (key, value) in generated_dict {
            merged.insert(key, value);
        }
        Ok(PdfObject::Dictionary(merged))
    }

    fn inherit_acroform_defaults(
        &mut self,
        effective: &mut BTreeMap<String, PdfObject>,
    ) -> PdfResult<()> {
        let Some(root) = self.store.trailer().get("Root").cloned() else {
            return Ok(());
        };
        let catalog = self.store.resolve_object(&root)?;
        let Some(acroform) = catalog
            .as_dict()
            .and_then(|dict| dict.get("AcroForm"))
            .cloned()
        else {
            return Ok(());
        };
        let acroform = self.store.resolve_object(&acroform)?;
        let Some(acroform_dict) = acroform.as_dict() else {
            return Ok(());
        };
        for key in ["DA", "Q", "DR"] {
            if !effective.contains_key(key) {
                if let Some(value) = acroform_dict.get(key) {
                    effective.insert(key.to_string(), self.store.resolve_object(value)?);
                }
            }
        }
        Ok(())
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
