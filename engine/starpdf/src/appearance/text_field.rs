use std::collections::BTreeMap;

use crate::appearance::da_parser::DefaultAppearance;
use crate::appearance::fonts::FontMetricsHelper;
use crate::error::{PdfError, PdfResult};
use crate::syntax::object::{PdfObject, StreamObject};

const MAX_APPEARANCE_BYTES: usize = 256 * 1024; // 256 KB
const MAX_MULTILINE_LINES: usize = 10_000;

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

        let font_key = da.font_name.trim_start_matches('/');
        let font_op = format!("BT\n/{} {:.2} Tf\n", font_key, font_size);
        content.extend_from_slice(font_op.as_bytes());

        // 3. Text output using text matrix (1 0 0 1 x y Tm)
        if multiline {
            let lines: Vec<&str> = text.lines().take(MAX_MULTILINE_LINES).collect();

            let line_height = font_size * 1.2;
            let start_y = height - padding_x - font_size;

            for (idx, &line) in lines.iter().enumerate() {
                if content.len() >= MAX_APPEARANCE_BYTES {
                    break;
                }
                let line_width =
                    FontMetricsHelper::estimate_text_width(line, &da.font_name, font_size);
                let x_pos = match quadding {
                    1 => (padding_x + (usable_width - line_width) / 2.0).max(padding_x), // Center
                    2 => (width - padding_x - line_width).max(padding_x),                // Right
                    _ => padding_x,                                                      // Left
                };
                let y_pos = start_y - (idx as f64 * line_height);

                let escaped_line = escape_pdf_string(line);
                let text_line_op = format!(
                    "1 0 0 1 {:.2} {:.2} Tm\n({}) Tj\n",
                    x_pos, y_pos, escaped_line
                );
                content.extend_from_slice(text_line_op.as_bytes());
            }
        } else {
            let text_width = FontMetricsHelper::estimate_text_width(text, &da.font_name, font_size);
            let x_pos = match quadding {
                1 => (padding_x + (usable_width - text_width) / 2.0).max(padding_x), // Center
                2 => (width - padding_x - text_width).max(padding_x),                // Right
                _ => padding_x,                                                      // Left
            };
            let y_pos = ((height - font_size) / 2.0 + font_size * 0.15).max(1.0);

            let escaped = escape_pdf_string(text);
            let text_op = format!("1 0 0 1 {:.2} {:.2} Tm\n({}) Tj\n", x_pos, y_pos, escaped);
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

        let res_dict = FontMetricsHelper::build_font_resource(&da.font_name);
        dict.insert("Resources".to_string(), PdfObject::Dictionary(res_dict));

        let stream_length = content.len();
        Ok(StreamObject {
            dict,
            data: content,
            stream_offset: 0,
            stream_length,
        })
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
