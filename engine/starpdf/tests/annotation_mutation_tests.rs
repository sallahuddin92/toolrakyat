use std::collections::BTreeMap;

use starpdf::annotation::types::{AnnotationSpec, AnnotationUpdateSpec};
use starpdf::appearance::AppearanceStatus;
use starpdf::document::PdfDocument;
use starpdf::mutation::{MutationPlan, PdfChange};
use starpdf::syntax::object::{ObjectRef, PdfObject};
use starpdf::writer::MinimalWriter;

fn all_supported_annotations() -> Vec<AnnotationSpec> {
    vec![
        AnnotationSpec::FreeText {
            rect: [20.0, 20.0, 180.0, 55.0],
            text: "Visible note".to_string(),
            font_size: Some(11.0),
            color: Some(vec![0.0]),
        },
        AnnotationSpec::Highlight {
            rect: [20.0, 65.0, 180.0, 85.0],
            quad_points: vec![20.0, 85.0, 180.0, 85.0, 20.0, 65.0, 180.0, 65.0],
            color: Some(vec![1.0, 1.0, 0.0]),
        },
        AnnotationSpec::Underline {
            rect: [20.0, 95.0, 180.0, 115.0],
            quad_points: vec![20.0, 115.0, 180.0, 115.0, 20.0, 95.0, 180.0, 95.0],
            color: Some(vec![0.0, 0.0, 1.0]),
        },
        AnnotationSpec::StrikeOut {
            rect: [20.0, 125.0, 180.0, 145.0],
            quad_points: vec![20.0, 145.0, 180.0, 145.0, 20.0, 125.0, 180.0, 125.0],
            color: Some(vec![1.0, 0.0, 0.0]),
        },
        AnnotationSpec::Square {
            rect: [200.0, 20.0, 280.0, 80.0],
            stroke_color: Some(vec![1.0, 0.0, 0.0]),
            fill_color: None,
            border_width: Some(2.0),
        },
        AnnotationSpec::Circle {
            rect: [300.0, 20.0, 380.0, 80.0],
            stroke_color: Some(vec![0.0, 0.5, 0.0]),
            fill_color: None,
            border_width: Some(2.0),
        },
        AnnotationSpec::Line {
            line_points: [200.0, 100.0, 380.0, 140.0],
            stroke_color: Some(vec![0.0]),
            stroke_width: Some(1.5),
        },
        AnnotationSpec::Ink {
            rect: [200.0, 160.0, 380.0, 220.0],
            ink_list: vec![vec![[205.0, 170.0], [250.0, 210.0], [375.0, 175.0]]],
            stroke_color: Some(vec![0.2, 0.2, 0.8]),
            stroke_width: Some(2.0),
        },
        AnnotationSpec::Link {
            rect: [20.0, 240.0, 180.0, 265.0],
            uri: "https://example.com/starpdf".to_string(),
        },
    ]
}

#[test]
fn test_add_freetext_and_shape_annotations() {
    let pdf_bytes = MinimalWriter::create_minimal_pdf("Hello StarPDF v0.7").unwrap();
    let mut doc = PdfDocument::from_bytes(&pdf_bytes).unwrap();

    let initial_annots = doc.page_annotations(0).unwrap();
    assert_eq!(initial_annots.len(), 0);

    // 1. Add FreeText annotation
    let freetext_change = PdfChange::AddAnnotation {
        page_index: 0,
        spec: AnnotationSpec::FreeText {
            rect: [50.0, 100.0, 200.0, 150.0],
            text: "Approved by StarPDF".to_string(),
            font_size: Some(12.0),
            color: Some(vec![0.0, 0.0, 0.0]),
        },
    };

    // 2. Add Square annotation
    let square_change = PdfChange::AddAnnotation {
        page_index: 0,
        spec: AnnotationSpec::Square {
            rect: [220.0, 100.0, 320.0, 200.0],
            stroke_color: Some(vec![1.0, 0.0, 0.0]),
            fill_color: Some(vec![0.9, 0.9, 0.9]),
            border_width: Some(2.0),
        },
    };

    let plan = doc
        .apply_mutation(&[freetext_change, square_change])
        .unwrap();
    assert_eq!(plan.modified_objects.len(), 5); // Page + 2 annot dicts + 2 AP streams

    let mutated_bytes = doc.export_incremental(&plan).unwrap();

    // Reopen mutated document
    let mut mutated_doc = PdfDocument::from_bytes(&mutated_bytes).unwrap();
    let updated_annots = mutated_doc.page_annotations(0).unwrap();
    assert_eq!(updated_annots.len(), 2);
    assert_eq!(updated_annots[0].subtype.as_name(), "FreeText");
    assert_eq!(updated_annots[1].subtype.as_name(), "Square");
}

#[test]
fn test_update_and_remove_annotation() {
    let pdf_bytes = MinimalWriter::create_minimal_pdf("Annotation Lifecycle Test").unwrap();
    let mut doc = PdfDocument::from_bytes(&pdf_bytes).unwrap();

    // Step 1: Add FreeText
    let add_change = PdfChange::AddAnnotation {
        page_index: 0,
        spec: AnnotationSpec::FreeText {
            rect: [50.0, 50.0, 150.0, 80.0],
            text: "Initial Draft".to_string(),
            font_size: Some(10.0),
            color: Some(vec![0.0, 0.0, 0.0]),
        },
    };
    let step1_bytes = doc.mutate_and_export(&[add_change]).unwrap();

    let mut doc2 = PdfDocument::from_bytes(&step1_bytes).unwrap();
    let annots = doc2.page_annotations(0).unwrap();
    assert_eq!(annots.len(), 1);
    let annot_ref = annots[0].object_ref;

    // Step 2: Update Annotation contents
    let update_change = PdfChange::UpdateAnnotation {
        annot_ref,
        update: AnnotationUpdateSpec {
            rect: None,
            contents: Some("Final Review".to_string()),
            color: Some(vec![0.0, 0.5, 0.0]),
        },
    };
    let step2_bytes = doc2.mutate_and_export(&[update_change]).unwrap();

    let mut doc3 = PdfDocument::from_bytes(&step2_bytes).unwrap();
    let annots2 = doc3.page_annotations(0).unwrap();
    assert_eq!(annots2.len(), 1);
    assert_eq!(annots2[0].contents.as_deref(), Some("Final Review"));

    // Step 3: Remove Annotation
    let remove_change = PdfChange::RemoveAnnotation {
        page_index: 0,
        annot_ref,
    };
    let step3_bytes = doc3.mutate_and_export(&[remove_change]).unwrap();

    let mut doc4 = PdfDocument::from_bytes(&step3_bytes).unwrap();
    let annots3 = doc4.page_annotations(0).unwrap();
    assert_eq!(annots3.len(), 0);
}

#[test]
fn test_atomic_transaction_refusal_on_invalid_change() {
    let pdf_bytes = MinimalWriter::create_minimal_pdf("Atomic Guard Test").unwrap();
    let mut doc = PdfDocument::from_bytes(&pdf_bytes).unwrap();

    let valid_change = PdfChange::AddAnnotation {
        page_index: 0,
        spec: AnnotationSpec::Square {
            rect: [10.0, 10.0, 50.0, 50.0],
            stroke_color: None,
            fill_color: None,
            border_width: None,
        },
    };

    // Invalid change: Page index out of bounds (999)
    let invalid_change = PdfChange::AddAnnotation {
        page_index: 999,
        spec: AnnotationSpec::FreeText {
            rect: [10.0, 10.0, 50.0, 50.0],
            text: "Will fail".to_string(),
            font_size: None,
            color: None,
        },
    };

    let result = doc.apply_mutation(&[valid_change, invalid_change]);
    assert!(
        result.is_err(),
        "Transaction must fail atomically on invalid change"
    );

    // Ensure document state unchanged
    assert_eq!(doc.page_annotations(0).unwrap().len(), 0);
}

#[test]
fn test_all_supported_annotation_types_roundtrip_with_geometry_and_page_association() {
    let original = MinimalWriter::create_minimal_pdf("All annotation types").unwrap();
    let mut doc = PdfDocument::from_bytes(&original).unwrap();
    let specs = all_supported_annotations();
    let expected_rects: Vec<[f64; 4]> = specs.iter().map(AnnotationSpec::rect).collect();
    let changes: Vec<PdfChange> = specs
        .into_iter()
        .map(|spec| PdfChange::AddAnnotation {
            page_index: 0,
            spec,
        })
        .collect();

    let output = doc.mutate_and_export(&changes).unwrap();
    assert!(output.starts_with(&original));

    let mut reopened = PdfDocument::from_bytes(&output).unwrap();
    let annotations = reopened.page_annotations(0).unwrap();
    let expected_subtypes = [
        "FreeText",
        "Highlight",
        "Underline",
        "StrikeOut",
        "Square",
        "Circle",
        "Line",
        "Ink",
        "Link",
    ];
    assert_eq!(annotations.len(), expected_subtypes.len());

    for ((annotation, expected_subtype), expected_rect) in annotations
        .iter()
        .zip(expected_subtypes)
        .zip(expected_rects)
    {
        assert_eq!(annotation.subtype.as_name(), expected_subtype);
        assert_eq!(annotation.page_index, 0);
        assert_eq!(annotation.rect, expected_rect);
        let object = reopened
            .store_mut()
            .resolve(annotation.object_ref)
            .unwrap()
            .clone();
        let dict = object.as_dict().unwrap();
        assert_eq!(
            dict.get("P").and_then(|value| value.as_reference()),
            Some(reopened.page_ref(0).unwrap())
        );

        match expected_subtype {
            "Highlight" | "Underline" | "StrikeOut" => {
                assert_eq!(
                    dict.get("QuadPoints")
                        .and_then(|value| value.as_array())
                        .map(<[_]>::len),
                    Some(8)
                );
            }
            "Line" => {
                assert_eq!(
                    dict.get("L")
                        .and_then(|value| value.as_array())
                        .map(<[_]>::len),
                    Some(4)
                );
            }
            "Ink" => assert!(dict
                .get("InkList")
                .and_then(|value| value.as_array())
                .is_some()),
            "Link" => assert!(dict.get("A").and_then(|value| value.as_dict()).is_some()),
            "FreeText" | "Square" | "Circle" => assert!(dict.get("AP").is_some()),
            _ => unreachable!(),
        }
    }
}

#[test]
fn test_unknown_annotation_subtype_is_preserved_during_supported_update() {
    let original = MinimalWriter::create_minimal_pdf("Unknown annotation preservation").unwrap();
    let mut doc = PdfDocument::from_bytes(&original).unwrap();
    let page_ref = doc.page_ref(0).unwrap();
    let unknown_ref = ObjectRef::new(9_000, 0);
    let mut page_dict = doc.page_dict(0).unwrap();
    page_dict.insert(
        "Annots".to_string(),
        PdfObject::Array(vec![PdfObject::Reference(unknown_ref)]),
    );
    let unknown_dict = BTreeMap::from([
        ("Type".to_string(), PdfObject::Name("Annot".to_string())),
        (
            "Subtype".to_string(),
            PdfObject::Name("VendorPrivate".to_string()),
        ),
        (
            "Rect".to_string(),
            PdfObject::Array(vec![
                PdfObject::Real(10.0),
                PdfObject::Real(10.0),
                PdfObject::Real(80.0),
                PdfObject::Real(40.0),
            ]),
        ),
        ("P".to_string(), PdfObject::Reference(page_ref)),
        (
            "VendorData".to_string(),
            PdfObject::String(b"preserve-me".to_vec()),
        ),
    ]);
    let initial_plan = MutationPlan {
        modified_objects: BTreeMap::from([
            (page_ref, PdfObject::Dictionary(page_dict)),
            (unknown_ref, PdfObject::Dictionary(unknown_dict)),
        ]),
        appearance_status: AppearanceStatus::AppearancePreserved,
    };
    let with_unknown = doc.export_incremental(&initial_plan).unwrap();

    let mut reopened = PdfDocument::from_bytes(&with_unknown).unwrap();
    let parsed = reopened.page_annotations(0).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].subtype.as_name(), "VendorPrivate");

    let updated = reopened
        .mutate_and_export(&[PdfChange::UpdateAnnotation {
            annot_ref: unknown_ref,
            update: AnnotationUpdateSpec {
                contents: Some("safe update".to_string()),
                ..AnnotationUpdateSpec::default()
            },
        }])
        .unwrap();
    let mut final_doc = PdfDocument::from_bytes(&updated).unwrap();
    let final_object = final_doc.store_mut().resolve(unknown_ref).unwrap().clone();
    let final_dict = final_object.as_dict().unwrap();
    assert_eq!(
        final_dict
            .get("VendorData")
            .and_then(PdfObject::as_string_lossy)
            .as_deref(),
        Some("preserve-me")
    );
    assert_eq!(
        final_dict
            .get("Contents")
            .and_then(PdfObject::as_string_lossy)
            .as_deref(),
        Some("safe update")
    );
}
