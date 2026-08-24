use starpdf::document::PdfDocument;
use starpdf::font::encoding::SimpleEncoding;
use starpdf::font::font::{Font, FontFamily, FontStyle};
use starpdf::font::standard_metrics::StandardFontMetrics;
use starpdf::mutation::text_edit::{ContentStreamEditor, TextEditTarget};

#[test]
fn test_standard_14_metrics_and_widths() {
    let helvetica_width = StandardFontMetrics::get_char_width("Helvetica", 'A').unwrap();
    let helvetica_bold_width = StandardFontMetrics::get_char_width("Helvetica-Bold", 'A').unwrap();
    assert_eq!(helvetica_width, 667.0);
    assert_eq!(helvetica_bold_width, 722.0);

    let times_width = StandardFontMetrics::get_char_width("Times-Roman", 'A').unwrap();
    assert_eq!(times_width, 722.0);

    let courier_width = StandardFontMetrics::get_char_width("Courier", 'A').unwrap();
    assert_eq!(courier_width, 600.0);
}

#[test]
fn test_font_style_matching_and_standard_constructors() {
    let bold_italic_serif = FontStyle {
        family: FontFamily::Serif,
        is_bold: true,
        is_italic: true,
        is_monospace: false,
    };
    assert_eq!(
        bold_italic_serif.standard_base_font_name(),
        "Times-BoldItalic"
    );

    let mono_style = FontStyle {
        family: FontFamily::Monospace,
        is_bold: false,
        is_italic: false,
        is_monospace: true,
    };
    assert_eq!(mono_style.standard_base_font_name(), "Courier");

    let font = Font::standard_with_style("F_Std", &bold_italic_serif);
    assert_eq!(font.base_font, "Times-BoldItalic");
    assert!(font.can_encode_text("Café & Résumé 123"));
}

#[test]
fn test_simple_encoding_extended_latin_mappings() {
    let win_ansi = SimpleEncoding::standard_win_ansi();

    // Standard ASCII
    assert_eq!(win_ansi.encode_char('A'), Some(0x41));
    assert_eq!(win_ansi.decode_byte(0x41), 'A');

    // Extended Latin characters
    assert_eq!(win_ansi.encode_char('é'), Some(0xE9));
    assert_eq!(win_ansi.decode_byte(0xE9), 'é');

    assert_eq!(win_ansi.encode_char('ü'), Some(0xFC));
    assert_eq!(win_ansi.decode_byte(0xFC), 'ü');

    assert_eq!(win_ansi.encode_char('ç'), Some(0xE7));
    assert_eq!(win_ansi.decode_byte(0xE7), 'ç');

    assert_eq!(win_ansi.encode_char('€'), Some(0x80));
    assert_eq!(win_ansi.decode_byte(0x80), '€');

    assert_eq!(win_ansi.encode_char('—'), Some(0x97)); // em-dash
    assert_eq!(win_ansi.decode_byte(0x97), '—');
}

#[test]
fn test_complex_script_refusal_in_encoding() {
    let font = Font::standard_fallback("Helvetica");

    // Arabic / Jawi
    assert!(!font.can_encode_char('\u{0627}')); // Arabic Alef
    assert!(!font.can_encode_text("مرحبا"));

    // Devanagari
    assert!(!font.can_encode_char('\u{0915}')); // Devanagari Ka
    assert!(!font.can_encode_text("नमस्ते"));
}

#[test]
fn test_content_stream_font_switch_injection() {
    let original_stream =
        b"BT\n/F1 12.0 Tf\n100.0 700.0 Td\n(Original Text) Tj\n(Downstream) Tj\nET\n";
    let target = TextEditTarget::new(0, 0, 3, 0);
    let new_bytes = b"New Replacement Text";

    let modified = ContentStreamEditor::replace_multiple_in_stream_with_font_switch(
        original_stream,
        &[(&target, new_bytes)],
        None,
        Some(("F_StarPDF_HelveticaBold", 12.0, "F1")),
    )
    .unwrap();

    let text = String::from_utf8_lossy(&modified);
    assert!(text.contains("/F_StarPDF_HelveticaBold 12.0 Tf"));
    assert!(text.contains("(New Replacement Text) Tj"));
    assert!(text.contains("/F1 12.0 Tf"));
    assert!(text.contains("(Downstream) Tj"));
}

#[test]
fn test_end_to_end_native_text_replacement_with_accents_and_standard_fonts() {
    let minimal_pdf = b"%PDF-1.4\n\
1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n\
4 0 obj\n<< /Length 53 >>\nstream\nBT\n/F1 12 Tf\n100 700 Td\n(Hello World) Tj\nET\nendstream\nendobj\n\
5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n\
xref\n0 6\n0000000000 65535 f \n0000000009 00000 n \n0000000056 00000 n \n0000000111 00000 n \n0000000234 00000 n \n0000000337 00000 n \ntrailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n412\n%%EOF";

    let mut doc = PdfDocument::from_bytes(minimal_pdf).unwrap();
    let text = doc.extract_page_text(0).unwrap();
    assert_eq!(text.spans.len(), 1);
    assert_eq!(text.spans[0].text, "Hello World");
    assert!(text.spans[0].is_editable);

    // Mutate text with accented characters
    let target = TextEditTarget::new(0, 0, 3, 0);
    let plan = doc.replace_text(0, &target, "Café & Résumé").unwrap();
    let exported = doc.export_incremental(&plan).unwrap();

    // Reopen and verify
    let mut reopened = PdfDocument::from_bytes(&exported).unwrap();
    let reopened_text = reopened.extract_page_text(0).unwrap();
    assert_eq!(reopened_text.spans.len(), 1);
    assert_eq!(reopened_text.spans[0].text, "Café & Résumé");
}
