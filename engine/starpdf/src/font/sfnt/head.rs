use crate::error::{PdfError, PdfResult};
use crate::font::sfnt::table::{read_i16_be, read_u16_be};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeadTable {
    pub units_per_em: u16,
    pub index_to_loc_format: i16,
}

impl HeadTable {
    pub fn parse(data: &[u8]) -> PdfResult<Self> {
        if data.len() < 54 {
            return Err(PdfError::InvalidSyntax("head table too short".into()));
        }

        let units_per_em = read_u16_be(data, 18).unwrap_or(1000);
        let index_to_loc_format = read_i16_be(data, 50).unwrap_or(0);

        Ok(Self {
            units_per_em: if units_per_em == 0 {
                1000
            } else {
                units_per_em
            },
            index_to_loc_format,
        })
    }
}
