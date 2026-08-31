use crate::error::{PdfError, PdfResult};
use crate::font::font::{FontFamily, FontStyle};
use crate::font::planner::{TextPlanner, TextReplacementPlan};
use crate::mutation::text_edit::TextEditTarget;
use crate::mutation::MutationPlan;
use crate::PdfDocument;

#[cfg(feature = "wasm")]
use serde::{Deserialize, Serialize};

pub const MIN_TEXT_FONT_SIZE: f64 = 6.0;
pub const MAX_TEXT_FONT_SIZE: f64 = 144.0;

pub fn validate_text_font_size(size: f64) -> PdfResult<()> {
    if !size.is_finite() || !(MIN_TEXT_FONT_SIZE..=MAX_TEXT_FONT_SIZE).contains(&size) {
        return Err(PdfError::InvalidOperation(
            "TEXT_STYLE_SIZE_OUT_OF_RANGE: font size must be finite and within 6..=144 pt".into(),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
pub enum TextWeight {
    Normal,
    Bold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
pub enum TextStyleCapability {
    Editable,
    SafeRefusal,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "wasm", derive(Serialize, Deserialize))]
pub struct ComputedTextStyle {
    pub font_family: String,
    pub font_size: f64,
    pub weight: TextWeight,
    pub italic: bool,
    pub fill_color: [f64; 3],
    pub source_font: String,
    pub capability: TextStyleCapability,
}

#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "wasm", derive(Serialize, Deserialize))]
pub struct TextStylePatch {
    pub font_family: Option<String>,
    pub font_size: Option<f64>,
    pub weight: Option<TextWeight>,
    pub italic: Option<bool>,
    pub fill_color: Option<[f64; 3]>,
    /// Optional replacement text used by the combined edit+style UI transaction.
    pub replacement_text: Option<String>,
}

impl TextStylePatch {
    pub fn validate(&self) -> PdfResult<()> {
        if let Some(size) = self.font_size {
            validate_text_font_size(size)?;
        }
        if self.fill_color.is_some_and(|color| {
            color
                .iter()
                .any(|component| !component.is_finite() || !(0.0..=1.0).contains(component))
        }) {
            return Err(PdfError::InvalidOperation(
                "TEXT_STYLE_COLOR_INVALID: RGB components must be finite and within 0..=1".into(),
            ));
        }
        if let Some(family) = &self.font_family {
            parse_font_family(family)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct TextStylePlan {
    pub computed: ComputedTextStyle,
    pub requested: ComputedTextStyle,
    pub replacement: TextReplacementPlan,
}

pub struct TextStylePlanner;

impl TextStylePlanner {
    pub fn inspect_native(
        doc: &mut PdfDocument<'_>,
        page_index: usize,
        target: &TextEditTarget,
    ) -> PdfResult<ComputedTextStyle> {
        let page = doc.extract_page_text(page_index)?;
        let span = page
            .spans
            .iter()
            .find(|span| {
                span.stream_index == target.stream_index
                    && span.instruction_index == target.instruction_index
                    && span.operand_index == target.operand_index
            })
            .ok_or_else(|| {
                PdfError::TargetTextNotFound("Text style target was not found".into())
            })?;
        Ok(ComputedTextStyle {
            font_family: span.font_family.clone(),
            font_size: span.font_size,
            weight: if span.is_bold {
                TextWeight::Bold
            } else {
                TextWeight::Normal
            },
            italic: span.is_italic,
            fill_color: span.fill_color,
            source_font: span.font_base_name.clone(),
            capability: if span.is_editable {
                TextStyleCapability::Editable
            } else {
                TextStyleCapability::SafeRefusal
            },
        })
    }

    pub fn plan_native(
        doc: &mut PdfDocument<'_>,
        page_index: usize,
        target: &TextEditTarget,
        patch: &TextStylePatch,
    ) -> PdfResult<TextStylePlan> {
        patch.validate()?;
        let computed = Self::inspect_native(doc, page_index, target)?;
        if computed.capability != TextStyleCapability::Editable {
            return Err(PdfError::UnsupportedLayout(
                "TEXT_STYLE_TARGET_READ_ONLY: selected text cannot be safely isolated".into(),
            ));
        }
        let page = doc.extract_page_text(page_index)?;
        let span = page
            .spans
            .iter()
            .find(|span| {
                span.stream_index == target.stream_index
                    && span.instruction_index == target.instruction_index
                    && span.operand_index == target.operand_index
            })
            .ok_or_else(|| {
                PdfError::TargetTextNotFound("Text style target was not found".into())
            })?;

        let family = patch
            .font_family
            .as_deref()
            .map(parse_font_family)
            .transpose()?
            .unwrap_or_else(|| {
                parse_font_family(&computed.font_family).unwrap_or(FontFamily::SansSerif)
            });
        let weight = patch.weight.unwrap_or(computed.weight);
        let italic = patch.italic.unwrap_or(computed.italic);
        let requested_style = FontStyle {
            family,
            is_bold: weight == TextWeight::Bold,
            is_italic: italic,
            is_monospace: family == FontFamily::Monospace,
        };
        let font_size = patch.font_size.unwrap_or(computed.font_size);
        let fill_color = patch.fill_color.unwrap_or(computed.fill_color);
        let replacement_text = patch.replacement_text.as_deref().unwrap_or(&span.text);
        let mut replacement = TextPlanner::plan(
            doc,
            page_index,
            target,
            replacement_text,
            Some(&requested_style),
        )?;
        replacement.font_size = font_size;
        replacement.predicted_width = replacement
            .runs
            .iter()
            .map(|run| run.total_advance)
            .sum::<f64>()
            / 1000.0
            * font_size;
        replacement.requested_style = Some(requested_style);
        replacement.fill_color = Some(fill_color);
        replacement.isolated_style = true;

        let requested = ComputedTextStyle {
            font_family: font_family_name(family).to_string(),
            font_size,
            weight,
            italic,
            fill_color,
            source_font: replacement.font_resource_name.clone(),
            capability: if replacement.is_executable() {
                TextStyleCapability::Editable
            } else {
                TextStyleCapability::SafeRefusal
            },
        };
        Ok(TextStylePlan {
            computed,
            requested,
            replacement,
        })
    }

    pub fn apply_native(
        doc: &mut PdfDocument<'_>,
        plan: &TextStylePlan,
    ) -> PdfResult<MutationPlan> {
        TextPlanner::apply(doc, &plan.replacement)
    }
}

pub fn parse_font_family(value: &str) -> PdfResult<FontFamily> {
    match value
        .trim()
        .to_ascii_lowercase()
        .replace(['-', ' '], "")
        .as_str()
    {
        "sans" | "sansserif" | "helvetica" | "arial" => Ok(FontFamily::SansSerif),
        "serif" | "times" | "timesroman" => Ok(FontFamily::Serif),
        "mono" | "monospace" | "courier" => Ok(FontFamily::Monospace),
        other => Err(PdfError::InvalidOperation(format!(
            "TEXT_STYLE_FONT_FAMILY_UNSUPPORTED: '{other}' is not a qualified PDF font family"
        ))),
    }
}

pub const fn font_family_name(family: FontFamily) -> &'static str {
    match family {
        FontFamily::SansSerif => "SansSerif",
        FontFamily::Serif => "Serif",
        FontFamily::Monospace => "Monospace",
        FontFamily::Symbolic => "Symbolic",
    }
}

pub fn style_from_da_font_name(font_name: &str) -> FontStyle {
    let lower = font_name.to_ascii_lowercase();
    let family = if lower.contains("times") || lower.contains("serif") {
        FontFamily::Serif
    } else if lower.contains("courier") || lower.contains("mono") {
        FontFamily::Monospace
    } else {
        FontFamily::SansSerif
    };
    FontStyle {
        family,
        is_bold: lower.contains("bold"),
        is_italic: lower.contains("italic") || lower.contains("oblique"),
        is_monospace: family == FontFamily::Monospace,
    }
}
