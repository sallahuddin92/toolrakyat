use super::{ContractRefusal, ContractResult, RefusalCode, SourceLocation};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
pub enum EvidenceLevel {
    Verified,
    RecoveredVerified,
    Inferred,
    Untrusted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceRecord {
    pub level: EvidenceLevel,
    pub proof: String,
    pub source_revision: Option<u64>,
    pub provenance: Option<SourceLocation>,
}

/// The caller chooses accepted levels and required proof for this operation.
/// Enum ordering never implies permission or editability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidencePolicy {
    pub operation: String,
    pub accepted_levels: BTreeSet<EvidenceLevel>,
    pub required_revision: u64,
}

impl EvidencePolicy {
    pub fn check(&self, record: &EvidenceRecord) -> ContractResult<()> {
        if record.source_revision != Some(self.required_revision) {
            return Err(ContractRefusal::new(
                RefusalCode::StaleRevision,
                "evidence revision is missing or stale",
            ));
        }
        if !self.accepted_levels.contains(&record.level) || record.proof.trim().is_empty() {
            return Err(ContractRefusal::new(
                RefusalCode::InsufficientEvidence,
                format!("insufficient evidence for {}", self.operation),
            ));
        }
        Ok(())
    }
}
