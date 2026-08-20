use std::collections::BTreeMap;

use crate::document::object_store::ObjectStore;
use crate::error::PdfResult;
use crate::font::font::Font;
use crate::syntax::object::PdfObject;

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
}
