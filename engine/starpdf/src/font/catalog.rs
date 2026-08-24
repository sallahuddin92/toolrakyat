use crate::font::font::{FontFamily, FontStyle};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontCatalogEntry {
    pub font_id: &'static str,
    pub display_name: &'static str,
    pub family: FontFamily,
    pub weight: u16, // 400 = Regular, 700 = Bold
    pub is_italic: bool,
    pub is_monospace: bool,
    pub asset_filename: &'static str,
    pub license: &'static str,
    pub version: &'static str,
    pub supported_unicode_pages: &'static [u16],
}

impl FontCatalogEntry {
    pub fn matches_style(&self, target_style: &FontStyle) -> u32 {
        let mut score = 0u32;
        if self.family == target_style.family {
            score += 100;
        }
        let is_bold = self.weight >= 600;
        if is_bold == target_style.is_bold {
            score += 50;
        }
        if self.is_italic == target_style.is_italic {
            score += 40;
        }
        if self.is_monospace == target_style.is_monospace {
            score += 30;
        }
        score
    }

    pub fn covers_char(&self, ch: char) -> bool {
        let page = (ch as u32 / 256) as u16;
        self.supported_unicode_pages.contains(&page)
    }

    pub fn covers_text(&self, text: &str) -> bool {
        text.chars().all(|ch| self.covers_char(ch))
    }
}

pub static BUILTIN_FONT_CATALOG: &[FontCatalogEntry] = &[
    // 1. Standard 14 Sans (Helvetica)
    FontCatalogEntry {
        font_id: "helvetica-regular",
        display_name: "Helvetica",
        family: FontFamily::SansSerif,
        weight: 400,
        is_italic: false,
        is_monospace: false,
        asset_filename: "Helvetica.afm",
        license: "Standard 14 PDF",
        version: "1.0",
        supported_unicode_pages: &[0, 1, 2, 32],
    },
    FontCatalogEntry {
        font_id: "helvetica-bold",
        display_name: "Helvetica-Bold",
        family: FontFamily::SansSerif,
        weight: 700,
        is_italic: false,
        is_monospace: false,
        asset_filename: "Helvetica-Bold.afm",
        license: "Standard 14 PDF",
        version: "1.0",
        supported_unicode_pages: &[0, 1, 2, 32],
    },
    FontCatalogEntry {
        font_id: "helvetica-oblique",
        display_name: "Helvetica-Oblique",
        family: FontFamily::SansSerif,
        weight: 400,
        is_italic: true,
        is_monospace: false,
        asset_filename: "Helvetica-Oblique.afm",
        license: "Standard 14 PDF",
        version: "1.0",
        supported_unicode_pages: &[0, 1, 2, 32],
    },
    FontCatalogEntry {
        font_id: "helvetica-boldoblique",
        display_name: "Helvetica-BoldOblique",
        family: FontFamily::SansSerif,
        weight: 700,
        is_italic: true,
        is_monospace: false,
        asset_filename: "Helvetica-BoldOblique.afm",
        license: "Standard 14 PDF",
        version: "1.0",
        supported_unicode_pages: &[0, 1, 2, 32],
    },
    // 2. Standard 14 Serif (Times)
    FontCatalogEntry {
        font_id: "times-roman",
        display_name: "Times-Roman",
        family: FontFamily::Serif,
        weight: 400,
        is_italic: false,
        is_monospace: false,
        asset_filename: "Times-Roman.afm",
        license: "Standard 14 PDF",
        version: "1.0",
        supported_unicode_pages: &[0, 1, 2, 32],
    },
    FontCatalogEntry {
        font_id: "times-bold",
        display_name: "Times-Bold",
        family: FontFamily::Serif,
        weight: 700,
        is_italic: false,
        is_monospace: false,
        asset_filename: "Times-Bold.afm",
        license: "Standard 14 PDF",
        version: "1.0",
        supported_unicode_pages: &[0, 1, 2, 32],
    },
    FontCatalogEntry {
        font_id: "times-italic",
        display_name: "Times-Italic",
        family: FontFamily::Serif,
        weight: 400,
        is_italic: true,
        is_monospace: false,
        asset_filename: "Times-Italic.afm",
        license: "Standard 14 PDF",
        version: "1.0",
        supported_unicode_pages: &[0, 1, 2, 32],
    },
    FontCatalogEntry {
        font_id: "times-bolditalic",
        display_name: "Times-BoldItalic",
        family: FontFamily::Serif,
        weight: 700,
        is_italic: true,
        is_monospace: false,
        asset_filename: "Times-BoldItalic.afm",
        license: "Standard 14 PDF",
        version: "1.0",
        supported_unicode_pages: &[0, 1, 2, 32],
    },
    // 3. Standard 14 Monospace (Courier)
    FontCatalogEntry {
        font_id: "courier-regular",
        display_name: "Courier",
        family: FontFamily::Monospace,
        weight: 400,
        is_italic: false,
        is_monospace: true,
        asset_filename: "Courier.afm",
        license: "Standard 14 PDF",
        version: "1.0",
        supported_unicode_pages: &[0, 1, 2, 32],
    },
    FontCatalogEntry {
        font_id: "courier-bold",
        display_name: "Courier-Bold",
        family: FontFamily::Monospace,
        weight: 700,
        is_italic: false,
        is_monospace: true,
        asset_filename: "Courier-Bold.afm",
        license: "Standard 14 PDF",
        version: "1.0",
        supported_unicode_pages: &[0, 1, 2, 32],
    },
    // 4. Multilingual Fallback Assets (Noto / Liberation / OpenType)
    FontCatalogEntry {
        font_id: "noto-sans-arabic",
        display_name: "Noto Sans Arabic",
        family: FontFamily::SansSerif,
        weight: 400,
        is_italic: false,
        is_monospace: false,
        asset_filename: "NotoSansArabic-Regular.ttf",
        license: "SIL OFL 1.1",
        version: "2.004",
        // Unicode pages: 0, 6 (Arabic 0600-06FF), 7 (Arabic Supp 0750-077F), 8 (Arabic Ext-A 08A0-08FF), 32, 251 (Arabic Presentation Forms-A FB50-FDFF), 254 (Arabic Presentation Forms-B FE70-FEFF)
        supported_unicode_pages: &[0, 6, 7, 8, 32, 251, 254],
    },
    FontCatalogEntry {
        font_id: "noto-sans-hebrew",
        display_name: "Noto Sans Hebrew",
        family: FontFamily::SansSerif,
        weight: 400,
        is_italic: false,
        is_monospace: false,
        asset_filename: "NotoSansHebrew-Regular.ttf",
        license: "SIL OFL 1.1",
        version: "2.003",
        // Unicode pages: 0, 5 (Hebrew 0590-05FF), 32, 251 (Alphabetic Presentation Forms FB1D-FB4F)
        supported_unicode_pages: &[0, 5, 32, 251],
    },
    FontCatalogEntry {
        font_id: "noto-sans-devanagari",
        display_name: "Noto Sans Devanagari",
        family: FontFamily::SansSerif,
        weight: 400,
        is_italic: false,
        is_monospace: false,
        asset_filename: "NotoSansDevanagari-Regular.ttf",
        license: "SIL OFL 1.1",
        version: "2.004",
        // Unicode pages: 0, 9 (Devanagari 0900-097F), 32, 168 (Devanagari Extended A8E0-A8FF)
        supported_unicode_pages: &[0, 9, 32, 168],
    },
    FontCatalogEntry {
        font_id: "noto-sans-cjk-sc",
        display_name: "Noto Sans CJK SC",
        family: FontFamily::SansSerif,
        weight: 400,
        is_italic: false,
        is_monospace: false,
        asset_filename: "NotoSansSC-Regular.otf",
        license: "SIL OFL 1.1",
        version: "2.004",
        // Unicode pages: 0, 32, 46..159 (CJK Unified Ideographs 4E00-9FFF), 19..20 (Hiragana/Katakana), 172..215 (Hangul Syllables)
        supported_unicode_pages: &[
            0, 1, 2, 19, 20, 32, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61,
            62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83,
            84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103,
            104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120,
            121, 122, 123, 124, 125, 126, 127, 128, 129, 130, 131, 132, 133, 134, 135, 136, 137,
            138, 139, 140, 141, 142, 143, 144, 145, 146, 147, 148, 149, 150, 151, 152, 153, 154,
            155, 156, 157, 158, 159,
        ],
    },
];

pub fn find_candidate_fallbacks(
    text: &str,
    target_style: &FontStyle,
) -> Vec<&'static FontCatalogEntry> {
    let mut candidates: Vec<(&'static FontCatalogEntry, u32)> = BUILTIN_FONT_CATALOG
        .iter()
        .filter(|entry| entry.covers_text(text))
        .map(|entry| (entry, entry.matches_style(target_style)))
        .collect();

    // Sort descending by style match score
    candidates.sort_by(|a, b| b.1.cmp(&a.1));
    candidates.into_iter().map(|(entry, _)| entry).collect()
}
