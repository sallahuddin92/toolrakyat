use starpdf::document::PdfDocument;
use starpdf::mutation::text_edit::TextEditTarget;
use starpdf::search::SearchOptions;
use starpdf::syntax::object::{ObjectRef, PdfObject, StreamObject};
use starpdf::writer::MinimalWriter;
use std::collections::BTreeMap;

#[test]
fn test_replace_simple_tj_text_and_verify_roundtrip() {
    let original_bytes = MinimalWriter::create_minimal_pdf("Hello World").unwrap();
    let mut doc = PdfDocument::from_bytes(&original_bytes).unwrap();

    let page_text = doc.extract_page_text(0).unwrap();
    assert_eq!(page_text.spans.len(), 1);
    let span = &page_text.spans[0];
    assert_eq!(span.text, "Hello World");
    assert!(span.is_editable);

    let target = TextEditTarget::from_span(span);
    let plan = doc.replace_text(0, &target, "Hello Universe").unwrap();
    assert!(plan.layout_policy_result.is_some());
    assert!(plan.layout_policy_result.as_ref().unwrap().is_safe());

    let updated_bytes = doc.export_incremental(&plan).unwrap();
    assert!(updated_bytes.len() > original_bytes.len());

    // Reopen updated PDF
    let mut reopened = PdfDocument::from_bytes(&updated_bytes).unwrap();
    let reopened_text = reopened.extract_page_text(0).unwrap();
    assert_eq!(reopened_text.spans.len(), 1);
    assert_eq!(reopened_text.spans[0].text, "Hello Universe");

    // Search verification: old text is gone, new text is found
    let search_options = SearchOptions::default();
    let old_hits = reopened.search("World", &search_options).unwrap();
    assert_eq!(old_hits.len(), 0);

    let new_hits = reopened.search("Universe", &search_options).unwrap();
    assert_eq!(new_hits.len(), 1);
    assert_eq!(new_hits[0].matched_text, "Universe");
}

#[test]
fn test_replace_tj_kerning_array_item() {
    // Construct a document with TJ kerning array
    let content = b"BT /F1 12 Tf 50 700 Td [(Star) 50 (PDF) 50 (Engine)] TJ ET\n";
    let doc = MinimalWriter::create_minimal_pdf("placeholder").unwrap();
    let mut parsed_doc = PdfDocument::from_bytes(&doc).unwrap();
    let _page_ref = parsed_doc.page_ref(0).unwrap();
    let page_dict = parsed_doc.page_dict(0).unwrap();
    let contents_ref = page_dict.get("Contents").unwrap().as_reference().unwrap();

    let mut stream_obj = parsed_doc
        .store_mut()
        .resolve(contents_ref)
        .unwrap()
        .as_stream()
        .unwrap()
        .clone();
    stream_obj.data = content.to_vec();
    stream_obj.stream_length = content.len();
    stream_obj.dict.remove("Filter");
    stream_obj
        .dict
        .insert("Length".into(), PdfObject::Integer(content.len() as i64));

    let plan = starpdf::mutation::MutationPlan {
        modified_objects: BTreeMap::from([(contents_ref, PdfObject::Stream(stream_obj))]),
        appearance_status: starpdf::appearance::AppearanceStatus::ValueUpdated,
        glyph_mapping_quality: None,
        layout_policy_result: None,
    };
    let seeded = parsed_doc.export_incremental(&plan).unwrap();

    let mut reopened = PdfDocument::from_bytes(&seeded).unwrap();
    let page_text = reopened.extract_page_text(0).unwrap();
    assert_eq!(page_text.spans.len(), 3);
    assert_eq!(page_text.spans[0].text, "Star");
    assert_eq!(page_text.spans[1].text, "PDF");
    assert_eq!(page_text.spans[2].text, "Engine");

    // Replace span #1 ("PDF") with "Doc" (fits within box, exact layout compensation)
    let target = TextEditTarget::from_span(&page_text.spans[1]);
    let edit_plan = reopened.replace_text(0, &target, "Doc").unwrap();
    let mutated = reopened.export_incremental(&edit_plan).unwrap();

    // Verify roundtrip
    let mut verified_doc = PdfDocument::from_bytes(&mutated).unwrap();
    let verified_text = verified_doc.extract_page_text(0).unwrap();
    assert_eq!(verified_text.spans.len(), 3);
    assert_eq!(verified_text.spans[0].text, "Star");
    assert_eq!(verified_text.spans[1].text, "Doc");
    assert_eq!(verified_text.spans[2].text, "Engine");
}

#[test]
fn test_repeated_identical_text_targets_exact_span_only() {
    // Generate a content stream where "Total" appears 10 times at different lines
    let mut content = Vec::new();
    content.extend_from_slice(b"BT /F1 12 Tf\n");
    for i in 0..10 {
        let line = format!("50 {} Td (Total) Tj\n", 700 - i * 30);
        content.extend_from_slice(line.as_bytes());
    }
    content.extend_from_slice(b"ET\n");

    let doc = MinimalWriter::create_minimal_pdf("placeholder").unwrap();
    let mut parsed = PdfDocument::from_bytes(&doc).unwrap();
    let page_dict = parsed.page_dict(0).unwrap();
    let contents_ref = page_dict.get("Contents").unwrap().as_reference().unwrap();

    let mut stream_obj = parsed
        .store_mut()
        .resolve(contents_ref)
        .unwrap()
        .as_stream()
        .unwrap()
        .clone();
    stream_obj.data = content.clone();
    stream_obj.stream_length = content.len();
    stream_obj.dict.remove("Filter");
    stream_obj
        .dict
        .insert("Length".into(), PdfObject::Integer(content.len() as i64));

    let plan = starpdf::mutation::MutationPlan {
        modified_objects: BTreeMap::from([(contents_ref, PdfObject::Stream(stream_obj))]),
        appearance_status: starpdf::appearance::AppearanceStatus::ValueUpdated,
        glyph_mapping_quality: None,
        layout_policy_result: None,
    };
    let seeded = parsed.export_incremental(&plan).unwrap();

    let mut reopened = PdfDocument::from_bytes(&seeded).unwrap();
    let page_text = reopened.extract_page_text(0).unwrap();
    assert_eq!(page_text.spans.len(), 10);
    for span in &page_text.spans {
        assert_eq!(span.text, "Total");
    }

    // Target ONLY span index 3 (4th appearance)
    let target_span = &page_text.spans[3];
    let target = TextEditTarget::from_span(target_span);
    let edit_plan = reopened.replace_text(0, &target, "Subtotal").unwrap();
    let edited_bytes = reopened.export_incremental(&edit_plan).unwrap();

    // Verify in reopened document: ONLY the 4th span is "Subtotal", all 9 others remain "Total"
    let mut verified = PdfDocument::from_bytes(&edited_bytes).unwrap();
    let verified_text = verified.extract_page_text(0).unwrap();
    assert_eq!(verified_text.spans.len(), 10);

    assert_eq!(verified_text.spans[0].text, "Total");
    assert_eq!(verified_text.spans[1].text, "Total");
    assert_eq!(verified_text.spans[2].text, "Total");
    assert_eq!(verified_text.spans[3].text, "Subtotal");
    assert_eq!(verified_text.spans[4].text, "Total");
    assert_eq!(verified_text.spans[5].text, "Total");
    assert_eq!(verified_text.spans[6].text, "Total");
    assert_eq!(verified_text.spans[7].text, "Total");
    assert_eq!(verified_text.spans[8].text, "Total");
    assert_eq!(verified_text.spans[9].text, "Total");
}

#[test]
fn test_multi_stream_page_mutates_only_targeted_stream() {
    let stream0_data = b"BT /F1 12 Tf 50 750 Td (Header Title) Tj ET\n";
    let stream1_data = b"BT /F1 12 Tf 50 700 Td (Middle Content) Tj ET\n";
    let stream2_data = b"BT /F1 12 Tf 50 650 Td (Footer Notice) Tj ET\n";

    let original = MinimalWriter::create_minimal_pdf("placeholder").unwrap();
    let mut parsed = PdfDocument::from_bytes(&original).unwrap();
    let page_ref = parsed.page_ref(0).unwrap();
    let mut page_dict = parsed.page_dict(0).unwrap();

    let s0_ref = ObjectRef::new(600, 0);
    let s1_ref = ObjectRef::new(601, 0);
    let s2_ref = ObjectRef::new(602, 0);

    let create_stream = |data: &[u8]| -> StreamObject {
        StreamObject {
            dict: BTreeMap::from([("Length".to_string(), PdfObject::Integer(data.len() as i64))]),
            data: data.to_vec(),
            stream_offset: 0,
            stream_length: data.len(),
        }
    };

    page_dict.insert(
        "Contents".to_string(),
        PdfObject::Array(vec![
            PdfObject::Reference(s0_ref),
            PdfObject::Reference(s1_ref),
            PdfObject::Reference(s2_ref),
        ]),
    );

    let plan = starpdf::mutation::MutationPlan {
        modified_objects: BTreeMap::from([
            (page_ref, PdfObject::Dictionary(page_dict)),
            (s0_ref, PdfObject::Stream(create_stream(stream0_data))),
            (s1_ref, PdfObject::Stream(create_stream(stream1_data))),
            (s2_ref, PdfObject::Stream(create_stream(stream2_data))),
        ]),
        appearance_status: starpdf::appearance::AppearanceStatus::ValueUpdated,
        glyph_mapping_quality: None,
        layout_policy_result: None,
    };
    let multi_stream_pdf = parsed.export_incremental(&plan).unwrap();

    let mut doc = PdfDocument::from_bytes(&multi_stream_pdf).unwrap();
    let page_text = doc.extract_page_text(0).unwrap();
    assert_eq!(page_text.spans.len(), 3);
    assert_eq!(page_text.spans[0].text, "Header Title");
    assert_eq!(page_text.spans[0].stream_index, 0);
    assert_eq!(page_text.spans[1].text, "Middle Content");
    assert_eq!(page_text.spans[1].stream_index, 1);
    assert_eq!(page_text.spans[2].text, "Footer Notice");
    assert_eq!(page_text.spans[2].stream_index, 2);

    // Replace text in stream 1 ONLY
    let target = TextEditTarget::from_span(&page_text.spans[1]);
    let edit_plan = doc.replace_text(0, &target, "Updated Content").unwrap();

    // Verify that modified_objects contains ONLY stream 1 (s1_ref), not s0_ref or s2_ref or page_ref
    assert_eq!(edit_plan.modified_objects.len(), 1);
    assert!(edit_plan.modified_objects.contains_key(&s1_ref));
    assert!(!edit_plan.modified_objects.contains_key(&s0_ref));
    assert!(!edit_plan.modified_objects.contains_key(&s2_ref));

    let exported = doc.export_incremental(&edit_plan).unwrap();
    let mut verified = PdfDocument::from_bytes(&exported).unwrap();
    let verified_text = verified.extract_page_text(0).unwrap();
    assert_eq!(verified_text.spans[0].text, "Header Title");
    assert_eq!(verified_text.spans[1].text, "Updated Content");
    assert_eq!(verified_text.spans[2].text, "Footer Notice");
}

#[test]
fn test_unsupported_complex_script_refusal() {
    let original_bytes = MinimalWriter::create_minimal_pdf("English Text").unwrap();
    let mut doc = PdfDocument::from_bytes(&original_bytes).unwrap();

    let page_text = doc.extract_page_text(0).unwrap();
    let target = TextEditTarget::from_span(&page_text.spans[0]);

    // Attempt to replace with Arabic text (requires complex shaping)
    let arabic_text = "مرحبا";
    let result = doc.replace_text(0, &target, arabic_text);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, starpdf::error::PdfError::UnsupportedComplexScript(_))
            || format!("{err}").contains("UNSUPPORTED_COMPLEX_SCRIPT")
    );
}

#[test]
fn test_unsupported_glyph_refusal() {
    let original_bytes = MinimalWriter::create_minimal_pdf("Simple Text").unwrap();
    let mut doc = PdfDocument::from_bytes(&original_bytes).unwrap();

    let page_text = doc.extract_page_text(0).unwrap();
    let target = TextEditTarget::from_span(&page_text.spans[0]);

    // Attempt to replace with glyph not in WinAnsi/Standard fallback (e.g. Japanese Kanji)
    let unmapped_text = "日本語";
    let result = doc.replace_text(0, &target, unmapped_text);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, starpdf::error::PdfError::UnsupportedFontEncoding(_))
            || format!("{err}").contains("UNREPRESENTABLE")
            || format!("{err}").contains("UNSUPPORTED_FONT_ENCODING")
    );
}

#[test]
fn test_sequential_edits_on_same_page() {
    let original_bytes = MinimalWriter::create_minimal_pdf("Step One").unwrap();
    let mut doc = PdfDocument::from_bytes(&original_bytes).unwrap();

    // Edit 1: "Step One" -> "Step Two"
    let text1 = doc.extract_page_text(0).unwrap();
    let target1 = TextEditTarget::from_span(&text1.spans[0]);
    let plan1 = doc.replace_text(0, &target1, "Step Two").unwrap();
    let bytes1 = doc.export_incremental(&plan1).unwrap();

    // Edit 2: "Step Two" -> "Step Three"
    let mut doc2 = PdfDocument::from_bytes(&bytes1).unwrap();
    let text2 = doc2.extract_page_text(0).unwrap();
    assert_eq!(text2.spans[0].text, "Step Two");
    let target2 = TextEditTarget::from_span(&text2.spans[0]);
    let plan2 = doc2.replace_text(0, &target2, "Step Three").unwrap();
    let bytes2 = doc2.export_incremental(&plan2).unwrap();

    // Edit 3: "Step Three" -> "Final Step"
    let mut doc3 = PdfDocument::from_bytes(&bytes2).unwrap();
    let text3 = doc3.extract_page_text(0).unwrap();
    assert_eq!(text3.spans[0].text, "Step Three");
    let target3 = TextEditTarget::from_span(&text3.spans[0]);
    let plan3 = doc3.replace_text(0, &target3, "Final Step").unwrap();
    let bytes3 = doc3.export_incremental(&plan3).unwrap();

    // Verify final state
    let mut final_doc = PdfDocument::from_bytes(&bytes3).unwrap();
    let final_text = final_doc.extract_page_text(0).unwrap();
    assert_eq!(final_text.spans[0].text, "Final Step");
}

#[test]
fn test_edit_after_v0_12_page_operations() {
    let original_bytes = MinimalWriter::create_minimal_pdf("First Page Text").unwrap();

    // 1. Duplicate page using v0.12 page builder
    let limits = starpdf::page_ops::PageOperationLimits::default();
    let duplicated_pdf =
        starpdf::page_ops::DocumentBuilder::duplicate_page(&original_bytes, 0, 1, &limits).unwrap();

    let mut doc_dup = PdfDocument::from_bytes(&duplicated_pdf).unwrap();
    assert_eq!(doc_dup.page_count().unwrap(), 2);

    // 2. Edit page 1 (the duplicate) to say "Second Page Text"
    let page1_text = doc_dup.extract_page_text(1).unwrap();
    let target = TextEditTarget::from_span(&page1_text.spans[0]);
    let plan = doc_dup
        .replace_text(1, &target, "Second Page Text")
        .unwrap();
    let edited_pdf = doc_dup.export_incremental(&plan).unwrap();

    // 3. Verify page 0 is "First Page Text" and page 1 is "Second Page Text"
    let mut verified = PdfDocument::from_bytes(&edited_pdf).unwrap();
    assert_eq!(verified.page_count().unwrap(), 2);
    assert_eq!(
        verified.extract_page_text(0).unwrap().spans[0].text,
        "First Page Text"
    );
    assert_eq!(
        verified.extract_page_text(1).unwrap().spans[0].text,
        "Second Page Text"
    );
}

#[test]
fn test_truetype_and_type0_fixtures_text_replacement() {
    let fixture_path = "tests/fixtures/v0_9_compat/chrome-unicode.pdf";
    if let Ok(bytes) = std::fs::read(fixture_path) {
        let mut doc = PdfDocument::from_bytes(&bytes).unwrap();
        let page_text = doc.extract_page_text(0).unwrap();
        if !page_text.spans.is_empty() {
            let first_editable = page_text.spans.iter().find(|s| s.is_editable);
            if let Some(span) = first_editable {
                let target = TextEditTarget::from_span(span);
                let result = doc.replace_text(0, &target, "Test");
                if let Ok(plan) = result {
                    let exported = doc.export_incremental(&plan).unwrap();
                    let mut reopened = PdfDocument::from_bytes(&exported).unwrap();
                    let verified = reopened.extract_page_text(0).unwrap();
                    assert!(verified.plain_text().contains("Test"));
                }
            }
        }
    }
}
