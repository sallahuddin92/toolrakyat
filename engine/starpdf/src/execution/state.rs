use super::{
    object_content_hash, ContentHash, ContractRefusal, ContractResult, DocumentDelta,
    EvidenceLevel, EvidencePolicy, EvidenceRecord, RefusalCode, SourceRange, StarNodeId,
};
use crate::document::{ObjectStore, PdfDocument};
use crate::syntax::object::ObjectRef;
use crate::xref::table::{XrefEntry, XrefStatus};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageKind {
    Direct,
    Compressed,
    Generated,
    Replaced,
    Recovered,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceLocation {
    FileOffset(u64),
    ObjectStream { container: ObjectRef, index: u32 },
    Generated,
    Unknown,
}

/// A store-owned object or shadow value identified by its hash; no duplicate PdfObject.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectiveValueReference {
    StoreObject(ObjectRef),
    ShadowValue(ContentHash),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectState {
    pub object_ref: ObjectRef,
    pub storage_kind: StorageKind,
    /// None when the existing effective xref does not retain per-object revision provenance.
    pub source_revision: Option<u64>,
    pub source_location: SourceLocation,
    pub effective_value: EffectiveValueReference,
    pub evidence: EvidenceRecord,
    /// None until an explicit semantic profile has been computed.
    pub content_hash: Option<ContentHash>,
}

impl ObjectState {
    pub fn supersedes(&self, previous: &Self) -> ContractResult<bool> {
        if self.object_ref != previous.object_ref {
            return Err(ContractRefusal::new(
                RefusalCode::AmbiguousObjectState,
                "object references differ",
            ));
        }
        if self == previous {
            return Ok(false);
        }
        match (self.source_revision, previous.source_revision) {
            (Some(new), Some(old)) if new != old => Ok(new > old),
            _ => Err(ContractRefusal::new(
                RefusalCode::AmbiguousObjectState,
                "conflicting states need known distinct revision provenance",
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenanceLink {
    pub object_ref: ObjectRef,
    pub source: SourceLocation,
    pub revision: Option<u64>,
    pub source_range: Option<SourceRange>,
    pub derivation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct StarExecutionState {
    pub effective_object_index: BTreeMap<ObjectRef, ObjectState>,
    /// Prior compact locations for shadow replacements; source bytes remain in the store.
    pub object_origins: BTreeMap<ObjectRef, Vec<ProvenanceLink>>,
    pub provenance: BTreeMap<StarNodeId, Vec<ProvenanceLink>>,
    pub current_revision: u64,
    pub pending_delta: Option<DocumentDelta>,
    pub evidence_records: BTreeMap<StarNodeId, EvidenceRecord>,
}

impl StarExecutionState {
    /// Read-only, lazy shadow snapshot. Effective xref presence is not a proof of editability.
    /// `current_revision` is the caller's document-state revision, not an occurrence identity.
    pub fn from_document(document: &PdfDocument<'_>, current_revision: u64) -> Self {
        let mut state = Self::from_store(document.store(), current_revision);
        let bytes = document.source();
        let revision_offsets = {
            let mut offsets: Vec<_> = document
                .store()
                .xref()
                .revisions
                .iter()
                .map(|r| r.xref_offset)
                .collect();
            offsets.sort_unstable();
            offsets.dedup();
            offsets
        };
        for entry in state.effective_object_index.values_mut() {
            let SourceLocation::FileOffset(offset) = &entry.source_location else {
                continue;
            };
            let Some(start) = usize::try_from(*offset).ok() else {
                continue;
            };
            let header = format!(
                "{} {} obj",
                entry.object_ref.number, entry.object_ref.generation
            );
            if bytes
                .get_slice(start, header.len())
                .is_ok_and(|slice| slice == header.as_bytes())
            {
                continue;
            }
            let lower = start.saturating_sub(64);
            let upper = start
                .saturating_add(64)
                .saturating_add(header.len())
                .min(bytes.len());
            let Some(actual) = bytes.get_slice_range(lower, upper).ok().and_then(|slice| {
                slice
                    .windows(header.len())
                    .position(|window| window == header.as_bytes())
                    .map(|relative| lower + relative)
            }) else {
                continue;
            };
            entry.storage_kind = StorageKind::Recovered;
            entry.source_location = SourceLocation::FileOffset(actual as u64);
            entry.source_revision = (revision_offsets.len() > 1).then(|| {
                revision_offsets.partition_point(|section| *section < actual as u64) as u64
            });
            entry.evidence.provenance = Some(entry.source_location.clone());
            entry.evidence.proof =
                "xref offset recovered by bounded object-header search; value unverified".into();
        }
        state
    }

    pub fn from_store(store: &ObjectStore<'_>, current_revision: u64) -> Self {
        // Xref sections are parsed newest first. Offsets mark the PDF's physical
        // incremental revisions even when a later section keeps an older object.
        let mut revision_offsets: Vec<_> = store
            .xref()
            .revisions
            .iter()
            .map(|r| r.xref_offset)
            .collect();
        revision_offsets.sort_unstable();
        revision_offsets.dedup();
        let recovered = store.xref().status != XrefStatus::Valid;
        let effective_object_index = store
            .xref()
            .entries
            .iter()
            .filter_map(|(&number, entry)| {
                let (mut storage_kind, source_location, object_offset) = match entry {
                    XrefEntry::Free { .. } => return None,
                    XrefEntry::InUse { byte_offset, .. } => (
                        StorageKind::Direct,
                        SourceLocation::FileOffset(*byte_offset),
                        Some(*byte_offset),
                    ),
                    XrefEntry::Compressed {
                        stream_obj_num,
                        index_in_stream,
                    } => (
                        StorageKind::Compressed,
                        SourceLocation::ObjectStream {
                            container: ObjectRef::new(*stream_obj_num, 0),
                            index: *index_in_stream,
                        },
                        store.xref().get_offset(*stream_obj_num),
                    ),
                };
                let source_revision = if revision_offsets.len() > 1 {
                    object_offset.map(|offset| {
                        revision_offsets.partition_point(|section| *section < offset) as u64
                    })
                } else {
                    None
                };
                if recovered {
                    storage_kind = StorageKind::Recovered;
                }
                let object_ref = ObjectRef::new(number, entry.generation());
                let evidence = EvidenceRecord {
                    level: EvidenceLevel::Untrusted,
                    proof: "effective xref entry only; semantic verification pending".into(),
                    source_revision: Some(current_revision),
                    provenance: Some(source_location.clone()),
                };
                Some((
                    object_ref,
                    ObjectState {
                        object_ref,
                        storage_kind,
                        source_revision,
                        source_location,
                        effective_value: EffectiveValueReference::StoreObject(object_ref),
                        evidence,
                        content_hash: None,
                    },
                ))
            })
            .collect();
        Self {
            effective_object_index,
            current_revision,
            ..Self::default()
        }
    }

    /// Realize only the requested object. The store owns the parsed value and cache.
    pub fn verify_object(
        &mut self,
        store: &mut ObjectStore<'_>,
        object: ObjectRef,
    ) -> ContractResult<ContentHash> {
        let state = self
            .effective_object_index
            .get_mut(&object)
            .ok_or_else(|| {
                ContractRefusal::new(RefusalCode::AmbiguousObjectState, "object is not effective")
            })?;
        let value = store.resolve(object).map_err(|error| {
            ContractRefusal::new(RefusalCode::InsufficientEvidence, error.to_string())
        })?;
        let hash = object_content_hash(value)?;
        state.content_hash = Some(hash);
        state.evidence = EvidenceRecord {
            level: if state.storage_kind == StorageKind::Recovered {
                EvidenceLevel::RecoveredVerified
            } else {
                EvidenceLevel::Verified
            },
            proof: format!("resolved and canonically hashed object {object}"),
            source_revision: Some(self.current_revision),
            provenance: Some(state.source_location.clone()),
        };
        Ok(hash)
    }

    /// Stage a proposed value without changing the production object store.
    /// A hash alone is not proof, so the replacement remains untrusted.
    pub fn stage_replacement(
        &mut self,
        object: ObjectRef,
        value_hash: ContentHash,
    ) -> ContractResult<()> {
        let prior = self
            .effective_object_index
            .get_mut(&object)
            .ok_or_else(|| {
                ContractRefusal::new(RefusalCode::AmbiguousObjectState, "object is not effective")
            })?;
        if prior.effective_value == EffectiveValueReference::ShadowValue(value_hash) {
            return Ok(());
        }
        self.object_origins
            .entry(object)
            .or_default()
            .push(ProvenanceLink {
                object_ref: object,
                source: prior.source_location.clone(),
                revision: prior.source_revision,
                source_range: None,
                derivation: Some("superseded by shadow value".into()),
            });
        prior.storage_kind = StorageKind::Replaced;
        prior.source_location = SourceLocation::Generated;
        prior.source_revision = Some(self.current_revision.saturating_add(1));
        prior.effective_value = EffectiveValueReference::ShadowValue(value_hash);
        prior.content_hash = Some(value_hash);
        prior.evidence = EvidenceRecord {
            level: EvidenceLevel::Untrusted,
            proof: "proposed shadow hash; exported bytes unverified".into(),
            source_revision: Some(self.current_revision),
            provenance: Some(SourceLocation::Generated),
        };
        Ok(())
    }

    pub fn link_occurrence(
        &mut self,
        node: StarNodeId,
        object: ObjectRef,
        source_range: Option<SourceRange>,
        derivation: Option<String>,
    ) -> ContractResult<()> {
        let state = self.effective_object_index.get(&object).ok_or_else(|| {
            ContractRefusal::new(
                RefusalCode::AmbiguousObjectState,
                "provenance object is not effective",
            )
        })?;
        self.provenance
            .entry(node)
            .or_default()
            .push(ProvenanceLink {
                object_ref: object,
                source: state.source_location.clone(),
                revision: state.source_revision,
                source_range,
                derivation,
            });
        Ok(())
    }

    pub fn record_occurrence_evidence(&mut self, node: StarNodeId, record: EvidenceRecord) {
        self.evidence_records.insert(node, record);
    }

    pub fn require_capability(
        &self,
        node: &StarNodeId,
        policy: &EvidencePolicy,
    ) -> ContractResult<()> {
        let record = self.evidence_records.get(node).ok_or_else(|| {
            ContractRefusal::new(RefusalCode::InsufficientEvidence, "occurrence has no proof")
        })?;
        policy.check(record)?;
        let object = node.parts().1;
        let state = self.effective_object_index.get(&object).ok_or_else(|| {
            ContractRefusal::new(
                RefusalCode::AmbiguousObjectState,
                "occurrence object is not effective",
            )
        })?;
        policy.check(&state.evidence)
    }
}
