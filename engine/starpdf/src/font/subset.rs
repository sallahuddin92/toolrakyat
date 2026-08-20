use std::collections::BTreeSet;

use crate::error::{PdfError, PdfResult};
use crate::font::appearance::MAX_EMBEDDED_FONT_BYTES;
use crate::font::sfnt::table::{read_i16_be, read_u16_be, read_u32_be, TableDirectory};
use crate::font::SfntFont;

pub const MAX_SUBSET_GLYPHS: usize = 4_096;
pub const MAX_SUBSET_FONT_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_SOURCE_GLYPHS: usize = 65_535;
const MAX_COMPOSITE_DEPTH: usize = 32;

#[derive(Debug, Clone)]
pub struct FontSubset {
    pub bytes: Vec<u8>,
    pub glyph_ids: Vec<u16>,
}

pub struct TrueTypeSubsetter;

impl TrueTypeSubsetter {
    /// Builds a valid TrueType subset while preserving original glyph IDs. Unselected
    /// `glyf` entries become empty and all non-glyph tables remain byte-for-byte stable.
    pub fn subset(data: &[u8], requested_glyphs: &[u16]) -> PdfResult<FontSubset> {
        if data.len() > MAX_EMBEDDED_FONT_BYTES {
            return Err(PdfError::InvalidOperation(format!(
                "Embedded font exceeds maximum of {MAX_EMBEDDED_FONT_BYTES} bytes"
            )));
        }
        if requested_glyphs.len() > MAX_SUBSET_GLYPHS {
            return Err(PdfError::InvalidOperation(format!(
                "Font subset exceeds maximum of {MAX_SUBSET_GLYPHS} requested glyphs"
            )));
        }
        let directory = TableDirectory::parse(data)?;
        let head = directory.get_table(b"head", data).ok_or_else(|| {
            PdfError::InvalidOperation("TrueType font is missing head table".into())
        })?;
        let maxp = directory.get_table(b"maxp", data).ok_or_else(|| {
            PdfError::InvalidOperation("TrueType font is missing maxp table".into())
        })?;
        let loca = directory.get_table(b"loca", data).ok_or_else(|| {
            PdfError::InvalidOperation(
                "Unsupported CFF/OpenType font: bounded subsetting requires TrueType loca/glyf"
                    .into(),
            )
        })?;
        let glyf = directory.get_table(b"glyf", data).ok_or_else(|| {
            PdfError::InvalidOperation(
                "Unsupported CFF/OpenType font: bounded subsetting requires TrueType loca/glyf"
                    .into(),
            )
        })?;
        let num_glyphs = read_u16_be(maxp, 4)
            .ok_or_else(|| PdfError::InvalidSyntax("Truncated maxp glyph count".into()))?
            as usize;
        if num_glyphs == 0 || num_glyphs > MAX_SOURCE_GLYPHS {
            return Err(PdfError::InvalidOperation(
                "TrueType source glyph count is outside the supported range".into(),
            ));
        }
        let loca_format = read_i16_be(head, 50)
            .ok_or_else(|| PdfError::InvalidSyntax("Truncated head indexToLocFormat".into()))?;
        if loca_format != 0 && loca_format != 1 {
            return Err(PdfError::InvalidSyntax(
                "Invalid TrueType indexToLocFormat".into(),
            ));
        }
        let offsets = Self::parse_loca(loca, loca_format, num_glyphs, glyf.len())?;
        let mut selected = BTreeSet::from([0u16]);
        for glyph in requested_glyphs {
            if usize::from(*glyph) >= num_glyphs {
                return Err(PdfError::InvalidOperation(format!(
                    "Impossible glyph ID {glyph} for font with {num_glyphs} glyphs"
                )));
            }
            selected.insert(*glyph);
        }
        Self::include_composite_dependencies(glyf, &offsets, num_glyphs, &mut selected)?;
        if selected.len() > MAX_SUBSET_GLYPHS {
            return Err(PdfError::InvalidOperation(format!(
                "Composite glyph closure exceeds maximum of {MAX_SUBSET_GLYPHS} glyphs"
            )));
        }

        let mut new_glyf = Vec::new();
        let mut new_loca = Vec::with_capacity((num_glyphs + 1).saturating_mul(4));
        for glyph_index in 0..num_glyphs {
            new_loca.extend_from_slice(&Self::usize_to_u32(new_glyf.len())?.to_be_bytes());
            if selected.contains(&(glyph_index as u16)) {
                let start = offsets[glyph_index];
                let end = offsets[glyph_index + 1];
                new_glyf.extend_from_slice(&glyf[start..end]);
                while new_glyf.len() % 4 != 0 {
                    new_glyf.push(0);
                }
            }
        }
        new_loca.extend_from_slice(&Self::usize_to_u32(new_glyf.len())?.to_be_bytes());

        let mut tables = Vec::with_capacity(directory.tables.len());
        for (tag, record) in &directory.tables {
            let bytes = match tag {
                b"glyf" => new_glyf.clone(),
                b"loca" => new_loca.clone(),
                b"head" => {
                    let mut bytes = data[record.offset..record.offset + record.length].to_vec();
                    if bytes.len() < 54 {
                        return Err(PdfError::InvalidSyntax("Truncated head table".into()));
                    }
                    bytes[8..12].fill(0);
                    bytes[50..52].copy_from_slice(&1i16.to_be_bytes());
                    bytes
                }
                _ => data[record.offset..record.offset + record.length].to_vec(),
            };
            tables.push((*tag, bytes));
        }
        let output = Self::build_sfnt(data, &tables)?;
        if output.len() > MAX_SUBSET_FONT_BYTES {
            return Err(PdfError::InvalidOperation(format!(
                "Subset font exceeds maximum of {MAX_SUBSET_FONT_BYTES} bytes"
            )));
        }
        SfntFont::parse(&output)?;
        Ok(FontSubset {
            bytes: output,
            glyph_ids: selected.into_iter().collect(),
        })
    }

    fn parse_loca(
        loca: &[u8],
        format: i16,
        num_glyphs: usize,
        glyf_len: usize,
    ) -> PdfResult<Vec<usize>> {
        let count = num_glyphs
            .checked_add(1)
            .ok_or_else(|| PdfError::InvalidSyntax("loca count overflow".into()))?;
        let entry_size = if format == 0 { 2 } else { 4 };
        let required = count
            .checked_mul(entry_size)
            .ok_or_else(|| PdfError::InvalidSyntax("loca byte length overflow".into()))?;
        if required > loca.len() {
            return Err(PdfError::InvalidSyntax("Truncated loca table".into()));
        }
        let mut offsets = Vec::with_capacity(count);
        for index in 0..count {
            let position = index
                .checked_mul(entry_size)
                .ok_or_else(|| PdfError::InvalidSyntax("loca offset overflow".into()))?;
            let offset = if format == 0 {
                usize::from(
                    read_u16_be(loca, position)
                        .ok_or_else(|| PdfError::InvalidSyntax("Truncated short loca".into()))?,
                ) * 2
            } else {
                read_u32_be(loca, position)
                    .ok_or_else(|| PdfError::InvalidSyntax("Truncated long loca".into()))?
                    as usize
            };
            if offset > glyf_len || offsets.last().is_some_and(|previous| *previous > offset) {
                return Err(PdfError::InvalidSyntax(
                    "loca offsets are out of bounds or non-monotonic".into(),
                ));
            }
            offsets.push(offset);
        }
        Ok(offsets)
    }

    fn include_composite_dependencies(
        glyf: &[u8],
        offsets: &[usize],
        num_glyphs: usize,
        selected: &mut BTreeSet<u16>,
    ) -> PdfResult<()> {
        let roots: Vec<u16> = selected.iter().copied().collect();
        let mut visiting = BTreeSet::new();
        let mut visited = BTreeSet::new();
        for root in roots {
            let mut stack = vec![(root, 0usize, false)];
            while let Some((glyph, depth, exiting)) = stack.pop() {
                if exiting {
                    visiting.remove(&glyph);
                    visited.insert(glyph);
                    continue;
                }
                if visited.contains(&glyph) {
                    continue;
                }
                if !visiting.insert(glyph) {
                    return Err(PdfError::CircularReference(
                        "Cycle in TrueType composite glyph dependencies".into(),
                    ));
                }
                if depth > MAX_COMPOSITE_DEPTH {
                    return Err(PdfError::RecursionLimitExceeded);
                }
                selected.insert(glyph);
                if selected.len() > MAX_SUBSET_GLYPHS {
                    return Err(PdfError::InvalidOperation(
                        "Composite glyph closure exceeds subset limit".into(),
                    ));
                }
                stack.push((glyph, depth, true));
                let dependencies = Self::composite_dependencies(glyf, offsets, num_glyphs, glyph)?;
                for component in dependencies.into_iter().rev() {
                    if visiting.contains(&component) {
                        return Err(PdfError::CircularReference(
                            "Cycle in TrueType composite glyph dependencies".into(),
                        ));
                    }
                    stack.push((component, depth.saturating_add(1), false));
                }
            }
        }
        Ok(())
    }

    fn composite_dependencies(
        glyf: &[u8],
        offsets: &[usize],
        num_glyphs: usize,
        glyph: u16,
    ) -> PdfResult<Vec<u16>> {
        let start = offsets[usize::from(glyph)];
        let end = offsets[usize::from(glyph) + 1];
        let bytes = &glyf[start..end];
        if bytes.len() < 10 || read_i16_be(bytes, 0).unwrap_or(0) >= 0 {
            return Ok(Vec::new());
        }
        let mut dependencies = Vec::new();
        let mut position = 10usize;
        loop {
            let flags = read_u16_be(bytes, position)
                .ok_or_else(|| PdfError::InvalidSyntax("Truncated composite glyph flags".into()))?;
            let component = read_u16_be(bytes, position + 2)
                .ok_or_else(|| PdfError::InvalidSyntax("Truncated composite glyph ID".into()))?;
            if usize::from(component) >= num_glyphs {
                return Err(PdfError::InvalidSyntax(
                    "Composite glyph references impossible glyph ID".into(),
                ));
            }
            dependencies.push(component);
            let args = if flags & 0x0001 != 0 { 4 } else { 2 };
            let transform = if flags & 0x0008 != 0 {
                2
            } else if flags & 0x0040 != 0 {
                4
            } else if flags & 0x0080 != 0 {
                8
            } else {
                0
            };
            position = position
                .checked_add(4 + args + transform)
                .ok_or_else(|| PdfError::InvalidSyntax("Composite offset overflow".into()))?;
            if position > bytes.len() {
                return Err(PdfError::InvalidSyntax("Truncated composite glyph".into()));
            }
            if flags & 0x0020 == 0 {
                break;
            }
        }
        Ok(dependencies)
    }

    fn build_sfnt(original: &[u8], tables: &[([u8; 4], Vec<u8>)]) -> PdfResult<Vec<u8>> {
        let num_tables = u16::try_from(tables.len())
            .map_err(|_| PdfError::InvalidOperation("Too many output font tables".into()))?;
        let header_len = 12usize
            .checked_add(tables.len().checked_mul(16).ok_or_else(|| {
                PdfError::InvalidOperation("Subset table directory overflow".into())
            })?)
            .ok_or_else(|| PdfError::InvalidOperation("Subset header overflow".into()))?;
        let mut output = vec![0u8; header_len];
        output[0..4].copy_from_slice(&original[0..4]);
        output[4..6].copy_from_slice(&num_tables.to_be_bytes());
        let max_power = if num_tables == 0 {
            0
        } else {
            1u16 << (15 - num_tables.leading_zeros() as u16)
        };
        let search_range = max_power.saturating_mul(16);
        let entry_selector = if max_power == 0 {
            0
        } else {
            max_power.trailing_zeros() as u16
        };
        let range_shift = num_tables.saturating_mul(16).saturating_sub(search_range);
        output[6..8].copy_from_slice(&search_range.to_be_bytes());
        output[8..10].copy_from_slice(&entry_selector.to_be_bytes());
        output[10..12].copy_from_slice(&range_shift.to_be_bytes());

        let mut head_offset = None;
        for (index, (tag, bytes)) in tables.iter().enumerate() {
            while output.len() % 4 != 0 {
                output.push(0);
            }
            let offset = output.len();
            let record = 12 + index * 16;
            output[record..record + 4].copy_from_slice(tag);
            output[record + 4..record + 8].copy_from_slice(&Self::checksum(bytes).to_be_bytes());
            output[record + 8..record + 12]
                .copy_from_slice(&Self::usize_to_u32(offset)?.to_be_bytes());
            output[record + 12..record + 16]
                .copy_from_slice(&Self::usize_to_u32(bytes.len())?.to_be_bytes());
            if tag == b"head" {
                head_offset = Some(offset);
            }
            output.extend_from_slice(bytes);
        }
        while output.len() % 4 != 0 {
            output.push(0);
        }
        let head_offset = head_offset.ok_or_else(|| {
            PdfError::InvalidOperation("Subset output is missing head table".into())
        })?;
        let adjustment_position = head_offset
            .checked_add(8)
            .ok_or_else(|| PdfError::InvalidOperation("head checksum offset overflow".into()))?;
        let adjustment = 0xB1B0_AFBAu32.wrapping_sub(Self::checksum(&output));
        output[adjustment_position..adjustment_position + 4]
            .copy_from_slice(&adjustment.to_be_bytes());
        Ok(output)
    }

    fn checksum(bytes: &[u8]) -> u32 {
        let mut sum = 0u32;
        for chunk in bytes.chunks(4) {
            let mut word = [0u8; 4];
            word[..chunk.len()].copy_from_slice(chunk);
            sum = sum.wrapping_add(u32::from_be_bytes(word));
        }
        sum
    }

    fn usize_to_u32(value: usize) -> PdfResult<u32> {
        u32::try_from(value)
            .map_err(|_| PdfError::InvalidOperation("Subset font offset exceeds u32".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_glyph() -> Vec<u8> {
        vec![0; 10]
    }

    fn composite_glyph(component: u16) -> Vec<u8> {
        let mut bytes = Vec::from((-1i16).to_be_bytes());
        bytes.extend_from_slice(&[0; 8]);
        bytes.extend_from_slice(&0u16.to_be_bytes());
        bytes.extend_from_slice(&component.to_be_bytes());
        bytes.extend_from_slice(&[0; 2]);
        bytes
    }

    #[test]
    fn composite_dependency_closure_is_transitive_and_bounded() {
        let glyphs = [simple_glyph(), composite_glyph(2), simple_glyph()];
        let mut glyf = Vec::new();
        let mut offsets = vec![0];
        for glyph in glyphs {
            glyf.extend_from_slice(&glyph);
            offsets.push(glyf.len());
        }
        let mut selected = BTreeSet::from([1]);
        TrueTypeSubsetter::include_composite_dependencies(&glyf, &offsets, 3, &mut selected)
            .unwrap_or_else(|error| panic!("composite closure failed: {error}"));
        assert_eq!(selected, BTreeSet::from([1, 2]));
    }

    #[test]
    fn composite_dependency_cycle_is_rejected() {
        let glyphs = [simple_glyph(), composite_glyph(2), composite_glyph(1)];
        let mut glyf = Vec::new();
        let mut offsets = vec![0];
        for glyph in glyphs {
            glyf.extend_from_slice(&glyph);
            offsets.push(glyf.len());
        }
        let mut selected = BTreeSet::from([1]);
        let error =
            TrueTypeSubsetter::include_composite_dependencies(&glyf, &offsets, 3, &mut selected)
                .unwrap_err();
        assert!(matches!(error, PdfError::CircularReference(_)));
    }
}
