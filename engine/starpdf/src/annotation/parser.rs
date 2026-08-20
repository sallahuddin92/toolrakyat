use std::collections::BTreeMap;

use crate::annotation::types::{Annotation, AnnotationSubtype};
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
}
