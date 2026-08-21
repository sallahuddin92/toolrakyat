/// Vector and content graphics types for StarPDF v0.15.

#[derive(Debug, Clone, PartialEq)]
pub enum VectorColor {
    Gray(f64),
    Rgb(f64, f64, f64),
    Cmyk(f64, f64, f64, f64),
}

impl VectorColor {
    pub fn to_rgb(&self) -> (f64, f64, f64) {
        match self {
            Self::Gray(g) => (*g, *g, *g),
            Self::Rgb(r, g, b) => (*r, *g, *b),
            Self::Cmyk(c, m, y, k) => {
                let r = (1.0 - c) * (1.0 - k);
                let g = (1.0 - m) * (1.0 - k);
                let b = (1.0 - y) * (1.0 - k);
                (r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0))
            }
        }
    }

    pub fn to_rgb_array(&self) -> [f64; 3] {
        let (r, g, b) = self.to_rgb();
        [r, g, b]
    }

    pub fn to_hex(&self) -> String {
        let (r, g, b) = self.to_rgb();
        let ir = (r * 255.0).round().clamp(0.0, 255.0) as u8;
        let ig = (g * 255.0).round().clamp(0.0, 255.0) as u8;
        let ib = (b * 255.0).round().clamp(0.0, 255.0) as u8;
        format!("#{ir:02x}{ig:02x}{ib:02x}")
    }

    pub fn from_rgb(r: f64, g: f64, b: f64) -> Self {
        Self::Rgb(r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0))
    }

    pub fn from_hex(hex: &str) -> Option<Self> {
        let s = hex.trim().trim_start_matches('#');
        if s.len() == 6 {
            let r = u8::from_str_radix(&s[0..2], 16).ok()? as f64 / 255.0;
            let g = u8::from_str_radix(&s[2..4], 16).ok()? as f64 / 255.0;
            let b = u8::from_str_radix(&s[4..6], 16).ok()? as f64 / 255.0;
            Some(Self::from_rgb(r, g, b))
        } else if s.len() == 3 {
            let r = u8::from_str_radix(&s[0..1].repeat(2), 16).ok()? as f64 / 255.0;
            let g = u8::from_str_radix(&s[1..2].repeat(2), 16).ok()? as f64 / 255.0;
            let b = u8::from_str_radix(&s[2..3].repeat(2), 16).ok()? as f64 / 255.0;
            Some(Self::from_rgb(r, g, b))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VectorGraphicType {
    Rectangle,
    Line,
    Path,
}

impl VectorGraphicType {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Rectangle => "Rectangle",
            Self::Line => "Line",
            Self::Path => "Path",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum VectorGeometry {
    Rectangle {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    },
    Line {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
    },
    Path {
        points: Vec<(f64, f64)>,
        closed: bool,
    },
}

impl VectorGeometry {
    pub fn compute_local_bounds(&self) -> [f64; 4] {
        match self {
            Self::Rectangle {
                x,
                y,
                width,
                height,
            } => {
                let x2 = x + width;
                let y2 = y + height;
                [x.min(x2), y.min(y2), x.max(x2), y.max(y2)]
            }
            Self::Line { x1, y1, x2, y2 } => [x1.min(*x2), y1.min(*y2), x1.max(*x2), y1.max(*y2)],
            Self::Path { points, .. } => {
                if points.is_empty() {
                    return [0.0, 0.0, 0.0, 0.0];
                }
                let mut min_x = f64::INFINITY;
                let mut min_y = f64::INFINITY;
                let mut max_x = f64::NEG_INFINITY;
                let mut max_y = f64::NEG_INFINITY;
                for &(px, py) in points {
                    if px < min_x {
                        min_x = px;
                    }
                    if py < min_y {
                        min_y = py;
                    }
                    if px > max_x {
                        max_x = px;
                    }
                    if py > max_y {
                        max_y = py;
                    }
                }
                [min_x, min_y, max_x, max_y]
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VectorEditability {
    Editable,
    ReadOnly(String),
    UnsupportedOperator(String),
    ComplexClipping,
    UnsupportedColorSpace(String),
    MalformedPathState(String),
}

impl VectorEditability {
    pub const fn is_editable(&self) -> bool {
        matches!(self, Self::Editable)
    }

    pub const fn code(&self) -> &'static str {
        match self {
            Self::Editable => "EDITABLE_VECTOR_GRAPHIC",
            Self::ReadOnly(_) => "READ_ONLY_VECTOR_GRAPHIC",
            Self::UnsupportedOperator(_) => "UNSUPPORTED_OPERATOR",
            Self::ComplexClipping => "COMPLEX_CLIPPING",
            Self::UnsupportedColorSpace(_) => "UNSUPPORTED_COLOR_SPACE",
            Self::MalformedPathState(_) => "MALFORMED_PATH_STATE",
        }
    }

    pub fn reason(&self) -> Option<String> {
        match self {
            Self::Editable => None,
            Self::ReadOnly(r)
            | Self::UnsupportedOperator(r)
            | Self::UnsupportedColorSpace(r)
            | Self::MalformedPathState(r) => Some(r.clone()),
            Self::ComplexClipping => Some(
                "Complex clipping paths (W/W*) are not supported for vector editing".to_string(),
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VectorGraphicInfo {
    pub graphic_id: String,
    pub page_index: usize,
    pub stream_index: usize,
    pub start_instruction_index: usize,
    pub end_instruction_index: usize,
    pub graphic_type: VectorGraphicType,
    pub bounds: [f64; 4],
    pub local_bounds: [f64; 4],
    pub transform: [f64; 6],
    pub stroke_color: Option<VectorColor>,
    pub fill_color: Option<VectorColor>,
    pub line_width: f64,
    pub is_stroked: bool,
    pub is_filled: bool,
    pub is_shared: bool,
    pub editability: VectorEditability,
    pub geometry: VectorGeometry,
    pub has_isolated_q: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UpdateVectorGraphicSpec {
    pub page_index: usize,
    pub graphic_id: String,
    pub new_geometry: Option<VectorGeometry>,
    pub new_stroke_color: Option<Option<VectorColor>>,
    pub new_fill_color: Option<Option<VectorColor>>,
    pub new_line_width: Option<f64>,
    pub new_is_stroked: Option<bool>,
    pub new_is_filled: Option<bool>,
    pub clone_if_shared: bool,
}

impl Default for UpdateVectorGraphicSpec {
    fn default() -> Self {
        Self {
            page_index: 0,
            graphic_id: String::new(),
            new_geometry: None,
            new_stroke_color: None,
            new_fill_color: None,
            new_line_width: None,
            new_is_stroked: None,
            new_is_filled: None,
            clone_if_shared: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AddVectorGraphicSpec {
    pub page_index: usize,
    pub geometry: VectorGeometry,
    pub stroke_color: Option<VectorColor>,
    pub fill_color: Option<VectorColor>,
    pub line_width: f64,
    pub is_stroked: bool,
    pub is_filled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeleteVectorGraphicSpec {
    pub page_index: usize,
    pub graphic_id: String,
    pub clone_if_shared: bool,
}
