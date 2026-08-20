use starpdf::annotation::types::{AnnotationSpec, AnnotationUpdateSpec};
use starpdf::annotation::AnnotationGenerator;
use starpdf::appearance::da_parser::DefaultAppearance;
use starpdf::appearance::generator::AppearanceGenerator;
use starpdf::appearance::text_field::TextFieldAppearance;
use starpdf::document::PdfDocument;
use starpdf::forms::field::FieldType;
use starpdf::mutation::PdfChange;
use starpdf::writer::MinimalWriter;

#[test]
fn test_fuzz_appearance_da_parser_hostile_strings() {
    let hostile_inputs = [
        "",
        "   ",
        "/",
        "/123",
        "Tf Tf Tf",
        "/Helv -10 Tf",
        "/Helv 999999999999999999999999999999999999999999999999999999999 Tf",
        "/Helv NaN Tf",
        "/Helv Inf Tf",
        "0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 rg",
        "0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 k",
        "g g g g g g g g",
        "/F1 12 Tf 0 0 1 rg /F2 14 Tf 0.5 g",
        "(((((((((((escaped))))))))))) /Helv 12 Tf",
        "\0\0\0\0\0\0\0\0 /Helv 12 Tf",
    ];

    for input in &hostile_inputs {
        let res = DefaultAppearance::parse(input);
        assert!(
            res.is_ok(),
            "DA parser must not panic on hostile input: {input}"
        );
    }
}

#[test]
fn test_fuzz_text_field_appearance_non_finite_and_degenerate_rects() {
    let da = DefaultAppearance::default();
    let degenerate_rects = [
        [f64::NAN, 0.0, 100.0, 100.0],
        [0.0, f64::INFINITY, 100.0, 100.0],
        [0.0, 0.0, f64::NEG_INFINITY, 100.0],
        [100.0, 100.0, 50.0, 50.0], // Inverted dimensions
        [0.0, 0.0, 0.0, 0.0],       // Zero area
        [-100.0, -100.0, -100.0, -100.0],
    ];

    for rect in &degenerate_rects {
        let res = TextFieldAppearance::generate_stream(*rect, "Test", &da, 0, false);
        assert!(
            res.is_err(),
            "Must safely reject degenerate rectangle: {:?}",
            rect
        );
    }
}

#[test]
fn test_fuzz_annotation_generator_corrupted_geometry() {
    let hostile_specs = [
        AnnotationSpec::FreeText {
            rect: [f64::NAN, 0.0, 100.0, 100.0],
            text: "Corrupted NaN".to_string(),
            font_size: Some(f64::INFINITY),
            color: Some(vec![f64::NAN]),
        },
        AnnotationSpec::Highlight {
            rect: [0.0, 0.0, 100.0, 100.0],
            quad_points: vec![f64::NAN, 0.0, 100.0, 100.0],
            color: None,
        },
        AnnotationSpec::Ink {
            rect: [0.0, 0.0, 100.0, 100.0],
            ink_list: vec![vec![[f64::NAN, 0.0], [0.0, f64::INFINITY]]],
            stroke_color: None,
            stroke_width: Some(f64::NAN),
        },
    ];

    for spec in &hostile_specs {
        let res = AnnotationGenerator::generate_annotation_objects(spec);
        // Either gracefully handles or returns a typed error, never panics
        let _ = res;
    }
}

#[test]
fn test_fuzz_appearance_generator_unsupported_and_hostile_states() {
    let da = DefaultAppearance::default();
    let rect = [10.0, 10.0, 50.0, 50.0];

    // Unknown field type
    let res = AppearanceGenerator::generate_widget_ap(
        &FieldType::Unknown("CustomUnsupportedType".to_string()),
        rect,
        "Val",
        &da,
        0,
        None,
        false,
    );
    assert!(res.is_ok());
    let (_, status) = res.unwrap();
    assert_eq!(
        status,
        starpdf::appearance::AppearanceStatus::AppearanceUnsupported
    );
}

#[test]
fn test_fuzz_mutation_transaction_abort_preserves_clean_state() {
    let pdf_bytes = MinimalWriter::create_minimal_pdf("Transactional Integrity").unwrap();
    let mut doc = PdfDocument::from_bytes(&pdf_bytes).unwrap();

    let batch = vec![
        PdfChange::AddAnnotation {
            page_index: 0,
            spec: AnnotationSpec::Square {
                rect: [10.0, 10.0, 50.0, 50.0],
                stroke_color: None,
                fill_color: None,
                border_width: None,
            },
        },
        PdfChange::UpdateAnnotation {
            annot_ref: starpdf::syntax::object::ObjectRef::new(99999, 0),
            update: AnnotationUpdateSpec::default(),
        },
    ];

    let res = doc.apply_mutation(&batch);
    assert!(
        res.is_err(),
        "Transaction with invalid reference must abort"
    );
    assert_eq!(
        doc.page_annotations(0).unwrap().len(),
        0,
        "No changes should leak into doc state"
    );
}

#[test]
fn test_v0_7_annotation_resource_limits_and_non_finite_values_are_rejected() {
    let invalid_specs = [
        AnnotationSpec::Highlight {
            rect: [0.0, 0.0, 100.0, 20.0],
            quad_points: vec![0.0; 8_008],
            color: None,
        },
        AnnotationSpec::Underline {
            rect: [0.0, 0.0, 100.0, 20.0],
            quad_points: vec![0.0; 7],
            color: None,
        },
        AnnotationSpec::Ink {
            rect: [0.0, 0.0, 100.0, 100.0],
            ink_list: vec![vec![[0.0, 0.0]]; 1_001],
            stroke_color: None,
            stroke_width: None,
        },
        AnnotationSpec::Square {
            rect: [100.0, 0.0, 0.0, 100.0],
            stroke_color: None,
            fill_color: None,
            border_width: None,
        },
        AnnotationSpec::Circle {
            rect: [0.0, 0.0, 100.0, 100.0],
            stroke_color: Some(vec![f64::NAN, 0.0, 0.0]),
            fill_color: None,
            border_width: None,
        },
        AnnotationSpec::Line {
            line_points: [0.0, 0.0, f64::INFINITY, 10.0],
            stroke_color: None,
            fill_color: None,
            stroke_width: Some(f64::NAN),
            line_endings: Default::default(),
            contents: None,
        },
    ];

    for spec in invalid_specs {
        assert!(AnnotationGenerator::generate_annotation_objects(&spec).is_err());
    }
}

#[test]
fn test_v0_7_annotation_contents_and_incremental_batch_limits_are_atomic() {
    let bytes = MinimalWriter::create_minimal_pdf("Bounded transaction").unwrap();
    let mut doc = PdfDocument::from_bytes(&bytes).unwrap();
    let oversized = "x".repeat(1_048_577);
    let changes = [
        PdfChange::AddAnnotation {
            page_index: 0,
            spec: AnnotationSpec::Square {
                rect: [10.0, 10.0, 20.0, 20.0],
                stroke_color: None,
                fill_color: None,
                border_width: None,
            },
        },
        PdfChange::AddAnnotation {
            page_index: 0,
            spec: AnnotationSpec::FreeText {
                rect: [30.0, 10.0, 80.0, 30.0],
                text: oversized,
                font_size: None,
                color: None,
            },
        },
    ];

    assert!(doc.mutate_and_export(&changes).is_err());
    assert!(doc.page_annotations(0).unwrap().is_empty());
}
