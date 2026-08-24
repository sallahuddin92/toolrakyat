use crate::font::font::{FontFamily, FontStyle};
use read_fonts::{FontRef, TableProvider};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

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
    pub sha256: &'static str,
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

    pub fn covers_char_coarse(&self, ch: char) -> bool {
        let page = (ch as u32 / 256) as u16;
        self.supported_unicode_pages.contains(&page)
    }

    pub fn covers_text_coarse(&self, text: &str) -> bool {
        text.chars().all(|ch| self.covers_char_coarse(ch))
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
        sha256: "builtin",
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
        sha256: "builtin",
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
        sha256: "builtin",
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
        sha256: "builtin",
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
        sha256: "builtin",
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
        sha256: "builtin",
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
        sha256: "builtin",
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
        sha256: "builtin",
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
        sha256: "builtin",
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
        sha256: "builtin",
        supported_unicode_pages: &[0, 1, 2, 32],
    },
    // 4. Multilingual Fallback Assets (Noto / SIL OFL 1.1)
    FontCatalogEntry {
        font_id: "noto-sans-arabic",
        display_name: "Noto Sans Arabic",
        family: FontFamily::SansSerif,
        weight: 400,
        is_italic: false,
        is_monospace: false,
        asset_filename: "NotoSansArabic-Regular.ttf",
        license: "SIL OFL 1.1",
        version: "2.009",
        sha256: "ceea25b464a656dc3b26849bab9356740401af62aedf1bfa8b7f0d9b75925b1b",
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
        version: "3.000",
        sha256: "a7fa16fffb27bedb060a0866267c29e9859aeb9c21cc33f5b3aaf6eb062eca85",
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
        version: "2.002",
        sha256: "385e78e6359a9d88a0f243d53b1209d7548361ba2194e2b9ec779bcaa7e8949d",
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
        asset_filename: "NotoSansSC-Regular.ttf",
        license: "SIL OFL 1.1",
        version: "2.004",
        sha256: "a3041811a78c361b1de50f953c805e0244951c21c5bd412f7232ef0d899af0da",
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
    FontCatalogEntry {
        font_id: "noto-sans-cjk-tc",
        display_name: "Noto Sans CJK TC",
        family: FontFamily::SansSerif,
        weight: 400,
        is_italic: false,
        is_monospace: false,
        asset_filename: "NotoSansTC-Regular.ttf",
        license: "SIL OFL 1.1",
        version: "2.004",
        sha256: "864727d210d54f2537bbe23b3a839436c3992af72de9322af5270897246bd44f",
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
    FontCatalogEntry {
        font_id: "noto-sans-cjk-jp",
        display_name: "Noto Sans CJK JP",
        family: FontFamily::SansSerif,
        weight: 400,
        is_italic: false,
        is_monospace: false,
        asset_filename: "NotoSansJP-Regular.ttf",
        license: "SIL OFL 1.1",
        version: "2.004",
        sha256: "c2f3b4d463500a2ddcd3849cded1fceeb9fd6d1c32e6cbecd568453ba50fc68f",
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
    FontCatalogEntry {
        font_id: "noto-sans-cjk-kr",
        display_name: "Noto Sans CJK KR",
        family: FontFamily::SansSerif,
        weight: 400,
        is_italic: false,
        is_monospace: false,
        asset_filename: "NotoSansKR-Regular.ttf",
        license: "SIL OFL 1.1",
        version: "2.004",
        sha256: "194018e6b2b293a7964f037b25c0249ce1418bc9ab3c971060a03aa57861e252",
        supported_unicode_pages: &[
            0, 1, 2, 172, 173, 174, 175, 176, 177, 178, 179, 180, 181, 182, 183, 184, 185, 186,
            187, 188, 189, 190, 191, 192, 193, 194, 195, 196, 197, 198, 199, 200, 201, 202, 203,
            204, 205, 206, 207, 208, 209, 210, 211, 212, 213, 214, 215,
        ],
    },
];

pub fn find_candidate_fallbacks(
    text: &str,
    target_style: &FontStyle,
) -> Vec<&'static FontCatalogEntry> {
    let preferred_cjk = if text
        .chars()
        .any(|ch| matches!(ch as u32, 0x3040..=0x30ff | 0x31f0..=0x31ff))
    {
        Some("noto-sans-cjk-jp")
    } else if text
        .chars()
        .any(|ch| matches!(ch as u32, 0x1100..=0x11ff | 0x3130..=0x318f | 0xac00..=0xd7af))
    {
        Some("noto-sans-cjk-kr")
    } else if text
        .chars()
        .any(|ch| "體測試繁國語學書車門風馬龍臺灣".contains(ch))
    {
        Some("noto-sans-cjk-tc")
    } else if text.chars().any(|ch| matches!(ch as u32, 0x3400..=0x9fff)) {
        Some("noto-sans-cjk-sc")
    } else {
        None
    };

    let mut candidates: Vec<(&'static FontCatalogEntry, u32, bool)> = BUILTIN_FONT_CATALOG
        .iter()
        .filter(|entry| entry.covers_text_coarse(text))
        .map(|entry| {
            (
                entry,
                entry.matches_style(target_style),
                preferred_cjk == Some(entry.font_id),
            )
        })
        .collect();

    candidates.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| b.1.cmp(&a.1)));
    candidates.into_iter().map(|(entry, _, _)| entry).collect()
}

pub struct FontAssetRegistry {
    assets: Mutex<HashMap<String, Vec<u8>>>,
}

impl Default for FontAssetRegistry {
    fn default() -> Self {
        Self {
            assets: Mutex::new(HashMap::new()),
        }
    }
}

static GLOBAL_FONT_REGISTRY: OnceLock<FontAssetRegistry> = OnceLock::new();

pub fn get_font_registry() -> &'static FontAssetRegistry {
    GLOBAL_FONT_REGISTRY.get_or_init(FontAssetRegistry::default)
}

impl FontAssetRegistry {
    pub fn register_font(&self, font_id: &str, bytes: Vec<u8>) {
        if let Ok(mut map) = self.assets.lock() {
            map.insert(font_id.to_string(), bytes);
        }
    }

    pub fn get_font(&self, font_id: &str) -> Option<Vec<u8>> {
        if let Ok(map) = self.assets.lock() {
            if let Some(bytes) = map.get(font_id) {
                return Some(bytes.clone());
            }
        }

        // On native platforms (e.g. during cargo test), attempt reading from public/fonts
        #[cfg(not(target_arch = "wasm32"))]
        {
            let filename = BUILTIN_FONT_CATALOG
                .iter()
                .find(|e| e.font_id == font_id)
                .map_or("", |e| e.asset_filename);

            if !filename.is_empty() {
                let candidates = [
                    format!("public/fonts/{}", filename),
                    format!("../../public/fonts/{}", filename),
                    format!("../public/fonts/{}", filename),
                ];
                for path in candidates {
                    if let Ok(data) = std::fs::read(&path) {
                        if let Ok(_font_ref) = FontRef::from_index(&data, 0) {
                            return Some(data);
                        }
                    }
                }
            }
        }

        None
    }

    /// Verifies exact OpenType glyph coverage using read-fonts cmap tables.
    pub fn verify_exact_coverage(&self, font_bytes: &[u8], text: &str) -> bool {
        if let Ok(font_ref) = FontRef::from_index(font_bytes, 0) {
            if let Ok(cmap) = font_ref.cmap() {
                for record in cmap.encoding_records() {
                    if let Ok(subtable) = record.subtable(cmap.offset_data()) {
                        let mut all_found = true;
                        for ch in text.chars() {
                            if subtable.map_codepoint(ch as u32).is_none() {
                                all_found = false;
                                break;
                            }
                        }
                        if all_found {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}
