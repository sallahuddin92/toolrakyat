use starpdf::io::ByteSource;
use starpdf::syntax::object::PdfObject;
use starpdf::xref::{XrefEntry, XrefResolver};

#[test]
fn test_find_startxref_simple() {
    let header = b"%PDF-1.7\n1 0 obj\n<<>>\nendobj\n";
    let xref_offset = header.len();
    let body = format!(
        "xref\n0 2\n0000000000 65535 f \n0000000009 00000 n \ntrailer\n<< /Size 2 /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF"
    );
    let mut pdf = Vec::new();
    pdf.extend_from_slice(header);
    pdf.extend_from_slice(body.as_bytes());

    let source = ByteSource::new(&pdf);
    let offset = XrefResolver::find_startxref(source).unwrap();
    assert_eq!(offset, xref_offset as u64);
}

#[test]
fn test_parse_xref_table_subsections() {
    let header = b"%PDF-1.7\n1 0 obj\n<<>>\nendobj\n";
    let xref_offset = header.len();
    let body = format!(
        "xref\n0 1\n0000000000 65535 f \n5 2\n0000000100 00000 n \n0000000200 00000 n \ntrailer\n<< /Size 7 /Root 5 0 R >>\nstartxref\n{xref_offset}\n%%EOF"
    );
    let mut pdf = Vec::new();
    pdf.extend_from_slice(header);
    pdf.extend_from_slice(body.as_bytes());

    let source = ByteSource::new(&pdf);
    let table = XrefResolver::load_xref_and_trailer(source).unwrap();

    assert_eq!(
        table.get_entry(0),
        Some(&XrefEntry::Free {
            next_free_obj: 0,
            generation: 65535
        })
    );
    assert_eq!(
        table.get_entry(5),
        Some(&XrefEntry::InUse {
            byte_offset: 100,
            generation: 0
        })
    );
    assert_eq!(
        table.get_entry(6),
        Some(&XrefEntry::InUse {
            byte_offset: 200,
            generation: 0
        })
    );
    assert_eq!(table.get_offset(5), Some(100));
    assert_eq!(table.get_offset(6), Some(200));
    assert_eq!(table.get_offset(99), None);
    assert_eq!(table.trailer.get("Size"), Some(&PdfObject::Integer(7)));
}

#[test]
fn test_parse_xref_multiple_disjoint_subsections() {
    let header = b"%PDF-1.7\n";
    let xref_offset = header.len();
    let body = format!(
        "xref\n0 1\n0000000000 65535 f \n10 1\n0000000500 00000 n \n20 1\n0000000900 00000 n \ntrailer\n<< /Size 21 /Root 10 0 R >>\nstartxref\n{xref_offset}\n%%EOF"
    );
    let mut pdf = Vec::new();
    pdf.extend_from_slice(header);
    pdf.extend_from_slice(body.as_bytes());

    let source = ByteSource::new(&pdf);
    let table = XrefResolver::load_xref_and_trailer(source).unwrap();

    assert_eq!(table.get_offset(10), Some(500));
    assert_eq!(table.get_offset(20), Some(900));
    assert_eq!(table.get_offset(0), None);
}

#[test]
fn test_xref_entry_predicates() {
    let in_use = XrefEntry::InUse {
        byte_offset: 1234,
        generation: 0,
    };
    let free = XrefEntry::Free {
        next_free_obj: 0,
        generation: 65535,
    };
    assert!(in_use.is_in_use());
    assert!(!free.is_in_use());
    assert_eq!(in_use.byte_offset(), Some(1234));
    assert_eq!(free.byte_offset(), None);
}
