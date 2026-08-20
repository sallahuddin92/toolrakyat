use std::collections::BTreeMap;

use crate::error::{PdfError, PdfResult};
use crate::syntax::object::{PdfObject, StreamObject};

pub struct RadioAppearance;

impl RadioAppearance {
    /// Generates the off-state appearance Form XObject stream for a radio button.
    pub fn generate_off_stream(rect: [f64; 4]) -> PdfResult<StreamObject> {
        let width = rect[2] - rect[0];
        let height = rect[3] - rect[1];

        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
            return Err(PdfError::InvalidOperation(
                "Invalid radio button bounding box dimensions".into(),
            ));
        }

        let cx = width / 2.0;
        let cy = height / 2.0;
        let r = (cx.min(cy) - 1.0).max(1.0);

        let mut content = Vec::new();
        content.extend_from_slice(b"q\n1 1 1 rg\n0.5 0.5 0.5 RG\n1 w\n");
        Self::append_circle(&mut content, cx, cy, r);
        content.extend_from_slice(b"B\nQ\n");

        Self::wrap_stream(width, height, content)
    }

    /// Generates the on-state appearance Form XObject stream for a radio button.
    pub fn generate_on_stream(rect: [f64; 4]) -> PdfResult<StreamObject> {
        let width = rect[2] - rect[0];
        let height = rect[3] - rect[1];

        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
            return Err(PdfError::InvalidOperation(
                "Invalid radio button bounding box dimensions".into(),
            ));
        }

        let cx = width / 2.0;
        let cy = height / 2.0;
        let r = (cx.min(cy) - 1.0).max(1.0);
        let inner_r = (r * 0.45).max(1.0);

        let mut content = Vec::new();
        // Outer circle
        content.extend_from_slice(b"q\n1 1 1 rg\n0 0 0 RG\n1 w\n");
        Self::append_circle(&mut content, cx, cy, r);
        content.extend_from_slice(b"B\n");

        // Inner filled dot
        content.extend_from_slice(b"0 0 0 rg\n");
        Self::append_circle(&mut content, cx, cy, inner_r);
        content.extend_from_slice(b"f\nQ\n");

        Self::wrap_stream(width, height, content)
    }

    fn append_circle(content: &mut Vec<u8>, cx: f64, cy: f64, r: f64) {
        let c = 0.552_284_749_8 * r;
        let op = format!(
            "{:.2} {:.2} m\n\
             {:.2} {:.2} {:.2} {:.2} {:.2} {:.2} c\n\
             {:.2} {:.2} {:.2} {:.2} {:.2} {:.2} c\n\
             {:.2} {:.2} {:.2} {:.2} {:.2} {:.2} c\n\
             {:.2} {:.2} {:.2} {:.2} {:.2} {:.2} c\n",
            cx,
            cy + r,
            cx + c,
            cy + r,
            cx + r,
            cy + c,
            cx + r,
            cy,
            cx + r,
            cy - c,
            cx + c,
            cy - r,
            cx,
            cy - r,
            cx - c,
            cy - r,
            cx - r,
            cy - c,
            cx - r,
            cy,
            cx - r,
            cy + c,
            cx - c,
            cy + r,
            cx,
            cy + r
        );
        content.extend_from_slice(op.as_bytes());
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
