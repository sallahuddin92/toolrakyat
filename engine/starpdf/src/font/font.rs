use std::collections::BTreeMap;

use crate::document::object_store::ObjectStore;
use crate::error::PdfResult;
use crate::font::cmap::UnicodeCMap;
use crate::font::encoding::SimpleEncoding;
use crate::syntax::object::PdfObject;

#[derive(Debug, Clone)]
pub struct Font {
    pub name: String,
    pub base_font: String,
    pub subtype: String,
    pub is_composite: bool,
    pub widths: BTreeMap<u32, f64>,
    pub default_width: f64,
    pub first_char: u32,
    pub last_char: u32,
    pub encoding: SimpleEncoding,
    pub to_unicode: Option<UnicodeCMap>,
}

impl Font {
    /// Constructs a fallback standard font (e.g. Helvetica / Type1).
    pub fn standard_fallback(name: &str) -> Self {
        Self {
            name: name.to_string(),
            base_font: "Helvetica".to_string(),
            subtype: "Type1".to_string(),
            is_composite: false,
            widths: BTreeMap::new(),
            default_width: 500.0,
            first_char: 0,
            last_char: 255,
            encoding: SimpleEncoding::standard_win_ansi(),
            to_unicode: None,
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

        // 1. Resolve /Widths array
        let mut widths = BTreeMap::new();
        let mut default_width = 1000.0;

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

        // 4. Handle Type0 DescendantFonts /DW and /W if composite
        if is_composite {
            if let Some(desc_obj) = font_dict.get("DescendantFonts") {
                let resolved_desc = store.resolve_object(desc_obj)?;
                if let Some(desc_arr) = resolved_desc.as_array() {
                    if let Some(first_cid_ref) = desc_arr.first() {
                        if let Ok(cid_obj) = store.resolve_object(first_cid_ref) {
                            if let Some(cid_dict) = cid_obj.as_dict() {
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
            widths,
            default_width,
            first_char,
            last_char,
            encoding,
            to_unicode,
        })
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
            .unwrap_or(self.default_width);

        // 2. Unicode decoding
        // Priority 1: /ToUnicode CMap
        if let Some(ref cmap) = self.to_unicode {
            if let Some(s) = cmap.lookup(code) {
                return (s.to_string(), width);
            }
        }

        // Priority 2: /Encoding table (for simple 8-bit fonts)
        if !self.is_composite && code <= 255 {
            let ch = self.encoding.decode_byte(code as u8);
            return (ch.to_string(), width);
        }

        // Priority 3: Fallback char from code
        let fallback = char::from_u32(code).unwrap_or('\u{FFFD}');
        (fallback.to_string(), width)
    }
}
