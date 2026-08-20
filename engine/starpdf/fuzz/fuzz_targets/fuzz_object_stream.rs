#![no_main]
use libfuzzer_sys::fuzz_target;
use starpdf::document::ObjectStreamReader;
use starpdf::filter::limits::DecompressLimits;
use starpdf::syntax::object::{PdfObject, StreamObject};
use std::collections::BTreeMap;

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }
    let limits = DecompressLimits::default();
    let mut dict = BTreeMap::new();
    dict.insert("Type".into(), PdfObject::Name("ObjStm".into()));
    let n = (data[0] as i64) % 100;
    let first = (data[1] as i64) % (data.len() as i64);
    dict.insert("N".into(), PdfObject::Integer(n));
    dict.insert("First".into(), PdfObject::Integer(first));

    let stream = StreamObject {
        dict,
        data: data[2..].to_vec(),
        stream_offset: 0,
        stream_length: data.len() - 2,
    };

    if let Ok(decoded) = ObjectStreamReader::decode_stream(&stream, &limits) {
        let _ = ObjectStreamReader::extract_object(&decoded, 0);
    }
});
