use starpdf::document::PdfDocument;
use starpdf::error::PdfError;
use starpdf::syntax::{Lexer, Parser};

#[test]
fn test_malformed_empty_bytes() {
    assert!(PdfDocument::from_bytes(b"").is_err());
}

#[test]
fn test_malformed_truncated_header() {
    assert!(PdfDocument::from_bytes(b"%PDF").is_err());
    assert!(PdfDocument::from_bytes(b"NOT A PDF AT ALL").is_err());
}

#[test]
fn test_malformed_unclosed_literal_string() {
    let mut lexer = Lexer::from_bytes(b"(Unclosed string without closing paren");
    assert_eq!(lexer.next_token(), Err(PdfError::UnexpectedEof));
}

#[test]
fn test_malformed_unclosed_hex_string() {
    let mut lexer = Lexer::from_bytes(b"<48656c6c6f");
    assert_eq!(lexer.next_token(), Err(PdfError::UnexpectedEof));
}

#[test]
fn test_malformed_invalid_hex_char() {
    let mut lexer = Lexer::from_bytes(b"<48656Z>");
    assert!(matches!(lexer.next_token(), Err(PdfError::InvalidToken(_))));
}

#[test]
fn test_malformed_unclosed_array() {
    let mut parser = Parser::from_bytes(b"[ 1 2 3 /A");
    assert!(matches!(
        parser.parse_object(),
        Err(PdfError::InvalidSyntax(_))
    ));
}

#[test]
fn test_malformed_unclosed_dict() {
    let mut parser = Parser::from_bytes(b"<< /Key 123");
    assert!(matches!(
        parser.parse_object(),
        Err(PdfError::InvalidSyntax(_))
    ));
}

#[test]
fn test_malformed_dict_missing_value() {
    let mut parser = Parser::from_bytes(b"<< /Key >>");
    assert!(matches!(
        parser.parse_object(),
        Err(PdfError::InvalidSyntax(_))
    ));
}

#[test]
fn test_malformed_recursion_limit() {
    // Generate deeply nested arrays [[[[...]]]]
    let mut nested = Vec::new();
    for _ in 0..100 {
        nested.extend_from_slice(b"[");
    }
    for _ in 0..100 {
        nested.extend_from_slice(b"]");
    }
    let mut parser = Parser::from_bytes(&nested);
    assert_eq!(parser.parse_object(), Err(PdfError::RecursionLimitExceeded));
}

#[test]
fn test_malformed_random_fuzz_bytes_no_panic() {
    // Random hostile byte sequences
    let junk: &[&[u8]] = &[
        b"\x00\xFF\xFE\xFD",
        b"%PDF-1.7\x00\x00\x00xref\nstartxref",
        b"12 0 obj << /Length -999 >> stream\n",
        b"trailer << /Root 999 0 R >> startxref\n9999999\n%%EOF",
    ];

    for data in junk {
        let _ = PdfDocument::from_bytes(data);
    }
}
