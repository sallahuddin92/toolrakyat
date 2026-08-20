use starpdf::document::object_store::ObjectStore;
use starpdf::forms::{AcroFormParser, FieldType, FieldValue};
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
fn test_acroform_simple_text_field() {
    let dummy_bytes = b"%PDF-1.7\n";
    let source = ByteSource::new(dummy_bytes);

    let mut objects = BTreeMap::new();
    let form_ref = ObjectRef::new(10, 0);
    let field_ref = ObjectRef::new(20, 0);
    let page_ref = ObjectRef::new(1, 0);

    // Form Dict
    let mut form_dict = BTreeMap::new();
    form_dict.insert(
        "Fields".to_string(),
        PdfObject::Array(vec![PdfObject::Reference(field_ref)]),
    );
    form_dict.insert(
        "DA".to_string(),
        PdfObject::String(b"/Helv 12 Tf 0 g".to_vec()),
    );
    objects.insert(form_ref, PdfObject::Dictionary(form_dict.clone()));

    // Field Dict: Text field
    let mut field_dict = BTreeMap::new();
    field_dict.insert("FT".to_string(), PdfObject::Name("Tx".to_string()));
    field_dict.insert("T".to_string(), PdfObject::String(b"full_name".to_vec()));
    field_dict.insert("V".to_string(), PdfObject::String(b"Alice Smith".to_vec()));
    field_dict.insert(
        "Rect".to_string(),
        PdfObject::Array(vec![
            PdfObject::Real(100.0),
            PdfObject::Real(200.0),
            PdfObject::Real(300.0),
            PdfObject::Real(220.0),
        ]),
    );
    field_dict.insert("P".to_string(), PdfObject::Reference(page_ref));
    objects.insert(field_ref, PdfObject::Dictionary(field_dict));

    let mut store = create_test_store(source, objects);
    let parser = AcroFormParser::new(&mut store, &[page_ref]);
    let acroform = parser
        .parse_catalog_acroform(&BTreeMap::from([(
            "AcroForm".to_string(),
            PdfObject::Reference(form_ref),
        )]))
        .unwrap()
        .unwrap();

    assert_eq!(acroform.fields.len(), 1);
    let f = &acroform.fields[0];
    assert_eq!(f.fully_qualified_name, "full_name");
    assert_eq!(
        f.field_type,
        FieldType::Text {
            multiline: false,
            password: false
        }
    );
    assert_eq!(f.value, FieldValue::Text("Alice Smith".to_string()));
    assert_eq!(f.widgets.len(), 1);
    assert_eq!(f.widgets[0].rect, [100.0, 200.0, 300.0, 220.0]);
    assert_eq!(f.widgets[0].page_index, Some(0));
}

#[test]
fn test_acroform_nested_field_hierarchy() {
    let dummy_bytes = b"%PDF-1.7\n";
    let source = ByteSource::new(dummy_bytes);

    let mut objects = BTreeMap::new();
    let form_ref = ObjectRef::new(10, 0);
    let parent_field_ref = ObjectRef::new(20, 0);
    let child_field_ref = ObjectRef::new(21, 0);

    // Form Dict
    let mut form_dict = BTreeMap::new();
    form_dict.insert(
        "Fields".to_string(),
        PdfObject::Array(vec![PdfObject::Reference(parent_field_ref)]),
    );
    objects.insert(form_ref, PdfObject::Dictionary(form_dict.clone()));

    // Parent Field Dict (has /T "billing" and /Kids [child])
    let mut parent_dict = BTreeMap::new();
    parent_dict.insert("T".to_string(), PdfObject::String(b"billing".to_vec()));
    parent_dict.insert("FT".to_string(), PdfObject::Name("Tx".to_string()));
    parent_dict.insert(
        "Kids".to_string(),
        PdfObject::Array(vec![PdfObject::Reference(child_field_ref)]),
    );
    objects.insert(parent_field_ref, PdfObject::Dictionary(parent_dict));

    // Child Field Dict (has /T "address" and /V "123 Main St")
    let mut child_dict = BTreeMap::new();
    child_dict.insert("T".to_string(), PdfObject::String(b"address".to_vec()));
    child_dict.insert("V".to_string(), PdfObject::String(b"123 Main St".to_vec()));
    objects.insert(child_field_ref, PdfObject::Dictionary(child_dict));

    let mut store = create_test_store(source, objects);
    let parser = AcroFormParser::new(&mut store, &[]);
    let acroform = parser
        .parse_catalog_acroform(&BTreeMap::from([(
            "AcroForm".to_string(),
            PdfObject::Reference(form_ref),
        )]))
        .unwrap()
        .unwrap();

    assert_eq!(acroform.fields.len(), 1);
    let f = &acroform.fields[0];
    assert_eq!(f.fully_qualified_name, "billing.address");
    assert_eq!(f.value, FieldValue::Text("123 Main St".to_string()));
    assert_eq!(f.parent_ref, Some(parent_field_ref));
}

#[test]
fn test_acroform_checkbox_with_custom_on_state() {
    let dummy_bytes = b"%PDF-1.7\n";
    let source = ByteSource::new(dummy_bytes);

    let mut objects = BTreeMap::new();
    let form_ref = ObjectRef::new(10, 0);
    let check_ref = ObjectRef::new(30, 0);

    // Appearance dict /AP with /N << /Off ... /CustomYes ... >>
    let mut n_dict = BTreeMap::new();
    n_dict.insert("Off".to_string(), PdfObject::Dictionary(BTreeMap::new()));
    n_dict.insert(
        "CustomYes".to_string(),
        PdfObject::Dictionary(BTreeMap::new()),
    );
    let ap_dict = BTreeMap::from([("N".to_string(), PdfObject::Dictionary(n_dict))]);

    let mut check_dict = BTreeMap::new();
    check_dict.insert("FT".to_string(), PdfObject::Name("Btn".to_string()));
    check_dict.insert("T".to_string(), PdfObject::String(b"terms_agreed".to_vec()));
    check_dict.insert("V".to_string(), PdfObject::Name("CustomYes".to_string()));
    check_dict.insert("AS".to_string(), PdfObject::Name("CustomYes".to_string()));
    check_dict.insert("AP".to_string(), PdfObject::Dictionary(ap_dict));
    objects.insert(check_ref, PdfObject::Dictionary(check_dict));

    let form_dict = BTreeMap::from([(
        "Fields".to_string(),
        PdfObject::Array(vec![PdfObject::Reference(check_ref)]),
    )]);
    objects.insert(form_ref, PdfObject::Dictionary(form_dict));

    let mut store = create_test_store(source, objects);
    let parser = AcroFormParser::new(&mut store, &[]);
    let acroform = parser
        .parse_catalog_acroform(&BTreeMap::from([(
            "AcroForm".to_string(),
            PdfObject::Reference(form_ref),
        )]))
        .unwrap()
        .unwrap();

    assert_eq!(acroform.fields.len(), 1);
    let f = &acroform.fields[0];
    assert_eq!(f.field_type, FieldType::Checkbox);
    assert_eq!(f.value, FieldValue::Boolean(true));
    assert_eq!(f.widgets.len(), 1);
    assert_eq!(f.widgets[0].on_state_name(), Some("CustomYes"));
    assert!(f.widgets[0].is_checked());
}

#[test]
fn test_acroform_radio_group_and_choices() {
    let dummy_bytes = b"%PDF-1.7\n";
    let source = ByteSource::new(dummy_bytes);

    let mut objects = BTreeMap::new();
    let form_ref = ObjectRef::new(10, 0);
    let radio_ref = ObjectRef::new(40, 0);
    let widget1_ref = ObjectRef::new(41, 0);
    let widget2_ref = ObjectRef::new(42, 0);

    // Radio widgets: widget1 (/AS /Male), widget2 (/AS /Off)
    let w1_dict = BTreeMap::from([
        ("Subtype".to_string(), PdfObject::Name("Widget".to_string())),
        ("AS".to_string(), PdfObject::Name("Male".to_string())),
        (
            "AP".to_string(),
            PdfObject::Dictionary(BTreeMap::from([(
                "N".to_string(),
                PdfObject::Dictionary(BTreeMap::from([
                    ("Off".to_string(), PdfObject::Dictionary(BTreeMap::new())),
                    ("Male".to_string(), PdfObject::Dictionary(BTreeMap::new())),
                ])),
            )])),
        ),
    ]);
    objects.insert(widget1_ref, PdfObject::Dictionary(w1_dict));

    let w2_dict = BTreeMap::from([
        ("Subtype".to_string(), PdfObject::Name("Widget".to_string())),
        ("AS".to_string(), PdfObject::Name("Off".to_string())),
        (
            "AP".to_string(),
            PdfObject::Dictionary(BTreeMap::from([(
                "N".to_string(),
                PdfObject::Dictionary(BTreeMap::from([
                    ("Off".to_string(), PdfObject::Dictionary(BTreeMap::new())),
                    ("Female".to_string(), PdfObject::Dictionary(BTreeMap::new())),
                ])),
            )])),
        ),
    ]);
    objects.insert(widget2_ref, PdfObject::Dictionary(w2_dict));

    // Radio parent field (flags bit 16 = 1 << 15: Radio)
    let mut radio_dict = BTreeMap::new();
    radio_dict.insert("FT".to_string(), PdfObject::Name("Btn".to_string()));
    radio_dict.insert("Ff".to_string(), PdfObject::Integer(1 << 15));
    radio_dict.insert("T".to_string(), PdfObject::String(b"gender".to_vec()));
    radio_dict.insert("V".to_string(), PdfObject::Name("Male".to_string()));
    radio_dict.insert(
        "Kids".to_string(),
        PdfObject::Array(vec![
            PdfObject::Reference(widget1_ref),
            PdfObject::Reference(widget2_ref),
        ]),
    );
    objects.insert(radio_ref, PdfObject::Dictionary(radio_dict));

    let form_dict = BTreeMap::from([(
        "Fields".to_string(),
        PdfObject::Array(vec![PdfObject::Reference(radio_ref)]),
    )]);
    objects.insert(form_ref, PdfObject::Dictionary(form_dict));

    let mut store = create_test_store(source, objects);
    let parser = AcroFormParser::new(&mut store, &[]);
    let acroform = parser
        .parse_catalog_acroform(&BTreeMap::from([(
            "AcroForm".to_string(),
            PdfObject::Reference(form_ref),
        )]))
        .unwrap()
        .unwrap();

    assert_eq!(acroform.fields.len(), 1);
    let f = &acroform.fields[0];
    assert_eq!(f.field_type, FieldType::RadioButtonGroup);
    assert_eq!(f.value, FieldValue::Name("Male".to_string()));
    assert_eq!(f.widgets.len(), 2);
    assert!(f.widgets[0].is_checked());
    assert!(!f.widgets[1].is_checked());
}
