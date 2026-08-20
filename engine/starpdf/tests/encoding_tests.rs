use std::collections::BTreeMap;

use starpdf::font::encoding::{glyph_name_to_unicode, SimpleEncoding};
use starpdf::syntax::object::PdfObject;

#[test]
fn test_adobe_glyph_list_lookups() {
    assert_eq!(glyph_name_to_unicode("space"), Some(' '));
    assert_eq!(glyph_name_to_unicode("A"), Some('A'));
    assert_eq!(glyph_name_to_unicode("euro"), Some('€'));
    assert_eq!(glyph_name_to_unicode("bullet"), Some('•'));
    assert_eq!(glyph_name_to_unicode("uni0041"), Some('A'));
    assert_eq!(glyph_name_to_unicode("nonexistentglyphname123"), None);
}

#[test]
fn test_standard_win_ansi_encoding() {
    let enc = SimpleEncoding::standard_win_ansi();
    assert_eq!(enc.decode_byte(b'A'), 'A');
    assert_eq!(enc.decode_byte(b'1'), '1');
    assert_eq!(enc.decode_byte(0x80), '€'); // Euro in Windows-1252
    assert_eq!(enc.decode_byte(0x93), '“'); // Left double quote
}

#[test]
fn test_encoding_with_differences_array() {
    let mut dict = BTreeMap::new();
    dict.insert(
        "BaseEncoding".into(),
        PdfObject::Name("WinAnsiEncoding".into()),
    );
    // Differences: [ 65, /B, /C ] -> char code 65 ('A') is now 'B', 66 ('B') is now 'C'
    dict.insert(
        "Differences".into(),
        PdfObject::Array(vec![
            PdfObject::Integer(65),
            PdfObject::Name("B".into()),
            PdfObject::Name("C".into()),
        ]),
    );

    let enc = SimpleEncoding::from_pdf_object(&PdfObject::Dictionary(dict));
    assert_eq!(enc.decode_byte(65), 'B');
    assert_eq!(enc.decode_byte(66), 'C');
    assert_eq!(enc.decode_byte(67), 'C'); // unchanged
}
