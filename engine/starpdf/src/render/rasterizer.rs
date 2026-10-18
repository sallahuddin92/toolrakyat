use std::time::Instant;

use read_fonts::FontRef;
use skrifa::instance::Size;
use skrifa::outline::OutlinePen;
use skrifa::MetadataProvider;

use crate::render::ir::{
    LineCap, LineJoin, PathVerb, RenderItem, RenderPage, Transform2D,
};

struct SkiaPen<'a> {
    builder: &'a mut tiny_skia::PathBuilder,
}

impl<'a> OutlinePen for SkiaPen<'a> {
    fn move_to(&mut self, x: f32, y: f32) {
        self.builder.move_to(x, y);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.builder.line_to(x, y);
    }
    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.builder.quad_to(cx0, cy0, x, y);
    }
    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.builder.cubic_to(cx0, cy0, cx1, cy1, x, y);
    }
    fn close(&mut self) {
        self.builder.close();
    }
}

#[derive(Debug, Clone)]
pub struct RenderedPage {
    pub page_index: usize,
    pub width: u32,
    pub height: u32,
    pub rgba_pixels: Vec<u8>,
    pub duration_ms: f64,
    pub unsupported_operators: Vec<String>,
}

impl RenderedPage {
    #[inline]
    pub fn has_unsupported_operators(&self) -> bool {
        !self.unsupported_operators.is_empty()
    }
}

pub struct NativeRasterizer;

impl NativeRasterizer {
    /// Renders a compiled `RenderPage` into an RGBA pixmap at the given scale factor.
    pub fn render(page: &RenderPage, scale: f32) -> Option<RenderedPage> {
        let start_time = Instant::now();
        let scale = scale.max(0.1);

        let target_width = ((page.width * scale).round() as u32).max(1);
        let target_height = ((page.height * scale).round() as u32).max(1);

        let mut pixmap = tiny_skia::Pixmap::new(target_width, target_height)?;
        pixmap.fill(tiny_skia::Color::WHITE);

        // Page to Pixmap transform:
        // PDF user space: (0,0) at bottom-left, y goes up.
        // Pixmap screen space: (0,0) at top-left, y goes down.
        let page_transform = Transform2D::new(
            scale,
            0.0,
            0.0,
            -scale,
            -page.crop_box[0] * scale,
            page.crop_box[3] * scale,
        );

        let mut mask_stack: Vec<tiny_skia::Mask> = Vec::new();

        // Load fallback font for text rendering
        let fallback_font_bytes = Self::load_system_fallback_font();

        for item in &page.items {
            match item {
                RenderItem::PushClip(clip) => {
                    let mut pb = tiny_skia::PathBuilder::new();
                    Self::build_path(&mut pb, &clip.verbs);
                    if let Some(path) = pb.finish() {
                        let combined_tf = clip.transform.multiply(&page_transform).to_tiny_skia();
                        if let Some(mut new_mask) =
                            tiny_skia::Mask::new(target_width, target_height)
                        {
                            new_mask.fill_path(
                                &path,
                                clip.rule.to_tiny_skia(),
                                true,
                                combined_tf,
                            );
                            if let Some(prev) = mask_stack.last() {
                                for (curr, p) in new_mask.data_mut().iter_mut().zip(prev.data()) {
                                    *curr = ((*curr as u16 * *p as u16) / 255) as u8;
                                }
                            }
                            mask_stack.push(new_mask);
                        }
                    }
                }
                RenderItem::PopClip => {
                    mask_stack.pop();
                }
                RenderItem::Path(render_path) => {
                    let mut pb = tiny_skia::PathBuilder::new();
                    Self::build_path(&mut pb, &render_path.verbs);
                    if let Some(path) = pb.finish() {
                        let combined_tf = render_path.transform.multiply(&page_transform).to_tiny_skia();
                        let active_mask = mask_stack.last();

                        // Fill
                        if let Some(fill) = &render_path.fill {
                            let mut paint = tiny_skia::Paint::default();
                            let [r, g, b, a] = fill.color.to_u8_array();
                            paint.set_color_rgba8(r, g, b, a);
                            paint.anti_alias = true;

                            pixmap.fill_path(
                                &path,
                                &paint,
                                fill.rule.to_tiny_skia(),
                                combined_tf,
                                active_mask,
                            );
                        }

                        // Stroke
                        if let Some(stroke) = &render_path.stroke {
                            let mut paint = tiny_skia::Paint::default();
                            let [r, g, b, a] = stroke.color.to_u8_array();
                            paint.set_color_rgba8(r, g, b, a);
                            paint.anti_alias = true;

                            let mut skia_stroke = tiny_skia::Stroke::default();
                            skia_stroke.width = stroke.width;
                            skia_stroke.miter_limit = stroke.miter_limit;
                            skia_stroke.line_cap = match stroke.cap {
                                LineCap::Butt => tiny_skia::LineCap::Butt,
                                LineCap::Round => tiny_skia::LineCap::Round,
                                LineCap::Square => tiny_skia::LineCap::Square,
                            };
                            skia_stroke.line_join = match stroke.join {
                                LineJoin::Miter => tiny_skia::LineJoin::Miter,
                                LineJoin::Round => tiny_skia::LineJoin::Round,
                                LineJoin::Bevel => tiny_skia::LineJoin::Bevel,
                            };
                            if let Some((dashes, phase)) = &stroke.dash_pattern {
                                if let Some(dash) = tiny_skia::StrokeDash::new(dashes.clone(), *phase) {
                                    skia_stroke.dash = Some(dash);
                                }
                            }

                            pixmap.stroke_path(
                                &path,
                                &paint,
                                &skia_stroke,
                                combined_tf,
                                active_mask,
                            );
                        }
                    }
                }
                RenderItem::Image(render_img) => {
                    if let Some(img_pixmap) = tiny_skia::PixmapRef::from_bytes(
                        &render_img.rgba_data,
                        render_img.width,
                        render_img.height,
                    ) {
                        let active_mask = mask_stack.last();
                        let paint = tiny_skia::PixmapPaint {
                            opacity: render_img.opacity,
                            blend_mode: tiny_skia::BlendMode::SourceOver,
                            quality: tiny_skia::FilterQuality::Bilinear,
                        };

                        // Image space mapping:
                        // Pixel coordinates: (0,0) to (W, H)
                        // PDF unit image square: (0,0) bottom-left, (1,1) top-right
                        let sx = 1.0 / render_img.width as f32;
                        let sy = -1.0 / render_img.height as f32;
                        let img_unit_tf = Transform2D::new(sx, 0.0, 0.0, sy, 0.0, 1.0);
                        let final_tf = img_unit_tf
                            .multiply(&render_img.transform)
                            .multiply(&page_transform)
                            .to_tiny_skia();

                        pixmap.draw_pixmap(0, 0, img_pixmap, &paint, final_tf, active_mask);
                    }
                }
                RenderItem::TextRun(text_run) => {
                    let active_mask = mask_stack.last();
                    let font_bytes = text_run
                        .font_data
                        .as_deref()
                        .or(fallback_font_bytes.as_deref());

                    if let Some(bytes) = font_bytes {
                        if let Ok(font_ref) = FontRef::from_index(bytes, 0) {
                            let outlines = font_ref.outline_glyphs();
                            let charmap = font_ref.charmap();

                            let mut paint = tiny_skia::Paint::default();
                            let col = text_run.fill_color.unwrap_or(crate::render::ir::ColorRgba::BLACK);
                            let [r, g, b, a] = col.to_u8_array();
                            paint.set_color_rgba8(r, g, b, a);
                            paint.anti_alias = true;

                            for glyph in &text_run.glyphs {
                                let gid = glyph
                                    .glyph_id
                                    .map(|g| skrifa::GlyphId::new(g as u32))
                                    .or_else(|| {
                                        char::from_u32(glyph.codepoint)
                                            .and_then(|ch| charmap.map(ch))
                                    });

                                if let Some(gid) = gid {
                                    if let Some(outline_glyph) = outlines.get(gid) {
                                        let mut pb = tiny_skia::PathBuilder::new();
                                        let mut pen = SkiaPen { builder: &mut pb };
                                        let draw_settings = skrifa::outline::DrawSettings::unhinted(
                                            Size::new(text_run.font_size),
                                            skrifa::instance::LocationRef::default(),
                                        );
                                        if outline_glyph.draw(draw_settings, &mut pen).is_ok() {
                                            if let Some(path) = pb.finish() {
                                                // Glyph position: translate by glyph.x, glyph.y
                                                // Note font coordinate baseline y=0, outline y points UP,
                                                // so flip outline y in glyph local transform:
                                                let glyph_tf = Transform2D::new(
                                                    1.0,
                                                    0.0,
                                                    0.0,
                                                    -1.0,
                                                    glyph.x,
                                                    glyph.y,
                                                );
                                                let combined = glyph_tf
                                                    .multiply(&text_run.transform)
                                                    .multiply(&page_transform)
                                                    .to_tiny_skia();

                                                pixmap.fill_path(
                                                    &path,
                                                    &paint,
                                                    tiny_skia::FillRule::Winding,
                                                    combined,
                                                    active_mask,
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let duration_ms = start_time.elapsed().as_secs_f64() * 1000.0;
        let unsupported_operators = page
            .unsupported_operators
            .iter()
            .map(|op| op.operator.clone())
            .collect();

        Some(RenderedPage {
            page_index: page.page_index,
            width: target_width,
            height: target_height,
            rgba_pixels: pixmap.take(),
            duration_ms,
            unsupported_operators,
        })
    }

    fn build_path(pb: &mut tiny_skia::PathBuilder, verbs: &[PathVerb]) {
        for verb in verbs {
            match verb {
                PathVerb::MoveTo(x, y) => pb.move_to(*x, *y),
                PathVerb::LineTo(x, y) => pb.line_to(*x, *y),
                PathVerb::QuadTo(x1, y1, x, y) => pb.quad_to(*x1, *y1, *x, *y),
                PathVerb::CubicTo(x1, y1, x2, y2, x3, y3) => {
                    pb.cubic_to(*x1, *y1, *x2, *y2, *x3, *y3);
                }
                PathVerb::Close => pb.close(),
            }
        }
    }

    fn load_system_fallback_font() -> Option<Vec<u8>> {
        // Try known relative paths in repo
        let candidate_paths = [
            "public/fonts/NotoSansSC-Regular.ttf",
            "../../public/fonts/NotoSansSC-Regular.ttf",
            "apps/smartpdf-web/public/fonts/NotoSansSC-Regular.ttf",
            "../apps/smartpdf-web/public/fonts/NotoSansSC-Regular.ttf",
            "/System/Library/Fonts/Helvetica.ttc",
            "/System/Library/Fonts/SFNS.ttf",
        ];

        for path in &candidate_paths {
            if let Ok(bytes) = std::fs::read(path) {
                return Some(bytes);
            }
        }
        None
    }
}
