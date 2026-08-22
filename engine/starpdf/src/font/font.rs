use std::collections::BTreeMap;

use crate::document::object_store::ObjectStore;
use crate::error::PdfResult;
use crate::font::cmap::UnicodeCMap;
use crate::font::encoding::SimpleEncoding;
use crate::font::sfnt::SfntFont;
use crate::syntax::object::PdfObject;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontProgramKind {
    TrueTypeSupported,
    CffDetectedUnsupported,
    Cff2DetectedUnsupported,
    UnknownFontProgram,
}

impl FontProgramKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TrueTypeSupported => "TRUETYPE_SUPPORTED",
            Self::CffDetectedUnsupported => "CFF_DETECTED_UNSUPPORTED",
            Self::Cff2DetectedUnsupported => "CFF2_DETECTED_UNSUPPORTED",
            Self::UnknownFontProgram => "UNKNOWN_FONT_PROGRAM",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Font {
    pub name: String,
    pub base_font: String,
    pub subtype: String,
    pub is_composite: bool,
    pub composite_identity_mapping: bool,
    pub widths: BTreeMap<u32, f64>,
    pub default_width: f64,
    pub first_char: u32,
    pub last_char: u32,
    pub encoding: SimpleEncoding,
    pub to_unicode: Option<UnicodeCMap>,
    pub embedded_sfnt: Option<SfntFont>,
    pub font_program_kind: FontProgramKind,
}

impl Font {
    /// Constructs a fallback standard font (e.g. Helvetica / Type1).
    pub fn standard_fallback(name: &str) -> Self {
        Self {
            name: name.to_string(),
            base_font: "Helvetica".to_string(),
            subtype: "Type1".to_string(),
            is_composite: false,
            composite_identity_mapping: false,
            widths: BTreeMap::new(),
            default_width: 500.0,
            first_char: 0,
            last_char: 255,
            encoding: SimpleEncoding::standard_win_ansi(),
            to_unicode: None,
            embedded_sfnt: None,
            font_program_kind: FontProgramKind::UnknownFontProgram,
        }
    }

    /// Resolves and builds a `Font` from a PDF Font dictionary.
    pub fn from_dict(
        name: &str,
        font_dict: &BTreeMap<String, PdfObject>,
        store: &mut ObjectStore<'_>,
    ) -> PdfResult<Self> {
        let subtype = font_dict
            .get("Subtype")
            .and_then(|v| v.as_name())
            .unwrap_or("Type1")
            .to_string();

        let base_font = font_dict
            .get("BaseFont")
            .and_then(|v| v.as_name())
            .unwrap_or(name)
            .to_string();

        let is_composite = subtype == "Type0";
        let mut composite_identity_mapping = false;

        // 1. Resolve /Widths array
        let mut widths = BTreeMap::new();
        let mut default_width = 500.0;

        let first_char = font_dict
            .get("FirstChar")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as u32;

        let last_char = font_dict
            .get("LastChar")
            .and_then(|v| v.as_i64())
            .unwrap_or(255) as u32;

        if let Some(widths_obj) = font_dict.get("Widths") {
            let resolved_widths = store.resolve_object(widths_obj)?;
            if let Some(arr) = resolved_widths.as_array() {
                for (i, w_val) in arr.iter().enumerate() {
                    let char_code = first_char.saturating_add(i as u32);
                    if let Some(w) = w_val.as_f64() {
                        widths.insert(char_code, w);
                    }
                }
            }
        }

        // 2. Resolve /Encoding
        let mut encoding = SimpleEncoding::standard_win_ansi();
        if let Some(enc_obj) = font_dict.get("Encoding") {
            let resolved_enc = store.resolve_object(enc_obj)?;
            encoding = SimpleEncoding::from_pdf_object(&resolved_enc);
        }

        // 3. Resolve /ToUnicode CMap stream
        let mut to_unicode = None;
        if let Some(tu_obj) = font_dict.get("ToUnicode") {
            if let Ok(resolved_tu) = store.resolve_object(tu_obj) {
                if let Some(stream) = resolved_tu.as_stream() {
                    // Check if stream is Flate-encoded
                    let mut data = stream.data.clone();
                    if let Some(filter) = stream.dict.get("Filter").and_then(|v| v.as_name()) {
                        if filter == "FlateDecode" {
                            if let Ok(decompressed) = crate::filter::flate::FlateDecoder::decode(
                                &stream.data,
                                &crate::filter::limits::DecompressLimits::default(),
                            ) {
                                data = decompressed;
                            }
                        }
                    }
                    if let Ok(parsed_cmap) = UnicodeCMap::parse(&data) {
                        to_unicode = Some(parsed_cmap);
                    }
                }
            }
        }

        // 4. Resolve embedded TrueType/SFNT data from simple or Type0 descendant fonts.
        let (mut font_program_kind, mut embedded_sfnt) =
            Self::embedded_program_from_font_dict(font_dict, store)?;

        // 5. Handle Type0 DescendantFonts /DW and /W if composite
        if is_composite {
            let identity_encoding = match font_dict.get("Encoding") {
                Some(encoding) => store
                    .resolve_object(encoding)?
                    .as_name()
                    .is_some_and(|name| matches!(name, "Identity-H" | "Identity-V")),
                None => false,
            };
            if let Some(desc_obj) = font_dict.get("DescendantFonts") {
                let resolved_desc = store.resolve_object(desc_obj)?;
                if let Some(desc_arr) = resolved_desc.as_array() {
                    if let Some(first_cid_ref) = desc_arr.first() {
                        if let Ok(cid_obj) = store.resolve_object(first_cid_ref) {
                            if let Some(cid_dict) = cid_obj.as_dict() {
                                let identity_cid_to_gid = match cid_dict.get("CIDToGIDMap") {
                                    None => true,
                                    Some(mapping) => store
                                        .resolve_object(mapping)?
                                        .as_name()
                                        .is_some_and(|name| name == "Identity"),
                                };
                                composite_identity_mapping =
                                    identity_encoding && identity_cid_to_gid;
                                if embedded_sfnt.is_none() {
                                    let (descendant_kind, descendant_sfnt) =
                                        Self::embedded_program_from_font_dict(cid_dict, store)?;
                                    font_program_kind = descendant_kind;
                                    embedded_sfnt = descendant_sfnt;
                                }
                                if let Some(dw) = cid_dict.get("DW").and_then(|v| v.as_f64()) {
                                    default_width = dw;
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(Self {
            name: name.to_string(),
            base_font,
            subtype,
            is_composite,
            composite_identity_mapping,
            widths,
            default_width,
            first_char,
            last_char,
            encoding,
            to_unicode,
            embedded_sfnt,
            font_program_kind,
        })
    }

    fn embedded_program_from_font_dict(
        font_dict: &BTreeMap<String, PdfObject>,
        store: &mut ObjectStore<'_>,
    ) -> PdfResult<(FontProgramKind, Option<SfntFont>)> {
        let Some(descriptor) = font_dict.get("FontDescriptor") else {
            return Ok((FontProgramKind::UnknownFontProgram, None));
        };
        let descriptor = store.resolve_object(descriptor)?;
        let Some(descriptor_dict) = descriptor.as_dict() else {
            return Err(crate::error::PdfError::TypeMismatch {
                expected: "font descriptor dictionary",
                actual: descriptor.type_name(),
            });
        };
        let (font_file_key, font_file) = if let Some(file) = descriptor_dict.get("FontFile2") {
            ("FontFile2", file)
        } else if let Some(file) = descriptor_dict.get("FontFile3") {
            ("FontFile3", file)
        } else if let Some(file) = descriptor_dict.get("FontFile") {
            ("FontFile", file)
        } else {
            return Ok((FontProgramKind::UnknownFontProgram, None));
        };
        let font_file = store.resolve_object(font_file)?;
        let stream = font_file
            .as_stream()
            .ok_or_else(|| crate::error::PdfError::TypeMismatch {
                expected: "embedded font stream",
                actual: font_file.type_name(),
            })?;
        if stream.data.len() > crate::font::appearance::MAX_EMBEDDED_FONT_BYTES {
            return Err(crate::error::PdfError::InvalidOperation(format!(
                "Embedded font stream exceeds maximum of {} bytes",
                crate::font::appearance::MAX_EMBEDDED_FONT_BYTES
            )));
        }
        let data = match stream.dict.get("Filter").and_then(PdfObject::as_name) {
            Some("FlateDecode") => crate::filter::flate::FlateDecoder::decode(
                &stream.data,
                &crate::filter::limits::DecompressLimits::default(),
            )?,
            Some(filter) => {
                return Err(crate::error::PdfError::InvalidOperation(format!(
                    "Unsupported embedded font stream filter /{filter}"
                )))
            }
            None => stream.data.clone(),
        };
        let stream_subtype = stream.dict.get("Subtype").and_then(PdfObject::as_name);
        let kind = Self::detect_font_program(font_file_key, stream_subtype, &data)?;
        let sfnt = if kind == FontProgramKind::TrueTypeSupported {
            Some(SfntFont::parse(&data)?)
        } else {
            None
        };
        Ok((kind, sfnt))
    }

    pub fn detect_font_program(
        font_file_key: &str,
        stream_subtype: Option<&str>,
        data: &[u8],
    ) -> PdfResult<FontProgramKind> {
        if font_file_key == "FontFile3"
            && matches!(stream_subtype, Some("Type1C" | "CIDFontType0C"))
        {
            return Ok(FontProgramKind::CffDetectedUnsupported);
        }
        let is_sfnt = data.starts_with(&[0x00, 0x01, 0x00, 0x00])
            || data.starts_with(b"true")
            || data.starts_with(b"OTTO");
        if !is_sfnt {
            return Ok(FontProgramKind::UnknownFontProgram);
        }
        let directory = crate::font::sfnt::TableDirectory::parse(data)?;
        if directory.tables.contains_key(b"CFF2") {
            return Ok(FontProgramKind::Cff2DetectedUnsupported);
        }
        if directory.tables.contains_key(b"CFF ") {
            return Ok(FontProgramKind::CffDetectedUnsupported);
        }
        if directory.tables.contains_key(b"glyf") && directory.tables.contains_key(b"loca") {
            return Ok(FontProgramKind::TrueTypeSupported);
        }
        Ok(FontProgramKind::UnknownFontProgram)
    }

    /// Decodes a byte sequence into individual character glyph representations and advances.
    pub fn decode_bytes(&self, bytes: &[u8]) -> Vec<(String, f64)> {
        let mut result = Vec::new();

        if self.is_composite {
            // 2 bytes per CID for Type0 composite fonts
            let mut i = 0;
            while i < bytes.len() {
                let code = if i + 1 < bytes.len() {
                    let c = ((bytes[i] as u32) << 8) | (bytes[i + 1] as u32);
                    i += 2;
                    c
                } else {
                    let c = bytes[i] as u32;
                    i += 1;
                    c
                };

                let (text, width) = self.decode_char_code(code);
                result.push((text, width));
            }
        } else {
            // 1 byte per char for simple fonts
            for &b in bytes {
                let code = b as u32;
                let (text, width) = self.decode_char_code(code);
                result.push((text, width));
            }
        }

        result
    }

    /// Decodes a single character code into Unicode string and width.
    pub fn decode_char_code(&self, code: u32) -> (String, f64) {
        // 1. Width calculation
        let width = self
            .widths
            .get(&code)
            .copied()
            .or_else(|| {
                self.embedded_sfnt
                    .as_ref()
                    .and_then(|f| f.get_advance_width(code))
            })
            .unwrap_or(self.default_width);

        // 2. Unicode decoding
        // Priority 1: /ToUnicode CMap
        if let Some(ref cmap) = self.to_unicode {
            if let Some(s) = cmap.lookup(code) {
                return (s.to_string(), width);
            }
        }

        // Priority 2: Embedded font cmap table (when /ToUnicode is absent)
        if let Some(ref sfnt) = self.embedded_sfnt {
            if let Some(ch) = sfnt.decode_char_code(code) {
                return (ch.to_string(), width);
            }
        }

        // Priority 3: /Encoding table (for simple 8-bit fonts)
        if !self.is_composite && code <= 255 {
            let ch = self.encoding.decode_byte(code as u8);
            return (ch.to_string(), width);
        }

        // Priority 4: Fallback char from code
        let fallback = char::from_u32(code).unwrap_or('\u{FFFD}');
        (fallback.to_string(), width)
    }

    /// Checks if text extracted with this font can be edited within the v0.13 boundary.
    pub fn check_span_editability(
        &self,
        original_text: &str,
    ) -> crate::text::span::TextEditability {
        if self.is_composite && !self.composite_identity_mapping {
            return crate::text::span::TextEditability::UnsupportedFontEncoding(
                "Composite font requires Identity-H/Identity-V with Identity CIDToGIDMap"
                    .to_string(),
            );
        }
        for ch in original_text.chars() {
            if is_complex_script_char(ch) {
                return crate::text::span::TextEditability::UnsupportedComplexScript(format!(
                    "Character U+{:04X} requires complex script shaping",
                    ch as u32
                ));
            }
        }
        if self.is_composite && self.embedded_sfnt.is_none() && self.to_unicode.is_none() {
            return crate::text::span::TextEditability::UnsupportedFontEncoding(
                "Composite font missing embedded SFNT and ToUnicode map".to_string(),
            );
        }
        crate::text::span::TextEditability::EditableNativeText
    }

    /// Encodes a text string into the font's native character/glyph byte representation.
    pub fn encode_text(&self, text: &str) -> PdfResult<Vec<u8>> {
        for ch in text.chars() {
            if is_complex_script_char(ch) {
                return Err(crate::error::PdfError::UnsupportedComplexScript(format!(
                    "Character U+{:04X} requires complex script shaping",
                    ch as u32
                )));
            }
        }

        let mut output = Vec::with_capacity(text.len().saturating_mul(2));
        for character in text.chars() {
            if self.is_composite {
                if !self.composite_identity_mapping {
                    return Err(crate::error::PdfError::UnsupportedCompositeMapping(
                        "composite font requires Identity-H/Identity-V and an identity CIDToGIDMap"
                            .into(),
                    ));
                }
                let glyph = if let Some(sfnt) = &self.embedded_sfnt {
                    sfnt.cmap
                        .as_ref()
                        .and_then(|cmap| cmap.map_char_to_glyph(character as u32))
                        .or_else(|| {
                            self.to_unicode
                                .as_ref()
                                .and_then(|tu| tu.reverse_lookup(character))
                                .map(|c| c as u16)
                        })
                } else if let Some(tu) = &self.to_unicode {
                    tu.reverse_lookup(character).map(|c| c as u16)
                } else {
                    None
                };

                let glyph = glyph.ok_or_else(|| {
                    crate::error::PdfError::UnsupportedFontEncoding(format!(
                        "UNREPRESENTABLE glyph U+{:04X} in composite font /{}",
                        character as u32, self.base_font
                    ))
                })?;
                output.extend_from_slice(&glyph.to_be_bytes());
            } else {
                let mut mapped_code: Option<u8> = None;
                if let Some(tu) = &self.to_unicode {
                    if let Some(code) = tu.reverse_lookup(character) {
                        if code <= 255 {
                            mapped_code = Some(code as u8);
                        }
                    }
                }

                if mapped_code.is_none() {
                    mapped_code = (0u16..=255)
                        .find(|&code| self.encoding.decode_byte(code as u8) == character)
                        .map(|c| c as u8);
                }

                let code = mapped_code.ok_or_else(|| {
                    crate::error::PdfError::UnsupportedFontEncoding(format!(
                        "UNREPRESENTABLE glyph U+{:04X} in font /{}",
                        character as u32, self.base_font
                    ))
                })?;

                if let Some(sfnt) = &self.embedded_sfnt {
                    let has_glyph = sfnt
                        .cmap
                        .as_ref()
                        .and_then(|cmap| cmap.map_char_to_glyph(character as u32))
                        .is_some()
                        || sfnt
                            .cmap
                            .as_ref()
                            .and_then(|cmap| cmap.map_char_to_glyph(code as u32))
                            .is_some();
                    if !has_glyph {
                        return Err(crate::error::PdfError::UnsupportedFontEncoding(format!(
                            "UNREPRESENTABLE glyph U+{:04X} in embedded font /{}",
                            character as u32, self.base_font
                        )));
                    }
                }
                output.push(code);
            }
        }
        Ok(output)
    }

    /// Calculates the horizontal advance width in text space for a text string.
    pub fn calculate_text_width(
        &self,
        text: &str,
        font_size: f64,
        char_spacing: f64,
        word_spacing: f64,
        horizontal_scaling: f64,
    ) -> PdfResult<f64> {
        let mut total_advance = 0.0;
        for character in text.chars() {
            let code = if self.is_composite {
                let glyph = if let Some(sfnt) = &self.embedded_sfnt {
                    sfnt.cmap
                        .as_ref()
                        .and_then(|cmap| cmap.map_char_to_glyph(character as u32))
                        .map(u32::from)
                        .or_else(|| {
                            self.to_unicode
                                .as_ref()
                                .and_then(|tu| tu.reverse_lookup(character))
                        })
                } else if let Some(tu) = &self.to_unicode {
                    tu.reverse_lookup(character)
                } else {
                    None
                };
                glyph.ok_or_else(|| {
                    crate::error::PdfError::UnsupportedFontEncoding(format!(
                        "UNREPRESENTABLE glyph U+{:04X} in font /{}",
                        character as u32, self.base_font
                    ))
                })?
            } else {
                let code_opt = self
                    .to_unicode
                    .as_ref()
                    .and_then(|tu| tu.reverse_lookup(character))
                    .or_else(|| {
                        (0u16..=255)
                            .find(|&c| self.encoding.decode_byte(c as u8) == character)
                            .map(u32::from)
                    });
                code_opt.ok_or_else(|| {
                    crate::error::PdfError::UnsupportedFontEncoding(format!(
                        "UNREPRESENTABLE glyph U+{:04X} in font /{}",
                        character as u32, self.base_font
                    ))
                })?
            };

            let width = self
                .widths
                .get(&code)
                .copied()
                .or_else(|| {
                    self.embedded_sfnt
                        .as_ref()
                        .and_then(|f| f.get_advance_width(code))
                })
                .unwrap_or(self.default_width);

            let mut advance = (width / 1000.0) * font_size;
            advance += char_spacing;
            if character == ' ' {
                advance += word_spacing;
            }
            advance *= horizontal_scaling / 100.0;
            total_advance += advance;
        }
        Ok(total_advance)
    }
}

/// Identifies characters belonging to complex script writing systems that require shaping engines.
pub fn is_complex_script_char(ch: char) -> bool {
    let u = ch as u32;
    matches!(
        u,
        0x0600..=0x08FF
            | 0xFB50..=0xFDFF
            | 0xFE70..=0xFEFF
            | 0x0900..=0x0D7F
            | 0x0E00..=0x0EFF
            | 0x0F00..=0x109F
            | 0x1780..=0x17FF
            | 0x19E0..=0x19FF
    )
}

#[cfg(test)]
mod tests {
    use super::{Font, FontProgramKind};

    fn sfnt(tags: &[[u8; 4]]) -> Vec<u8> {
        let mut output = vec![0u8; 12 + tags.len() * 16];
        output[0..4].copy_from_slice(b"OTTO");
        output[4..6].copy_from_slice(&(tags.len() as u16).to_be_bytes());
        let table_offset = output.len() as u32;
        for (index, tag) in tags.iter().enumerate() {
            let record = 12 + index * 16;
            output[record..record + 4].copy_from_slice(tag);
            output[record + 8..record + 12].copy_from_slice(&table_offset.to_be_bytes());
        }
        output
    }

    #[test]
    fn detects_supported_and_unsupported_font_programs_without_guessing() {
        assert_eq!(
            Font::detect_font_program("FontFile2", None, &sfnt(&[*b"glyf", *b"loca"]))
                .unwrap_or(FontProgramKind::UnknownFontProgram),
            FontProgramKind::TrueTypeSupported
        );
        assert_eq!(
            Font::detect_font_program("FontFile3", Some("OpenType"), &sfnt(&[*b"CFF "]))
                .unwrap_or(FontProgramKind::UnknownFontProgram),
            FontProgramKind::CffDetectedUnsupported
        );
        assert_eq!(
            Font::detect_font_program("FontFile3", Some("OpenType"), &sfnt(&[*b"CFF2"]))
                .unwrap_or(FontProgramKind::UnknownFontProgram),
            FontProgramKind::Cff2DetectedUnsupported
        );
        assert_eq!(
            Font::detect_font_program("FontFile3", Some("Type1C"), &[1, 0, 4, 4])
                .unwrap_or(FontProgramKind::UnknownFontProgram),
            FontProgramKind::CffDetectedUnsupported
        );
        assert_eq!(
            Font::detect_font_program("FontFile", None, b"%!PS-AdobeFont")
                .unwrap_or(FontProgramKind::TrueTypeSupported),
            FontProgramKind::UnknownFontProgram
        );
    }
}
