use std::collections::BTreeMap;

use crate::appearance::da_parser::DefaultAppearance;
use crate::appearance::fonts::FontMetricsHelper;
use crate::error::{PdfError, PdfResult};
use crate::font::appearance::AppearanceFont;
use crate::syntax::object::{PdfObject, StreamObject};

pub const MAX_APPEARANCE_BYTES: usize = 256 * 1024;
pub const MAX_MULTILINE_LINES: usize = 2_048;
pub const MAX_COMB_CELLS: usize = 4_096;

#[derive(Debug, Clone, Copy, Default)]
pub struct TextLayoutOptions {
    pub multiline: bool,
    pub comb_max_len: Option<usize>,
}

pub struct TextFieldAppearance;

impl TextFieldAppearance {
    /// Generates a normal appearance Form XObject stream for a text field.
    pub fn generate_stream(
        rect: [f64; 4],
        text: &str,
        da: &DefaultAppearance,
        quadding: i32,
        multiline: bool,
    ) -> PdfResult<StreamObject> {
        Self::generate_stream_with_options(
            rect,
            text,
            da,
            quadding,
            TextLayoutOptions {
                multiline,
                comb_max_len: None,
            },
        )
    }

    pub fn generate_stream_with_options(
        rect: [f64; 4],
        text: &str,
        da: &DefaultAppearance,
        quadding: i32,
        options: TextLayoutOptions,
    ) -> PdfResult<StreamObject> {
        Self::generate_stream_with_font(rect, text, da, quadding, options, None)
    }

    pub fn generate_stream_with_font(
        rect: [f64; 4],
        text: &str,
        da: &DefaultAppearance,
        quadding: i32,
        options: TextLayoutOptions,
        resolved_font: Option<&AppearanceFont>,
    ) -> PdfResult<StreamObject> {
        let width = rect[2] - rect[0];
        let height = rect[3] - rect[1];

        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
            return Err(PdfError::InvalidOperation(
                "Invalid text field widget bounding box dimensions".into(),
            ));
        }

        let padding_x = 2.0_f64.min(width * 0.1);
        let usable_width = (width - 2.0 * padding_x).max(1.0);

        let font_size = if da.font_size > 0.0 {
            da.font_size
        } else {
            // Auto-size font based on height
            (height * 0.65).clamp(6.0, 14.0)
        };

        let mut content = Vec::with_capacity(512);

        // 1. Clipping and graphics state
        content.extend_from_slice(b"/Tx BMC\nq\n");
        let clip_op = format!("0 0 {:.2} {:.2} re\nW\nn\n", width, height);
        content.extend_from_slice(clip_op.as_bytes());

        // 2. Color and font selection
        let color_op = format!("{}\n", da.color.to_fill_ops());
        content.extend_from_slice(color_op.as_bytes());

        if let Some(font) = resolved_font {
            font.verify_text(text)?;
        }
        let font_key = resolved_font.map_or_else(
            || da.font_name.trim_start_matches('/'),
            |font| font.resource_name.as_str(),
        );
        let font_op = format!("BT\n/{} {:.2} Tf\n", font_key, font_size);
        content.extend_from_slice(font_op.as_bytes());

        // 3. Text output using text matrix (1 0 0 1 x y Tm)
        if let Some(max_len) = options.comb_max_len {
            if options.multiline {
                return Err(PdfError::InvalidOperation(
                    "Comb text fields cannot also be multiline".into(),
                ));
            }
            if max_len == 0 || max_len > MAX_COMB_CELLS {
                return Err(PdfError::InvalidOperation(format!(
                    "Comb /MaxLen must be within 1..={MAX_COMB_CELLS}"
                )));
            }
            let characters: Vec<char> = text.chars().collect();
            if characters.len() > max_len {
                return Err(PdfError::InvalidOperation(format!(
                    "Comb value contains {} characters but /MaxLen is {max_len}",
                    characters.len()
                )));
            }
            let cell_width = width / max_len as f64;
            for index in 1..max_len {
                let x = index as f64 * cell_width;
                content.extend_from_slice(
                    format!(
                        "ET\nq\n0.75 G\n0.5 w\n{:.2} 0 m\n{:.2} {:.2} l\nS\nQ\nBT\n/{} {:.2} Tf\n{}\n",
                        x,
                        x,
                        height,
                        font_key,
                        font_size,
                        da.color.to_fill_ops()
                    )
                        .as_bytes(),
                );
            }
            for (index, character) in characters.iter().enumerate() {
                let value = character.to_string();
                let glyph_width =
                    Self::measure_text(&value, &da.font_name, font_size, resolved_font)?;
                let x = index as f64 * cell_width + ((cell_width - glyph_width) / 2.0).max(0.0);
                let y = ((height - font_size) / 2.0 + font_size * 0.15).max(1.0);
                content.extend_from_slice(
                    format!(
                        "1 0 0 1 {:.2} {:.2} Tm\n{} Tj\n",
                        x,
                        y,
                        Self::text_operand(&value, resolved_font)?
                    )
                    .as_bytes(),
                );
            }
        } else if options.multiline {
            let lines =
                Self::wrap_multiline(text, usable_width, &da.font_name, font_size, resolved_font)?;
            let line_height = font_size * 1.2;
            let start_y = height - padding_x - font_size;

            for (idx, line) in lines.iter().enumerate() {
                let y_pos = start_y - (idx as f64 * line_height);
                if y_pos + line_height < 0.0 {
                    break;
                }
                let line_width = Self::measure_text(line, &da.font_name, font_size, resolved_font)?;
                let x_pos = match quadding {
                    1 => (padding_x + (usable_width - line_width) / 2.0).max(padding_x), // Center
                    2 => (width - padding_x - line_width).max(padding_x),                // Right
                    _ => padding_x,                                                      // Left
                };
                let escaped_line = Self::text_operand(line, resolved_font)?;
                let text_line_op = format!(
                    "1 0 0 1 {:.2} {:.2} Tm\n{} Tj\n",
                    x_pos, y_pos, escaped_line
                );
                content.extend_from_slice(text_line_op.as_bytes());
            }
        } else {
            let text_width = Self::measure_text(text, &da.font_name, font_size, resolved_font)?;
            let x_pos = match quadding {
                1 => (padding_x + (usable_width - text_width) / 2.0).max(padding_x), // Center
                2 => (width - padding_x - text_width).max(padding_x),                // Right
                _ => padding_x,                                                      // Left
            };
            let y_pos = ((height - font_size) / 2.0 + font_size * 0.15).max(1.0);

            let escaped = Self::text_operand(text, resolved_font)?;
            let text_op = format!("1 0 0 1 {:.2} {:.2} Tm\n{} Tj\n", x_pos, y_pos, escaped);
            content.extend_from_slice(text_op.as_bytes());
        }

        content.extend_from_slice(b"ET\nQ\nEMC\n");

        if content.len() > MAX_APPEARANCE_BYTES {
            return Err(PdfError::InvalidOperation(
                "Generated text field appearance exceeds maximum buffer limit".into(),
            ));
        }

        // 4. Form XObject Dictionary
        let mut dict = BTreeMap::new();
        dict.insert("Type".to_string(), PdfObject::Name("XObject".to_string()));
        dict.insert("Subtype".to_string(), PdfObject::Name("Form".to_string()));
        dict.insert("FormType".to_string(), PdfObject::Integer(1));
        dict.insert(
            "BBox".to_string(),
            PdfObject::Array(vec![
                PdfObject::Real(0.0),
                PdfObject::Real(0.0),
                PdfObject::Real(width),
                PdfObject::Real(height),
            ]),
        );
        dict.insert(
            "Length".to_string(),
            PdfObject::Integer(content.len() as i64),
        );

        let res_dict = resolved_font.map_or_else(
            || FontMetricsHelper::build_font_resource(&da.font_name),
            AppearanceFont::resource_dictionary,
        );
        dict.insert("Resources".to_string(), PdfObject::Dictionary(res_dict));

        let stream_length = content.len();
        Ok(StreamObject {
            dict,
            data: content,
            stream_offset: 0,
            stream_length,
        })
    }

    fn wrap_multiline(
        text: &str,
        usable_width: f64,
        font_name: &str,
        font_size: f64,
        resolved_font: Option<&AppearanceFont>,
    ) -> PdfResult<Vec<String>> {
        let mut lines = Vec::new();
        for explicit in text.split('\n') {
            let explicit = explicit.strip_suffix('\r').unwrap_or(explicit);
            if explicit.is_empty() {
                Self::push_line(&mut lines, String::new())?;
                continue;
            }
            let mut current = String::new();
            for word in explicit.split_inclusive(char::is_whitespace) {
                let candidate = format!("{current}{word}");
                if current.is_empty()
                    || Self::measure_text(&candidate, font_name, font_size, resolved_font)?
                        <= usable_width
                {
                    current = candidate;
                    continue;
                }
                Self::push_line(&mut lines, current.trim_end().to_string())?;
                current.clear();
                if Self::measure_text(word, font_name, font_size, resolved_font)? <= usable_width {
                    current.push_str(word.trim_start());
                } else {
                    for character in word.trim().chars() {
                        let candidate = format!("{current}{character}");
                        if !current.is_empty()
                            && Self::measure_text(&candidate, font_name, font_size, resolved_font)?
                                > usable_width
                        {
                            Self::push_line(&mut lines, current)?;
                            current = String::new();
                        }
                        current.push(character);
                    }
                }
            }
            Self::push_line(&mut lines, current.trim_end().to_string())?;
        }
        Ok(lines)
    }

    fn measure_text(
        text: &str,
        font_name: &str,
        font_size: f64,
        resolved_font: Option<&AppearanceFont>,
    ) -> PdfResult<f64> {
        match resolved_font {
            Some(font) => font.text_width(text, font_size),
            None => Ok(FontMetricsHelper::estimate_text_width(
                text, font_name, font_size,
            )),
        }
    }

    pub(crate) fn text_operand(
        text: &str,
        resolved_font: Option<&AppearanceFont>,
    ) -> PdfResult<String> {
        if let Some(font) = resolved_font {
            let encoded = font.encode_text(text)?;
            let mut output =
                String::with_capacity(encoded.len().saturating_mul(2).saturating_add(2));
            output.push('<');
            for byte in encoded {
                use std::fmt::Write as _;
                write!(&mut output, "{byte:02X}").map_err(|_| {
                    PdfError::InvalidOperation("Failed to encode appearance glyphs".into())
                })?;
            }
            output.push('>');
            Ok(output)
        } else {
            Ok(format!("({})", escape_pdf_string(text)))
        }
    }

    fn push_line(lines: &mut Vec<String>, line: String) -> PdfResult<()> {
        if lines.len() >= MAX_MULTILINE_LINES {
            return Err(PdfError::InvalidOperation(format!(
                "Multiline appearance exceeds maximum of {MAX_MULTILINE_LINES} generated lines"
            )));
        }
        lines.push(line);
        Ok(())
    }
}

pub fn escape_pdf_string(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 16);
    for c in input.chars() {
        match c {
            '(' => out.push_str("\\("),
            ')' => out.push_str("\\)"),
            '\\' => out.push_str("\\\\"),
            '\r' => out.push_str("\\r"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out
}
