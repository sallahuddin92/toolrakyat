use super::{
    ContentHash, ContractRefusal, ContractResult, DocumentDelta, EvidenceLevel, EvidenceRecord,
    RefusalCode, StarNodeId,
};
use crate::document::{ObjectStore, PdfDocument};
use crate::syntax::object::ObjectRef;
use crate::xref::table::XrefEntry;
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
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct StarExecutionState {
    pub effective_object_index: BTreeMap<ObjectRef, ObjectState>,
    pub provenance: BTreeMap<StarNodeId, Vec<ProvenanceLink>>,
    pub current_revision: u64,
    pub pending_delta: Option<DocumentDelta>,
    pub evidence_records: BTreeMap<StarNodeId, EvidenceRecord>,
}

impl StarExecutionState {
    /// Read-only, lazy shadow snapshot. Effective xref presence is not a proof of editability.
    /// `current_revision` is the caller's document-state revision, not an occurrence identity.
    pub fn from_document(document: &PdfDocument<'_>, current_revision: u64) -> Self {
        Self::from_store(document.store(), current_revision)
    }

    pub fn from_store(store: &ObjectStore<'_>, current_revision: u64) -> Self {
        let effective_object_index = store
            .xref()
            .entries
            .iter()
            .filter_map(|(&number, entry)| {
                let (storage_kind, source_location) = match entry {
                    XrefEntry::Free { .. } => return None,
                    XrefEntry::InUse { byte_offset, .. } => (
                        StorageKind::Direct,
                        SourceLocation::FileOffset(*byte_offset),
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
                    ),
                };
                let object_ref = ObjectRef::new(number, entry.generation());
                let evidence = EvidenceRecord {
                    level: EvidenceLevel::Untrusted,
                    proof: "effective xref entry only; semantic verification pending".into(),
                    source_revision: None,
                    provenance: Some(source_location.clone()),
                };
                Some((
                    object_ref,
                    ObjectState {
                        object_ref,
                        storage_kind,
                        source_revision: None,
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
}
