use std::collections::BTreeMap;
use std::io::Write;

use starpdf::appearance::AppearanceStatus;
use starpdf::syntax::object::{ObjectRef, PdfObject, StreamObject};
use starpdf::{
    ByteSource, MutationPlan, PdfDocument, PdfError, Serializer, XrefEntry, XrefResolver,
    XrefStatus,
};

fn append_object(output: &mut Vec<u8>, reference: ObjectRef, object: &PdfObject) -> usize {
    let offset = output.len();
    writeln!(output, "{} {} obj", reference.number, reference.generation).unwrap();
    Serializer::write_object(output, object).unwrap();
    output.extend_from_slice(b"\nendobj\n");
    offset
}

fn object_stream() -> StreamObject {
    let bodies = [
        b"<< /Type /Catalog /Pages 2 0 R >>".as_slice(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".as_slice(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 6 0 R >> >> /Contents 5 0 R >>".as_slice(),
        b"(Compressed Sentinel)".as_slice(),
    ];
    let object_numbers = [1u64, 2, 3, 20];
    let mut body_data = Vec::new();
    let mut offsets = Vec::new();
    for body in bodies {
        offsets.push(body_data.len());
        body_data.extend_from_slice(body);
        body_data.push(b' ');
    }
    let header = object_numbers
        .iter()
        .zip(offsets)
        .map(|(number, offset)| format!("{number} {offset}"))
        .collect::<Vec<_>>()
        .join(" ")
        + " ";
    let mut data = header.as_bytes().to_vec();
    data.extend_from_slice(&body_data);
    StreamObject {
        dict: BTreeMap::from([
            ("Type".into(), PdfObject::Name("ObjStm".into())),
            ("N".into(), PdfObject::Integer(4)),
            ("First".into(), PdfObject::Integer(header.len() as i64)),
        ]),
        stream_offset: 0,
        stream_length: data.len(),
        data,
    }
}

fn push_xref_record(data: &mut Vec<u8>, entry_type: u8, field2: u64, field3: u16) {
    data.push(entry_type);
    data.extend_from_slice(&(field2 as u32).to_be_bytes());
    data.extend_from_slice(&field3.to_be_bytes());
}

fn recovered_compressed_fixture(invalid_sentinel_index: bool) -> Vec<u8> {
    let mut output = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let objstm_offset = append_object(
        &mut output,
        ObjectRef::new(4, 0),
        &PdfObject::Stream(object_stream()),
    );
    let content = StreamObject {
        dict: BTreeMap::new(),
        data: b"BT /F1 12 Tf 72 720 Td (Original Alpha Text) Tj ET".to_vec(),
        stream_offset: 0,
        stream_length: 0,
    };
    let content_offset = append_object(
        &mut output,
        ObjectRef::new(5, 0),
        &PdfObject::Stream(content),
    );
    let font_offset = append_object(
        &mut output,
        ObjectRef::new(6, 0),
        &PdfObject::Dictionary(BTreeMap::from([
            ("Type".into(), PdfObject::Name("Font".into())),
            ("Subtype".into(), PdfObject::Name("Type1".into())),
            ("BaseFont".into(), PdfObject::Name("Helvetica".into())),
        ])),
    );
    let info_offset = append_object(
        &mut output,
        ObjectRef::new(10, 0),
        &PdfObject::Dictionary(BTreeMap::from([(
            "Producer".into(),
            PdfObject::String(b"StarPDF test".to_vec()),
        )])),
    );
    let xref_offset = output.len();

    let mut xref_data = Vec::new();
    push_xref_record(&mut xref_data, 0, 0, u16::MAX);
    push_xref_record(&mut xref_data, 2, 4, 0);
    push_xref_record(&mut xref_data, 2, 4, 1);
    push_xref_record(&mut xref_data, 2, 4, 2);
    push_xref_record(&mut xref_data, 1, objstm_offset as u64, 0);
    push_xref_record(&mut xref_data, 1, content_offset as u64, 0);
    push_xref_record(&mut xref_data, 1, font_offset as u64, 0);
    push_xref_record(&mut xref_data, 1, info_offset as u64, 0);
    push_xref_record(
        &mut xref_data,
        2,
        4,
        if invalid_sentinel_index { 0 } else { 3 },
    );
    push_xref_record(&mut xref_data, 1, xref_offset as u64, 0);

    let xref_stream = StreamObject {
        dict: BTreeMap::from([
            ("Type".into(), PdfObject::Name("XRef".into())),
            ("Size".into(), PdfObject::Integer(31)),
            (
                "W".into(),
                PdfObject::Array(vec![
                    PdfObject::Integer(1),
                    PdfObject::Integer(4),
                    PdfObject::Integer(2),
                ]),
            ),
            (
                "Index".into(),
                PdfObject::Array(vec![
                    PdfObject::Integer(0),
                    PdfObject::Integer(7),
                    PdfObject::Integer(10),
                    PdfObject::Integer(1),
                    PdfObject::Integer(20),
                    PdfObject::Integer(1),
                    PdfObject::Integer(30),
                    PdfObject::Integer(1),
                ]),
            ),
            ("Root".into(), PdfObject::Reference(ObjectRef::new(1, 0))),
            ("Info".into(), PdfObject::Reference(ObjectRef::new(10, 0))),
            (
                "ID".into(),
                PdfObject::Array(vec![
                    PdfObject::String(b"fixture-id-one".to_vec()),
                    PdfObject::String(b"fixture-id-two".to_vec()),
                ]),
            ),
            ("Prev".into(), PdfObject::Integer(99_999_999)),
        ]),
        stream_offset: 0,
        stream_length: xref_data.len(),
        data: xref_data,
    };
    append_object(
        &mut output,
        ObjectRef::new(30, 0),
        &PdfObject::Stream(xref_stream),
    );
    write!(output, "startxref\n{xref_offset}\n%%EOF\n").unwrap();
    output
}

fn terminal_xref_stream(bytes: &[u8]) -> (ObjectRef, StreamObject) {
    let offset = XrefResolver::find_startxref(ByteSource::new(bytes)).unwrap() as usize;
    let (reference, object) = starpdf::syntax::parser::Parser::from_bytes(&bytes[offset..])
        .parse_indirect_object()
        .unwrap();
    (reference, object.as_stream().unwrap().clone())
}

#[test]
fn recovered_compressed_export_serializes_complete_sparse_xref_stream() {
    let input = recovered_compressed_fixture(false);
    let mut document = PdfDocument::from_bytes(&input).expect("recovered open");
    assert_eq!(document.xref_status(), XrefStatus::RecoveredMalformedPrev);
    let page = document.extract_page_text(0).expect("extract text");
    let span = page.spans.first().expect("text span");
    let plan = document
        .replace_text_span(0, &span.span_id, "Recovered Beta Text")
        .expect("native edit");
    let output = document
        .export_incremental(&plan)
        .expect("xref-stream export");

    let table = XrefResolver::load_xref_and_trailer(ByteSource::new(&output)).unwrap();
    assert!(matches!(table.get_entry(0), Some(XrefEntry::Free { .. })));
    assert!(matches!(table.get_entry(5), Some(XrefEntry::InUse { .. })));
    assert_eq!(
        table.get_entry(20),
        Some(&XrefEntry::Compressed {
            stream_obj_num: 4,
            index_in_stream: 3,
        })
    );
    let (xref_ref, stream) = terminal_xref_stream(&output);
    assert!(matches!(
        table.get_entry(xref_ref.number),
        Some(XrefEntry::InUse { byte_offset, generation: 0 })
            if *byte_offset == table.startxref_offset
    ));
    assert_eq!(
        stream.dict.get("Type").and_then(PdfObject::as_name),
        Some("XRef")
    );
    assert!(stream.dict.contains_key("Index"));
    assert_eq!(
        stream.dict.get("Root"),
        Some(&PdfObject::Reference(ObjectRef::new(1, 0)))
    );
    assert_eq!(
        stream.dict.get("Info"),
        Some(&PdfObject::Reference(ObjectRef::new(10, 0)))
    );
    assert!(stream.dict.contains_key("ID"));
    assert!(!stream.dict.contains_key("Prev"));
    assert!(!stream.dict.contains_key("XRefStm"));

    let mut reopened = PdfDocument::from_bytes(&output).expect("valid reopen");
    assert_eq!(reopened.xref_status(), XrefStatus::Valid);
    assert!(reopened
        .extract_page_text(0)
        .unwrap()
        .plain_text()
        .contains("Recovered Beta Text"));
    assert_eq!(
        reopened.store_mut().resolve(ObjectRef::new(20, 0)).unwrap(),
        &PdfObject::String(b"Compressed Sentinel".to_vec())
    );
}

#[test]
fn modified_compressed_object_becomes_type_one() {
    let input = recovered_compressed_fixture(false);
    let mut document = PdfDocument::from_bytes(&input).expect("recovered open");
    let replacement_catalog = PdfObject::Dictionary(BTreeMap::from([
        ("Type".into(), PdfObject::Name("Catalog".into())),
        ("Pages".into(), PdfObject::Reference(ObjectRef::new(2, 0))),
        ("Marker".into(), PdfObject::String(b"replacement".to_vec())),
    ]));
    let plan = MutationPlan {
        modified_objects: BTreeMap::from([(ObjectRef::new(1, 0), replacement_catalog)]),
        appearance_status: AppearanceStatus::AppearancePreserved,
        glyph_mapping_quality: None,
        layout_policy_result: None,
    };
    let output = document.export_incremental(&plan).expect("export");
    let table = XrefResolver::load_xref_and_trailer(ByteSource::new(&output)).unwrap();
    assert!(matches!(
        table.get_entry(1),
        Some(XrefEntry::InUse { generation: 0, .. })
    ));
    let mut reopened = PdfDocument::from_bytes(&output).expect("reopen");
    let catalog = reopened.store_mut().resolve(ObjectRef::new(1, 0)).unwrap();
    assert_eq!(
        catalog
            .as_dict()
            .and_then(|dict| dict.get("Marker"))
            .and_then(PdfObject::as_bytes),
        Some(b"replacement".as_slice())
    );
}

#[test]
fn invalid_retained_compressed_reference_refuses_before_output() {
    let input = recovered_compressed_fixture(true);
    let mut document = PdfDocument::from_bytes(&input).expect("coherent current page graph");
    let page = document.extract_page_text(0).expect("extract text");
    let span = page.spans.first().expect("text span");
    let plan = document
        .replace_text_span(0, &span.span_id, "Refusal Check")
        .expect("prepare edit");
    let error = document.export_incremental(&plan).unwrap_err();
    assert!(matches!(error, PdfError::RecoveredXrefExport(_)));
    assert!(error.to_string().contains("expected 20"));
}
