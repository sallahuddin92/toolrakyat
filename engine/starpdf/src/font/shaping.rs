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

use harfrust::{Direction, FontRef as HrFontRef, GlyphBuffer, UnicodeBuffer};
use read_fonts::{FontRef as ReadFontRef, TableProvider};

impl TextShaper {
    /// Shapes text using HarfRust OpenType shaping (GSUB/GPOS) if font bytes are available.
    pub fn shape_opentype(font_bytes: &[u8], text: &str, is_rtl: bool) -> Option<ShapedRun> {
        let font_ref = HrFontRef::from_index(font_bytes, 0).ok()?;
        let units_per_em = ReadFontRef::from_index(font_bytes, 0)
            .ok()?
            .head()
            .ok()?
            .units_per_em() as f64;
        let scale = 1000.0 / units_per_em;
        let mut buffer = UnicodeBuffer::new();
        buffer.push_str(text);
        if is_rtl {
            buffer.set_direction(Direction::RightToLeft);
        } else {
            buffer.set_direction(Direction::LeftToRight);
        }
        buffer.guess_segment_properties();

        let shaper_data = harfrust::ShaperData::new(&font_ref);
        let shaper = shaper_data.shaper(&font_ref).build();
        let glyph_buffer: GlyphBuffer = shaper.shape(buffer, harfrust::ShapeOptions::default());
        let glyph_infos = glyph_buffer.glyph_infos();
        let glyph_positions = glyph_buffer.glyph_positions();

        let mut shaped_glyphs = Vec::with_capacity(glyph_infos.len());
        let mut total_advance = 0.0;

        for (info, pos) in glyph_infos.iter().zip(glyph_positions.iter()) {
            let gid = info.glyph_id;
            let x_adv = pos.x_advance as f64 * scale;
            let x_off = pos.x_offset as f64 * scale;
            let y_off = pos.y_offset as f64 * scale;

            shaped_glyphs.push(ShapedGlyph {
                glyph_id: gid,
                cluster: info.cluster,
                advance: x_adv,
                x_offset: x_off,
                y_offset: y_off,
            });
            total_advance += x_adv;
        }

        Some(ShapedRun {
            text: text.to_string(),
            glyphs: shaped_glyphs,
            is_rtl,
            total_advance,
        })
    }

    /// Itemizes input text into bidirectional runs and shapes each run using HarfRust or font metrics.
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

                // If embedded font program has raw SFNT bytes, perform real HarfRust shaping
                if let Some(ref sfnt) = font.embedded_sfnt {
                    if let Some(shaped) = Self::shape_opentype(&sfnt.data, run_text, is_rtl) {
                        runs.push(shaped);
                        continue;
                    }
                }

                // Fallback shaping using cmap and metrics
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
