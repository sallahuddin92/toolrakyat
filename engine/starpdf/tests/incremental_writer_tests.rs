use std::fs;
use std::path::PathBuf;

use starpdf::appearance::AppearanceStatus;
use starpdf::document::PdfDocument;
use starpdf::forms::FieldValue;
use starpdf::mutation::PdfChange;
use starpdf::writer::MinimalWriter;

fn fixture_path(name: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("../../test-assets");
    p.push(name);
    p
}

#[test]
fn test_incremental_writer_roundtrip_minimal_doc() {
    let original_bytes = MinimalWriter::create_minimal_pdf("Hello StarPDF v0.6").unwrap();
    let mut doc = PdfDocument::from_bytes(&original_bytes).unwrap();
    assert_eq!(doc.page_count().unwrap(), 1);

    // Mutate page object (Object 3)
    let page_ref = doc.page_ref(0).unwrap();
    let mut page_dict = doc.page_dict(0).unwrap();
    page_dict.insert(
        "CustomTag".to_string(),
        starpdf::syntax::object::PdfObject::String(b"IncrementalSuccess".to_vec()),
    );

    let plan = starpdf::mutation::MutationPlan {
        modified_objects: std::collections::BTreeMap::from([(
            page_ref,
            starpdf::syntax::object::PdfObject::Dictionary(page_dict),
        )]),
        appearance_status: AppearanceStatus::ValueUpdated,
    };

    let updated_bytes = doc.export_incremental(&plan).unwrap();

    // Verify original bytes prefix is preserved verbatim
    assert!(updated_bytes.len() > original_bytes.len());
    assert_eq!(&updated_bytes[..original_bytes.len()], &original_bytes[..]);

    // Reopen updated document with StarPDF
    let mut reopened = PdfDocument::from_bytes(&updated_bytes).unwrap();
    assert_eq!(reopened.page_count().unwrap(), 1);
    let reopened_page = reopened.page_dict(0).unwrap();
    assert_eq!(
        reopened_page.get("CustomTag"),
        Some(&starpdf::syntax::object::PdfObject::String(
            b"IncrementalSuccess".to_vec()
        ))
    );

    // Apply second sequential incremental update
    let mut page_dict_2 = reopened.page_dict(0).unwrap();
    page_dict_2.insert(
        "SecondTag".to_string(),
        starpdf::syntax::object::PdfObject::String(b"SequentialSuccess".to_vec()),
    );

    let plan_2 = starpdf::mutation::MutationPlan {
        modified_objects: std::collections::BTreeMap::from([(
            page_ref,
            starpdf::syntax::object::PdfObject::Dictionary(page_dict_2),
        )]),
        appearance_status: AppearanceStatus::ValueUpdated,
    };

    let second_updated_bytes = reopened.export_incremental(&plan_2).unwrap();
    assert!(second_updated_bytes.len() > updated_bytes.len());

    let mut final_reopened = PdfDocument::from_bytes(&second_updated_bytes).unwrap();
    let final_page = final_reopened.page_dict(0).unwrap();
    assert_eq!(
        final_page.get("SecondTag"),
        Some(&starpdf::syntax::object::PdfObject::String(
            b"SequentialSuccess".to_vec()
        ))
    );
}

#[test]
fn test_incremental_mutation_real_form_fixture() {
    let path = fixture_path("smartpdf-form.pdf");
    let original_bytes = fs::read(path).unwrap();

    let mut doc = PdfDocument::from_bytes(&original_bytes).unwrap();
    let fields = doc.form_fields().unwrap();
    assert!(!fields.is_empty());

    // Find the text field "full_name" or "fullName"
    let text_field = fields
        .iter()
        .find(|f| f.partial_name == "full_name" || f.partial_name == "fullName")
        .expect("Expected full_name field");

    // Find the checkbox field "agree" or "subscribe"
    let check_field = fields
        .iter()
        .find(|f| f.partial_name == "agree" || f.partial_name == "subscribe")
        .expect("Expected agree/subscribe field");

    let changes = vec![
        PdfChange::SetTextField {
            field_ref: text_field.object_ref,
            value: "Dr. ToolRakyat".to_string(),
        },
        PdfChange::SetCheckbox {
            field_ref: check_field.object_ref,
            widget_refs: check_field.widgets.iter().map(|w| w.object_ref).collect(),
            checked: true,
        },
    ];

    let mutated_bytes = doc.mutate_and_export(&changes).unwrap();
    assert!(mutated_bytes.len() > original_bytes.len());
    assert_eq!(&mutated_bytes[..original_bytes.len()], &original_bytes[..]);

    // Reopen and verify mutated values
    let mut reopened = PdfDocument::from_bytes(&mutated_bytes).unwrap();
    assert_eq!(reopened.page_count().unwrap(), 1);

    let reopened_fields = reopened.form_fields().unwrap();
    let updated_text = reopened_fields
        .iter()
        .find(|f| f.partial_name == "full_name" || f.partial_name == "fullName")
        .unwrap();
    assert_eq!(
        updated_text.value,
        FieldValue::Text("Dr. ToolRakyat".to_string())
    );

    let updated_check = reopened_fields
        .iter()
        .find(|f| f.partial_name == "agree" || f.partial_name == "subscribe")
        .unwrap();
    assert_eq!(updated_check.value, FieldValue::Boolean(true));
}
