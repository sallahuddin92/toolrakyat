use std::fs;
use std::path::Path;

use starpdf::document::PdfDocument;
use starpdf::search::SearchOptions;
use starpdf::writer::MinimalWriter;

#[test]
fn test_real_world_search_minimal_writer() {
    let pdf_bytes = MinimalWriter::create_minimal_pdf("StarPDF Searchable Document 2026").unwrap();
    let mut doc = PdfDocument::from_bytes(&pdf_bytes).unwrap();

    let results = doc.search("Searchable", &SearchOptions::default()).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].matched_text, "Searchable");
    assert_eq!(results[0].boxes[0].y, 700.0);
}

#[test]
fn test_real_world_search_form_fixture() {
    let path = Path::new("../../test-assets/smartpdf-form.pdf");
    if let Ok(bytes) = fs::read(path) {
        let mut doc = PdfDocument::from_bytes(&bytes).unwrap();
        let all_text = doc.extract_all_text().unwrap();
        if !all_text.is_empty() && !all_text[0].spans.is_empty() {
            let first_word = all_text[0].spans[0]
                .text
                .split_whitespace()
                .next()
                .unwrap_or("");
            if !first_word.is_empty() {
                let results = doc.search(first_word, &SearchOptions::default()).unwrap();
                assert!(!results.is_empty());
            }
        }
    }
}
