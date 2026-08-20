#![allow(clippy::approx_constant)]
use starpdf::syntax::{Lexer, Token};

#[test]
fn test_lexer_empty() {
    let mut lexer = Lexer::from_bytes(b"");
    assert_eq!(lexer.next_token().unwrap(), None);
}

#[test]
fn test_lexer_whitespace_only() {
    let mut lexer = Lexer::from_bytes(b" \t\r\n\x00\x0C  \n");
    assert_eq!(lexer.next_token().unwrap(), None);
}

#[test]
fn test_lexer_comments() {
    let mut lexer = Lexer::from_bytes(b"% This is a comment\n123%another\r\n456");
    assert_eq!(lexer.next_token().unwrap(), Some(Token::Integer(123)));
    assert_eq!(lexer.next_token().unwrap(), Some(Token::Integer(456)));
    assert_eq!(lexer.next_token().unwrap(), None);
}

#[test]
fn test_lexer_integers_positive() {
    let mut lexer = Lexer::from_bytes(b"0 1 42 +100 999999");
    assert_eq!(lexer.next_token().unwrap(), Some(Token::Integer(0)));
    assert_eq!(lexer.next_token().unwrap(), Some(Token::Integer(1)));
    assert_eq!(lexer.next_token().unwrap(), Some(Token::Integer(42)));
    assert_eq!(lexer.next_token().unwrap(), Some(Token::Integer(100)));
    assert_eq!(lexer.next_token().unwrap(), Some(Token::Integer(999999)));
}

#[test]
fn test_lexer_integers_negative() {
    let mut lexer = Lexer::from_bytes(b"-1 -42 -9999");
    assert_eq!(lexer.next_token().unwrap(), Some(Token::Integer(-1)));
    assert_eq!(lexer.next_token().unwrap(), Some(Token::Integer(-42)));
    assert_eq!(lexer.next_token().unwrap(), Some(Token::Integer(-9999)));
}

#[test]
fn test_lexer_reals() {
    let mut lexer = Lexer::from_bytes(b"3.14 0.0 -0.5 +4.2 123.456");
    assert_eq!(lexer.next_token().unwrap(), Some(Token::Real(3.14)));
    assert_eq!(lexer.next_token().unwrap(), Some(Token::Real(0.0)));
    assert_eq!(lexer.next_token().unwrap(), Some(Token::Real(-0.5)));
    assert_eq!(lexer.next_token().unwrap(), Some(Token::Real(4.2)));
    assert_eq!(lexer.next_token().unwrap(), Some(Token::Real(123.456)));
}

#[test]
fn test_lexer_names_simple() {
    let mut lexer = Lexer::from_bytes(b"/Name1 /Type /Pages /Length");
    assert_eq!(
        lexer.next_token().unwrap(),
        Some(Token::Name("Name1".into()))
    );
    assert_eq!(
        lexer.next_token().unwrap(),
        Some(Token::Name("Type".into()))
    );
    assert_eq!(
        lexer.next_token().unwrap(),
        Some(Token::Name("Pages".into()))
    );
    assert_eq!(
        lexer.next_token().unwrap(),
        Some(Token::Name("Length".into()))
    );
}

#[test]
fn test_lexer_names_with_hex_escapes() {
    let mut lexer = Lexer::from_bytes(b"/PANTONE#20123 /A#23B /#46#6F#6F");
    assert_eq!(
        lexer.next_token().unwrap(),
        Some(Token::Name("PANTONE 123".into()))
    );
    assert_eq!(lexer.next_token().unwrap(), Some(Token::Name("A#B".into())));
    assert_eq!(lexer.next_token().unwrap(), Some(Token::Name("Foo".into())));
}

#[test]
fn test_lexer_names_with_special_chars() {
    let mut lexer = Lexer::from_bytes(b"/Lime#20Green /With-Hyphen /With.Dot /With_Underscore");
    assert_eq!(
        lexer.next_token().unwrap(),
        Some(Token::Name("Lime Green".into()))
    );
    assert_eq!(
        lexer.next_token().unwrap(),
        Some(Token::Name("With-Hyphen".into()))
    );
    assert_eq!(
        lexer.next_token().unwrap(),
        Some(Token::Name("With.Dot".into()))
    );
    assert_eq!(
        lexer.next_token().unwrap(),
        Some(Token::Name("With_Underscore".into()))
    );
}

#[test]
fn test_lexer_literal_string_simple() {
    let mut lexer = Lexer::from_bytes(b"(Hello World)");
    assert_eq!(
        lexer.next_token().unwrap(),
        Some(Token::LiteralString(b"Hello World".to_vec()))
    );
}

#[test]
fn test_lexer_literal_string_nested_parens() {
    let mut lexer = Lexer::from_bytes(b"(This (is (nested)) text)");
    assert_eq!(
        lexer.next_token().unwrap(),
        Some(Token::LiteralString(b"This (is (nested)) text".to_vec()))
    );
}

#[test]
fn test_lexer_literal_string_escapes() {
    let mut lexer = Lexer::from_bytes(b"(Line1\\nLine2\\tTab\\\\Escaped\\(Paren\\))");
    assert_eq!(
        lexer.next_token().unwrap(),
        Some(Token::LiteralString(
            b"Line1\nLine2\tTab\\Escaped(Paren)".to_vec()
        ))
    );
}

#[test]
fn test_lexer_literal_string_carriage_return() {
    let mut lexer = Lexer::from_bytes(b"(Line1\\rLine2)");
    assert_eq!(
        lexer.next_token().unwrap(),
        Some(Token::LiteralString(b"Line1\rLine2".to_vec()))
    );
}

#[test]
fn test_lexer_literal_string_form_feed_backspace() {
    let mut lexer = Lexer::from_bytes(b"(\\b\\f)");
    assert_eq!(
        lexer.next_token().unwrap(),
        Some(Token::LiteralString(vec![0x08, 0x0C]))
    );
}

#[test]
fn test_lexer_literal_string_octal_escapes() {
    let mut lexer = Lexer::from_bytes(b"(\\101\\102\\103)");
    assert_eq!(
        lexer.next_token().unwrap(),
        Some(Token::LiteralString(b"ABC".to_vec()))
    );
}

#[test]
fn test_lexer_literal_string_short_octal() {
    let mut lexer = Lexer::from_bytes(b"(\\0\\53\\123)");
    assert_eq!(
        lexer.next_token().unwrap(),
        Some(Token::LiteralString(vec![0, 0o53, 0o123]))
    );
}

#[test]
fn test_lexer_literal_string_line_continuation() {
    let mut lexer = Lexer::from_bytes(b"(Split\\\nLine\\\r\nCont)");
    assert_eq!(
        lexer.next_token().unwrap(),
        Some(Token::LiteralString(b"SplitLineCont".to_vec()))
    );
}

#[test]
fn test_lexer_hex_string_simple() {
    let mut lexer = Lexer::from_bytes(b"<48656c6c6f>");
    assert_eq!(
        lexer.next_token().unwrap(),
        Some(Token::HexString(b"Hello".to_vec()))
    );
}

#[test]
fn test_lexer_hex_string_with_whitespace() {
    let mut lexer = Lexer::from_bytes(b"< 48 65 6c 6c 6f >");
    assert_eq!(
        lexer.next_token().unwrap(),
        Some(Token::HexString(b"Hello".to_vec()))
    );
}

#[test]
fn test_lexer_hex_string_odd_digits() {
    let mut lexer = Lexer::from_bytes(b"<A>");
    assert_eq!(
        lexer.next_token().unwrap(),
        Some(Token::HexString(vec![0xA0]))
    );
}

#[test]
fn test_lexer_hex_string_empty() {
    let mut lexer = Lexer::from_bytes(b"<>");
    assert_eq!(lexer.next_token().unwrap(), Some(Token::HexString(vec![])));
}

#[test]
fn test_lexer_booleans_and_null() {
    let mut lexer = Lexer::from_bytes(b"true false null");
    assert_eq!(lexer.next_token().unwrap(), Some(Token::Boolean(true)));
    assert_eq!(lexer.next_token().unwrap(), Some(Token::Boolean(false)));
    assert_eq!(lexer.next_token().unwrap(), Some(Token::Null));
}

#[test]
fn test_lexer_delimiters() {
    let mut lexer = Lexer::from_bytes(b"[ ] << >>");
    assert_eq!(lexer.next_token().unwrap(), Some(Token::ArrayOpen));
    assert_eq!(lexer.next_token().unwrap(), Some(Token::ArrayClose));
    assert_eq!(lexer.next_token().unwrap(), Some(Token::DictOpen));
    assert_eq!(lexer.next_token().unwrap(), Some(Token::DictClose));
}

#[test]
fn test_lexer_keywords() {
    let mut lexer = Lexer::from_bytes(b"obj endobj stream endstream xref trailer startxref R");
    assert_eq!(lexer.next_token().unwrap(), Some(Token::KeywordObj));
    assert_eq!(lexer.next_token().unwrap(), Some(Token::KeywordEndObj));
    assert_eq!(lexer.next_token().unwrap(), Some(Token::KeywordStream));
    assert_eq!(lexer.next_token().unwrap(), Some(Token::KeywordEndStream));
    assert_eq!(lexer.next_token().unwrap(), Some(Token::KeywordXref));
    assert_eq!(lexer.next_token().unwrap(), Some(Token::KeywordTrailer));
    assert_eq!(lexer.next_token().unwrap(), Some(Token::KeywordStartXref));
    assert_eq!(lexer.next_token().unwrap(), Some(Token::KeywordR));
}

#[test]
fn test_lexer_operators() {
    let mut lexer = Lexer::from_bytes(b"BT ET Tf Tj TJ cm q Q re m l c h S s f F f* B B* b b* Do");
    let mut ops = Vec::new();
    while let Some(tok) = lexer.next_token().unwrap() {
        if let Token::Keyword(kw) = tok {
            ops.push(kw);
        }
    }
    assert_eq!(ops.len(), 23);
    assert_eq!(ops[0], "BT");
    assert_eq!(ops[1], "ET");
    assert_eq!(ops[2], "Tf");
    assert_eq!(ops[3], "Tj");
    assert_eq!(ops[4], "TJ");
    assert_eq!(ops[5], "cm");
    assert_eq!(ops[6], "q");
    assert_eq!(ops[7], "Q");
}

#[test]
fn test_lexer_adjacent_tokens() {
    let mut lexer = Lexer::from_bytes(b"/Name[12(str)<41>]");
    assert_eq!(
        lexer.next_token().unwrap(),
        Some(Token::Name("Name".into()))
    );
    assert_eq!(lexer.next_token().unwrap(), Some(Token::ArrayOpen));
    assert_eq!(lexer.next_token().unwrap(), Some(Token::Integer(12)));
    assert_eq!(
        lexer.next_token().unwrap(),
        Some(Token::LiteralString(b"str".to_vec()))
    );
    assert_eq!(
        lexer.next_token().unwrap(),
        Some(Token::HexString(b"A".to_vec()))
    );
    assert_eq!(lexer.next_token().unwrap(), Some(Token::ArrayClose));
}

#[test]
fn test_lexer_peek_token_preserves_position() {
    let mut lexer = Lexer::from_bytes(b"/First /Second");
    assert_eq!(
        lexer.peek_token().unwrap(),
        Some(Token::Name("First".into()))
    );
    assert_eq!(
        lexer.peek_token().unwrap(),
        Some(Token::Name("First".into()))
    );
    assert_eq!(
        lexer.next_token().unwrap(),
        Some(Token::Name("First".into()))
    );
    assert_eq!(
        lexer.next_token().unwrap(),
        Some(Token::Name("Second".into()))
    );
    assert_eq!(lexer.next_token().unwrap(), None);
}
