use std::collections::BTreeMap;

use crate::error::{PdfError, PdfResult};
use crate::font::sfnt::table::{read_i16_be, read_u16_be, read_u32_be};

#[derive(Debug, Clone, Default)]
pub struct SfntCmapTable {
    pub char_to_glyph: BTreeMap<u32, u16>,
    pub glyph_to_char: BTreeMap<u16, char>,
}

impl SfntCmapTable {
    pub fn parse(data: &[u8]) -> PdfResult<Self> {
        if data.len() < 4 {
            return Err(PdfError::InvalidSyntax("cmap table too short".into()));
        }

        let num_tables = read_u16_be(data, 2).unwrap_or(0) as usize;
        let mut best_offset = None;
        let mut best_priority = 0;

        for i in 0..num_tables {
            let record_offset = 4 + i * 8;
            if record_offset + 8 > data.len() {
                break;
            }

            let platform_id = read_u16_be(data, record_offset).unwrap_or(0);
            let encoding_id = read_u16_be(data, record_offset + 2).unwrap_or(0);
            let subtable_offset = read_u32_be(data, record_offset + 4).unwrap_or(0) as usize;

            let priority = match (platform_id, encoding_id) {
                (3, 10) => 10, // Windows Unicode full repertoire (UCS-4)
                (3, 1) => 9,   // Windows Unicode BMP (UCS-2)
                (0, 4) => 8,   // Unicode 2.0+ full repertoire
                (0, 3) => 7,   // Unicode 2.0+ BMP
                (0, _) => 6,   // Unicode platform generic
                (3, 0) => 5,   // Windows Symbol
                (1, 0) => 4,   // Mac Roman
                _ => 1,
            };

            if priority > best_priority && subtable_offset < data.len() {
                best_priority = priority;
                best_offset = Some(subtable_offset);
            }
        }

        let mut cmap = Self::default();
        if let Some(offset) = best_offset {
            let _ = Self::parse_subtable(data, offset, &mut cmap);
        }

        Ok(cmap)
    }

    fn parse_subtable(data: &[u8], offset: usize, cmap: &mut Self) -> PdfResult<()> {
        if offset + 2 > data.len() {
            return Ok(());
        }

        let format = read_u16_be(data, offset).unwrap_or(0);
        match format {
            0 => Self::parse_format_0(data, offset, cmap),
            4 => Self::parse_format_4(data, offset, cmap),
            6 => Self::parse_format_6(data, offset, cmap),
            12 => Self::parse_format_12(data, offset, cmap),
            _ => Ok(()),
        }
    }

    fn parse_format_0(data: &[u8], offset: usize, cmap: &mut Self) -> PdfResult<()> {
        if offset + 262 > data.len() {
            return Ok(());
        }
        for code in 0u32..=255 {
            let glyph_id = data[offset + 6 + code as usize] as u16;
            if glyph_id != 0 {
                cmap.char_to_glyph.insert(code, glyph_id);
                if let Some(ch) = char::from_u32(code) {
                    cmap.glyph_to_char.insert(glyph_id, ch);
                }
            }
        }
        Ok(())
    }

    fn parse_format_6(data: &[u8], offset: usize, cmap: &mut Self) -> PdfResult<()> {
        if offset + 10 > data.len() {
            return Ok(());
        }
        let first_code = read_u16_be(data, offset + 6).unwrap_or(0) as u32;
        let entry_count = read_u16_be(data, offset + 8).unwrap_or(0) as usize;
        let mut curr = offset + 10;
        for i in 0..entry_count {
            if curr + 2 > data.len() {
                break;
            }
            let glyph_id = read_u16_be(data, curr).unwrap_or(0);
            if glyph_id != 0 {
                let code = first_code.saturating_add(i as u32);
                cmap.char_to_glyph.insert(code, glyph_id);
                if let Some(ch) = char::from_u32(code) {
                    cmap.glyph_to_char.insert(glyph_id, ch);
                }
            }
            curr += 2;
        }
        Ok(())
    }

    fn parse_format_4(data: &[u8], offset: usize, cmap: &mut Self) -> PdfResult<()> {
        if offset + 14 > data.len() {
            return Ok(());
        }

        let seg_count_x2 = read_u16_be(data, offset + 6).unwrap_or(0) as usize;
        let seg_count = seg_count_x2 / 2;
        const MAX_SEGS: usize = 4096;
        if seg_count == 0 || seg_count > MAX_SEGS {
            return Ok(());
        }

        let end_code_offset = offset + 14;
        let start_code_offset = end_code_offset + seg_count * 2 + 2; // +2 for reservedPad
        let id_delta_offset = start_code_offset + seg_count * 2;
        let id_range_offset = id_delta_offset + seg_count * 2;

        if id_range_offset + seg_count * 2 > data.len() {
            return Ok(());
        }

        let mut end_codes = Vec::with_capacity(seg_count);
        for i in 0..seg_count {
            end_codes.push(read_u16_be(data, end_code_offset + i * 2).unwrap_or(0));
        }

        let mut start_codes = Vec::with_capacity(seg_count);
        for i in 0..seg_count {
            start_codes.push(read_u16_be(data, start_code_offset + i * 2).unwrap_or(0));
        }

        let mut id_deltas = Vec::with_capacity(seg_count);
        for i in 0..seg_count {
            id_deltas.push(read_i16_be(data, id_delta_offset + i * 2).unwrap_or(0));
        }

        for i in 0..seg_count {
            let start = start_codes[i];
            let end = end_codes[i];
            let delta = id_deltas[i];
            let range_offset_pos = id_range_offset + i * 2;
            let range_offset = read_u16_be(data, range_offset_pos).unwrap_or(0) as usize;

            if start == 0xFFFF || end < start {
                continue;
            }

            for c in start..=end {
                let glyph_id: u16 = if range_offset == 0 {
                    (c as i32 + delta as i32) as u16
                } else {
                    let glyph_offset = range_offset_pos + range_offset + ((c - start) as usize * 2);
                    if glyph_offset + 2 <= data.len() {
                        let raw_id = read_u16_be(data, glyph_offset).unwrap_or(0);
                        if raw_id != 0 {
                            (raw_id as i32 + delta as i32) as u16
                        } else {
                            0
                        }
                    } else {
                        0
                    }
                };

                if glyph_id != 0 {
                    let char_code = c as u32;
                    cmap.char_to_glyph.insert(char_code, glyph_id);
                    if let Some(ch) = char::from_u32(char_code) {
                        cmap.glyph_to_char.insert(glyph_id, ch);
                    }
                }
            }
        }

        Ok(())
    }

    fn parse_format_12(data: &[u8], offset: usize, cmap: &mut Self) -> PdfResult<()> {
        if offset + 16 > data.len() {
            return Ok(());
        }

        let num_groups = read_u32_be(data, offset + 12).unwrap_or(0) as usize;
        const MAX_GROUPS: usize = 65_536;
        if num_groups > MAX_GROUPS {
            return Ok(());
        }

        let mut curr = offset + 16;
        for _ in 0..num_groups {
            if curr + 12 > data.len() {
                break;
            }

            let start_char = read_u32_be(data, curr).unwrap_or(0);
            let end_char = read_u32_be(data, curr + 4).unwrap_or(0);
            let mut start_glyph = read_u32_be(data, curr + 8).unwrap_or(0);

            if end_char >= start_char && (end_char - start_char) < 65_536 {
                for c in start_char..=end_char {
                    let gid = start_glyph as u16;
                    cmap.char_to_glyph.insert(c, gid);
                    if let Some(ch) = char::from_u32(c) {
                        cmap.glyph_to_char.insert(gid, ch);
                    }
                    start_glyph = start_glyph.saturating_add(1);
                }
            }

            curr += 12;
        }

        Ok(())
    }

    pub fn map_char_to_glyph(&self, char_code: u32) -> Option<u16> {
        self.char_to_glyph.get(&char_code).copied()
    }

    pub fn map_glyph_to_char(&self, glyph_id: u16) -> Option<char> {
        self.glyph_to_char.get(&glyph_id).copied()
    }
}
