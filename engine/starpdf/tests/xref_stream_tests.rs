use miniz_oxide::deflate::compress_to_vec_zlib;
use std::collections::BTreeMap;

use starpdf::filter::DecompressLimits;
use starpdf::syntax::object::{PdfObject, StreamObject};
use starpdf::xref::table::{XrefEntry, XrefTable};
use starpdf::xref::XrefStreamParser;

#[test]
fn test_xref_stream_uncompressed_basic() {
    let mut dict = BTreeMap::new();
    dict.insert("Type".into(), PdfObject::Name("XRef".into()));
    dict.insert("Size".into(), PdfObject::Integer(4));
    dict.insert(
        "W".into(),
        PdfObject::Array(vec![
            PdfObject::Integer(1),
            PdfObject::Integer(2),
            PdfObject::Integer(1),
        ]),
    );

    // Entries (W=[1, 2, 1], entry_len = 4 bytes):
    // 0: Type 0 (Free), next=0, gen=65535 (0xFF) -> [0, 0, 0, 255]
    // 1: Type 1 (InUse), offset=100 (0x0064), gen=0 -> [1, 0, 100, 0]
    // 2: Type 2 (Compressed), stm=10 (0x000A), idx=0 -> [2, 0, 10, 0]
    // 3: Type 2 (Compressed), stm=10 (0x000A), idx=1 -> [2, 0, 10, 1]
    let raw_data = vec![
        0, 0, 0, 255, // obj 0
        1, 0, 100, 0, // obj 1
        2, 0, 10, 0, // obj 2
        2, 0, 10, 1, // obj 3
    ];

    let stream = StreamObject {
        dict,
        data: raw_data,
        stream_offset: 0,
        stream_length: 16,
    };

    let mut table = XrefTable::new();
    let limits = DecompressLimits::default();
    XrefStreamParser::parse_into_table(&stream, &mut table, &limits).unwrap();

    assert_eq!(
        table.get_entry(0),
        Some(&XrefEntry::Free {
            next_free_obj: 0,
            generation: 255
        })
    );
    assert_eq!(
        table.get_entry(1),
        Some(&XrefEntry::InUse {
            byte_offset: 100,
            generation: 0
        })
    );
    assert_eq!(
        table.get_entry(2),
        Some(&XrefEntry::Compressed {
            stream_obj_num: 10,
            index_in_stream: 0
        })
    );
    assert_eq!(
        table.get_entry(3),
        Some(&XrefEntry::Compressed {
            stream_obj_num: 10,
            index_in_stream: 1
        })
    );
}

#[test]
fn test_xref_stream_flate_compressed_with_index() {
    let mut dict = BTreeMap::new();
    dict.insert("Type".into(), PdfObject::Name("XRef".into()));
    dict.insert("Size".into(), PdfObject::Integer(20));
    dict.insert("Filter".into(), PdfObject::Name("FlateDecode".into()));
    dict.insert(
        "W".into(),
        PdfObject::Array(vec![
            PdfObject::Integer(1),
            PdfObject::Integer(2),
            PdfObject::Integer(1),
        ]),
    );
    // Index: [5, 2] -> objects 5 and 6
    dict.insert(
        "Index".into(),
        PdfObject::Array(vec![PdfObject::Integer(5), PdfObject::Integer(2)]),
    );

    let raw_entries = vec![
        1, 1, 0, 0, // obj 5: InUse, offset 256, gen 0
        2, 0, 50, 3, // obj 6: Compressed, stm 50, idx 3
    ];
    let compressed_data = compress_to_vec_zlib(&raw_entries, 6);

    let stream = StreamObject {
        dict,
        data: compressed_data.clone(),
        stream_offset: 0,
        stream_length: compressed_data.len(),
    };

    let mut table = XrefTable::new();
    let limits = DecompressLimits::default();
    XrefStreamParser::parse_into_table(&stream, &mut table, &limits).unwrap();

    assert_eq!(
        table.get_entry(5),
        Some(&XrefEntry::InUse {
            byte_offset: 256,
            generation: 0
        })
    );
    assert_eq!(
        table.get_entry(6),
        Some(&XrefEntry::Compressed {
            stream_obj_num: 50,
            index_in_stream: 3
        })
    );
}

#[test]
fn test_xref_stream_rejects_unrepresentable_widths_and_index_ranges() {
    let limits = DecompressLimits::default();
    let mut wide_dict = BTreeMap::from([
        ("Type".into(), PdfObject::Name("XRef".into())),
        ("Size".into(), PdfObject::Integer(1)),
        (
            "W".into(),
            PdfObject::Array(vec![
                PdfObject::Integer(1),
                PdfObject::Integer(9),
                PdfObject::Integer(1),
            ]),
        ),
    ]);
    let wide_stream = StreamObject {
        dict: wide_dict.clone(),
        data: vec![0; 11],
        stream_offset: 0,
        stream_length: 11,
    };
    assert!(
        XrefStreamParser::parse_into_table(&wide_stream, &mut XrefTable::new(), &limits).is_err()
    );

    wide_dict.insert(
        "W".into(),
        PdfObject::Array(vec![
            PdfObject::Integer(1),
            PdfObject::Integer(2),
            PdfObject::Integer(1),
        ]),
    );
    wide_dict.insert(
        "Index".into(),
        PdfObject::Array(vec![PdfObject::Integer(1), PdfObject::Integer(1)]),
    );
    let out_of_size_stream = StreamObject {
        dict: wide_dict,
        data: vec![0; 4],
        stream_offset: 0,
        stream_length: 4,
    };
    assert!(XrefStreamParser::parse_into_table(
        &out_of_size_stream,
        &mut XrefTable::new(),
        &limits
    )
    .is_err());
}
