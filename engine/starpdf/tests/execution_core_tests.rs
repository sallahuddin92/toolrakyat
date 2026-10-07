use starpdf::execution::*;
use starpdf::syntax::object::{ObjectRef, PdfObject};
use starpdf::{MinimalWriter, PdfDocument};
use std::collections::{BTreeMap, BTreeSet};
fn r(n: u64) -> ObjectRef {
    ObjectRef::new(n, 0)
}
fn h(n: u8) -> ContentHash {
    ContentHash([n; 32])
}

#[test]
fn semantic_object_hash_is_stable() {
    let a = PdfObject::Dictionary(BTreeMap::from([
        ("B".into(), PdfObject::Real(-0.0)),
        ("A".into(), PdfObject::String(b"x".to_vec())),
    ]));
    let b = PdfObject::Dictionary(BTreeMap::from([
        ("A".into(), PdfObject::String(b"x".to_vec())),
        ("B".into(), PdfObject::Real(0.0)),
    ]));
    assert_eq!(
        object_content_hash(&a).unwrap(),
        object_content_hash(&b).unwrap()
    );
    assert_ne!(
        object_content_hash(&a).unwrap(),
        object_content_hash(&PdfObject::Null).unwrap()
    );
    assert!(object_content_hash(&PdfObject::Real(f64::NAN)).is_err());
}

#[test]
fn lazy_verification_and_occurrence_policy() {
    let bytes = MinimalWriter::create_minimal_pdf("proof").unwrap();
    let mut document = PdfDocument::from_bytes(&bytes).unwrap();
    let mut state = StarExecutionState::from_document(&document, 5);
    let node = StarNodeId::new("doc", r(4), "text:0").unwrap();
    let policy = EvidencePolicy {
        operation: "text style".into(),
        accepted_levels: BTreeSet::from([EvidenceLevel::Verified]),
        required_revision: 5,
    };
    state.record_occurrence_evidence(
        node.clone(),
        EvidenceRecord {
            level: EvidenceLevel::Verified,
            proof: "occurrence inspected".into(),
            source_revision: Some(5),
            provenance: None,
        },
    );
    assert!(state.require_capability(&node, &policy).is_err());
    let previous = document.store().metrics().objects_resolved;
    let hash = state.verify_object(document.store_mut(), r(4)).unwrap();
    assert!(document.store().metrics().objects_resolved <= previous + 1);
    state
        .link_occurrence(node.clone(), r(4), None, Some("paint".into()))
        .unwrap();
    state.require_capability(&node, &policy).unwrap();
    assert_eq!(state.provenance[&node][0].object_ref, r(4));
    let mut reopened = PdfDocument::from_bytes(&bytes).unwrap();
    let mut again = StarExecutionState::from_document(&reopened, 5);
    assert_eq!(
        hash,
        again.verify_object(reopened.store_mut(), r(4)).unwrap()
    );
}

#[test]
fn delta_and_closure_are_deterministic_and_fail_closed() {
    let before = ObjectSnapshot(BTreeMap::from([(r(2), h(1)), (r(3), h(2))]));
    let after = ObjectSnapshot(BTreeMap::from([(r(3), h(3)), (r(2), h(1)), (r(4), h(4))]));
    let delta = before.diff(&after);
    assert_eq!(delta.modified_objects, BTreeSet::from([r(3)]));
    assert_eq!(delta.added_objects, BTreeSet::from([r(4)]));
    let mut graph = DependencyGraph::default();
    graph.inspect_for(SupportedEdit::ImageReplace, r(3), [r(2)], []);
    graph.inspect_for(SupportedEdit::ImageReplace, r(4), [], []);
    assert_eq!(
        graph
            .closure(SupportedEdit::ImageReplace, &delta)
            .completeness,
        ClosureCompleteness::Incomplete
    );
    graph.inspect_for(SupportedEdit::ImageReplace, r(2), [], []);
    assert_eq!(
        graph.closure(SupportedEdit::ImageReplace, &delta).objects,
        BTreeSet::from([r(2), r(3), r(4)])
    );
    assert_eq!(
        graph
            .closure(SupportedEdit::FreeTextStyle, &delta)
            .completeness,
        ClosureCompleteness::Incomplete
    );
}

#[test]
fn report_exposes_forbidden_and_unexplained_changes() {
    let delta = DocumentDelta {
        modified_objects: BTreeSet::from([r(3)]),
        before_hashes: BTreeMap::from([(r(3), h(1))]),
        after_hashes: BTreeMap::from([(r(3), h(2))]),
        ..Default::default()
    };
    let envelope = MutationEnvelope {
        forbidden_dependencies: BTreeSet::from([r(2)]),
        ..Default::default()
    };
    let structural = StructuralClosure {
        objects: BTreeSet::from([r(2), r(3)]),
        completeness: ClosureCompleteness::Complete,
        ..Default::default()
    };
    let visual = VisualClosure {
        completeness: ClosureCompleteness::Complete,
        ..Default::default()
    };
    let report = envelope.evaluate(&delta, &structural, &visual);
    assert_eq!(report.forbidden_dependency_touches, BTreeSet::from([r(2)]));
    assert_eq!(
        report.certificate.classification,
        CertificateClassification::Refused
    );
    let clean = StructuralClosure {
        objects: BTreeSet::from([r(3)]),
        ..structural
    };
    let report = envelope.evaluate(&delta, &clean, &visual);
    assert_eq!(
        report.certificate.classification,
        CertificateClassification::Unexplained
    );
    assert!(report
        .certificate
        .unexplained_changes
        .contains("object:3 0 R"));
}

#[test]
fn effective_index_follows_incremental_replacement_and_reopen() {
    let bytes = MinimalWriter::create_minimal_pdf("old").unwrap();
    let mut document = PdfDocument::from_bytes(&bytes).unwrap();
    let page = document.extract_page_text(0).unwrap();
    let plan = document
        .replace_text_span(0, &page.spans[0].span_id, "new")
        .unwrap();
    let output = document.export_incremental(&plan).unwrap();
    let reopened = PdfDocument::from_bytes(&output).unwrap();
    let state = StarExecutionState::from_document(&reopened, 2);
    assert!(reopened.store().xref().revisions.len() >= 2);
    assert!(state
        .effective_object_index
        .values()
        .any(|entry| entry.source_revision == Some(1)));
    assert!(state
        .effective_object_index
        .values()
        .any(|entry| entry.source_revision == Some(0)));
}

#[test]
fn bounded_recovered_object_keeps_actual_origin() {
    let mut bytes = MinimalWriter::create_minimal_pdf("drift").unwrap();
    let object_start = bytes
        .windows(b"4 0 obj".len())
        .position(|w| w == b"4 0 obj")
        .unwrap();
    let xref_line = format!("{object_start:010} 00000 n ");
    let line_start = bytes
        .windows(xref_line.len())
        .position(|w| w == xref_line.as_bytes())
        .unwrap();
    bytes[line_start..line_start + 10]
        .copy_from_slice(format!("{:010}", object_start + 1).as_bytes());
    let document = PdfDocument::from_bytes(&bytes).unwrap();
    let state = StarExecutionState::from_document(&document, 1);
    let object = &state.effective_object_index[&r(4)];
    assert_eq!(object.storage_kind, StorageKind::Recovered);
    assert_eq!(
        object.source_location,
        SourceLocation::FileOffset(object_start as u64)
    );
}

#[test]
fn replacement_keeps_origin_and_never_inherits_edit_proof() {
    let bytes = MinimalWriter::create_minimal_pdf("origin").unwrap();
    let mut document = PdfDocument::from_bytes(&bytes).unwrap();
    let mut state = StarExecutionState::from_document(&document, 7);
    state.verify_object(document.store_mut(), r(4)).unwrap();
    let origin = state.effective_object_index[&r(4)].source_location.clone();
    state.stage_replacement(r(4), h(9)).unwrap();
    assert_eq!(state.object_origins[&r(4)][0].source, origin);
    assert_eq!(
        state.effective_object_index[&r(4)].storage_kind,
        StorageKind::Replaced
    );
    assert_eq!(
        state.effective_object_index[&r(4)].evidence.level,
        EvidenceLevel::Untrusted
    );
}
