use miniz_oxide::deflate::compress_to_vec_zlib;
use std::collections::BTreeMap;

use starpdf::document::ObjectStreamReader;
use starpdf::filter::DecompressLimits;
use starpdf::syntax::object::{PdfObject, StreamObject};

#[test]
fn test_object_stream_decode_and_extract() {
    let mut dict = BTreeMap::new();
    dict.insert("Type".into(), PdfObject::Name("ObjStm".into()));
    dict.insert("N".into(), PdfObject::Integer(3));

    // Header:
    // 10 0  (obj 10 at offset 0 relative to /First)
    // 11 15 (obj 11 at offset 15 relative to /First)
    // 12 35 (obj 12 at offset 35 relative to /First)
    let header = b"10 0 11 15 12 35 ";
    let first = header.len();
    dict.insert("First".into(), PdfObject::Integer(first as i64));

    // Body:
    // obj 10: << /A 1 >>       (len ~12)
    // obj 11: [ 1 2 3 4 ]      (len ~12)
    // obj 12: (Hello Object)   (len ~14)
    let body = b"<< /A 1 >>      [ 1 2 3 4 ]          (Hello Object)";

    let mut stream_payload = Vec::new();
    stream_payload.extend_from_slice(header);
    stream_payload.extend_from_slice(body);

    let stream = StreamObject {
        dict,
        data: stream_payload.clone(),
        stream_offset: 0,
        stream_length: stream_payload.len(),
    };

    let limits = DecompressLimits::default();
    let decoded = ObjectStreamReader::decode_stream(&stream, &limits).unwrap();

    assert_eq!(decoded.n_objects, 3);

    // Extract obj index 0 (obj 10)
    let obj0 = ObjectStreamReader::extract_object(&decoded, 0).unwrap();
    assert_eq!(
        obj0.as_dict().unwrap().get("A"),
        Some(&PdfObject::Integer(1))
    );

    // Extract obj index 1 (obj 11)
    let obj1 = ObjectStreamReader::extract_object(&decoded, 1).unwrap();
    assert_eq!(obj1.as_array().unwrap().len(), 4);

    // Extract obj index 2 (obj 12)
    let obj2 = ObjectStreamReader::extract_object(&decoded, 2).unwrap();
    assert_eq!(obj2.as_bytes(), Some(b"Hello Object".as_slice()));
}

#[test]
fn test_object_stream_compressed_flate() {
    let mut dict = BTreeMap::new();
    dict.insert("Type".into(), PdfObject::Name("ObjStm".into()));
    dict.insert("N".into(), PdfObject::Integer(1));
    dict.insert("Filter".into(), PdfObject::Name("FlateDecode".into()));

    let header = b"5 0 ";
    let first = header.len();
    dict.insert("First".into(), PdfObject::Integer(first as i64));

    let body = b"/AcrobatDirect";
    let mut raw = Vec::new();
    raw.extend_from_slice(header);
    raw.extend_from_slice(body);

    let compressed = compress_to_vec_zlib(&raw, 6);
    let stream = StreamObject {
        dict,
        data: compressed.clone(),
        stream_offset: 0,
        stream_length: compressed.len(),
    };

    let limits = DecompressLimits::default();
    let decoded = ObjectStreamReader::decode_stream(&stream, &limits).unwrap();
    let obj = ObjectStreamReader::extract_object(&decoded, 0).unwrap();
    assert_eq!(obj.as_name(), Some("AcrobatDirect"));
}
