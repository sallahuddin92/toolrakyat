use std::collections::HashMap;

use crate::content::operand::ContentOperand;
use crate::content::operator::ContentOperator;
use crate::content::parser::ContentParser;
use crate::document::{ObjectStore, PdfDocument};
use crate::error::PdfResult;
use crate::filter::flate::FlateDecoder;
use crate::filter::limits::DecompressLimits;
use crate::image::extractor::Matrix2D;
use crate::syntax::object::{ObjectRef, PdfObject};
use crate::vector::types::{
    VectorColor, VectorEditability, VectorGeometry, VectorGraphicInfo, VectorGraphicType,
};

#[derive(Debug, Clone)]
struct GraphicsState {
    ctm: Matrix2D,
    stroke_color: Option<VectorColor>,
    fill_color: Option<VectorColor>,
    line_width: f64,
    clipping_active: bool,
}

impl Default for GraphicsState {
    fn default() -> Self {
        Self {
            ctm: Matrix2D::identity(),
            stroke_color: None,
            fill_color: None,
            line_width: 1.0,
            clipping_active: false,
        }
    }
}

/// Discovers all vector and path content objects across document pages.
pub struct VectorExtractor<'a, 'b> {
    store: &'a mut ObjectStore<'b>,
    page_refs: Vec<ObjectRef>,
}

impl<'a, 'b> VectorExtractor<'a, 'b> {
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

    /// Extracts all vector graphics across all document pages.
    pub fn extract_all_graphics(&mut self) -> PdfResult<Vec<VectorGraphicInfo>> {
        let mut all_graphics = Vec::new();
        let mut stream_counts: HashMap<ObjectRef, usize> = HashMap::new();

        // 1. First pass: count content stream sharing across pages
        for page_ref in &self.page_refs {
            let page_obj = self.store.resolve(*page_ref)?;
            if let Some(contents_obj) = page_obj.as_dict().and_then(|d| d.get("Contents")) {
                match contents_obj {
                    PdfObject::Reference(stream_ref) => {
                        *stream_counts.entry(*stream_ref).or_insert(0) += 1;
                    }
                    PdfObject::Array(arr) => {
                        for item in arr {
                            if let PdfObject::Reference(stream_ref) = item {
                                *stream_counts.entry(*stream_ref).or_insert(0) += 1;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // 2. Second pass: extract vector graphics from each page
        for page_index in 0..self.page_refs.len() {
            let page_graphics = self.extract_page_graphics(page_index, &stream_counts)?;
            all_graphics.extend(page_graphics);
        }

        Ok(all_graphics)
    }

    /// Extracts vector graphics for a specific page.
    pub fn extract_page_graphics(
        &mut self,
        page_index: usize,
        stream_counts: &HashMap<ObjectRef, usize>,
    ) -> PdfResult<Vec<VectorGraphicInfo>> {
        if page_index >= self.page_refs.len() {
            return Ok(Vec::new());
        }

        let page_ref = self.page_refs[page_index];
        let page_obj = self.store.resolve(page_ref)?;

        let contents_entry = page_obj.as_dict().and_then(|d| d.get("Contents")).cloned();
        let contents_streams = match contents_entry {
            Some(PdfObject::Reference(r)) => vec![r],
            Some(PdfObject::Array(arr)) => arr
                .iter()
                .filter_map(|obj| {
                    if let PdfObject::Reference(r) = obj {
                        Some(*r)
                    } else {
                        None
                    }
                })
                .collect(),
            _ => Vec::new(),
        };

        let mut graphics = Vec::new();

        for (stream_index, &stream_ref) in contents_streams.iter().enumerate() {
            let is_shared = stream_counts.get(&stream_ref).copied().unwrap_or(1) > 1;

            let stream_bytes = {
                let stream_obj = self.store.resolve(stream_ref)?;
                if let Some(stream) = stream_obj.as_stream() {
                    let filter = stream.dict.get("Filter").and_then(|f| f.as_name());
                    if filter == Some("FlateDecode") {
                        let limits = DecompressLimits::default();
                        FlateDecoder::decode(&stream.data, &limits)?
                    } else {
                        stream.data.clone()
                    }
                } else {
                    continue;
                }
            };

            let mut parser = ContentParser::from_bytes(&stream_bytes);
            let instructions = match parser.parse_instructions() {
                Ok(inst) => inst,
                Err(_) => continue,
            };

            let mut state_stack: Vec<GraphicsState> = Vec::new();
            let mut current_state = GraphicsState::default();

            // Track active path construction
            let mut path_start_idx: Option<usize> = None;
            let mut current_path_points: Vec<(f64, f64)> = Vec::new();
            let mut rect_candidate: Option<(f64, f64, f64, f64)> = None;
            let mut is_closed_path = false;

            for (idx, inst) in instructions.iter().enumerate() {
                match &inst.operator {
                    ContentOperator::Q => {
                        state_stack.push(current_state.clone());
                    }
                    ContentOperator::QEnd => {
                        if let Some(prev) = state_stack.pop() {
                            current_state = prev;
                        }
                    }
                    ContentOperator::Cm => {
                        if inst.operands.len() >= 6 {
                            let a = inst.operands[0].as_f64().unwrap_or(1.0);
                            let b = inst.operands[1].as_f64().unwrap_or(0.0);
                            let c = inst.operands[2].as_f64().unwrap_or(0.0);
                            let d = inst.operands[3].as_f64().unwrap_or(1.0);
                            let e = inst.operands[4].as_f64().unwrap_or(0.0);
                            let f = inst.operands[5].as_f64().unwrap_or(0.0);
                            let matrix = Matrix2D { a, b, c, d, e, f };
                            current_state.ctm = matrix.multiply(&current_state.ctm);
                        }
                    }
                    ContentOperator::LineWidth => {
                        if let Some(first) = inst.operands.first().and_then(ContentOperand::as_f64)
                        {
                            current_state.line_width = first.max(0.0);
                        }
                    }
                    ContentOperator::GStroke => {
                        if let Some(g) = inst.operands.first().and_then(ContentOperand::as_f64) {
                            current_state.stroke_color = Some(VectorColor::Gray(g));
                        }
                    }
                    ContentOperator::GFill => {
                        if let Some(g) = inst.operands.first().and_then(ContentOperand::as_f64) {
                            current_state.fill_color = Some(VectorColor::Gray(g));
                        }
                    }
                    ContentOperator::RGStroke => {
                        if inst.operands.len() >= 3 {
                            let r = inst.operands[0].as_f64().unwrap_or(0.0);
                            let g = inst.operands[1].as_f64().unwrap_or(0.0);
                            let b = inst.operands[2].as_f64().unwrap_or(0.0);
                            current_state.stroke_color = Some(VectorColor::from_rgb(r, g, b));
                        }
                    }
                    ContentOperator::RGFill => {
                        if inst.operands.len() >= 3 {
                            let r = inst.operands[0].as_f64().unwrap_or(0.0);
                            let g = inst.operands[1].as_f64().unwrap_or(0.0);
                            let b = inst.operands[2].as_f64().unwrap_or(0.0);
                            current_state.fill_color = Some(VectorColor::from_rgb(r, g, b));
                        }
                    }
                    ContentOperator::KStroke => {
                        if inst.operands.len() >= 4 {
                            let c = inst.operands[0].as_f64().unwrap_or(0.0);
                            let m = inst.operands[1].as_f64().unwrap_or(0.0);
                            let y = inst.operands[2].as_f64().unwrap_or(0.0);
                            let k = inst.operands[3].as_f64().unwrap_or(0.0);
                            current_state.stroke_color = Some(VectorColor::Cmyk(c, m, y, k));
                        }
                    }
                    ContentOperator::KFill => {
                        if inst.operands.len() >= 4 {
                            let c = inst.operands[0].as_f64().unwrap_or(0.0);
                            let m = inst.operands[1].as_f64().unwrap_or(0.0);
                            let y = inst.operands[2].as_f64().unwrap_or(0.0);
                            let k = inst.operands[3].as_f64().unwrap_or(0.0);
                            current_state.fill_color = Some(VectorColor::Cmyk(c, m, y, k));
                        }
                    }
                    ContentOperator::W | ContentOperator::WStar => {
                        current_state.clipping_active = true;
                    }
                    // Path Construction
                    ContentOperator::Re => {
                        if inst.operands.len() >= 4 {
                            if path_start_idx.is_none() {
                                path_start_idx = Some(idx);
                            }
                            let x = inst.operands[0].as_f64().unwrap_or(0.0);
                            let y = inst.operands[1].as_f64().unwrap_or(0.0);
                            let w = inst.operands[2].as_f64().unwrap_or(0.0);
                            let h = inst.operands[3].as_f64().unwrap_or(0.0);
                            rect_candidate = Some((x, y, w, h));
                        }
                    }
                    ContentOperator::M => {
                        if inst.operands.len() >= 2 {
                            if path_start_idx.is_none() {
                                path_start_idx = Some(idx);
                            }
                            let x = inst.operands[0].as_f64().unwrap_or(0.0);
                            let y = inst.operands[1].as_f64().unwrap_or(0.0);
                            current_path_points.push((x, y));
                        }
                    }
                    ContentOperator::L => {
                        if inst.operands.len() >= 2 {
                            if path_start_idx.is_none() {
                                path_start_idx = Some(idx);
                            }
                            let x = inst.operands[0].as_f64().unwrap_or(0.0);
                            let y = inst.operands[1].as_f64().unwrap_or(0.0);
                            current_path_points.push((x, y));
                        }
                    }
                    ContentOperator::C | ContentOperator::V | ContentOperator::Y => {
                        if path_start_idx.is_none() {
                            path_start_idx = Some(idx);
                        }
                        let mut i = 0;
                        while i + 1 < inst.operands.len() {
                            if let (Some(x), Some(y)) =
                                (inst.operands[i].as_f64(), inst.operands[i + 1].as_f64())
                            {
                                current_path_points.push((x, y));
                            }
                            i += 2;
                        }
                    }
                    ContentOperator::H => {
                        is_closed_path = true;
                    }
                    // Path Painting Operators
                    ContentOperator::S
                    | ContentOperator::SClose
                    | ContentOperator::F
                    | ContentOperator::FUpper
                    | ContentOperator::FStar
                    | ContentOperator::B
                    | ContentOperator::BStar
                    | ContentOperator::BClose
                    | ContentOperator::BCloseStar
                    | ContentOperator::N => {
                        if let Some(start_idx) = path_start_idx {
                            let end_idx = idx;
                            let is_stroked = matches!(
                                inst.operator,
                                ContentOperator::S
                                    | ContentOperator::SClose
                                    | ContentOperator::B
                                    | ContentOperator::BStar
                                    | ContentOperator::BClose
                                    | ContentOperator::BCloseStar
                            );
                            let is_filled = matches!(
                                inst.operator,
                                ContentOperator::F
                                    | ContentOperator::FUpper
                                    | ContentOperator::FStar
                                    | ContentOperator::B
                                    | ContentOperator::BStar
                                    | ContentOperator::BClose
                                    | ContentOperator::BCloseStar
                            );

                            let editability = if current_state.clipping_active {
                                VectorEditability::ComplexClipping
                            } else {
                                VectorEditability::Editable
                            };

                            let (graphic_type, geometry) =
                                if let Some((x, y, w, h)) = rect_candidate {
                                    (
                                        VectorGraphicType::Rectangle,
                                        VectorGeometry::Rectangle {
                                            x,
                                            y,
                                            width: w,
                                            height: h,
                                        },
                                    )
                                } else if current_path_points.len() == 2 {
                                    (
                                        VectorGraphicType::Line,
                                        VectorGeometry::Line {
                                            x1: current_path_points[0].0,
                                            y1: current_path_points[0].1,
                                            x2: current_path_points[1].0,
                                            y2: current_path_points[1].1,
                                        },
                                    )
                                } else {
                                    (
                                        VectorGraphicType::Path,
                                        VectorGeometry::Path {
                                            points: current_path_points.clone(),
                                            closed: is_closed_path,
                                        },
                                    )
                                };

                            let local_bounds = geometry.compute_local_bounds();
                            let bounds = Self::transform_bounds(&local_bounds, &current_state.ctm);

                            let has_isolated_q = start_idx > 0
                                && instructions[start_idx - 1].operator == ContentOperator::Q
                                && end_idx + 1 < instructions.len()
                                && instructions[end_idx + 1].operator == ContentOperator::QEnd;

                            let graphic_id =
                                format!("vec_p{page_index}_s{stream_index}_i{start_idx}_{end_idx}");

                            graphics.push(VectorGraphicInfo {
                                graphic_id,
                                page_index,
                                stream_index,
                                start_instruction_index: start_idx,
                                end_instruction_index: end_idx,
                                graphic_type,
                                bounds,
                                local_bounds,
                                transform: current_state.ctm.to_array(),
                                stroke_color: current_state.stroke_color.clone(),
                                fill_color: current_state.fill_color.clone(),
                                line_width: current_state.line_width,
                                is_stroked,
                                is_filled,
                                is_shared,
                                editability,
                                geometry,
                                has_isolated_q,
                            });
                        }

                        // Reset path collector
                        path_start_idx = None;
                        current_path_points.clear();
                        rect_candidate = None;
                        is_closed_path = false;
                    }
                    _ => {}
                }
            }
        }

        Ok(graphics)
    }

    fn transform_bounds(local: &[f64; 4], ctm: &Matrix2D) -> [f64; 4] {
        let x1 = local[0];
        let y1 = local[1];
        let x2 = local[2];
        let y2 = local[3];

        let p0 = ctm.transform_point(x1, y1);
        let p1 = ctm.transform_point(x2, y1);
        let p2 = ctm.transform_point(x1, y2);
        let p3 = ctm.transform_point(x2, y2);

        let min_x = p0.0.min(p1.0).min(p2.0).min(p3.0);
        let min_y = p0.1.min(p1.1).min(p2.1).min(p3.1);
        let max_x = p0.0.max(p1.0).max(p2.0).max(p3.0);
        let max_y = p0.1.max(p1.1).max(p2.1).max(p3.1);

        [min_x, min_y, max_x, max_y]
    }
}
