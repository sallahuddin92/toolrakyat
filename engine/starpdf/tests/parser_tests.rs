#![allow(clippy::approx_constant)]
use starpdf::syntax::{ObjectRef, Parser, PdfObject};

#[test]
fn test_parser_primitives_null_and_bool() {
    let mut p = Parser::from_bytes(b"null true false");
    assert_eq!(p.parse_object().unwrap(), PdfObject::Null);
    assert_eq!(p.parse_object().unwrap(), PdfObject::Bool(true));
    assert_eq!(p.parse_object().unwrap(), PdfObject::Bool(false));
}

#[test]
fn test_parser_primitives_integers() {
    let mut p = Parser::from_bytes(b"0 100 -50 2147483647 -2147483648");
    assert_eq!(p.parse_object().unwrap(), PdfObject::Integer(0));
    assert_eq!(p.parse_object().unwrap(), PdfObject::Integer(100));
    assert_eq!(p.parse_object().unwrap(), PdfObject::Integer(-50));
    assert_eq!(p.parse_object().unwrap(), PdfObject::Integer(2147483647));
    assert_eq!(p.parse_object().unwrap(), PdfObject::Integer(-2147483648));
}

#[test]
fn test_parser_primitives_reals() {
    let mut p = Parser::from_bytes(b"0.0 3.14159 -0.001 100.5");
    assert_eq!(p.parse_object().unwrap(), PdfObject::Real(0.0));
    assert_eq!(p.parse_object().unwrap(), PdfObject::Real(3.14159));
    assert_eq!(p.parse_object().unwrap(), PdfObject::Real(-0.001));
    assert_eq!(p.parse_object().unwrap(), PdfObject::Real(100.5));
}

#[test]
fn test_parser_primitives_names() {
    let mut p = Parser::from_bytes(b"/Root /Catalog /Type /Pages /Font /MediaBox");
    assert_eq!(p.parse_object().unwrap(), PdfObject::Name("Root".into()));
    assert_eq!(p.parse_object().unwrap(), PdfObject::Name("Catalog".into()));
    assert_eq!(p.parse_object().unwrap(), PdfObject::Name("Type".into()));
    assert_eq!(p.parse_object().unwrap(), PdfObject::Name("Pages".into()));
    assert_eq!(p.parse_object().unwrap(), PdfObject::Name("Font".into()));
    assert_eq!(
        p.parse_object().unwrap(),
        PdfObject::Name("MediaBox".into())
    );
}

#[test]
fn test_parser_primitives_strings() {
    let mut p = Parser::from_bytes(b"(Literal String) <48656c6c6f>");
    assert_eq!(
        p.parse_object().unwrap(),
        PdfObject::String(b"Literal String".to_vec())
    );
    assert_eq!(
        p.parse_object().unwrap(),
        PdfObject::String(b"Hello".to_vec())
    );
}

#[test]
fn test_parser_indirect_reference() {
    let mut p = Parser::from_bytes(b"12 0 R 45 2 R 999 65535 R");
    assert_eq!(
        p.parse_object().unwrap(),
        PdfObject::Reference(ObjectRef::new(12, 0))
    );
    assert_eq!(
        p.parse_object().unwrap(),
        PdfObject::Reference(ObjectRef::new(45, 2))
    );
    assert_eq!(
        p.parse_object().unwrap(),
        PdfObject::Reference(ObjectRef::new(999, 65535))
    );
}

#[test]
fn test_parser_array_empty_and_simple() {
    let mut p = Parser::from_bytes(b"[] [ 1 2 3 /A (test) ]");
    assert_eq!(p.parse_object().unwrap(), PdfObject::Array(vec![]));

    let obj = p.parse_object().unwrap();
    let arr = obj.as_array().unwrap();
    assert_eq!(arr.len(), 5);
    assert_eq!(arr[0], PdfObject::Integer(1));
    assert_eq!(arr[1], PdfObject::Integer(2));
    assert_eq!(arr[2], PdfObject::Integer(3));
    assert_eq!(arr[3], PdfObject::Name("A".into()));
    assert_eq!(arr[4], PdfObject::String(b"test".to_vec()));
}

#[test]
fn test_parser_array_nested() {
    let mut p = Parser::from_bytes(b"[ [ 1 2 ] [ 3 [ 4 ] ] ]");
    let obj = p.parse_object().unwrap();
    let arr = obj.as_array().unwrap();
    assert_eq!(arr.len(), 2);
}

#[test]
fn test_parser_dict_empty_and_simple() {
    let mut p = Parser::from_bytes(b"<<>> << /Type /Catalog /Pages 2 0 R /Version 1.7 >>");
    let empty_dict = p.parse_object().unwrap();
    assert_eq!(empty_dict.as_dict().unwrap().len(), 0);

    let obj = p.parse_object().unwrap();
    let dict = obj.as_dict().unwrap();
    assert_eq!(dict.get("Type"), Some(&PdfObject::Name("Catalog".into())));
    assert_eq!(
        dict.get("Pages"),
        Some(&PdfObject::Reference(ObjectRef::new(2, 0)))
    );
    assert_eq!(dict.get("Version"), Some(&PdfObject::Real(1.7)));
}

#[test]
fn test_parser_dict_nested() {
    let mut p = Parser::from_bytes(b"<< /Resources << /Font << /F1 5 0 R >> >> >>");
    let obj = p.parse_object().unwrap();
    let dict = obj.as_dict().unwrap();
    let res = dict.get("Resources").unwrap().as_dict().unwrap();
    let font = res.get("Font").unwrap().as_dict().unwrap();
    assert_eq!(
        font.get("F1"),
        Some(&PdfObject::Reference(ObjectRef::new(5, 0)))
    );
}

#[test]
fn test_parser_stream_object() {
    let stream_data = b"<< /Length 12 >>\nstream\nHello World!\nendstream";
    let mut p = Parser::from_bytes(stream_data);
    let obj = p.parse_object().unwrap();
    let stream = obj.as_stream().unwrap();
    assert_eq!(stream.data, b"Hello World!");
    assert_eq!(stream.stream_length, 12);
}

#[test]
fn test_parser_stream_object_crlf() {
    let stream_data = b"<< /Length 5 >>\r\nstream\r\n12345\r\nendstream";
    let mut p = Parser::from_bytes(stream_data);
    let obj = p.parse_object().unwrap();
    let stream = obj.as_stream().unwrap();
    assert_eq!(stream.data, b"12345");
}

#[test]
fn test_parser_indirect_object_definition() {
    let raw = b"10 0 obj\n<< /Type /Page /Parent 2 0 R >>\nendobj";
    let mut p = Parser::from_bytes(raw);
    let (obj_ref, obj) = p.parse_indirect_object().unwrap();
    assert_eq!(obj_ref, ObjectRef::new(10, 0));
    assert_eq!(
        obj.as_dict().unwrap().get("Type"),
        Some(&PdfObject::Name("Page".into()))
    );
}
