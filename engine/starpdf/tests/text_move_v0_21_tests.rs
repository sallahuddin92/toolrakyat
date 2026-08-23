use starpdf::document::PdfDocument;
use starpdf::syntax::object::PdfObject;
use starpdf::writer::MinimalWriter;
use std::collections::BTreeMap;

#[test]
fn test_move_text_span_and_verify_roundtrip() {
    let original_bytes = MinimalWriter::create_minimal_pdf("Hello World").unwrap();
    let mut doc = PdfDocument::from_bytes(&original_bytes).unwrap();

    let page_text = doc.extract_page_text(0).unwrap();
    assert_eq!(page_text.spans.len(), 1);
    let span = &page_text.spans[0];
    let orig_x = span.x;
    let orig_y = span.y;

    let plan = doc.move_text_span(0, &span.span_id, 25.0, 50.0).unwrap();
    let updated_bytes = doc.export_incremental(&plan).unwrap();

    // Reopen and verify new coordinates
    let mut reopened = PdfDocument::from_bytes(&updated_bytes).unwrap();
    let reopened_text = reopened.extract_page_text(0).unwrap();
    assert_eq!(reopened_text.spans.len(), 1);
    let new_span = &reopened_text.spans[0];
    assert_eq!(new_span.text, "Hello World");
    assert!((new_span.x - (orig_x + 25.0)).abs() < 0.1);
    assert!((new_span.y - (orig_y + 50.0)).abs() < 0.1);
}

#[test]
fn test_move_text_group_atomically() {
    // Construct a document with two independent text blocks (each with its own BT..ET and Tm)
    let content = b"BT /F1 12 Tf 12 0 0 12 50 700 Tm (Block A) Tj ET\nBT /F1 12 Tf 12 0 0 12 50 600 Tm (Block B) Tj ET\n";
    let doc = MinimalWriter::create_minimal_pdf("placeholder").unwrap();
    let mut parsed_doc = PdfDocument::from_bytes(&doc).unwrap();
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
    assert_eq!(page_text.spans.len(), 2);
    let span_a = &page_text.spans[0];
    let span_b = &page_text.spans[1];

    let orig_ax = span_a.x;
    let orig_ay = span_a.y;
    let orig_bx = span_b.x;
    let orig_by = span_b.y;

    // Move only Block A by (15, -30)
    let move_plan = reopened
        .move_text_span(0, &span_a.span_id, 15.0, -30.0)
        .unwrap();
    let mutated = reopened.export_incremental(&move_plan).unwrap();

    let mut verified = PdfDocument::from_bytes(&mutated).unwrap();
    let verified_text = verified.extract_page_text(0).unwrap();
    assert_eq!(verified_text.spans.len(), 2);

    // Block A moved
    assert!((verified_text.spans[0].x - (orig_ax + 15.0)).abs() < 0.1);
    assert!((verified_text.spans[0].y - (orig_ay - 30.0)).abs() < 0.1);

    // Unrelated Block B movement MUST BE EXACTLY 0
    assert!((verified_text.spans[1].x - orig_bx).abs() < 0.0001);
    assert!((verified_text.spans[1].y - orig_by).abs() < 0.0001);
}

#[test]
fn test_refuse_unsafe_downstream_dependent_text_move() {
    // Construct a document where two spans share text positioning in one text block
    let content = b"BT /F1 12 Tf 50 700 Td (First Line) Tj 0 -20 Td (Second Line) Tj ET\n";
    let doc = MinimalWriter::create_minimal_pdf("placeholder").unwrap();
    let mut parsed_doc = PdfDocument::from_bytes(&doc).unwrap();
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
    assert_eq!(page_text.spans.len(), 2);

    // First line has downstream dependent text -> moving it must be safely refused
    let res_first = reopened.move_text_span(0, &page_text.spans[0].span_id, 10.0, 10.0);
    assert!(res_first.is_err());

    // Second line depends on prior positioning -> moving it must be safely refused
    let res_second = reopened.move_text_span(0, &page_text.spans[1].span_id, 10.0, 10.0);
    assert!(res_second.is_err());

    // Both lines together form a complete bounded dependency closure -> succeeds rigidly
    let orig_y1 = page_text.spans[0].y;
    let orig_y2 = page_text.spans[1].y;
    let span_ids = vec![
        page_text.spans[0].span_id.clone(),
        page_text.spans[1].span_id.clone(),
    ];
    let group_plan = reopened.move_text_group(0, &span_ids, 20.0, -15.0).unwrap();
    let group_mutated = reopened.export_incremental(&group_plan).unwrap();

    let mut group_verified = PdfDocument::from_bytes(&group_mutated).unwrap();
    let group_text = group_verified.extract_page_text(0).unwrap();
    assert_eq!(group_text.spans.len(), 2);
    assert!((group_text.spans[0].x - (page_text.spans[0].x + 20.0)).abs() < 0.1);
    assert!((group_text.spans[0].y - (orig_y1 - 15.0)).abs() < 0.1);
    assert!((group_text.spans[1].x - (page_text.spans[1].x + 20.0)).abs() < 0.1);
    assert!((group_text.spans[1].y - (orig_y2 - 15.0)).abs() < 0.1);
    // Relative distance between spans is exactly preserved
    assert!(((group_text.spans[0].y - group_text.spans[1].y) - (orig_y1 - orig_y2)).abs() < 0.0001);
}
