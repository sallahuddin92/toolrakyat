use crate::error::PdfResult;
use crate::font::sfnt::table::read_u16_be;

#[derive(Debug, Clone)]
pub struct HmtxTable {
    pub advance_widths: Vec<u16>,
    pub default_advance: u16,
}

impl HmtxTable {
    pub fn parse(data: &[u8], num_metrics: u16, _num_glyphs: u16) -> PdfResult<Self> {
        let count = num_metrics as usize;
        let mut advance_widths = Vec::with_capacity(count);

        for i in 0..count {
            let offset = i * 4;
            if offset + 2 <= data.len() {
                let adv = read_u16_be(data, offset).unwrap_or(1000);
                advance_widths.push(adv);
            } else {
                break;
            }
        }

        let default_advance = advance_widths.last().copied().unwrap_or(1000);

        Ok(Self {
            advance_widths,
            default_advance,
        })
    }

    pub fn get_advance_width(&self, glyph_id: u16) -> u16 {
        let idx = glyph_id as usize;
        if idx < self.advance_widths.len() {
            self.advance_widths[idx]
        } else {
            self.default_advance
        }
    }
}
