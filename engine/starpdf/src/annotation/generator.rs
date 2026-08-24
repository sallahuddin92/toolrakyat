use std::collections::BTreeMap;

use crate::annotation::types::{AnnotationSpec, LineEndingStyle};
use crate::appearance::color::PdfColor;
use crate::appearance::fonts::FontMetricsHelper;
use crate::error::{PdfError, PdfResult};
use crate::font::Font;
use crate::syntax::object::{PdfObject, StreamObject};

pub const MAX_ANNOTATION_CONTENTS_LEN: usize = 1_048_576;
pub const MAX_ANNOTATION_URI_LEN: usize = 16_384;
pub const MAX_QUADPOINT_VALUES: usize = 8_000;
pub const MAX_INK_PATHS: usize = 1_000;
pub const MAX_INK_POINTS_PER_PATH: usize = 10_000;
pub const MAX_INK_POINTS_TOTAL: usize = 100_000;
pub const MAX_ANNOTATION_APPEARANCE_BYTES: usize = 256 * 1024;

pub struct AnnotationGenerator;

pub enum AnnotationAppearance {
    Regenerated(StreamObject),
    NotRequired,
    Unsupported,
}

#[derive(Clone, Copy)]
enum MarkupAppearance {
    Highlight,
    Underline,
    StrikeOut,
}

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
                dict.insert("Contents".to_string(), PdfObject::text_string(text));

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

                // Transparent appearance: only the text itself is painted.
                // No opaque background fill and no rectangle border stroke.
                // /Border [0 0 0] additionally suppresses viewer-synthesized borders.
                dict.insert(
                    "Border".to_string(),
                    PdfObject::Array(vec![
                        PdfObject::Integer(0),
                        PdfObject::Integer(0),
                        PdfObject::Integer(0),
                    ]),
                );

                let mut content = Vec::new();
                content.extend_from_slice(b"q\n");

                let color_op = format!("{}\n", col.to_fill_ops());
                content.extend_from_slice(color_op.as_bytes());

                let y_pos = ((height - sz) / 2.0).max(2.0);
                match text.as_str() {
                    "✓" => content.extend_from_slice(
                        format!(
                            "{:.2} w\n2.0 {:.2} m\n{:.2} 2.0 l\n{:.2} {:.2} l\nS\nQ\n",
                            (sz / 8.0).clamp(1.0, 3.0),
                            height * 0.48,
                            width * 0.38,
                            width - 2.0,
                            height - 2.0,
                        )
                        .as_bytes(),
                    ),
                    "✕" => content.extend_from_slice(
                        format!(
                            "{:.2} w\n2.0 2.0 m\n{:.2} {:.2} l\nS\n2.0 {:.2} m\n{:.2} 2.0 l\nS\nQ\n",
                            (sz / 8.0).clamp(1.0, 3.0),
                            width - 2.0,
                            height - 2.0,
                            height - 2.0,
                            width - 2.0,
                        )
                        .as_bytes(),
                    ),
                    _ => {
                        let Ok(operand) = Self::win_ansi_text_operand(text) else {
                            // The mutation engine owns adaptive font object allocation. Returning
                            // no fast-path appearance asks it to use the shared shaped runtime.
                            return Ok((dict, None));
                        };
                        let text_op = format!(
                            "BT\n/Helv {:.2} Tf\n2.0 {:.2} Td\n{} Tj\nET\nQ\n",
                            sz, y_pos, operand
                        );
                        content.extend_from_slice(text_op.as_bytes());
                    }
                }
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
                fill_color,
                stroke_width,
                line_endings,
                contents,
            } => {
                if !line_points.iter().all(|value| value.is_finite()) {
                    return Err(PdfError::InvalidOperation(
                        "Line annotation points must be finite".into(),
                    ));
                }
                Self::validate_optional_width(*stroke_width, "Line stroke width")?;
                Self::validate_optional_color(stroke_color.as_deref(), "Line stroke color")?;
                Self::validate_optional_color(fill_color.as_deref(), "Line fill color")?;
                if let Some(value) = contents {
                    Self::validate_contents(value)?;
                    dict.insert("Contents".to_string(), PdfObject::text_string(value));
                }
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
                if let Some(col_arr) = fill_color {
                    dict.insert("IC".to_string(), Self::vec_to_pdf_array(col_arr));
                }
                dict.insert(
                    "LE".to_string(),
                    PdfObject::Array(
                        line_endings
                            .iter()
                            .map(|ending| PdfObject::Name(ending.as_name().to_string()))
                            .collect(),
                    ),
                );
                dict.insert(
                    "BS".to_string(),
                    PdfObject::Dictionary(BTreeMap::from([
                        ("Type".to_string(), PdfObject::Name("Border".to_string())),
                        ("W".to_string(), PdfObject::Real(sw)),
                        ("S".to_string(), PdfObject::Name("S".to_string())),
                    ])),
                );
                dict.insert(
                    "Border".to_string(),
                    PdfObject::Array(vec![
                        PdfObject::Integer(0),
                        PdfObject::Integer(0),
                        PdfObject::Real(sw),
                    ]),
                );

                let mut content = Vec::new();
                content.extend_from_slice(b"q\n");
                content.extend_from_slice(
                    format!("{}\n{:.2} w\n", s_col.to_stroke_ops(), sw).as_bytes(),
                );
                let fill = fill_color
                    .as_deref()
                    .and_then(PdfColor::parse_from_slice)
                    .unwrap_or(s_col);
                content.extend_from_slice(format!("{}\n", fill.to_fill_ops()).as_bytes());

                let start = [line_points[0] - rect[0], line_points[1] - rect[1]];
                let end = [line_points[2] - rect[0], line_points[3] - rect[1]];
                content.extend_from_slice(
                    format!(
                        "{:.2} {:.2} m\n{:.2} {:.2} l\nS\n",
                        start[0], start[1], end[0], end[1]
                    )
                    .as_bytes(),
                );
                Self::append_line_ending(&mut content, start, end, line_endings[0], sw, true);
                Self::append_line_ending(&mut content, end, start, line_endings[1], sw, false);
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

                let stream = Self::generate_markup_stream(
                    rect,
                    quad_points,
                    &col_arr,
                    MarkupAppearance::Highlight,
                )?;
                Ok((dict, Some(stream)))
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

                let stream = Self::generate_markup_stream(
                    rect,
                    quad_points,
                    &col_arr,
                    MarkupAppearance::Underline,
                )?;
                Ok((dict, Some(stream)))
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

                let stream = Self::generate_markup_stream(
                    rect,
                    quad_points,
                    &col_arr,
                    MarkupAppearance::StrikeOut,
                )?;
                Ok((dict, Some(stream)))
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

    /// Reconstructs a supported annotation from its current dictionary and regenerates `/AP /N`.
    /// The caller owns indirect-object allocation so regeneration remains part of the atomic plan.
    pub fn regenerate_from_dictionary(
        dict: &BTreeMap<String, PdfObject>,
    ) -> PdfResult<AnnotationAppearance> {
        let subtype = dict
            .get("Subtype")
            .and_then(PdfObject::as_name)
            .ok_or_else(|| PdfError::InvalidOperation("Annotation is missing /Subtype".into()))?;
        if subtype == "Link" || subtype == "Text" {
            return Ok(AnnotationAppearance::NotRequired);
        }
        let rect = Self::dict_rect(dict)?;
        let color = Self::dict_number_array(dict.get("C"));
        let fill_color = Self::dict_number_array(dict.get("IC"));
        let border_width = Self::dict_border_width(dict);
        let contents = dict.get("Contents").and_then(PdfObject::as_string_lossy);

        let spec = match subtype {
            "FreeText" => {
                let font_size = dict
                    .get("DA")
                    .and_then(PdfObject::as_string_lossy)
                    .and_then(|value| crate::appearance::DefaultAppearance::parse(&value).ok())
                    .map(|da| da.font_size);
                AnnotationSpec::FreeText {
                    rect,
                    text: contents.unwrap_or_default(),
                    font_size,
                    color,
                }
            }
            "Square" => AnnotationSpec::Square {
                rect,
                stroke_color: color,
                fill_color,
                border_width,
            },
            "Circle" => AnnotationSpec::Circle {
                rect,
                stroke_color: color,
                fill_color,
                border_width,
            },
            "Line" => {
                let values = Self::dict_number_array(dict.get("L")).ok_or_else(|| {
                    PdfError::InvalidOperation(
                        "Line annotation is missing valid /L endpoints".into(),
                    )
                })?;
                if values.len() != 4 {
                    return Err(PdfError::InvalidOperation(
                        "Line annotation /L must contain exactly four numbers".into(),
                    ));
                }
                let line_endings = Self::dict_line_endings(dict)?;
                AnnotationSpec::Line {
                    line_points: [values[0], values[1], values[2], values[3]],
                    stroke_color: color,
                    fill_color,
                    stroke_width: border_width,
                    line_endings,
                    contents,
                }
            }
            "Highlight" | "Underline" | "StrikeOut" => {
                let quad_points =
                    Self::dict_number_array(dict.get("QuadPoints")).ok_or_else(|| {
                        PdfError::InvalidOperation(format!(
                            "{subtype} annotation is missing valid /QuadPoints"
                        ))
                    })?;
                match subtype {
                    "Highlight" => AnnotationSpec::Highlight {
                        rect,
                        quad_points,
                        color,
                    },
                    "Underline" => AnnotationSpec::Underline {
                        rect,
                        quad_points,
                        color,
                    },
                    _ => AnnotationSpec::StrikeOut {
                        rect,
                        quad_points,
                        color,
                    },
                }
            }
            "Ink" => AnnotationSpec::Ink {
                rect,
                ink_list: Self::dict_ink_list(dict)?,
                stroke_color: color,
                stroke_width: border_width,
            },
            _ => return Ok(AnnotationAppearance::Unsupported),
        };

        let (_, stream) = Self::generate_annotation_objects(&spec)?;
        match stream {
            Some(stream) => Ok(AnnotationAppearance::Regenerated(stream)),
            None => Ok(AnnotationAppearance::Unsupported),
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

    fn dict_number_array(obj: Option<&PdfObject>) -> Option<Vec<f64>> {
        let values = obj?.as_array()?;
        let mut result = Vec::with_capacity(values.len());
        for value in values {
            result.push(value.as_real()?);
        }
        Some(result)
    }

    fn dict_rect(dict: &BTreeMap<String, PdfObject>) -> PdfResult<[f64; 4]> {
        let values = Self::dict_number_array(dict.get("Rect")).ok_or_else(|| {
            PdfError::InvalidOperation("Annotation is missing a direct numeric /Rect".into())
        })?;
        if values.len() != 4 {
            return Err(PdfError::InvalidOperation(
                "Annotation /Rect must contain exactly four numbers".into(),
            ));
        }
        let rect = [values[0], values[1], values[2], values[3]];
        if !rect.iter().all(|value| value.is_finite()) || rect[2] <= rect[0] || rect[3] <= rect[1] {
            return Err(PdfError::InvalidOperation(
                "Annotation /Rect must have finite positive dimensions".into(),
            ));
        }
        Ok(rect)
    }

    fn dict_border_width(dict: &BTreeMap<String, PdfObject>) -> Option<f64> {
        dict.get("BS")
            .and_then(PdfObject::as_dict)
            .and_then(|bs| bs.get("W"))
            .and_then(PdfObject::as_real)
            .or_else(|| {
                dict.get("Border")
                    .and_then(PdfObject::as_array)
                    .and_then(|values| values.get(2))
                    .and_then(PdfObject::as_real)
            })
    }

    fn dict_line_endings(dict: &BTreeMap<String, PdfObject>) -> PdfResult<[LineEndingStyle; 2]> {
        let Some(values) = dict.get("LE").and_then(PdfObject::as_array) else {
            return Ok([LineEndingStyle::None, LineEndingStyle::None]);
        };
        if values.len() != 2 {
            return Err(PdfError::InvalidOperation(
                "Line annotation /LE must contain two names".into(),
            ));
        }
        let parse = |value: &PdfObject| {
            value
                .as_name()
                .and_then(LineEndingStyle::from_name)
                .ok_or_else(|| PdfError::InvalidOperation("Unsupported line ending style".into()))
        };
        Ok([parse(&values[0])?, parse(&values[1])?])
    }

    fn dict_ink_list(dict: &BTreeMap<String, PdfObject>) -> PdfResult<Vec<Vec<[f64; 2]>>> {
        let paths = dict
            .get("InkList")
            .and_then(PdfObject::as_array)
            .ok_or_else(|| {
                PdfError::InvalidOperation("Ink annotation is missing /InkList".into())
            })?;
        let mut result = Vec::with_capacity(paths.len().min(MAX_INK_PATHS));
        for path in paths {
            let values = path.as_array().ok_or_else(|| {
                PdfError::InvalidOperation("InkList path must be an array".into())
            })?;
            if values.len() % 2 != 0 {
                return Err(PdfError::InvalidOperation(
                    "InkList path must contain coordinate pairs".into(),
                ));
            }
            let mut points = Vec::with_capacity(values.len() / 2);
            for pair in values.chunks_exact(2) {
                let x = pair[0].as_real().ok_or_else(|| {
                    PdfError::InvalidOperation("InkList coordinate must be numeric".into())
                })?;
                let y = pair[1].as_real().ok_or_else(|| {
                    PdfError::InvalidOperation("InkList coordinate must be numeric".into())
                })?;
                points.push([x, y]);
            }
            result.push(points);
        }
        Self::validate_ink_list(&result)?;
        Ok(result)
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

    fn win_ansi_text_operand(text: &str) -> PdfResult<String> {
        let encoded = Font::standard_fallback("Helvetica").encode_text(text)?;
        let mut operand = String::with_capacity(encoded.len().saturating_mul(2).saturating_add(2));
        operand.push('<');
        for byte in encoded {
            use std::fmt::Write as _;
            write!(&mut operand, "{byte:02X}").map_err(|_| {
                PdfError::InvalidOperation("Failed to encode FreeText appearance".into())
            })?;
        }
        operand.push('>');
        Ok(operand)
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

    fn generate_markup_stream(
        rect: [f64; 4],
        quad_points: &[f64],
        color: &[f64],
        kind: MarkupAppearance,
    ) -> PdfResult<StreamObject> {
        let width = rect[2] - rect[0];
        let height = rect[3] - rect[1];
        let pdf_color = PdfColor::parse_from_slice(color).ok_or_else(|| {
            PdfError::InvalidOperation("Markup appearance color is invalid".into())
        })?;
        let mut content = Vec::with_capacity(quad_points.len().saturating_mul(12));
        content.extend_from_slice(b"q\n");

        for quad in quad_points.chunks_exact(8) {
            let points = [
                [quad[0] - rect[0], quad[1] - rect[1]],
                [quad[2] - rect[0], quad[3] - rect[1]],
                [quad[4] - rect[0], quad[5] - rect[1]],
                [quad[6] - rect[0], quad[7] - rect[1]],
            ];
            match kind {
                MarkupAppearance::Highlight => {
                    content.extend_from_slice(format!("{}\n", pdf_color.to_fill_ops()).as_bytes());
                    content.extend_from_slice(
                        format!(
                            "{:.2} {:.2} m\n{:.2} {:.2} l\n{:.2} {:.2} l\n{:.2} {:.2} l\nh\nf\n",
                            points[0][0],
                            points[0][1],
                            points[1][0],
                            points[1][1],
                            points[3][0],
                            points[3][1],
                            points[2][0],
                            points[2][1]
                        )
                        .as_bytes(),
                    );
                }
                MarkupAppearance::Underline | MarkupAppearance::StrikeOut => {
                    let min_x = points
                        .iter()
                        .map(|point| point[0])
                        .fold(f64::INFINITY, f64::min);
                    let max_x = points
                        .iter()
                        .map(|point| point[0])
                        .fold(f64::NEG_INFINITY, f64::max);
                    let min_y = points
                        .iter()
                        .map(|point| point[1])
                        .fold(f64::INFINITY, f64::min);
                    let max_y = points
                        .iter()
                        .map(|point| point[1])
                        .fold(f64::NEG_INFINITY, f64::max);
                    let y = match kind {
                        MarkupAppearance::Underline => min_y + 0.5,
                        MarkupAppearance::StrikeOut => f64::midpoint(min_y, max_y),
                        MarkupAppearance::Highlight => min_y,
                    };
                    let line_width = ((max_y - min_y) * 0.08).clamp(0.5, 3.0);
                    content.extend_from_slice(
                        format!(
                            "{}\n{:.2} w\n{:.2} {:.2} m\n{:.2} {:.2} l\nS\n",
                            pdf_color.to_stroke_ops(),
                            line_width,
                            min_x,
                            y,
                            max_x,
                            y
                        )
                        .as_bytes(),
                    );
                }
            }
        }
        content.extend_from_slice(b"Q\n");
        Self::validate_appearance_size(content.len())?;
        let dict = Self::create_form_dict(width, height, content.len());
        Ok(StreamObject {
            dict,
            stream_offset: 0,
            stream_length: content.len(),
            data: content,
        })
    }

    fn append_line_ending(
        content: &mut Vec<u8>,
        point: [f64; 2],
        toward: [f64; 2],
        ending: LineEndingStyle,
        stroke_width: f64,
        _is_start: bool,
    ) {
        if ending == LineEndingStyle::None {
            return;
        }
        let dx = toward[0] - point[0];
        let dy = toward[1] - point[1];
        let length = (dx * dx + dy * dy).sqrt();
        if !length.is_finite() || length <= f64::EPSILON {
            return;
        }
        let ux = dx / length;
        let uy = dy / length;
        let px = -uy;
        let py = ux;
        let size = (stroke_width * 4.0).clamp(4.0, 16.0);
        let back = [point[0] + ux * size, point[1] + uy * size];
        let left = [back[0] + px * size * 0.55, back[1] + py * size * 0.55];
        let right = [back[0] - px * size * 0.55, back[1] - py * size * 0.55];
        content.extend_from_slice(b"q\n");
        match ending {
            LineEndingStyle::Square => content.extend_from_slice(
                format!(
                    "{:.2} {:.2} {:.2} {:.2} re\nB\n",
                    point[0] - size / 2.0,
                    point[1] - size / 2.0,
                    size,
                    size
                )
                .as_bytes(),
            ),
            LineEndingStyle::Circle => {
                Self::append_ellipse(content, point[0], point[1], size / 2.0, size / 2.0);
                content.extend_from_slice(b"B\n");
            }
            LineEndingStyle::Diamond => content.extend_from_slice(
                format!(
                    "{:.2} {:.2} m\n{:.2} {:.2} l\n{:.2} {:.2} l\n{:.2} {:.2} l\nh\nB\n",
                    point[0],
                    point[1] + size / 2.0,
                    point[0] + size / 2.0,
                    point[1],
                    point[0],
                    point[1] - size / 2.0,
                    point[0] - size / 2.0,
                    point[1]
                )
                .as_bytes(),
            ),
            LineEndingStyle::OpenArrow => content.extend_from_slice(
                format!(
                    "{:.2} {:.2} m\n{:.2} {:.2} l\n{:.2} {:.2} l\nS\n",
                    left[0], left[1], point[0], point[1], right[0], right[1]
                )
                .as_bytes(),
            ),
            LineEndingStyle::ClosedArrow => content.extend_from_slice(
                format!(
                    "{:.2} {:.2} m\n{:.2} {:.2} l\n{:.2} {:.2} l\nh\nB\n",
                    left[0], left[1], point[0], point[1], right[0], right[1]
                )
                .as_bytes(),
            ),
            LineEndingStyle::None => {}
        }
        content.extend_from_slice(b"Q\n");
    }

    fn vec_to_pdf_array(vals: &[f64]) -> PdfObject {
        PdfObject::Array(vals.iter().map(|v| PdfObject::Real(*v)).collect())
    }
}
