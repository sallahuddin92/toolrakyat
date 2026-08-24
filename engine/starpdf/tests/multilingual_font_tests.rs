use starpdf::document::PdfDocument;
use starpdf::font::catalog::get_font_registry;
use starpdf::font::planner::{ReplacementStrategy, TextPlanner};
use starpdf::mutation::text_edit::TextEditTarget;
use starpdf::search::SearchOptions;
use std::fs;

fn get_minimal_test_pdf() -> Vec<u8> {
    b"%PDF-1.4\n\
1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n\
4 0 obj\n<< /Length 93 >>\nstream\nBT\n/F1 12 Tf\n100 700 Td\n(Original Text) Tj\nET\nBT\n/F1 10 Tf\n100 650 Td\n(Unrelated Text) Tj\nET\nendstream\nendobj\n\
5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n\
xref\n0 6\n0000000000 65535 f \n0000000009 00000 n \n0000000058 00000 n \n0000000115 00000 n \n0000000241 00000 n \n0000000383 00000 n \ntrailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n453\n%%EOF".to_vec()
}

fn ensure_test_fonts_registered() {
    let registry = get_font_registry();

    let font_mappings = [
        ("noto-sans-arabic", "NotoSansArabic-Regular.ttf"),
        ("noto-sans-hebrew", "NotoSansHebrew-Regular.ttf"),
        ("noto-sans-devanagari", "NotoSansDevanagari-Regular.ttf"),
        ("noto-sans-cjk-sc", "NotoSansSC-Regular.ttf"),
        ("noto-sans-cjk-tc", "NotoSansTC-Regular.ttf"),
        ("noto-sans-cjk-jp", "NotoSansJP-Regular.ttf"),
        ("noto-sans-cjk-kr", "NotoSansKR-Regular.ttf"),
    ];

    for (font_id, file) in font_mappings {
        let candidates = [
            format!("public/fonts/{}", file),
            format!("../../public/fonts/{}", file),
            format!("../public/fonts/{}", file),
        ];
        for path in candidates {
            if let Ok(bytes) = fs::read(&path) {
                registry.register_font(font_id, bytes);
                break;
            }
        }
    }
}

fn assert_multilingual_roundtrip(replacement: &str) -> starpdf::font::TextReplacementPlan {
    ensure_test_fonts_registered();

    let pdf_bytes = get_minimal_test_pdf();
    let mut doc = PdfDocument::from_bytes(&pdf_bytes).unwrap();
    let target = TextEditTarget::new(0, 0, 3, 0);
    let original_text = doc.extract_page_text(0).unwrap();
    let original_span = original_text.spans[0].clone();
    let unrelated_span = original_text.spans[1].clone();
    let plan = doc
        .plan_text_replacement(0, &target, replacement, None)
        .unwrap();
    assert!(plan.is_executable());
    assert_eq!(plan.strategy, ReplacementStrategy::ShapedFallback);
    assert!(!plan.fallback_runs.is_empty());
    assert!(plan
        .runs
        .iter()
        .flat_map(|run| &run.glyphs)
        .all(|glyph| glyph.glyph_id != 0));
    let largest_source_font = plan
        .fallback_runs
        .iter()
        .map(|run| run.font_bytes.len())
        .max()
        .unwrap();

    let mutation_plan = TextPlanner::apply(&mut doc, &plan).unwrap();
    let exported = doc.export_incremental(&mutation_plan).unwrap();

    assert!(exported.windows(b"/Type0".len()).any(|w| w == b"/Type0"));
    assert!(exported
        .windows(b"/CIDFontType2".len())
        .any(|w| w == b"/CIDFontType2"));
    assert!(exported
        .windows(b"/Identity-H".len())
        .any(|w| w == b"/Identity-H"));
    assert!(exported
        .windows(b"begincmap".len())
        .any(|w| w == b"begincmap"));
    assert!(
        exported.len() < largest_source_font,
        "embedded subset ({} bytes) must be smaller than source font ({} bytes)",
        exported.len(),
        largest_source_font
    );

    let mut reopened = PdfDocument::from_bytes(&exported).unwrap();
    let text = reopened.extract_page_text(0).unwrap();
    let combined: String = text
        .spans
        .iter()
        .filter(|span| span.text != unrelated_span.text)
        .map(|span| span.text.as_str())
        .collect();
    assert_eq!(combined, replacement);
    assert_eq!(text.spans[0].x, original_span.x);
    assert_eq!(text.spans[0].y, original_span.y);
    assert_eq!(text.spans[0].rotation, original_span.rotation);
    let reopened_unrelated = text
        .spans
        .iter()
        .find(|span| span.text == unrelated_span.text)
        .expect("unrelated text must remain present");
    assert_eq!(reopened_unrelated.x, unrelated_span.x);
    assert_eq!(reopened_unrelated.y, unrelated_span.y);
    assert_eq!(reopened_unrelated.rotation, unrelated_span.rotation);
    let hits = reopened
        .search(
            replacement,
            &SearchOptions {
                case_sensitive: true,
            },
        )
        .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].matched_text, replacement);

    plan
}

#[test]
fn test_arabic_shaping_and_type0_roundtrip() {
    let replacement = "سلام";
    let plan = assert_multilingual_roundtrip(replacement);
    assert!(!plan.runs.is_empty());
    assert!(plan.runs[0].is_rtl);
    assert_eq!(
        plan.runs[0]
            .glyphs
            .iter()
            .map(|glyph| glyph.glyph_id)
            .collect::<Vec<_>>(),
        vec![770, 705, 1083]
    );
    assert_eq!(
        plan.runs[0]
            .glyphs
            .iter()
            .map(|glyph| glyph.cluster)
            .collect::<Vec<_>>(),
        vec![6, 2, 0]
    );
}

#[test]
fn test_jawi_complex_chars_and_type0_roundtrip() {
    let replacement = "ساي جاوي ڤ ڠ ڽ چ";
    let plan = assert_multilingual_roundtrip(replacement);
    assert!(plan.runs[0].is_rtl);
    assert!(plan.runs[0].glyphs.iter().all(|glyph| glyph.glyph_id != 0));
    assert!(plan.runs[0]
        .glyphs
        .windows(2)
        .any(|pair| pair[0].cluster > pair[1].cluster));
}

#[test]
fn test_hebrew_shaping_and_type0_roundtrip() {
    let replacement = "שלום";
    let plan = assert_multilingual_roundtrip(replacement);
    assert!(plan.runs[0].is_rtl);
}

#[test]
fn test_devanagari_shaping_and_type0_roundtrip() {
    let replacement = "नमस्ते";
    let plan = assert_multilingual_roundtrip(replacement);
    assert!(plan.runs[0].glyphs.len() < replacement.chars().count());
    assert!(plan.runs[0]
        .glyphs
        .iter()
        .any(|glyph| glyph.glyph_id == 215));
}

#[test]
fn test_japanese_shaping_and_type0_roundtrip() {
    let replacement = "こんにちは 日本";
    let plan = assert_multilingual_roundtrip(replacement);
    assert_eq!(plan.fallback_runs[0].font_id, "noto-sans-cjk-jp");
}

#[test]
fn test_simplified_chinese_shaping_and_type0_roundtrip() {
    let replacement = "简体中文 测试";
    let plan = assert_multilingual_roundtrip(replacement);
    assert_eq!(plan.fallback_runs[0].font_id, "noto-sans-cjk-sc");
}

#[test]
fn test_traditional_chinese_shaping_and_type0_roundtrip() {
    let replacement = "繁體中文 測試";
    let plan = assert_multilingual_roundtrip(replacement);
    assert_eq!(plan.fallback_runs[0].font_id, "noto-sans-cjk-tc");
}

#[test]
fn test_korean_shaping_and_type0_roundtrip() {
    let replacement = "안녕하세요 한국어";
    let plan = assert_multilingual_roundtrip(replacement);
    assert_eq!(plan.fallback_runs[0].font_id, "noto-sans-cjk-kr");
}

#[test]
fn test_mixed_script_shaping_and_type0_roundtrip() {
    let replacement = "Report تقرير 日本 2026";
    let plan = assert_multilingual_roundtrip(replacement);
    assert!(plan.fallback_runs.len() >= 2);
}
