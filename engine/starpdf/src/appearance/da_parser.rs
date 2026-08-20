use crate::appearance::color::PdfColor;
use crate::error::{PdfError, PdfResult};

/// Parsed PDF Default Appearance (`/DA`) string representation.
#[derive(Debug, Clone, PartialEq)]
pub struct DefaultAppearance {
    pub font_name: String,
    pub font_size: f64,
    pub color: PdfColor,
}

impl Default for DefaultAppearance {
    fn default() -> Self {
        Self {
            font_name: "Helv".to_string(),
            font_size: 12.0,
            color: PdfColor::black(),
        }
    }
}

impl DefaultAppearance {
    pub fn new(font_name: &str, font_size: f64, color: PdfColor) -> PdfResult<Self> {
        if !font_size.is_finite() || !(0.0..=1000.0).contains(&font_size) {
            return Err(PdfError::InvalidOperation(
                "Invalid font size in DefaultAppearance".into(),
            ));
        }
        let clean_name = font_name.trim_start_matches('/').to_string();
        Ok(Self {
            font_name: clean_name,
            font_size,
            color,
        })
    }

    /// Parses a `/DA` string (e.g. `"/Helv 12 Tf 0 g"` or `"0 0 0 rg /Helvetica 14 Tf"`).
    pub fn parse(da_str: &str) -> PdfResult<Self> {
        let tokens = da_str.split_whitespace();
        let mut font_name = "Helv".to_string();
        let mut font_size = 12.0;
        let mut color = PdfColor::black();

        let mut num_stack: Vec<f64> = Vec::new();
        let mut name_stack: Vec<String> = Vec::new();

        for tok in tokens {
            if let Ok(val) = tok.parse::<f64>() {
                if val.is_finite() {
                    num_stack.push(val);
                }
            } else if tok.starts_with('/') {
                name_stack.push(tok.trim_start_matches('/').to_string());
            } else {
                match tok {
                    "Tf" => {
                        if let Some(sz) = num_stack.pop() {
                            if sz.is_finite() && (0.0..=1000.0).contains(&sz) {
                                font_size = sz;
                            }
                        }
                        if let Some(nm) = name_stack.pop() {
                            font_name = nm;
                        }
                    }
                    "g" | "G" => {
                        if let Some(g) = num_stack.pop() {
                            if let Ok(c) = PdfColor::grayscale(g) {
                                color = c;
                            }
                        }
                    }
                    "rg" | "RG" => {
                        if num_stack.len() >= 3 {
                            let b = num_stack.pop().unwrap_or(0.0);
                            let g = num_stack.pop().unwrap_or(0.0);
                            let r = num_stack.pop().unwrap_or(0.0);
                            if let Ok(c) = PdfColor::rgb(r, g, b) {
                                color = c;
                            }
                        }
                    }
                    "k" | "K" => {
                        if num_stack.len() >= 4 {
                            let k = num_stack.pop().unwrap_or(0.0);
                            let y = num_stack.pop().unwrap_or(0.0);
                            let m = num_stack.pop().unwrap_or(0.0);
                            let c = num_stack.pop().unwrap_or(0.0);
                            if c.is_finite() && m.is_finite() && y.is_finite() && k.is_finite() {
                                color = PdfColor::Cmyk(
                                    c.clamp(0.0, 1.0),
                                    m.clamp(0.0, 1.0),
                                    y.clamp(0.0, 1.0),
                                    k.clamp(0.0, 1.0),
                                );
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(Self {
            font_name,
            font_size,
            color,
        })
    }

    /// Formats the appearance into standard DA string format.
    pub fn to_da_string(&self) -> String {
        format!(
            "/{} {:.2} Tf {}",
            self.font_name,
            self.font_size,
            self.color.to_fill_ops()
        )
    }
}
