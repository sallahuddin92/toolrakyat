use starpdf::document::recovery::RecoveryKind;
use starpdf::document::PdfDocument;
use starpdf::TextEditTarget;

/// Helper: generates a valid minimal PDF with customizable producer tag and header prefix
fn create_test_pdf_with_producer(producer: &str, header_prefix: &[u8]) -> Vec<u8> {
    let mut pdf = Vec::new();
    pdf.extend_from_slice(header_prefix);
    pdf.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");

    let mut offsets = vec![0usize];

    // 1 0 obj: Catalog
    offsets.push(pdf.len());
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    // 2 0 obj: Pages
    offsets.push(pdf.len());
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    // 3 0 obj: Page
    offsets.push(pdf.len());
    pdf.extend_from_slice(b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n");

    // 4 0 obj: Contents Stream
    let stream_content = format!("BT\n/F1 12 Tf\n50 700 Td (Produced by {producer}) Tj\nET\n");
    offsets.push(pdf.len());
    pdf.extend_from_slice(
        format!(
            "4 0 obj\n<< /Length {} >>\nstream\n{stream_content}endstream\nendobj\n",
            stream_content.len()
        )
        .as_bytes(),
    );

    // 5 0 obj: Font
    offsets.push(pdf.len());
    pdf.extend_from_slice(
        b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
    );

    // 6 0 obj: Info Dict
    offsets.push(pdf.len());
    pdf.extend_from_slice(
        format!(
            "6 0 obj\n<< /Producer ({producer}) /Title (StarPDF v0.18 Qualification) >>\nendobj\n"
        )
        .as_bytes(),
    );

    // XRef Table
    let xref_offset = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len()).as_bytes());
    for offset in offsets.iter().skip(1) {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }

    // Trailer
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R /Info 6 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n",
            offsets.len()
        )
        .as_bytes(),
    );

    pdf
}

#[test]
fn test_corpus_multi_producer_open_and_inspect() {
    let producers = [
        "Skia/PDF m120 (Google Chrome)",
        "Mozilla Firefox 125.0",
        "LibreOffice 7.6.4.1",
        "Microsoft: Print to PDF",
        "macOS Version 15.3 (Build 24D60) Quartz PDFContext",
        "PDFKit v0.15.0",
        "pdf-lib v1.17.1",
        "qpdf version 11.9.0",
        "GPL Ghostscript 10.03.0",
    ];

    for producer in producers {
        let pdf_bytes = create_test_pdf_with_producer(producer, b"");
        let mut doc =
            PdfDocument::from_bytes(&pdf_bytes).expect("Failed to open valid producer PDF");

        assert_eq!(doc.page_count().expect("page count"), 1);
        let text = doc.extract_page_text(0).expect("extract text");
        assert!(text.plain_text().contains(producer));

        // Reopen unchanged validation
        assert_eq!(doc.recovery_status(), RecoveryKind::None);
    }
}

#[test]
fn test_recovery_header_preceding_bom_or_comments() {
    let utf8_bom = b"\xEF\xBB\xBF";
    let pdf_bytes = create_test_pdf_with_producer("Preceding BOM Producer", utf8_bom);
    let mut doc = PdfDocument::from_bytes(&pdf_bytes).expect("Failed to open PDF with UTF-8 BOM");

    assert_eq!(doc.page_count().expect("page count"), 1);
    assert_eq!(
        doc.recovery_status(),
        RecoveryKind::ProducerCompatibilityPath
    );
    let events = doc.recovery_tracker().events.clone();
    assert!(events
        .iter()
        .any(|e| e.description.contains("BOM") || e.description.contains("leading bytes")));
}

#[test]
fn test_recovery_extended_startxref_search() {
    let mut pdf_bytes = create_test_pdf_with_producer("Extended StartXref Producer", b"");
    // Append 3000 bytes of trailing padding/comments after %%EOF (simulating appended metadata)
    let padding = vec![b'%'; 3000];
    pdf_bytes.extend_from_slice(&padding);
    pdf_bytes.extend_from_slice(b"\n%%EOF\n");

    // The standard 2048-byte search window will miss it, but extended search up to 64KB will recover it.
    let mut doc =
        PdfDocument::from_bytes(&pdf_bytes).expect("Failed to open PDF with distant startxref");
    assert_eq!(doc.page_count().expect("page count"), 1);
}

#[test]
fn test_recovery_xref_offset_drift_tolerance() {
    let pdf_bytes = create_test_pdf_with_producer("Offset Drift Producer", b"");

    // Locate "startxref\n" in binary bytes
    let startxref_pos = pdf_bytes
        .windows(10)
        .rposition(|w| w == b"startxref\n")
        .expect("startxref position");
    let offset_start = startxref_pos + 10;
    let newline_rel = pdf_bytes[offset_start..]
        .iter()
        .position(|&b| b == b'\n')
        .expect("newline after offset");
    let orig_offset_str = std::str::from_utf8(&pdf_bytes[offset_start..offset_start + newline_rel])
        .expect("ascii offset");
    let orig_offset: u64 = orig_offset_str.parse().expect("parsed offset");

    let drifted_offset = orig_offset + 5;
    let mut mutated_pdf = pdf_bytes[..offset_start].to_vec();
    mutated_pdf.extend_from_slice(format!("{drifted_offset}\n%%EOF\n").as_bytes());

    let mut doc =
        PdfDocument::from_bytes(&mutated_pdf).expect("Failed to open PDF with drifted xref offset");
    assert_eq!(doc.page_count().expect("page count"), 1);
}

#[test]
fn test_recovery_stream_length_reconciliation() {
    // Generate a PDF where /Length is declared as 500 but actual stream is 35 bytes
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = vec![0usize];

    // 1 0 obj: Catalog
    offsets.push(pdf.len());
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    // 2 0 obj: Pages
    offsets.push(pdf.len());
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    // 3 0 obj: Page
    offsets.push(pdf.len());
    pdf.extend_from_slice(b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n");

    // 4 0 obj: Stream with mismatched /Length 500
    let stream_content = "BT\n/F1 12 Tf\n50 700 Td (Reconciled Stream Length) Tj\nET\n";
    offsets.push(pdf.len());
    pdf.extend_from_slice(
        format!("4 0 obj\n<< /Length 500 >>\nstream\n{stream_content}endstream\nendobj\n")
            .as_bytes(),
    );

    // 5 0 obj: Font
    offsets.push(pdf.len());
    pdf.extend_from_slice(
        b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
    );

    // XRef
    let xref_offset = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len()).as_bytes());
    for offset in offsets.iter().skip(1) {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n",
            offsets.len()
        )
        .as_bytes(),
    );

    let mut doc =
        PdfDocument::from_bytes(&pdf).expect("Failed to open PDF with mismatched stream length");
    let text = doc.extract_page_text(0).expect("extract text");
    assert!(text.plain_text().contains("Reconciled Stream Length"));
}

#[test]
fn test_corpus_scanned_image_only_pdf() {
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = vec![0usize];

    offsets.push(pdf.len());
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    offsets.push(pdf.len());
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    // Page with image XObject /Im1 and no text stream
    offsets.push(pdf.len());
    pdf.extend_from_slice(b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /XObject << /Im1 5 0 R >> >> >>\nendobj\n");

    let stream_content = "q\n612 0 0 792 0 0 cm\n/Im1 Do\nQ\n";
    offsets.push(pdf.len());
    pdf.extend_from_slice(
        format!(
            "4 0 obj\n<< /Length {} >>\nstream\n{stream_content}endstream\nendobj\n",
            stream_content.len()
        )
        .as_bytes(),
    );

    // 5 0 obj: Dummy 1x1 Image XObject
    offsets.push(pdf.len());
    pdf.extend_from_slice(b"5 0 obj\n<< /Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Length 3 >>\nstream\n\xFF\x00\x00endstream\nendobj\n");

    let xref_offset = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len()).as_bytes());
    for offset in offsets.iter().skip(1) {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n",
            offsets.len()
        )
        .as_bytes(),
    );

    let mut doc = PdfDocument::from_bytes(&pdf).expect("Failed to open scanned image-only PDF");
    let text = doc.extract_page_text(0).expect("extract text");
    assert!(
        text.spans.is_empty(),
        "Scanned PDF should have 0 text spans"
    );

    let images = doc.enumerate_images(0).expect("enumerate images");
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].resource_name, "Im1");
    assert_eq!(images[0].width, 1);
    assert_eq!(images[0].height, 1);
}

#[test]
fn test_corpus_hierarchical_nested_pages_tree() {
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = vec![0usize];

    // 1 0 obj: Catalog
    offsets.push(pdf.len());
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    // 2 0 obj: Root Pages node (has 2 sub-pages nodes 3 0 R and 4 0 R)
    offsets.push(pdf.len());
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>\nendobj\n");

    // 3 0 obj: Sub-Pages node 1 (has page 5 0 R)
    offsets.push(pdf.len());
    pdf.extend_from_slice(b"3 0 obj\n<< /Type /Pages /Parent 2 0 R /Kids [5 0 R] /Count 1 /MediaBox [0 0 612 792] >>\nendobj\n");

    // 4 0 obj: Sub-Pages node 2 (has page 6 0 R)
    offsets.push(pdf.len());
    pdf.extend_from_slice(b"4 0 obj\n<< /Type /Pages /Parent 2 0 R /Kids [6 0 R] /Count 1 /MediaBox [0 0 612 792] >>\nendobj\n");

    // 5 0 obj: Page 1 (inherits MediaBox from parent 3 0 R)
    offsets.push(pdf.len());
    pdf.extend_from_slice(b"5 0 obj\n<< /Type /Page /Parent 3 0 R /Contents 7 0 R /Resources << /Font << /F1 9 0 R >> >> >>\nendobj\n");

    // 6 0 obj: Page 2 (inherits MediaBox from parent 4 0 R)
    offsets.push(pdf.len());
    pdf.extend_from_slice(b"6 0 obj\n<< /Type /Page /Parent 4 0 R /Contents 8 0 R /Resources << /Font << /F1 9 0 R >> >> >>\nendobj\n");

    // Content 1
    let c1 = "BT\n/F1 12 Tf\n50 700 Td (Page 1 in nested tree) Tj\nET\n";
    offsets.push(pdf.len());
    pdf.extend_from_slice(
        format!(
            "7 0 obj\n<< /Length {} >>\nstream\n{c1}endstream\nendobj\n",
            c1.len()
        )
        .as_bytes(),
    );

    // Content 2
    let c2 = "BT\n/F1 12 Tf\n50 700 Td (Page 2 in nested tree) Tj\nET\n";
    offsets.push(pdf.len());
    pdf.extend_from_slice(
        format!(
            "8 0 obj\n<< /Length {} >>\nstream\n{c2}endstream\nendobj\n",
            c2.len()
        )
        .as_bytes(),
    );

    // Font
    offsets.push(pdf.len());
    pdf.extend_from_slice(
        b"9 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
    );

    let xref_offset = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len()).as_bytes());
    for offset in offsets.iter().skip(1) {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n",
            offsets.len()
        )
        .as_bytes(),
    );

    let mut doc =
        PdfDocument::from_bytes(&pdf).expect("Failed to open hierarchical nested page tree");
    assert_eq!(doc.page_count().expect("page count"), 2);
    let p1_text = doc.extract_page_text(0).expect("extract p1");
    let p2_text = doc.extract_page_text(1).expect("extract p2");
    assert!(p1_text.plain_text().contains("Page 1 in nested tree"));
    assert!(p2_text.plain_text().contains("Page 2 in nested tree"));
}

#[test]
fn test_mutation_compatibility_across_producers() {
    let producers = [
        "Skia/PDF m120 (Google Chrome)",
        "LibreOffice 7.6.4.1",
        "Microsoft: Print to PDF",
        "macOS Version 15.3 (Build 24D60) Quartz PDFContext",
    ];

    for producer in producers {
        let pdf_bytes = create_test_pdf_with_producer(producer, b"");
        let mut doc = PdfDocument::from_bytes(&pdf_bytes).expect("Failed to open producer PDF");

        // 1. Text extraction & mutation
        let text = doc.extract_page_text(0).expect("extract text");
        assert!(!text.spans.is_empty());
        let span = &text.spans[0];
        let target = TextEditTarget::from_span(span);

        let plan = doc
            .replace_text(0, &target, "MUTATED_CONTENT")
            .expect("replace text");

        // 2. Export incremental & reopen
        let exported = doc.export_incremental(&plan).expect("export incremental");
        let mut reopened = PdfDocument::from_bytes(&exported).expect("reopen mutated PDF");
        assert_eq!(reopened.page_count().expect("page count"), 1);

        let search_hits = reopened
            .search(
                "MUTATED_CONTENT",
                &starpdf::search::SearchOptions::default(),
            )
            .expect("search mutated");
        assert_eq!(search_hits.len(), 1);
    }
}

#[test]
fn test_compatibility_scorecard_evaluation() {
    let mut full_pass_count = 0;
    let mut recovered_pass_count = 0;
    let mut typed_unsupported_count = 0;
    let mut malformed_refused_count = 0;

    // 1. Valid producer documents -> FULL_PASS
    let producers = [
        "Skia/PDF m120 (Google Chrome)",
        "Mozilla Firefox 125.0",
        "LibreOffice 7.6.4.1",
        "Microsoft: Print to PDF",
        "macOS Version 15.3 (Build 24D60) Quartz PDFContext",
        "PDFKit v0.15.0",
        "pdf-lib v1.17.1",
        "qpdf version 11.9.0",
        "GPL Ghostscript 10.03.0",
    ];
    for p in producers {
        let pdf = create_test_pdf_with_producer(p, b"");
        if let Ok(mut doc) = PdfDocument::from_bytes(&pdf) {
            if doc.recovery_status() == RecoveryKind::None && doc.page_count().is_ok() {
                full_pass_count += 1;
            }
        }
    }

    // 2. Recoverable documents -> RECOVERED_PASS
    let bom_pdf = create_test_pdf_with_producer("BOM", b"\xEF\xBB\xBF");
    if let Ok(mut doc) = PdfDocument::from_bytes(&bom_pdf) {
        if doc.recovery_status() != RecoveryKind::None && doc.page_count().is_ok() {
            recovered_pass_count += 1;
        }
    }

    // 3. Encrypted document fixture -> TYPED_UNSUPPORTED (refused for mutation)
    let encrypted_fixture =
        include_bytes!("fixtures/v0_11_complex/synthetic-encrypted-standard.pdf");
    if let Ok(mut doc) = PdfDocument::from_bytes(encrypted_fixture) {
        let sec = doc.security_info().expect("security info");
        if !sec.mutation_allowed {
            typed_unsupported_count += 1;
        }
    }

    // 4. Garbage non-PDF bytes -> MALFORMED_REFUSED
    let garbage = b"THIS IS NOT A VALID PDF DOCUMENT AT ALL";
    if PdfDocument::from_bytes(garbage).is_err() {
        malformed_refused_count += 1;
    }

    assert_eq!(full_pass_count, 9);
    assert_eq!(recovered_pass_count, 1);
    assert_eq!(typed_unsupported_count, 1);
    assert_eq!(malformed_refused_count, 1);
}
