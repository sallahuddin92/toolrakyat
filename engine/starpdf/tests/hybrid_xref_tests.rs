use starpdf::document::PdfDocument;

#[test]
fn test_hybrid_reference_pdf_resolution() {
    // Generate a hybrid PDF:
    // Classic xref table at EOF contains /XRefStm pointing to an XRef stream.
    // Classic table defines object 1 (Catalog) and object 2 (Pages).
    // XRef stream defines object 3 (Page).
    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.7\n");

    let mut offsets = Vec::new();
    offsets.push(0u64);

    // 1 0 obj: Catalog
    offsets.push(pdf.len() as u64);
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    // 2 0 obj: Pages
    offsets.push(pdf.len() as u64);
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    // 3 0 obj: Page
    let obj3_offset = pdf.len() as u64;
    pdf.extend_from_slice(b"3 0 obj\n<< /Type /Page /Parent 2 0 R >>\nendobj\n");

    // 4 0 obj: XRef Stream defining object 3
    let xref_stm_offset = pdf.len() as u64;
    // W=[1, 2, 1], Index=[3, 1], Type 1, offset = obj3_offset, gen = 0
    let o3_hi = ((obj3_offset >> 8) & 0xFF) as u8;
    let o3_lo = (obj3_offset & 0xFF) as u8;
    let xref_stm_data = vec![1, o3_hi, o3_lo, 0];
    let xref_stm_str = format!(
        "4 0 obj\n<< /Type /XRef /Size 5 /W [1 2 1] /Index [3 1] /Length {} >>\nstream\n",
        xref_stm_data.len()
    );
    pdf.extend_from_slice(xref_stm_str.as_bytes());
    pdf.extend_from_slice(&xref_stm_data);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    // Classic XRef table defining objects 0, 1, 2
    let classic_xref_offset = pdf.len() as u64;
    let classic_xref_body = format!(
        "xref\n0 3\n0000000000 65535 f \n{:010} 00000 n \n{:010} 00000 n \ntrailer\n<< /Size 5 /Root 1 0 R /XRefStm {} >>\nstartxref\n{}\n%%EOF",
        offsets[1], offsets[2], xref_stm_offset, classic_xref_offset
    );
    pdf.extend_from_slice(classic_xref_body.as_bytes());

    let mut doc = PdfDocument::from_bytes(&pdf).unwrap();
    assert_eq!(doc.page_count().unwrap(), 1);
    let page_dict = doc.page_dict(0).unwrap();
    assert_eq!(page_dict.get("Type").unwrap().as_name(), Some("Page"));
}

#[test]
fn test_cyclic_prev_detection_fails_safely() {
    // Construct a malicious PDF where /Prev points back to the same xref offset
    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.7\n1 0 obj\n<<>>\nendobj\n");
    let xref_offset = pdf.len() as u64;
    let xref_body = format!(
        "xref\n0 1\n0000000000 65535 f \ntrailer\n<< /Size 2 /Root 1 0 R /Prev {} >>\nstartxref\n{}\n%%EOF",
        xref_offset, xref_offset
    );
    pdf.extend_from_slice(xref_body.as_bytes());

    let result = PdfDocument::from_bytes(&pdf);
    assert!(result.is_err());
}
