use starpdf::execution::*;
use starpdf::syntax::object::ObjectRef;
use starpdf::xref::table::{XrefEntry, XrefTable};
use starpdf::{ByteSource, MinimalWriter, ObjectStore, PdfDocument};
use std::collections::{BTreeMap, BTreeSet};

fn node(path: &str) -> StarNodeId {
    StarNodeId::new("document-a", ObjectRef::new(7, 0), path).unwrap()
}
fn hash(value: u8) -> ContentHash {
    ContentHash([value; 32])
}
fn vectors() -> serde_json::Value {
    serde_json::from_str(include_str!("contracts_vectors.json")).unwrap()
}
fn text_vector() -> SemanticHashInput {
    SemanticHashInput {
        domain: HashDomain::TextRun,
        fields: BTreeMap::from([
            ("font".into(), b"font-a".to_vec()),
            ("text".into(), "你好".as_bytes().to_vec()),
        ]),
    }
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn stable_occurrence_identity_and_document_namespace() {
    let first = node("page:0/form:2/paint:0");
    assert_ne!(first, node("page:0/form:2/paint:1"));
    assert_ne!(
        first,
        StarNodeId::new("document-b", ObjectRef::new(7, 0), "page:0/form:2/paint:0").unwrap()
    );
    assert_ne!(
        first,
        StarNodeId::new("document-a", ObjectRef::new(7, 1), "page:0/form:2/paint:0").unwrap()
    );
    assert_eq!(
        first.parts(),
        (
            "document-a".into(),
            ObjectRef::new(7, 0),
            "page:0/form:2/paint:0".into()
        )
    );
}

#[test]
fn shared_identity_vectors_preserve_unicode_and_u64_max() {
    let vector = vectors();
    let number = vector["object_number"].as_str().unwrap().parse().unwrap();
    let id = StarNodeId::new(
        vector["document"].as_str().unwrap(),
        ObjectRef::new(number, 65535),
        vector["occurrence"].as_str().unwrap(),
    )
    .unwrap();
    assert_eq!(id.as_str(), vector["star_node"].as_str().unwrap());
    assert_eq!(id.as_str().parse::<StarNodeId>().unwrap(), id);
    assert_eq!(id.parts().1.number, u64::MAX);
    let render = RenderNodeId::new(&id, "glyph:0").unwrap();
    assert_eq!(render.as_str(), vector["render_node"].as_str().unwrap());
    assert_eq!(
        render.as_str().parse::<RenderNodeId>().unwrap().star_node(),
        id
    );
    assert_ne!(render, RenderNodeId::new(&id, "glyph:1").unwrap());
}

#[test]
fn rejects_noncanonical_or_ambiguous_identity() {
    for bad in [
        "sn1/64/01/0/70",
        "sn1/64/1/+0/70",
        "sn1/FF/1/0/70",
        "sn1/ff/1/0/70",
        "sn1/64/18446744073709551616/0/70",
        "sn1/64/0/0/70",
        "sn1//1/0/70",
        "sn1/64/1/65536/70",
        "sn1/64/1/0/7",
        "sn1/64/1/0/70/extra",
    ] {
        assert_eq!(
            bad.parse::<StarNodeId>().unwrap_err().code,
            RefusalCode::InvalidIdentity
        );
    }
    assert!("rn1/ff/70".parse::<RenderNodeId>().is_err());
}

#[cfg(feature = "wasm")]
#[test]
fn serde_identity_wire_is_an_exact_string_and_round_trips() {
    let id = node("page:0/paint:3");
    let json = serde_json::to_string(&id).unwrap();
    assert_eq!(serde_json::from_str::<String>(&json).unwrap(), id.as_str());
    assert_eq!(serde_json::from_str::<StarNodeId>(&json).unwrap(), id);
    let render = RenderNodeId::new(&id, "glyph:0").unwrap();
    assert_eq!(
        serde_json::from_str::<RenderNodeId>(&serde_json::to_string(&render).unwrap()).unwrap(),
        render
    );
    assert!(serde_json::from_str::<StarNodeId>("\"sn1/64/01/0/70\"").is_err());
}

#[test]
fn evidence_policy_is_operation_specific_and_revision_bound() {
    let mut policy = EvidencePolicy {
        operation: "replace text".into(),
        accepted_levels: BTreeSet::from([EvidenceLevel::Verified]),
        required_revision: 2,
    };
    let mut evidence = EvidenceRecord {
        level: EvidenceLevel::RecoveredVerified,
        proof: "local structure validated".into(),
        source_revision: Some(2),
        provenance: None,
    };
    assert_eq!(
        policy.check(&evidence).unwrap_err().code,
        RefusalCode::InsufficientEvidence
    );
    policy
        .accepted_levels
        .insert(EvidenceLevel::RecoveredVerified);
    policy.check(&evidence).unwrap();
    evidence.source_revision = Some(1);
    assert_eq!(
        policy.check(&evidence).unwrap_err().code,
        RefusalCode::StaleRevision
    );
    evidence.source_revision = Some(2);
    for level in [EvidenceLevel::Inferred, EvidenceLevel::Untrusted] {
        evidence.level = level;
        assert!(policy.check(&evidence).is_err());
    }
    evidence.level = EvidenceLevel::Verified;
    evidence.proof = " ".into();
    assert!(policy.check(&evidence).is_err());
}

fn state(revision: Option<u64>, value: u8) -> ObjectState {
    ObjectState {
        object_ref: ObjectRef::new(7, 0),
        storage_kind: StorageKind::Replaced,
        source_revision: revision,
        source_location: SourceLocation::Generated,
        effective_value: EffectiveValueReference::ShadowValue(hash(value)),
        evidence: EvidenceRecord {
            level: EvidenceLevel::Verified,
            proof: "checked".into(),
            source_revision: revision,
            provenance: None,
        },
        content_hash: Some(hash(value)),
    }
}

#[test]
fn object_state_precedence_refuses_ambiguous_provenance() {
    assert!(state(Some(2), 2).supersedes(&state(Some(1), 1)).unwrap());
    assert!(!state(Some(1), 1).supersedes(&state(Some(2), 2)).unwrap());
    assert!(!state(Some(2), 2).supersedes(&state(Some(2), 2)).unwrap());
    assert_eq!(
        state(Some(2), 3)
            .supersedes(&state(Some(2), 2))
            .unwrap_err()
            .code,
        RefusalCode::AmbiguousObjectState
    );
    assert!(state(None, 2).supersedes(&state(Some(1), 1)).is_err());
    let mut other = state(Some(3), 3);
    other.object_ref = ObjectRef::new(8, 0);
    assert!(other.supersedes(&state(Some(1), 1)).is_err());
}

#[test]
fn deterministic_delta_order_is_independent_of_insertion() {
    let a = ObjectRef::new(4, 0);
    let b = ObjectRef::new(7, 0);
    let first = DocumentDelta {
        modified_objects: BTreeSet::from([a, b]),
        before_hashes: BTreeMap::from([(a, hash(1)), (b, hash(2))]),
        ..DocumentDelta::default()
    };
    let second = DocumentDelta {
        modified_objects: BTreeSet::from([b, a]),
        before_hashes: BTreeMap::from([(b, hash(2)), (a, hash(1))]),
        ..DocumentDelta::default()
    };
    assert_eq!(first, second);
    assert_eq!(format!("{first:?}"), format!("{second:?}"));
}

fn comparison() -> (
    MutationEnvelope,
    DocumentDelta,
    StructuralClosure,
    VisualClosure,
) {
    let object = ObjectRef::new(7, 0);
    let id = node("page:0/paint:0");
    let bounds = PageBounds {
        page_index: 0,
        rect: [10.0, 20.0, 100.0, 60.0],
    };
    let text = ValueChange {
        before: "old".into(),
        after: "new".into(),
    };
    let envelope = MutationEnvelope {
        target_nodes: BTreeSet::from([id.clone()]),
        allowed_pdf_objects: BTreeSet::from([object]),
        allowed_bounds: vec![bounds],
        expected_text: BTreeMap::from([(id.clone(), text.clone())]),
        ..MutationEnvelope::default()
    };
    let delta = DocumentDelta {
        modified_objects: BTreeSet::from([object]),
        before_hashes: BTreeMap::from([(object, hash(1))]),
        after_hashes: BTreeMap::from([(object, hash(2))]),
        semantic_changes: BTreeMap::from([(
            id.clone(),
            SemanticChange {
                text: Some(text),
                ..SemanticChange::default()
            },
        )]),
        affected_pages: BTreeSet::from([0]),
        ..DocumentDelta::default()
    };
    let structural = StructuralClosure {
        objects: BTreeSet::from([object]),
        completeness: ClosureCompleteness::Complete,
        ..StructuralClosure::default()
    };
    let visual = VisualClosure {
        bounds: BTreeMap::from([(RenderNodeId::new(&id, "text").unwrap(), bounds)]),
        completeness: ClosureCompleteness::Complete,
    };
    (envelope, delta, structural, visual)
}

#[test]
fn certificate_explains_exact_declared_changes() {
    let (envelope, delta, structural, visual) = comparison();
    let certificate = envelope.compare(&delta, &structural, &visual);
    assert_eq!(
        certificate.classification,
        CertificateClassification::Explained
    );
    assert!(certificate.unexplained_changes.is_empty());
    certificate.require_explained().unwrap();
}

#[test]
fn certificate_refuses_unexplained_object_and_text_changes() {
    let (envelope, mut delta, mut structural, visual) = comparison();
    let unexpected = ObjectRef::new(8, 0);
    delta.added_objects.insert(unexpected);
    delta.after_hashes.insert(unexpected, hash(3));
    structural.objects.insert(unexpected);
    let certificate = envelope.compare(&delta, &structural, &visual);
    assert_eq!(
        certificate.classification,
        CertificateClassification::Unexplained
    );
    assert_eq!(
        certificate.require_explained().unwrap_err().code,
        RefusalCode::UnexplainedChange
    );
    delta
        .semantic_changes
        .values_mut()
        .next()
        .unwrap()
        .text
        .as_mut()
        .unwrap()
        .after = "wrong".into();
    assert!(envelope
        .compare(&delta, &structural, &visual)
        .unexplained_changes
        .contains("expected-text"));
}

#[test]
fn incomplete_or_forbidden_closure_refuses_certification() {
    let (mut envelope, delta, mut structural, visual) = comparison();
    structural.completeness = ClosureCompleteness::Incomplete;
    let certificate = envelope.compare(&delta, &structural, &visual);
    assert_eq!(
        certificate.classification,
        CertificateClassification::Refused
    );
    assert_eq!(
        certificate.require_explained().unwrap_err().code,
        RefusalCode::IncompleteClosure
    );
    structural.completeness = ClosureCompleteness::Complete;
    envelope.forbidden_dependencies.insert(ObjectRef::new(7, 0));
    assert_eq!(
        envelope
            .compare(&delta, &structural, &visual)
            .require_explained()
            .unwrap_err()
            .code,
        RefusalCode::ForbiddenDependency
    );
}

#[test]
fn missing_hashes_or_inconsistent_pages_are_refused() {
    let (envelope, mut delta, structural, mut visual) = comparison();
    delta.before_hashes.clear();
    assert_eq!(
        envelope
            .compare(&delta, &structural, &visual)
            .require_explained()
            .unwrap_err()
            .code,
        RefusalCode::InvalidDelta
    );
    delta.before_hashes.insert(ObjectRef::new(7, 0), hash(1));
    visual.bounds.values_mut().next().unwrap().page_index = 1;
    assert!(envelope
        .compare(&delta, &structural, &visual)
        .require_explained()
        .is_err());
}

#[test]
fn source_ranges_require_matching_owner_revision_space_and_containment() {
    let (mut envelope, mut delta, mut structural, visual) = comparison();
    let allowed = SourceRange {
        owner: ObjectRef::new(7, 0),
        revision: 2,
        space: SourceSpace::DecodedStream,
        start: 10,
        end: 50,
    };
    let mut edited = allowed.clone();
    edited.start = 20;
    edited.end = 30;
    envelope.allowed_source_ranges.insert(allowed.clone());
    structural.source_ranges.insert(allowed.clone());
    delta.source_range_edits.insert(
        edited.clone(),
        ValueChange {
            before: hash(1),
            after: hash(2),
        },
    );
    envelope
        .compare(&delta, &structural, &visual)
        .require_explained()
        .unwrap();
    envelope.allowed_source_ranges.clear();
    assert_eq!(
        envelope
            .compare(&delta, &structural, &visual)
            .classification,
        CertificateClassification::Unexplained
    );
    edited.revision = 1;
    assert!(!allowed.contains(&edited));
    edited.revision = 2;
    edited.space = SourceSpace::FileBytes;
    assert!(!allowed.contains(&edited));
    edited.space = SourceSpace::DecodedStream;
    edited.start = 51;
    edited.end = 51;
    assert!(!allowed.contains(&edited));
}

#[test]
fn resources_removed_references_and_unknown_semantics_cannot_escape_allowlists() {
    let (envelope, mut delta, mut structural, visual) = comparison();
    let resource = ResourceKey {
        owner: ObjectRef::new(7, 0),
        category: "Font".into(),
        name: "F1".into(),
    };
    structural.resources.insert(resource.clone());
    delta.resource_changes.insert(
        resource,
        ValueChange {
            before: None,
            after: Some(ObjectRef::new(7, 0)),
        },
    );
    delta.removed_references.insert(ReferenceEdge {
        from: ObjectRef::new(7, 0),
        to: ObjectRef::new(8, 0),
    });
    structural.objects.insert(ObjectRef::new(8, 0));
    delta
        .semantic_changes
        .values_mut()
        .next()
        .unwrap()
        .other
        .insert("opacity".into());
    let certificate = envelope.compare(&delta, &structural, &visual);
    assert_eq!(
        certificate.classification,
        CertificateClassification::Unexplained
    );
    assert!(certificate
        .unexplained_changes
        .iter()
        .any(|s| s.starts_with("reference:")));
    assert!(certificate
        .unexplained_changes
        .iter()
        .any(|s| s.starts_with("resource:")));
    assert!(certificate
        .unexplained_changes
        .iter()
        .any(|s| s.starts_with("semantic:")));
}

#[test]
fn unexpected_visual_changes_are_unexplained() {
    let (envelope, delta, structural, mut visual) = comparison();
    visual.bounds.values_mut().next().unwrap().rect[2] = 101.0;
    assert_eq!(
        envelope
            .compare(&delta, &structural, &visual)
            .classification,
        CertificateClassification::Unexplained
    );
}

#[test]
fn semantic_hash_matches_shared_golden_vector_and_domain_separates() {
    let input = text_vector();
    let vector = vectors();
    assert_eq!(
        hex(&input.canonical_bytes().unwrap()),
        vector["canonical_hash_input"].as_str().unwrap()
    );
    assert_eq!(
        input.content_hash().unwrap().to_hex(),
        vector["semantic_hash"].as_str().unwrap()
    );
    let mut reordered = input.clone();
    reordered.fields = input
        .fields
        .iter()
        .rev()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    assert_eq!(
        input.content_hash().unwrap(),
        reordered.content_hash().unwrap()
    );
    for domain in [
        HashDomain::Font,
        HashDomain::Image,
        HashDomain::Annotation,
        HashDomain::PdfObject,
    ] {
        reordered.domain = domain;
        assert_ne!(
            input.content_hash().unwrap(),
            reordered.content_hash().unwrap()
        );
    }
    reordered.domain = input.domain;
    reordered
        .fields
        .insert("text".into(), b"different".to_vec());
    assert_ne!(
        input.content_hash().unwrap(),
        reordered.content_hash().unwrap()
    );
    assert_eq!(
        semantic_number_bytes(-0.0).unwrap(),
        semantic_number_bytes(0.0).unwrap()
    );
    assert!(semantic_number_bytes(f64::NAN).is_err());
    reordered.fields.insert("字体".into(), vec![]);
    assert!(reordered.content_hash().is_err());
}

#[test]
fn length_prefixes_prevent_field_concatenation_ambiguity() {
    let a = SemanticHashInput {
        domain: HashDomain::Font,
        fields: BTreeMap::from([("a".into(), b"bc".to_vec())]),
    };
    let b = SemanticHashInput {
        domain: HashDomain::Font,
        fields: BTreeMap::from([("ab".into(), b"c".to_vec())]),
    };
    assert_ne!(a.content_hash().unwrap(), b.content_hash().unwrap());
}

#[test]
fn coordinates_have_known_rotation_anchors_and_exact_inverse() {
    for (rotation, expected) in [
        (PageRotation::R0, [20.0, 320.0]),
        (PageRotation::R90, [80.0, 20.0]),
        (PageRotation::R180, [180.0, 80.0]),
        (PageRotation::R270, [320.0, 180.0]),
    ] {
        let transform = CoordinateTransform::new(
            [0.0, 0.0, 150.0, 250.0],
            Some([10.0, 20.0, 110.0, 220.0]),
            rotation,
            1.0,
            2.0,
            3.0,
        )
        .unwrap();
        assert_eq!(transform.pdf_to_viewport([20.0, 60.0]).unwrap(), expected);
        assert_eq!(transform.viewport_to_pdf(expected).unwrap(), [20.0, 60.0]);
        for point in [[-20.0, -30.0], [10.0, 20.0], [110.0, 220.0], [40.5, 72.25]] {
            assert_eq!(
                transform
                    .viewport_to_pdf(transform.pdf_to_viewport(point).unwrap())
                    .unwrap(),
                point
            );
        }
        let device = transform.viewport_to_device(expected).unwrap();
        assert_eq!(device, expected.map(|n| n * 3.0));
        assert_eq!(transform.device_to_viewport(device).unwrap(), expected);
    }
}

#[test]
fn coordinates_intersect_crop_media_and_preserve_unclipped_geometry() {
    let transform = CoordinateTransform::new(
        [0.0, 0.0, 100.0, 100.0],
        Some([-10.0, 20.0, 200.0, 120.0]),
        PageRotation::R90,
        1.0,
        1.0,
        1.0,
    )
    .unwrap();
    assert_eq!(transform.pdf_to_viewport([0.0, 20.0]).unwrap(), [0.0, 0.0]);
    assert_eq!(
        transform
            .pdf_box_to_viewport([10.0, 30.0, 40.0, 50.0])
            .unwrap(),
        [10.0, 10.0, 30.0, 40.0]
    );
    assert!(CoordinateTransform::new(
        [0.0, 0.0, 100.0, 100.0],
        Some([110.0, 0.0, 120.0, 100.0]),
        PageRotation::R0,
        1.0,
        1.0,
        1.0
    )
    .is_err());
    assert!(CoordinateTransform::new(
        [0.0, 0.0, 100.0, 100.0],
        None,
        PageRotation::R0,
        0.0,
        1.0,
        1.0
    )
    .is_err());
    assert!(transform.pdf_to_viewport([f64::INFINITY, 0.0]).is_err());
}

#[test]
fn shadow_construction_does_not_resolve_or_change_production_document() {
    let bytes = MinimalWriter::create_minimal_pdf("shadow contract").unwrap();
    let document = PdfDocument::from_bytes(&bytes).unwrap();
    let metrics = document.store().metrics();
    let xref = document.store().xref().clone();
    let shadow = StarExecutionState::from_document(&document, 9);
    assert_eq!(shadow.current_revision, 9);
    assert_eq!(document.store().metrics(), metrics);
    assert_eq!(document.store().xref(), &xref);
    assert_eq!(document.source().as_bytes(), bytes);
    assert!(!shadow.effective_object_index.is_empty());
    assert!(shadow.pending_delta.is_none());
    assert!(shadow.provenance.is_empty());
    assert!(shadow
        .effective_object_index
        .values()
        .all(|s| s.source_revision.is_none()
            && s.content_hash.is_none()
            && s.evidence.level == EvidenceLevel::Untrusted));
}

#[test]
fn shadow_uses_effective_xref_and_retains_compressed_locations() {
    let mut xref = XrefTable::new();
    xref.entries.insert(
        7,
        XrefEntry::InUse {
            byte_offset: 10,
            generation: 2,
        },
    );
    xref.entries.insert(
        8,
        XrefEntry::Compressed {
            stream_obj_num: 7,
            index_in_stream: 3,
        },
    );
    xref.entries.insert(
        9,
        XrefEntry::Free {
            next_free_obj: 0,
            generation: 1,
        },
    );
    let store = ObjectStore::new(ByteSource::new(b""), xref);
    let shadow = StarExecutionState::from_store(&store, 0);
    assert_eq!(shadow.effective_object_index.len(), 2);
    assert_eq!(
        shadow.effective_object_index[&ObjectRef::new(8, 0)].source_location,
        SourceLocation::ObjectStream {
            container: ObjectRef::new(7, 0),
            index: 3
        }
    );
    assert_eq!(
        shadow.effective_object_index[&ObjectRef::new(7, 2)].storage_kind,
        StorageKind::Direct
    );
}

#[derive(Default)]
struct MockRenderer {
    state: Option<RendererDocumentState>,
    invalidated: Vec<PageBounds>,
}
impl Renderer for MockRenderer {
    type Document = ();
    type Frame = ();
    fn open(&mut self, (): (), state: RendererDocumentState) -> ContractResult<()> {
        self.state = Some(state);
        Ok(())
    }
    fn current_state(&self) -> Option<&RendererDocumentState> {
        self.state.as_ref()
    }
    fn update(
        &mut self,
        state: RendererDocumentState,
        _delta: &DocumentDelta,
    ) -> ContractResult<()> {
        self.state = Some(state);
        Ok(())
    }
    fn render_viewport(&mut self, _viewport: &ViewportRequest) -> ContractResult<()> {
        Ok(())
    }
    fn hit_test(&self, _page_index: u32, _point: [f64; 2]) -> ContractResult<Vec<RenderHit>> {
        Ok(vec![])
    }
    fn invalidate(&mut self, bounds: &[PageBounds]) -> ContractResult<()> {
        self.invalidated.extend_from_slice(bounds);
        Ok(())
    }
}

#[test]
fn renderer_contract_supports_state_delta_and_bounds_without_production_renderer() {
    let mut renderer = MockRenderer::default();
    renderer
        .open(
            (),
            RendererDocumentState {
                document_id: "doc".into(),
                revision: 0,
            },
        )
        .unwrap();
    let viewport = ViewportRequest {
        page_index: 0,
        transform: CoordinateTransform::new(
            [0.0, 0.0, 100.0, 100.0],
            None,
            PageRotation::R0,
            1.0,
            1.0,
            1.0,
        )
        .unwrap(),
        viewport_bounds: [0.0, 0.0, 100.0, 100.0],
    };
    renderer.render_viewport(&viewport).unwrap();
    renderer
        .update(
            RendererDocumentState {
                document_id: "doc".into(),
                revision: 1,
            },
            &DocumentDelta::default(),
        )
        .unwrap();
    renderer
        .invalidate(&[PageBounds {
            page_index: 0,
            rect: [0.0, 0.0, 10.0, 10.0],
        }])
        .unwrap();
    assert_eq!(renderer.current_state().unwrap().revision, 1);
    assert_eq!(renderer.invalidated.len(), 1);
    assert!(renderer.hit_test(0, [1.0, 1.0]).unwrap().is_empty());
}

#[test]
fn geometry_requires_old_and_new_visual_coverage_and_exact_expectations() {
    let (mut envelope, mut delta, structural, mut visual) = comparison();
    let id = node("page:0/paint:0");
    let geometry = ValueChange {
        before: PageBounds {
            page_index: 0,
            rect: [10.0, 20.0, 50.0, 40.0],
        },
        after: PageBounds {
            page_index: 0,
            rect: [50.0, 20.0, 100.0, 60.0],
        },
    };
    envelope
        .expected_geometry_changes
        .insert(id.clone(), geometry.clone());
    delta.semantic_changes.get_mut(&id).unwrap().geometry = Some(geometry);
    envelope
        .compare(&delta, &structural, &visual)
        .require_explained()
        .unwrap();
    visual.bounds.values_mut().next().unwrap().rect = [10.0, 20.0, 50.0, 40.0];
    assert_eq!(
        envelope
            .compare(&delta, &structural, &visual)
            .require_explained()
            .unwrap_err()
            .code,
        RefusalCode::InvalidDelta
    );
    visual.bounds.values_mut().next().unwrap().rect = [10.0, 20.0, 100.0, 60.0];
    envelope.expected_geometry_changes.clear();
    assert_eq!(
        envelope
            .compare(&delta, &structural, &visual)
            .classification,
        CertificateClassification::Unexplained
    );
}

#[test]
fn font_changes_require_exact_hash_expectations() {
    let (mut envelope, mut delta, structural, visual) = comparison();
    let id = node("page:0/paint:0");
    let font = ValueChange {
        before: hash(4),
        after: hash(5),
    };
    envelope
        .expected_font_changes
        .insert(id.clone(), font.clone());
    delta.semantic_changes.get_mut(&id).unwrap().font = Some(font);
    envelope
        .compare(&delta, &structural, &visual)
        .require_explained()
        .unwrap();
    envelope.expected_font_changes.get_mut(&id).unwrap().after = hash(6);
    assert!(envelope
        .compare(&delta, &structural, &visual)
        .unexplained_changes
        .contains("expected-font"));
}

#[test]
fn omitted_resource_dependencies_or_modified_owners_are_refused() {
    let (envelope, mut delta, mut structural, visual) = comparison();
    let key = ResourceKey {
        owner: ObjectRef::new(7, 0),
        category: "Font".into(),
        name: "F2".into(),
    };
    delta.resource_changes.insert(
        key.clone(),
        ValueChange {
            before: None,
            after: Some(ObjectRef::new(99, 0)),
        },
    );
    structural.resources.insert(key);
    assert_eq!(
        envelope
            .compare(&delta, &structural, &visual)
            .require_explained()
            .unwrap_err()
            .code,
        RefusalCode::InvalidDelta
    );
    delta.resource_changes.clear();
    let range = SourceRange {
        owner: ObjectRef::new(99, 0),
        revision: 0,
        space: SourceSpace::DecodedStream,
        start: 0,
        end: 1,
    };
    structural.objects.insert(range.owner);
    structural.source_ranges.insert(range.clone());
    delta.source_range_edits.insert(
        range,
        ValueChange {
            before: hash(1),
            after: hash(2),
        },
    );
    assert_eq!(
        envelope
            .compare(&delta, &structural, &visual)
            .require_explained()
            .unwrap_err()
            .code,
        RefusalCode::InvalidDelta
    );
}

#[test]
fn inconsistent_explained_certificate_cannot_pass_the_gate() {
    let certificate = MutationCertificate {
        classification: CertificateClassification::Explained,
        unexplained_changes: BTreeSet::from(["unexpected".into()]),
        refusal: None,
    };
    assert_eq!(
        certificate.require_explained().unwrap_err().code,
        RefusalCode::InvalidDelta
    );
}

#[cfg(feature = "wasm")]
#[test]
fn evidence_and_certificate_enum_wire_names_match_typescript() {
    assert_eq!(
        serde_json::to_string(&EvidenceLevel::RecoveredVerified).unwrap(),
        "\"RECOVERED_VERIFIED\""
    );
    assert_eq!(
        serde_json::to_string(&CertificateClassification::Unexplained).unwrap(),
        "\"UNEXPLAINED\""
    );
}
