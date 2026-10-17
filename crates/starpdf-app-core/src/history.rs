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
}

impl HistoryStack {
    pub const DEFAULT_MAX_DEPTH: usize = 50;

    /// Creates a new `HistoryStack` initialized with the document's baseline byte snapshot.
    pub fn new(initial_bytes: Vec<u8>, max_depth: usize) -> Self {
        let max_depth = max_depth.max(1);
        Self {
            snapshots: vec![HistoryRecord {
                description: "Initial".to_string(),
                bytes: initial_bytes,
            }],
            current_index: 0,
            clean_baseline_index: Some(0),
            max_depth,
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
}
