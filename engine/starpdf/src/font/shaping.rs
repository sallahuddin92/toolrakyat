use crate::font::font::Font;
use unicode_bidi::{BidiInfo, Level};

#[derive(Debug, Clone, PartialEq)]
pub struct ShapedGlyph {
    pub glyph_id: u32,
    pub cluster: u32,
    pub advance: f64,  // in 1/1000 font units
    pub x_offset: f64, // in 1/1000 font units
    pub y_offset: f64, // in 1/1000 font units
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShapedRun {
    pub text: String,
    pub glyphs: Vec<ShapedGlyph>,
    pub is_rtl: bool,
    pub total_advance: f64, // in 1/1000 font units
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextDirection {
    LeftToRight,
    RightToLeft,
    Mixed,
}

pub struct TextShaper;

impl TextShaper {
    /// Itemizes input text into bidirectional runs and shapes each run using the provided font.
    pub fn shape_text(font: &Font, text: &str) -> Vec<ShapedRun> {
        if text.is_empty() {
            return Vec::new();
        }

        // 1. BiDi itemization
        let bidi_info = BidiInfo::new(text, None);
        let mut runs = Vec::new();

        for para in &bidi_info.paragraphs {
            let (_, visual_run_ranges) = bidi_info.visual_runs(para, para.range.clone());
            for run_range in visual_run_ranges {
                let run_text = &text[run_range.clone()];
                let is_rtl = bidi_info.levels[run_range.start].is_rtl();

                let mut shaped_glyphs = Vec::with_capacity(run_text.len());
                let mut total_advance = 0.0;

                for (idx, ch) in run_text.chars().enumerate() {
                    let (gid, width) = font
                        .embedded_sfnt
                        .as_ref()
                        .and_then(|sfnt| {
                            let gid = sfnt
                                .cmap
                                .as_ref()?
                                .char_to_glyph
                                .get(&(ch as u32))
                                .copied()?;
                            let w = sfnt.get_advance_width(gid as u32).unwrap_or(500.0);
                            Some((gid as u32, w))
                        })
                        .or_else(|| {
                            let w =
                                crate::font::standard_metrics::StandardFontMetrics::get_char_width(
                                    &font.base_font,
                                    ch,
                                )
                                .unwrap_or(font.default_width);
                            let code = font.encoding.encode_char(ch).unwrap_or(b'?') as u32;
                            Some((code, w))
                        })
                        .unwrap_or((0, font.default_width));

                    shaped_glyphs.push(ShapedGlyph {
                        glyph_id: gid,
                        cluster: idx as u32,
                        advance: width,
                        x_offset: 0.0,
                        y_offset: 0.0,
                    });

                    total_advance += width;
                }

                runs.push(ShapedRun {
                    text: run_text.to_string(),
                    glyphs: shaped_glyphs,
                    is_rtl,
                    total_advance,
                });
            }
        }

        runs
    }

    /// Detects overall text direction
    pub fn detect_direction(text: &str) -> TextDirection {
        let bidi_info = BidiInfo::new(text, None);
        let mut has_ltr = false;
        let mut has_rtl = false;

        for level in &bidi_info.levels {
            if level.is_rtl() {
                has_rtl = true;
            } else if *level != Level::ltr() || !level.is_rtl() {
                has_ltr = true;
            }
        }

        if has_ltr && has_rtl {
            TextDirection::Mixed
        } else if has_rtl {
            TextDirection::RightToLeft
        } else {
            TextDirection::LeftToRight
        }
    }
}
