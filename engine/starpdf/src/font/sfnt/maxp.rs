use crate::error::{PdfError, PdfResult};
use crate::font::sfnt::table::read_u16_be;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaxpTable {
    pub num_glyphs: u16,
}

impl MaxpTable {
    pub fn parse(data: &[u8]) -> PdfResult<Self> {
        if data.len() < 6 {
            return Err(PdfError::InvalidSyntax("maxp table too short".into()));
        }

        let num_glyphs = read_u16_be(data, 4).unwrap_or(0);
        Ok(Self { num_glyphs })
    }
}
