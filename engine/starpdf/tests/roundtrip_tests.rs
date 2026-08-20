use starpdf::document::PdfDocument;
use starpdf::validate::StructuralValidator;
use starpdf::writer::MinimalWriter;

#[test]
fn test_roundtrip_generate_and_reopen_pdf() {
    let original_text = "StarPDF Roundtrip Verification Test Document";
    let pdf_bytes = MinimalWriter::create_minimal_pdf(original_text).unwrap();

    // 1. Reopen with StarPDF
    let mut doc = PdfDocument::from_bytes(&pdf_bytes).unwrap();

    // 2. Validate structural soundness
    StructuralValidator::validate(&mut doc).unwrap();

    // 3. Verify metadata
    assert_eq!(doc.version(), "1.7");
    assert_eq!(doc.page_count().unwrap(), 1);

    // 4. Verify Page dictionary
    let page = doc.page_dict(0).unwrap();
    assert_eq!(page.get("Type").unwrap().as_name(), Some("Page"));
    assert!(page.contains_key("MediaBox"));
    assert!(page.contains_key("Contents"));
}
