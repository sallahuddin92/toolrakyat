use starpdf::forms::AcroFormParser;
use starpdf::mutation::{MutationEngine, PdfChange};
use starpdf::syntax::object::{ObjectRef, PdfObject};
use starpdf::writer::IncrementalWriter;
use std::collections::BTreeMap;

#[test]
fn test_fuzz_acroform_cyclic_kids_no_panic() {
    let dummy_bytes = b"%PDF-1.7\n";
    let source = starpdf::io::source::ByteSource::new(dummy_bytes);

    let mut objects = BTreeMap::new();
    let f1 = ObjectRef::new(10, 0);
    let f2 = ObjectRef::new(11, 0);

    // f1 -> kids: [f2]
    // f2 -> kids: [f1] (cycle)
    let d1 = BTreeMap::from([
        ("T".to_string(), PdfObject::String(b"node1".to_vec())),
        (
            "Kids".to_string(),
            PdfObject::Array(vec![PdfObject::Reference(f2)]),
        ),
    ]);
    let d2 = BTreeMap::from([
        ("T".to_string(), PdfObject::String(b"node2".to_vec())),
        (
            "Kids".to_string(),
            PdfObject::Array(vec![PdfObject::Reference(f1)]),
        ),
    ]);

    objects.insert(f1, PdfObject::Dictionary(d1));
    objects.insert(f2, PdfObject::Dictionary(d2));

    let mut xref = starpdf::xref::table::XrefTable::new();
    xref.insert_in_use(10, 10, 0);
    xref.insert_in_use(11, 20, 0);

    let mut store = starpdf::document::object_store::ObjectStore::new(source, xref);
    for (r, obj) in objects {
        store.insert_cached(r, obj);
    }

    let parser = AcroFormParser::new(&mut store, &[]);
    let catalog = BTreeMap::from([(
        "AcroForm".to_string(),
        PdfObject::Dictionary(BTreeMap::from([(
            "Fields".to_string(),
            PdfObject::Array(vec![PdfObject::Reference(f1)]),
        )])),
    )]);

    // Parser must cleanly terminate with cycle detection, without panic or hang
    let result = parser.parse_catalog_acroform(&catalog);
    assert!(result.is_ok());
}

#[test]
fn test_fuzz_incremental_writer_corrupt_inputs_no_panic() {
    let raw_inputs = [
        b"" as &[u8],
        b"%PDF-",
        b"%PDF-1.7\nstartxref\n9999\n%%EOF",
        b"\x00\xFF\xAA\x55\xDE\xAD\xBE\xEF",
    ];

    for input in raw_inputs {
        let modified =
            BTreeMap::from([(ObjectRef::new(1, 0), PdfObject::String(b"test".to_vec()))]);
        let trailer = BTreeMap::new();
        let res = IncrementalWriter::write_update(input, &modified, 0, &trailer);
        assert!(res.is_ok());
    }
}

#[test]
fn test_fuzz_mutation_engine_random_and_huge_strings() {
    let dummy_bytes = b"%PDF-1.7\n";
    let source = starpdf::io::source::ByteSource::new(dummy_bytes);
    let mut store = starpdf::document::object_store::ObjectStore::new(
        source,
        starpdf::xref::table::XrefTable::new(),
    );

    let mut engine = MutationEngine::new(&mut store);

    // 1. Target not found
    let res = engine.prepare_plan(&[PdfChange::SetTextField {
        field_ref: ObjectRef::new(999, 0),
        value: "SomeValue".to_string(),
    }]);
    assert!(res.is_err());

    // 2. Oversized string (> 1MB)
    let huge_str = "A".repeat(2_000_000);
    let res = engine.prepare_plan(&[PdfChange::SetTextField {
        field_ref: ObjectRef::new(1, 0),
        value: huge_str,
    }]);
    assert!(res.is_err());
}
