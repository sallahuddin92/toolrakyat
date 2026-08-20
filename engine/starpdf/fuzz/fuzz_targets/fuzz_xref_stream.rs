#![no_main]
use libfuzzer_sys::fuzz_target;
use starpdf::filter::limits::DecompressLimits;
use starpdf::syntax::object::{PdfObject, StreamObject};
use starpdf::xref::table::XrefTable;
use starpdf::xref::XrefStreamParser;
use std::collections::BTreeMap;

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }
    let limits = DecompressLimits::default();
    let mut dict = BTreeMap::new();
    dict.insert("Type".into(), PdfObject::Name("XRef".into()));
    dict.insert("Size".into(), PdfObject::Integer(100));

    let w0 = (data[0] % 4) as i64;
    let w1 = (data[1] % 4) as i64;
    let w2 = (data[2] % 4) as i64;
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
        data: data[3..].to_vec(),
        stream_offset: 0,
        stream_length: data.len() - 3,
    };

    let mut table = XrefTable::new();
    let _ = XrefStreamParser::parse_into_table(&stream, &mut table, &limits);
});
