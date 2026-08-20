use crate::error::{PdfError, PdfResult};
use crate::font::sfnt::table::{read_i16_be, read_u16_be};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HheaTable {
    pub ascender: i16,
    pub descender: i16,
    pub line_gap: i16,
    pub number_of_h_metrics: u16,
}

impl HheaTable {
    pub fn parse(data: &[u8]) -> PdfResult<Self> {
        if data.len() < 36 {
            return Err(PdfError::InvalidSyntax("hhea table too short".into()));
        }

        let ascender = read_i16_be(data, 4).unwrap_or(0);
        let descender = read_i16_be(data, 6).unwrap_or(0);
        let line_gap = read_i16_be(data, 8).unwrap_or(0);
        let number_of_h_metrics = read_u16_be(data, 34).unwrap_or(0);

        Ok(Self {
            ascender,
            descender,
            line_gap,
            number_of_h_metrics,
        })
    }
}
