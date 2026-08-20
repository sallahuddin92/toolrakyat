use starpdf::document::PdfDocument;
use starpdf::syntax::ObjectRef;
use starpdf::writer::MinimalWriter;

#[test]
fn test_page_tree_minimal_pdf() {
    let pdf_bytes = MinimalWriter::create_minimal_pdf("Test Content").unwrap();
    let mut doc = PdfDocument::from_bytes(&pdf_bytes).unwrap();

    assert_eq!(doc.page_count().unwrap(), 1);
    assert_eq!(doc.page_ref(0).unwrap(), ObjectRef::new(3, 0));

    let page_dict = doc.page_dict(0).unwrap();
    assert_eq!(page_dict.get("Type").unwrap().as_name(), Some("Page"));
    assert!(doc.page_dict(1).is_err());
}

#[test]
fn test_page_tree_out_of_bounds() {
    let pdf_bytes = MinimalWriter::create_minimal_pdf("Out of bounds test").unwrap();
    let mut doc = PdfDocument::from_bytes(&pdf_bytes).unwrap();

    assert!(doc.page_ref(1).is_err());
    assert!(doc.page_ref(999).is_err());
}

#[test]
fn test_page_tree_multi_level_resolution() {
    let mut buf = Vec::new();
    buf.extend_from_slice(b"%PDF-1.7\n");

    let mut offsets = Vec::new();
    offsets.push(0u64); // 0 entry free

    // 1 0 obj: Catalog
    offsets.push(buf.len() as u64);
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    // 2 0 obj: Root Pages -> Kids [5 0 R, 3 0 R], Count 3
    offsets.push(buf.len() as u64);
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [5 0 R 3 0 R] /Count 3 >>\nendobj\n");

    // 3 0 obj: Page (direct child of 2)
    offsets.push(buf.len() as u64);
    buf.extend_from_slice(b"3 0 obj\n<< /Type /Page /Parent 2 0 R >>\nendobj\n");

    // 4 0 obj: Dummy placeholder
    offsets.push(0u64);

    // 5 0 obj: Sub Pages -> Kids [6 0 R, 7 0 R], Count 2
    offsets.push(buf.len() as u64);
    buf.extend_from_slice(b"5 0 obj\n<< /Type /Pages /Kids [6 0 R 7 0 R] /Count 2 >>\nendobj\n");

    // 6 0 obj: Page (child of 5)
    offsets.push(buf.len() as u64);
    buf.extend_from_slice(b"6 0 obj\n<< /Type /Page /Parent 5 0 R >>\nendobj\n");

    // 7 0 obj: Page (child of 5)
    offsets.push(buf.len() as u64);
    buf.extend_from_slice(b"7 0 obj\n<< /Type /Page /Parent 5 0 R >>\nendobj\n");

    let xref_offset = buf.len() as u64;
    let mut xref_body = format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len());
    for &off in &offsets[1..] {
        if off == 0 {
            xref_body.push_str("0000000000 65535 f \n");
        } else {
            xref_body.push_str(&format!("{off:010} 00000 n \n"));
        }
    }
    xref_body.push_str(&format!(
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF",
        offsets.len()
    ));

    buf.extend_from_slice(xref_body.as_bytes());

    let mut doc = PdfDocument::from_bytes(&buf).unwrap();
    assert_eq!(doc.page_count().unwrap(), 3);
    assert_eq!(doc.page_ref(0).unwrap(), ObjectRef::new(6, 0));
    assert_eq!(doc.page_ref(1).unwrap(), ObjectRef::new(7, 0));
    assert_eq!(doc.page_ref(2).unwrap(), ObjectRef::new(3, 0));
}
