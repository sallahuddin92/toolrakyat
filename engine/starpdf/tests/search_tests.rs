use starpdf::search::{DocumentSearchIndex, SearchOptions};
use starpdf::text::{PageText, TextSpan};

#[test]
fn test_search_exact_and_case_insensitive() {
    let mut page = PageText::new(0);
    page.spans.push(TextSpan::new(
        0,
        "ToolRakyat PDF Search Engine".into(),
        50.0,
        700.0,
        200.0,
        14.0,
        0.0,
        "/F1".into(),
        14.0,
        1.0,
    ));

    // Case insensitive
    let res_ci = page.search(
        "pdf search",
        &SearchOptions {
            case_sensitive: false,
        },
    );
    assert_eq!(res_ci.len(), 1);
    assert_eq!(res_ci[0].matched_text, "PDF Search");
    assert_eq!(res_ci[0].boxes.len(), 1);

    // Case sensitive positive
    let res_cs = page.search(
        "ToolRakyat",
        &SearchOptions {
            case_sensitive: true,
        },
    );
    assert_eq!(res_cs.len(), 1);

    // Case sensitive negative
    let res_cs_neg = page.search(
        "toolrakyat",
        &SearchOptions {
            case_sensitive: true,
        },
    );
    assert!(res_cs_neg.is_empty());
}

#[test]
fn test_search_multi_span_phrase() {
    let mut page = PageText::new(0);
    // Span 1: "Hello " at x=50, y=700
    page.spans.push(TextSpan::new(
        0,
        "Hello ".into(),
        50.0,
        700.0,
        50.0,
        12.0,
        0.0,
        "/F1".into(),
        12.0,
        1.0,
    ));
    // Span 2: "World" at x=100, y=700
    page.spans.push(TextSpan::new(
        0,
        "World".into(),
        100.0,
        700.0,
        50.0,
        12.0,
        0.0,
        "/F1".into(),
        12.0,
        1.0,
    ));

    let results = page.search("Hello World", &SearchOptions::default());
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].start_span_index, 0);
    assert_eq!(results[0].end_span_index, 1);
    // Should produce 2 bounding boxes, one for each span!
    assert_eq!(results[0].boxes.len(), 2);
}

#[test]
fn test_search_joins_overlapping_producer_fragments_without_inventing_space() {
    let mut page = PageText::new(0);
    page.spans.push(TextSpan::new(
        0,
        "CHROME-BET".into(),
        50.0,
        700.0,
        100.0,
        12.0,
        0.0,
        "/F1".into(),
        12.0,
        1.0,
    ));
    page.spans.push(TextSpan::new(
        0,
        "A".into(),
        145.0,
        700.0,
        10.0,
        12.0,
        0.0,
        "/F1".into(),
        12.0,
        1.0,
    ));
    let results = page.search("CHROME-BETA", &SearchOptions::default());
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].boxes.len(), 2);
}

#[test]
fn test_search_multi_line_phrase_preserves_separate_boxes() {
    let mut page = PageText::new(0);
    // Line 1: "First Line Ending" at y=700
    page.spans.push(TextSpan::new(
        0,
        "First Line Ending".into(),
        50.0,
        700.0,
        150.0,
        12.0,
        0.0,
        "/F1".into(),
        12.0,
        1.0,
    ));
    // Line 2: "Second Line Starting" at y=680
    page.spans.push(TextSpan::new(
        0,
        "Second Line Starting".into(),
        50.0,
        680.0,
        160.0,
        12.0,
        0.0,
        "/F1".into(),
        12.0,
        1.0,
    ));

    let results = page.search("Ending Second", &SearchOptions::default());
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].boxes.len(), 2);
    // Line 1 box must be at y=700
    assert_eq!(results[0].boxes[0].y, 700.0);
    // Line 2 box must be at y=680
    assert_eq!(results[0].boxes[1].y, 680.0);
}

#[test]
fn test_search_rotated_text() {
    let mut page = PageText::new(0);
    page.spans.push(TextSpan::new(
        0,
        "Rotated Search Target".into(),
        100.0,
        200.0,
        150.0,
        12.0,
        90.0,
        "/F1".into(),
        12.0,
        1.0,
    ));

    let results = page.search("Search", &SearchOptions::default());
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].boxes[0].rotation, 90.0);
}

#[test]
fn test_document_search_index_multi_page() {
    let mut page0 = PageText::new(0);
    page0.spans.push(TextSpan::new(
        0,
        "Page 1 Content Target".into(),
        50.0,
        700.0,
        100.0,
        12.0,
        0.0,
        "/F1".into(),
        12.0,
        1.0,
    ));

    let mut page1 = PageText::new(1);
    page1.spans.push(TextSpan::new(
        1,
        "Page 2 Another Target Here".into(),
        50.0,
        700.0,
        120.0,
        12.0,
        0.0,
        "/F1".into(),
        12.0,
        1.0,
    ));

    let doc_index = DocumentSearchIndex::new(vec![page0, page1]);
    let results = doc_index.search("Target", &SearchOptions::default());
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].page_index, 0);
    assert_eq!(results[1].page_index, 1);
}
