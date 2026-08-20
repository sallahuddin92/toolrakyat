use crate::syntax::object::PdfObject;
use std::collections::BTreeMap;

/// Resolves standard font aliases and provides width estimations for visual layout.
pub struct FontMetricsHelper;

impl FontMetricsHelper {
    /// Normalizes standard font names and aliases.
    pub fn normalize_font_name(font_name: &str) -> &'static str {
        let clean = font_name.trim_start_matches('/');
        match clean {
            "Helv" | "Helvetica" | "Arial" => "Helvetica",
            "HeBo" | "Helvetica-Bold" | "Arial-Bold" => "Helvetica-Bold",
            "HeOb" | "Helvetica-Oblique" | "Arial-Italic" => "Helvetica-Oblique",
            "TiRo" | "Times-Roman" | "Times" | "TimesNewRoman" => "Times-Roman",
            "TiBo" | "Times-Bold" | "TimesNewRoman-Bold" => "Times-Bold",
            "Cour" | "Courier" | "CourierNew" => "Courier",
            "CoBo" | "Courier-Bold" | "CourierNew-Bold" => "Courier-Bold",
            "ZaDb" | "ZapfDingbats" => "ZapfDingbats",
            "Symb" | "Symbol" => "Symbol",
            _ => "Helvetica", // Safe standard fallback
        }
    }

    /// Estimates text advance width based on standard font metrics.
    pub fn estimate_text_width(text: &str, font_name: &str, font_size: f64) -> f64 {
        let normalized = Self::normalize_font_name(font_name);
        let char_factor = match normalized {
            "Courier" | "Courier-Bold" => 0.60,
            "Times-Roman" | "Times-Bold" => 0.50,
            _ => 0.53, // Helvetica standard average
        };

        text.chars().count() as f64 * font_size * char_factor
    }

    /// Generates a standard font resource dictionary for an appearance Form XObject.
    pub fn build_font_resource(font_name: &str) -> BTreeMap<String, PdfObject> {
        let normalized = Self::normalize_font_name(font_name);
        let clean_key = font_name.trim_start_matches('/').to_string();

        let mut font_dict = BTreeMap::new();
        font_dict.insert("Type".to_string(), PdfObject::Name("Font".to_string()));
        font_dict.insert("Subtype".to_string(), PdfObject::Name("Type1".to_string()));
        font_dict.insert(
            "BaseFont".to_string(),
            PdfObject::Name(normalized.to_string()),
        );
        font_dict.insert(
            "Encoding".to_string(),
            PdfObject::Name("WinAnsiEncoding".to_string()),
        );

        let mut fonts_map = BTreeMap::new();
        fonts_map.insert(clean_key, PdfObject::Dictionary(font_dict));

        let mut res_map = BTreeMap::new();
        res_map.insert("Font".to_string(), PdfObject::Dictionary(fonts_map));
        res_map
    }
}
