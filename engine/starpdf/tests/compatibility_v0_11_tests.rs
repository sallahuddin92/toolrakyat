use std::path::{Path, PathBuf};

use starpdf::annotation::AnnotationSpec;
use starpdf::document::PdfDocument;
use starpdf::forms::FieldGraphClassification;
use starpdf::mutation::PdfChange;
use starpdf::security::{EncryptionState, SignatureState};
use starpdf::syntax::object::PdfObject;
use starpdf::syntax::ObjectRef;
use starpdf::xref::table::{XrefEntry, XrefKind};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/v0_11_complex")
        .join(name)
}

fn open(name: &str) -> PdfDocument<'static> {
    let bytes = std::fs::read(fixture(name)).unwrap_or_else(|error| panic!("{name}: {error}"));
    let leaked = Box::leak(bytes.into_boxed_slice());
    PdfDocument::from_bytes(leaked).unwrap_or_else(|error| panic!("{name}: {error}"))
}

fn two_revision_object(replacement: Option<(u16, &str)>) -> Vec<u8> {
    two_revision_object_with_id(replacement, "")
}

fn two_revision_object_with_id(replacement: Option<(u16, &str)>, id_entry: &str) -> Vec<u8> {
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (number, generation, body) in [
        (1, 0, "<< /Type /Catalog /Pages 2 0 R >>"),
        (2, 0, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, 0, "<< /Type /Page /Parent 2 0 R >>"),
        (5, 0, "(old)"),
    ] {
        offsets.push((number, pdf.len()));
        pdf.extend_from_slice(format!("{number} {generation} obj\n{body}\nendobj\n").as_bytes());
    }
    let first_xref = pdf.len();
    pdf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for number in 1..6 {
        if let Some((_, offset)) = offsets.iter().find(|(candidate, _)| *candidate == number) {
            pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        } else {
            pdf.extend_from_slice(b"0000000000 00000 f \n");
        }
    }
    pdf.extend_from_slice(
        format!("trailer\n<< /Size 6 /Root 1 0 R {id_entry} >>\nstartxref\n{first_xref}\n%%EOF\n")
            .as_bytes(),
    );
    let replacement_offset = replacement.map(|(generation, body)| {
        let offset = pdf.len();
        pdf.extend_from_slice(format!("5 {generation} obj\n{body}\nendobj\n").as_bytes());
        (offset, generation)
    });
    let second_xref = pdf.len();
    pdf.extend_from_slice(b"xref\n5 1\n");
    if let Some((offset, generation)) = replacement_offset {
        pdf.extend_from_slice(format!("{offset:010} {generation:05} n \n").as_bytes());
    } else {
        pdf.extend_from_slice(b"0000000000 00001 f \n");
    }
    pdf.extend_from_slice(
        format!("trailer\n<< /Size 6 /Root 1 0 R /Prev {first_xref} >>\nstartxref\n{second_xref}\n%%EOF\n")
            .as_bytes(),
    );
    pdf
}

fn xref_stream_entry(kind: u8, field2: u32, field3: u16) -> [u8; 7] {
    [
        kind,
        ((field2 >> 24) & 0xff) as u8,
        ((field2 >> 16) & 0xff) as u8,
        ((field2 >> 8) & 0xff) as u8,
        (field2 & 0xff) as u8,
        ((field3 >> 8) & 0xff) as u8,
        (field3 & 0xff) as u8,
    ]
}

fn stream_classic_stream_replacement() -> Vec<u8> {
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>"),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, "<< /Type /Page /Parent 2 0 R >>"),
        (
            4,
            "<< /Type /ObjStm /N 0 /First 0 /Length 0 >>\nstream\n\nendstream",
        ),
    ] {
        offsets.push((number, pdf.len()));
        pdf.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let first_stream = pdf.len();
    let mut first_data = Vec::new();
    first_data.extend_from_slice(&xref_stream_entry(0, 0, u16::MAX));
    for number in 1..=4 {
        let offset = offsets
            .iter()
            .find(|(candidate, _)| *candidate == number)
            .map(|(_, offset)| *offset)
            .unwrap_or(0);
        first_data.extend_from_slice(&xref_stream_entry(1, offset as u32, 0));
    }
    first_data.extend_from_slice(&xref_stream_entry(2, 4, 0));
    first_data.extend_from_slice(&xref_stream_entry(1, first_stream as u32, 0));
    pdf.extend_from_slice(
        format!(
            "6 0 obj\n<< /Type /XRef /Size 7 /Root 1 0 R /W [1 4 2] /Index [0 7] /Length {} >>\nstream\n",
            first_data.len()
        )
        .as_bytes(),
    );
    pdf.extend_from_slice(&first_data);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    let classic_object = pdf.len();
    pdf.extend_from_slice(b"5 0 obj\n(classic-middle)\nendobj\n");
    let classic_xref = pdf.len();
    pdf.extend_from_slice(
        format!("xref\n5 1\n{classic_object:010} 00000 n \ntrailer\n<< /Size 7 /Prev {first_stream} >>\nstartxref\n{classic_xref}\n%%EOF\n")
            .as_bytes(),
    );
    let latest_object = pdf.len();
    pdf.extend_from_slice(b"5 0 obj\n(stream-latest)\nendobj\n");
    let latest_stream = pdf.len();
    let mut latest_data = Vec::new();
    latest_data.extend_from_slice(&xref_stream_entry(1, latest_object as u32, 0));
    latest_data.extend_from_slice(&xref_stream_entry(1, latest_stream as u32, 0));
    pdf.extend_from_slice(
        format!(
            "7 0 obj\n<< /Type /XRef /Size 8 /W [1 4 2] /Index [5 1 7 1] /Prev {classic_xref} /Length {} >>\nstream\n",
            latest_data.len()
        )
        .as_bytes(),
    );
    pdf.extend_from_slice(&latest_data);
    pdf.extend_from_slice(
        format!("\nendstream\nendobj\nstartxref\n{latest_stream}\n%%EOF\n").as_bytes(),
    );
    pdf
}

#[test]
fn signature_states_and_byte_ranges_are_structurally_classified() {
    let mut signed = open("synthetic-signed-valid.pdf");
    let info = signed.security_info().unwrap();
    assert_eq!(info.signature_state, SignatureState::SignedWithByteRange);
    assert_eq!(info.signature_count, 1);
    assert_eq!(info.byte_ranges.len(), 1);
    assert!(info.mutation_allowed);

    let mut field_only = open("synthetic-signature-field-only.pdf");
    let info = field_only.security_info().unwrap();
    assert_eq!(info.signature_state, SignatureState::SignedPresent);
    assert_eq!(info.signature_count, 1);

    let mut malformed = open("synthetic-signed-malformed.pdf");
    let info = malformed.security_info().unwrap();
    assert_eq!(
        info.signature_state,
        SignatureState::SignedStructureMalformed
    );
    assert!(!info.mutation_allowed);
}

#[test]
fn signed_unrelated_update_preserves_prefix_signature_and_adds_revision() {
    let bytes = std::fs::read(fixture("synthetic-signed-valid.pdf")).unwrap();
    let mut document = PdfDocument::from_bytes(&bytes).unwrap();
    let signature_field = document
        .form_fields()
        .unwrap()
        .into_iter()
        .find(|field| field.is_signature())
        .unwrap();
    let output = document
        .mutate_and_export(&[PdfChange::AddAnnotation {
            page_index: 0,
            spec: AnnotationSpec::Square {
                rect: [20.0, 20.0, 40.0, 40.0],
                stroke_color: Some(vec![0.1, 0.2, 0.8]),
                fill_color: None,
                border_width: Some(1.0),
            },
        }])
        .unwrap();
    assert!(output.starts_with(&bytes));
    let mut reopened = PdfDocument::from_bytes(&output).unwrap();
    assert_eq!(
        reopened.security_info().unwrap().signature_state,
        SignatureState::SignedWithByteRange
    );
    assert!(reopened
        .form_fields()
        .unwrap()
        .iter()
        .any(|field| field.object_ref == signature_field.object_ref && field.is_signature()));
    assert_eq!(reopened.store().xref().revisions.len(), 2);
}

#[test]
fn malformed_signature_and_signature_field_mutation_refuse_atomically() {
    let malformed_bytes = std::fs::read(fixture("synthetic-signed-malformed.pdf")).unwrap();
    let mut malformed = PdfDocument::from_bytes(&malformed_bytes).unwrap();
    let error = malformed
        .apply_mutation(&[PdfChange::AddAnnotation {
            page_index: 0,
            spec: AnnotationSpec::Square {
                rect: [1.0, 1.0, 2.0, 2.0],
                stroke_color: None,
                fill_color: None,
                border_width: None,
            },
        }])
        .unwrap_err();
    assert!(error.to_string().contains("SIGNATURE_MUTATION_UNSUPPORTED"));

    let mut field_only = open("synthetic-signature-field-only.pdf");
    let signature = field_only.form_fields().unwrap().remove(0);
    let error = field_only
        .apply_mutation(&[PdfChange::SetTextField {
            field_ref: signature.object_ref,
            value: "not a signature".into(),
        }])
        .unwrap_err();
    assert!(error.to_string().contains("SIGNATURE_MUTATION_UNSUPPORTED"));
}

#[test]
fn encryption_handlers_and_permissions_are_detected_and_mutation_refuses() {
    let cases = [
        (
            "synthetic-encrypted-standard.pdf",
            EncryptionState::StandardSecurityDetected,
        ),
        (
            "synthetic-encrypted-public-key.pdf",
            EncryptionState::PublicKeySecurityDetected,
        ),
        (
            "synthetic-encrypted-malformed.pdf",
            EncryptionState::MalformedEncryptionDictionary,
        ),
    ];
    for (name, expected) in cases {
        let mut document = open(name);
        let info = document.security_info().unwrap();
        assert_eq!(info.encryption_state, expected, "{name}");
        assert!(!info.mutation_allowed, "{name}");
        if expected == EncryptionState::StandardSecurityDetected {
            assert_eq!(info.permissions.raw, Some(-4));
            assert_eq!(info.permissions.modification, Some(true));
        }
        let error = document
            .apply_mutation(&[PdfChange::AddAnnotation {
                page_index: 0,
                spec: AnnotationSpec::Square {
                    rect: [1.0, 1.0, 2.0, 2.0],
                    stroke_color: None,
                    fill_color: None,
                    border_width: None,
                },
            }])
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("ENCRYPTED_DOCUMENT_MUTATION_UNSUPPORTED"));
    }
}

#[test]
fn metadata_catalog_info_and_document_id_survive_unrelated_update() {
    let bytes = std::fs::read(fixture("synthetic-metadata-rich.pdf")).unwrap();
    let mut document = PdfDocument::from_bytes(&bytes).unwrap();
    let catalog_ref = document.catalog_ref();
    let catalog = document.store_mut().resolve(catalog_ref).unwrap().clone();
    let trailer_id = document.trailer().get("ID").cloned();
    let info_ref = document.trailer().get("Info").cloned();
    let output = document
        .mutate_and_export(&[PdfChange::AddAnnotation {
            page_index: 0,
            spec: AnnotationSpec::Square {
                rect: [20.0, 20.0, 40.0, 40.0],
                stroke_color: None,
                fill_color: None,
                border_width: None,
            },
        }])
        .unwrap();
    let mut reopened = PdfDocument::from_bytes(&output).unwrap();
    assert_eq!(reopened.trailer().get("ID"), trailer_id.as_ref());
    assert_eq!(reopened.trailer().get("Info"), info_ref.as_ref());
    assert_eq!(reopened.store_mut().resolve(catalog_ref).unwrap(), &catalog);
    for key in [
        "Lang",
        "ViewerPreferences",
        "PageMode",
        "PageLayout",
        "OpenAction",
        "Names",
        "Outlines",
        "Metadata",
        "StarPDFUnknown",
    ] {
        assert!(catalog.as_dict().unwrap().contains_key(key));
    }
}

#[test]
fn missing_and_malformed_document_ids_are_preserved_without_regeneration() {
    for (id_entry, expected) in [
        ("", None),
        (
            "/ID (malformed)",
            Some(PdfObject::String(b"malformed".to_vec())),
        ),
    ] {
        let bytes = two_revision_object_with_id(Some((0, "(latest)")), id_entry);
        let mut document = PdfDocument::from_bytes(&bytes).unwrap();
        assert_eq!(document.trailer().get("ID").cloned(), expected);
        let output = document
            .mutate_and_export(&[PdfChange::AddAnnotation {
                page_index: 0,
                spec: AnnotationSpec::Square {
                    rect: [20.0, 20.0, 40.0, 40.0],
                    stroke_color: None,
                    fill_color: None,
                    border_width: None,
                },
            }])
            .unwrap();
        assert!(output.starts_with(&bytes));
        let reopened = PdfDocument::from_bytes(&output).unwrap();
        assert_eq!(reopened.trailer().get("ID").cloned(), expected);
    }
}

#[test]
fn ambiguous_orphan_radio_widgets_are_inspectable_but_not_group_mutable() {
    let mut document = open("synthetic-ambiguous-orphan-radio.pdf");
    let fields = document.form_fields().unwrap();
    assert_eq!(fields.len(), 2);
    assert!(fields.iter().all(|field| {
        field.graph_classification == FieldGraphClassification::AmbiguousWidgetGroup
    }));
    let error = document
        .apply_mutation(&[PdfChange::SetRadio {
            parent_ref: fields[0].object_ref,
            selected_widget_ref: fields[0].object_ref,
            on_state: "A".into(),
        }])
        .unwrap_err();
    assert!(error.to_string().contains("AMBIGUOUS_FIELD_GRAPH"));
}

#[test]
fn malformed_parent_relationship_is_inspectable_but_not_group_mutable() {
    let mut document = open("synthetic-malformed-parent-radio.pdf");
    let fields = document.form_fields().unwrap();
    assert_eq!(fields.len(), 1);
    assert_eq!(
        fields[0].graph_classification,
        FieldGraphClassification::MalformedFieldGraph
    );
    let error = document
        .apply_mutation(&[PdfChange::SetRadio {
            parent_ref: fields[0].object_ref,
            selected_widget_ref: fields[0].widgets[0].object_ref,
            on_state: "A".into(),
        }])
        .unwrap_err();
    assert!(error.to_string().contains("AMBIGUOUS_FIELD_GRAPH"));
}

#[test]
fn hybrid_revision_chain_uses_latest_uncompressed_definition_and_latest_id() {
    let mut document = open("synthetic-hybrid-multi-revision.pdf");
    let revisions = &document.store().xref().revisions;
    assert_eq!(revisions.len(), 3);
    assert_eq!(revisions[0].kind, XrefKind::Classic);
    assert_eq!(revisions[1].kind, XrefKind::Classic);
    assert_eq!(revisions[2].kind, XrefKind::HybridStream);
    assert!(revisions[0].prev_offset.is_some());
    assert!(revisions[1].xrefstm_offset.is_some());
    assert!(matches!(
        document.store().xref().get_entry(3),
        Some(XrefEntry::InUse { .. })
    ));
    let page = document.page_dict(0).unwrap();
    assert_eq!(
        page.get("StarPDFRevision")
            .and_then(PdfObject::as_string_lossy),
        Some("latest".into())
    );
    let ids = document
        .trailer()
        .get("ID")
        .and_then(PdfObject::as_array)
        .unwrap();
    assert_eq!(ids[1].as_string_lossy().as_deref(), Some("DU"));
}

#[test]
fn hybrid_signed_encrypted_chain_uses_effective_security_and_trailer_state() {
    let mut document = open("synthetic-hybrid-signed-encrypted.pdf");
    assert_eq!(document.store().xref().revisions.len(), 3);
    assert_eq!(document.catalog_ref(), ObjectRef::new(1, 0));
    let security = document.security_info().unwrap();
    assert_eq!(
        security.signature_state,
        SignatureState::SignedWithByteRange
    );
    assert_eq!(
        security.encryption_state,
        EncryptionState::StandardSecurityDetected
    );
    assert!(!security.mutation_allowed);
    let page = document.page_dict(0).unwrap();
    assert_eq!(
        page.get("StarPDFRevision")
            .and_then(PdfObject::as_string_lossy)
            .as_deref(),
        Some("security-latest")
    );
    let ids = document
        .trailer()
        .get("ID")
        .and_then(PdfObject::as_array)
        .unwrap();
    assert_eq!(ids[1].as_string_lossy().as_deref(), Some("DU"));
    assert_eq!(
        document.trailer().get("Encrypt"),
        Some(&PdfObject::Reference(ObjectRef::new(8, 0)))
    );
}

#[test]
fn xref_conflicts_use_latest_valid_revision_free_state_and_generation() {
    let replacement = two_revision_object(Some((0, "(latest)")));
    let mut document = PdfDocument::from_bytes(&replacement).unwrap();
    assert_eq!(
        document
            .store_mut()
            .resolve(ObjectRef::new(5, 0))
            .unwrap()
            .as_string_lossy()
            .as_deref(),
        Some("latest")
    );

    let freed = two_revision_object(None);
    let mut document = PdfDocument::from_bytes(&freed).unwrap();
    assert!(matches!(
        document.store().xref().get_entry(5),
        Some(XrefEntry::Free { generation: 1, .. })
    ));
    assert!(document.store_mut().resolve(ObjectRef::new(5, 0)).is_err());

    let reused = two_revision_object(Some((1, "(generation-one)")));
    let mut document = PdfDocument::from_bytes(&reused).unwrap();
    assert!(document.store_mut().resolve(ObjectRef::new(5, 0)).is_err());
    assert_eq!(
        document
            .store_mut()
            .resolve(ObjectRef::new(5, 1))
            .unwrap()
            .as_string_lossy()
            .as_deref(),
        Some("generation-one")
    );
}

#[test]
fn xref_stream_classic_stream_chain_replaces_compressed_entry_with_latest_object() {
    let bytes = stream_classic_stream_replacement();
    let mut document = PdfDocument::from_bytes(&bytes).unwrap();
    let revisions = &document.store().xref().revisions;
    assert_eq!(revisions.len(), 3);
    assert_eq!(revisions[0].kind, XrefKind::Stream);
    assert_eq!(revisions[1].kind, XrefKind::Classic);
    assert_eq!(revisions[2].kind, XrefKind::Stream);
    assert_eq!(
        document
            .store_mut()
            .resolve(ObjectRef::new(5, 0))
            .unwrap()
            .as_string_lossy()
            .as_deref(),
        Some("stream-latest")
    );
}

#[test]
fn corpus_expansion_contains_eleven_deterministic_non_sensitive_fixtures() {
    let files = std::fs::read_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/v0_11_complex"),
    )
    .unwrap()
    .filter_map(Result::ok)
    .filter(|entry| entry.path().extension().and_then(|value| value.to_str()) == Some("pdf"))
    .count();
    assert_eq!(files, 11);
}
