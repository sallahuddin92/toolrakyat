use std::collections::BTreeMap;

use crate::annotation::types::{Annotation, AnnotationSubtype, LineEndingStyle};
use crate::document::object_store::ObjectStore;
use crate::error::{PdfError, PdfResult};
use crate::syntax::object::{ObjectRef, PdfObject};

const MAX_PAGE_ANNOTATIONS: usize = 2000;

pub struct AnnotationParser<'a, 'b> {
    store: &'a mut ObjectStore<'b>,
}

impl<'a, 'b> AnnotationParser<'a, 'b> {
    pub fn new(store: &'a mut ObjectStore<'b>) -> Self {
        Self { store }
    }

    /// Parses all annotations defined on a page dictionary.
    pub fn parse_page_annotations(
        &mut self,
        page_dict: &BTreeMap<String, PdfObject>,
        page_index: usize,
    ) -> PdfResult<Vec<Annotation>> {
        let annots_obj = match page_dict.get("Annots") {
            Some(obj) => obj,
            None => return Ok(Vec::new()),
        };

        let annots_arr = match annots_obj {
            PdfObject::Array(arr) => arr.clone(),
            PdfObject::Reference(r) => {
                let resolved = self.store.resolve(*r)?;
                resolved.as_array().map(|s| s.to_vec()).unwrap_or_default()
            }
            _ => return Ok(Vec::new()),
        };

        let mut annotations = Vec::new();
        for item in annots_arr.iter().take(MAX_PAGE_ANNOTATIONS) {
            match item {
                PdfObject::Reference(r) => {
                    if let Ok(annot) = self.parse_single_annotation(*r, page_index) {
                        annotations.push(annot);
                    }
                }
                PdfObject::Dictionary(dict) => {
                    let pseudo_ref = ObjectRef {
                        number: 0,
                        generation: 0,
                    };
                    if let Ok(annot) = self.parse_annotation_dict(pseudo_ref, dict, page_index) {
                        annotations.push(annot);
                    }
                }
                _ => {}
            }
        }

        Ok(annotations)
    }

    fn parse_single_annotation(
        &mut self,
        annot_ref: ObjectRef,
        page_index: usize,
    ) -> PdfResult<Annotation> {
        let resolved = self.store.resolve(annot_ref)?.clone();
        let dict = resolved.as_dict().ok_or_else(|| PdfError::TypeMismatch {
            expected: "dictionary",
            actual: resolved.type_name(),
        })?;

        self.parse_annotation_dict(annot_ref, dict, page_index)
    }

    fn parse_annotation_dict(
        &mut self,
        annot_ref: ObjectRef,
        dict: &BTreeMap<String, PdfObject>,
        page_index: usize,
    ) -> PdfResult<Annotation> {
        let subtype_str = dict
            .get("Subtype")
            .and_then(|v| v.as_name())
            .unwrap_or("Unknown");
        let subtype = AnnotationSubtype::from_name(subtype_str);

        let rect = match dict.get("Rect") {
            Some(obj) => self.parse_rect(obj)?,
            None => [0.0, 0.0, 0.0, 0.0],
        };

        let contents = dict.get("Contents").and_then(|v| v.as_string_lossy());
        let name = dict.get("NM").and_then(|v| v.as_string_lossy());

        let flags = dict
            .get("F")
            .and_then(|v| v.as_integer())
            .map_or(0, |i| i.max(0) as u32);

        let appearance_state = dict
            .get("AS")
            .and_then(|v| v.as_name())
            .map(|s| s.to_string());

        let color = self.parse_color(dict.get("C"))?;
        let interior_color = self.parse_color(dict.get("IC"))?;
        let border_width = dict
            .get("BS")
            .and_then(PdfObject::as_dict)
            .and_then(|bs| bs.get("W"))
            .and_then(PdfObject::as_real)
            .or_else(|| {
                dict.get("Border")
                    .and_then(PdfObject::as_array)
                    .and_then(|border| border.get(2))
                    .and_then(PdfObject::as_real)
            });
        if border_width.is_some_and(|width| !width.is_finite() || width < 0.0) {
            return Err(PdfError::InvalidOperation(
                "Annotation border width must be finite and non-negative".into(),
            ));
        }
        let line_points = if subtype == AnnotationSubtype::Line {
            let values = self.parse_number_array(dict.get("L"), 4)?;
            if values.len() != 4 || !values.iter().all(|value| value.is_finite()) {
                return Err(PdfError::InvalidOperation(
                    "Line annotation /L must contain four finite numbers".into(),
                ));
            }
            Some([values[0], values[1], values[2], values[3]])
        } else {
            None
        };
        let line_endings = if subtype == AnnotationSubtype::Line {
            Some(self.parse_line_endings(dict.get("LE"))?)
        } else {
            None
        };
        let quad_points = self.parse_number_array(dict.get("QuadPoints"), 8_000)?;
        if !quad_points.is_empty()
            && (quad_points.len() % 8 != 0 || !quad_points.iter().all(|value| value.is_finite()))
        {
            return Err(PdfError::InvalidOperation(
                "Annotation /QuadPoints are malformed".into(),
            ));
        }
        let ink_list = self.parse_ink_list(dict.get("InkList"))?;

        let is_invisible = (flags & (1 << 0)) != 0; // Bit 1
        let is_hidden = (flags & (1 << 1)) != 0; // Bit 2
        let is_print = (flags & (1 << 2)) != 0; // Bit 3

        Ok(Annotation {
            object_ref: annot_ref,
            page_index,
            subtype,
            rect,
            contents,
            name,
            flags,
            appearance_state,
            color,
            interior_color,
            border_width,
            line_points,
            line_endings,
            quad_points,
            ink_list,
            is_hidden,
            is_invisible,
            is_print,
        })
    }

    fn parse_rect(&mut self, obj: &PdfObject) -> PdfResult<[f64; 4]> {
        let arr = match obj {
            PdfObject::Array(a) => a.clone(),
            PdfObject::Reference(r) => {
                let resolved = self.store.resolve(*r)?;
                resolved.as_array().map(|s| s.to_vec()).unwrap_or_default()
            }
            _ => return Ok([0.0, 0.0, 0.0, 0.0]),
        };

        if arr.len() < 4 {
            return Ok([0.0, 0.0, 0.0, 0.0]);
        }

        let x1 = arr[0].as_real().unwrap_or(0.0);
        let y1 = arr[1].as_real().unwrap_or(0.0);
        let x2 = arr[2].as_real().unwrap_or(0.0);
        let y2 = arr[3].as_real().unwrap_or(0.0);

        Ok([x1, y1, x2, y2])
    }

    fn parse_color(&mut self, obj: Option<&PdfObject>) -> PdfResult<Option<Vec<f64>>> {
        let obj = match obj {
            Some(v) => v,
            None => return Ok(None),
        };

        let arr = match obj {
            PdfObject::Array(a) => a.clone(),
            PdfObject::Reference(r) => {
                let resolved = self.store.resolve(*r)?;
                resolved.as_array().map(|s| s.to_vec()).unwrap_or_default()
            }
            _ => return Ok(None),
        };

        let mut colors = Vec::new();
        for item in arr {
            if let Some(r) = item.as_real() {
                colors.push(r);
            }
        }

        if colors.is_empty() {
            Ok(None)
        } else {
            Ok(Some(colors))
        }
    }

    fn parse_number_array(
        &mut self,
        object: Option<&PdfObject>,
        maximum: usize,
    ) -> PdfResult<Vec<f64>> {
        let Some(object) = object else {
            return Ok(Vec::new());
        };
        let resolved = self.store.resolve_object(object)?;
        let array = resolved.as_array().ok_or_else(|| PdfError::TypeMismatch {
            expected: "numeric array",
            actual: resolved.type_name(),
        })?;
        if array.len() > maximum {
            return Err(PdfError::InvalidOperation(format!(
                "Annotation numeric array exceeds maximum of {maximum} values"
            )));
        }
        array
            .iter()
            .map(|value| {
                value.as_real().ok_or_else(|| {
                    PdfError::InvalidOperation("Annotation array value must be numeric".into())
                })
            })
            .collect()
    }

    fn parse_line_endings(
        &mut self,
        object: Option<&PdfObject>,
    ) -> PdfResult<[LineEndingStyle; 2]> {
        let Some(object) = object else {
            return Ok([LineEndingStyle::None, LineEndingStyle::None]);
        };
        let resolved = self.store.resolve_object(object)?;
        let array = resolved.as_array().ok_or_else(|| PdfError::TypeMismatch {
            expected: "line ending array",
            actual: resolved.type_name(),
        })?;
        if array.len() != 2 {
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
        Ok([parse(&array[0])?, parse(&array[1])?])
    }

    fn parse_ink_list(&mut self, object: Option<&PdfObject>) -> PdfResult<Vec<Vec<[f64; 2]>>> {
        let Some(object) = object else {
            return Ok(Vec::new());
        };
        let resolved = self.store.resolve_object(object)?;
        let paths = resolved.as_array().ok_or_else(|| PdfError::TypeMismatch {
            expected: "InkList array",
            actual: resolved.type_name(),
        })?;
        if paths.len() > 1_000 {
            return Err(PdfError::InvalidOperation(
                "InkList exceeds maximum of 1000 paths".into(),
            ));
        }
        let mut output = Vec::with_capacity(paths.len());
        let mut total = 0usize;
        for path in paths {
            let values = path.as_array().ok_or_else(|| PdfError::TypeMismatch {
                expected: "InkList coordinate array",
                actual: path.type_name(),
            })?;
            if values.is_empty() || values.len() % 2 != 0 || values.len() > 20_000 {
                return Err(PdfError::InvalidOperation(
                    "InkList path contains an invalid coordinate count".into(),
                ));
            }
            total = total
                .checked_add(values.len() / 2)
                .ok_or_else(|| PdfError::InvalidOperation("InkList point count overflow".into()))?;
            if total > 100_000 {
                return Err(PdfError::InvalidOperation(
                    "InkList exceeds maximum total point count".into(),
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
                if !x.is_finite() || !y.is_finite() {
                    return Err(PdfError::InvalidOperation(
                        "InkList coordinate must be finite".into(),
                    ));
                }
                points.push([x, y]);
            }
            output.push(points);
        }
        Ok(output)
    }
}
