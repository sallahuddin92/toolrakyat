use starpdf::writer::MinimalWriter;
use starpdf::{PdfDocument, XrefStatus};

fn find_last(bytes: &[u8], needle: &[u8]) -> usize {
    bytes
        .windows(needle.len())
        .rposition(|window| window == needle)
        .expect("test marker")
}

fn startxref(bytes: &[u8]) -> usize {
    let marker = find_last(bytes, b"startxref\n") + b"startxref\n".len();
    let end = bytes[marker..]
        .iter()
        .position(|byte| !byte.is_ascii_digit())
        .map(|position| marker + position)
        .expect("startxref terminator");
    std::str::from_utf8(&bytes[marker..end])
        .expect("startxref utf8")
        .parse()
        .expect("startxref integer")
}

fn insert_prev(mut pdf: Vec<u8>, value: &str) -> Vec<u8> {
    let trailer_root = find_last(&pdf, b"  /Root 1 0 R\n") + b"  /Root 1 0 R\n".len();
    pdf.splice(
        trailer_root..trailer_root,
        format!("  /Prev {value}\n").bytes(),
    );
    pdf
}

fn valid_single_xref() -> Vec<u8> {
    MinimalWriter::create_minimal_pdf("Original Alpha Text").expect("minimal PDF")
}

fn forward_prev() -> Vec<u8> {
    let original = valid_single_xref();
    let current_xref = startxref(&original);
    let conflicting_catalog_offset = find_last(&original, b"4 0 obj");
    let mut pdf = insert_prev(original, "0000000000");
    let candidate = pdf.len();
    let placeholder = find_last(&pdf, b"/Prev 0000000000") + b"/Prev ".len();
    pdf[placeholder..placeholder + 10].copy_from_slice(format!("{candidate:010}").as_bytes());
    pdf.extend_from_slice(
        format!(
            "\nxref\n0 2\n0000000000 65535 f \n{conflicting_catalog_offset:010} 00000 n \ntrailer\n<< /Size 5 >>\n"
        )
        .as_bytes(),
    );
    pdf.extend_from_slice(format!("startxref\n{current_xref}\n%%EOF\n").as_bytes());
    pdf
}

fn current_graph_with_prev(value: usize) -> Vec<u8> {
    insert_prev(valid_single_xref(), &value.to_string())
}

#[test]
fn fixture_a_valid_single_xref_uses_strict_path() {
    let bytes = valid_single_xref();
    let mut document = PdfDocument::from_bytes(&bytes).expect("valid PDF");
    assert_eq!(document.xref_status(), XrefStatus::Valid);
    assert_eq!(document.page_count().expect("page count"), 1);
}

#[test]
fn fixture_b_valid_incremental_prev_is_unchanged() {
    let bytes = valid_single_xref();
    let mut document = PdfDocument::from_bytes(&bytes).expect("valid PDF");
    let page = document.extract_page_text(0).expect("extract text");
    let span = page.spans.first().expect("text span");
    let plan = document
        .replace_text_span(0, &span.span_id, "Valid Revision")
        .expect("replace text");
    let output = document.export_incremental(&plan).expect("export");
    let mut reopened = PdfDocument::from_bytes(&output).expect("reopen");
    assert_eq!(reopened.xref_status(), XrefStatus::Valid);
    assert!(reopened
        .extract_page_text(0)
        .expect("reopened text")
        .plain_text()
        .contains("Valid Revision"));
}

#[test]
fn fixture_c_forward_pointing_prev_recovers_checked_xref() {
    let bytes = forward_prev();
    let mut document = PdfDocument::from_bytes(&bytes).expect("recover forward /Prev");
    assert_eq!(document.xref_status(), XrefStatus::RecoveredMalformedPrev);
    assert_eq!(document.page_count().expect("page count"), 1);
}

#[test]
fn fixture_d_prev_beyond_eof_recovers_current_graph_and_exports_cleanly() {
    let bytes = current_graph_with_prev(9_999_999_999);
    let mut document = PdfDocument::from_bytes(&bytes).expect("recover current graph");
    assert_eq!(document.xref_status(), XrefStatus::RecoveredMalformedPrev);

    let page = document.extract_page_text(0).expect("extract text");
    let span = page.spans.first().expect("text span");
    let plan = document
        .replace_text_span(0, &span.span_id, "Recovered Omega Text")
        .expect("safe edit");
    let output = document.export_incremental(&plan).expect("clean export");
    assert!(output[find_last(&output, b"xref\n")..].starts_with(b"xref\n"));
    let terminal_trailer = &output[find_last(&output, b"trailer\n")..];
    assert!(!terminal_trailer.windows(5).any(|window| window == b"/Prev"));

    let mut reopened = PdfDocument::from_bytes(&output).expect("reopen clean output");
    assert_eq!(reopened.xref_status(), XrefStatus::Valid);
    assert!(reopened
        .extract_page_text(0)
        .expect("reopened text")
        .plain_text()
        .contains("Recovered Omega Text"));
}

#[test]
fn fixture_e_cyclic_prev_recovers_only_when_current_graph_is_coherent() {
    let bytes = valid_single_xref();
    let current_xref = startxref(&bytes);
    let cyclic = current_graph_with_prev(current_xref);
    let mut document = PdfDocument::from_bytes(&cyclic).expect("recover cycle");
    assert_eq!(document.xref_status(), XrefStatus::RecoveredMalformedPrev);
    assert_eq!(document.page_count().expect("page count"), 1);
}

#[test]
fn fixture_f_malformed_old_revision_does_not_discard_valid_current_graph() {
    let bytes = current_graph_with_prev(16);
    let mut document = PdfDocument::from_bytes(&bytes).expect("recover malformed history");
    assert_eq!(document.xref_status(), XrefStatus::RecoveredMalformedPrev);
    assert_eq!(document.page_count().expect("page count"), 1);
}

#[test]
fn fixture_g_unrecoverable_current_graph_refuses_with_typed_status() {
    let bytes = valid_single_xref();
    let object_two_offset = find_last(&bytes, b"2 0 obj");
    let entry = format!("{object_two_offset:010} 00000 n ");
    let mut malformed = insert_prev(bytes, "9999999999");
    let position = find_last(&malformed, entry.as_bytes()) + entry.len() - 2;
    malformed[position] = b'f';

    let error = match PdfDocument::from_bytes(&malformed) {
        Ok(_) => panic!("unrecoverable graph must refuse"),
        Err(error) => error,
    };
    assert!(
        error.to_string().contains("XREF_STATUS_UNRECOVERABLE"),
        "unexpected refusal: {error}"
    );
}
