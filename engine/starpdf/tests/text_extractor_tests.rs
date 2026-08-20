use std::collections::BTreeMap;

use starpdf::font::{Font, PageResources, SimpleEncoding, UnicodeCMap};
use starpdf::text::TextExtractor;

#[test]
fn test_extractor_simple_text_run() {
    let content = b"BT /F1 14 Tf 50 700 Td (Hello World) Tj ET";
    let mut resources = PageResources {
        fonts: BTreeMap::new(),
    };
    resources
        .fonts
        .insert("/F1".into(), Font::standard_fallback("/F1"));

    let page_text = TextExtractor::extract_from_content(0, content, &resources).unwrap();
    assert_eq!(page_text.spans.len(), 1);

    let span = &page_text.spans[0];
    assert_eq!(span.text, "Hello World");
    assert_eq!(span.x, 50.0);
    assert_eq!(span.y, 700.0);
    assert_eq!(span.font_size, 14.0);
    assert_eq!(span.rotation, 0.0);
    assert!(span.width > 0.0);
    assert!(span.height > 0.0);
}

#[test]
fn test_extractor_multiple_lines_and_tj_kerning() {
    let content = b"
BT
/F1 12 Tf
100 600 Td
[(First) 120 (Line)] TJ
T*
(Second Line) Tj
ET
";
    let mut resources = PageResources {
        fonts: BTreeMap::new(),
    };
    resources
        .fonts
        .insert("/F1".into(), Font::standard_fallback("/F1"));

    let page_text = TextExtractor::extract_from_content(0, content, &resources).unwrap();
    assert_eq!(page_text.spans.len(), 3);
    assert_eq!(page_text.spans[0].text, "First");
    assert_eq!(page_text.spans[1].text, "Line");
    assert_eq!(page_text.spans[2].text, "Second Line");

    let full_text = page_text.plain_text();
    assert!(full_text.contains("First Line"));
    assert!(full_text.contains("Second Line"));
}

#[test]
fn test_extractor_transformed_ctm_and_rotation() {
    // Current transformation matrix rotates 90 degrees: [0 1 -1 0 100 200] cm
    let content = b"
q
0 1 -1 0 100 200 cm
BT
/F1 10 Tf
10 0 Td
(Rotated Text) Tj
ET
Q
";
    let mut resources = PageResources {
        fonts: BTreeMap::new(),
    };
    resources
        .fonts
        .insert("/F1".into(), Font::standard_fallback("/F1"));

    let page_text = TextExtractor::extract_from_content(0, content, &resources).unwrap();
    assert_eq!(page_text.spans.len(), 1);
    let span = &page_text.spans[0];
    assert_eq!(span.text, "Rotated Text");
    assert!((span.rotation - 90.0).abs() < 1e-4);
}

#[test]
fn test_extractor_with_tounicode_cmap() {
    let mut font = Font::standard_fallback("/F2");
    let mut cmap = UnicodeCMap::new();
    cmap.mappings.insert(1, "A".into());
    cmap.mappings.insert(2, "B".into());
    cmap.mappings.insert(3, "C".into());
    font.to_unicode = Some(cmap);

    let mut resources = PageResources {
        fonts: BTreeMap::new(),
    };
    resources.fonts.insert("/F2".into(), font);

    // Byte string with char codes \x01\x02\x03
    let content = b"BT /F2 12 Tf 50 500 Td (\x01\x02\x03) Tj ET";
    let page_text = TextExtractor::extract_from_content(0, content, &resources).unwrap();
    assert_eq!(page_text.spans.len(), 1);
    assert_eq!(page_text.spans[0].text, "ABC");
    assert_eq!(page_text.spans[0].confidence, 1.0);
}

#[test]
fn test_extractor_with_differences_encoding() {
    let mut font = Font::standard_fallback("/F3");
    let mut enc = SimpleEncoding::standard_win_ansi();
    enc.map[65] = Some('X'); // Override 'A' (65) to 'X'
    font.encoding = enc;

    let mut resources = PageResources {
        fonts: BTreeMap::new(),
    };
    resources.fonts.insert("/F3".into(), font);

    let content = b"BT /F3 12 Tf 50 500 Td (A) Tj ET";
    let page_text = TextExtractor::extract_from_content(0, content, &resources).unwrap();
    assert_eq!(page_text.spans.len(), 1);
    assert_eq!(page_text.spans[0].text, "X");
}
