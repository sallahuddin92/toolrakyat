use crate::content::operand::ContentOperand;
use crate::content::operator::ContentOperator;
use crate::content::parser::ContentParser;
use crate::document::PdfDocument;
use crate::error::PdfResult;
use crate::execution::{RenderNodeId, StarNodeId};
use crate::font::font::Font;
use crate::font::resource::PageResources;
use crate::render::ir::{
    ColorRgba, FillRule, FillStyle, LineCap, LineJoin, PathVerb, RenderClip, RenderGlyph,
    RenderImage, RenderItem, RenderPage, RenderPath, RenderTextRun, StrokeStyle, Transform2D,
    UnsupportedOperatorReport,
};
use crate::syntax::object::PdfObject;

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
    clip_active: bool,
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
            clip_active: false,
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

        // 4. Process Content Streams into Render Items
        let mut items = Vec::new();
        let mut unsupported_operators = Vec::new();

        let mut graphics_stack: Vec<GraphicsState> = Vec::new();
        let mut current_graphics = GraphicsState::default();
        let mut text_state = TextState::default();

        let mut current_path: Vec<PathVerb> = Vec::new();
        let mut subpath_start: Option<(f32, f32)> = None;
        let mut current_point: Option<(f32, f32)> = None;

        let fallback_font = Font::standard_fallback("Helvetica");

        for (stream_index, stream_bytes) in decompressed_streams.iter().enumerate() {
            let mut parser = ContentParser::from_bytes(stream_bytes);
            let instructions = match parser.parse_instructions() {
                Ok(inst) => inst,
                Err(_) => continue,
            };

            for (inst_idx, inst) in instructions.into_iter().enumerate() {
                let occurrence = format!("p{page_index}/s{stream_index}/i{inst_idx}");
                let star_node_id = StarNodeId::new(&doc_id, page_ref, &occurrence).ok();
                let render_node_id = star_node_id
                    .as_ref()
                    .and_then(|sn| RenderNodeId::new(sn, &format!("op_{}", inst.operator.as_str())).ok());

                match inst.operator {
                    ContentOperator::Q => {
                        graphics_stack.push(current_graphics.clone());
                    }
                    ContentOperator::QEnd => {
                        if let Some(prev) = graphics_stack.pop() {
                            if current_graphics.clip_active && !prev.clip_active {
                                items.push(RenderItem::PopClip);
                            }
                            current_graphics = prev;
                        }
                    }
                    ContentOperator::Cm => {
                        if inst.operands.len() >= 6 {
                            let a = Self::num(&inst.operands[0]);
                            let b = Self::num(&inst.operands[1]);
                            let c = Self::num(&inst.operands[2]);
                            let d = Self::num(&inst.operands[3]);
                            let e = Self::num(&inst.operands[4]);
                            let f = Self::num(&inst.operands[5]);
                            let cm = Transform2D::new(a, b, c, d, e, f);
                            current_graphics.ctm = cm.multiply(&current_graphics.ctm);
                        }
                    }

                    // Path construction
                    ContentOperator::M => {
                        if inst.operands.len() >= 2 {
                            let x = Self::num(&inst.operands[0]);
                            let y = Self::num(&inst.operands[1]);
                            current_path.push(PathVerb::MoveTo(x, y));
                            subpath_start = Some((x, y));
                            current_point = Some((x, y));
                        }
                    }
                    ContentOperator::L => {
                        if inst.operands.len() >= 2 {
                            let x = Self::num(&inst.operands[0]);
                            let y = Self::num(&inst.operands[1]);
                            current_path.push(PathVerb::LineTo(x, y));
                            current_point = Some((x, y));
                        }
                    }
                    ContentOperator::C => {
                        if inst.operands.len() >= 6 {
                            let x1 = Self::num(&inst.operands[0]);
                            let y1 = Self::num(&inst.operands[1]);
                            let x2 = Self::num(&inst.operands[2]);
                            let y2 = Self::num(&inst.operands[3]);
                            let x3 = Self::num(&inst.operands[4]);
                            let y3 = Self::num(&inst.operands[5]);
                            current_path.push(PathVerb::CubicTo(x1, y1, x2, y2, x3, y3));
                            current_point = Some((x3, y3));
                        }
                    }
                    ContentOperator::V => {
                        if inst.operands.len() >= 4 {
                            let (x1, y1) = current_point.unwrap_or((0.0, 0.0));
                            let x2 = Self::num(&inst.operands[0]);
                            let y2 = Self::num(&inst.operands[1]);
                            let x3 = Self::num(&inst.operands[2]);
                            let y3 = Self::num(&inst.operands[3]);
                            current_path.push(PathVerb::CubicTo(x1, y1, x2, y2, x3, y3));
                            current_point = Some((x3, y3));
                        }
                    }
                    ContentOperator::Y => {
                        if inst.operands.len() >= 4 {
                            let x1 = Self::num(&inst.operands[0]);
                            let y1 = Self::num(&inst.operands[1]);
                            let x3 = Self::num(&inst.operands[2]);
                            let y3 = Self::num(&inst.operands[3]);
                            current_path.push(PathVerb::CubicTo(x1, y1, x3, y3, x3, y3));
                            current_point = Some((x3, y3));
                        }
                    }
                    ContentOperator::Re => {
                        if inst.operands.len() >= 4 {
                            let x = Self::num(&inst.operands[0]);
                            let y = Self::num(&inst.operands[1]);
                            let w = Self::num(&inst.operands[2]);
                            let h = Self::num(&inst.operands[3]);
                            current_path.push(PathVerb::MoveTo(x, y));
                            current_path.push(PathVerb::LineTo(x + w, y));
                            current_path.push(PathVerb::LineTo(x + w, y + h));
                            current_path.push(PathVerb::LineTo(x, y + h));
                            current_path.push(PathVerb::Close);
                            subpath_start = Some((x, y));
                            current_point = Some((x, y));
                        }
                    }
                    ContentOperator::H => {
                        current_path.push(PathVerb::Close);
                        current_point = subpath_start;
                    }

                    // Clipping
                    ContentOperator::W => {
                        current_graphics.clip_pending = Some(FillRule::NonZero);
                    }
                    ContentOperator::WStar => {
                        current_graphics.clip_pending = Some(FillRule::EvenOdd);
                    }

                    // Path painting
                    ContentOperator::F | ContentOperator::FUpper => {
                        Self::emit_path(
                            &mut items,
                            &mut current_path,
                            &mut current_graphics,
                            true,
                            false,
                            FillRule::NonZero,
                            false,
                            star_node_id,
                            render_node_id,
                        );
                    }
                    ContentOperator::FStar => {
                        Self::emit_path(
                            &mut items,
                            &mut current_path,
                            &mut current_graphics,
                            true,
                            false,
                            FillRule::EvenOdd,
                            false,
                            star_node_id,
                            render_node_id,
                        );
                    }
                    ContentOperator::S => {
                        Self::emit_path(
                            &mut items,
                            &mut current_path,
                            &mut current_graphics,
                            false,
                            true,
                            FillRule::NonZero,
                            false,
                            star_node_id,
                            render_node_id,
                        );
                    }
                    ContentOperator::SClose => {
                        Self::emit_path(
                            &mut items,
                            &mut current_path,
                            &mut current_graphics,
                            false,
                            true,
                            FillRule::NonZero,
                            true,
                            star_node_id,
                            render_node_id,
                        );
                    }
                    ContentOperator::B => {
                        Self::emit_path(
                            &mut items,
                            &mut current_path,
                            &mut current_graphics,
                            true,
                            true,
                            FillRule::NonZero,
                            false,
                            star_node_id,
                            render_node_id,
                        );
                    }
                    ContentOperator::BStar => {
                        Self::emit_path(
                            &mut items,
                            &mut current_path,
                            &mut current_graphics,
                            true,
                            true,
                            FillRule::EvenOdd,
                            false,
                            star_node_id,
                            render_node_id,
                        );
                    }
                    ContentOperator::BClose => {
                        Self::emit_path(
                            &mut items,
                            &mut current_path,
                            &mut current_graphics,
                            true,
                            true,
                            FillRule::NonZero,
                            true,
                            star_node_id,
                            render_node_id,
                        );
                    }
                    ContentOperator::BCloseStar => {
                        Self::emit_path(
                            &mut items,
                            &mut current_path,
                            &mut current_graphics,
                            true,
                            true,
                            FillRule::EvenOdd,
                            true,
                            star_node_id,
                            render_node_id,
                        );
                    }
                    ContentOperator::N => {
                        if let Some(rule) = current_graphics.clip_pending.take() {
                            items.push(RenderItem::PushClip(RenderClip {
                                verbs: current_path.clone(),
                                rule,
                                transform: current_graphics.ctm,
                            }));
                            current_graphics.clip_active = true;
                        }
                        current_path.clear();
                        subpath_start = None;
                        current_point = None;
                    }

                    // Stroke & Fill Settings
                    ContentOperator::LineWidth => {
                        if !inst.operands.is_empty() {
                            current_graphics.line_width = Self::num(&inst.operands[0]).max(0.1);
                        }
                    }
                    ContentOperator::LineCap => {
                        if !inst.operands.is_empty() {
                            current_graphics.line_cap = match Self::num(&inst.operands[0]) as i32 {
                                1 => LineCap::Round,
                                2 => LineCap::Square,
                                _ => LineCap::Butt,
                            };
                        }
                    }
                    ContentOperator::LineJoin => {
                        if !inst.operands.is_empty() {
                            current_graphics.line_join = match Self::num(&inst.operands[0]) as i32 {
                                1 => LineJoin::Round,
                                2 => LineJoin::Bevel,
                                _ => LineJoin::Miter,
                            };
                        }
                    }
                    ContentOperator::MiterLimit => {
                        if !inst.operands.is_empty() {
                            current_graphics.miter_limit = Self::num(&inst.operands[0]).max(1.0);
                        }
                    }
                    ContentOperator::Dash => {
                        if inst.operands.len() >= 2 {
                            let phase = Self::num(&inst.operands[1]);
                            if let ContentOperand::Array(arr) = &inst.operands[0] {
                                let dashes: Vec<f32> = arr.iter().map(Self::num).collect();
                                current_graphics.dash_pattern = if dashes.is_empty() {
                                    None
                                } else {
                                    Some((dashes, phase))
                                };
                            }
                        }
                    }
                    ContentOperator::GFill => {
                        if !inst.operands.is_empty() {
                            current_graphics.fill_color = ColorRgba::from_gray(Self::num(&inst.operands[0]));
                        }
                    }
                    ContentOperator::GStroke => {
                        if !inst.operands.is_empty() {
                            current_graphics.stroke_color = ColorRgba::from_gray(Self::num(&inst.operands[0]));
                        }
                    }
                    ContentOperator::RGFill => {
                        if inst.operands.len() >= 3 {
                            let r = Self::num(&inst.operands[0]);
                            let g = Self::num(&inst.operands[1]);
                            let b = Self::num(&inst.operands[2]);
                            current_graphics.fill_color = ColorRgba::from_rgb(r, g, b);
                        }
                    }
                    ContentOperator::RGStroke => {
                        if inst.operands.len() >= 3 {
                            let r = Self::num(&inst.operands[0]);
                            let g = Self::num(&inst.operands[1]);
                            let b = Self::num(&inst.operands[2]);
                            current_graphics.stroke_color = ColorRgba::from_rgb(r, g, b);
                        }
                    }
                    ContentOperator::KFill => {
                        if inst.operands.len() >= 4 {
                            let c = Self::num(&inst.operands[0]);
                            let m = Self::num(&inst.operands[1]);
                            let y = Self::num(&inst.operands[2]);
                            let k = Self::num(&inst.operands[3]);
                            current_graphics.fill_color = ColorRgba::from_cmyk(c, m, y, k);
                        }
                    }
                    ContentOperator::KStroke => {
                        if inst.operands.len() >= 4 {
                            let c = Self::num(&inst.operands[0]);
                            let m = Self::num(&inst.operands[1]);
                            let y = Self::num(&inst.operands[2]);
                            let k = Self::num(&inst.operands[3]);
                            current_graphics.stroke_color = ColorRgba::from_cmyk(c, m, y, k);
                        }
                    }

                    // Text Operators
                    ContentOperator::Bt => {
                        text_state.text_matrix = Transform2D::identity();
                        text_state.text_line_matrix = Transform2D::identity();
                    }
                    ContentOperator::Et => {}
                    ContentOperator::Tf => {
                        if inst.operands.len() >= 2 {
                            if let ContentOperand::Name(name) = &inst.operands[0] {
                                text_state.font_name = name.clone();
                            }
                            text_state.font_size = Self::num(&inst.operands[1]).max(0.1);
                        }
                    }
                    ContentOperator::Tm => {
                        if inst.operands.len() >= 6 {
                            let a = Self::num(&inst.operands[0]);
                            let b = Self::num(&inst.operands[1]);
                            let c = Self::num(&inst.operands[2]);
                            let d = Self::num(&inst.operands[3]);
                            let e = Self::num(&inst.operands[4]);
                            let f = Self::num(&inst.operands[5]);
                            let tm = Transform2D::new(a, b, c, d, e, f);
                            text_state.text_matrix = tm;
                            text_state.text_line_matrix = tm;
                        }
                    }
                    ContentOperator::Td => {
                        if inst.operands.len() >= 2 {
                            let tx = Self::num(&inst.operands[0]);
                            let ty = Self::num(&inst.operands[1]);
                            let translation = Transform2D::translation(tx, ty);
                            let new_matrix = translation.multiply(&text_state.text_line_matrix);
                            text_state.text_matrix = new_matrix;
                            text_state.text_line_matrix = new_matrix;
                        }
                    }
                    ContentOperator::TD => {
                        if inst.operands.len() >= 2 {
                            let tx = Self::num(&inst.operands[0]);
                            let ty = Self::num(&inst.operands[1]);
                            text_state.leading = -ty;
                            let translation = Transform2D::translation(tx, ty);
                            let new_matrix = translation.multiply(&text_state.text_line_matrix);
                            text_state.text_matrix = new_matrix;
                            text_state.text_line_matrix = new_matrix;
                        }
                    }
                    ContentOperator::TStar => {
                        let translation = Transform2D::translation(0.0, -text_state.leading);
                        let new_matrix = translation.multiply(&text_state.text_line_matrix);
                        text_state.text_matrix = new_matrix;
                        text_state.text_line_matrix = new_matrix;
                    }
                    ContentOperator::Tc => {
                        if !inst.operands.is_empty() {
                            text_state.char_spacing = Self::num(&inst.operands[0]);
                        }
                    }
                    ContentOperator::Tw => {
                        if !inst.operands.is_empty() {
                            text_state.word_spacing = Self::num(&inst.operands[0]);
                        }
                    }
                    ContentOperator::Tz => {
                        if !inst.operands.is_empty() {
                            text_state.horizontal_scaling = Self::num(&inst.operands[0]);
                        }
                    }
                    ContentOperator::TL => {
                        if !inst.operands.is_empty() {
                            text_state.leading = Self::num(&inst.operands[0]);
                        }
                    }
                    ContentOperator::Ts => {
                        if !inst.operands.is_empty() {
                            text_state.text_rise = Self::num(&inst.operands[0]);
                        }
                    }

                    // Text Showing
                    ContentOperator::Tj => {
                        if let Some(ContentOperand::String(bytes)) = inst.operands.first() {
                            Self::show_text_run(
                                &mut items,
                                bytes,
                                &resources,
                                &fallback_font,
                                &mut text_state,
                                &current_graphics,
                                star_node_id,
                                render_node_id,
                            );
                        }
                    }
                    ContentOperator::TJ => {
                        if let Some(ContentOperand::Array(elements)) = inst.operands.first() {
                            for elem in elements {
                                match elem {
                                    ContentOperand::String(bytes) => {
                                        Self::show_text_run(
                                            &mut items,
                                            bytes,
                                            &resources,
                                            &fallback_font,
                                            &mut text_state,
                                            &current_graphics,
                                            star_node_id.clone(),
                                            render_node_id.clone(),
                                        );
                                    }
                                    ContentOperand::Integer(n) => {
                                        let adj = -(*n as f32) / 1000.0
                                            * text_state.font_size
                                            * (text_state.horizontal_scaling / 100.0);
                                        let t = Transform2D::translation(adj, 0.0);
                                        text_state.text_matrix = t.multiply(&text_state.text_matrix);
                                    }
                                    ContentOperand::Real(r) => {
                                        let adj = -(*r as f32) / 1000.0
                                            * text_state.font_size
                                            * (text_state.horizontal_scaling / 100.0);
                                        let t = Transform2D::translation(adj, 0.0);
                                        text_state.text_matrix = t.multiply(&text_state.text_matrix);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    ContentOperator::Quote => {
                        let translation = Transform2D::translation(0.0, -text_state.leading);
                        text_state.text_matrix = translation.multiply(&text_state.text_line_matrix);
                        text_state.text_line_matrix = text_state.text_matrix;
                        if let Some(ContentOperand::String(bytes)) = inst.operands.first() {
                            Self::show_text_run(
                                &mut items,
                                bytes,
                                &resources,
                                &fallback_font,
                                &mut text_state,
                                &current_graphics,
                                star_node_id,
                                render_node_id,
                            );
                        }
                    }
                    ContentOperator::DoubleQuote => {
                        if inst.operands.len() >= 3 {
                            text_state.word_spacing = Self::num(&inst.operands[0]);
                            text_state.char_spacing = Self::num(&inst.operands[1]);
                            let translation = Transform2D::translation(0.0, -text_state.leading);
                            text_state.text_matrix = translation.multiply(&text_state.text_line_matrix);
                            text_state.text_line_matrix = text_state.text_matrix;
                            if let ContentOperand::String(bytes) = &inst.operands[2] {
                                Self::show_text_run(
                                    &mut items,
                                    bytes,
                                    &resources,
                                    &fallback_font,
                                    &mut text_state,
                                    &current_graphics,
                                    star_node_id,
                                    render_node_id,
                                );
                            }
                        }
                    }

                    // XObject Invocation
                    ContentOperator::Do => {
                        if let Some(ContentOperand::Name(res_name)) = inst.operands.first() {
                            Self::compile_xobject(
                                doc,
                                &page_dict,
                                res_name,
                                &current_graphics.ctm,
                                &mut items,
                                star_node_id,
                                render_node_id,
                            );
                        }
                    }

                    // Ignored harmless operators
                    ContentOperator::Gs
                    | ContentOperator::CSFill
                    | ContentOperator::CSStroke
                    | ContentOperator::SCFill
                    | ContentOperator::SCStroke
                    | ContentOperator::SCNFill
                    | ContentOperator::SCNStroke => {}

                    // Report unsupported operators
                    other => {
                        unsupported_operators.push(UnsupportedOperatorReport {
                            operator: other.as_str().to_string(),
                            operands: inst.operands.iter().map(|o| format!("{o:?}")).collect(),
                            stream_index,
                            instruction_index: inst_idx,
                        });
                    }
                }
            }
        }

        Ok(RenderPage {
            page_index,
            width,
            height,
            media_box,
            crop_box,
            rotate,
            items,
            unsupported_operators,
        })
    }

    fn emit_path(
        items: &mut Vec<RenderItem>,
        current_path: &mut Vec<PathVerb>,
        current_graphics: &mut GraphicsState,
        fill: bool,
        stroke: bool,
        fill_rule: FillRule,
        close_before: bool,
        star_node_id: Option<StarNodeId>,
        render_node_id: Option<RenderNodeId>,
    ) {
        if close_before {
            current_path.push(PathVerb::Close);
        }

        if let Some(rule) = current_graphics.clip_pending.take() {
            items.push(RenderItem::PushClip(RenderClip {
                verbs: current_path.clone(),
                rule,
                transform: current_graphics.ctm,
            }));
            current_graphics.clip_active = true;
        }

        if !current_path.is_empty() {
            let fill_style = if fill {
                Some(FillStyle {
                    color: current_graphics.fill_color,
                    rule: fill_rule,
                })
            } else {
                None
            };

            let stroke_style = if stroke {
                Some(StrokeStyle {
                    color: current_graphics.stroke_color,
                    width: current_graphics.line_width,
                    cap: current_graphics.line_cap,
                    join: current_graphics.line_join,
                    miter_limit: current_graphics.miter_limit,
                    dash_pattern: current_graphics.dash_pattern.clone(),
                })
            } else {
                None
            };

            items.push(RenderItem::Path(RenderPath {
                verbs: std::mem::take(current_path),
                fill: fill_style,
                stroke: stroke_style,
                transform: current_graphics.ctm,
                star_node_id,
                render_node_id,
            }));
        }

        current_path.clear();
    }

    fn show_text_run(
        items: &mut Vec<RenderItem>,
        bytes: &[u8],
        resources: &PageResources,
        fallback_font: &Font,
        text_state: &mut TextState,
        current_graphics: &GraphicsState,
        star_node_id: Option<StarNodeId>,
        render_node_id: Option<RenderNodeId>,
    ) {
        let font = resources
            .fonts
            .get(&text_state.font_name)
            .unwrap_or(fallback_font);

        let decoded_glyphs = font.decode_bytes(bytes);
        if decoded_glyphs.is_empty() {
            return;
        }

        let mut glyphs = Vec::new();
        let mut local_x = 0.0f32;

        for (text_chunk, char_width) in decoded_glyphs {
            let scaled_advance = (char_width as f32) / 1000.0 * text_state.font_size;
            let spacing = text_state.char_spacing
                + if text_chunk == " " {
                    text_state.word_spacing
                } else {
                    0.0
                };
            let total_advance = (scaled_advance + spacing) * (text_state.horizontal_scaling / 100.0);

            let codepoint = text_chunk.chars().next().map_or(0, |c| c as u32);
            glyphs.push(RenderGlyph {
                codepoint,
                glyph_id: None,
                x: local_x,
                y: text_state.text_rise,
                advance: total_advance,
                text: text_chunk,
            });

            local_x += total_advance;
        }

        // Full matrix: CTM * TextMatrix * TextScaling
        let combined_transform = text_state.text_matrix.multiply(&current_graphics.ctm);

        let font_data = font.embedded_sfnt.as_ref().map(|s| s.data.clone());

        items.push(RenderItem::TextRun(RenderTextRun {
            font_name: text_state.font_name.clone(),
            font_size: text_state.font_size,
            glyphs,
            fill_color: Some(current_graphics.fill_color),
            stroke_color: None,
            transform: combined_transform,
            font_data,
            is_vertical: false,
            star_node_id,
            render_node_id,
        }));

        // Advance text matrix
        let advance_transform = Transform2D::translation(local_x, 0.0);
        text_state.text_matrix = advance_transform.multiply(&text_state.text_matrix);
    }

    fn compile_xobject(
        doc: &mut PdfDocument<'_>,
        page_dict: &std::collections::BTreeMap<String, PdfObject>,
        res_name: &str,
        ctm: &Transform2D,
        items: &mut Vec<RenderItem>,
        star_node_id: Option<StarNodeId>,
        render_node_id: Option<RenderNodeId>,
    ) {
        let stream_obj = match Self::find_xobject_stream(doc, page_dict, res_name) {
            Some(s) => s,
            None => return,
        };

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
                .unwrap_or(0) as u32;
            let height = stream_obj
                .dict
                .get("Height")
                .and_then(PdfObject::as_i64)
                .unwrap_or(0) as u32;

            if width == 0 || height == 0 {
                return;
            }

            let filter = stream_obj
                .dict
                .get("Filter")
                .and_then(PdfObject::as_name)
                .unwrap_or("");

            let raw_data = &stream_obj.data;
            let rgba_data = if filter == "DCTDecode" {
                // Decode JPEG
                let mut decoder = jpeg_decoder::Decoder::new(&raw_data[..]);
                let res = decoder.decode();
                match res {
                    Ok(pixels) => {
                        let info = decoder.info().unwrap();
                        let pixel_format = info.pixel_format;
                        match pixel_format {
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
                let decompressed = doc.decompress_stream_data(&stream_obj).unwrap_or_default();
                let color_space = stream_obj
                    .dict
                    .get("ColorSpace")
                    .and_then(PdfObject::as_name)
                    .unwrap_or("DeviceRGB");

                match color_space {
                    "DeviceGray" => {
                        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
                        for &g in &decompressed {
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
                        // Assume RGB
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
            };

            if rgba_data.len() == (width * height * 4) as usize {
                items.push(RenderItem::Image(RenderImage {
                    width,
                    height,
                    rgba_data,
                    transform: *ctm,
                    opacity: 1.0,
                    star_node_id,
                    render_node_id,
                }));
            }
        }
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

    fn find_xobject_stream(
        doc: &mut PdfDocument<'_>,
        page_dict: &std::collections::BTreeMap<String, PdfObject>,
        res_name: &str,
    ) -> Option<crate::syntax::object::StreamObject> {
        // 1. Direct page resources
        if let Some(res_obj) = page_dict.get("Resources") {
            if let Ok(resolved_res) = doc.store_mut().resolve_object(res_obj) {
                if let Some(res_dict) = resolved_res.as_dict() {
                    if let Some(xo_obj) = res_dict.get("XObject") {
                        if let Ok(resolved_xo) = doc.store_mut().resolve_object(xo_obj) {
                            if let Some(xo_dict) = resolved_xo.as_dict() {
                                if let Some(target_obj) = xo_dict.get(res_name) {
                                    if let Ok(resolved_target) = doc.store_mut().resolve_object(target_obj) {
                                        if let Some(stream) = resolved_target.as_stream() {
                                            return Some(stream.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 2. Walk Parent chain if not found directly
        let mut current_parent = page_dict.get("Parent").and_then(PdfObject::as_reference);
        while let Some(parent_ref) = current_parent {
            if let Ok(parent_obj) = doc.store_mut().resolve(parent_ref).cloned() {
                if let Some(parent_dict) = parent_obj.as_dict() {
                    if let Some(res_obj) = parent_dict.get("Resources") {
                        if let Ok(resolved_res) = doc.store_mut().resolve_object(res_obj) {
                            if let Some(res_dict) = resolved_res.as_dict() {
                                if let Some(xo_obj) = res_dict.get("XObject") {
                                    if let Ok(resolved_xo) = doc.store_mut().resolve_object(xo_obj) {
                                        if let Some(xo_dict) = resolved_xo.as_dict() {
                                            if let Some(target_obj) = xo_dict.get(res_name) {
                                                if let Ok(resolved_target) = doc.store_mut().resolve_object(target_obj) {
                                                    if let Some(stream) = resolved_target.as_stream() {
                                                        return Some(stream.clone());
                                                    }
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
}
