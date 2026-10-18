use crate::execution::{RenderNodeId, StarNodeId};

/// Affine 2D transformation matrix:
/// [ a  b  0 ]
/// [ c  d  0 ]
/// [ e  f  1 ]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform2D {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub e: f32,
    pub f: f32,
}

impl Default for Transform2D {
    fn default() -> Self {
        Self::identity()
    }
}

impl Transform2D {
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

    pub const fn new(a: f32, b: f32, c: f32, d: f32, e: f32, f: f32) -> Self {
        Self { a, b, c, d, e, f }
    }

    pub fn translation(tx: f32, ty: f32) -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: tx,
            f: ty,
        }
    }

    pub fn scale(sx: f32, sy: f32) -> Self {
        Self {
            a: sx,
            b: 0.0,
            c: 0.0,
            d: 1.0 * sy,
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

    pub fn transform_point(&self, x: f32, y: f32) -> (f32, f32) {
        (
            self.a * x + self.c * y + self.e,
            self.b * x + self.d * y + self.f,
        )
    }

    pub fn to_tiny_skia(self) -> tiny_skia::Transform {
        tiny_skia::Transform::from_row(self.a, self.b, self.c, self.d, self.e, self.f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorRgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl ColorRgba {
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    pub const TRANSPARENT: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };

    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn from_gray(gray: f32) -> Self {
        Self {
            r: gray.clamp(0.0, 1.0),
            g: gray.clamp(0.0, 1.0),
            b: gray.clamp(0.0, 1.0),
            a: 1.0,
        }
    }

    pub fn from_rgb(r: f32, g: f32, b: f32) -> Self {
        Self {
            r: r.clamp(0.0, 1.0),
            g: g.clamp(0.0, 1.0),
            b: b.clamp(0.0, 1.0),
            a: 1.0,
        }
    }

    pub fn from_cmyk(c: f32, m: f32, y: f32, k: f32) -> Self {
        let c = c.clamp(0.0, 1.0);
        let m = m.clamp(0.0, 1.0);
        let y = y.clamp(0.0, 1.0);
        let k = k.clamp(0.0, 1.0);
        let r = (1.0 - c) * (1.0 - k);
        let g = (1.0 - m) * (1.0 - k);
        let b = (1.0 - y) * (1.0 - k);
        Self {
            r: r.clamp(0.0, 1.0),
            g: g.clamp(0.0, 1.0),
            b: b.clamp(0.0, 1.0),
            a: 1.0,
        }
    }

    pub fn to_u8_array(&self) -> [u8; 4] {
        [
            (self.r * 255.0).clamp(0.0, 255.0) as u8,
            (self.g * 255.0).clamp(0.0, 255.0) as u8,
            (self.b * 255.0).clamp(0.0, 255.0) as u8,
            (self.a * 255.0).clamp(0.0, 255.0) as u8,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillRule {
    NonZero,
    EvenOdd,
}

impl FillRule {
    pub fn to_tiny_skia(self) -> tiny_skia::FillRule {
        match self {
            Self::NonZero => tiny_skia::FillRule::Winding,
            Self::EvenOdd => tiny_skia::FillRule::EvenOdd,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineCap {
    Butt,
    Round,
    Square,
}

impl LineCap {
    pub fn to_tiny_skia(self) -> tiny_skia::LineCap {
        match self {
            Self::Butt => tiny_skia::LineCap::Butt,
            Self::Round => tiny_skia::LineCap::Round,
            Self::Square => tiny_skia::LineCap::Square,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineJoin {
    Miter,
    Round,
    Bevel,
}

impl LineJoin {
    pub fn to_tiny_skia(self) -> tiny_skia::LineJoin {
        match self {
            Self::Miter => tiny_skia::LineJoin::Miter,
            Self::Round => tiny_skia::LineJoin::Round,
            Self::Bevel => tiny_skia::LineJoin::Bevel,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StrokeStyle {
    pub color: ColorRgba,
    pub width: f32,
    pub cap: LineCap,
    pub join: LineJoin,
    pub miter_limit: f32,
    pub dash_pattern: Option<(Vec<f32>, f32)>,
}

impl Default for StrokeStyle {
    fn default() -> Self {
        Self {
            color: ColorRgba::BLACK,
            width: 1.0,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            miter_limit: 10.0,
            dash_pattern: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
}

impl Default for BlendMode {
    fn default() -> Self {
        Self::Normal
    }
}

impl BlendMode {
    pub fn to_tiny_skia(self) -> tiny_skia::BlendMode {
        match self {
            Self::Normal => tiny_skia::BlendMode::SourceOver,
            Self::Multiply => tiny_skia::BlendMode::Multiply,
            Self::Screen => tiny_skia::BlendMode::Screen,
            Self::Overlay => tiny_skia::BlendMode::Overlay,
            Self::Darken => tiny_skia::BlendMode::Darken,
            Self::Lighten => tiny_skia::BlendMode::Lighten,
            Self::ColorDodge => tiny_skia::BlendMode::ColorDodge,
            Self::ColorBurn => tiny_skia::BlendMode::ColorBurn,
            Self::HardLight => tiny_skia::BlendMode::HardLight,
            Self::SoftLight => tiny_skia::BlendMode::SoftLight,
            Self::Difference => tiny_skia::BlendMode::Difference,
            Self::Exclusion => tiny_skia::BlendMode::Exclusion,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FillStyle {
    pub color: ColorRgba,
    pub rule: FillRule,
}

impl Default for FillStyle {
    fn default() -> Self {
        Self {
            color: ColorRgba::BLACK,
            rule: FillRule::NonZero,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PathVerb {
    MoveTo(f32, f32),
    LineTo(f32, f32),
    QuadTo(f32, f32, f32, f32),
    CubicTo(f32, f32, f32, f32, f32, f32),
    Close,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderPath {
    pub verbs: Vec<PathVerb>,
    pub fill: Option<FillStyle>,
    pub stroke: Option<StrokeStyle>,
    pub transform: Transform2D,
    pub stroke_alpha: f32,
    pub fill_alpha: f32,
    pub blend_mode: BlendMode,
    pub star_node_id: Option<StarNodeId>,
    pub render_node_id: Option<RenderNodeId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderGlyph {
    pub codepoint: u32,
    pub glyph_id: Option<u16>,
    pub x: f32,
    pub y: f32,
    pub advance: f32,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderTextRun {
    pub font_name: String,
    pub font_size: f32,
    pub glyphs: Vec<RenderGlyph>,
    pub fill_color: Option<ColorRgba>,
    pub stroke_color: Option<ColorRgba>,
    pub transform: Transform2D,
    pub fill_alpha: f32,
    pub stroke_alpha: f32,
    pub render_mode: u8,
    pub font_data: Option<Vec<u8>>,
    pub is_vertical: bool,
    pub star_node_id: Option<StarNodeId>,
    pub render_node_id: Option<RenderNodeId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderImage {
    pub width: u32,
    pub height: u32,
    pub rgba_data: Vec<u8>,
    pub transform: Transform2D,
    pub opacity: f32,
    pub is_mask: bool,
    pub blend_mode: BlendMode,
    pub star_node_id: Option<StarNodeId>,
    pub render_node_id: Option<RenderNodeId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderClip {
    pub verbs: Vec<PathVerb>,
    pub rule: FillRule,
    pub transform: Transform2D,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RenderItem {
    Path(RenderPath),
    TextRun(RenderTextRun),
    Image(RenderImage),
    PushClip(RenderClip),
    PopClip,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnsupportedOperatorReport {
    pub operator: String,
    pub operands: Vec<String>,
    pub stream_index: usize,
    pub instruction_index: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderPage {
    pub page_index: usize,
    pub width: f32,
    pub height: f32,
    pub media_box: [f32; 4],
    pub crop_box: [f32; 4],
    pub rotate: i32,
    pub items: Vec<RenderItem>,
    pub unsupported_operators: Vec<UnsupportedOperatorReport>,
}

impl RenderPage {
    pub fn has_unsupported_operators(&self) -> bool {
        !self.unsupported_operators.is_empty()
    }
}
