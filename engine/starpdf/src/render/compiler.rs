use crate::content::operand::ContentOperand;
use crate::content::operator::ContentOperator;
use crate::content::parser::ContentParser;
use crate::document::object_store::ObjectStore;
use crate::document::PdfDocument;
use crate::error::PdfResult;
use crate::execution::{RenderNodeId, StarNodeId};
use crate::font::font::Font;
use crate::font::resource::PageResources;
use crate::render::ir::{
    BlendMode, ColorRgba, FillRule, FillStyle, LineCap, LineJoin, PathVerb, RenderClip,
    RenderGlyph, RenderImage, RenderItem, RenderPage, RenderPath, RenderShading, RenderTextRun,
    ShadingPattern, StrokeStyle, Transform2D, UnsupportedOperatorReport,
};
use crate::syntax::object::{ObjectRef, PdfObject};

#[derive(Debug, Clone)]
struct GraphicsState {
    ctm: Transform2D,
    stroke_color: ColorRgba,
    fill_color: ColorRgba,
    line_width: f32,
    line_cap: LineCap,
    line_join: LineJoin,
    miter_limit: f32,
    dash_pattern: Option<(Vec<f32>, f32)>,
    clip_pending: Option<FillRule>,
    clip_depth: usize,
    stroke_alpha: f32,
    fill_alpha: f32,
    blend_mode: BlendMode,
    text_render_mode: u8,
}

impl Default for GraphicsState {
    fn default() -> Self {
        Self {
            ctm: Transform2D::identity(),
            stroke_color: ColorRgba::BLACK,
            fill_color: ColorRgba::BLACK,
            line_width: 1.0,
            line_cap: LineCap::Butt,
            line_join: LineJoin::Miter,
            miter_limit: 10.0,
            dash_pattern: None,
            clip_pending: None,
            clip_depth: 0,
            stroke_alpha: 1.0,
            fill_alpha: 1.0,
            blend_mode: BlendMode::Normal,
            text_render_mode: 0,
        }
    }
}

#[derive(Debug, Clone)]
struct TextState {
    font_name: String,
    font_size: f32,
    text_matrix: Transform2D,
    text_line_matrix: Transform2D,
    char_spacing: f32,
    word_spacing: f32,
    horizontal_scaling: f32,
    leading: f32,
    text_rise: f32,
}

impl Default for TextState {
    fn default() -> Self {
        Self {
            font_name: "Helvetica".to_string(),
            font_size: 12.0,
            text_matrix: Transform2D::identity(),
            text_line_matrix: Transform2D::identity(),
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            leading: 0.0,
            text_rise: 0.0,
        }
    }
}

pub struct RenderCompiler;

impl RenderCompiler {
    /// Compiles a document page into the typed Render IR.
    pub fn compile_page(doc: &mut PdfDocument<'_>, page_index: usize) -> PdfResult<RenderPage> {
        let page_dict = doc.page_dict(page_index)?;
        let page_ref = doc.page_ref(page_index)?;
        let doc_id = format!("doc_{}", doc.catalog_ref().number);

        // 1. Resolve MediaBox and CropBox
        let media_box = Self::resolve_box(&page_dict, "MediaBox")
            .unwrap_or([0.0, 0.0, 612.0, 792.0]);
        let crop_box = Self::resolve_box(&page_dict, "CropBox").unwrap_or(media_box);
        let rotate = page_dict
            .get("Rotate")
            .and_then(PdfObject::as_i64)
            .unwrap_or(0) as i32;

        let width = (crop_box[2] - crop_box[0]).abs();
        let height = (crop_box[3] - crop_box[1]).abs();

        // 2. Resolve Page Resources
        let resources = PageResources::resolve_for_page(&page_dict, doc.store_mut())?;

        // 3. Resolve and decompress Content Streams
        let mut decompressed_streams = Vec::new();
        if let Some(contents_obj) = page_dict.get("Contents") {
            let resolved = doc.store_mut().resolve_object(contents_obj)?;
            match resolved {
                PdfObject::Stream(stream) => {
                    let bytes = doc.decompress_stream_data(&stream)?;
                    decompressed_streams.push(bytes);
                }
                PdfObject::Array(arr) => {
                    for s_ref in arr {
                        let s_obj = doc.store_mut().resolve_object(&s_ref)?;
                        if let Some(stream) = s_obj.as_stream() {
                            let bytes = doc.decompress_stream_data(stream)?;
                            decompressed_streams.push(bytes);
                        }
                    }
                }
                _ => {}
            }
        }

        let fallback_font = Font::standard_fallback("Helvetica");

        let mut ctx = CompilationContext {
            doc,
            page_dict,
            resources,
            fallback_font,
            doc_id,
            page_ref,
            page_index,
            items: Vec::new(),
            unsupported_operators: Vec::new(),
            graphics_stack: Vec::new(),
            current_graphics: GraphicsState::default(),
            text_state: TextState::default(),
            current_path: Vec::new(),
            subpath_start: None,
            current_point: None,
            recursion_depth: 0,
            local_fonts_stack: Vec::new(),
        };

        // 4. Process Content Streams into Render Items
        for (stream_index, stream_bytes) in decompressed_streams.iter().enumerate() {
            ctx.process_stream(stream_bytes, stream_index, None);
        }

        // Pop any unclosed clips left on the page
        while ctx.current_graphics.clip_depth > 0 {
            ctx.items.push(RenderItem::PopClip);
            ctx.current_graphics.clip_depth -= 1;
        }

        // 5. Process Page Annotations (FreeText, Widget, Stamp, Ink appearances)
        ctx.process_annotations();

        while ctx.current_graphics.clip_depth > 0 {
            ctx.items.push(RenderItem::PopClip);
            ctx.current_graphics.clip_depth -= 1;
        }

        Ok(RenderPage {
            page_index,
            width,
            height,
            media_box,
            crop_box,
            rotate,
            items: ctx.items,
            unsupported_operators: ctx.unsupported_operators,
        })
    }

    fn resolve_box(
        dict: &std::collections::BTreeMap<String, PdfObject>,
        key: &str,
    ) -> Option<[f32; 4]> {
        let arr = dict.get(key).and_then(PdfObject::as_array)?;
        if arr.len() == 4 {
            let x0 = Self::obj_num(&arr[0]);
            let y0 = Self::obj_num(&arr[1]);
            let x1 = Self::obj_num(&arr[2]);
            let y1 = Self::obj_num(&arr[3]);
            Some([x0, y0, x1, y1])
        } else {
            None
        }
    }

    fn obj_num(obj: &PdfObject) -> f32 {
        match obj {
            PdfObject::Integer(i) => *i as f32,
            PdfObject::Real(r) => *r as f32,
            _ => 0.0,
        }
    }

    fn num(operand: &ContentOperand) -> f32 {
        match operand {
            ContentOperand::Integer(i) => *i as f32,
            ContentOperand::Real(r) => *r as f32,
            _ => 0.0,
        }
    }

    fn find_resource_object(
        doc: &mut PdfDocument<'_>,
        page_dict: &std::collections::BTreeMap<String, PdfObject>,
        local_res: Option<&std::collections::BTreeMap<String, PdfObject>>,
        category: &str,
        res_name: &str,
    ) -> Option<PdfObject> {
        // 1. Check local_res (e.g. from Form XObject)
        if let Some(res_dict) = local_res {
            if let Some(cat_obj) = res_dict.get(category) {
                if let Ok(resolved_cat) = doc.store_mut().resolve_object(cat_obj) {
                    if let Some(cat_dict) = resolved_cat.as_dict() {
                        if let Some(target) = cat_dict.get(res_name) {
                            if let Ok(resolved_target) = doc.store_mut().resolve_object(target) {
                                return Some(resolved_target);
                            }
                        }
                    }
                }
            }
        }

        // 2. Direct page resources
        if let Some(res_obj) = page_dict.get("Resources") {
            if let Ok(resolved_res) = doc.store_mut().resolve_object(res_obj) {
                if let Some(res_dict) = resolved_res.as_dict() {
                    if let Some(cat_obj) = res_dict.get(category) {
                        if let Ok(resolved_cat) = doc.store_mut().resolve_object(cat_obj) {
                            if let Some(cat_dict) = resolved_cat.as_dict() {
                                if let Some(target) = cat_dict.get(res_name) {
                                    if let Ok(resolved_target) =
                                        doc.store_mut().resolve_object(target)
                                    {
                                        return Some(resolved_target);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 3. Walk Parent chain
        let mut current_parent = page_dict.get("Parent").and_then(PdfObject::as_reference);
        while let Some(parent_ref) = current_parent {
            if let Ok(parent_obj) = doc.store_mut().resolve(parent_ref).cloned() {
                if let Some(parent_dict) = parent_obj.as_dict() {
                    if let Some(res_obj) = parent_dict.get("Resources") {
                        if let Ok(resolved_res) = doc.store_mut().resolve_object(res_obj) {
                            if let Some(res_dict) = resolved_res.as_dict() {
                                if let Some(cat_obj) = res_dict.get(category) {
                                    if let Ok(resolved_cat) =
                                        doc.store_mut().resolve_object(cat_obj)
                                    {
                                        if let Some(cat_dict) = resolved_cat.as_dict() {
                                            if let Some(target) = cat_dict.get(res_name) {
                                                if let Ok(resolved_target) =
                                                    doc.store_mut().resolve_object(target)
                                                {
                                                    return Some(resolved_target);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    current_parent = parent_dict.get("Parent").and_then(PdfObject::as_reference);
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        None
    }

    fn decode_direct_pixels(
        decompressed: &[u8],
        cs_obj: Option<&PdfObject>,
        width: u32,
        height: u32,
    ) -> Vec<u8> {
        let color_space = cs_obj.and_then(PdfObject::as_name).unwrap_or("DeviceRGB");
        match color_space {
            "DeviceGray" => {
                let mut rgba = Vec::with_capacity((width * height * 4) as usize);
                for &g in decompressed {
                    rgba.push(g);
                    rgba.push(g);
                    rgba.push(g);
                    rgba.push(255);
                }
                rgba
            }
            "DeviceCMYK" => {
                let mut rgba = Vec::with_capacity((width * height * 4) as usize);
                for chunk in decompressed.chunks_exact(4) {
                    let c = chunk[0] as f32 / 255.0;
                    let m = chunk[1] as f32 / 255.0;
                    let y = chunk[2] as f32 / 255.0;
                    let k = chunk[3] as f32 / 255.0;
                    let col = ColorRgba::from_cmyk(c, m, y, k);
                    let [r, g, b, a] = col.to_u8_array();
                    rgba.push(r);
                    rgba.push(g);
                    rgba.push(b);
                    rgba.push(a);
                }
                rgba
            }
            _ => {
                let mut rgba = Vec::with_capacity((width * height * 4) as usize);
                for chunk in decompressed.chunks_exact(3) {
                    rgba.push(chunk[0]);
                    rgba.push(chunk[1]);
                    rgba.push(chunk[2]);
                    rgba.push(255);
                }
                rgba
            }
        }
    }
}

struct CompilationContext<'a, 'doc> {
    doc: &'a mut PdfDocument<'doc>,
    page_dict: std::collections::BTreeMap<String, PdfObject>,
    resources: PageResources,
    fallback_font: Font,
    doc_id: String,
    page_ref: ObjectRef,
    page_index: usize,
    items: Vec<RenderItem>,
    unsupported_operators: Vec<UnsupportedOperatorReport>,
    graphics_stack: Vec<GraphicsState>,
    current_graphics: GraphicsState,
    text_state: TextState,
    current_path: Vec<PathVerb>,
    subpath_start: Option<(f32, f32)>,
    current_point: Option<(f32, f32)>,
    recursion_depth: usize,
    local_fonts_stack: Vec<std::collections::BTreeMap<String, Font>>,
}

impl<'a, 'doc> CompilationContext<'a, 'doc> {
    fn process_stream(
        &mut self,
        stream_bytes: &[u8],
        stream_index: usize,
        local_res: Option<&std::collections::BTreeMap<String, PdfObject>>,
    ) {
        let mut parser = ContentParser::from_bytes(stream_bytes);
        let instructions = match parser.parse_instructions() {
            Ok(inst) => inst,
            Err(_) => return,
        };

        for (inst_idx, inst) in instructions.into_iter().enumerate() {
            let occurrence = format!(
                "p{}/s{}/d{}/i{}",
                self.page_index, stream_index, self.recursion_depth, inst_idx
            );
            let star_node_id = StarNodeId::new(&self.doc_id, self.page_ref, &occurrence).ok();
            let render_node_id = star_node_id
                .as_ref()
                .and_then(|sn| RenderNodeId::new(sn, &format!("op_{}", inst.operator.as_str())).ok());

            match inst.operator {
                ContentOperator::Q => {
                    self.graphics_stack.push(self.current_graphics.clone());
                }
                ContentOperator::QEnd => {
                    if let Some(prev) = self.graphics_stack.pop() {
                        let diff = self.current_graphics.clip_depth.saturating_sub(prev.clip_depth);
                        for _ in 0..diff {
                            self.items.push(RenderItem::PopClip);
                        }
                        self.current_graphics = prev;
                    }
                }
                ContentOperator::Cm => {
                    if inst.operands.len() >= 6 {
                        let a = RenderCompiler::num(&inst.operands[0]);
                        let b = RenderCompiler::num(&inst.operands[1]);
                        let c = RenderCompiler::num(&inst.operands[2]);
                        let d = RenderCompiler::num(&inst.operands[3]);
                        let e = RenderCompiler::num(&inst.operands[4]);
                        let f = RenderCompiler::num(&inst.operands[5]);
                        let cm = Transform2D::new(a, b, c, d, e, f);
                        self.current_graphics.ctm = cm.multiply(&self.current_graphics.ctm);
                    }
                }

                // Path construction
                ContentOperator::M => {
                    if inst.operands.len() >= 2 {
                        let x = RenderCompiler::num(&inst.operands[0]);
                        let y = RenderCompiler::num(&inst.operands[1]);
                        self.current_path.push(PathVerb::MoveTo(x, y));
                        self.subpath_start = Some((x, y));
                        self.current_point = Some((x, y));
                    }
                }
                ContentOperator::L => {
                    if inst.operands.len() >= 2 {
                        let x = RenderCompiler::num(&inst.operands[0]);
                        let y = RenderCompiler::num(&inst.operands[1]);
                        self.current_path.push(PathVerb::LineTo(x, y));
                        self.current_point = Some((x, y));
                    }
                }
                ContentOperator::C => {
                    if inst.operands.len() >= 6 {
                        let x1 = RenderCompiler::num(&inst.operands[0]);
                        let y1 = RenderCompiler::num(&inst.operands[1]);
                        let x2 = RenderCompiler::num(&inst.operands[2]);
                        let y2 = RenderCompiler::num(&inst.operands[3]);
                        let x3 = RenderCompiler::num(&inst.operands[4]);
                        let y3 = RenderCompiler::num(&inst.operands[5]);
                        self.current_path.push(PathVerb::CubicTo(x1, y1, x2, y2, x3, y3));
                        self.current_point = Some((x3, y3));
                    }
                }
                ContentOperator::V => {
                    if inst.operands.len() >= 4 {
                        let (x1, y1) = self.current_point.unwrap_or((0.0, 0.0));
                        let x2 = RenderCompiler::num(&inst.operands[0]);
                        let y2 = RenderCompiler::num(&inst.operands[1]);
                        let x3 = RenderCompiler::num(&inst.operands[2]);
                        let y3 = RenderCompiler::num(&inst.operands[3]);
                        self.current_path.push(PathVerb::CubicTo(x1, y1, x2, y2, x3, y3));
                        self.current_point = Some((x3, y3));
                    }
                }
                ContentOperator::Y => {
                    if inst.operands.len() >= 4 {
                        let x1 = RenderCompiler::num(&inst.operands[0]);
                        let y1 = RenderCompiler::num(&inst.operands[1]);
                        let x3 = RenderCompiler::num(&inst.operands[2]);
                        let y3 = RenderCompiler::num(&inst.operands[3]);
                        self.current_path.push(PathVerb::CubicTo(x1, y1, x3, y3, x3, y3));
                        self.current_point = Some((x3, y3));
                    }
                }
                ContentOperator::Re => {
                    if inst.operands.len() >= 4 {
                        let x = RenderCompiler::num(&inst.operands[0]);
                        let y = RenderCompiler::num(&inst.operands[1]);
                        let w = RenderCompiler::num(&inst.operands[2]);
                        let h = RenderCompiler::num(&inst.operands[3]);
                        self.current_path.push(PathVerb::MoveTo(x, y));
                        self.current_path.push(PathVerb::LineTo(x + w, y));
                        self.current_path.push(PathVerb::LineTo(x + w, y + h));
                        self.current_path.push(PathVerb::LineTo(x, y + h));
                        self.current_path.push(PathVerb::Close);
                        self.subpath_start = Some((x, y));
                        self.current_point = Some((x, y));
                    }
                }
                ContentOperator::H => {
                    self.current_path.push(PathVerb::Close);
                    self.current_point = self.subpath_start;
                }

                // Clipping
                ContentOperator::W => {
                    self.current_graphics.clip_pending = Some(FillRule::NonZero);
                }
                ContentOperator::WStar => {
                    self.current_graphics.clip_pending = Some(FillRule::EvenOdd);
                }

                // Path painting
                ContentOperator::F | ContentOperator::FUpper => {
                    self.emit_path(
                        true,
                        false,
                        FillRule::NonZero,
                        false,
                        star_node_id,
                        render_node_id,
                    );
                }
                ContentOperator::FStar => {
                    self.emit_path(
                        true,
                        false,
                        FillRule::EvenOdd,
                        false,
                        star_node_id,
                        render_node_id,
                    );
                }
                ContentOperator::S => {
                    self.emit_path(
                        false,
                        true,
                        FillRule::NonZero,
                        false,
                        star_node_id,
                        render_node_id,
                    );
                }
                ContentOperator::SClose => {
                    self.emit_path(
                        false,
                        true,
                        FillRule::NonZero,
                        true,
                        star_node_id,
                        render_node_id,
                    );
                }
                ContentOperator::B => {
                    self.emit_path(
                        true,
                        true,
                        FillRule::NonZero,
                        false,
                        star_node_id,
                        render_node_id,
                    );
                }
                ContentOperator::BStar => {
                    self.emit_path(
                        true,
                        true,
                        FillRule::EvenOdd,
                        false,
                        star_node_id,
                        render_node_id,
                    );
                }
                ContentOperator::BClose => {
                    self.emit_path(
                        true,
                        true,
                        FillRule::NonZero,
                        true,
                        star_node_id,
                        render_node_id,
                    );
                }
                ContentOperator::BCloseStar => {
                    self.emit_path(
                        true,
                        true,
                        FillRule::EvenOdd,
                        true,
                        star_node_id,
                        render_node_id,
                    );
                }
                ContentOperator::N => {
                    if let Some(rule) = self.current_graphics.clip_pending.take() {
                        self.items.push(RenderItem::PushClip(RenderClip {
                            verbs: self.current_path.clone(),
                            rule,
                            transform: self.current_graphics.ctm,
                        }));
                        self.current_graphics.clip_depth += 1;
                    }
                    self.current_path.clear();
                    self.subpath_start = None;
                    self.current_point = None;
                }

                // Stroke & Fill Settings
                ContentOperator::LineWidth => {
                    if !inst.operands.is_empty() {
                        self.current_graphics.line_width =
                            RenderCompiler::num(&inst.operands[0]).max(0.1);
                    }
                }
                ContentOperator::LineCap => {
                    if !inst.operands.is_empty() {
                        self.current_graphics.line_cap =
                            match RenderCompiler::num(&inst.operands[0]) as i32 {
                                1 => LineCap::Round,
                                2 => LineCap::Square,
                                _ => LineCap::Butt,
                            };
                    }
                }
                ContentOperator::LineJoin => {
                    if !inst.operands.is_empty() {
                        self.current_graphics.line_join =
                            match RenderCompiler::num(&inst.operands[0]) as i32 {
                                1 => LineJoin::Round,
                                2 => LineJoin::Bevel,
                                _ => LineJoin::Miter,
                            };
                    }
                }
                ContentOperator::MiterLimit => {
                    if !inst.operands.is_empty() {
                        self.current_graphics.miter_limit =
                            RenderCompiler::num(&inst.operands[0]).max(1.0);
                    }
                }
                ContentOperator::Dash => {
                    if inst.operands.len() >= 2 {
                        let phase = RenderCompiler::num(&inst.operands[1]);
                        if let ContentOperand::Array(arr) = &inst.operands[0] {
                            let dashes: Vec<f32> = arr.iter().map(RenderCompiler::num).collect();
                            self.current_graphics.dash_pattern = if dashes.is_empty() {
                                None
                            } else {
                                Some((dashes, phase))
                            };
                        }
                    }
                }
                ContentOperator::GFill => {
                    if !inst.operands.is_empty() {
                        self.current_graphics.fill_color =
                            ColorRgba::from_gray(RenderCompiler::num(&inst.operands[0]));
                    }
                }
                ContentOperator::GStroke => {
                    if !inst.operands.is_empty() {
                        self.current_graphics.stroke_color =
                            ColorRgba::from_gray(RenderCompiler::num(&inst.operands[0]));
                    }
                }
                ContentOperator::RGFill => {
                    if inst.operands.len() >= 3 {
                        let r = RenderCompiler::num(&inst.operands[0]);
                        let g = RenderCompiler::num(&inst.operands[1]);
                        let b = RenderCompiler::num(&inst.operands[2]);
                        self.current_graphics.fill_color = ColorRgba::from_rgb(r, g, b);
                    }
                }
                ContentOperator::RGStroke => {
                    if inst.operands.len() >= 3 {
                        let r = RenderCompiler::num(&inst.operands[0]);
                        let g = RenderCompiler::num(&inst.operands[1]);
                        let b = RenderCompiler::num(&inst.operands[2]);
                        self.current_graphics.stroke_color = ColorRgba::from_rgb(r, g, b);
                    }
                }
                ContentOperator::KFill => {
                    if inst.operands.len() >= 4 {
                        let c = RenderCompiler::num(&inst.operands[0]);
                        let m = RenderCompiler::num(&inst.operands[1]);
                        let y = RenderCompiler::num(&inst.operands[2]);
                        let k = RenderCompiler::num(&inst.operands[3]);
                        self.current_graphics.fill_color = ColorRgba::from_cmyk(c, m, y, k);
                    }
                }
                ContentOperator::KStroke => {
                    if inst.operands.len() >= 4 {
                        let c = RenderCompiler::num(&inst.operands[0]);
                        let m = RenderCompiler::num(&inst.operands[1]);
                        let y = RenderCompiler::num(&inst.operands[2]);
                        let k = RenderCompiler::num(&inst.operands[3]);
                        self.current_graphics.stroke_color = ColorRgba::from_cmyk(c, m, y, k);
                    }
                }

                // ExtGState
                ContentOperator::Gs => {
                    if let Some(ContentOperand::Name(gs_name)) = inst.operands.first() {
                        self.apply_ext_gstate(gs_name, local_res);
                    }
                }

                // Text Rendering Mode
                ContentOperator::Tr => {
                    if let Some(op) = inst.operands.first() {
                        self.current_graphics.text_render_mode =
                            RenderCompiler::num(op) as u8;
                    }
                }

                // Text Operators
                ContentOperator::Bt => {
                    self.text_state.text_matrix = Transform2D::identity();
                    self.text_state.text_line_matrix = Transform2D::identity();
                }
                ContentOperator::Et => {}
                ContentOperator::Tf => {
                    if inst.operands.len() >= 2 {
                        if let ContentOperand::Name(name) = &inst.operands[0] {
                            self.text_state.font_name = name.clone();
                        }
                        self.text_state.font_size =
                            RenderCompiler::num(&inst.operands[1]).max(0.1);
                    }
                }
                ContentOperator::Tm => {
                    if inst.operands.len() >= 6 {
                        let a = RenderCompiler::num(&inst.operands[0]);
                        let b = RenderCompiler::num(&inst.operands[1]);
                        let c = RenderCompiler::num(&inst.operands[2]);
                        let d = RenderCompiler::num(&inst.operands[3]);
                        let e = RenderCompiler::num(&inst.operands[4]);
                        let f = RenderCompiler::num(&inst.operands[5]);
                        let tm = Transform2D::new(a, b, c, d, e, f);
                        self.text_state.text_matrix = tm;
                        self.text_state.text_line_matrix = tm;
                    }
                }
                ContentOperator::Td => {
                    if inst.operands.len() >= 2 {
                        let tx = RenderCompiler::num(&inst.operands[0]);
                        let ty = RenderCompiler::num(&inst.operands[1]);
                        let translation = Transform2D::translation(tx, ty);
                        let new_matrix =
                            translation.multiply(&self.text_state.text_line_matrix);
                        self.text_state.text_matrix = new_matrix;
                        self.text_state.text_line_matrix = new_matrix;
                    }
                }
                ContentOperator::TD => {
                    if inst.operands.len() >= 2 {
                        let tx = RenderCompiler::num(&inst.operands[0]);
                        let ty = RenderCompiler::num(&inst.operands[1]);
                        self.text_state.leading = -ty;
                        let translation = Transform2D::translation(tx, ty);
                        let new_matrix =
                            translation.multiply(&self.text_state.text_line_matrix);
                        self.text_state.text_matrix = new_matrix;
                        self.text_state.text_line_matrix = new_matrix;
                    }
                }
                ContentOperator::TStar => {
                    let translation =
                        Transform2D::translation(0.0, -self.text_state.leading);
                    let new_matrix = translation.multiply(&self.text_state.text_line_matrix);
                    self.text_state.text_matrix = new_matrix;
                    self.text_state.text_line_matrix = new_matrix;
                }
                ContentOperator::Tc => {
                    if !inst.operands.is_empty() {
                        self.text_state.char_spacing = RenderCompiler::num(&inst.operands[0]);
                    }
                }
                ContentOperator::Tw => {
                    if !inst.operands.is_empty() {
                        self.text_state.word_spacing = RenderCompiler::num(&inst.operands[0]);
                    }
                }
                ContentOperator::Tz => {
                    if !inst.operands.is_empty() {
                        self.text_state.horizontal_scaling =
                            RenderCompiler::num(&inst.operands[0]);
                    }
                }
                ContentOperator::TL => {
                    if !inst.operands.is_empty() {
                        self.text_state.leading = RenderCompiler::num(&inst.operands[0]);
                    }
                }
                ContentOperator::Ts => {
                    if !inst.operands.is_empty() {
                        self.text_state.text_rise = RenderCompiler::num(&inst.operands[0]);
                    }
                }

                // Text Showing
                ContentOperator::Tj => {
                    if let Some(ContentOperand::String(bytes)) = inst.operands.first() {
                        self.show_text_run(bytes, star_node_id, render_node_id);
                    }
                }
                ContentOperator::TJ => {
                    if let Some(ContentOperand::Array(elements)) = inst.operands.first() {
                        for elem in elements {
                            match elem {
                                ContentOperand::String(bytes) => {
                                    self.show_text_run(
                                        bytes,
                                        star_node_id.clone(),
                                        render_node_id.clone(),
                                    );
                                }
                                ContentOperand::Integer(n) => {
                                    let adj = -(*n as f32) / 1000.0
                                        * self.text_state.font_size
                                        * (self.text_state.horizontal_scaling / 100.0);
                                    let t = Transform2D::translation(adj, 0.0);
                                    self.text_state.text_matrix =
                                        t.multiply(&self.text_state.text_matrix);
                                }
                                ContentOperand::Real(r) => {
                                    let adj = -(*r as f32) / 1000.0
                                        * self.text_state.font_size
                                        * (self.text_state.horizontal_scaling / 100.0);
                                    let t = Transform2D::translation(adj, 0.0);
                                    self.text_state.text_matrix =
                                        t.multiply(&self.text_state.text_matrix);
                                }
                                _ => {}
                            }
                        }
                    }
                }
                ContentOperator::Quote => {
                    let translation =
                        Transform2D::translation(0.0, -self.text_state.leading);
                    self.text_state.text_matrix =
                        translation.multiply(&self.text_state.text_line_matrix);
                    self.text_state.text_line_matrix = self.text_state.text_matrix;
                    if let Some(ContentOperand::String(bytes)) = inst.operands.first() {
                        self.show_text_run(bytes, star_node_id, render_node_id);
                    }
                }
                ContentOperator::DoubleQuote => {
                    if inst.operands.len() >= 3 {
                        self.text_state.word_spacing = RenderCompiler::num(&inst.operands[0]);
                        self.text_state.char_spacing = RenderCompiler::num(&inst.operands[1]);
                        let translation =
                            Transform2D::translation(0.0, -self.text_state.leading);
                        self.text_state.text_matrix =
                            translation.multiply(&self.text_state.text_line_matrix);
                        self.text_state.text_line_matrix = self.text_state.text_matrix;
                        if let ContentOperand::String(bytes) = &inst.operands[2] {
                            self.show_text_run(bytes, star_node_id, render_node_id);
                        }
                    }
                }

                // XObject Invocation
                ContentOperator::Do => {
                    if let Some(ContentOperand::Name(res_name)) = inst.operands.first() {
                        self.compile_xobject(
                            res_name,
                            local_res,
                            star_node_id,
                            render_node_id,
                        );
                    }
                }

                // Shading Invocation
                ContentOperator::Sh => {
                    if let Some(ContentOperand::Name(sh_name)) = inst.operands.first() {
                        self.compile_shading(
                            sh_name,
                            local_res,
                            star_node_id,
                            render_node_id,
                        );
                    }
                }

                ContentOperator::SCFill | ContentOperator::SCNFill => {
                    let nums: Vec<f32> = inst
                        .operands
                        .iter()
                        .filter_map(|op| match op {
                            ContentOperand::Integer(i) => Some(*i as f32),
                            ContentOperand::Real(r) => Some(*r as f32),
                            _ => None,
                        })
                        .collect();
                    match nums.len() {
                        1 => {
                            self.current_graphics.fill_color = ColorRgba::from_gray(nums[0]);
                        }
                        3 => {
                            self.current_graphics.fill_color =
                                ColorRgba::from_rgb(nums[0], nums[1], nums[2]);
                        }
                        4 => {
                            self.current_graphics.fill_color =
                                ColorRgba::from_cmyk(nums[0], nums[1], nums[2], nums[3]);
                        }
                        _ => {}
                    }
                }
                ContentOperator::SCStroke | ContentOperator::SCNStroke => {
                    let nums: Vec<f32> = inst
                        .operands
                        .iter()
                        .filter_map(|op| match op {
                            ContentOperand::Integer(i) => Some(*i as f32),
                            ContentOperand::Real(r) => Some(*r as f32),
                            _ => None,
                        })
                        .collect();
                    match nums.len() {
                        1 => {
                            self.current_graphics.stroke_color = ColorRgba::from_gray(nums[0]);
                        }
                        3 => {
                            self.current_graphics.stroke_color =
                                ColorRgba::from_rgb(nums[0], nums[1], nums[2]);
                        }
                        4 => {
                            self.current_graphics.stroke_color =
                                ColorRgba::from_cmyk(nums[0], nums[1], nums[2], nums[3]);
                        }
                        _ => {}
                    }
                }

                // Harmless color space setting and marked content / compatibility metadata
                ContentOperator::CSFill
                | ContentOperator::CSStroke
                | ContentOperator::Bmc
                | ContentOperator::Bdc
                | ContentOperator::Emc
                | ContentOperator::Mp
                | ContentOperator::Dp
                | ContentOperator::Bx
                | ContentOperator::Ex => {}

                // Report unsupported operators
                other => {
                    self.unsupported_operators.push(UnsupportedOperatorReport {
                        operator: other.as_str().to_string(),
                        operands: inst.operands.iter().map(|o| format!("{o:?}")).collect(),
                        stream_index,
                        instruction_index: inst_idx,
                    });
                }
            }
        }
    }

    fn apply_ext_gstate(
        &mut self,
        name: &str,
        local_res: Option<&std::collections::BTreeMap<String, PdfObject>>,
    ) {
        if let Some(ext_obj) = RenderCompiler::find_resource_object(
            self.doc,
            &self.page_dict,
            local_res,
            "ExtGState",
            name,
        ) {
            if let Some(ext_dict) = ext_obj.as_dict() {
                if let Some(lw) = ext_dict.get("LW").map(RenderCompiler::obj_num) {
                    self.current_graphics.line_width = lw.max(0.1);
                }
                if let Some(lc) = ext_dict.get("LC").map(RenderCompiler::obj_num) {
                    self.current_graphics.line_cap = match lc as i32 {
                        1 => LineCap::Round,
                        2 => LineCap::Square,
                        _ => LineCap::Butt,
                    };
                }
                if let Some(lj) = ext_dict.get("LJ").map(RenderCompiler::obj_num) {
                    self.current_graphics.line_join = match lj as i32 {
                        1 => LineJoin::Round,
                        2 => LineJoin::Bevel,
                        _ => LineJoin::Miter,
                    };
                }
                if let Some(ml) = ext_dict.get("ML").map(RenderCompiler::obj_num) {
                    self.current_graphics.miter_limit = ml.max(1.0);
                }
                if let Some(ca) = ext_dict.get("CA").map(RenderCompiler::obj_num) {
                    self.current_graphics.stroke_alpha = ca.clamp(0.0, 1.0);
                }
                if let Some(ca) = ext_dict.get("ca").map(RenderCompiler::obj_num) {
                    self.current_graphics.fill_alpha = ca.clamp(0.0, 1.0);
                }
                if let Some(bm_obj) = ext_dict.get("BM") {
                    let bm_str = match bm_obj {
                        PdfObject::Name(n) => Some(n.as_str()),
                        PdfObject::Array(arr) => arr.first().and_then(PdfObject::as_name),
                        _ => None,
                    };
                    if let Some(s) = bm_str {
                        self.current_graphics.blend_mode = match s {
                            "Multiply" => BlendMode::Multiply,
                            "Screen" => BlendMode::Screen,
                            "Overlay" => BlendMode::Overlay,
                            "Darken" => BlendMode::Darken,
                            "Lighten" => BlendMode::Lighten,
                            "ColorDodge" => BlendMode::ColorDodge,
                            "ColorBurn" => BlendMode::ColorBurn,
                            "HardLight" => BlendMode::HardLight,
                            "SoftLight" => BlendMode::SoftLight,
                            "Difference" => BlendMode::Difference,
                            "Exclusion" => BlendMode::Exclusion,
                            _ => BlendMode::Normal,
                        };
                    }
                }
                if let Some(d_arr) = ext_dict.get("D").and_then(PdfObject::as_array) {
                    if d_arr.len() >= 2 {
                        if let Some(pattern_arr) = d_arr[0].as_array() {
                            let dashes: Vec<f32> =
                                pattern_arr.iter().map(RenderCompiler::obj_num).collect();
                            let phase = RenderCompiler::obj_num(&d_arr[1]);
                            self.current_graphics.dash_pattern = if dashes.is_empty() {
                                None
                            } else {
                                Some((dashes, phase))
                            };
                        }
                    }
                }
            }
        }
    }

    fn compile_shading(
        &mut self,
        sh_name: &str,
        local_res: Option<&std::collections::BTreeMap<String, PdfObject>>,
        star_node_id: Option<StarNodeId>,
        render_node_id: Option<RenderNodeId>,
    ) {
        let sh_obj = match RenderCompiler::find_resource_object(
            self.doc,
            &self.page_dict,
            local_res,
            "Shading",
            sh_name,
        ) {
            Some(obj) => obj,
            None => return,
        };

        let dict = match &sh_obj {
            PdfObject::Dictionary(d) => d,
            PdfObject::Stream(s) => &s.dict,
            _ => return,
        };

        let sh_type = dict
            .get("ShadingType")
            .and_then(PdfObject::as_i64)
            .unwrap_or(2);

        let bbox = RenderCompiler::resolve_box(dict, "BBox");

        let (start_color, end_color) = if let Some(func_obj) = dict.get("Function") {
            Self::resolve_gradient_colors(self.doc, func_obj)
        } else {
            (ColorRgba::BLACK, self.current_graphics.fill_color)
        };

        if sh_type == 2 {
            let coords = if let Some(arr) = dict.get("Coords").and_then(PdfObject::as_array) {
                if arr.len() >= 4 {
                    [
                        RenderCompiler::obj_num(&arr[0]),
                        RenderCompiler::obj_num(&arr[1]),
                        RenderCompiler::obj_num(&arr[2]),
                        RenderCompiler::obj_num(&arr[3]),
                    ]
                } else {
                    [0.0, 0.0, 100.0, 0.0]
                }
            } else {
                [0.0, 0.0, 100.0, 0.0]
            };

            let extend = if let Some(arr) = dict.get("Extend").and_then(PdfObject::as_array) {
                if arr.len() >= 2 {
                    [
                        arr[0].as_bool().unwrap_or(true),
                        arr[1].as_bool().unwrap_or(true),
                    ]
                } else {
                    [true, true]
                }
            } else {
                [true, true]
            };

            self.items.push(RenderItem::Shading(RenderShading {
                pattern: ShadingPattern::Axial {
                    coords,
                    start_color,
                    end_color,
                    extend,
                },
                bbox,
                transform: self.current_graphics.ctm,
                alpha: self.current_graphics.fill_alpha,
                blend_mode: self.current_graphics.blend_mode,
                star_node_id,
                render_node_id,
            }));
        } else if sh_type == 3 {
            let coords = if let Some(arr) = dict.get("Coords").and_then(PdfObject::as_array) {
                if arr.len() >= 6 {
                    [
                        RenderCompiler::obj_num(&arr[0]),
                        RenderCompiler::obj_num(&arr[1]),
                        RenderCompiler::obj_num(&arr[2]),
                        RenderCompiler::obj_num(&arr[3]),
                        RenderCompiler::obj_num(&arr[4]),
                        RenderCompiler::obj_num(&arr[5]),
                    ]
                } else {
                    [0.0, 0.0, 0.0, 100.0, 100.0, 50.0]
                }
            } else {
                [0.0, 0.0, 0.0, 100.0, 100.0, 50.0]
            };

            let extend = if let Some(arr) = dict.get("Extend").and_then(PdfObject::as_array) {
                if arr.len() >= 2 {
                    [
                        arr[0].as_bool().unwrap_or(true),
                        arr[1].as_bool().unwrap_or(true),
                    ]
                } else {
                    [true, true]
                }
            } else {
                [true, true]
            };

            self.items.push(RenderItem::Shading(RenderShading {
                pattern: ShadingPattern::Radial {
                    coords,
                    start_color,
                    end_color,
                    extend,
                },
                bbox,
                transform: self.current_graphics.ctm,
                alpha: self.current_graphics.fill_alpha,
                blend_mode: self.current_graphics.blend_mode,
                star_node_id,
                render_node_id,
            }));
        }
    }

    fn resolve_gradient_colors(
        doc: &mut PdfDocument<'_>,
        func_obj: &PdfObject,
    ) -> (ColorRgba, ColorRgba) {
        let resolved = doc.store_mut().resolve_object(func_obj).ok();
        if let Some(f_dict) = resolved.as_ref().and_then(|o| o.as_dict()) {
            let c0 = f_dict.get("C0").and_then(PdfObject::as_array);
            let c1 = f_dict.get("C1").and_then(PdfObject::as_array);

            let start = if let Some(arr) = c0 {
                if arr.len() >= 3 {
                    ColorRgba::from_rgb(
                        RenderCompiler::obj_num(&arr[0]),
                        RenderCompiler::obj_num(&arr[1]),
                        RenderCompiler::obj_num(&arr[2]),
                    )
                } else if arr.len() == 1 {
                    ColorRgba::from_gray(RenderCompiler::obj_num(&arr[0]))
                } else {
                    ColorRgba::BLACK
                }
            } else {
                ColorRgba::BLACK
            };

            let end = if let Some(arr) = c1 {
                if arr.len() >= 3 {
                    ColorRgba::from_rgb(
                        RenderCompiler::obj_num(&arr[0]),
                        RenderCompiler::obj_num(&arr[1]),
                        RenderCompiler::obj_num(&arr[2]),
                    )
                } else if arr.len() == 1 {
                    ColorRgba::from_gray(RenderCompiler::obj_num(&arr[0]))
                } else {
                    ColorRgba::WHITE
                }
            } else {
                ColorRgba::WHITE
            };

            (start, end)
        } else {
            (ColorRgba::BLACK, ColorRgba::WHITE)
        }
    }

    fn compile_xobject(
        &mut self,
        res_name: &str,
        local_res: Option<&std::collections::BTreeMap<String, PdfObject>>,
        star_node_id: Option<StarNodeId>,
        render_node_id: Option<RenderNodeId>,
    ) {
        let stream_obj = match RenderCompiler::find_resource_object(
            self.doc,
            &self.page_dict,
            local_res,
            "XObject",
            res_name,
        ) {
            Some(PdfObject::Stream(s)) => s,
            _ => return,
        };

        let subtype = stream_obj
            .dict
            .get("Subtype")
            .and_then(PdfObject::as_name)
            .unwrap_or("");

        if subtype == "Form" {
            if self.recursion_depth >= 16 {
                return;
            }

            let bbox = RenderCompiler::resolve_box(&stream_obj.dict, "BBox")
                .unwrap_or([0.0, 0.0, 100.0, 100.0]);
            let form_matrix = if let Some(m_arr) =
                stream_obj.dict.get("Matrix").and_then(PdfObject::as_array)
            {
                if m_arr.len() >= 6 {
                    Transform2D::new(
                        RenderCompiler::obj_num(&m_arr[0]),
                        RenderCompiler::obj_num(&m_arr[1]),
                        RenderCompiler::obj_num(&m_arr[2]),
                        RenderCompiler::obj_num(&m_arr[3]),
                        RenderCompiler::obj_num(&m_arr[4]),
                        RenderCompiler::obj_num(&m_arr[5]),
                    )
                } else {
                    Transform2D::identity()
                }
            } else {
                Transform2D::identity()
            };

            let form_resources = if let Some(r_obj) = stream_obj.dict.get("Resources") {
                self.doc
                    .store_mut()
                    .resolve_object(r_obj)
                    .ok()
                    .and_then(|o| o.as_dict().cloned())
            } else {
                None
            };

            let form_stream_bytes = self
                .doc
                .decompress_stream_data(&stream_obj)
                .unwrap_or_default();

            // Push graphics state
            self.graphics_stack.push(self.current_graphics.clone());
            self.current_graphics.ctm = form_matrix.multiply(&self.current_graphics.ctm);

            // Push clip for BBox
            let clip_verbs = vec![
                PathVerb::MoveTo(bbox[0], bbox[1]),
                PathVerb::LineTo(bbox[2], bbox[1]),
                PathVerb::LineTo(bbox[2], bbox[3]),
                PathVerb::LineTo(bbox[0], bbox[3]),
                PathVerb::Close,
            ];
            self.items.push(RenderItem::PushClip(RenderClip {
                verbs: clip_verbs,
                rule: FillRule::NonZero,
                transform: self.current_graphics.ctm,
            }));
            self.current_graphics.clip_depth += 1;

            let has_fonts = if let Some(ref res) = form_resources {
                if res.contains_key("Font") {
                    let fonts = Self::extract_fonts_from_resources(res, self.doc.store_mut());
                    self.local_fonts_stack.push(fonts);
                    true
                } else {
                    false
                }
            } else {
                false
            };

            self.recursion_depth += 1;
            self.process_stream(
                &form_stream_bytes,
                9999 + self.recursion_depth,
                form_resources.as_ref(),
            );
            self.recursion_depth -= 1;

            if has_fonts {
                self.local_fonts_stack.pop();
            }

            // Pop clip
            self.items.push(RenderItem::PopClip);
            self.current_graphics.clip_depth = self.current_graphics.clip_depth.saturating_sub(1);

            // Restore graphics state
            if let Some(prev) = self.graphics_stack.pop() {
                self.current_graphics = prev;
            }
        } else if subtype == "Image" {
            let width = stream_obj
                .dict
                .get("Width")
                .and_then(PdfObject::as_i64)
                .unwrap_or(0) as u32;
            let height = stream_obj
                .dict
                .get("Height")
                .and_then(PdfObject::as_i64)
                .unwrap_or(0) as u32;

            if width == 0 || height == 0 {
                return;
            }

            let is_mask = stream_obj
                .dict
                .get("ImageMask")
                .and_then(PdfObject::as_bool)
                .unwrap_or(false);

            if is_mask {
                let decompressed = self
                    .doc
                    .decompress_stream_data(&stream_obj)
                    .unwrap_or_default();
                let decode_invert = if let Some(arr) =
                    stream_obj.dict.get("Decode").and_then(PdfObject::as_array)
                {
                    if arr.len() >= 2 {
                        RenderCompiler::obj_num(&arr[0]) > RenderCompiler::obj_num(&arr[1])
                    } else {
                        false
                    }
                } else {
                    false
                };

                let stride = ((width + 7) / 8) as usize;
                let [fill_r, fill_g, fill_b, fill_a] =
                    self.current_graphics.fill_color.to_u8_array();
                let final_alpha =
                    (fill_a as f32 * self.current_graphics.fill_alpha).round() as u8;

                let mut rgba_data = Vec::with_capacity((width * height * 4) as usize);
                for row in 0..height {
                    let row_start = (row as usize) * stride;
                    for col in 0..width {
                        let byte_idx = row_start + (col as usize / 8);
                        let bit_idx = 7 - (col % 8);
                        let bit = if byte_idx < decompressed.len() {
                            (decompressed[byte_idx] >> bit_idx) & 1
                        } else {
                            0
                        };

                        let paint = if decode_invert { bit == 0 } else { bit != 0 };
                        if paint {
                            rgba_data.push(fill_r);
                            rgba_data.push(fill_g);
                            rgba_data.push(fill_b);
                            rgba_data.push(final_alpha);
                        } else {
                            rgba_data.push(0);
                            rgba_data.push(0);
                            rgba_data.push(0);
                            rgba_data.push(0);
                        }
                    }
                }

                if rgba_data.len() == (width * height * 4) as usize {
                    self.items.push(RenderItem::Image(RenderImage {
                        width,
                        height,
                        rgba_data,
                        transform: self.current_graphics.ctm,
                        opacity: 1.0,
                        is_mask: true,
                        blend_mode: self.current_graphics.blend_mode,
                        star_node_id,
                        render_node_id,
                    }));
                }
                return;
            }

            let filter = stream_obj
                .dict
                .get("Filter")
                .and_then(PdfObject::as_name)
                .unwrap_or("");

            let raw_data = &stream_obj.data;
            let mut rgba_data = if filter == "DCTDecode" {
                // Decode JPEG
                let mut decoder = jpeg_decoder::Decoder::new(&raw_data[..]);
                match decoder.decode() {
                    Ok(pixels) => {
                        let info = decoder.info().unwrap();
                        match info.pixel_format {
                            jpeg_decoder::PixelFormat::RGB24 => {
                                let mut rgba = Vec::with_capacity((width * height * 4) as usize);
                                for chunk in pixels.chunks_exact(3) {
                                    rgba.push(chunk[0]);
                                    rgba.push(chunk[1]);
                                    rgba.push(chunk[2]);
                                    rgba.push(255);
                                }
                                rgba
                            }
                            jpeg_decoder::PixelFormat::L8 => {
                                let mut rgba = Vec::with_capacity((width * height * 4) as usize);
                                for &g in &pixels {
                                    rgba.push(g);
                                    rgba.push(g);
                                    rgba.push(g);
                                    rgba.push(255);
                                }
                                rgba
                            }
                            jpeg_decoder::PixelFormat::CMYK32 => {
                                let mut rgba = Vec::with_capacity((width * height * 4) as usize);
                                for chunk in pixels.chunks_exact(4) {
                                    let c = chunk[0] as f32 / 255.0;
                                    let m = chunk[1] as f32 / 255.0;
                                    let y = chunk[2] as f32 / 255.0;
                                    let k = chunk[3] as f32 / 255.0;
                                    let col = ColorRgba::from_cmyk(c, m, y, k);
                                    let [r, g, b, a] = col.to_u8_array();
                                    rgba.push(r);
                                    rgba.push(g);
                                    rgba.push(b);
                                    rgba.push(a);
                                }
                                rgba
                            }
                            jpeg_decoder::PixelFormat::L16 => {
                                let mut rgba = Vec::with_capacity((width * height * 4) as usize);
                                for chunk in pixels.chunks_exact(2) {
                                    let g = chunk[0];
                                    rgba.push(g);
                                    rgba.push(g);
                                    rgba.push(g);
                                    rgba.push(255);
                                }
                                rgba
                            }
                        }
                    }
                    Err(_) => {
                        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
                        for y in 0..height {
                            for x in 0..width {
                                let gray = if (x + y) % 2 == 0 { 210 } else { 170 };
                                rgba.push(gray);
                                rgba.push(gray);
                                rgba.push(gray);
                                rgba.push(255);
                            }
                        }
                        rgba
                    }
                }
            } else {
                // FlateDecode or uncompressed
                let decompressed = self
                    .doc
                    .decompress_stream_data(&stream_obj)
                    .unwrap_or_default();
                let cs_obj = stream_obj.dict.get("ColorSpace");

                if let Some(cs_arr) = cs_obj.and_then(PdfObject::as_array) {
                    if cs_arr.len() >= 4 && cs_arr[0].as_name() == Some("Indexed") {
                        let base_name = cs_arr[1].as_name().unwrap_or("DeviceRGB");
                        let hival = RenderCompiler::obj_num(&cs_arr[2]) as usize;
                        let lookup_bytes = match &cs_arr[3] {
                            PdfObject::String(bytes) => bytes.clone(),
                            PdfObject::Stream(st) => {
                                self.doc.decompress_stream_data(st).unwrap_or_default()
                            }
                            PdfObject::Reference(r) => {
                                let resolved = self.doc.store_mut().resolve(*r).cloned().ok();
                                match resolved {
                                    Some(PdfObject::String(bytes)) => bytes,
                                    Some(PdfObject::Stream(st)) => {
                                        self.doc.decompress_stream_data(&st).unwrap_or_default()
                                    }
                                    _ => Vec::new(),
                                }
                            }
                            _ => Vec::new(),
                        };

                        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
                        for &idx_byte in &decompressed {
                            let idx = (idx_byte as usize).min(hival);
                            if base_name == "DeviceRGB" {
                                let start = idx * 3;
                                if start + 2 < lookup_bytes.len() {
                                    rgba.push(lookup_bytes[start]);
                                    rgba.push(lookup_bytes[start + 1]);
                                    rgba.push(lookup_bytes[start + 2]);
                                    rgba.push(255);
                                } else {
                                    rgba.push(0);
                                    rgba.push(0);
                                    rgba.push(0);
                                    rgba.push(255);
                                }
                            } else if base_name == "DeviceGray" {
                                let g = if idx < lookup_bytes.len() {
                                    lookup_bytes[idx]
                                } else {
                                    0
                                };
                                rgba.push(g);
                                rgba.push(g);
                                rgba.push(g);
                                rgba.push(255);
                            } else if base_name == "DeviceCMYK" {
                                let start = idx * 4;
                                if start + 3 < lookup_bytes.len() {
                                    let c = lookup_bytes[start] as f32 / 255.0;
                                    let m = lookup_bytes[start + 1] as f32 / 255.0;
                                    let y = lookup_bytes[start + 2] as f32 / 255.0;
                                    let k = lookup_bytes[start + 3] as f32 / 255.0;
                                    let col = ColorRgba::from_cmyk(c, m, y, k);
                                    let [r, g, b, a] = col.to_u8_array();
                                    rgba.push(r);
                                    rgba.push(g);
                                    rgba.push(b);
                                    rgba.push(a);
                                } else {
                                    rgba.push(0);
                                    rgba.push(0);
                                    rgba.push(0);
                                    rgba.push(255);
                                }
                            } else {
                                rgba.push(idx_byte);
                                rgba.push(idx_byte);
                                rgba.push(idx_byte);
                                rgba.push(255);
                            }
                        }
                        rgba
                    } else {
                        RenderCompiler::decode_direct_pixels(&decompressed, cs_obj, width, height)
                    }
                } else {
                    RenderCompiler::decode_direct_pixels(&decompressed, cs_obj, width, height)
                }
            };

            // Apply Image /SMask (soft mask) if present
            if let Some(smask_ref) = stream_obj.dict.get("SMask") {
                if let Ok(smask_obj) = self.doc.store_mut().resolve_object(smask_ref) {
                    if let Some(smask_stream) = smask_obj.as_stream() {
                        if let Ok(smask_bytes) = self.doc.decompress_stream_data(&smask_stream) {
                            let decode_invert = if let Some(PdfObject::Array(dec)) = smask_stream.dict.get("Decode") {
                                if dec.len() >= 2 {
                                    dec[0].as_f64().unwrap_or(0.0) > dec[1].as_f64().unwrap_or(1.0)
                                } else {
                                    false
                                }
                            } else {
                                false
                            };
                            let total_px = (width * height) as usize;
                            if smask_bytes.len() == total_px {
                                for i in 0..total_px {
                                    if i * 4 + 3 < rgba_data.len() {
                                        let alpha_val = if decode_invert {
                                            255 - smask_bytes[i]
                                        } else {
                                            smask_bytes[i]
                                        };
                                        rgba_data[i * 4 + 3] = alpha_val;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if rgba_data.len() == (width * height * 4) as usize {
                self.items.push(RenderItem::Image(RenderImage {
                    width,
                    height,
                    rgba_data,
                    transform: self.current_graphics.ctm,
                    opacity: self.current_graphics.fill_alpha,
                    is_mask: false,
                    blend_mode: self.current_graphics.blend_mode,
                    star_node_id,
                    render_node_id,
                }));
            }
        }
    }

    fn emit_path(
        &mut self,
        fill: bool,
        stroke: bool,
        fill_rule: FillRule,
        close_before: bool,
        star_node_id: Option<StarNodeId>,
        render_node_id: Option<RenderNodeId>,
    ) {
        if close_before {
            self.current_path.push(PathVerb::Close);
        }

        if let Some(rule) = self.current_graphics.clip_pending.take() {
            self.items.push(RenderItem::PushClip(RenderClip {
                verbs: self.current_path.clone(),
                rule,
                transform: self.current_graphics.ctm,
            }));
            self.current_graphics.clip_depth += 1;
        }

        if !self.current_path.is_empty() {
            let fill_style = if fill {
                Some(FillStyle {
                    color: self.current_graphics.fill_color,
                    rule: fill_rule,
                })
            } else {
                None
            };

            let stroke_style = if stroke {
                Some(StrokeStyle {
                    color: self.current_graphics.stroke_color,
                    width: self.current_graphics.line_width,
                    cap: self.current_graphics.line_cap,
                    join: self.current_graphics.line_join,
                    miter_limit: self.current_graphics.miter_limit,
                    dash_pattern: self.current_graphics.dash_pattern.clone(),
                })
            } else {
                None
            };

            self.items.push(RenderItem::Path(RenderPath {
                verbs: std::mem::take(&mut self.current_path),
                fill: fill_style,
                stroke: stroke_style,
                transform: self.current_graphics.ctm,
                stroke_alpha: self.current_graphics.stroke_alpha,
                fill_alpha: self.current_graphics.fill_alpha,
                blend_mode: self.current_graphics.blend_mode,
                star_node_id,
                render_node_id,
            }));
        }

        self.current_path.clear();
    }

    fn show_text_run(
        &mut self,
        bytes: &[u8],
        star_node_id: Option<StarNodeId>,
        render_node_id: Option<RenderNodeId>,
    ) {
        let font_name = &self.text_state.font_name;
        let clean_name = font_name.strip_prefix('/').unwrap_or(font_name);

        let local_font = self.local_fonts_stack.iter().rev().find_map(|map| {
            map.get(clean_name)
                .or_else(|| map.get(font_name))
                .or_else(|| {
                    let with_slash = format!("/{}", clean_name);
                    map.get(&with_slash)
                })
        });

        let font = local_font
            .or_else(|| self.resources.get_font(font_name))
            .unwrap_or(&self.fallback_font);

        let decoded_glyphs = font.decode_bytes_full(bytes);
        if decoded_glyphs.is_empty() {
            return;
        }

        let mut glyphs = Vec::new();
        let mut local_x = 0.0f32;

        for span in decoded_glyphs {
            let scaled_advance = (span.width as f32) / 1000.0 * self.text_state.font_size;
            let spacing = self.text_state.char_spacing
                + if span.text == " " {
                    self.text_state.word_spacing
                } else {
                    0.0
                };
            let total_advance =
                (scaled_advance + spacing) * (self.text_state.horizontal_scaling / 100.0);

            let codepoint = span.text.chars().next().map_or(0, |c| c as u32);
            glyphs.push(RenderGlyph {
                codepoint,
                glyph_id: span.glyph_id,
                x: local_x,
                y: self.text_state.text_rise,
                advance: total_advance,
                text: span.text,
            });

            local_x += total_advance;
        }

        let combined_transform = self
            .text_state
            .text_matrix
            .multiply(&self.current_graphics.ctm);
        let font_data = font.embedded_sfnt.as_ref().map(|s| s.data.clone());

        self.items.push(RenderItem::TextRun(RenderTextRun {
            font_name: self.text_state.font_name.clone(),
            font_size: self.text_state.font_size,
            glyphs,
            fill_color: Some(self.current_graphics.fill_color),
            stroke_color: Some(self.current_graphics.stroke_color),
            transform: combined_transform,
            fill_alpha: self.current_graphics.fill_alpha,
            stroke_alpha: self.current_graphics.stroke_alpha,
            render_mode: self.current_graphics.text_render_mode,
            font_data,
            is_vertical: false,
            star_node_id,
            render_node_id,
        }));

        let advance_transform = Transform2D::translation(local_x, 0.0);
        self.text_state.text_matrix = advance_transform.multiply(&self.text_state.text_matrix);
    }

    fn extract_fonts_from_resources(
        res_dict: &std::collections::BTreeMap<String, PdfObject>,
        store: &mut ObjectStore<'_>,
    ) -> std::collections::BTreeMap<String, Font> {
        let mut fonts = std::collections::BTreeMap::new();
        if let Some(font_entry) = res_dict.get("Font") {
            if let Ok(resolved_fonts) = store.resolve_object(font_entry) {
                if let Some(font_map) = resolved_fonts.as_dict() {
                    for (font_name, font_obj_ref) in font_map {
                        if let Ok(resolved_font_obj) = store.resolve_object(font_obj_ref) {
                            if let Some(font_dict) = resolved_font_obj.as_dict() {
                                if let Ok(font) = Font::from_dict(font_name, font_dict, store) {
                                    fonts.insert(font_name.clone(), font);
                                }
                            }
                        }
                    }
                }
            }
        }
        fonts
    }

    fn process_annotations(&mut self) {
        let annots_arr = match self.page_dict.get("Annots") {
            Some(PdfObject::Array(arr)) => arr.clone(),
            Some(PdfObject::Reference(r)) => {
                if let Ok(PdfObject::Array(arr)) =
                    self.doc.store_mut().resolve_object(&PdfObject::Reference(*r))
                {
                    arr
                } else {
                    return;
                }
            }
            _ => return,
        };

        for annot_ref in annots_arr {
            let annot_obj = match self.doc.store_mut().resolve_object(&annot_ref) {
                Ok(obj) => obj,
                Err(_) => continue,
            };
            let annot_dict = match annot_obj.as_dict() {
                Some(d) => d.clone(),
                None => continue,
            };

            // Check annotation flags /F: bit 2 (Hidden = 2) or bit 6 (NoView = 32)
            if let Some(flags) = annot_dict.get("F").and_then(PdfObject::as_i64) {
                if (flags & 2) != 0 || (flags & 32) != 0 {
                    continue;
                }
            }

            // Resolve /Rect [x0 y0 x1 y1]
            let rect = match RenderCompiler::resolve_box(&annot_dict, "Rect") {
                Some(r) => r,
                None => continue,
            };
            let rx0 = rect[0].min(rect[2]);
            let rx1 = rect[0].max(rect[2]);
            let ry0 = rect[1].min(rect[3]);
            let ry1 = rect[1].max(rect[3]);
            let rect_w = rx1 - rx0;
            let rect_h = ry1 - ry0;
            if rect_w <= 0.0 || rect_h <= 0.0 {
                continue;
            }

            // Resolve /AP (Appearance dictionary)
            let ap_obj = match annot_dict.get("AP") {
                Some(o) => match self.doc.store_mut().resolve_object(o) {
                    Ok(resolved) => resolved,
                    Err(_) => continue,
                },
                None => continue,
            };

            // /AP can be a Stream directly, or a Dict with /N
            let n_stream = match ap_obj {
                PdfObject::Stream(s) => Some(s),
                PdfObject::Dictionary(ap_dict) => {
                    if let Some(n_ref) = ap_dict.get("N") {
                        if let Ok(n_obj) = self.doc.store_mut().resolve_object(n_ref) {
                            match n_obj {
                                PdfObject::Stream(s) => Some(s),
                                PdfObject::Dictionary(sub_dict) => {
                                    // Check /AS appearance state
                                    let as_state = annot_dict.get("AS").and_then(PdfObject::as_name);
                                    let target_ref = if let Some(state) = as_state {
                                        sub_dict.get(state)
                                    } else {
                                        sub_dict.values().next()
                                    };
                                    if let Some(t_ref) = target_ref {
                                        if let Ok(PdfObject::Stream(s)) =
                                            self.doc.store_mut().resolve_object(t_ref)
                                        {
                                            Some(s)
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                }
                                _ => None,
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            };

            let form_stream = match n_stream {
                Some(s) => s,
                None => continue,
            };

            // BBox of the form
            let bbox = RenderCompiler::resolve_box(&form_stream.dict, "BBox")
                .unwrap_or([0.0, 0.0, rect_w, rect_h]);
            let bx0 = bbox[0].min(bbox[2]);
            let bx1 = bbox[0].max(bbox[2]);
            let by0 = bbox[1].min(bbox[3]);
            let by1 = bbox[1].max(bbox[3]);
            let bw = (bx1 - bx0).max(0.001);
            let bh = (by1 - by0).max(0.001);

            let form_matrix = if let Some(m_arr) = form_stream.dict.get("Matrix").and_then(PdfObject::as_array) {
                if m_arr.len() >= 6 {
                    let m = Transform2D::new(
                        RenderCompiler::obj_num(&m_arr[0]),
                        RenderCompiler::obj_num(&m_arr[1]),
                        RenderCompiler::obj_num(&m_arr[2]),
                        RenderCompiler::obj_num(&m_arr[3]),
                        RenderCompiler::obj_num(&m_arr[4]),
                        RenderCompiler::obj_num(&m_arr[5]),
                    );
                    Transform2D::translation(rx0, ry0).multiply(&m)
                } else {
                    let sx = rect_w / bw;
                    let sy = rect_h / bh;
                    Transform2D::new(sx, 0.0, 0.0, sy, rx0 - bx0 * sx, ry0 - by0 * sy)
                }
            } else {
                let sx = rect_w / bw;
                let sy = rect_h / bh;
                Transform2D::new(sx, 0.0, 0.0, sy, rx0 - bx0 * sx, ry0 - by0 * sy)
            };

            let form_resources = if let Some(r_obj) = form_stream.dict.get("Resources") {
                self.doc
                    .store_mut()
                    .resolve_object(r_obj)
                    .ok()
                    .and_then(|o| o.as_dict().cloned())
            } else {
                None
            };

            let has_fonts = if let Some(ref res) = form_resources {
                if res.contains_key("Font") {
                    let fonts = Self::extract_fonts_from_resources(res, self.doc.store_mut());
                    self.local_fonts_stack.push(fonts);
                    true
                } else {
                    false
                }
            } else {
                false
            };

            let stream_bytes = match self.doc.decompress_stream_data(&form_stream) {
                Ok(b) => b,
                Err(_) => {
                    if has_fonts {
                        self.local_fonts_stack.pop();
                    }
                    continue;
                }
            };

            // Push graphics state and establish annotation CTM
            self.graphics_stack.push(self.current_graphics.clone());
            self.current_graphics = GraphicsState::default();
            self.current_graphics.ctm = form_matrix;

            // Push BBox clip
            let clip_verbs = vec![
                PathVerb::MoveTo(bbox[0], bbox[1]),
                PathVerb::LineTo(bbox[2], bbox[1]),
                PathVerb::LineTo(bbox[2], bbox[3]),
                PathVerb::LineTo(bbox[0], bbox[3]),
                PathVerb::Close,
            ];
            self.items.push(RenderItem::PushClip(RenderClip {
                verbs: clip_verbs,
                rule: FillRule::NonZero,
                transform: self.current_graphics.ctm,
            }));
            self.current_graphics.clip_depth += 1;

            // Process appearance stream
            self.recursion_depth += 1;
            self.process_stream(&stream_bytes, 88888, form_resources.as_ref());
            self.recursion_depth -= 1;

            // Pop clip
            self.items.push(RenderItem::PopClip);
            self.current_graphics.clip_depth = self.current_graphics.clip_depth.saturating_sub(1);

            if has_fonts {
                self.local_fonts_stack.pop();
            }

            // Restore graphics state
            if let Some(saved) = self.graphics_stack.pop() {
                self.current_graphics = saved;
            }
        }
    }
}
