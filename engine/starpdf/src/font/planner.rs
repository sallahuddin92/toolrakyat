use crate::content::operand::ContentOperand;
use crate::content::operator::{ContentInstruction, ContentOperator};
use crate::document::PdfDocument;
use crate::error::{PdfError, PdfResult};
use crate::font::appearance::GlyphMappingQuality;
use crate::font::catalog::{find_candidate_fallbacks, get_font_registry};
use crate::font::embed::Type0FontEmbedder;
use crate::font::font::{Font, FontStyle};
use crate::font::shaping::{ShapedGlyph, ShapedRun, TextDirection, TextShaper};
use crate::mutation::text_edit::{ContentStreamEditor, TextEditTarget};
use crate::mutation::MutationPlan;
use crate::syntax::object::PdfObject;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplacementStrategy {
    OriginalFont,
    DocumentFont,
    BundledFallback,
    ShapedFallback,
    SafeRefusal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutSafetyClassification {
    SafeExact,
    SafeBounded,
    SafeFallback,
    SafeShaped,
    UnsafeLayout,
    UnsupportedFont,
}

#[derive(Debug, Clone)]
pub struct ShapedFallbackData {
    pub font_id: String,
    pub font_name: String,
    pub font_bytes: Vec<u8>,
    pub requested_glyphs: Vec<u16>,
    pub cid_to_gid: BTreeMap<u16, u16>,
    pub cid_to_unicode: BTreeMap<u16, String>,
    pub glyph_widths: BTreeMap<u16, f64>,
    pub raw_bytes: Vec<u8>,
}

/// A document-independent plan produced by the shared adaptive font runtime.
/// Consumers embed the returned fallback resources in either page or Form XObject resources.
#[derive(Debug, Clone)]
pub struct AdaptiveTextPlan {
    pub runs: Vec<ShapedRun>,
    pub direction: TextDirection,
    pub predicted_width: f64,
    pub fallback_runs: Vec<ShapedFallbackData>,
}

#[derive(Debug, Clone)]
pub struct TextReplacementPlan {
    pub page_index: usize,
    pub target: TextEditTarget,
    pub original_text: String,
    pub replacement_text: String,
    pub strategy: ReplacementStrategy,
    pub runs: Vec<ShapedRun>,
    pub direction: TextDirection,
    pub font_resource_name: String,
    pub font_size: f64,
    pub predicted_width: f64,
    pub available_width: f64,
    pub layout_safety: LayoutSafetyClassification,
    pub fallback_runs: Vec<ShapedFallbackData>,
    pub refusal_reason: Option<String>,
}

impl TextReplacementPlan {
    pub fn is_executable(&self) -> bool {
        self.strategy != ReplacementStrategy::SafeRefusal
            && self.layout_safety != LayoutSafetyClassification::UnsafeLayout
            && self.layout_safety != LayoutSafetyClassification::UnsupportedFont
    }
}

pub fn resolve_glyph_cluster_strings(text: &str, glyphs: &[ShapedGlyph]) -> Vec<String> {
    if glyphs.is_empty() {
        return Vec::new();
    }

    let mut cluster_offsets: Vec<usize> = glyphs.iter().map(|g| g.cluster as usize).collect();
    cluster_offsets.sort_unstable();
    cluster_offsets.dedup();

    let mut result = vec![String::new(); glyphs.len()];

    for &c in &cluster_offsets {
        let end = cluster_offsets
            .iter()
            .find(|&&next_c| next_c > c)
            .copied()
            .unwrap_or(text.len());

        let cluster_text = if c < text.len() && end <= text.len() && c <= end {
            &text[c..end]
        } else {
            ""
        };
        let cluster_chars: Vec<char> = cluster_text.chars().collect();

        let matching_indices: Vec<usize> = glyphs
            .iter()
            .enumerate()
            .filter(|(_, g)| g.cluster as usize == c)
            .map(|(i, _)| i)
            .collect();

        if matching_indices.len() == 1 {
            result[matching_indices[0]] = cluster_text.to_string();
        } else if matching_indices.len() == cluster_chars.len() {
            for (idx, &glyph_idx) in matching_indices.iter().enumerate() {
                result[glyph_idx] = cluster_chars[idx].to_string();
            }
        } else if matching_indices.len() < cluster_chars.len() {
            // Fewer glyphs than chars (e.g. ligature)
            result[matching_indices[0]] = cluster_text.to_string();
            for &glyph_idx in &matching_indices[1..] {
                result[glyph_idx] = String::new();
            }
        } else {
            // More glyphs than chars (e.g. multiple mark glyphs)
            for (idx, &glyph_idx) in matching_indices.iter().enumerate() {
                if idx < cluster_chars.len() {
                    result[glyph_idx] = cluster_chars[idx].to_string();
                } else {
                    result[glyph_idx] = String::new();
                }
            }
        }
    }

    result
}

/// Returns logical Unicode strings in source-text order. HarfRust emits RTL glyphs in visual
/// order, so these values are assigned to occurrence-specific CIDs rather than directly to GIDs.
pub fn resolve_logical_cluster_strings(text: &str, glyphs: &[ShapedGlyph]) -> Vec<String> {
    let by_glyph = resolve_glyph_cluster_strings(text, glyphs);
    let mut indices: Vec<usize> = (0..glyphs.len()).collect();
    indices.sort_by_key(|&index| (glyphs[index].cluster, index));
    indices
        .into_iter()
        .map(|index| by_glyph[index].clone())
        .collect()
}

/// Plans bundled fallback fonts and HarfRust shaping without depending on a page text span.
/// Returns `None` when the catalog cannot prove complete coverage for every run.
pub fn plan_adaptive_text(
    text: &str,
    style: &FontStyle,
    font_size: f64,
) -> PdfResult<Option<AdaptiveTextPlan>> {
    let direction = TextShaper::detect_direction(text);
    let registry = get_font_registry();

    // Prefer one font when it has exact coverage for the complete logical string.
    for entry in find_candidate_fallbacks(text, style) {
        if let Some(font_bytes) = registry.get_font(entry.font_id) {
            if registry.verify_exact_coverage(&font_bytes, text) {
                let is_rtl = direction == TextDirection::RightToLeft;
                if let Some(shaped_run) = TextShaper::shape_opentype(&font_bytes, text, is_rtl) {
                    let cluster_strings = resolve_logical_cluster_strings(text, &shaped_run.glyphs);
                    let fallback = build_fallback_run(
                        entry.font_id,
                        entry.display_name,
                        font_bytes,
                        &shaped_run,
                        &cluster_strings,
                        &mut 1,
                    )?;
                    let predicted_width = (shaped_run.total_advance / 1000.0) * font_size;
                    return Ok(Some(AdaptiveTextPlan {
                        runs: vec![shaped_run],
                        direction,
                        predicted_width,
                        fallback_runs: vec![fallback],
                    }));
                }
            }
        }
    }

    // Mixed-script fallback: itemize in visual bidi order and select a font per run.
    let bidi_info = unicode_bidi::BidiInfo::new(text, None);
    let mut runs = Vec::new();
    let mut fallback_runs = Vec::new();
    let mut next_cid_by_font: BTreeMap<String, u16> = BTreeMap::new();
    let mut total_font_units = 0.0;

    for para in &bidi_info.paragraphs {
        let (_, visual_run_ranges) = bidi_info.visual_runs(para, para.range.clone());
        for run_range in visual_run_ranges {
            let run_text = &text[run_range.clone()];
            let is_rtl = bidi_info.levels[run_range.start].is_rtl();
            let mut planned = None;

            for entry in find_candidate_fallbacks(run_text, style) {
                if !entry.covers_text_coarse(run_text) {
                    continue;
                }
                let Some(font_bytes) = registry.get_font(entry.font_id) else {
                    continue;
                };
                if !registry.verify_exact_coverage(&font_bytes, run_text) {
                    continue;
                }
                let Some(shaped_run) = TextShaper::shape_opentype(&font_bytes, run_text, is_rtl)
                else {
                    continue;
                };
                let cluster_strings = resolve_logical_cluster_strings(run_text, &shaped_run.glyphs);
                let next_cid = next_cid_by_font
                    .entry(entry.font_id.to_string())
                    .or_insert(1);
                let fallback = build_fallback_run(
                    entry.font_id,
                    entry.display_name,
                    font_bytes,
                    &shaped_run,
                    &cluster_strings,
                    next_cid,
                )?;
                planned = Some((shaped_run, fallback));
                break;
            }

            let Some((run, fallback)) = planned else {
                return Ok(None);
            };
            total_font_units += run.total_advance;
            runs.push(run);
            fallback_runs.push(fallback);
        }
    }

    if fallback_runs.is_empty() {
        return Ok(None);
    }
    Ok(Some(AdaptiveTextPlan {
        runs,
        direction,
        predicted_width: (total_font_units / 1000.0) * font_size,
        fallback_runs,
    }))
}

fn build_fallback_run(
    font_id: &str,
    font_name: &str,
    font_bytes: Vec<u8>,
    shaped_run: &ShapedRun,
    cluster_strings: &[String],
    next_cid: &mut u16,
) -> PdfResult<ShapedFallbackData> {
    let mut requested_glyphs = Vec::new();
    let mut cid_to_gid = BTreeMap::new();
    let mut cid_to_unicode = BTreeMap::new();
    let mut glyph_widths = BTreeMap::new();
    let mut raw_bytes = Vec::new();

    for (index, glyph) in shaped_run.glyphs.iter().enumerate() {
        let cid = *next_cid;
        *next_cid = next_cid.checked_add(1).ok_or_else(|| {
            PdfError::InvalidOperation("Fallback font CID space exhausted".into())
        })?;
        let gid = u16::try_from(glyph.glyph_id)
            .map_err(|_| PdfError::InvalidOperation("Fallback glyph ID exceeds 16 bits".into()))?;
        requested_glyphs.push(gid);
        cid_to_gid.insert(cid, gid);
        glyph_widths.insert(cid, glyph.advance);
        raw_bytes.extend_from_slice(&cid.to_be_bytes());
        if let Some(cluster) = cluster_strings.get(index) {
            if !cluster.is_empty() {
                cid_to_unicode.insert(cid, cluster.clone());
            }
        }
    }

    Ok(ShapedFallbackData {
        font_id: font_id.to_string(),
        font_name: font_name.to_string(),
        font_bytes,
        requested_glyphs,
        cid_to_gid,
        cid_to_unicode,
        glyph_widths,
        raw_bytes,
    })
}

pub struct TextPlanner;

impl TextPlanner {
    /// Plans a text replacement operation without mutating the document.
    pub fn plan(
        doc: &mut PdfDocument,
        page_index: usize,
        target: &TextEditTarget,
        replacement: &str,
        requested_style: Option<&FontStyle>,
    ) -> PdfResult<TextReplacementPlan> {
        let page_text = doc.extract_page_text(page_index)?;
        let span = page_text
            .spans
            .iter()
            .find(|s| {
                s.stream_index == target.stream_index
                    && s.instruction_index == target.instruction_index
                    && s.operand_index == target.operand_index
            })
            .ok_or_else(|| {
                PdfError::InvalidOperation(format!(
                    "Target text span not found at stream={}, instr={}, op={}",
                    target.stream_index, target.instruction_index, target.operand_index
                ))
            })?;

        let orig_text = span.text.clone();
        let font_size = span.font_size;
        let direction = TextShaper::detect_direction(replacement);

        // Fetch page font resources
        let page_dict = doc.page_dict(page_index)?.clone();
        let resources =
            crate::font::resource::PageResources::resolve_for_page(&page_dict, doc.store_mut())?;
        // Some producer PDFs omit a resolvable font dictionary while their text still decodes
        // through the established Standard-14 fallback. Preserve that compatibility behavior in
        // planning so routing public replacements through plan/apply does not regress them.
        let unresolved_standard_font;
        let orig_font = if let Some(font) = resources.get_font(&span.font_resource_name) {
            font
        } else {
            unresolved_standard_font = Font::standard_fallback(&span.font_resource_name);
            &unresolved_standard_font
        };

        let orig_style = requested_style.copied().unwrap_or(orig_font.style);

        // 1. FAST PATH: Check if original font can directly encode the replacement text
        if orig_font.can_encode_text(replacement) {
            let runs = TextShaper::shape_text(orig_font, replacement);
            let total_font_units: f64 = runs.iter().map(|r| r.total_advance).sum();
            let predicted_width = (total_font_units / 1000.0) * font_size;

            return Ok(TextReplacementPlan {
                page_index,
                target: target.clone(),
                original_text: orig_text,
                replacement_text: replacement.to_string(),
                strategy: ReplacementStrategy::OriginalFont,
                runs,
                direction,
                font_resource_name: span.font_resource_name.clone(),
                font_size,
                predicted_width,
                available_width: span.width,
                layout_safety: LayoutSafetyClassification::SafeExact,
                fallback_runs: Vec::new(),
                refusal_reason: None,
            });
        }

        // 2. ADAPTIVE PATH: Document font reuse
        if let Some((compat_res_name, compat_font)) =
            resources.find_compatible_font(&orig_style, replacement)
        {
            let runs = TextShaper::shape_text(compat_font, replacement);
            let total_font_units: f64 = runs.iter().map(|r| r.total_advance).sum();
            let predicted_width = (total_font_units / 1000.0) * font_size;

            return Ok(TextReplacementPlan {
                page_index,
                target: target.clone(),
                original_text: orig_text,
                replacement_text: replacement.to_string(),
                strategy: ReplacementStrategy::DocumentFont,
                runs,
                direction,
                font_resource_name: compat_res_name.to_string(),
                font_size,
                predicted_width,
                available_width: span.width,
                layout_safety: LayoutSafetyClassification::SafeBounded,
                fallback_runs: Vec::new(),
                refusal_reason: None,
            });
        }

        // 3. ADAPTIVE PATH: Bundled standard fallback font (for standard Latin/Accents)
        let std_res_name = format!(
            "F_StarPDF_{}",
            orig_style.standard_base_font_name().replace('-', "")
        );
        let std_font = Font::standard_with_style(&std_res_name, &orig_style);

        if std_font.can_encode_text(replacement) {
            let runs = TextShaper::shape_text(&std_font, replacement);
            let total_font_units: f64 = runs.iter().map(|r| r.total_advance).sum();
            let predicted_width = (total_font_units / 1000.0) * font_size;

            return Ok(TextReplacementPlan {
                page_index,
                target: target.clone(),
                original_text: orig_text,
                replacement_text: replacement.to_string(),
                strategy: ReplacementStrategy::BundledFallback,
                runs,
                direction,
                font_resource_name: std_res_name,
                font_size,
                predicted_width,
                available_width: span.width,
                layout_safety: LayoutSafetyClassification::SafeFallback,
                fallback_runs: Vec::new(),
                refusal_reason: None,
            });
        }

        if let Some(adaptive) = plan_adaptive_text(replacement, &orig_style, font_size)? {
            let primary_res_name = format!(
                "F_StarPDF_{}",
                adaptive.fallback_runs[0].font_name.replace([' ', '-'], "")
            );
            return Ok(TextReplacementPlan {
                page_index,
                target: target.clone(),
                original_text: orig_text,
                replacement_text: replacement.to_string(),
                strategy: ReplacementStrategy::ShapedFallback,
                runs: adaptive.runs,
                direction: adaptive.direction,
                font_resource_name: primary_res_name,
                font_size,
                predicted_width: adaptive.predicted_width,
                available_width: span.width,
                layout_safety: LayoutSafetyClassification::SafeShaped,
                fallback_runs: adaptive.fallback_runs,
                refusal_reason: None,
            });
        }

        // 5. Safe Refusal if no candidate font can represent the glyphs
        let missing = orig_font.missing_chars(replacement);
        let reason = format!(
            "Missing glyphs for characters {:?} in font '{}'",
            missing, orig_font.name
        );

        Ok(TextReplacementPlan {
            page_index,
            target: target.clone(),
            original_text: orig_text,
            replacement_text: replacement.to_string(),
            strategy: ReplacementStrategy::SafeRefusal,
            runs: Vec::new(),
            direction,
            font_resource_name: span.font_resource_name.clone(),
            font_size,
            predicted_width: 0.0,
            available_width: span.width,
            layout_safety: LayoutSafetyClassification::UnsupportedFont,
            fallback_runs: Vec::new(),
            refusal_reason: Some(reason),
        })
    }

    /// Atomically executes a planned text replacement using the exact plan data.
    pub fn apply(doc: &mut PdfDocument, plan: &TextReplacementPlan) -> PdfResult<MutationPlan> {
        if !plan.is_executable() {
            return Err(PdfError::UnsupportedFontEncoding(
                plan.refusal_reason
                    .clone()
                    .unwrap_or_else(|| "Planned text replacement is not executable".to_string()),
            ));
        }

        match plan.strategy {
            ReplacementStrategy::OriginalFont
            | ReplacementStrategy::DocumentFont
            | ReplacementStrategy::BundledFallback => {
                doc.apply_mutation(&[crate::mutation::PdfChange::ReplaceText {
                    page_index: plan.page_index,
                    target: plan.target.clone(),
                    replacement: plan.replacement_text.clone(),
                }])
            }
            ReplacementStrategy::ShapedFallback => {
                if plan.fallback_runs.is_empty() {
                    return Err(PdfError::InvalidOperation(
                        "Missing shaped fallback data".into(),
                    ));
                }

                let page_ref = doc.page_ref(plan.page_index)?;
                let max_num = doc
                    .store()
                    .xref()
                    .entries
                    .keys()
                    .copied()
                    .max()
                    .unwrap_or(0);
                let mut next_alloc_num = max_num.saturating_add(1);
                let mut modified = BTreeMap::new();

                // 1. Group and merge fallback runs by unique font_id to build unified ToUnicode CMaps
                let mut merged_by_font: BTreeMap<String, ShapedFallbackData> = BTreeMap::new();
                for fb in &plan.fallback_runs {
                    if let Some(existing) = merged_by_font.get_mut(&fb.font_id) {
                        existing
                            .requested_glyphs
                            .extend_from_slice(&fb.requested_glyphs);
                        existing.requested_glyphs.sort_unstable();
                        existing.requested_glyphs.dedup();
                        existing.cid_to_gid.extend(fb.cid_to_gid.clone());
                        existing.cid_to_unicode.extend(fb.cid_to_unicode.clone());
                        existing.glyph_widths.extend(fb.glyph_widths.clone());
                    } else {
                        merged_by_font.insert(fb.font_id.clone(), fb.clone());
                    }
                }

                // 2. Embed each unique fallback font once
                let mut font_tags: BTreeMap<String, String> = BTreeMap::new();
                for (font_id, fb) in &merged_by_font {
                    let font_tag = Type0FontEmbedder::embed_type0_font(
                        doc.store_mut(),
                        page_ref,
                        &fb.font_bytes,
                        &fb.font_name,
                        &fb.requested_glyphs,
                        &fb.cid_to_gid,
                        &fb.cid_to_unicode,
                        &fb.glyph_widths,
                        &mut modified,
                        &mut next_alloc_num,
                    )?;
                    font_tags.insert(font_id.clone(), font_tag);
                }

                // 3. Resolve page content stream and original span
                let page_obj = match modified.get(&page_ref) {
                    Some(obj) => obj.clone(),
                    None => doc.store_mut().resolve(page_ref)?.clone(),
                };
                let page_dict = page_obj.as_dict().ok_or_else(|| PdfError::TypeMismatch {
                    expected: "dictionary",
                    actual: page_obj.type_name(),
                })?;

                let contents_obj = match page_dict.get("Contents") {
                    Some(c) => c.clone(),
                    None => {
                        return Err(PdfError::InvalidOperation(
                            "Page has no Contents stream".into(),
                        ))
                    }
                };

                let stream_ref = match contents_obj {
                    PdfObject::Reference(r) => {
                        let resolved = doc.store_mut().resolve(r)?.clone();
                        match resolved {
                            PdfObject::Stream(_) if plan.target.stream_index == 0 => r,
                            PdfObject::Array(items) => items
                                .get(plan.target.stream_index)
                                .and_then(PdfObject::as_reference)
                                .ok_or_else(|| {
                                    PdfError::InvalidOperation(
                                        "Target content stream is not an indirect stream".into(),
                                    )
                                })?,
                            _ => {
                                return Err(PdfError::InvalidOperation(
                                    "Target content stream is not an indirect stream".into(),
                                ))
                            }
                        }
                    }
                    PdfObject::Array(items) => items
                        .get(plan.target.stream_index)
                        .and_then(PdfObject::as_reference)
                        .ok_or_else(|| {
                            PdfError::InvalidOperation(
                                "Target content stream is not an indirect stream".into(),
                            )
                        })?,
                    _ => {
                        return Err(PdfError::InvalidOperation(
                            "Direct Contents streams are not supported for shaped replacement"
                                .into(),
                        ))
                    }
                };

                let stream_obj = doc.store_mut().resolve(stream_ref)?.clone();
                let (stream_dict, decompressed) = match stream_obj {
                    PdfObject::Stream(st) => {
                        let filter = st.dict.get("Filter");
                        let data = match filter {
                            Some(PdfObject::Name(f)) if f == "FlateDecode" => {
                                miniz_oxide::inflate::decompress_to_vec_zlib(&st.data)
                                    .unwrap_or_else(|_| st.data.clone())
                            }
                            _ => st.data.clone(),
                        };
                        (st.dict.clone(), data)
                    }
                    _ => {
                        return Err(PdfError::InvalidOperation(
                            "Contents object is not a stream".into(),
                        ))
                    }
                };

                let original_span = doc
                    .extract_page_text(plan.page_index)?
                    .spans
                    .into_iter()
                    .find(|s| {
                        s.stream_index == plan.target.stream_index
                            && s.instruction_index == plan.target.instruction_index
                            && s.operand_index == plan.target.operand_index
                    })
                    .ok_or_else(|| {
                        PdfError::InvalidOperation("Original text span not found".into())
                    })?;

                // 4. Build sequence of Tf + Tj instructions for the shaped runs
                let mut replacement_instructions = Vec::new();
                for fb in &plan.fallback_runs {
                    let font_tag = font_tags.get(&fb.font_id).ok_or_else(|| {
                        PdfError::InvalidOperation("Missing allocated font tag".into())
                    })?;

                    replacement_instructions.push(ContentInstruction::new(
                        vec![
                            ContentOperand::Name(font_tag.trim_start_matches('/').to_string()),
                            ContentOperand::Real(plan.font_size),
                        ],
                        ContentOperator::Tf,
                    ));
                    replacement_instructions.push(ContentInstruction::new(
                        vec![ContentOperand::String(fb.raw_bytes.clone())],
                        ContentOperator::Tj,
                    ));
                }

                // Restore original font if downstream text exists
                replacement_instructions.push(ContentInstruction::new(
                    vec![
                        ContentOperand::Name(
                            original_span
                                .font_resource_name
                                .trim_start_matches('/')
                                .to_string(),
                        ),
                        ContentOperand::Real(original_span.font_size),
                    ],
                    ContentOperator::Tf,
                ));

                let modified_stream = ContentStreamEditor::replace_instruction_with_sequence(
                    &decompressed,
                    plan.target.instruction_index,
                    replacement_instructions,
                )?;

                // 4. Update the content stream in modified
                let mut new_stream_dict = stream_dict;
                new_stream_dict.remove("Filter");
                new_stream_dict.remove("DecodeParms");
                new_stream_dict.insert(
                    "Length".into(),
                    PdfObject::Integer(modified_stream.len() as i64),
                );
                modified.insert(
                    stream_ref,
                    PdfObject::Stream(crate::syntax::object::StreamObject {
                        dict: new_stream_dict,
                        data: modified_stream.clone(),
                        stream_offset: 0,
                        stream_length: modified_stream.len(),
                    }),
                );

                Ok(MutationPlan {
                    modified_objects: modified,
                    appearance_status: crate::mutation::AppearanceStatus::ValueUpdated,
                    glyph_mapping_quality: Some(GlyphMappingQuality::Exact),
                    layout_policy_result: Some(crate::mutation::LayoutPolicyResult::ExactFit),
                })
            }
            ReplacementStrategy::SafeRefusal => Err(PdfError::UnsupportedFontEncoding(
                plan.refusal_reason
                    .clone()
                    .unwrap_or_else(|| "Safe refusal".to_string()),
            )),
        }
    }
}
