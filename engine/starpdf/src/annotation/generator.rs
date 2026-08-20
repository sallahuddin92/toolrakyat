use std::collections::BTreeMap;

use crate::annotation::types::AnnotationSpec;
use crate::appearance::color::PdfColor;
use crate::appearance::fonts::FontMetricsHelper;
use crate::appearance::text_field::escape_pdf_string;
use crate::error::{PdfError, PdfResult};
use crate::syntax::object::{PdfObject, StreamObject};

pub const MAX_ANNOTATION_CONTENTS_LEN: usize = 1_048_576;
pub const MAX_ANNOTATION_URI_LEN: usize = 16_384;
pub const MAX_QUADPOINT_VALUES: usize = 8_000;
pub const MAX_INK_PATHS: usize = 1_000;
pub const MAX_INK_POINTS_PER_PATH: usize = 10_000;
pub const MAX_INK_POINTS_TOTAL: usize = 100_000;
pub const MAX_ANNOTATION_APPEARANCE_BYTES: usize = 256 * 1024;

pub struct AnnotationGenerator;

impl AnnotationGenerator {
    /// Generates the annotation dictionary and its associated `/AP /N` appearance stream (if applicable).
    pub fn generate_annotation_objects(
        spec: &AnnotationSpec,
    ) -> PdfResult<(BTreeMap<String, PdfObject>, Option<StreamObject>)> {
        let mut dict = BTreeMap::new();
        dict.insert("Type".to_string(), PdfObject::Name("Annot".to_string()));
        dict.insert("F".to_string(), PdfObject::Integer(4)); // Print flag

        let rect = spec.rect();
        if !rect.iter().all(|v| v.is_finite()) {
            return Err(PdfError::InvalidOperation(
                "Non-finite coordinates in annotation rectangle".into(),
            ));
        }
        if rect[2] <= rect[0] || rect[3] <= rect[1] {
            return Err(PdfError::InvalidOperation(
                "Annotation rectangle must have positive width and height".into(),
            ));
        }

        let width = rect[2] - rect[0];
        let height = rect[3] - rect[1];

        dict.insert(
            "Rect".to_string(),
            PdfObject::Array(vec![
                PdfObject::Real(rect[0]),
                PdfObject::Real(rect[1]),
                PdfObject::Real(rect[2]),
                PdfObject::Real(rect[3]),
            ]),
        );

        match spec {
            AnnotationSpec::FreeText {
                text,
                font_size,
                color,
                ..
            } => {
                Self::validate_contents(text)?;
                if font_size.is_some_and(|size| !size.is_finite()) {
                    return Err(PdfError::InvalidOperation(
                        "FreeText font size must be finite".into(),
                    ));
                }
                Self::validate_optional_color(color.as_deref(), "FreeText color")?;
                dict.insert(
                    "Subtype".to_string(),
                    PdfObject::Name("FreeText".to_string()),
                );
                dict.insert(
                    "Contents".to_string(),
                    PdfObject::String(text.as_bytes().to_vec()),
                );

                let sz = font_size.unwrap_or(12.0).clamp(6.0, 72.0);
                let col = color
                    .as_deref()
                    .and_then(PdfColor::parse_from_slice)
                    .unwrap_or_else(PdfColor::black);

                dict.insert(
                    "DA".to_string(),
                    PdfObject::String(
                        format!("/Helv {:.2} Tf {}", sz, col.to_fill_ops()).into_bytes(),
                    ),
                );

                let mut content = Vec::new();
                content.extend_from_slice(b"q\n1 1 1 rg\n");
                let bg_op = format!("0 0 {:.2} {:.2} re\nf\n", width, height);
                content.extend_from_slice(bg_op.as_bytes());

                content.extend_from_slice(b"0 0 0 RG\n1 w\n");
                let border_op = format!("0.5 0.5 {:.2} {:.2} re\nS\n", width - 1.0, height - 1.0);
                content.extend_from_slice(border_op.as_bytes());

                let color_op = format!("{}\n", col.to_fill_ops());
                content.extend_from_slice(color_op.as_bytes());

                let escaped = escape_pdf_string(text);
                let y_pos = ((height - sz) / 2.0).max(2.0);
                let text_op = format!(
                    "BT\n/Helv {:.2} Tf\n2.0 {:.2} Td\n({}) Tj\nET\nQ\n",
                    sz, y_pos, escaped
                );
                content.extend_from_slice(text_op.as_bytes());
                Self::validate_appearance_size(content.len())?;

                let mut stream_dict = Self::create_form_dict(width, height, content.len());
                let res_dict = FontMetricsHelper::build_font_resource("Helv");
                stream_dict.insert("Resources".to_string(), PdfObject::Dictionary(res_dict));

                let stream = StreamObject {
                    dict: stream_dict,
                    stream_offset: 0,
                    stream_length: content.len(),
                    data: content,
                };

                Ok((dict, Some(stream)))
            }
            AnnotationSpec::Square {
                stroke_color,
                fill_color,
                border_width,
                ..
            } => {
                Self::validate_optional_width(*border_width, "Square border width")?;
                Self::validate_optional_color(stroke_color.as_deref(), "Square stroke color")?;
                Self::validate_optional_color(fill_color.as_deref(), "Square fill color")?;
                dict.insert("Subtype".to_string(), PdfObject::Name("Square".to_string()));
                let bw = border_width.unwrap_or(1.0).clamp(0.1, 20.0);
                let s_col = stroke_color
                    .as_deref()
                    .and_then(PdfColor::parse_from_slice)
                    .unwrap_or_else(PdfColor::black);

                if let Some(col_arr) = stroke_color {
                    dict.insert("C".to_string(), Self::vec_to_pdf_array(col_arr));
                }
                if let Some(f_arr) = fill_color {
                    dict.insert("IC".to_string(), Self::vec_to_pdf_array(f_arr));
                }

                dict.insert(
                    "BS".to_string(),
                    PdfObject::Dictionary(BTreeMap::from([("W".to_string(), PdfObject::Real(bw))])),
                );

                let mut content = Vec::new();
                content.extend_from_slice(b"q\n");
                if let Some(f_col) = fill_color.as_deref().and_then(PdfColor::parse_from_slice) {
                    content.extend_from_slice(format!("{}\n", f_col.to_fill_ops()).as_bytes());
                }
                content.extend_from_slice(format!("{}\n", s_col.to_stroke_ops()).as_bytes());
                content.extend_from_slice(format!("{:.2} w\n", bw).as_bytes());

                let half_bw = bw / 2.0;
                let re_w = (width - bw).max(0.1);
                let re_h = (height - bw).max(0.1);
                let draw_cmd = if fill_color.is_some() { "B\n" } else { "S\n" };
                content.extend_from_slice(
                    format!(
                        "{:.2} {:.2} {:.2} {:.2} re\n{}Q\n",
                        half_bw, half_bw, re_w, re_h, draw_cmd
                    )
                    .as_bytes(),
                );
                Self::validate_appearance_size(content.len())?;

                let stream = StreamObject {
                    dict: Self::create_form_dict(width, height, content.len()),
                    stream_offset: 0,
                    stream_length: content.len(),
                    data: content,
                };

                Ok((dict, Some(stream)))
            }
            AnnotationSpec::Circle {
                stroke_color,
                fill_color,
                border_width,
                ..
            } => {
                Self::validate_optional_width(*border_width, "Circle border width")?;
                Self::validate_optional_color(stroke_color.as_deref(), "Circle stroke color")?;
                Self::validate_optional_color(fill_color.as_deref(), "Circle fill color")?;
                dict.insert("Subtype".to_string(), PdfObject::Name("Circle".to_string()));
                let bw = border_width.unwrap_or(1.0).clamp(0.1, 20.0);
                let s_col = stroke_color
                    .as_deref()
                    .and_then(PdfColor::parse_from_slice)
                    .unwrap_or_else(PdfColor::black);

                if let Some(col_arr) = stroke_color {
                    dict.insert("C".to_string(), Self::vec_to_pdf_array(col_arr));
                }
                if let Some(f_arr) = fill_color {
                    dict.insert("IC".to_string(), Self::vec_to_pdf_array(f_arr));
                }

                dict.insert(
                    "BS".to_string(),
                    PdfObject::Dictionary(BTreeMap::from([("W".to_string(), PdfObject::Real(bw))])),
                );

                let mut content = Vec::new();
                content.extend_from_slice(b"q\n");
                if let Some(f_col) = fill_color.as_deref().and_then(PdfColor::parse_from_slice) {
                    content.extend_from_slice(format!("{}\n", f_col.to_fill_ops()).as_bytes());
                }
                content.extend_from_slice(format!("{}\n", s_col.to_stroke_ops()).as_bytes());
                content.extend_from_slice(format!("{:.2} w\n", bw).as_bytes());

                let cx = width / 2.0;
                let cy = height / 2.0;
                let rx = ((width - bw) / 2.0).max(0.1);
                let ry = ((height - bw) / 2.0).max(0.1);

                Self::append_ellipse(&mut content, cx, cy, rx, ry);
                let draw_cmd = if fill_color.is_some() { "B\n" } else { "S\n" };
                content.extend_from_slice(format!("{}Q\n", draw_cmd).as_bytes());
                Self::validate_appearance_size(content.len())?;

                let stream = StreamObject {
                    dict: Self::create_form_dict(width, height, content.len()),
                    stream_offset: 0,
                    stream_length: content.len(),
                    data: content,
                };

                Ok((dict, Some(stream)))
            }
            AnnotationSpec::Line {
                line_points,
                stroke_color,
                stroke_width,
            } => {
                if !line_points.iter().all(|value| value.is_finite()) {
                    return Err(PdfError::InvalidOperation(
                        "Line annotation points must be finite".into(),
                    ));
                }
                Self::validate_optional_width(*stroke_width, "Line stroke width")?;
                Self::validate_optional_color(stroke_color.as_deref(), "Line stroke color")?;
                dict.insert("Subtype".to_string(), PdfObject::Name("Line".to_string()));
                dict.insert(
                    "L".to_string(),
                    PdfObject::Array(vec![
                        PdfObject::Real(line_points[0]),
                        PdfObject::Real(line_points[1]),
                        PdfObject::Real(line_points[2]),
                        PdfObject::Real(line_points[3]),
                    ]),
                );

                let sw = stroke_width.unwrap_or(1.0).clamp(0.1, 20.0);
                let s_col = stroke_color
                    .as_deref()
                    .and_then(PdfColor::parse_from_slice)
                    .unwrap_or_else(PdfColor::black);

                if let Some(col_arr) = stroke_color {
                    dict.insert("C".to_string(), Self::vec_to_pdf_array(col_arr));
                }

                let min_x = line_points[0].min(line_points[2]);
                let min_y = line_points[1].min(line_points[3]);

                let mut content = Vec::new();
                content.extend_from_slice(b"q\n");
                content.extend_from_slice(
                    format!("{}\n{:.2} w\n", s_col.to_stroke_ops(), sw).as_bytes(),
                );
                Self::validate_appearance_size(content.len())?;
                content.extend_from_slice(
                    format!(
                        "{:.2} {:.2} m\n{:.2} {:.2} l\nS\nQ\n",
                        line_points[0] - min_x,
                        line_points[1] - min_y,
                        line_points[2] - min_x,
                        line_points[3] - min_y
                    )
                    .as_bytes(),
                );

                let stream = StreamObject {
                    dict: Self::create_form_dict(width, height, content.len()),
                    stream_offset: 0,
                    stream_length: content.len(),
                    data: content,
                };

                Ok((dict, Some(stream)))
            }
            AnnotationSpec::Highlight {
                quad_points, color, ..
            } => {
                Self::validate_quad_points(quad_points)?;
                Self::validate_optional_color(color.as_deref(), "Highlight color")?;
                dict.insert(
                    "Subtype".to_string(),
                    PdfObject::Name("Highlight".to_string()),
                );
                dict.insert(
                    "QuadPoints".to_string(),
                    Self::vec_to_pdf_array(quad_points),
                );
                let col_arr = color.clone().unwrap_or_else(|| vec![1.0, 1.0, 0.0]); // Default yellow
                dict.insert("C".to_string(), Self::vec_to_pdf_array(&col_arr));

                // Viewer-derived appearance is standard for markup annotations
                Ok((dict, None))
            }
            AnnotationSpec::Underline {
                quad_points, color, ..
            } => {
                Self::validate_quad_points(quad_points)?;
                Self::validate_optional_color(color.as_deref(), "Underline color")?;
                dict.insert(
                    "Subtype".to_string(),
                    PdfObject::Name("Underline".to_string()),
                );
                dict.insert(
                    "QuadPoints".to_string(),
                    Self::vec_to_pdf_array(quad_points),
                );
                let col_arr = color.clone().unwrap_or_else(|| vec![0.0, 0.0, 0.0]);
                dict.insert("C".to_string(), Self::vec_to_pdf_array(&col_arr));

                Ok((dict, None))
            }
            AnnotationSpec::StrikeOut {
                quad_points, color, ..
            } => {
                Self::validate_quad_points(quad_points)?;
                Self::validate_optional_color(color.as_deref(), "StrikeOut color")?;
                dict.insert(
                    "Subtype".to_string(),
                    PdfObject::Name("StrikeOut".to_string()),
                );
                dict.insert(
                    "QuadPoints".to_string(),
                    Self::vec_to_pdf_array(quad_points),
                );
                let col_arr = color.clone().unwrap_or_else(|| vec![1.0, 0.0, 0.0]);
                dict.insert("C".to_string(), Self::vec_to_pdf_array(&col_arr));

                Ok((dict, None))
            }
            AnnotationSpec::Ink {
                ink_list,
                stroke_color,
                stroke_width,
                ..
            } => {
                Self::validate_ink_list(ink_list)?;
                Self::validate_optional_width(*stroke_width, "Ink stroke width")?;
                Self::validate_optional_color(stroke_color.as_deref(), "Ink stroke color")?;
                dict.insert("Subtype".to_string(), PdfObject::Name("Ink".to_string()));
                let mut ink_arrays = Vec::new();
                for path in ink_list {
                    let mut path_objs = Vec::new();
                    for pt in path {
                        path_objs.push(PdfObject::Real(pt[0]));
                        path_objs.push(PdfObject::Real(pt[1]));
                    }
                    ink_arrays.push(PdfObject::Array(path_objs));
                }
                dict.insert("InkList".to_string(), PdfObject::Array(ink_arrays));

                let sw = stroke_width.unwrap_or(1.0).clamp(0.1, 20.0);
                let s_col = stroke_color
                    .as_deref()
                    .and_then(PdfColor::parse_from_slice)
                    .unwrap_or_else(PdfColor::black);

                if let Some(col_arr) = stroke_color {
                    dict.insert("C".to_string(), Self::vec_to_pdf_array(col_arr));
                }

                let mut content = Vec::new();
                content.extend_from_slice(b"q\n");
                content.extend_from_slice(
                    format!("{}\n{:.2} w\n1 J\n1 j\n", s_col.to_stroke_ops(), sw).as_bytes(),
                );

                let min_x = rect[0];
                let min_y = rect[1];

                for path in ink_list {
                    for (i, pt) in path.iter().enumerate() {
                        let rel_x = pt[0] - min_x;
                        let rel_y = pt[1] - min_y;
                        if i == 0 {
                            content.extend_from_slice(
                                format!("{:.2} {:.2} m\n", rel_x, rel_y).as_bytes(),
                            );
                        } else {
                            content.extend_from_slice(
                                format!("{:.2} {:.2} l\n", rel_x, rel_y).as_bytes(),
                            );
                        }
                    }
                    content.extend_from_slice(b"S\n");
                }
                content.extend_from_slice(b"Q\n");
                Self::validate_appearance_size(content.len())?;

                let stream = StreamObject {
                    dict: Self::create_form_dict(width, height, content.len()),
                    stream_offset: 0,
                    stream_length: content.len(),
                    data: content,
                };

                Ok((dict, Some(stream)))
            }
            AnnotationSpec::Link { uri, .. } => {
                if uri.len() > MAX_ANNOTATION_URI_LEN {
                    return Err(PdfError::InvalidOperation(format!(
                        "Link URI exceeds maximum length of {MAX_ANNOTATION_URI_LEN} bytes"
                    )));
                }
                dict.insert("Subtype".to_string(), PdfObject::Name("Link".to_string()));
                dict.insert(
                    "Border".to_string(),
                    PdfObject::Array(vec![
                        PdfObject::Integer(0),
                        PdfObject::Integer(0),
                        PdfObject::Integer(0),
                    ]),
                );
                let mut action_dict = BTreeMap::new();
                action_dict.insert("Type".to_string(), PdfObject::Name("Action".to_string()));
                action_dict.insert("S".to_string(), PdfObject::Name("URI".to_string()));
                action_dict.insert(
                    "URI".to_string(),
                    PdfObject::String(uri.as_bytes().to_vec()),
                );
                dict.insert("A".to_string(), PdfObject::Dictionary(action_dict));

                Ok((dict, None))
            }
        }
    }

    fn create_form_dict(width: f64, height: f64, length: usize) -> BTreeMap<String, PdfObject> {
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
        dict.insert("Length".to_string(), PdfObject::Integer(length as i64));
        dict
    }

    fn validate_contents(contents: &str) -> PdfResult<()> {
        if contents.len() > MAX_ANNOTATION_CONTENTS_LEN {
            return Err(PdfError::InvalidOperation(format!(
                "Annotation contents exceed maximum length of {MAX_ANNOTATION_CONTENTS_LEN} bytes"
            )));
        }
        Ok(())
    }

    fn validate_optional_width(width: Option<f64>, label: &str) -> PdfResult<()> {
        if width.is_some_and(|value| !value.is_finite()) {
            return Err(PdfError::InvalidOperation(format!(
                "{label} must be finite"
            )));
        }
        Ok(())
    }

    fn validate_optional_color(color: Option<&[f64]>, label: &str) -> PdfResult<()> {
        if let Some(values) = color {
            if PdfColor::parse_from_slice(values).is_none() {
                return Err(PdfError::InvalidOperation(format!(
                    "{label} must contain 1, 3, or 4 finite components"
                )));
            }
        }
        Ok(())
    }

    fn validate_quad_points(quad_points: &[f64]) -> PdfResult<()> {
        if quad_points.is_empty()
            || quad_points.len() > MAX_QUADPOINT_VALUES
            || quad_points.len() % 8 != 0
        {
            return Err(PdfError::InvalidOperation(format!(
                "QuadPoints must contain 1..={} complete quadrilaterals",
                MAX_QUADPOINT_VALUES / 8
            )));
        }
        if !quad_points.iter().all(|value| value.is_finite()) {
            return Err(PdfError::InvalidOperation(
                "QuadPoints must contain only finite coordinates".into(),
            ));
        }
        Ok(())
    }

    fn validate_ink_list(ink_list: &[Vec<[f64; 2]>]) -> PdfResult<()> {
        if ink_list.is_empty() || ink_list.len() > MAX_INK_PATHS {
            return Err(PdfError::InvalidOperation(format!(
                "InkList must contain 1..={MAX_INK_PATHS} paths"
            )));
        }

        let mut total_points = 0usize;
        for path in ink_list {
            if path.is_empty() || path.len() > MAX_INK_POINTS_PER_PATH {
                return Err(PdfError::InvalidOperation(format!(
                    "Each InkList path must contain 1..={MAX_INK_POINTS_PER_PATH} points"
                )));
            }
            total_points = total_points
                .checked_add(path.len())
                .ok_or_else(|| PdfError::InvalidOperation("InkList point count overflow".into()))?;
            if total_points > MAX_INK_POINTS_TOTAL {
                return Err(PdfError::InvalidOperation(format!(
                    "InkList exceeds maximum total of {MAX_INK_POINTS_TOTAL} points"
                )));
            }
            if !path
                .iter()
                .flat_map(|point| point.iter())
                .all(|value| value.is_finite())
            {
                return Err(PdfError::InvalidOperation(
                    "InkList must contain only finite coordinates".into(),
                ));
            }
        }
        Ok(())
    }

    fn validate_appearance_size(length: usize) -> PdfResult<()> {
        if length > MAX_ANNOTATION_APPEARANCE_BYTES {
            return Err(PdfError::InvalidOperation(format!(
                "Annotation appearance exceeds maximum length of {MAX_ANNOTATION_APPEARANCE_BYTES} bytes"
            )));
        }
        Ok(())
    }

    fn append_ellipse(content: &mut Vec<u8>, cx: f64, cy: f64, rx: f64, ry: f64) {
        let cx_c = 0.552_284_749_8 * rx;
        let cy_c = 0.552_284_749_8 * ry;

        let op = format!(
            "{:.2} {:.2} m\n\
             {:.2} {:.2} {:.2} {:.2} {:.2} {:.2} c\n\
             {:.2} {:.2} {:.2} {:.2} {:.2} {:.2} c\n\
             {:.2} {:.2} {:.2} {:.2} {:.2} {:.2} c\n\
             {:.2} {:.2} {:.2} {:.2} {:.2} {:.2} c\n",
            cx,
            cy + ry,
            cx + cx_c,
            cy + ry,
            cx + rx,
            cy + cy_c,
            cx + rx,
            cy,
            cx + rx,
            cy - cy_c,
            cx + cx_c,
            cy - ry,
            cx,
            cy - ry,
            cx - cx_c,
            cy - ry,
            cx - rx,
            cy - cy_c,
            cx - rx,
            cy,
            cx - rx,
            cy + cy_c,
            cx - cx_c,
            cy + ry,
            cx,
            cy + ry
        );
        content.extend_from_slice(op.as_bytes());
    }

    fn vec_to_pdf_array(vals: &[f64]) -> PdfObject {
        PdfObject::Array(vals.iter().map(|v| PdfObject::Real(*v)).collect())
    }
}
