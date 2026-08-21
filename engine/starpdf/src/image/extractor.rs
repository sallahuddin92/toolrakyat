use std::collections::{BTreeMap, HashMap};

use crate::content::operand::ContentOperand;
use crate::content::operator::ContentOperator;
use crate::content::parser::ContentParser;
use crate::document::{ObjectStore, PdfDocument};
use crate::error::PdfResult;
use crate::filter::flate::FlateDecoder;
use crate::filter::limits::DecompressLimits;
use crate::image::types::ImageXObjectInfo;
use crate::syntax::object::{ObjectRef, PdfObject};

/// Transformation matrix: `[a, b, c, d, e, f]` representing:
/// `[ a b 0 ]`
/// `[ c d 0 ]`
/// `[ e f 1 ]`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix2D {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

impl Matrix2D {
    pub const fn identity() -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: 0.0,
            f: 0.0,
        }
    }

    pub fn multiply(&self, other: &Self) -> Self {
        Self {
            a: self.a * other.a + self.b * other.c,
            b: self.a * other.b + self.b * other.d,
            c: self.c * other.a + self.d * other.c,
            d: self.c * other.b + self.d * other.d,
            e: self.e * other.a + self.f * other.c + other.e,
            f: self.e * other.b + self.f * other.d + other.f,
        }
    }

    pub fn transform_point(&self, x: f64, y: f64) -> (f64, f64) {
        (
            self.a * x + self.c * y + self.e,
            self.b * x + self.d * y + self.f,
        )
    }

    pub fn transform_unit_box(&self) -> [f64; 4] {
        let p0 = self.transform_point(0.0, 0.0);
        let p1 = self.transform_point(1.0, 0.0);
        let p2 = self.transform_point(0.0, 1.0);
        let p3 = self.transform_point(1.0, 1.0);

        let min_x = p0.0.min(p1.0).min(p2.0).min(p3.0);
        let min_y = p0.1.min(p1.1).min(p2.1).min(p3.1);
        let max_x = p0.0.max(p1.0).max(p2.0).max(p3.0);
        let max_y = p0.1.max(p1.1).max(p2.1).max(p3.1);

        [min_x, min_y, max_x, max_y]
    }

    pub const fn to_array(&self) -> [f64; 6] {
        [self.a, self.b, self.c, self.d, self.e, self.f]
    }
}

/// Discovers all Image XObjects across document pages.
pub struct ImageExtractor<'a, 'b> {
    store: &'a mut ObjectStore<'b>,
    page_refs: Vec<ObjectRef>,
}

impl<'a, 'b> ImageExtractor<'a, 'b> {
    pub fn new(doc: &'a mut PdfDocument<'b>) -> PdfResult<Self> {
        let page_refs = doc.page_refs()?.clone();
        let store = doc.store_mut();
        Ok(Self { store, page_refs })
    }

    pub fn from_store(store: &'a mut ObjectStore<'b>, page_refs: &[ObjectRef]) -> Self {
        Self {
            store,
            page_refs: page_refs.to_vec(),
        }
    }

    /// Extracts all Image XObjects from the entire document.
    pub fn extract_all_images(&mut self) -> PdfResult<Vec<ImageXObjectInfo>> {
        let mut all_images = Vec::new();
        let mut object_counts: HashMap<ObjectRef, usize> = HashMap::new();

        let num_pages = self.page_refs.len();
        for page_idx in 0..num_pages {
            if let Ok(images) = self.extract_page_images(page_idx) {
                for img in &images {
                    *object_counts.entry(img.object_ref).or_insert(0) += 1;
                }
                all_images.extend(images);
            }
        }

        for img in &mut all_images {
            if let Some(&count) = object_counts.get(&img.object_ref) {
                img.is_shared = count > 1;
            }
        }

        Ok(all_images)
    }

    /// Extracts all Image XObjects discovered on a given page.
    pub fn extract_page_images(&mut self, page_index: usize) -> PdfResult<Vec<ImageXObjectInfo>> {
        if page_index >= self.page_refs.len() {
            return Err(crate::error::PdfError::PageNotFound(page_index));
        }

        let page_ref = self.page_refs[page_index];
        let page_obj = self.store.resolve(page_ref)?.clone();
        let page_dict =
            page_obj
                .as_dict()
                .cloned()
                .ok_or_else(|| crate::error::PdfError::TypeMismatch {
                    expected: "dictionary",
                    actual: page_obj.type_name(),
                })?;

        // 1. Resolve XObject resource mapping from /Resources
        let mut xobjects = BTreeMap::new();
        if let Some(res_obj) = page_dict.get("Resources") {
            let res_resolved = self.store.resolve_object(res_obj)?;
            if let Some(res_dict) = res_resolved.as_dict() {
                if let Some(xo_obj) = res_dict.get("XObject") {
                    let xo_resolved = self.store.resolve_object(xo_obj)?;
                    if let Some(xo_dict) = xo_resolved.as_dict() {
                        for (name, val) in xo_dict {
                            if let Some(obj_ref) = val.as_reference() {
                                xobjects.insert(name.clone(), obj_ref);
                            }
                        }
                    }
                }
            }
        }

        // 2. Resolve content streams
        let content_streams = self.get_page_content_streams(&page_dict)?;
        let mut discovered = Vec::new();

        for (stream_index, stream_bytes) in content_streams.into_iter().enumerate() {
            let mut parser = ContentParser::from_bytes(&stream_bytes);
            let instructions = match parser.parse_instructions() {
                Ok(instrs) => instrs,
                Err(_) => continue,
            };

            let mut ctm = Matrix2D::identity();
            let mut ctm_stack: Vec<Matrix2D> = Vec::new();

            for (instruction_index, instr) in instructions.into_iter().enumerate() {
                match instr.operator {
                    ContentOperator::Q => {
                        ctm_stack.push(ctm);
                    }
                    ContentOperator::QEnd => {
                        if let Some(saved) = ctm_stack.pop() {
                            ctm = saved;
                        }
                    }
                    ContentOperator::Cm => {
                        if instr.operands.len() == 6 {
                            let a = instr.operands[0].as_f64().unwrap_or(1.0);
                            let b = instr.operands[1].as_f64().unwrap_or(0.0);
                            let c = instr.operands[2].as_f64().unwrap_or(0.0);
                            let d = instr.operands[3].as_f64().unwrap_or(1.0);
                            let e = instr.operands[4].as_f64().unwrap_or(0.0);
                            let f = instr.operands[5].as_f64().unwrap_or(0.0);
                            let m = Matrix2D { a, b, c, d, e, f };
                            ctm = m.multiply(&ctm);
                        }
                    }
                    ContentOperator::Do => {
                        if let Some(ContentOperand::Name(ref res_name)) = instr.operands.first() {
                            if let Some(&obj_ref) = xobjects.get(res_name) {
                                if let Ok(resolved) = self.store.resolve(obj_ref) {
                                    let resolved_obj = resolved.clone();
                                    if let Some(stream_obj) = resolved_obj.as_stream() {
                                        let subtype = stream_obj
                                            .dict
                                            .get("Subtype")
                                            .and_then(PdfObject::as_name)
                                            .unwrap_or("");

                                        if subtype == "Image" {
                                            let width = stream_obj
                                                .dict
                                                .get("Width")
                                                .and_then(PdfObject::as_i64)
                                                .unwrap_or(0)
                                                as u32;
                                            let height = stream_obj
                                                .dict
                                                .get("Height")
                                                .and_then(PdfObject::as_i64)
                                                .unwrap_or(0)
                                                as u32;
                                            let color_space = match stream_obj.dict.get("ColorSpace") {
                                                Some(PdfObject::Name(n)) => n.clone(),
                                                Some(PdfObject::Array(_)) => {
                                                    "Indexed/Array".into()
                                                }
                                                Some(_) => "Unknown".into(),
                                                None => "DeviceRGB".into(),
                                            };
                                            let bits_per_component = stream_obj
                                                .dict
                                                .get("BitsPerComponent")
                                                .and_then(PdfObject::as_i64)
                                                .unwrap_or(8)
                                                as u32;
                                            let filter = stream_obj
                                                .dict
                                                .get("Filter")
                                                .and_then(|f| f.as_name().map(ToString::to_string));

                                            let transform = ctm.to_array();
                                            let rect = ctm.transform_unit_box();
                                            let image_id = format!(
                                                "img_p{page_index}_s{stream_index}_i{instruction_index}_{res_name}"
                                            );

                                            discovered.push(ImageXObjectInfo {
                                                image_id,
                                                page_index,
                                                stream_index,
                                                instruction_index,
                                                resource_name: res_name.clone(),
                                                object_ref: obj_ref,
                                                width,
                                                height,
                                                color_space,
                                                bits_per_component,
                                                filter,
                                                transform,
                                                rect,
                                                is_nested_form: false,
                                                is_shared: false,
                                            });
                                        } else if subtype == "Form" {
                                            if let Ok(nested_images) = self.inspect_form_xobject(
                                                obj_ref,
                                                stream_obj,
                                                page_index,
                                                stream_index,
                                                instruction_index,
                                                &ctm,
                                            ) {
                                                discovered.extend(nested_images);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(discovered)
    }

    /// Recursively inspects a Form XObject for nested image resources.
    fn inspect_form_xobject(
        &mut self,
        form_ref: ObjectRef,
        form_stream: &crate::syntax::object::StreamObject,
        page_index: usize,
        stream_index: usize,
        instruction_index: usize,
        parent_ctm: &Matrix2D,
    ) -> PdfResult<Vec<ImageXObjectInfo>> {
        let mut nested = Vec::new();

        let mut form_xobjects = BTreeMap::new();
        if let Some(res_obj) = form_stream.dict.get("Resources") {
            let res_resolved = self.store.resolve_object(res_obj)?;
            if let Some(res_dict) = res_resolved.as_dict() {
                if let Some(xo_obj) = res_dict.get("XObject") {
                    let xo_resolved = self.store.resolve_object(xo_obj)?;
                    if let Some(xo_dict) = xo_resolved.as_dict() {
                        for (name, val) in xo_dict {
                            if let Some(obj_ref) = val.as_reference() {
                                form_xobjects.insert(name.clone(), obj_ref);
                            }
                        }
                    }
                }
            }
        }

        let form_bytes = FlateDecoder::decode(&form_stream.data, &DecompressLimits::default())
            .unwrap_or_else(|_| form_stream.data.clone());

        let mut parser = ContentParser::from_bytes(&form_bytes);
        let instructions = parser.parse_instructions().unwrap_or_default();
        let mut form_ctm = *parent_ctm;
        let mut stack: Vec<Matrix2D> = Vec::new();

        for (inner_idx, instr) in instructions.into_iter().enumerate() {
            match instr.operator {
                ContentOperator::Q => stack.push(form_ctm),
                ContentOperator::QEnd => {
                    if let Some(saved) = stack.pop() {
                        form_ctm = saved;
                    }
                }
                ContentOperator::Cm => {
                    if instr.operands.len() == 6 {
                        let a = instr.operands[0].as_f64().unwrap_or(1.0);
                        let b = instr.operands[1].as_f64().unwrap_or(0.0);
                        let c = instr.operands[2].as_f64().unwrap_or(0.0);
                        let d = instr.operands[3].as_f64().unwrap_or(1.0);
                        let e = instr.operands[4].as_f64().unwrap_or(0.0);
                        let f = instr.operands[5].as_f64().unwrap_or(0.0);
                        let m = Matrix2D { a, b, c, d, e, f };
                        form_ctm = m.multiply(&form_ctm);
                    }
                }
                ContentOperator::Do => {
                    if let Some(ContentOperand::Name(ref res_name)) = instr.operands.first() {
                        if let Some(&img_ref) = form_xobjects.get(res_name) {
                            if let Ok(resolved) = self.store.resolve(img_ref) {
                                let resolved_obj = resolved.clone();
                                if let Some(stream_obj) = resolved_obj.as_stream() {
                                    let subtype = stream_obj
                                        .dict
                                        .get("Subtype")
                                        .and_then(PdfObject::as_name)
                                        .unwrap_or("");

                                    if subtype == "Image" {
                                        let width = stream_obj
                                            .dict
                                            .get("Width")
                                            .and_then(PdfObject::as_i64)
                                            .unwrap_or(0)
                                            as u32;
                                        let height = stream_obj
                                            .dict
                                            .get("Height")
                                            .and_then(PdfObject::as_i64)
                                            .unwrap_or(0)
                                            as u32;
                                        let color_space = stream_obj
                                            .dict
                                            .get("ColorSpace")
                                            .and_then(|cs| cs.as_name().map(ToString::to_string))
                                            .unwrap_or_else(|| "DeviceRGB".into());
                                        let bits_per_component = stream_obj
                                            .dict
                                            .get("BitsPerComponent")
                                            .and_then(PdfObject::as_i64)
                                            .unwrap_or(8)
                                            as u32;
                                        let filter = stream_obj
                                            .dict
                                            .get("Filter")
                                            .and_then(|f| f.as_name().map(ToString::to_string));

                                        let transform = form_ctm.to_array();
                                        let rect = form_ctm.transform_unit_box();
                                        let image_id = format!(
                                            "img_p{page_index}_s{stream_index}_i{instruction_index}_form_{form_ref}_inner_{inner_idx}_{res_name}"
                                        );

                                        nested.push(ImageXObjectInfo {
                                            image_id,
                                            page_index,
                                            stream_index,
                                            instruction_index,
                                            resource_name: res_name.clone(),
                                            object_ref: img_ref,
                                            width,
                                            height,
                                            color_space,
                                            bits_per_component,
                                            filter,
                                            transform,
                                            rect,
                                            is_nested_form: true,
                                            is_shared: false,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(nested)
    }

    /// Resolves and decodes all content streams for a page.
    fn get_page_content_streams(
        &mut self,
        page_dict: &BTreeMap<String, PdfObject>,
    ) -> PdfResult<Vec<Vec<u8>>> {
        let mut results = Vec::new();
        if let Some(contents_obj) = page_dict.get("Contents") {
            match contents_obj {
                PdfObject::Reference(r) => {
                    let resolved = self.store.resolve(*r)?.clone();
                    if let Some(stream_obj) = resolved.as_stream() {
                        let decoded =
                            FlateDecoder::decode(&stream_obj.data, &DecompressLimits::default())
                                .unwrap_or_else(|_| stream_obj.data.clone());
                        results.push(decoded);
                    }
                }
                PdfObject::Array(arr) => {
                    let arr_refs: Vec<ObjectRef> =
                        arr.iter().filter_map(PdfObject::as_reference).collect();
                    for r in arr_refs {
                        if let Ok(resolved) = self.store.resolve(r) {
                            let resolved_obj = resolved.clone();
                            if let Some(stream_obj) = resolved_obj.as_stream() {
                                let decoded = FlateDecoder::decode(
                                    &stream_obj.data,
                                    &DecompressLimits::default(),
                                )
                                .unwrap_or_else(|_| stream_obj.data.clone());
                                results.push(decoded);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(results)
    }
}
