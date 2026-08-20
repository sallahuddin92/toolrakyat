use std::fs;
use std::path::Path;

use starpdf::document::PdfDocument;
use starpdf::writer::MinimalWriter;

#[test]
fn test_text_extraction_minimal_writer_roundtrip() {
    let original_text = "StarPDF Text Extraction Test 12345";
    let pdf_bytes = MinimalWriter::create_minimal_pdf(original_text).unwrap();

    let mut doc = PdfDocument::from_bytes(&pdf_bytes).unwrap();
    let page_text = doc.extract_page_text(0).unwrap();

    assert_eq!(page_text.spans.len(), 1);
    assert_eq!(page_text.spans[0].text, original_text);
    assert_eq!(page_text.spans[0].x, 100.0);
    assert_eq!(page_text.spans[0].y, 700.0);
    assert_eq!(page_text.spans[0].font_size, 24.0);
}

#[test]
fn test_text_extraction_multi_page_fixture() {
    let path = Path::new("../../test-assets/multi-page.test.pdf");
    if let Ok(bytes) = fs::read(path) {
        let mut doc = PdfDocument::from_bytes(&bytes).unwrap();
        let all_text = doc.extract_all_text().unwrap();
        assert_eq!(all_text.len(), 2);
    }
}

#[test]
fn test_text_extraction_smartpdf_form_fixture() {
    let path = Path::new("../../test-assets/smartpdf-form.pdf");
    if let Ok(bytes) = fs::read(path) {
        let mut doc = PdfDocument::from_bytes(&bytes).unwrap();
        let page_text = doc.extract_page_text(0).unwrap();
        assert!(!page_text.spans.is_empty());
    }
}
