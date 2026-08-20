use std::collections::{BTreeMap, HashSet};

use crate::document::object_store::ObjectStore;
use crate::error::{PdfError, PdfResult};
use crate::font::Font;
use crate::syntax::object::{ObjectRef, PdfObject};

pub const MAX_EMBEDDED_FONT_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_APPEARANCE_RESOURCES: usize = 256;
const MAX_RESOURCE_ANCESTORS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlyphMappingQuality {
    Exact,
    Fallback,
    Unrepresentable,
}

impl GlyphMappingQuality {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "EXACT",
            Self::Fallback => "FALLBACK",
            Self::Unrepresentable => "UNREPRESENTABLE",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppearanceFont {
    pub resource_name: String,
    pub resource_object: PdfObject,
    pub font: Font,
    pub quality: GlyphMappingQuality,
}

impl AppearanceFont {
    pub fn encode_text(&self, text: &str) -> PdfResult<Vec<u8>> {
        let mut output = Vec::with_capacity(text.len().saturating_mul(2));
        for character in text.chars() {
            if self.font.is_composite {
                if !self.font.composite_identity_mapping {
                    return Err(PdfError::InvalidOperation(
                        "UNREPRESENTABLE composite appearance font requires Identity-H/Identity-V and an identity CIDToGIDMap"
                            .into(),
                    ));
                }
                let glyph = self
                    .font
                    .embedded_sfnt
                    .as_ref()
                    .and_then(|font| font.cmap.as_ref())
                    .and_then(|cmap| cmap.map_char_to_glyph(character as u32))
                    .ok_or_else(|| Self::missing_glyph(character, &self.font.base_font))?;
                output.extend_from_slice(&glyph.to_be_bytes());
            } else {
                let code = (0u16..=255)
                    .find(|code| self.font.encoding.decode_byte(*code as u8) == character)
                    .map(|code| code as u8)
                    .ok_or_else(|| Self::missing_glyph(character, &self.font.base_font))?;
                if let Some(sfnt) = &self.font.embedded_sfnt {
                    if sfnt
                        .cmap
                        .as_ref()
                        .and_then(|cmap| cmap.map_char_to_glyph(character as u32))
                        .is_none()
                    {
                        return Err(Self::missing_glyph(character, &self.font.base_font));
                    }
                }
                output.push(code);
            }
        }
        Ok(output)
    }

    pub fn verify_text(&self, text: &str) -> PdfResult<GlyphMappingQuality> {
        self.encode_text(text)?;
        Ok(self.quality)
    }

    pub fn text_width(&self, text: &str, font_size: f64) -> PdfResult<f64> {
        if self.font.is_composite && !self.font.composite_identity_mapping {
            return Err(PdfError::InvalidOperation(
                "UNREPRESENTABLE composite appearance font requires Identity-H/Identity-V and an identity CIDToGIDMap"
                    .into(),
            ));
        }
        let mut total = 0.0;
        for character in text.chars() {
            let code = if self.font.is_composite {
                self.font
                    .embedded_sfnt
                    .as_ref()
                    .and_then(|font| font.cmap.as_ref())
                    .and_then(|cmap| cmap.map_char_to_glyph(character as u32))
                    .map(u32::from)
                    .ok_or_else(|| Self::missing_glyph(character, &self.font.base_font))?
            } else {
                (0u16..=255)
                    .find(|code| self.font.encoding.decode_byte(*code as u8) == character)
                    .map(u32::from)
                    .ok_or_else(|| Self::missing_glyph(character, &self.font.base_font))?
            };
            let width = self
                .font
                .widths
                .get(&code)
                .copied()
                .or_else(|| {
                    self.font
                        .embedded_sfnt
                        .as_ref()
                        .and_then(|font| font.get_advance_width(character as u32))
                })
                .unwrap_or(self.font.default_width);
            total += width * font_size / 1000.0;
        }
        Ok(total)
    }

    pub fn resource_dictionary(&self) -> BTreeMap<String, PdfObject> {
        BTreeMap::from([(
            "Font".to_string(),
            PdfObject::Dictionary(BTreeMap::from([(
                self.resource_name.clone(),
                self.resource_object.clone(),
            )])),
        )])
    }

    fn missing_glyph(character: char, font_name: &str) -> PdfError {
        PdfError::InvalidOperation(format!(
            "UNREPRESENTABLE glyph U+{:04X} in appearance font /{font_name}",
            character as u32
        ))
    }
}

pub struct AppearanceFontResolver;

impl AppearanceFontResolver {
    pub fn resolve(
        store: &mut ObjectStore<'_>,
        field_dict: &BTreeMap<String, PdfObject>,
        page_refs: &[ObjectRef],
        requested_name: &str,
        text: &str,
    ) -> PdfResult<AppearanceFont> {
        let clean_name = requested_name.trim_start_matches('/');
        if clean_name.is_empty() {
            return Err(PdfError::InvalidOperation(
                "Default appearance font resource name is empty".into(),
            ));
        }

        if let Some(resource) = field_dict.get("DR") {
            if let Some(font) = Self::find_in_resources(store, resource, clean_name)? {
                font.verify_text(text)?;
                return Ok(font);
            }
        }
        if let Some(font) = Self::find_in_acroform_resources(store, clean_name)? {
            font.verify_text(text)?;
            return Ok(font);
        }
        for page_ref in page_refs.iter().copied() {
            if let Some(font) = Self::find_in_page_resources(store, page_ref, clean_name)? {
                font.verify_text(text)?;
                return Ok(font);
            }
        }

        let fallback = Font::standard_fallback(clean_name);
        let resolved = AppearanceFont {
            resource_name: clean_name.to_string(),
            resource_object: Self::standard_font_object(),
            font: fallback,
            quality: GlyphMappingQuality::Fallback,
        };
        resolved.verify_text(text)?;
        Ok(resolved)
    }

    fn find_in_acroform_resources(
        store: &mut ObjectStore<'_>,
        font_name: &str,
    ) -> PdfResult<Option<AppearanceFont>> {
        let root = store
            .trailer()
            .get("Root")
            .cloned()
            .ok_or_else(|| PdfError::InvalidSyntax("Trailer missing /Root".into()))?;
        let catalog = store.resolve_object(&root)?;
        let Some(catalog_dict) = catalog.as_dict() else {
            return Ok(None);
        };
        let Some(acroform) = catalog_dict.get("AcroForm") else {
            return Ok(None);
        };
        let acroform = store.resolve_object(acroform)?;
        let Some(acroform_dict) = acroform.as_dict() else {
            return Ok(None);
        };
        let Some(resources) = acroform_dict.get("DR") else {
            return Ok(None);
        };
        Self::find_in_resources(store, resources, font_name)
    }

    fn find_in_page_resources(
        store: &mut ObjectStore<'_>,
        page_ref: ObjectRef,
        font_name: &str,
    ) -> PdfResult<Option<AppearanceFont>> {
        let mut current = Some(page_ref);
        let mut visited = HashSet::new();
        for _ in 0..MAX_RESOURCE_ANCESTORS {
            let Some(reference) = current else {
                break;
            };
            if !visited.insert(reference) {
                return Err(PdfError::CircularReference(
                    "Cycle while resolving page font resources".into(),
                ));
            }
            let object = store.resolve(reference)?.clone();
            let Some(dict) = object.as_dict() else {
                break;
            };
            if let Some(resources) = dict.get("Resources") {
                if let Some(font) = Self::find_in_resources(store, resources, font_name)? {
                    return Ok(Some(font));
                }
            }
            current = dict.get("Parent").and_then(PdfObject::as_reference);
        }
        Ok(None)
    }

    fn find_in_resources(
        store: &mut ObjectStore<'_>,
        resources: &PdfObject,
        font_name: &str,
    ) -> PdfResult<Option<AppearanceFont>> {
        let resources = store.resolve_object(resources)?;
        let Some(resources_dict) = resources.as_dict() else {
            return Ok(None);
        };
        if resources_dict.len() > MAX_APPEARANCE_RESOURCES {
            return Err(PdfError::InvalidOperation(format!(
                "Appearance resources exceed maximum of {MAX_APPEARANCE_RESOURCES}"
            )));
        }
        let Some(fonts) = resources_dict.get("Font") else {
            return Ok(None);
        };
        let fonts = store.resolve_object(fonts)?;
        let Some(fonts_dict) = fonts.as_dict() else {
            return Ok(None);
        };
        if fonts_dict.len() > MAX_APPEARANCE_RESOURCES {
            return Err(PdfError::InvalidOperation(format!(
                "Appearance font resources exceed maximum of {MAX_APPEARANCE_RESOURCES}"
            )));
        }
        let Some(font_object) = fonts_dict.get(font_name) else {
            return Ok(None);
        };
        let resource_object = font_object.clone();
        let resolved_font = store.resolve_object(font_object)?;
        let font_dict = resolved_font
            .as_dict()
            .ok_or_else(|| PdfError::TypeMismatch {
                expected: "font dictionary",
                actual: resolved_font.type_name(),
            })?;
        let font = Font::from_dict(font_name, font_dict, store)?;
        Ok(Some(AppearanceFont {
            resource_name: font_name.to_string(),
            resource_object,
            font,
            quality: GlyphMappingQuality::Exact,
        }))
    }

    fn standard_font_object() -> PdfObject {
        PdfObject::Dictionary(BTreeMap::from([
            ("Type".to_string(), PdfObject::Name("Font".to_string())),
            ("Subtype".to_string(), PdfObject::Name("Type1".to_string())),
            (
                "BaseFont".to_string(),
                PdfObject::Name("Helvetica".to_string()),
            ),
            (
                "Encoding".to_string(),
                PdfObject::Name("WinAnsiEncoding".to_string()),
            ),
        ]))
    }
}
