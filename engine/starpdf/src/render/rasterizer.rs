use std::time::Instant;

use read_fonts::FontRef;
use skrifa::instance::Size;
use skrifa::outline::OutlinePen;
use skrifa::MetadataProvider;

use crate::render::ir::{
    LineCap, LineJoin, PathVerb, RenderItem, RenderPage, ShadingPattern, Transform2D,
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

#[derive(Clone)]
pub(crate) struct FallbackFontCache {
    arabic: Option<Vec<u8>>,
    hebrew: Option<Vec<u8>>,
    devanagari: Option<Vec<u8>>,
    cjk: Option<Vec<u8>>,
    latin: Option<Vec<u8>>,
}

impl FallbackFontCache {
    pub(crate) fn new() -> Self {
        Self {
            arabic: Self::load_font("NotoSansArabic-Regular.ttf"),
            hebrew: Self::load_font("NotoSansHebrew-Regular.ttf"),
            devanagari: Self::load_font("NotoSansDevanagari-Regular.ttf"),
            cjk: Self::load_font("NotoSansSC-Regular.ttf"),
            latin: Self::load_latin(),
        }
    }

    fn load_font(filename: &str) -> Option<Vec<u8>> {
        let prefixes = [
            "public/fonts/",
            "../../public/fonts/",
            "apps/smartpdf-web/public/fonts/",
            "../apps/smartpdf-web/public/fonts/",
        ];
        for prefix in &prefixes {
            let p = format!("{prefix}{filename}");
            if let Ok(bytes) = std::fs::read(&p) {
                return Some(bytes);
            }
        }
        None
    }

    fn load_latin() -> Option<Vec<u8>> {
        if let Some(bytes) = Self::load_font("NotoSansSC-Regular.ttf") {
            return Some(bytes);
        }
        let system_paths = [
            "/System/Library/Fonts/Helvetica.ttc",
            "/System/Library/Fonts/SFNS.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        ];
        for path in &system_paths {
            if let Ok(bytes) = std::fs::read(path) {
                return Some(bytes);
            }
        }
        None
    }

    pub(crate) fn font_for_char(&self, ch: char) -> Option<&[u8]> {
        let cp = ch as u32;
        match cp {
            0x0600..=0x06FF
            | 0x0750..=0x077F
            | 0x08A0..=0x08FF
            | 0xFB50..=0xFDFF
            | 0xFE70..=0xFEFF => self.arabic.as_deref().or(self.latin.as_deref()),
            0x0590..=0x05FF => self.hebrew.as_deref().or(self.latin.as_deref()),
            0x0900..=0x097F => self.devanagari.as_deref().or(self.latin.as_deref()),
            0x2E80..=0x2FD5
            | 0x3000..=0x303F
            | 0x3040..=0x309F
            | 0x30A0..=0x30FF
            | 0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xF900..=0xFAFF => self.cjk.as_deref().or(self.latin.as_deref()),
            _ => self.latin.as_deref(),
        }
    }
}

pub struct NativeRasterizer;

impl NativeRasterizer {
    /// Renders a compiled `RenderPage` into an RGBA pixmap at the given scale factor with white background.
    pub fn render(page: &RenderPage, scale: f32) -> Option<RenderedPage> {
        Self::render_with_background(page, scale, tiny_skia::Color::WHITE)
    }

    /// Renders a compiled `RenderPage` into an RGBA pixmap with a specified background color.
    pub fn render_with_background(
        page: &RenderPage,
        scale: f32,
        background: tiny_skia::Color,
    ) -> Option<RenderedPage> {
        let start_time = Instant::now();
        let scale = scale.max(0.1);

        // Normalize rotation to 0, 90, 180, 270
        let norm_rotate = ((page.rotate % 360) + 360) % 360;
        let (target_width, target_height, page_transform) = match norm_rotate {
            90 => {
                let tw = ((page.height * scale).round() as u32).max(1);
                let th = ((page.width * scale).round() as u32).max(1);
                let tf = Transform2D::new(
                    0.0,
                    scale,
                    scale,
                    0.0,
                    -page.crop_box[1] * scale,
                    -page.crop_box[0] * scale,
                );
                (tw, th, tf)
            }
            180 => {
                let tw = ((page.width * scale).round() as u32).max(1);
                let th = ((page.height * scale).round() as u32).max(1);
                let tf = Transform2D::new(
                    -scale,
                    0.0,
                    0.0,
                    scale,
                    page.crop_box[2] * scale,
                    -page.crop_box[1] * scale,
                );
                (tw, th, tf)
            }
            270 => {
                let tw = ((page.height * scale).round() as u32).max(1);
                let th = ((page.width * scale).round() as u32).max(1);
                let tf = Transform2D::new(
                    0.0,
                    -scale,
                    -scale,
                    0.0,
                    page.crop_box[3] * scale,
                    page.crop_box[2] * scale,
                );
                (tw, th, tf)
            }
            _ => {
                let tw = ((page.width * scale).round() as u32).max(1);
                let th = ((page.height * scale).round() as u32).max(1);
                let tf = Transform2D::new(
                    scale,
                    0.0,
                    0.0,
                    -scale,
                    -page.crop_box[0] * scale,
                    page.crop_box[3] * scale,
                );
                (tw, th, tf)
            }
        };

        let mut pixmap = tiny_skia::Pixmap::new(target_width, target_height)?;
        pixmap.fill(background);

        let mut mask_stack: Vec<tiny_skia::Mask> = Vec::new();
        let fallback_fonts = FallbackFontCache::new();

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
                            new_mask.fill_path(&path, clip.rule.to_tiny_skia(), true, combined_tf);
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
                        let combined_tf = render_path
                            .transform
                            .multiply(&page_transform)
                            .to_tiny_skia();
                        let active_mask = mask_stack.last();

                        if let Some(fill) = &render_path.fill {
                            let mut paint = tiny_skia::Paint::default();
                            let [r, g, b, a] = fill.color.to_u8_array();
                            let final_a = (a as f32 * render_path.fill_alpha).round() as u8;
                            paint.set_color_rgba8(r, g, b, final_a);
                            paint.anti_alias = true;
                            paint.blend_mode = render_path.blend_mode.to_tiny_skia();

                            pixmap.fill_path(
                                &path,
                                &paint,
                                fill.rule.to_tiny_skia(),
                                combined_tf,
                                active_mask,
                            );
                        }

                        if let Some(stroke) = &render_path.stroke {
                            let mut paint = tiny_skia::Paint::default();
                            let [r, g, b, a] = stroke.color.to_u8_array();
                            let final_a = (a as f32 * render_path.stroke_alpha).round() as u8;
                            paint.set_color_rgba8(r, g, b, final_a);
                            paint.anti_alias = true;
                            paint.blend_mode = render_path.blend_mode.to_tiny_skia();

                            let mut skia_stroke = tiny_skia::Stroke {
                                width: stroke.width,
                                miter_limit: stroke.miter_limit,
                                line_cap: match stroke.cap {
                                    LineCap::Butt => tiny_skia::LineCap::Butt,
                                    LineCap::Round => tiny_skia::LineCap::Round,
                                    LineCap::Square => tiny_skia::LineCap::Square,
                                },
                                line_join: match stroke.join {
                                    LineJoin::Miter => tiny_skia::LineJoin::Miter,
                                    LineJoin::Round => tiny_skia::LineJoin::Round,
                                    LineJoin::Bevel => tiny_skia::LineJoin::Bevel,
                                },
                                dash: None,
                            };
                            if let Some((dashes, phase)) = &stroke.dash_pattern {
                                if let Some(dash) =
                                    tiny_skia::StrokeDash::new(dashes.clone(), *phase)
                                {
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
                            blend_mode: render_img.blend_mode.to_tiny_skia(),
                            quality: tiny_skia::FilterQuality::Bilinear,
                        };

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
                RenderItem::Shading(render_shading) => {
                    let active_mask = mask_stack.last();
                    let bbox = render_shading
                        .bbox
                        .unwrap_or([0.0, 0.0, page.width, page.height]);

                    let mut pb = tiny_skia::PathBuilder::new();
                    pb.move_to(bbox[0], bbox[1]);
                    pb.line_to(bbox[2], bbox[1]);
                    pb.line_to(bbox[2], bbox[3]);
                    pb.line_to(bbox[0], bbox[3]);
                    pb.close();

                    if let Some(path) = pb.finish() {
                        let combined_tf = render_shading.transform.multiply(&page_transform);

                        match &render_shading.pattern {
                            ShadingPattern::Axial {
                                coords,
                                start_color,
                                end_color,
                                ..
                            } => {
                                let (x0, y0) = combined_tf.transform_point(coords[0], coords[1]);
                                let (x1, y1) = combined_tf.transform_point(coords[2], coords[3]);

                                let [r0, g0, b0, a0] = start_color.to_u8_array();
                                let [r1, g1, b1, a1] = end_color.to_u8_array();

                                let a0_f = (a0 as f32 * render_shading.alpha).round() as u8;
                                let a1_f = (a1 as f32 * render_shading.alpha).round() as u8;

                                let stops = vec![
                                    tiny_skia::GradientStop::new(
                                        0.0,
                                        tiny_skia::Color::from_rgba8(r0, g0, b0, a0_f),
                                    ),
                                    tiny_skia::GradientStop::new(
                                        1.0,
                                        tiny_skia::Color::from_rgba8(r1, g1, b1, a1_f),
                                    ),
                                ];

                                if let Some(shader) = tiny_skia::LinearGradient::new(
                                    tiny_skia::Point::from_xy(x0, y0),
                                    tiny_skia::Point::from_xy(x1, y1),
                                    stops,
                                    tiny_skia::SpreadMode::Pad,
                                    tiny_skia::Transform::identity(),
                                ) {
                                    let mut paint = tiny_skia::Paint::default();
                                    paint.shader = shader;
                                    paint.anti_alias = true;
                                    paint.blend_mode = render_shading.blend_mode.to_tiny_skia();

                                    pixmap.fill_path(
                                        &path,
                                        &paint,
                                        tiny_skia::FillRule::Winding,
                                        combined_tf.to_tiny_skia(),
                                        active_mask,
                                    );
                                }
                            }
                            ShadingPattern::Radial {
                                coords,
                                start_color,
                                end_color,
                                ..
                            } => {
                                let (x1, y1) = combined_tf.transform_point(coords[3], coords[4]);
                                let r1 = coords[5] * page_transform.a.abs();

                                let [r0, g0, b0, a0] = start_color.to_u8_array();
                                let [r1_c, g1_c, b1_c, a1] = end_color.to_u8_array();

                                let a0_f = (a0 as f32 * render_shading.alpha).round() as u8;
                                let a1_f = (a1 as f32 * render_shading.alpha).round() as u8;

                                let stops = vec![
                                    tiny_skia::GradientStop::new(
                                        0.0,
                                        tiny_skia::Color::from_rgba8(r0, g0, b0, a0_f),
                                    ),
                                    tiny_skia::GradientStop::new(
                                        1.0,
                                        tiny_skia::Color::from_rgba8(r1_c, g1_c, b1_c, a1_f),
                                    ),
                                ];

                                if let Some(shader) = tiny_skia::RadialGradient::new(
                                    tiny_skia::Point::from_xy(x1, y1),
                                    tiny_skia::Point::from_xy(x1, y1),
                                    r1.max(1.0),
                                    stops,
                                    tiny_skia::SpreadMode::Pad,
                                    tiny_skia::Transform::identity(),
                                ) {
                                    let mut paint = tiny_skia::Paint::default();
                                    paint.shader = shader;
                                    paint.anti_alias = true;
                                    paint.blend_mode = render_shading.blend_mode.to_tiny_skia();

                                    pixmap.fill_path(
                                        &path,
                                        &paint,
                                        tiny_skia::FillRule::Winding,
                                        combined_tf.to_tiny_skia(),
                                        active_mask,
                                    );
                                }
                            }
                        }
                    }
                }
                RenderItem::TextRun(text_run) => {
                    // Mode 3 is invisible text (used for search/selection/OCR)
                    if text_run.render_mode == 3 {
                        continue;
                    }

                    let mut fill_paint = tiny_skia::Paint::default();
                    let fcol = text_run
                        .fill_color
                        .unwrap_or(crate::render::ir::ColorRgba::BLACK);
                    let [fr, fg, fb, fa] = fcol.to_u8_array();
                    fill_paint.set_color_rgba8(
                        fr,
                        fg,
                        fb,
                        (fa as f32 * text_run.fill_alpha).round() as u8,
                    );
                    fill_paint.anti_alias = true;

                    let mut stroke_paint = tiny_skia::Paint::default();
                    let scol = text_run
                        .stroke_color
                        .unwrap_or(crate::render::ir::ColorRgba::BLACK);
                    let [sr, sg, sb, sa] = scol.to_u8_array();
                    stroke_paint.set_color_rgba8(
                        sr,
                        sg,
                        sb,
                        (sa as f32 * text_run.stroke_alpha).round() as u8,
                    );
                    stroke_paint.anti_alias = true;

                    for glyph in &text_run.glyphs {
                        let ch = glyph.text.chars().next().unwrap_or(' ');
                        // Determine font bytes to use
                        let candidate_bytes = text_run
                            .font_data
                            .as_deref()
                            .or_else(|| fallback_fonts.font_for_char(ch));

                        if let Some(bytes) = candidate_bytes {
                            if let Ok(font_ref) = FontRef::from_index(bytes, 0) {
                                let outlines = font_ref.outline_glyphs();
                                let charmap = font_ref.charmap();

                                let gid = glyph
                                    .glyph_id
                                    .map(|g| skrifa::GlyphId::new(g as u32))
                                    .or_else(|| {
                                        char::from_u32(glyph.codepoint).and_then(|c| charmap.map(c))
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
                                                let glyph_tf = Transform2D::new(
                                                    1.0, 0.0, 0.0, 1.0, glyph.x, glyph.y,
                                                );
                                                let combined = glyph_tf
                                                    .multiply(&text_run.transform)
                                                    .multiply(&page_transform)
                                                    .to_tiny_skia();

                                                let should_fill = text_run.render_mode == 0
                                                    || text_run.render_mode == 2
                                                    || text_run.render_mode == 4
                                                    || text_run.render_mode == 6;

                                                let should_stroke = text_run.render_mode == 1
                                                    || text_run.render_mode == 2
                                                    || text_run.render_mode == 5
                                                    || text_run.render_mode == 6;

                                                let should_clip = text_run.render_mode >= 4
                                                    && text_run.render_mode <= 7;

                                                if should_fill {
                                                    pixmap.fill_path(
                                                        &path,
                                                        &fill_paint,
                                                        tiny_skia::FillRule::Winding,
                                                        combined,
                                                        mask_stack.last(),
                                                    );
                                                }
                                                if should_stroke {
                                                    let sk_stroke = tiny_skia::Stroke {
                                                        width: (text_run.font_size * 0.05).max(0.5),
                                                        ..Default::default()
                                                    };
                                                    pixmap.stroke_path(
                                                        &path,
                                                        &stroke_paint,
                                                        &sk_stroke,
                                                        combined,
                                                        mask_stack.last(),
                                                    );
                                                }
                                                if should_clip {
                                                    if let Some(mask) = mask_stack.last_mut() {
                                                        mask.fill_path(
                                                            &path,
                                                            tiny_skia::FillRule::Winding,
                                                            true,
                                                            combined,
                                                        );
                                                    } else if let Some(mut new_mask) =
                                                        tiny_skia::Mask::new(
                                                            target_width,
                                                            target_height,
                                                        )
                                                    {
                                                        new_mask.fill_path(
                                                            &path,
                                                            tiny_skia::FillRule::Winding,
                                                            true,
                                                            combined,
                                                        );
                                                        mask_stack.push(new_mask);
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
}
