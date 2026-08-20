use std::collections::BTreeMap;

use crate::error::{PdfError, PdfResult};

#[inline]
pub fn read_u16_be(data: &[u8], offset: usize) -> Option<u16> {
    if offset.checked_add(2)? <= data.len() {
        Some(u16::from_be_bytes([data[offset], data[offset + 1]]))
    } else {
        None
    }
}

#[inline]
pub fn read_i16_be(data: &[u8], offset: usize) -> Option<i16> {
    if offset.checked_add(2)? <= data.len() {
        Some(i16::from_be_bytes([data[offset], data[offset + 1]]))
    } else {
        None
    }
}

#[inline]
pub fn read_u32_be(data: &[u8], offset: usize) -> Option<u32> {
    if offset.checked_add(4)? <= data.len() {
        Some(u32::from_be_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]))
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableRecord {
    pub tag: [u8; 4],
    pub checksum: u32,
    pub offset: usize,
    pub length: usize,
}

#[derive(Debug, Clone)]
pub struct TableDirectory {
    pub tables: BTreeMap<[u8; 4], TableRecord>,
}

impl TableDirectory {
    pub fn parse(data: &[u8]) -> PdfResult<Self> {
        if data.len() < 12 {
            return Err(PdfError::InvalidSyntax("SFNT data too small".into()));
        }

        let num_tables = read_u16_be(data, 4)
            .ok_or_else(|| PdfError::InvalidSyntax("Failed to read numTables".into()))?
            as usize;

        const MAX_TABLES: usize = 64;
        if num_tables > MAX_TABLES {
            return Err(PdfError::InvalidSyntax("Too many SFNT tables".into()));
        }

        let mut tables = BTreeMap::new();
        let directory_len = num_tables
            .checked_mul(16)
            .and_then(|value| value.checked_add(12))
            .ok_or_else(|| PdfError::InvalidSyntax("SFNT table directory overflow".into()))?;
        if directory_len > data.len() {
            return Err(PdfError::InvalidSyntax(
                "Truncated SFNT table directory".into(),
            ));
        }
        let mut curr_offset = 12usize;

        for _ in 0..num_tables {
            let mut tag = [0u8; 4];
            tag.copy_from_slice(&data[curr_offset..curr_offset + 4]);
            let checksum = read_u32_be(data, curr_offset + 4)
                .ok_or_else(|| PdfError::InvalidSyntax("Truncated SFNT checksum".into()))?;
            let offset = read_u32_be(data, curr_offset + 8)
                .ok_or_else(|| PdfError::InvalidSyntax("Truncated SFNT table offset".into()))?
                as usize;
            let length = read_u32_be(data, curr_offset + 12)
                .ok_or_else(|| PdfError::InvalidSyntax("Truncated SFNT table length".into()))?
                as usize;

            // Bounds check table record against total font data length
            let end = offset
                .checked_add(length)
                .ok_or_else(|| PdfError::InvalidSyntax("SFNT table range overflow".into()))?;
            if end > data.len() {
                return Err(PdfError::InvalidSyntax(format!(
                    "SFNT table {:?} extends beyond font data",
                    String::from_utf8_lossy(&tag)
                )));
            }
            tables.insert(
                tag,
                TableRecord {
                    tag,
                    checksum,
                    offset,
                    length,
                },
            );

            curr_offset = curr_offset
                .checked_add(16)
                .ok_or_else(|| PdfError::InvalidSyntax("SFNT directory offset overflow".into()))?;
        }

        Ok(Self { tables })
    }

    pub fn get_table<'a>(&self, tag: &[u8; 4], data: &'a [u8]) -> Option<&'a [u8]> {
        let record = self.tables.get(tag)?;
        let end = record.offset.checked_add(record.length)?;
        if end <= data.len() {
            Some(&data[record.offset..end])
        } else {
            None
        }
    }
}
