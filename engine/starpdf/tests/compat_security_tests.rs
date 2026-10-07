use starpdf::document::PdfDocument;
use starpdf::security::{CredentialKind, CredentialRefusal, StandardRevision};
use starpdf::syntax::object::PdfObject;
use std::path::Path;

fn pdf(objects: &[&str], trailer_extra: &str) -> Vec<u8> {
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = vec![0];
    for (index, body) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", index + 1, body).as_bytes());
    }
    let xref = bytes.len();
    bytes.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len()).as_bytes());
    for offset in offsets.iter().skip(1) {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R {} >>\nstartxref\n{xref}\n%%EOF\n",
            offsets.len(),
            trailer_extra
        )
        .as_bytes(),
    );
    bytes
}

#[test]
fn unencrypted_capabilities_and_catalog_compatibility() {
    let bytes = pdf(&[
        "<< /Type /Catalog /Pages 2 0 R /Outlines 4 0 R /Names << /EmbeddedFiles 7 0 R >> /OCProperties << /OCGs [10 0 R] /D << /BaseState /OFF /ON [10 0 R] >> >> >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>",
        "<< /Type /Outlines /First 5 0 R /Count 2 >>",
        "<< /Title (Chapter) /Dest [3 0 R /Fit] /First 6 0 R >>",
        "<< /Title (Child) /A << /S /Launch /F (never-executed) >> >>",
        "<< /Names [(readme.txt) 8 0 R] >>",
        "<< /Type /Filespec /F (readme.txt) /EF << /F 9 0 R >> >>",
        "<< /Type /EmbeddedFile /Length 5 >>\nstream\nhello\nendstream",
        "<< /Type /OCG /Name (Layer A) >>",
        "<< /Type /OCMD /OCGs [10 0 R] /P /AllOn >>",
    ], "");
    let mut doc = PdfDocument::from_bytes(&bytes).unwrap();
    let capabilities = doc.security_info().unwrap().capabilities(true);
    assert!(capabilities.can_view.allowed);
    assert!(capabilities.can_edit.allowed);
    let outlines = doc.outlines().unwrap();
    assert_eq!(outlines[0].title, "Chapter");
    assert_eq!(outlines[0].children[0].title, "Child");
    assert_eq!(
        outlines[0].children[0]
            .action
            .as_ref()
            .unwrap()
            .as_dict()
            .unwrap()
            .get("S"),
        Some(&PdfObject::Name("Launch".into()))
    );
    let files = doc.embedded_files().unwrap();
    assert_eq!(files[0].name, "readme.txt");
    assert_eq!(doc.extract_embedded_file(&files[0]).unwrap(), b"hello");
    let groups = doc.optional_content_groups().unwrap();
    assert_eq!(groups[0].default_visible, Some(true));
    let memberships = doc.optional_content_memberships().unwrap();
    assert_eq!(memberships[0].default_visible, Some(true));
}

#[test]
fn encrypted_password_requests_fail_closed() {
    let bytes = pdf(
        &[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>",
            "<< /Filter /Standard /V 4 /R 4 /Length 128 /O <0011> /U <2233> /P -64 >>",
        ],
        "/Encrypt 4 0 R",
    );
    let mut doc = PdfDocument::from_bytes(&bytes).unwrap();
    let info = doc.security_info().unwrap();
    assert_eq!(info.standard_revision, Some(StandardRevision::R4));
    assert!(!info.capabilities(true).can_view.allowed);
    assert_eq!(
        info.capabilities(true).can_extract.reason,
        "COPY_PERMISSION_DENIED"
    );
    assert_eq!(
        info.authorize_credential(CredentialKind::UserPassword, b"wrong"),
        Err(CredentialRefusal::MalformedEncryption)
    );
    assert_eq!(
        info.authorize_credential(CredentialKind::OwnerPassword, b"owner"),
        Err(CredentialRefusal::MalformedEncryption)
    );
    assert!(doc.embedded_files().is_err());
}

#[test]
fn recognized_revision_never_treats_a_supplied_password_as_verified() {
    let owner = "11".repeat(32);
    let user = "22".repeat(32);
    let encryption =
        format!("<< /Filter /Standard /V 4 /R 4 /Length 128 /O <{owner}> /U <{user}> /P -4 >>");
    let bytes = pdf(
        &[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>",
            &encryption,
        ],
        "/Encrypt 4 0 R /ID [<0011223344556677> <0011223344556677>]",
    );
    let mut doc = PdfDocument::from_bytes(&bytes).unwrap();
    let info = doc.security_info().unwrap();
    assert!(info.encryption_dictionary_valid);
    assert_eq!(
        info.authorize_credential(CredentialKind::UserPassword, b"supplied"),
        Err(CredentialRefusal::DecryptionUnavailable)
    );
}

#[test]
fn cyclic_outline_and_filtered_attachment_fail_closed() {
    let bytes = pdf(&[
        "<< /Type /Catalog /Pages 2 0 R /Outlines << /First 4 0 R >> /Names << /EmbeddedFiles << /Names [(x) 5 0 R] >> >> >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>",
        "<< /Title (loop) /Next 4 0 R >>",
        "<< /Type /Filespec /EF << /F 6 0 R >> >>",
        "<< /Type /EmbeddedFile /Filter /FlateDecode /Length 3 >>\nstream\nabc\nendstream",
    ], "");
    let mut doc = PdfDocument::from_bytes(&bytes).unwrap();
    assert!(doc.outlines().is_err());
    let files = doc.embedded_files().unwrap();
    assert!(doc.extract_embedded_file(&files[0]).is_err());
}

#[test]
fn signed_incremental_export_preserves_original_bytes_and_reports_later_revision() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/v0_11_complex/synthetic-signed-valid.pdf");
    let bytes = std::fs::read(path).unwrap();
    let mut doc = PdfDocument::from_bytes(&bytes).unwrap();
    let before = doc.security_info().unwrap();
    assert!(before.signed_revision_end.is_some());
    let output = doc
        .mutate_and_export(&[starpdf::mutation::PdfChange::AddAnnotation {
            page_index: 0,
            spec: starpdf::annotation::AnnotationSpec::Square {
                rect: [10.0, 10.0, 20.0, 20.0],
                stroke_color: None,
                fill_color: None,
                border_width: None,
            },
        }])
        .unwrap();
    assert!(output.starts_with(&bytes));
    let assessment = starpdf::security::assess_signed_export(&bytes, &output, &before);
    assert!(assessment.signed_bytes_preserved);
    assert!(assessment.mutation_appends_after_signed_range);
    assert_eq!(assessment.cryptographic_verification, "NOT_PERFORMED");
    let mut reopened = PdfDocument::from_bytes(&output).unwrap();
    assert!(reopened.security_info().unwrap().later_revision_present);
    assert!(
        !reopened
            .security_info()
            .unwrap()
            .capabilities(true)
            .can_modify_pages
            .allowed
    );
}
