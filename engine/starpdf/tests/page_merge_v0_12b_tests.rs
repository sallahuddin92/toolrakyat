use std::collections::BTreeSet;
use std::io::Write;
use std::path::{Path, PathBuf};

use starpdf::{FieldType, FieldValue, PageRange, PageSource, PdfChange, PdfDocument};

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

fn document(texts: &[&str]) -> Vec<u8> {
    assert!(!texts.is_empty());
    let mut output = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
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
        "2 0 obj\n<< /Type /Pages /Kids [{kids}] /Count {} /MediaBox [0 0 612 792] /Resources << /Font << /F1 {} 0 R >> >> >>\nendobj",
        texts.len(),
        3 + texts.len() * 2
    )
    .unwrap();
    for (index, text) in texts.iter().enumerate() {
        let page_number = 3 + index * 2;
        let content_number = page_number + 1;
        offsets.push(output.len());
        writeln!(
            output,
            "{page_number} 0 obj\n<< /Type /Page /Parent 2 0 R /Rotate {} /Contents {content_number} 0 R >>\nendobj",
            (index % 4) * 90
        )
        .unwrap();
        let stream = format!("BT /F1 24 Tf 72 720 Td ({text}) Tj ET\n");
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
        "{} 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj",
        3 + texts.len() * 2
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

fn texts(bytes: &[u8]) -> Vec<String> {
    let mut document = PdfDocument::from_bytes(bytes).unwrap();
    document
        .extract_all_text()
        .unwrap()
        .into_iter()
        .map(|page| page.plain_text())
        .collect()
}

#[test]
fn merges_two_three_and_selected_documents_without_object_number_collisions() {
    let first = document(&["A-ONE", "A-TWO"]);
    let second = document(&["B-ONE"]);
    let third = document(&["C-ONE", "C-TWO"]);
    let merged = PdfDocument::merge_documents(&[&first, &second, &third]).unwrap();
    assert_eq!(
        texts(&merged),
        ["A-ONE", "A-TWO", "B-ONE", "C-ONE", "C-TWO"]
    );

    let selected = PdfDocument::merge_selected(
        &[&first, &second, &third],
        &[
            PageSource::new(2, 1),
            PageSource::new(0, 0),
            PageSource::new(1, 0),
            PageSource::new(0, 0),
        ],
    )
    .unwrap();
    assert_eq!(texts(&selected), ["C-TWO", "A-ONE", "B-ONE", "A-ONE"]);
    assert_eq!(
        PdfDocument::from_bytes(&selected)
            .unwrap()
            .page_count()
            .unwrap(),
        4
    );
}

#[test]
fn imported_page_insert_preserves_primary_order_and_inherited_state() {
    let primary = document(&["PRIMARY-A", "PRIMARY-B"]);
    let imported = document(&["IMPORTED-A", "IMPORTED-B"]);
    let mut primary_document = PdfDocument::from_bytes(&primary).unwrap();
    let imported_document = PdfDocument::from_bytes(&imported).unwrap();
    let output = primary_document
        .insert_page_from(&imported_document, 1, 1)
        .unwrap();
    assert_eq!(texts(&output), ["PRIMARY-A", "IMPORTED-B", "PRIMARY-B"]);
    let mut reopened = PdfDocument::from_bytes(&output).unwrap();
    let imported_page = reopened.page_dict(1).unwrap();
    assert!(imported_page.contains_key("Resources"));
    assert!(imported_page.contains_key("MediaBox"));
    assert_eq!(
        imported_page
            .get("Rotate")
            .and_then(|value| value.as_integer()),
        Some(90)
    );
}

#[test]
fn split_ranges_are_atomic_bounded_and_each_output_reopens() {
    let source = document(&["ONE", "TWO", "THREE", "FOUR", "FIVE"]);
    let mut source_document = PdfDocument::from_bytes(&source).unwrap();
    let outputs = source_document
        .split_document(&[
            PageRange::new(0, 2),
            PageRange::new(2, 4),
            PageRange::new(4, 5),
        ])
        .unwrap();
    assert_eq!(outputs.len(), 3);
    assert_eq!(texts(&outputs[0]), ["ONE", "TWO"]);
    assert_eq!(texts(&outputs[1]), ["THREE", "FOUR"]);
    assert_eq!(texts(&outputs[2]), ["FIVE"]);
    assert!(source_document
        .split_document(&[PageRange::new(0, 3), PageRange::new(2, 4)])
        .is_err());
    assert!(source_document
        .split_document(&[PageRange::new(1, 1)])
        .is_err());
    assert!(source_document
        .split_document(&[PageRange::new(4, 6)])
        .is_err());
}

#[test]
fn form_graphs_remain_distinct_and_duplicate_names_are_deterministically_renamed() {
    let form = read_fixture("v0_10_compat", "pdflib-complete-form.pdf");
    let mut source = PdfDocument::from_bytes(&form).unwrap();
    let source_fields = source.form_fields().unwrap();
    let merged = PdfDocument::merge_documents(&[&form, &form]).unwrap();
    let mut reopened = PdfDocument::from_bytes(&merged).unwrap();
    let fields = reopened.form_fields().unwrap();
    assert!(!fields.is_empty());
    let names = fields
        .iter()
        .map(|field| field.fully_qualified_name.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(names.len(), fields.len());
    assert!(fields
        .iter()
        .any(|field| field.fully_qualified_name.contains("__starpdf_d2_")));
    assert_eq!(
        fields.iter().filter(|field| field.is_radio()).count(),
        source_fields
            .iter()
            .filter(|field| field.is_radio())
            .count()
            * 2
    );
    assert_eq!(
        fields.iter().filter(|field| field.is_checkbox()).count(),
        source_fields
            .iter()
            .filter(|field| field.is_checkbox())
            .count()
            * 2
    );
    assert_eq!(
        fields.iter().filter(|field| field.is_choice()).count(),
        source_fields
            .iter()
            .filter(|field| field.is_choice())
            .count()
            * 2
    );
    assert_eq!(
        fields
            .iter()
            .filter(|field| matches!(field.field_type, FieldType::Text { .. }))
            .count(),
        source_fields
            .iter()
            .filter(|field| matches!(field.field_type, FieldType::Text { .. }))
            .count()
            * 2
    );
    let widget_refs = fields
        .iter()
        .flat_map(|field| field.widgets.iter().map(|widget| widget.object_ref))
        .collect::<BTreeSet<_>>();
    assert_eq!(
        widget_refs.len(),
        fields
            .iter()
            .map(|field| field.widgets.len())
            .sum::<usize>()
    );

    let first_text = fields
        .iter()
        .find(|field| field.is_text() && !field.fully_qualified_name.contains("__starpdf_d2_"))
        .unwrap();
    let second_text = fields
        .iter()
        .find(|field| field.is_text() && field.fully_qualified_name.contains("__starpdf_d2_"))
        .unwrap();
    let output = reopened
        .mutate_and_export(&[
            PdfChange::SetTextField {
                field_ref: first_text.object_ref,
                value: "FIRST-SOURCE-EDIT".into(),
            },
            PdfChange::SetTextField {
                field_ref: second_text.object_ref,
                value: "SECOND-SOURCE-EDIT".into(),
            },
        ])
        .unwrap();
    let mut mutated = PdfDocument::from_bytes(&output).unwrap();
    let values = mutated
        .form_fields()
        .unwrap()
        .into_iter()
        .filter_map(|field| match field.value {
            FieldValue::Text(value) => Some(value),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    assert!(values.contains("FIRST-SOURCE-EDIT"));
    assert!(values.contains("SECOND-SOURCE-EDIT"));
}

#[test]
fn annotations_images_fonts_metadata_and_mixed_xref_sources_survive_merge() {
    let annotations = read_fixture("v0_10_compat", "pdfkit-shapes-ink-link.pdf");
    let font = read_fixture("v0_9_compat", "chrome-landscape.pdf");
    let metadata = read_fixture("v0_11_complex", "synthetic-metadata-rich.pdf");
    let merged = PdfDocument::merge_documents(&[&metadata, &annotations, &font]).unwrap();
    let mut reopened = PdfDocument::from_bytes(&merged).unwrap();
    assert_eq!(reopened.page_count().unwrap(), 3);
    assert!(reopened.trailer().contains_key("Info"));
    let catalog_ref = reopened.catalog_ref();
    assert!(reopened
        .store_mut()
        .resolve(catalog_ref)
        .unwrap()
        .as_dict()
        .unwrap()
        .contains_key("Metadata"));
    let imported_annotations = reopened.page_annotations(1).unwrap();
    assert!(imported_annotations.len() >= 5);
    assert!(imported_annotations
        .iter()
        .filter(|annotation| annotation.subtype.as_name() != "Link")
        .all(|annotation| annotation.has_normal_appearance));
    assert!(!texts(&merged)[2].is_empty());
}

#[test]
fn merged_output_supports_follow_up_v0_12a_operations() {
    let first = document(&["A", "B"]);
    let second = document(&["C", "D"]);
    let merged = PdfDocument::merge_documents(&[&first, &second]).unwrap();
    let mut document = PdfDocument::from_bytes(&merged).unwrap();
    let reordered = document.move_page(3, 0).unwrap();
    let mut reordered_document = PdfDocument::from_bytes(&reordered).unwrap();
    let duplicated = reordered_document.duplicate_page(1, 2).unwrap();
    let mut duplicated_document = PdfDocument::from_bytes(&duplicated).unwrap();
    let deleted = duplicated_document.delete_page(3).unwrap();
    let mut deleted_document = PdfDocument::from_bytes(&deleted).unwrap();
    let extracted = deleted_document.extract_pages(&[0, 2]).unwrap();
    assert_eq!(texts(&extracted), ["D", "A"]);
}

#[test]
fn cross_document_limits_refuse_before_returning_partial_output() {
    let source = document(&["A"]);
    assert!(PdfDocument::merge_documents(&[&source]).is_err());
    let limits = starpdf::PageOperationLimits {
        max_input_documents: 1,
        ..starpdf::PageOperationLimits::default()
    };
    assert!(starpdf::DocumentBuilder::merge_documents(&[&source, &source], &limits).is_err());
    let limits = starpdf::PageOperationLimits {
        max_input_documents: 2,
        max_selected_pages: 1,
        ..starpdf::PageOperationLimits::default()
    };
    assert!(starpdf::DocumentBuilder::merge_documents(&[&source, &source], &limits).is_err());
}
