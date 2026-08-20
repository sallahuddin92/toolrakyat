use starpdf::annotation::{AnnotationParser, AnnotationSubtype};
use starpdf::document::object_store::ObjectStore;
use starpdf::io::source::ByteSource;
use starpdf::syntax::object::{ObjectRef, PdfObject};
use starpdf::xref::table::{XrefEntry, XrefTable};
use std::collections::BTreeMap;

fn create_test_store<'a>(
    source: ByteSource<'a>,
    objects: BTreeMap<ObjectRef, PdfObject>,
) -> ObjectStore<'a> {
    let mut xref = XrefTable::new();
    for r in objects.keys() {
        xref.entries.insert(
            r.number,
            XrefEntry::InUse {
                byte_offset: 10,
                generation: r.generation,
            },
        );
    }
    let mut store = ObjectStore::new(source, xref);
    for (r, obj) in objects {
        store.insert_cached(r, obj);
    }
    store
}

#[test]
fn test_page_annotations_mixed_subtypes() {
    let dummy_bytes = b"%PDF-1.7\n";
    let source = ByteSource::new(dummy_bytes);

    let mut objects = BTreeMap::new();
    let annot1_ref = ObjectRef::new(101, 0);
    let annot2_ref = ObjectRef::new(102, 0);
    let annot3_ref = ObjectRef::new(103, 0);

    // Annot 1: Highlight
    let mut a1_dict = BTreeMap::new();
    a1_dict.insert(
        "Subtype".to_string(),
        PdfObject::Name("Highlight".to_string()),
    );
    a1_dict.insert(
        "Rect".to_string(),
        PdfObject::Array(vec![
            PdfObject::Real(50.0),
            PdfObject::Real(700.0),
            PdfObject::Real(200.0),
            PdfObject::Real(720.0),
        ]),
    );
    a1_dict.insert(
        "Contents".to_string(),
        PdfObject::String(b"Important clause".to_vec()),
    );
    objects.insert(annot1_ref, PdfObject::Dictionary(a1_dict));

    // Annot 2: Link
    let mut a2_dict = BTreeMap::new();
    a2_dict.insert("Subtype".to_string(), PdfObject::Name("Link".to_string()));
    a2_dict.insert(
        "Rect".to_string(),
        PdfObject::Array(vec![
            PdfObject::Real(50.0),
            PdfObject::Real(650.0),
            PdfObject::Real(150.0),
            PdfObject::Real(670.0),
        ]),
    );
    objects.insert(annot2_ref, PdfObject::Dictionary(a2_dict));

    // Annot 3: Custom / Unknown Subtype
    let mut a3_dict = BTreeMap::new();
    a3_dict.insert(
        "Subtype".to_string(),
        PdfObject::Name("CustomVendorSeal".to_string()),
    );
    a3_dict.insert(
        "Rect".to_string(),
        PdfObject::Array(vec![
            PdfObject::Real(300.0),
            PdfObject::Real(300.0),
            PdfObject::Real(400.0),
            PdfObject::Real(400.0),
        ]),
    );
    objects.insert(annot3_ref, PdfObject::Dictionary(a3_dict));

    let page_dict = BTreeMap::from([(
        "Annots".to_string(),
        PdfObject::Array(vec![
            PdfObject::Reference(annot1_ref),
            PdfObject::Reference(annot2_ref),
            PdfObject::Reference(annot3_ref),
        ]),
    )]);

    let mut store = create_test_store(source, objects);
    let mut parser = AnnotationParser::new(&mut store);
    let annots = parser.parse_page_annotations(&page_dict, 0).unwrap();

    assert_eq!(annots.len(), 3);

    assert_eq!(annots[0].subtype, AnnotationSubtype::Highlight);
    assert_eq!(annots[0].contents.as_deref(), Some("Important clause"));
    assert_eq!(annots[0].rect, [50.0, 700.0, 200.0, 720.0]);

    assert_eq!(annots[1].subtype, AnnotationSubtype::Link);

    assert_eq!(
        annots[2].subtype,
        AnnotationSubtype::Unknown("CustomVendorSeal".to_string())
    );
}
