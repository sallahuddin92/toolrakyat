use std::path::{Path, PathBuf};

use starpdf::annotation::{AnnotationSpec, AnnotationUpdateSpec};
use starpdf::appearance::AppearanceStatus;
use starpdf::document::PdfDocument;
use starpdf::error::PdfError;
use starpdf::forms::FieldValue;
use starpdf::mutation::PdfChange;
use starpdf::syntax::{ObjectRef, PdfObject, StreamObject};
use starpdf::writer::MinimalWriter;

fn fixture_path(id: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/v0_10_compat")
        .join(format!("{id}.pdf"))
}

#[test]
fn inherited_field_properties_drive_semantics_and_mutation() {
    let bytes = std::fs::read(fixture_path("pdflib-inherited-field")).unwrap();
    let mut document = PdfDocument::from_bytes(&bytes).unwrap();
    let fields = document.form_fields().unwrap();
    assert_eq!(fields.len(), 1);
    let field = &fields[0];
    assert_eq!(field.fully_qualified_name, "group.leaf");
    assert_eq!(
        field.value,
        FieldValue::Text("Inherited current value".into())
    );
    assert_eq!(
        field.default_value,
        FieldValue::Text("Inherited default value".into())
    );
    assert_eq!(
        field.default_appearance.as_deref(),
        Some("/Helvetica 13 Tf 0 g")
    );
    assert_eq!(field.quadding, Some(2));
    assert_eq!(field.max_len, Some(64));

    let output = document
        .mutate_and_export(&[PdfChange::SetTextField {
            field_ref: field.object_ref,
            value: "Inherited mutation".into(),
        }])
        .unwrap();
    assert!(output.starts_with(&bytes));
    let mut reopened = PdfDocument::from_bytes(&output).unwrap();
    assert_eq!(
        reopened.form_fields().unwrap()[0].value,
        FieldValue::Text("Inherited mutation".into())
    );
}

#[test]
fn need_appearances_is_cleared_only_after_all_widgets_have_normal_appearances() {
    let bytes = std::fs::read(fixture_path("pdflib-needappearances-ap-rd")).unwrap();
    let mut document = PdfDocument::from_bytes(&bytes).unwrap();
    assert!(document.acroform().unwrap().unwrap().need_appearances);
    let field = document.form_fields().unwrap().remove(0);
    let widget_ref = field.widgets[0].object_ref;
    let plan = document
        .apply_mutation(&[PdfChange::SetTextField {
            field_ref: field.object_ref,
            value: "Regenerated without viewer help".into(),
        }])
        .unwrap();
    let widget = plan
        .modified_objects
        .get(&widget_ref)
        .unwrap()
        .as_dict()
        .unwrap();
    let appearance = widget.get("AP").unwrap().as_dict().unwrap();
    assert!(appearance.contains_key("N"));
    assert!(appearance.contains_key("R"));
    assert!(appearance.contains_key("D"));
    let output = document.export_incremental(&plan).unwrap();
    let mut reopened = PdfDocument::from_bytes(&output).unwrap();
    assert!(!reopened.acroform().unwrap().unwrap().need_appearances);
}

#[test]
fn orphan_pdfkit_widgets_are_distinct_objects_even_when_names_match() {
    let bytes = std::fs::read(fixture_path("pdfkit-text-checkbox")).unwrap();
    let mut document = PdfDocument::from_bytes(&bytes).unwrap();
    let fields = document.form_fields().unwrap();
    let mut shared = fields
        .iter()
        .filter(|field| field.fully_qualified_name == "pdfkit.person.name");
    let first = shared.next().unwrap();
    let second = shared.next().unwrap();
    assert_ne!(first.object_ref, second.object_ref);
    assert!(shared.next().is_none());
    let output = document
        .mutate_and_export(&[PdfChange::SetTextField {
            field_ref: first.object_ref,
            value: "Only first widget changes".into(),
        }])
        .unwrap();
    let mut reopened = PdfDocument::from_bytes(&output).unwrap();
    let reopened_fields = reopened.form_fields().unwrap();
    let first_value = reopened_fields
        .iter()
        .find(|field| field.object_ref == first.object_ref)
        .map(|field| field.value.clone());
    let second_value = reopened_fields
        .iter()
        .find(|field| field.object_ref == second.object_ref)
        .map(|field| field.value.clone());
    assert_eq!(
        first_value,
        Some(FieldValue::Text("Only first widget changes".into()))
    );
    assert_eq!(
        second_value,
        Some(FieldValue::Text("PDFKit multi-widget 2".into()))
    );
}

#[test]
fn semantic_annotation_update_preserves_producer_appearance() {
    let bytes = std::fs::read(fixture_path("pdfkit-shapes-ink-link")).unwrap();
    let mut document = PdfDocument::from_bytes(&bytes).unwrap();
    let line = document
        .page_annotations(0)
        .unwrap()
        .into_iter()
        .find(|annotation| annotation.subtype.as_name() == "Line")
        .unwrap();
    let plan = document
        .apply_mutation(&[PdfChange::UpdateAnnotation {
            annot_ref: line.object_ref,
            update: AnnotationUpdateSpec {
                contents: Some("Semantic-only producer update".into()),
                ..Default::default()
            },
        }])
        .unwrap();
    assert_eq!(
        plan.appearance_status,
        AppearanceStatus::AppearancePreserved
    );
    let updated = plan
        .modified_objects
        .get(&line.object_ref)
        .and_then(PdfObject::as_dict)
        .unwrap();
    assert!(updated.contains_key("AP"));
}

#[test]
fn producer_link_uri_is_parsed_and_preserved() {
    let bytes = std::fs::read(fixture_path("pdfkit-shapes-ink-link")).unwrap();
    let mut document = PdfDocument::from_bytes(&bytes).unwrap();
    let annotations = document.page_annotations(0).unwrap();
    let link = annotations
        .iter()
        .find(|annotation| annotation.subtype.as_name() == "Link")
        .unwrap();
    assert_eq!(
        link.uri.as_deref(),
        Some("https://example.invalid/starpdf-v0.10")
    );
    let output = document
        .mutate_and_export(&[PdfChange::UpdateAnnotation {
            annot_ref: annotations[0].object_ref,
            update: AnnotationUpdateSpec {
                contents: Some("Unrelated semantic update".into()),
                ..Default::default()
            },
        }])
        .unwrap();
    let mut reopened = PdfDocument::from_bytes(&output).unwrap();
    assert_eq!(
        reopened
            .page_annotations(0)
            .unwrap()
            .iter()
            .find(|annotation| annotation.subtype.as_name() == "Link")
            .and_then(|annotation| annotation.uri.as_deref()),
        Some("https://example.invalid/starpdf-v0.10")
    );
}

#[test]
fn mixed_multiwidget_and_annotation_failure_is_atomic() {
    let bytes = std::fs::read(fixture_path("pdflib-complete-form")).unwrap();
    let mut document = PdfDocument::from_bytes(&bytes).unwrap();
    let fields = document.form_fields().unwrap();
    let field = fields
        .iter()
        .find(|field| field.fully_qualified_name == "shared.contact")
        .unwrap();
    let original = field.value.clone();
    let result = document.apply_mutation(&[
        PdfChange::SetTextField {
            field_ref: field.object_ref,
            value: "Must remain atomic".into(),
        },
        PdfChange::UpdateAnnotation {
            annot_ref: field.object_ref,
            update: AnnotationUpdateSpec {
                color: Some(vec![1.0, 0.0, 0.0]),
                ..Default::default()
            },
        },
    ]);
    assert!(result.is_err());
    assert_eq!(
        document
            .form_fields()
            .unwrap()
            .into_iter()
            .find(|candidate| candidate.object_ref == field.object_ref)
            .unwrap()
            .value,
        original
    );
}

#[test]
fn cff_and_unknown_font_programs_refuse_appearance_mutation_explicitly() {
    let mut cff2 = vec![0u8; 28];
    cff2[0..4].copy_from_slice(b"OTTO");
    cff2[4..6].copy_from_slice(&1u16.to_be_bytes());
    cff2[12..16].copy_from_slice(b"CFF2");
    cff2[20..24].copy_from_slice(&28u32.to_be_bytes());
    for (subtype, data, expected) in [
        ("Type1C", vec![1, 0, 4, 4], PdfError::CffDetectedUnsupported),
        ("OpenType", cff2, PdfError::Cff2DetectedUnsupported),
        ("OpenType", vec![1, 0, 4, 4], PdfError::UnknownFontProgram),
    ] {
        let bytes = MinimalWriter::create_minimal_pdf("Font program refusal").unwrap();
        let mut document = PdfDocument::from_bytes(&bytes).unwrap();
        let field_ref = ObjectRef::new(9_100, 0);
        let font_ref = ObjectRef::new(9_101, 0);
        let descriptor_ref = ObjectRef::new(9_102, 0);
        let stream_ref = ObjectRef::new(9_103, 0);
        document.store_mut().insert_cached(
            stream_ref,
            PdfObject::Stream(StreamObject {
                dict: std::collections::BTreeMap::from([
                    ("Length".into(), PdfObject::Integer(data.len() as i64)),
                    ("Subtype".into(), PdfObject::Name(subtype.into())),
                ]),
                data: data.clone(),
                stream_offset: 0,
                stream_length: data.len(),
            }),
        );
        document.store_mut().insert_cached(
            descriptor_ref,
            PdfObject::Dictionary(std::collections::BTreeMap::from([(
                "FontFile3".into(),
                PdfObject::Reference(stream_ref),
            )])),
        );
        document.store_mut().insert_cached(
            font_ref,
            PdfObject::Dictionary(std::collections::BTreeMap::from([
                ("Type".into(), PdfObject::Name("Font".into())),
                ("Subtype".into(), PdfObject::Name("Type1".into())),
                ("BaseFont".into(), PdfObject::Name("SyntheticCFF".into())),
                ("Encoding".into(), PdfObject::Name("WinAnsiEncoding".into())),
                (
                    "FontDescriptor".into(),
                    PdfObject::Reference(descriptor_ref),
                ),
            ])),
        );
        document.store_mut().insert_cached(
            field_ref,
            PdfObject::Dictionary(std::collections::BTreeMap::from([
                ("FT".into(), PdfObject::Name("Tx".into())),
                ("DA".into(), PdfObject::String(b"/CFF 12 Tf 0 g".to_vec())),
                (
                    "Rect".into(),
                    PdfObject::Array(vec![
                        PdfObject::Integer(20),
                        PdfObject::Integer(20),
                        PdfObject::Integer(220),
                        PdfObject::Integer(50),
                    ]),
                ),
                (
                    "DR".into(),
                    PdfObject::Dictionary(std::collections::BTreeMap::from([(
                        "Font".into(),
                        PdfObject::Dictionary(std::collections::BTreeMap::from([(
                            "CFF".into(),
                            PdfObject::Reference(font_ref),
                        )])),
                    )])),
                ),
            ])),
        );
        let error = document
            .apply_mutation(&[PdfChange::SetTextField {
                field_ref,
                value: "A".into(),
            }])
            .unwrap_err();
        assert_eq!(error.to_string(), expected.to_string());
    }
}

#[test]
fn producer_authored_forms_and_annotations_are_enumerated() {
    let expected = [
        ("pdfkit-text-checkbox", 4, 4),
        ("pdfkit-choice-radio", 3, 3),
        ("pdfkit-markup-freetext", 0, 4),
        ("pdfkit-shapes-ink-link", 0, 5),
        ("pdfkit-rotated-widget", 1, 1),
        ("pdflib-complete-form", 6, 8),
        ("pdflib-inherited-field", 1, 1),
        ("pdflib-needappearances-ap-rd", 1, 1),
        ("pdflib-starpdf-two-revisions", 6, 9),
    ];
    for (id, field_count, annotation_count) in expected {
        let bytes = std::fs::read(fixture_path(id))
            .unwrap_or_else(|error| panic!("failed to read {id}: {error}"));
        let mut document = PdfDocument::from_bytes(&bytes)
            .unwrap_or_else(|error| panic!("failed to open {id}: {error}"));
        assert_eq!(document.page_count().unwrap(), 1, "{id}");
        assert_eq!(document.extract_all_text().unwrap().len(), 1, "{id}");
        assert_eq!(document.form_fields().unwrap().len(), field_count, "{id}");
        assert_eq!(
            document.page_annotations(0).unwrap().len(),
            annotation_count,
            "{id}"
        );
        let output = document
            .mutate_and_export(&[PdfChange::AddAnnotation {
                page_index: 0,
                spec: AnnotationSpec::Square {
                    rect: [12.0, 12.0, 36.0, 36.0],
                    stroke_color: Some(vec![0.0, 0.4, 0.8]),
                    fill_color: None,
                    border_width: Some(1.0),
                },
            }])
            .unwrap_or_else(|error| panic!("failed to mutate/export {id}: {error}"));
        assert!(output.starts_with(&bytes), "{id}");
        let mut reopened = PdfDocument::from_bytes(&output)
            .unwrap_or_else(|error| panic!("failed to reopen {id}: {error}"));
        assert_eq!(
            reopened.page_annotations(0).unwrap().len(),
            annotation_count + 1,
            "{id}"
        );
    }
}

#[test]
fn prior_incremental_history_accepts_another_collision_free_revision() {
    let bytes = std::fs::read(fixture_path("pdflib-starpdf-two-revisions")).unwrap();
    assert!(
        bytes
            .windows(5)
            .filter(|window| *window == b"/Prev")
            .count()
            >= 2
    );
    let mut document = PdfDocument::from_bytes(&bytes).unwrap();
    let catalog_ref = document.catalog_ref();
    let annotation_count = document.page_annotations(0).unwrap().len();
    let field = document
        .form_fields()
        .unwrap()
        .into_iter()
        .find(|field| field.fully_qualified_name == "shared.contact")
        .unwrap();
    let output = document
        .mutate_and_export(&[PdfChange::SetTextField {
            field_ref: field.object_ref,
            value: "StarPDF incremental revision three".into(),
        }])
        .unwrap();
    assert!(output.starts_with(&bytes));
    let mut reopened = PdfDocument::from_bytes(&output).unwrap();
    assert_eq!(reopened.catalog_ref(), catalog_ref);
    assert_eq!(
        reopened.page_annotations(0).unwrap().len(),
        annotation_count
    );
    assert_eq!(
        reopened
            .form_fields()
            .unwrap()
            .into_iter()
            .find(|candidate| candidate.object_ref == field.object_ref)
            .unwrap()
            .value,
        FieldValue::Text("StarPDF incremental revision three".into())
    );
}
