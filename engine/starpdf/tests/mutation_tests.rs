use starpdf::document::object_store::ObjectStore;
use starpdf::io::source::ByteSource;
use starpdf::mutation::{AppearanceStatus, MutationEngine, PdfChange};
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
fn test_mutation_text_field() {
    let dummy_bytes = b"%PDF-1.7\n";
    let source = ByteSource::new(dummy_bytes);

    let field_ref = ObjectRef::new(20, 0);
    let mut field_dict = BTreeMap::new();
    field_dict.insert("FT".to_string(), PdfObject::Name("Tx".to_string()));
    field_dict.insert("T".to_string(), PdfObject::String(b"username".to_vec()));
    field_dict.insert("V".to_string(), PdfObject::String(b"OldValue".to_vec()));

    let objects = BTreeMap::from([(field_ref, PdfObject::Dictionary(field_dict))]);
    let mut store = create_test_store(source, objects);

    let mut engine = MutationEngine::new(&mut store);
    let plan = engine
        .prepare_plan(&[PdfChange::SetTextField {
            field_ref,
            value: "NewValue".to_string(),
        }])
        .unwrap();

    assert_eq!(plan.modified_objects.len(), 1);
    let modified = plan.modified_objects.get(&field_ref).unwrap();
    let dict = modified.as_dict().unwrap();
    assert_eq!(
        dict.get("V"),
        Some(&PdfObject::String(b"NewValue".to_vec()))
    );
    assert_eq!(plan.appearance_status, AppearanceStatus::LogicalOnlyUpdated);
}

#[test]
fn test_mutation_checkbox_toggle() {
    let dummy_bytes = b"%PDF-1.7\n";
    let source = ByteSource::new(dummy_bytes);

    let field_ref = ObjectRef::new(30, 0);
    let mut field_dict = BTreeMap::new();
    field_dict.insert("FT".to_string(), PdfObject::Name("Btn".to_string()));
    field_dict.insert("T".to_string(), PdfObject::String(b"subscribe".to_vec()));
    field_dict.insert("V".to_string(), PdfObject::Name("Off".to_string()));
    field_dict.insert("AS".to_string(), PdfObject::Name("Off".to_string()));
    field_dict.insert(
        "AP".to_string(),
        PdfObject::Dictionary(BTreeMap::from([(
            "N".to_string(),
            PdfObject::Dictionary(BTreeMap::from([
                ("Off".to_string(), PdfObject::Dictionary(BTreeMap::new())),
                ("Yes".to_string(), PdfObject::Dictionary(BTreeMap::new())),
            ])),
        )])),
    );

    let objects = BTreeMap::from([(field_ref, PdfObject::Dictionary(field_dict))]);
    let mut store = create_test_store(source, objects);

    let mut engine = MutationEngine::new(&mut store);
    let plan = engine
        .prepare_plan(&[PdfChange::SetCheckbox {
            field_ref,
            widget_refs: vec![field_ref],
            checked: true,
        }])
        .unwrap();

    assert_eq!(plan.modified_objects.len(), 1);
    let modified = plan.modified_objects.get(&field_ref).unwrap();
    let dict = modified.as_dict().unwrap();
    assert_eq!(dict.get("V"), Some(&PdfObject::Name("Yes".to_string())));
    assert_eq!(dict.get("AS"), Some(&PdfObject::Name("Yes".to_string())));
    assert_eq!(
        plan.appearance_status,
        AppearanceStatus::AppearanceStateUpdated
    );
}
