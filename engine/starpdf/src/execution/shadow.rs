//! Compact shadow snapshots and deterministic delta generation.
use super::{
    ContentHash, ContractRefusal, ContractResult, DocumentDelta, RefusalCode, StarExecutionState,
};
use crate::syntax::object::ObjectRef;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ObjectSnapshot(pub BTreeMap<ObjectRef, ContentHash>);

impl ObjectSnapshot {
    /// Only verified or supplied semantic hashes are included. No PdfObject is copied.
    pub fn from_state(state: &StarExecutionState) -> Self {
        Self(
            state
                .effective_object_index
                .iter()
                .filter_map(|(key, value)| value.content_hash.map(|hash| (*key, hash)))
                .collect(),
        )
    }

    pub fn diff(&self, after: &Self) -> DocumentDelta {
        let mut delta = DocumentDelta::default();
        for (&object, &hash) in &after.0 {
            match self.0.get(&object) {
                None => {
                    delta.added_objects.insert(object);
                    delta.after_hashes.insert(object, hash);
                }
                Some(before) if *before != hash => {
                    delta.modified_objects.insert(object);
                    delta.before_hashes.insert(object, *before);
                    delta.after_hashes.insert(object, hash);
                }
                _ => {}
            }
        }
        delta
    }

    /// Deletion must be represented as a removed reference from its owner. The
    /// object may remain in the file and cannot be certified as globally removed.
    pub fn assert_no_disappeared_objects(&self, after: &Self) -> ContractResult<()> {
        let missing: BTreeSet<_> = self
            .0
            .keys()
            .filter(|key| !after.0.contains_key(key))
            .collect();
        if missing.is_empty() {
            Ok(())
        } else {
            Err(ContractRefusal::new(
                RefusalCode::InvalidDelta,
                "verified object disappeared from shadow snapshot",
            ))
        }
    }
}

impl StarExecutionState {
    pub fn stage_shadow_delta(
        &mut self,
        before: &ObjectSnapshot,
        after: &ObjectSnapshot,
    ) -> ContractResult<&DocumentDelta> {
        before.assert_no_disappeared_objects(after)?;
        self.pending_delta = Some(before.diff(after));
        Ok(self.pending_delta.as_ref().expect("just staged"))
    }
}
