use crate::error::{PdfError, PdfResult};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PdfColor {
    Grayscale(f64),
    Rgb(f64, f64, f64),
    Cmyk(f64, f64, f64, f64),
}

impl Default for PdfColor {
    fn default() -> Self {
        Self::Grayscale(0.0) // Black
    }
}

impl PdfColor {
    pub fn black() -> Self {
        Self::Grayscale(0.0)
    }

    pub fn white() -> Self {
        Self::Grayscale(1.0)
    }

    pub fn rgb(r: f64, g: f64, b: f64) -> PdfResult<Self> {
        if !r.is_finite() || !g.is_finite() || !b.is_finite() {
            return Err(PdfError::InvalidOperation(
                "Non-finite RGB color values".into(),
            ));
        }
        Ok(Self::Rgb(
            r.clamp(0.0, 1.0),
            g.clamp(0.0, 1.0),
            b.clamp(0.0, 1.0),
        ))
    }

    pub fn grayscale(g: f64) -> PdfResult<Self> {
        if !g.is_finite() {
            return Err(PdfError::InvalidOperation(
                "Non-finite Grayscale color value".into(),
            ));
        }
        Ok(Self::Grayscale(g.clamp(0.0, 1.0)))
    }

    pub fn parse_from_slice(arr: &[f64]) -> Option<Self> {
        match arr.len() {
            1 if arr[0].is_finite() => Some(Self::Grayscale(arr[0].clamp(0.0, 1.0))),
            3 if arr.iter().all(|v| v.is_finite()) => Some(Self::Rgb(
                arr[0].clamp(0.0, 1.0),
                arr[1].clamp(0.0, 1.0),
                arr[2].clamp(0.0, 1.0),
            )),
            4 if arr.iter().all(|v| v.is_finite()) => Some(Self::Cmyk(
                arr[0].clamp(0.0, 1.0),
                arr[1].clamp(0.0, 1.0),
                arr[2].clamp(0.0, 1.0),
                arr[3].clamp(0.0, 1.0),
            )),
            _ => None,
        }
    }

    pub fn to_fill_ops(&self) -> String {
        match self {
            Self::Grayscale(g) => format!("{:.3} g", g),
            Self::Rgb(r, g, b) => format!("{:.3} {:.3} {:.3} rg", r, g, b),
            Self::Cmyk(c, m, y, k) => format!("{:.3} {:.3} {:.3} {:.3} k", c, m, y, k),
        }
    }

    pub fn to_stroke_ops(&self) -> String {
        match self {
            Self::Grayscale(g) => format!("{:.3} G", g),
            Self::Rgb(r, g, b) => format!("{:.3} {:.3} {:.3} RG", r, g, b),
            Self::Cmyk(c, m, y, k) => format!("{:.3} {:.3} {:.3} {:.3} K", c, m, y, k),
        }
    }
}
