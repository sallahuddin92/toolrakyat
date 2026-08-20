use std::collections::BTreeMap;

use crate::error::{PdfError, PdfResult};
use crate::syntax::object::{PdfObject, StreamObject};

pub struct CheckboxAppearance;

impl CheckboxAppearance {
    /// Generates the off-state appearance Form XObject stream.
    pub fn generate_off_stream(rect: [f64; 4]) -> PdfResult<StreamObject> {
        let width = rect[2] - rect[0];
        let height = rect[3] - rect[1];

        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
            return Err(PdfError::InvalidOperation(
                "Invalid checkbox bounding box dimensions".into(),
            ));
        }

        // Draw a light grey/black border with white background
        let mut content = Vec::new();
        content.extend_from_slice(b"q\n1 1 1 rg\n");
        let bg_op = format!("0 0 {:.2} {:.2} re\nf\n", width, height);
        content.extend_from_slice(bg_op.as_bytes());

        content.extend_from_slice(b"0.5 0.5 0.5 RG\n1 w\n");
        let border_op = format!("0.5 0.5 {:.2} {:.2} re\nS\nQ\n", width - 1.0, height - 1.0);
        content.extend_from_slice(border_op.as_bytes());

        Self::wrap_stream(width, height, content)
    }

    /// Generates the on-state appearance Form XObject stream.
    pub fn generate_on_stream(rect: [f64; 4]) -> PdfResult<StreamObject> {
        let width = rect[2] - rect[0];
        let height = rect[3] - rect[1];

        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
            return Err(PdfError::InvalidOperation(
                "Invalid checkbox bounding box dimensions".into(),
            ));
        }

        let mut content = Vec::new();
        // Background and border
        content.extend_from_slice(b"q\n1 1 1 rg\n");
        let bg_op = format!("0 0 {:.2} {:.2} re\nf\n", width, height);
        content.extend_from_slice(bg_op.as_bytes());

        content.extend_from_slice(b"0 0 0 RG\n1 w\n");
        let border_op = format!("0.5 0.5 {:.2} {:.2} re\nS\n", width - 1.0, height - 1.0);
        content.extend_from_slice(border_op.as_bytes());

        // Checkmark path: from bottom-left to center-low to top-right
        let w = width;
        let h = height;
        let p1_x = w * 0.20;
        let p1_y = h * 0.50;
        let p2_x = w * 0.42;
        let p2_y = h * 0.22;
        let p3_x = w * 0.82;
        let p3_y = h * 0.78;

        let stroke_width = (w * 0.12).clamp(1.0, 3.0);
        let check_op = format!(
            "{:.2} w\n1 J\n1 j\n{:.2} {:.2} m\n{:.2} {:.2} l\n{:.2} {:.2} l\nS\nQ\n",
            stroke_width, p1_x, p1_y, p2_x, p2_y, p3_x, p3_y
        );
        content.extend_from_slice(check_op.as_bytes());

        Self::wrap_stream(width, height, content)
    }

    fn wrap_stream(width: f64, height: f64, content: Vec<u8>) -> PdfResult<StreamObject> {
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

        let stream_length = content.len();
        Ok(StreamObject {
            dict,
            data: content,
            stream_offset: 0,
            stream_length,
        })
    }
}
