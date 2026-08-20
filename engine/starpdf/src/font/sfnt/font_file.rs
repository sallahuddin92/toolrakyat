use crate::error::PdfResult;
use crate::font::sfnt::cmap::SfntCmapTable;
use crate::font::sfnt::head::HeadTable;
use crate::font::sfnt::hhea::HheaTable;
use crate::font::sfnt::hmtx::HmtxTable;
use crate::font::sfnt::maxp::MaxpTable;
use crate::font::sfnt::table::TableDirectory;

#[derive(Debug, Clone)]
pub struct SfntFont {
    pub head: Option<HeadTable>,
    pub maxp: Option<MaxpTable>,
    pub hhea: Option<HheaTable>,
    pub hmtx: Option<HmtxTable>,
    pub cmap: Option<SfntCmapTable>,
}

impl SfntFont {
    pub fn parse(data: &[u8]) -> PdfResult<Self> {
        let dir = TableDirectory::parse(data)?;

        let head = dir
            .get_table(b"head", data)
            .and_then(|t_data| HeadTable::parse(t_data).ok());

        let maxp = dir
            .get_table(b"maxp", data)
            .and_then(|t_data| MaxpTable::parse(t_data).ok());

        let hhea = dir
            .get_table(b"hhea", data)
            .and_then(|t_data| HheaTable::parse(t_data).ok());

        let hmtx = dir.get_table(b"hmtx", data).and_then(|t_data| {
            let num_metrics = hhea.map_or(0, |h| h.number_of_h_metrics);
            let num_glyphs = maxp.map_or(0, |m| m.num_glyphs);
            HmtxTable::parse(t_data, num_metrics, num_glyphs).ok()
        });

        let cmap = dir
            .get_table(b"cmap", data)
            .and_then(|t_data| SfntCmapTable::parse(t_data).ok());

        Ok(Self {
            head,
            maxp,
            hhea,
            hmtx,
            cmap,
        })
    }

    /// Resolves character code to Unicode char via embedded font cmap table.
    pub fn decode_char_code(&self, code: u32) -> Option<char> {
        if let Some(ref cmap_table) = self.cmap {
            // First check direct char code mapping
            if let Some(ch) = char::from_u32(code) {
                if cmap_table.map_char_to_glyph(code).is_some() {
                    return Some(ch);
                }
            }

            // Next check if code is a glyph ID
            if let Some(ch) = cmap_table.map_glyph_to_char(code as u16) {
                return Some(ch);
            }
        }
        None
    }

    /// Resolves advance width for a character code / glyph in standard 1/1000 font units.
    pub fn get_advance_width(&self, code: u32) -> Option<f64> {
        let hmtx = self.hmtx.as_ref()?;
        let upem = self.head.map_or(1000.0, |h| h.units_per_em as f64);

        let glyph_id = self
            .cmap
            .as_ref()
            .and_then(|c| c.map_char_to_glyph(code))
            .unwrap_or(code as u16);

        let raw_adv = hmtx.get_advance_width(glyph_id) as f64;
        Some((raw_adv / upem) * 1000.0)
    }
}
