use starpdf::document::PdfDocument;
use starpdf::mutation::text_edit::TextEditTarget;

#[test]
fn test_tj_followed_by_dependent_tj_exact_advance_preserved() {
    // Stream with two Tj operations on the same line without Tm between them:
    // First Tj: "Hello"
    // Second Tj: "World"
    let pdf_bytes =
        create_test_pdf_with_stream(b"BT\n/F1 12 Tf\n100 700 Td\n(Hello) Tj\n(World) Tj\nET\n");

    let mut doc = PdfDocument::from_bytes(&pdf_bytes).expect("Failed to open test document");
    let page_text = doc
        .extract_page_text(0)
        .expect("Failed to extract page text");
    assert_eq!(page_text.spans.len(), 2);
    assert_eq!(page_text.spans[0].text, "Hello");
    assert_eq!(page_text.spans[1].text, "World");
    let orig_world_x = page_text.spans[1].x;

    // Mutate "Hello" -> "Hi" (shorter text)
    let target = TextEditTarget::from_span(&page_text.spans[0]);
    let plan = doc
        .replace_text(0, &target, "Hi")
        .expect("Replace text failed");
    assert!(plan.layout_policy_result.is_some());

    let updated_bytes = doc.export_incremental(&plan).expect("Failed to export");
    let mut updated_doc = PdfDocument::from_bytes(&updated_bytes).expect("Failed to reopen");

    // Re-extract page text to verify downstream "World" did NOT move
    let updated_page_text = updated_doc
        .extract_page_text(0)
        .expect("Failed to re-extract page text");
    let updated_world_span = updated_page_text
        .spans
        .iter()
        .find(|s| s.text == "World")
        .expect("World span must still be present");

    assert!(
        (updated_world_span.x - orig_world_x).abs() < 0.01,
        "Downstream 'World' X position must not move! (orig: {}, updated: {})",
        orig_world_x,
        updated_world_span.x
    );
}

#[test]
fn test_tj_followed_by_tm_reset_allows_natural_width_change() {
    // Stream where second text operation resets Tm:
    let pdf_bytes = create_test_pdf_with_stream(
        b"BT\n/F1 12 Tf\n1 0 0 1 100 700 Tm\n(First) Tj\n1 0 0 1 250 700 Tm\n(Second) Tj\nET\n",
    );

    let mut doc = PdfDocument::from_bytes(&pdf_bytes).expect("Failed to open test document");
    let page_text = doc.extract_page_text(0).expect("Failed to extract text");
    let first_span = &page_text.spans[0];

    // Mutate "First" -> "A Longer Replacement Text"
    let target = TextEditTarget::from_span(first_span);
    let plan = doc
        .replace_text(0, &target, "A Longer Replacement")
        .expect("Replace text with Tm reset must succeed");
    assert!(plan.layout_policy_result.is_some());
    assert!(plan.layout_policy_result.unwrap().is_safe());
}

#[test]
fn test_tj_array_middle_string_replacement_preserves_downstream_position() {
    // TJ array: [(Leading) 20 (Target) 30 (Trailing)] TJ
    let pdf_bytes = create_test_pdf_with_stream(
        b"BT\n/F1 12 Tf\n100 700 Td\n[(Leading) 20 (Target) 30 (Trailing)] TJ\nET\n",
    );

    let mut doc = PdfDocument::from_bytes(&pdf_bytes).expect("Failed to open test document");
    let page_text = doc.extract_page_text(0).expect("Failed to extract text");
    let target_span = page_text
        .spans
        .iter()
        .find(|s| s.text == "Target")
        .expect("Target span must exist");
    let trailing_span = page_text
        .spans
        .iter()
        .find(|s| s.text == "Trailing")
        .expect("Trailing span must exist");
    let orig_trailing_x = trailing_span.x;

    // Mutate "Target" -> "Tag" (shorter)
    let target = TextEditTarget::from_span(target_span);
    let plan = doc
        .replace_text(0, &target, "Tag")
        .expect("Replace inside TJ must succeed");
    assert!(plan.layout_policy_result.is_some());

    let updated_bytes = doc.export_incremental(&plan).expect("Export failed");
    let mut updated_doc = PdfDocument::from_bytes(&updated_bytes).expect("Reopen failed");

    let updated_page_text = updated_doc
        .extract_page_text(0)
        .expect("Failed to re-extract");
    let updated_trailing = updated_page_text
        .spans
        .iter()
        .find(|s| s.text == "Trailing")
        .expect("Trailing must exist");

    assert!(
        (updated_trailing.x - orig_trailing_x).abs() < 0.01,
        "Downstream 'Trailing' in TJ array must not move! (orig: {}, updated: {})",
        orig_trailing_x,
        updated_trailing.x
    );
}

#[test]
fn test_multi_span_intra_word_replacement_preserves_downstream_position() {
    // Multi-span word in TJ array: [(Arch) (itect) (ural) 40 (NextWord)] TJ
    let pdf_bytes = create_test_pdf_with_stream(
        b"BT\n/F1 12 Tf\n100 700 Td\n[(Arch) (itect) (ural) 40 (NextWord)] TJ\nET\n",
    );

    let mut doc = PdfDocument::from_bytes(&pdf_bytes).expect("Failed to open test document");
    let page_text = doc.extract_page_text(0).expect("Failed to extract text");
    let span_ids: Vec<String> = page_text
        .spans
        .iter()
        .filter(|s| s.text == "Arch" || s.text == "itect" || s.text == "ural")
        .map(|s| s.span_id.clone())
        .collect();
    assert_eq!(span_ids.len(), 3);

    let next_word_span = page_text
        .spans
        .iter()
        .find(|s| s.text == "NextWord")
        .expect("NextWord must exist");
    let orig_next_x = next_word_span.x;

    // Mutate multi-span word "Architectural" -> "Design" (shorter)
    let plan = doc
        .replace_text_group(0, &span_ids, "Design")
        .expect("Multi-span replacement must succeed");

    assert!(plan.layout_policy_result.is_some());

    let updated_bytes = doc.export_incremental(&plan).expect("Export failed");
    let mut updated_doc = PdfDocument::from_bytes(&updated_bytes).expect("Reopen failed");

    let updated_page_text = updated_doc
        .extract_page_text(0)
        .expect("Failed to re-extract");
    let updated_next = updated_page_text
        .spans
        .iter()
        .find(|s| s.text == "NextWord")
        .expect("NextWord must exist");

    assert!(
        (updated_next.x - orig_next_x).abs() < 0.01,
        "Downstream 'NextWord' must not move after multi-span replacement! (orig: {}, updated: {})",
        orig_next_x,
        updated_next.x
    );
}

#[test]
fn test_dependent_downstream_refuses_when_replacement_exceeds_available_space() {
    // Stream with tight dependent text: "Short" followed immediately by "Next"
    let pdf_bytes =
        create_test_pdf_with_stream(b"BT\n/F1 12 Tf\n100 700 Td\n(Short) Tj\n(Next) Tj\nET\n");

    let mut doc = PdfDocument::from_bytes(&pdf_bytes).expect("Failed to open test document");
    let page_text = doc.extract_page_text(0).expect("Failed to extract text");
    let short_span = &page_text.spans[0];

    // Try to mutate "Short" -> "A Much Longer Word That Exceeds Original Advance"
    let target = TextEditTarget::from_span(short_span);
    let res = doc.replace_text(0, &target, "A Much Longer Word That Exceeds");
    assert!(res.is_err());
    let err_msg = res.unwrap_err().to_string();
    assert!(
        err_msg.contains("Other text in this PDF depends on the spacing of this text run"),
        "Error message must clearly cite dependent spacing! (got: {err_msg})"
    );
}

fn create_test_pdf_with_stream(stream_content: &[u8]) -> Vec<u8> {
    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.7\n");

    let obj1_offset = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let obj2_offset = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let obj4_offset = pdf.len();
    let stream_len = stream_content.len();
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", stream_len).as_bytes());
    pdf.extend_from_slice(stream_content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let obj5_offset = pdf.len();
    pdf.extend_from_slice(
        b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
    );

    let obj3_offset = pdf.len();
    pdf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n",
    );

    let xref_offset = pdf.len();
    pdf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{:010} 00000 n \n", obj1_offset).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", obj2_offset).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", obj3_offset).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", obj4_offset).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", obj5_offset).as_bytes());

    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            xref_offset
        )
        .as_bytes(),
    );

    pdf
}
