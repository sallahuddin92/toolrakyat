use std::collections::BTreeMap;

use crate::document::object_store::ObjectStore;
use crate::error::PdfResult;
use crate::font::font::{Font, FontStyle};
use crate::syntax::object::{ObjectRef, PdfObject};

pub struct PageResources {
    pub fonts: BTreeMap<String, Font>,
}

impl PageResources {
    /// Resolves all resources for a page, properly handling inheritance through the page tree hierarchy.
    pub fn resolve_for_page(
        page_dict: &BTreeMap<String, PdfObject>,
        store: &mut ObjectStore<'_>,
    ) -> PdfResult<Self> {
        let mut fonts = BTreeMap::new();

        // 1. Check direct /Resources on the page
        if let Some(res_obj) = page_dict.get("Resources") {
            let resolved_res = store.resolve_object(res_obj)?;
            if let Some(res_dict) = resolved_res.as_dict() {
                Self::extract_fonts_from_resource_dict(res_dict, store, &mut fonts)?;
            }
        }

        // 2. If no fonts found or /Resources missing, walk up /Parent chain to inherit
        if fonts.is_empty() {
            let mut current_parent_ref = page_dict.get("Parent").and_then(|v| v.as_reference());
            while let Some(parent_ref) = current_parent_ref {
                if let Ok(parent_obj) = store.resolve(parent_ref).cloned() {
                    if let Some(parent_dict) = parent_obj.as_dict() {
                        if let Some(res_obj) = parent_dict.get("Resources") {
                            if let Ok(resolved_res) = store.resolve_object(res_obj) {
                                if let Some(res_dict) = resolved_res.as_dict() {
                                    Self::extract_fonts_from_resource_dict(
                                        res_dict, store, &mut fonts,
                                    )?;
                                    if !fonts.is_empty() {
                                        break;
                                    }
                                }
                            }
                        }
                        current_parent_ref =
                            parent_dict.get("Parent").and_then(|v| v.as_reference());
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
        }

        Ok(Self { fonts })
    }

    fn extract_fonts_from_resource_dict(
        res_dict: &BTreeMap<String, PdfObject>,
        store: &mut ObjectStore<'_>,
        fonts: &mut BTreeMap<String, Font>,
    ) -> PdfResult<()> {
        if let Some(font_entry) = res_dict.get("Font") {
            let resolved_fonts = store.resolve_object(font_entry)?;
            if let Some(font_map) = resolved_fonts.as_dict() {
                for (font_name, font_obj_ref) in font_map {
                    let resolved_font_obj = store.resolve_object(font_obj_ref)?;
                    if let Some(font_dict) = resolved_font_obj.as_dict() {
                        if let Ok(font) = Font::from_dict(font_name, font_dict, store) {
                            fonts.insert(font_name.clone(), font);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn get_font(&self, name: &str) -> Option<&Font> {
        let clean = name.strip_prefix('/').unwrap_or(name);
        self.fonts
            .get(clean)
            .or_else(|| self.fonts.get(name))
            .or_else(|| {
                let with_slash = format!("/{}", clean);
                self.fonts.get(&with_slash)
            })
    }

    /// Finds an existing page font that matches the desired style and can encode the requested text.
    pub fn find_compatible_font(&self, style: &FontStyle, text: &str) -> Option<(&str, &Font)> {
        let mut best_match: Option<(&str, &Font, i32)> = None;

        for (name, font) in &self.fonts {
            if !font.can_encode_text(text) {
                continue;
            }

            let mut score = 0;
            if font.style.family == style.family {
                score += 10;
            }
            if font.style.is_bold == style.is_bold {
                score += 5;
            }
            if font.style.is_italic == style.is_italic {
                score += 5;
            }
            if font.style.is_monospace == style.is_monospace {
                score += 5;
            }

            if let Some((_, _, best_score)) = best_match {
                if score > best_score {
                    best_match = Some((name.as_str(), font, score));
                }
            } else {
                best_match = Some((name.as_str(), font, score));
            }
        }

        best_match.map(|(name, font, _)| (name, font))
    }

    /// Finds an existing page font with an exact family/weight/italic match and glyph coverage.
    pub fn find_exact_style_font(&self, style: &FontStyle, text: &str) -> Option<(&str, &Font)> {
        self.fonts.iter().find_map(|(name, font)| {
            (font.style == *style && font.can_encode_text(text)).then_some((name.as_str(), font))
        })
    }

    /// Ensures a standard style-matched font is registered in the page resources.
    /// Returns the font resource name (e.g. `F_StarPDF_HelveticaBold`).
    pub fn ensure_standard_font(
        &mut self,
        style: &FontStyle,
        page_dict: &mut BTreeMap<String, PdfObject>,
        font_ref: ObjectRef,
        store: &mut ObjectStore<'_>,
        modified: &mut BTreeMap<ObjectRef, PdfObject>,
    ) -> PdfResult<String> {
        let base_font = style.standard_base_font_name();
        let res_name = format!("F_StarPDF_{}", base_font.replace('-', ""));

        // If already present in resources, return it
        if self.fonts.contains_key(&res_name) {
            return Ok(res_name);
        }

        // Create standard Type1 font object
        let font_dict = BTreeMap::from([
            ("Type".to_string(), PdfObject::Name("Font".to_string())),
            ("Subtype".to_string(), PdfObject::Name("Type1".to_string())),
            (
                "BaseFont".to_string(),
                PdfObject::Name(base_font.to_string()),
            ),
            (
                "Encoding".to_string(),
                PdfObject::Name("WinAnsiEncoding".to_string()),
            ),
        ]);
        modified.insert(font_ref, PdfObject::Dictionary(font_dict));

        // Ensure page /Resources /Font dictionary exists and has this font
        let mut resources_dict = match page_dict.get("Resources") {
            Some(res_obj) => match store.resolve_object(res_obj)? {
                PdfObject::Dictionary(d) => d.clone(),
                _ => BTreeMap::new(),
            },
            None => BTreeMap::new(),
        };

        let mut font_map = match resources_dict.get("Font") {
            Some(font_obj) => match store.resolve_object(font_obj)? {
                PdfObject::Dictionary(d) => d.clone(),
                _ => BTreeMap::new(),
            },
            None => BTreeMap::new(),
        };

        font_map.insert(res_name.clone(), PdfObject::Reference(font_ref));
        resources_dict.insert("Font".to_string(), PdfObject::Dictionary(font_map));
        page_dict.insert(
            "Resources".to_string(),
            PdfObject::Dictionary(resources_dict),
        );

        // Add font to local `self.fonts` cache
        let font = Font::standard_with_style(&res_name, style);
        self.fonts.insert(res_name.clone(), font);

        Ok(res_name)
    }
}
