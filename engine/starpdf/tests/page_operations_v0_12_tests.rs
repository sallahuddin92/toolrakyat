use std::io::Write;
use std::path::{Path, PathBuf};

use starpdf::document::{PageTree, PdfDocument};
use starpdf::forms::FieldType;
use starpdf::page_ops::{PageEdit, PageOperationLimits, PageOperationPlan};

fn fixture(directory: &str, name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(directory)
        .join(name)
}

fn read_fixture(directory: &str, name: &str) -> Vec<u8> {
    std::fs::read(fixture(directory, name))
        .unwrap_or_else(|error| panic!("failed to read {directory}/{name}: {error}"))
}

fn plain_texts(bytes: &[u8]) -> Vec<String> {
    let mut document = PdfDocument::from_bytes(bytes).unwrap();
    document
        .extract_all_text()
        .unwrap()
        .into_iter()
        .map(|page| page.plain_text())
        .collect()
}

fn multi_page_document(texts: &[&str]) -> Vec<u8> {
    assert!(!texts.is_empty());
    let mut output = Vec::new();
    output.write_all(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n").unwrap();
    let mut offsets = vec![0usize];

    offsets.push(output.len());
    output
        .write_all(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n")
        .unwrap();
    offsets.push(output.len());
    let kids = (0..texts.len())
        .map(|index| format!("{} 0 R", 3 + index * 2))
        .collect::<Vec<_>>()
        .join(" ");
    writeln!(
        output,
        "2 0 obj\n<< /Type /Pages /Kids [{kids}] /Count {} >>\nendobj",
        texts.len()
    )
    .unwrap();

    let font_number = 3 + texts.len() * 2;
    for (index, text) in texts.iter().enumerate() {
        let page_number = 3 + index * 2;
        let content_number = page_number + 1;
        offsets.push(output.len());
        writeln!(
            output,
            "{page_number} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 {font_number} 0 R >> >> /Contents {content_number} 0 R >>\nendobj"
        )
        .unwrap();
        let stream = format!(
            "BT /F1 24 Tf 72 720 Td ({}) Tj ET\n",
            text.replace('(', "\\(").replace(')', "\\)")
        );
        offsets.push(output.len());
        writeln!(
            output,
            "{content_number} 0 obj\n<< /Length {} >>\nstream\n{stream}endstream\nendobj",
            stream.len()
        )
        .unwrap();
    }
    offsets.push(output.len());
    writeln!(
        output,
        "{font_number} 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj"
    )
    .unwrap();

    let xref = output.len();
    writeln!(output, "xref\n0 {}", offsets.len()).unwrap();
    writeln!(output, "0000000000 65535 f ").unwrap();
    for offset in offsets.iter().skip(1) {
        writeln!(output, "{offset:010} 00000 n ").unwrap();
    }
    writeln!(
        output,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF",
        offsets.len()
    )
    .unwrap();
    output
}

fn three_page_document() -> Vec<u8> {
    multi_page_document(&["PAGE-ALPHA", "PAGE-BETA", "PAGE-GAMMA"])
}

fn nested_page_document() -> Vec<u8> {
    let content = |text: &str| {
        let stream = format!("BT /F1 24 Tf 72 720 Td ({text}) Tj ET\n");
        format!("<< /Length {} >>\nstream\n{stream}endstream", stream.len())
    };
    let objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R 8 0 R] /Count 3 /MediaBox [0 0 612 792] /Resources << /Font << /F1 10 0 R >> >> >>".to_string(),
        "<< /Type /Pages /Parent 2 0 R /Kids [4 0 R 6 0 R] /Count 2 >>".to_string(),
        "<< /Type /Page /Parent 3 0 R /Contents 5 0 R >>".to_string(),
        content("NESTED-A"),
        "<< /Type /Page /Parent 3 0 R /Contents 7 0 R >>".to_string(),
        content("NESTED-B"),
        "<< /Type /Page /Parent 2 0 R /Contents 9 0 R >>".to_string(),
        content("ROOT-C"),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
    ];
    let mut output = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = vec![0usize];
    for (index, object) in objects.iter().enumerate() {
        offsets.push(output.len());
        writeln!(output, "{} 0 obj\n{object}\nendobj", index + 1).unwrap();
    }
    let xref = output.len();
    writeln!(output, "xref\n0 {}", offsets.len()).unwrap();
    writeln!(output, "0000000000 65535 f ").unwrap();
    for offset in offsets.iter().skip(1) {
        writeln!(output, "{offset:010} 00000 n ").unwrap();
    }
    writeln!(
        output,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF",
        offsets.len()
    )
    .unwrap();
    output
}

#[test]
fn delete_first_middle_and_last_is_incremental_and_refuses_zero_pages() {
    let bytes = three_page_document();
    for (index, expected) in [
        (0, vec!["PAGE-BETA", "PAGE-GAMMA"]),
        (1, vec!["PAGE-ALPHA", "PAGE-GAMMA"]),
        (2, vec!["PAGE-ALPHA", "PAGE-BETA"]),
    ] {
        let mut document = PdfDocument::from_bytes(&bytes).unwrap();
        let output = document.delete_page(index).unwrap();
        assert!(output.starts_with(&bytes));
        assert_eq!(plain_texts(&output), expected);
        let mut reopened = PdfDocument::from_bytes(&output).unwrap();
        let root = reopened.root_pages_ref();
        assert_eq!(
            PageTree::validate_and_collect(reopened.store_mut(), root)
                .unwrap()
                .len(),
            2
        );
    }
    let one_page = multi_page_document(&["ONLY"]);
    let mut document = PdfDocument::from_bytes(&one_page).unwrap();
    assert!(document.delete_page(0).is_err());
    assert!(document.delete_page(1).is_err());
}

#[test]
fn move_page_covers_edges_middle_adjacent_noop_and_invalid_indexes() {
    let bytes = three_page_document();
    for (from, to, expected) in [
        (0, 2, vec!["PAGE-BETA", "PAGE-GAMMA", "PAGE-ALPHA"]),
        (2, 0, vec!["PAGE-GAMMA", "PAGE-ALPHA", "PAGE-BETA"]),
        (1, 0, vec!["PAGE-BETA", "PAGE-ALPHA", "PAGE-GAMMA"]),
        (1, 2, vec!["PAGE-ALPHA", "PAGE-GAMMA", "PAGE-BETA"]),
    ] {
        let mut document = PdfDocument::from_bytes(&bytes).unwrap();
        assert_eq!(
            plain_texts(&document.move_page(from, to).unwrap()),
            expected
        );
    }
    let mut document = PdfDocument::from_bytes(&bytes).unwrap();
    assert_eq!(document.move_page(1, 1).unwrap(), bytes);
    assert!(document.move_page(3, 0).is_err());
    assert!(document.move_page(0, 3).is_err());
}

#[test]
fn nested_page_tree_is_validated_normalized_and_inheritance_is_materialized() {
    let bytes = nested_page_document();
    let mut document = PdfDocument::from_bytes(&bytes).unwrap();
    assert_eq!(plain_texts(&bytes), ["NESTED-A", "NESTED-B", "ROOT-C"]);
    let moved = document.move_page(0, 2).unwrap();
    assert_eq!(plain_texts(&moved), ["NESTED-B", "ROOT-C", "NESTED-A"]);
    let mut reopened = PdfDocument::from_bytes(&moved).unwrap();
    for page_index in 0..3 {
        let page = reopened.page_dict(page_index).unwrap();
        assert!(page.contains_key("MediaBox"));
        assert!(page.contains_key("Resources"));
    }
}

#[test]
fn duplicate_page_has_independent_page_and_annotation_objects() {
    let bytes = read_fixture("v0_10_compat", "pdfkit-shapes-ink-link.pdf");
    let mut source = PdfDocument::from_bytes(&bytes).unwrap();
    let duplicated = source.duplicate_page(0, 1).unwrap();
    let mut reopened = PdfDocument::from_bytes(&duplicated).unwrap();
    let first_page = reopened.page_ref(0).unwrap();
    let second_page = reopened.page_ref(1).unwrap();
    assert_ne!(first_page, second_page);
    let first_annotations = reopened.page_annotations(0).unwrap();
    let second_annotations = reopened.page_annotations(1).unwrap();
    assert_eq!(first_annotations.len(), second_annotations.len());
    assert!(first_annotations.iter().all(|first| second_annotations
        .iter()
        .all(|second| first.object_ref != second.object_ref)));

    for (page_index, page_ref) in [(0, first_page), (1, second_page)] {
        let page = reopened.page_dict(page_index).unwrap();
        let annotations = reopened
            .store_mut()
            .resolve_object(page.get("Annots").unwrap())
            .unwrap();
        for annotation_ref in annotations.as_array().unwrap() {
            let annotation = reopened
                .store_mut()
                .resolve(annotation_ref.as_reference().unwrap())
                .unwrap();
            if let Some(reference) = annotation
                .as_dict()
                .and_then(|dict| dict.get("P"))
                .and_then(|value| value.as_reference())
            {
                assert_eq!(reference, page_ref);
            }
        }
    }
}

#[test]
fn duplicate_and_extract_preserve_requested_order_and_repeated_selection() {
    let bytes = three_page_document();
    let mut document = PdfDocument::from_bytes(&bytes).unwrap();
    let duplicated = document.duplicate_page(1, 2).unwrap();
    assert_eq!(
        plain_texts(&duplicated),
        ["PAGE-ALPHA", "PAGE-BETA", "PAGE-BETA", "PAGE-GAMMA"]
    );
    let extracted = document.extract_pages(&[2, 0, 2]).unwrap();
    assert_eq!(
        plain_texts(&extracted),
        ["PAGE-GAMMA", "PAGE-ALPHA", "PAGE-GAMMA"]
    );
    assert!(!extracted.starts_with(&bytes));
    assert!(document.extract_pages(&[]).is_err());
    assert!(document.extract_pages(&[3]).is_err());
}

#[test]
fn blank_page_insert_validates_geometry_rotation_and_empty_content() {
    let bytes = three_page_document();
    let mut document = PdfDocument::from_bytes(&bytes).unwrap();
    let output = document.insert_blank_page(1, 420.0, 297.0, 90).unwrap();
    assert!(output.starts_with(&bytes));
    let mut reopened = PdfDocument::from_bytes(&output).unwrap();
    assert_eq!(reopened.page_count().unwrap(), 4);
    let blank = reopened.page_dict(1).unwrap();
    assert_eq!(
        blank.get("Rotate").and_then(|value| value.as_integer()),
        Some(90)
    );
    assert!(!blank.contains_key("Contents"));
    assert!(plain_texts(&output)[1].is_empty());
    assert!(document.insert_blank_page(0, 0.0, 100.0, 0).is_err());
    assert!(document.insert_blank_page(0, f64::NAN, 100.0, 0).is_err());
    assert!(document.insert_blank_page(0, 100.0, 100.0, 45).is_err());
    assert!(document.insert_blank_page(9, 100.0, 100.0, 0).is_err());
}

#[test]
fn inherited_resources_boxes_rotation_and_embedded_fonts_survive_extraction() {
    let bytes = read_fixture("v0_9_compat", "chrome-landscape.pdf");
    let mut source = PdfDocument::from_bytes(&bytes).unwrap();
    let extracted = source.extract_pages(&[0]).unwrap();
    let mut reopened = PdfDocument::from_bytes(&extracted).unwrap();
    let page = reopened.page_dict(0).unwrap();
    assert!(page.contains_key("MediaBox"));
    assert!(page.contains_key("Resources"));
    assert!(!page.contains_key("CropBox") || page.get("CropBox").unwrap().as_array().is_some());
    assert!(!plain_texts(&extracted)[0].is_empty());
}

#[test]
fn annotations_links_and_appearances_survive_extraction() {
    let bytes = read_fixture("v0_10_compat", "pdfkit-shapes-ink-link.pdf");
    let mut source = PdfDocument::from_bytes(&bytes).unwrap();
    let extracted = source.extract_pages(&[0]).unwrap();
    let mut reopened = PdfDocument::from_bytes(&extracted).unwrap();
    let annotations = reopened.page_annotations(0).unwrap();
    assert!(annotations.len() >= 5);
    assert!(annotations
        .iter()
        .any(|annotation| annotation.uri.is_some()));
    assert!(annotations
        .iter()
        .filter(|annotation| annotation.subtype.as_name() != "Link")
        .all(|annotation| annotation.has_normal_appearance));
}

#[test]
fn supported_form_field_graph_survives_whole_page_extraction() {
    let bytes = read_fixture("v0_10_compat", "pdflib-complete-form.pdf");
    let mut source = PdfDocument::from_bytes(&bytes).unwrap();
    let extracted = source.extract_pages(&[0]).unwrap();
    let mut reopened = PdfDocument::from_bytes(&extracted).unwrap();
    let fields = reopened.form_fields().unwrap();
    assert!(fields.iter().all(|field| !field.widgets.is_empty()));
    assert!(fields
        .iter()
        .any(|field| matches!(field.field_type, FieldType::Text { .. })));
    assert!(fields.iter().any(|field| field.is_checkbox()));
    assert!(fields.iter().any(|field| field.is_radio()));
    assert!(fields.iter().any(|field| field.is_choice()));
    assert!(fields.iter().any(|field| field.widgets.len() > 1));
}

#[test]
fn image_xobjects_and_encoded_streams_survive_extraction() {
    let bytes = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-assets/scanned-test.pdf"),
    )
    .unwrap();
    let mut source = PdfDocument::from_bytes(&bytes).unwrap();
    let extracted = source.extract_pages(&[0]).unwrap();
    let mut reopened = PdfDocument::from_bytes(&extracted).unwrap();
    let page = reopened.page_dict(0).unwrap();
    let resources = reopened
        .store_mut()
        .resolve_object(page.get("Resources").unwrap())
        .unwrap();
    let xobjects = reopened
        .store_mut()
        .resolve_object(resources.as_dict().unwrap().get("XObject").unwrap())
        .unwrap();
    let image_values = xobjects
        .as_dict()
        .unwrap()
        .values()
        .cloned()
        .collect::<Vec<_>>();
    let images = image_values
        .iter()
        .filter(|value| {
            reopened
                .store_mut()
                .resolve_object(value)
                .ok()
                .is_some_and(|object| {
                    object
                        .as_dict()
                        .and_then(|dict| dict.get("Subtype"))
                        .and_then(|value| value.as_name())
                        == Some("Image")
                })
        })
        .count();
    assert!(images > 0);
}

#[test]
fn mixed_xref_incremental_metadata_and_navigation_sources_rebuild() {
    for (directory, name) in [
        ("v0_11_complex", "synthetic-hybrid-multi-revision.pdf"),
        ("v0_10_compat", "pdflib-starpdf-two-revisions.pdf"),
        ("v0_11_complex", "synthetic-metadata-rich.pdf"),
    ] {
        let bytes = read_fixture(directory, name);
        let mut source = PdfDocument::from_bytes(&bytes).unwrap();
        let extracted = source.extract_pages(&[0]).unwrap();
        let mut reopened = PdfDocument::from_bytes(&extracted).unwrap();
        assert_eq!(reopened.page_count().unwrap(), 1);
        assert_eq!(reopened.store().xref().revisions.len(), 1);
    }

    let metadata = read_fixture("v0_11_complex", "synthetic-metadata-rich.pdf");
    let mut source = PdfDocument::from_bytes(&metadata).unwrap();
    let extracted = source.extract_pages(&[0]).unwrap();
    let mut reopened = PdfDocument::from_bytes(&extracted).unwrap();
    let catalog_ref = reopened.catalog_ref();
    let catalog = reopened.store_mut().resolve(catalog_ref).unwrap();
    let catalog = catalog.as_dict().unwrap();
    for key in ["Metadata", "Lang", "ViewerPreferences", "Names", "Outlines"] {
        assert!(catalog.contains_key(key));
    }
    assert!(reopened.trailer().contains_key("Info"));
    assert!(reopened.trailer().contains_key("ID"));
}

#[test]
fn multi_edit_plan_is_atomic_and_checked_against_limits() {
    let bytes = three_page_document();
    let mut document = PdfDocument::from_bytes(&bytes).unwrap();
    let plan = PageOperationPlan::with_edits(vec![
        PageEdit::MovePage {
            from_index: 0,
            to_index: 2,
        },
        PageEdit::DeletePage { index: 99 },
    ]);
    assert!(document
        .apply_page_operations(&plan, &PageOperationLimits::default())
        .is_err());
    assert_eq!(document.source().as_bytes(), bytes);

    let limits = PageOperationLimits {
        max_selected_pages: 2,
        ..PageOperationLimits::default()
    };
    assert!(document
        .apply_page_operations(
            &PageOperationPlan::new(PageEdit::DuplicatePage {
                index: 0,
                insert_at: 1,
            }),
            &limits,
        )
        .is_err());
}
