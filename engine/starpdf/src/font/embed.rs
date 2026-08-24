use crate::document::object_store::ObjectStore;
use crate::error::{PdfError, PdfResult};
use crate::font::subset::TrueTypeSubsetter;
use crate::syntax::object::{ObjectRef, PdfObject, StreamObject};
use read_fonts::{FontRef, TableProvider};
use std::collections::BTreeMap;
use std::fmt::Write;

pub struct Type0FontEmbedder;

impl Type0FontEmbedder {
    fn inherited_resources(
        store: &mut ObjectStore<'_>,
        page_dict: &BTreeMap<String, PdfObject>,
    ) -> PdfResult<BTreeMap<String, PdfObject>> {
        let mut current = page_dict.clone();
        for _ in 0..64 {
            if let Some(resources) = current.get("Resources") {
                let resolved = store.resolve_object(resources)?.clone();
                return Ok(resolved.as_dict().cloned().unwrap_or_default());
            }
            let Some(parent_ref) = current.get("Parent").and_then(PdfObject::as_reference) else {
                return Ok(BTreeMap::new());
            };
            let parent = store.resolve(parent_ref)?.clone();
            current = parent
                .as_dict()
                .cloned()
                .ok_or_else(|| PdfError::TypeMismatch {
                    expected: "dictionary",
                    actual: parent.type_name(),
                })?;
        }
        Err(PdfError::RecursionLimitExceeded)
    }

    /// Generates a valid Adobe ToUnicode CMap stream mapping CIDs (Glyph IDs) to Unicode codepoints/clusters.
    pub fn build_tounicode_cmap(
        cid_to_unicode: &BTreeMap<u16, String>,
        font_name: &str,
    ) -> Vec<u8> {
        let mut cmap = String::new();
        cmap.push_str("/CIDInit /ProcSet findresource begin\n");
        cmap.push_str("12 dict begin\n");
        cmap.push_str("begincmap\n");
        cmap.push_str("/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n");
        let _ = writeln!(cmap, "/CMapName /StarPDF-{}-ToUnicode def", font_name);
        cmap.push_str("/CMapType 2 def\n");
        cmap.push_str("1 begincodespacerange\n");
        cmap.push_str("<0000> <FFFF>\n");
        cmap.push_str("endcodespacerange\n");

        if !cid_to_unicode.is_empty() {
            // Group into chunks of up to 100 entries
            let entries: Vec<(&u16, &String)> = cid_to_unicode.iter().collect();
            for chunk in entries.chunks(100) {
                let _ = writeln!(cmap, "{} beginbfchar", chunk.len());
                for (cid, cluster_text) in chunk {
                    let mut utf16_hex = String::new();
                    for unit in cluster_text.encode_utf16() {
                        let _ = write!(utf16_hex, "{:04X}", unit);
                    }
                    if utf16_hex.is_empty() {
                        utf16_hex.push_str("0020");
                    }
                    let _ = writeln!(cmap, "<{:04X}> <{}>", cid, utf16_hex);
                }
                cmap.push_str("endbfchar\n");
            }
        }

        cmap.push_str("endcmap\n");
        cmap.push_str("CMapName currentdict /CMap defineresource pop\n");
        cmap.push_str("end\n");
        cmap.push_str("end\n");

        cmap.into_bytes()
    }

    fn allocate_ref(next_num: &mut u64) -> ObjectRef {
        let num = *next_num;
        *next_num += 1;
        ObjectRef::new(num, 0)
    }

    /// Embeds a real OpenType/TrueType font into the PDF document as a Type0 / CIDFontType2 font.
    /// Returns the allocated resource name and updates `modified`.
    pub fn embed_type0_font(
        store: &mut ObjectStore<'_>,
        page_ref: ObjectRef,
        font_bytes: &[u8],
        font_name: &str,
        requested_glyphs: &[u16],
        cid_to_gid: &BTreeMap<u16, u16>,
        cid_to_unicode: &BTreeMap<u16, String>,
        glyph_widths: &BTreeMap<u16, f64>,
        modified: &mut BTreeMap<ObjectRef, PdfObject>,
        next_alloc_num: &mut u64,
    ) -> PdfResult<String> {
        // 1. Subset the TrueType font
        let mut glyph_set: Vec<u16> = requested_glyphs.to_vec();
        glyph_set.sort_unstable();
        glyph_set.dedup();
        if glyph_set.is_empty() {
            glyph_set.push(0);
        }

        let subset = TrueTypeSubsetter::subset(font_bytes, &glyph_set)?;

        // 2. Read font metrics via read-fonts
        let font_ref = FontRef::from_index(font_bytes, 0).map_err(|e| {
            PdfError::InvalidOperation(format!("Failed to parse font for embedding: {}", e))
        })?;

        let head = font_ref
            .head()
            .map_err(|e| PdfError::InvalidOperation(format!("Missing head table: {}", e)))?;
        let hhea = font_ref
            .hhea()
            .map_err(|e| PdfError::InvalidOperation(format!("Missing hhea table: {}", e)))?;
        let units_per_em = head.units_per_em() as f64;
        let scale = 1000.0 / units_per_em;

        let ascent = (hhea.ascender().to_i16() as f64) * scale;
        let descent = (hhea.descender().to_i16() as f64) * scale;
        let bbox = vec![
            PdfObject::Real((head.x_min() as f64) * scale),
            PdfObject::Real((head.y_min() as f64) * scale),
            PdfObject::Real((head.x_max() as f64) * scale),
            PdfObject::Real((head.y_max() as f64) * scale),
        ];

        let base_font_name = format!("StarPDF+{}", font_name.replace(' ', "-"));

        // 3. Create FontFile2 stream object
        let font_file_ref = Self::allocate_ref(next_alloc_num);
        let mut font_file_dict = BTreeMap::new();
        font_file_dict.insert(
            "Length1".into(),
            PdfObject::Integer(subset.bytes.len() as i64),
        );
        font_file_dict.insert(
            "Length".into(),
            PdfObject::Integer(subset.bytes.len() as i64),
        );
        modified.insert(
            font_file_ref,
            PdfObject::Stream(StreamObject {
                dict: font_file_dict,
                data: subset.bytes.clone(),
                stream_offset: 0,
                stream_length: subset.bytes.len(),
            }),
        );

        // 4. Create FontDescriptor dictionary object
        let descriptor_ref = Self::allocate_ref(next_alloc_num);
        let mut descriptor_dict = BTreeMap::new();
        descriptor_dict.insert("Type".into(), PdfObject::Name("FontDescriptor".into()));
        descriptor_dict.insert("FontName".into(), PdfObject::Name(base_font_name.clone()));
        descriptor_dict.insert("Flags".into(), PdfObject::Integer(4)); // Symbolic
        descriptor_dict.insert("FontBBox".into(), PdfObject::Array(bbox));
        descriptor_dict.insert("ItalicAngle".into(), PdfObject::Integer(0));
        descriptor_dict.insert("Ascent".into(), PdfObject::Real(ascent));
        descriptor_dict.insert("Descent".into(), PdfObject::Real(descent));
        descriptor_dict.insert("CapHeight".into(), PdfObject::Real(ascent * 0.8));
        descriptor_dict.insert("StemV".into(), PdfObject::Integer(80));
        descriptor_dict.insert("FontFile2".into(), PdfObject::Reference(font_file_ref));
        modified.insert(descriptor_ref, PdfObject::Dictionary(descriptor_dict));

        // 5. Build CIDFontType2 /W widths array
        let mut w_array = Vec::new();
        for (cid, width) in glyph_widths {
            w_array.push(PdfObject::Integer(*cid as i64));
            w_array.push(PdfObject::Array(vec![PdfObject::Real(*width)]));
        }

        // 6. Create an occurrence-specific CIDToGIDMap. This keeps visual glyph order and exact
        // logical Unicode extraction independent, including repeated glyphs in complex scripts.
        let cid_to_gid_ref = Self::allocate_ref(next_alloc_num);
        let max_cid = cid_to_gid.keys().copied().max().unwrap_or(0) as usize;
        let mut cid_to_gid_bytes = vec![0u8; (max_cid + 1).saturating_mul(2)];
        for (cid, gid) in cid_to_gid {
            let offset = usize::from(*cid).saturating_mul(2);
            cid_to_gid_bytes[offset..offset + 2].copy_from_slice(&gid.to_be_bytes());
        }
        let mut cid_to_gid_dict = BTreeMap::new();
        cid_to_gid_dict.insert(
            "Length".into(),
            PdfObject::Integer(cid_to_gid_bytes.len() as i64),
        );
        modified.insert(
            cid_to_gid_ref,
            PdfObject::Stream(StreamObject {
                dict: cid_to_gid_dict,
                data: cid_to_gid_bytes.clone(),
                stream_offset: 0,
                stream_length: cid_to_gid_bytes.len(),
            }),
        );

        // 7. Create CIDFontType2 dictionary object
        let cid_font_ref = Self::allocate_ref(next_alloc_num);
        let mut cid_sys_info = BTreeMap::new();
        cid_sys_info.insert("Registry".into(), PdfObject::String(b"Adobe".to_vec()));
        cid_sys_info.insert("Ordering".into(), PdfObject::String(b"Identity".to_vec()));
        cid_sys_info.insert("Supplement".into(), PdfObject::Integer(0));

        let mut cid_font_dict = BTreeMap::new();
        cid_font_dict.insert("Type".into(), PdfObject::Name("Font".into()));
        cid_font_dict.insert("Subtype".into(), PdfObject::Name("CIDFontType2".into()));
        cid_font_dict.insert("BaseFont".into(), PdfObject::Name(base_font_name.clone()));
        cid_font_dict.insert("CIDSystemInfo".into(), PdfObject::Dictionary(cid_sys_info));
        cid_font_dict.insert(
            "FontDescriptor".into(),
            PdfObject::Reference(descriptor_ref),
        );
        cid_font_dict.insert("DW".into(), PdfObject::Integer(1000));
        if !w_array.is_empty() {
            cid_font_dict.insert("W".into(), PdfObject::Array(w_array));
        }
        cid_font_dict.insert("CIDToGIDMap".into(), PdfObject::Reference(cid_to_gid_ref));
        modified.insert(cid_font_ref, PdfObject::Dictionary(cid_font_dict));

        // 8. Create ToUnicode CMap stream object
        let tounicode_ref = Self::allocate_ref(next_alloc_num);
        let tounicode_bytes = Self::build_tounicode_cmap(cid_to_unicode, font_name);
        let mut tounicode_dict = BTreeMap::new();
        tounicode_dict.insert(
            "Length".into(),
            PdfObject::Integer(tounicode_bytes.len() as i64),
        );
        modified.insert(
            tounicode_ref,
            PdfObject::Stream(StreamObject {
                dict: tounicode_dict,
                data: tounicode_bytes.clone(),
                stream_offset: 0,
                stream_length: tounicode_bytes.len(),
            }),
        );

        // 9. Create Type0 composite font dictionary object
        let type0_ref = Self::allocate_ref(next_alloc_num);
        let mut type0_dict = BTreeMap::new();
        type0_dict.insert("Type".into(), PdfObject::Name("Font".into()));
        type0_dict.insert("Subtype".into(), PdfObject::Name("Type0".into()));
        type0_dict.insert("BaseFont".into(), PdfObject::Name(base_font_name));
        type0_dict.insert("Encoding".into(), PdfObject::Name("Identity-H".into()));
        type0_dict.insert(
            "DescendantFonts".into(),
            PdfObject::Array(vec![PdfObject::Reference(cid_font_ref)]),
        );
        type0_dict.insert("ToUnicode".into(), PdfObject::Reference(tounicode_ref));
        modified.insert(type0_ref, PdfObject::Dictionary(type0_dict));

        // 10. Register the Type0 font in the target page's /Resources /Font dictionary
        let res_tag = format!("F_StarPDF_{}", font_name.replace([' ', '-'], ""));
        let page_obj = match modified.get(&page_ref) {
            Some(obj) => obj.clone(),
            None => store.resolve(page_ref)?.clone(),
        };
        let mut page_dict = page_obj
            .as_dict()
            .cloned()
            .ok_or_else(|| PdfError::TypeMismatch {
                expected: "dictionary",
                actual: page_obj.type_name(),
            })?;

        let mut res_dict = Self::inherited_resources(store, &page_dict)?;

        let mut font_dict = match res_dict.get("Font") {
            Some(PdfObject::Dictionary(d)) => d.clone(),
            Some(PdfObject::Reference(r)) => {
                let r_obj = match modified.get(r) {
                    Some(obj) => obj.clone(),
                    None => store.resolve(*r)?.clone(),
                };
                r_obj.as_dict().cloned().unwrap_or_default()
            }
            _ => BTreeMap::new(),
        };

        font_dict.insert(res_tag.clone(), PdfObject::Reference(type0_ref));
        res_dict.insert("Font".into(), PdfObject::Dictionary(font_dict));
        page_dict.insert("Resources".into(), PdfObject::Dictionary(res_dict));

        modified.insert(page_ref, PdfObject::Dictionary(page_dict));

        Ok(res_tag)
    }
}
