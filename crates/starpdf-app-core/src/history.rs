use serde::{Deserialize, Serialize};

/// A single snapshot entry in the document history.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryRecord {
    pub description: String,
    pub bytes: Vec<u8>,
}

/// Multi-level undo/redo stack managing authoritative document byte snapshots
/// and dirty baseline index tracking.
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryStack {
    snapshots: Vec<HistoryRecord>,
    current_index: usize,
    clean_baseline_index: Option<usize>,
    max_depth: usize,
    max_bytes: usize,
}

impl HistoryStack {
    pub const DEFAULT_MAX_DEPTH: usize = 50;
    /// Default maximum memory threshold for history snapshots (128 MB).
    pub const DEFAULT_MAX_BYTES: usize = 128 * 1024 * 1024;

    /// Creates a new `HistoryStack` initialized with the document's baseline byte snapshot.
    pub fn new(initial_bytes: Vec<u8>, max_depth: usize) -> Self {
        Self::with_byte_limit(initial_bytes, max_depth, Self::DEFAULT_MAX_BYTES)
    }

    /// Creates a new `HistoryStack` with explicit max depth and byte memory limit.
    pub fn with_byte_limit(initial_bytes: Vec<u8>, max_depth: usize, max_bytes: usize) -> Self {
        let max_depth = max_depth.max(1);
        let max_bytes = max_bytes.max(1024 * 1024); // at least 1MB
        Self {
            snapshots: vec![HistoryRecord {
                description: "Initial".to_string(),
                bytes: initial_bytes,
            }],
            current_index: 0,
            clean_baseline_index: Some(0),
            max_depth,
            max_bytes,
        }
    }

    /// Pushes a new mutated document state onto the history stack, discarding any redos.
    pub fn push(&mut self, description: impl Into<String>, bytes: Vec<u8>) {
        // Discard any redo states ahead of current_index
        if self.current_index + 1 < self.snapshots.len() {
            if let Some(clean) = self.clean_baseline_index {
                if clean > self.current_index {
                    self.clean_baseline_index = None;
                }
            }
            self.snapshots.truncate(self.current_index + 1);
        }

        self.snapshots.push(HistoryRecord {
            description: description.into(),
            bytes,
        });
        self.current_index += 1;

        // Enforce max depth
        if self.snapshots.len() > self.max_depth {
            let overflow = self.snapshots.len() - self.max_depth;
            self.snapshots.drain(0..overflow);
            self.current_index = self.current_index.saturating_sub(overflow);
            self.clean_baseline_index = self
                .clean_baseline_index
                .and_then(|clean| clean.checked_sub(overflow));
        }

        // Enforce max memory budget: drain oldest history snapshots if total memory exceeds budget
        while self.snapshots.len() > 1 && self.memory_usage() > self.max_bytes {
            self.snapshots.remove(0);
            self.current_index = self.current_index.saturating_sub(1);
            self.clean_baseline_index = self
                .clean_baseline_index
                .and_then(|clean| clean.checked_sub(1));
        }
    }

    /// Whether there are past states available to undo.
    pub fn can_undo(&self) -> bool {
        self.current_index > 0
    }

    /// Whether there are undone states available to redo.
    pub fn can_redo(&self) -> bool {
        self.current_index + 1 < self.snapshots.len()
    }

    /// Undoes to the previous document state, returning the restored bytes.
    pub fn undo(&mut self) -> Option<&[u8]> {
        if !self.can_undo() {
            return None;
        }
        self.current_index -= 1;
        Some(&self.snapshots[self.current_index].bytes)
    }

    /// Redoes to the next document state, returning the restored bytes.
    pub fn redo(&mut self) -> Option<&[u8]> {
        if !self.can_redo() {
            return None;
        }
        self.current_index += 1;
        Some(&self.snapshots[self.current_index].bytes)
    }

    /// Returns a slice to the current document bytes.
    pub fn current_bytes(&self) -> &[u8] {
        &self.snapshots[self.current_index].bytes
    }

    /// Returns true if the current state differs from the clean baseline state.
    pub fn is_dirty(&self) -> bool {
        self.clean_baseline_index != Some(self.current_index)
    }

    /// Marks the current history state as the saved clean baseline.
    pub fn mark_clean(&mut self) {
        self.clean_baseline_index = Some(self.current_index);
    }

    /// Returns the number of undoable operations available.
    pub fn undo_depth(&self) -> usize {
        self.current_index
    }

    /// Returns the number of redoable operations available.
    pub fn redo_depth(&self) -> usize {
        self.snapshots.len().saturating_sub(self.current_index + 1)
    }

    /// Description of the current state.
    pub fn current_description(&self) -> &str {
        &self.snapshots[self.current_index].description
    }

    /// Description of the action that would be undone by `undo()`.
    pub fn undo_description(&self) -> Option<&str> {
        if self.can_undo() {
            Some(&self.snapshots[self.current_index].description)
        } else {
            None
        }
    }

    /// Description of the action that would be redone by `redo()`.
    pub fn redo_description(&self) -> Option<&str> {
        if self.can_redo() {
            Some(&self.snapshots[self.current_index + 1].description)
        } else {
            None
        }
    }

    /// Current active index in the stack.
    pub fn current_index(&self) -> usize {
        self.current_index
    }

    /// Total count of stored snapshots.
    pub fn snapshot_count(&self) -> usize {
        self.snapshots.len()
    }

    /// Approximated memory consumption of all snapshots in bytes.
    pub fn memory_usage(&self) -> usize {
        self.snapshots
            .iter()
            .map(|s| s.bytes.len() + s.description.len())
            .sum()
    }

    /// Configured maximum memory threshold for history.
    pub fn max_bytes(&self) -> usize {
        self.max_bytes
    }

    /// Updates the maximum memory threshold.
    pub fn set_max_bytes(&mut self, max_bytes: usize) {
        self.max_bytes = max_bytes.max(1024 * 1024);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_history_memory_limit_eviction() {
        // Initialize with 1MB max_bytes budget
        let mut history = HistoryStack::with_byte_limit(vec![0u8; 400_000], 10, 1024 * 1024);
        assert_eq!(history.snapshot_count(), 1);
        assert!(!history.is_dirty());

        // Push second snapshot (400KB -> 800KB total, within 1MB)
        history.push("Second", vec![1u8; 400_000]);
        assert_eq!(history.snapshot_count(), 2);
        assert!(history.is_dirty());

        // Push third snapshot (400KB -> 1.2MB total, exceeds 1MB -> oldest evicted)
        history.push("Third", vec![2u8; 400_000]);
        assert!(history.memory_usage() <= 1024 * 1024);
        assert_eq!(history.snapshot_count(), 2);
        // Baseline 0 was evicted, so dirty remains true
        assert!(history.is_dirty());
    }

    #[test]
    fn test_history_undo_redo_and_truncation() {
        let mut history = HistoryStack::new(vec![1, 2, 3], 5);
        history.push("Step 1", vec![1, 2, 3, 4]);
        history.push("Step 2", vec![1, 2, 3, 4, 5]);

        assert_eq!(history.undo_depth(), 2);
        assert_eq!(history.redo_depth(), 0);

        let undone = history.undo().unwrap();
        assert_eq!(undone, &[1, 2, 3, 4]);
        assert_eq!(history.undo_depth(), 1);
        assert_eq!(history.redo_depth(), 1);

        // Branching: pushing new step discards redo
        history.push("Step 2-branched", vec![9, 9, 9]);
        assert_eq!(history.redo_depth(), 0);
        assert_eq!(history.current_bytes(), &[9, 9, 9]);
    }
}
