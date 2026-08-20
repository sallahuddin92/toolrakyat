use std::fs;
use std::path::PathBuf;

use starpdf::document::PdfDocument;
use starpdf::forms::FieldValue;
use starpdf::mutation::PdfChange;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test-assets")
        .join(name)
}

#[test]
fn test_visual_roundtrip_form_appearance_regeneration() {
    let form_path = fixture_path("smartpdf-form.pdf");
    if !form_path.exists() {
        eprintln!("Fixture smartpdf-form.pdf not found, skipping");
        return;
    }

    let original_bytes = fs::read(&form_path).unwrap();
    let mut doc = PdfDocument::from_bytes(&original_bytes).unwrap();

    let fields = doc.form_fields().unwrap();
    assert!(!fields.is_empty(), "Document should have form fields");

    let full_name_field = fields
        .iter()
        .find(|f| f.partial_name == "full_name" || f.fully_qualified_name == "full_name")
        .expect("full_name field should exist");

    let agree_field = fields
        .iter()
        .find(|f| f.partial_name == "agree" || f.fully_qualified_name == "agree")
        .expect("agree field should exist");

    // 1. Mutate text field and checkbox
    let changes = vec![
        PdfChange::SetTextField {
            field_ref: full_name_field.object_ref,
            value: "Sallahuddin v0.7 Engineer".to_string(),
        },
        PdfChange::SetCheckbox {
            field_ref: agree_field.object_ref,
            widget_refs: agree_field.widgets.iter().map(|w| w.object_ref).collect(),
            checked: true,
        },
    ];

    let plan = doc.apply_mutation(&changes).unwrap();
    assert_eq!(
        plan.appearance_status,
        starpdf::appearance::AppearanceStatus::AppearanceRegenerated
    );

    let mutated_bytes = doc.export_incremental(&plan).unwrap();

    // Verify incremental byte preservation: starts with exact original bytes
    assert!(mutated_bytes.starts_with(&original_bytes));
    assert!(mutated_bytes.len() > original_bytes.len());

    // 2. Reopen mutated document with StarPDF
    let mut reopened_doc = PdfDocument::from_bytes(&mutated_bytes).unwrap();
    let reopened_fields = reopened_doc.form_fields().unwrap();

    let reopened_full_name = reopened_fields
        .iter()
        .find(|f| f.partial_name == "full_name" || f.fully_qualified_name == "full_name")
        .expect("full_name must exist in reopened doc");

    assert_eq!(
        reopened_full_name.value,
        FieldValue::Text("Sallahuddin v0.7 Engineer".to_string())
    );

    let reopened_agree = reopened_fields
        .iter()
        .find(|f| f.partial_name == "agree" || f.fully_qualified_name == "agree")
        .expect("agree must exist in reopened doc");

    assert_eq!(reopened_agree.value, FieldValue::Boolean(true));
}
