use std::fs;
use std::path::Path;

use starpdf::document::PdfDocument;
use starpdf::validate::StructuralValidator;

#[test]
fn test_real_world_multi_page_pdf() {
    let path = Path::new("../../test-assets/multi-page.test.pdf");
    if let Ok(bytes) = fs::read(path) {
        let mut doc = PdfDocument::from_bytes(&bytes).unwrap();
        StructuralValidator::validate(&mut doc).unwrap();
        assert_eq!(doc.page_count().unwrap(), 2);
    }
}

#[test]
fn test_real_world_smartpdf_form() {
    let path = Path::new("../../test-assets/smartpdf-form.pdf");
    if let Ok(bytes) = fs::read(path) {
        let mut doc = PdfDocument::from_bytes(&bytes).unwrap();
        StructuralValidator::validate(&mut doc).unwrap();
        assert_eq!(doc.page_count().unwrap(), 1);
        let page = doc.page_dict(0).unwrap();
        assert_eq!(page.get("Type").unwrap().as_name(), Some("Page"));
    }
}

#[test]
fn test_real_world_adobe_like_form() {
    let path = Path::new("../../test-assets/smartpdf-adobe-like-form.pdf");
    if let Ok(bytes) = fs::read(path) {
        let mut doc = PdfDocument::from_bytes(&bytes).unwrap();
        StructuralValidator::validate(&mut doc).unwrap();
        assert_eq!(doc.page_count().unwrap(), 1);
    }
}

#[test]
fn test_real_world_scanned_test_pdf() {
    let path = Path::new("../../test-assets/scanned-test.pdf");
    if let Ok(bytes) = fs::read(path) {
        let mut doc = PdfDocument::from_bytes(&bytes).unwrap();
        StructuralValidator::validate(&mut doc).unwrap();
        assert_eq!(doc.page_count().unwrap(), 1);
    }
}

#[test]
fn test_real_world_invalid_pdf_fails_safely() {
    let path = Path::new("../../test-assets/invalid.pdf");
    if let Ok(bytes) = fs::read(path) {
        let result = PdfDocument::from_bytes(&bytes);
        assert!(result.is_err());
    }
}
