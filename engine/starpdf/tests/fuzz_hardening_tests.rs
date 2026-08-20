use std::collections::BTreeMap;

use starpdf::content::ContentParser;
use starpdf::document::{ObjectStreamReader, PdfDocument};
use starpdf::filter::limits::DecompressLimits;
use starpdf::filter::{FlateDecoder, PredictorDecoder, PredictorParams};
use starpdf::font::sfnt::{
    HeadTable, HheaTable, HmtxTable, MaxpTable, SfntCmapTable, SfntFont, TableDirectory,
};
use starpdf::font::UnicodeCMap;
use starpdf::search::{SearchOptions, TextMatcher};
use starpdf::syntax::object::{PdfObject, StreamObject};
use starpdf::syntax::{Lexer, Parser};
use starpdf::text::{PageText, TextSpan};
use starpdf::xref::table::XrefTable;
use starpdf::xref::XrefStreamParser;

// Linear congruential generator for deterministic reproducible pseudo-random fuzz byte generation
struct PseudoRng {
    state: u64,
}

impl PseudoRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        (self.state >> 32) as u32
    }

    fn fill_bytes(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.next_u32() as u8;
        }
    }
}

#[test]
fn test_fuzz_lexer_hostile_inputs() {
    let mut rng = PseudoRng::new(0x1337_CAFE);
    let mut buf = [0u8; 256];

    for _ in 0..10_000 {
        rng.fill_bytes(&mut buf);
        let mut lexer = Lexer::from_bytes(&buf);
        while let Ok(Some(_)) = lexer.next_token() {
            // Must not panic
        }
    }
}

#[test]
fn test_fuzz_object_parser_hostile_inputs() {
    let mut rng = PseudoRng::new(0xDEAD_BEEF);
    let mut buf = [0u8; 512];

    for _ in 0..10_000 {
        rng.fill_bytes(&mut buf);
        let mut parser = Parser::from_bytes(&buf);
        let _ = parser.parse_object();
        let _ = parser.parse_indirect_object();
    }
}

#[test]
fn test_fuzz_flatedecode_decompression_bombs_and_corruptions() {
    let limits = DecompressLimits::default();
    let mut rng = PseudoRng::new(0xBAAD_F00D);
    let mut buf = [0u8; 128];

    for _ in 0..5_000 {
        rng.fill_bytes(&mut buf);
        let _ = FlateDecoder::decode(&buf, &limits);
    }
}

#[test]
fn test_fuzz_predictor_decoder_hostile_inputs() {
    let mut rng = PseudoRng::new(0xC001_D00D);
    let mut buf = [0u8; 256];

    for _ in 0..5_000 {
        rng.fill_bytes(&mut buf);
        let predictor_code = (rng.next_u32() % 20) as i32;
        let columns = (rng.next_u32() % 64 + 1) as usize;
        let colors = (rng.next_u32() % 8 + 1) as usize;
        let bits_per_component = match rng.next_u32() % 4 {
            0 => 8,
            1 => 16,
            2 => 4,
            _ => 1,
        };

        let params = PredictorParams {
            predictor: predictor_code,
            columns,
            colors,
            bits_per_component,
        };

        let _ = PredictorDecoder::decode(&buf, &params);
    }
}

#[test]
fn test_fuzz_xref_stream_parser_hostile_inputs() {
    let limits = DecompressLimits::default();
    let mut rng = PseudoRng::new(0x5EED_0001);

    for _ in 0..5_000 {
        let mut data = vec![0u8; (rng.next_u32() % 512 + 1) as usize];
        rng.fill_bytes(&mut data);

        let mut dict = BTreeMap::new();
        dict.insert("Type".into(), PdfObject::Name("XRef".into()));
        let size = (rng.next_u32() % 100_000) as i64;
        dict.insert("Size".into(), PdfObject::Integer(size));

        let w0 = (rng.next_u32() % 5) as i64;
        let w1 = (rng.next_u32() % 5) as i64;
        let w2 = (rng.next_u32() % 5) as i64;
        dict.insert(
            "W".into(),
            PdfObject::Array(vec![
                PdfObject::Integer(w0),
                PdfObject::Integer(w1),
                PdfObject::Integer(w2),
            ]),
        );

        let stream = StreamObject {
            dict,
            data,
            stream_offset: 0,
            stream_length: 512,
        };

        let mut table = XrefTable::new();
        let _ = XrefStreamParser::parse_into_table(&stream, &mut table, &limits);
    }
}

#[test]
fn test_fuzz_object_stream_hostile_inputs() {
    let limits = DecompressLimits::default();
    let mut rng = PseudoRng::new(0x5EED_0002);

    for _ in 0..5_000 {
        let mut data = vec![0u8; (rng.next_u32() % 512 + 1) as usize];
        rng.fill_bytes(&mut data);

        let mut dict = BTreeMap::new();
        dict.insert("Type".into(), PdfObject::Name("ObjStm".into()));
        let n = (rng.next_u32() % 1000) as i64;
        let first = (rng.next_u32() % 1000) as i64;
        dict.insert("N".into(), PdfObject::Integer(n));
        dict.insert("First".into(), PdfObject::Integer(first));

        let stream = StreamObject {
            dict,
            data,
            stream_offset: 0,
            stream_length: 512,
        };

        if let Ok(decoded) = ObjectStreamReader::decode_stream(&stream, &limits) {
            let idx = rng.next_u32() % 20;
            let _ = ObjectStreamReader::extract_object(&decoded, idx);
        }
    }
}

#[test]
fn test_fuzz_sfnt_font_and_tables_hostile_inputs() {
    let mut rng = PseudoRng::new(0x5EED_0003);

    for _ in 0..5_000 {
        let mut data = vec![0u8; (rng.next_u32() % 1024 + 1) as usize];
        rng.fill_bytes(&mut data);

        let _ = TableDirectory::parse(&data);
        let _ = HeadTable::parse(&data);
        let _ = MaxpTable::parse(&data);
        let _ = HheaTable::parse(&data);
        let _ = HmtxTable::parse(
            &data,
            (rng.next_u32() % 100) as u16,
            (rng.next_u32() % 200) as u16,
        );
        let _ = SfntCmapTable::parse(&data);
        let _ = SfntFont::parse(&data);
    }
}

#[test]
fn test_fuzz_tounicode_cmap_hostile_inputs() {
    let mut rng = PseudoRng::new(0x5EED_0004);

    for _ in 0..5_000 {
        let mut data = vec![0u8; (rng.next_u32() % 512 + 1) as usize];
        rng.fill_bytes(&mut data);
        let _ = UnicodeCMap::parse(&data);
    }
}

#[test]
fn test_fuzz_content_stream_parser_hostile_inputs() {
    let mut rng = PseudoRng::new(0x5EED_0005);

    for _ in 0..5_000 {
        let mut data = vec![0u8; (rng.next_u32() % 512 + 1) as usize];
        rng.fill_bytes(&mut data);
        let mut parser = ContentParser::from_bytes(&data);
        let _ = parser.parse_instructions();
    }
}

#[test]
fn test_fuzz_document_open_hostile_inputs() {
    let limits = DecompressLimits::default();
    let mut rng = PseudoRng::new(0x5EED_0006);

    for _ in 0..5_000 {
        let mut data = vec![0u8; (rng.next_u32() % 1024 + 1) as usize];
        rng.fill_bytes(&mut data);
        let _ = PdfDocument::from_bytes_with_limits(&data, limits);
    }
}

#[test]
fn test_fuzz_search_engine_hostile_inputs() {
    let mut rng = PseudoRng::new(0x5EED_0007);

    for _ in 0..2_000 {
        let mut query_bytes = vec![0u8; (rng.next_u32() % 64 + 1) as usize];
        rng.fill_bytes(&mut query_bytes);
        let query_str = String::from_utf8_lossy(&query_bytes);

        let mut page = PageText::new(0);
        for _span_idx in 0..(rng.next_u32() % 10) {
            let mut span_bytes = vec![0u8; (rng.next_u32() % 64 + 1) as usize];
            rng.fill_bytes(&mut span_bytes);
            page.spans.push(TextSpan::new(
                0,
                String::from_utf8_lossy(&span_bytes).into_owned(),
                (rng.next_u32() % 500) as f64,
                (rng.next_u32() % 700) as f64,
                (rng.next_u32() % 300) as f64,
                12.0,
                ((rng.next_u32() % 4) * 90) as f64,
                "/F1".into(),
                12.0,
                1.0,
            ));
        }

        let _ = TextMatcher::search_page(&page, &query_str, &SearchOptions::default());
        let _ = TextMatcher::search_page(
            &page,
            &query_str,
            &SearchOptions {
                case_sensitive: true,
            },
        );
    }
}
