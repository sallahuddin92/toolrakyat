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
    pub underline: bool,
    pub strikethrough: bool,
    pub highlight_color: Option<[f64; 3]>,
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
    pub underline: Option<bool>,
    pub strikethrough: Option<bool>,
    pub highlight_enabled: Option<bool>,
    pub highlight_color: Option<[f64; 3]>,
    /// Optional replacement text used by the combined edit+style UI transaction.
    pub replacement_text: Option<String>,
}

impl TextStylePatch {
    pub fn validate(&self) -> PdfResult<()> {
        if let Some(size) = self.font_size {
            validate_text_font_size(size)?;
        }
        if self
            .fill_color
            .into_iter()
            .chain(self.highlight_color)
            .any(|color| {
                color
                    .iter()
                    .any(|component| !component.is_finite() || !(0.0..=1.0).contains(component))
            })
        {
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
    pub decoration: crate::mutation::NativeTextDecorationMutation,
    pub rewrite_text: bool,
    pub decoration_changed: bool,
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
        let decoration = inspect_owned_decorations(doc, page_index, &span.span_id)?;
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
            underline: decoration.0,
            strikethrough: decoration.1,
            highlight_color: decoration.2,
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
            underline: patch.underline.unwrap_or(computed.underline),
            strikethrough: patch.strikethrough.unwrap_or(computed.strikethrough),
            highlight_color: match patch.highlight_enabled {
                Some(false) => None,
                Some(true) => Some(
                    patch
                        .highlight_color
                        .or(computed.highlight_color)
                        .unwrap_or([1.0, 0.92, 0.23]),
                ),
                None => patch.highlight_color.or(computed.highlight_color),
            },
            source_font: replacement.font_resource_name.clone(),
            capability: if replacement.is_executable() {
                TextStyleCapability::Editable
            } else {
                TextStyleCapability::SafeRefusal
            },
        };
        let rewrite_text = patch.font_family.is_some()
            || patch.font_size.is_some()
            || patch.weight.is_some()
            || patch.italic.is_some()
            || patch.fill_color.is_some()
            || patch.replacement_text.is_some();
        let decoration_changed = patch.underline.is_some()
            || patch.strikethrough.is_some()
            || patch.highlight_enabled.is_some()
            || patch.highlight_color.is_some()
            || (rewrite_text
                && (computed.underline
                    || computed.strikethrough
                    || computed.highlight_color.is_some()));
        let mut decoration =
            native_decoration_mutation(span, &requested, replacement.predicted_width)?;
        if rewrite_text {
            decoration.previous_target_id = Some(span.span_id.clone());
            decoration.target_id = format!(
                "p{}_s{}_i{}_o0",
                span.page_index,
                span.stream_index,
                span.instruction_index + 3
            );
        }
        Ok(TextStylePlan {
            computed,
            requested,
            replacement,
            decoration,
            rewrite_text,
            decoration_changed,
        })
    }

    pub fn apply_native(
        doc: &mut PdfDocument<'_>,
        plan: &TextStylePlan,
    ) -> PdfResult<MutationPlan> {
        if !plan.rewrite_text {
            return doc.apply_mutation(&[crate::mutation::PdfChange::SetNativeTextDecorations {
                page_index: plan.replacement.page_index,
                decoration: plan.decoration.clone(),
            }]);
        }
        if plan.decoration_changed
            && plan.replacement.strategy != crate::font::ReplacementStrategy::ShapedFallback
        {
            let font_style = plan
                .replacement
                .requested_style
                .ok_or_else(|| PdfError::InvalidOperation("Missing requested text style".into()))?;
            return doc.apply_mutation(&[
                crate::mutation::PdfChange::StyleText {
                    page_index: plan.replacement.page_index,
                    target: plan.replacement.target.clone(),
                    replacement: plan.replacement.replacement_text.clone(),
                    style: crate::mutation::TextStyleMutation {
                        font_style,
                        font_size: plan.replacement.font_size,
                        fill_color: plan.replacement.fill_color.unwrap_or([0.0, 0.0, 0.0]),
                        font_resource_name: plan.replacement.font_resource_name.clone(),
                    },
                },
                crate::mutation::PdfChange::SetNativeTextDecorations {
                    page_index: plan.replacement.page_index,
                    decoration: plan.decoration.clone(),
                },
            ]);
        }
        TextPlanner::apply(doc, &plan.replacement)
    }
}

pub(crate) fn inspect_owned_decorations(
    doc: &mut PdfDocument<'_>,
    page_index: usize,
    target_id: &str,
) -> PdfResult<(bool, bool, Option<[f64; 3]>)> {
    let page_ref = doc.page_ref(page_index)?;
    let page = doc.store_mut().resolve(page_ref)?.clone();
    let Some(dict) = page.as_dict() else {
        return Ok((false, false, None));
    };
    let annots = match dict.get("Annots") {
        Some(crate::syntax::PdfObject::Array(values)) => values.clone(),
        Some(crate::syntax::PdfObject::Reference(reference)) => doc
            .store_mut()
            .resolve(*reference)?
            .as_array()
            .map_or_else(Vec::new, ToOwned::to_owned),
        _ => Vec::new(),
    };
    let mut underline = false;
    let mut strikethrough = false;
    let mut highlight = None;
    for item in annots {
        let Some(reference) = item.as_reference() else {
            continue;
        };
        let object = doc.store_mut().resolve(reference)?.clone();
        let Some(annotation) = object.as_dict() else {
            continue;
        };
        if annotation
            .get("StarPDFDecoration")
            .and_then(crate::syntax::PdfObject::as_bool)
            != Some(true)
            || annotation
                .get("StarPDFDecorationTarget")
                .and_then(crate::syntax::PdfObject::as_string_lossy)
                .as_deref()
                != Some(target_id)
        {
            continue;
        }
        match annotation
            .get("Subtype")
            .and_then(crate::syntax::PdfObject::as_name)
        {
            Some("Underline") => underline = true,
            Some("StrikeOut") => strikethrough = true,
            Some("Highlight") => {
                let values = annotation
                    .get("C")
                    .and_then(crate::syntax::PdfObject::as_array);
                if let Some(values) = values.filter(|values| values.len() == 3) {
                    highlight = Some([
                        values[0].as_real().unwrap_or(1.0),
                        values[1].as_real().unwrap_or(0.92),
                        values[2].as_real().unwrap_or(0.23),
                    ]);
                } else {
                    highlight = Some([1.0, 0.92, 0.23]);
                }
            }
            _ => {}
        }
    }
    Ok((underline, strikethrough, highlight))
}

fn native_decoration_mutation(
    span: &crate::text::TextSpan,
    style: &ComputedTextStyle,
    planned_width: f64,
) -> PdfResult<crate::mutation::NativeTextDecorationMutation> {
    let width = if planned_width.is_finite() && planned_width > 0.0 {
        planned_width
    } else {
        span.width
    };
    let height = span.height * style.font_size / span.font_size.max(f64::EPSILON);
    if ![span.x, span.y, width, height, span.rotation]
        .iter()
        .all(|value| value.is_finite())
        || width <= 0.0
        || height <= 0.0
    {
        return Err(PdfError::UnsupportedLayout(
            "TEXT_DECORATION_GEOMETRY_UNPROVABLE: text run bounds are invalid".into(),
        ));
    }
    let radians = span.rotation.to_radians();
    let advance = [radians.cos() * width, radians.sin() * width];
    let vertical = [-radians.sin() * height, radians.cos() * height];
    let lower_left = [span.x, span.y];
    let lower_right = [span.x + advance[0], span.y + advance[1]];
    let upper_left = [lower_left[0] + vertical[0], lower_left[1] + vertical[1]];
    let upper_right = [lower_right[0] + vertical[0], lower_right[1] + vertical[1]];
    let quad = [
        upper_left[0],
        upper_left[1],
        upper_right[0],
        upper_right[1],
        lower_left[0],
        lower_left[1],
        lower_right[0],
        lower_right[1],
    ];
    let xs = [upper_left[0], upper_right[0], lower_left[0], lower_right[0]];
    let ys = [upper_left[1], upper_right[1], lower_left[1], lower_right[1]];
    let rect = [
        xs.into_iter().fold(f64::INFINITY, f64::min) - 0.5,
        ys.into_iter().fold(f64::INFINITY, f64::min) - 0.5,
        xs.into_iter().fold(f64::NEG_INFINITY, f64::max) + 0.5,
        ys.into_iter().fold(f64::NEG_INFINITY, f64::max) + 0.5,
    ];
    Ok(crate::mutation::NativeTextDecorationMutation {
        target_id: span.span_id.clone(),
        previous_target_id: None,
        rect,
        quad_points: quad,
        underline: style.underline,
        strikethrough: style.strikethrough,
        highlight_color: style.highlight_color,
        mark_color: style.fill_color,
    })
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod decoration_geometry_tests {
    use super::*;

    #[test]
    fn rotation_aware_quad_points_cover_the_exact_run_for_all_page_axes() {
        for rotation in [0.0, 90.0, 180.0, 270.0] {
            let mut span = crate::text::TextSpan::new(
                0,
                "Rotate".into(),
                100.0,
                200.0,
                60.0,
                12.0,
                rotation,
                "Helvetica".into(),
                12.0,
                1.0,
            );
            span.span_id = format!("rotation-{rotation}");
            let style = ComputedTextStyle {
                font_family: "SansSerif".into(),
                font_size: 12.0,
                weight: TextWeight::Normal,
                italic: false,
                fill_color: [0.0, 0.0, 0.0],
                underline: true,
                strikethrough: true,
                highlight_color: Some([1.0, 1.0, 0.0]),
                source_font: "Helvetica".into(),
                capability: TextStyleCapability::Editable,
            };
            let decoration = native_decoration_mutation(&span, &style, 60.0).unwrap();
            assert!(decoration.quad_points.iter().all(|value| value.is_finite()));
            assert!(decoration.rect[2] > decoration.rect[0]);
            assert!(decoration.rect[3] > decoration.rect[1]);
            let lower_left = [decoration.quad_points[4], decoration.quad_points[5]];
            assert!((lower_left[0] - 100.0).abs() < 0.001);
            assert!((lower_left[1] - 200.0).abs() < 0.001);
        }
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
