use crate::document::PdfDocument;
use crate::error::{PdfError, PdfResult};
use crate::font::font::{Font, FontStyle};
use crate::font::shaping::{ShapedRun, TextDirection, TextShaper};
use crate::mutation::text_edit::TextEditTarget;

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
    pub refusal_reason: Option<String>,
}

impl TextReplacementPlan {
    pub fn is_executable(&self) -> bool {
        self.strategy != ReplacementStrategy::SafeRefusal
            && self.layout_safety != LayoutSafetyClassification::UnsafeLayout
            && self.layout_safety != LayoutSafetyClassification::UnsupportedFont
    }
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
        let orig_font = resources
            .get_font(&span.font_resource_name)
            .ok_or_else(|| {
                PdfError::UnsupportedFontEncoding(format!(
                    "Font resource {} not found",
                    span.font_resource_name
                ))
            })?;

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
                refusal_reason: None,
            });
        }

        // 3. ADAPTIVE PATH: Fallback font catalog
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
                refusal_reason: None,
            });
        }

        // 4. Safe Refusal if no candidate font can represent the glyphs
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
            refusal_reason: Some(reason),
        })
    }

    /// Atomically executes a planned text replacement.
    pub fn apply(
        doc: &mut PdfDocument,
        plan: &TextReplacementPlan,
    ) -> PdfResult<crate::mutation::MutationPlan> {
        if !plan.is_executable() {
            return Err(PdfError::UnsupportedFontEncoding(
                plan.refusal_reason
                    .clone()
                    .unwrap_or_else(|| "Planned text replacement is not executable".to_string()),
            ));
        }

        doc.replace_text(plan.page_index, &plan.target, &plan.replacement_text)
    }
}
