#[cfg(feature = "wasm")]
use std::collections::HashMap;
#[cfg(feature = "wasm")]
use std::sync::atomic::{AtomicU32, Ordering};
#[cfg(feature = "wasm")]
use std::sync::{LazyLock, Mutex};

#[cfg(feature = "wasm")]
use crate::appearance::AppearanceStatus;
#[cfg(feature = "wasm")]
use crate::document::PdfDocument;
#[cfg(feature = "wasm")]
use crate::error::{PdfError, PdfResult};
#[cfg(feature = "wasm")]
use crate::mutation::PdfChange;

#[cfg(feature = "wasm")]
const MAX_PENDING_CHANGES: usize = 500;

#[cfg(feature = "wasm")]
pub struct DocumentHandleEntry {
    pub raw_bytes: Vec<u8>,
    pub pending_changes: Vec<PdfChange>,
    pub last_appearance_status: AppearanceStatus,
}

#[cfg(feature = "wasm")]
pub struct DocumentRegistry {
    handles: Mutex<HashMap<u32, DocumentHandleEntry>>,
    next_id: AtomicU32,
    max_handles: usize,
}

#[cfg(feature = "wasm")]
impl DocumentRegistry {
    pub fn new(max_handles: usize) -> Self {
        Self {
            handles: Mutex::new(HashMap::new()),
            next_id: AtomicU32::new(1),
            max_handles,
        }
    }

    pub fn insert(&self, bytes: Vec<u8>) -> PdfResult<u32> {
        // Validate that it's a parseable PDF first
        let _ = PdfDocument::from_bytes(&bytes)?;

        let mut map = self
            .handles
            .lock()
            .map_err(|_| PdfError::InvalidOperation("DocumentRegistry lock poisoned".into()))?;

        if map.len() >= self.max_handles {
            return Err(PdfError::InvalidOperation(format!(
                "Maximum open document handles limit ({}) exceeded",
                self.max_handles
            )));
        }

        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        map.insert(
            id,
            DocumentHandleEntry {
                raw_bytes: bytes,
                pending_changes: Vec::new(),
                last_appearance_status: AppearanceStatus::AppearancePreserved,
            },
        );

        Ok(id)
    }

    pub fn with_doc<F, R>(&self, handle: u32, f: F) -> PdfResult<R>
    where
        F: FnOnce(&mut PdfDocument) -> PdfResult<R>,
    {
        let map = self
            .handles
            .lock()
            .map_err(|_| PdfError::InvalidOperation("DocumentRegistry lock poisoned".into()))?;

        let entry = map
            .get(&handle)
            .ok_or_else(|| PdfError::InvalidSyntax(format!("Invalid document handle {handle}")))?;

        let mut doc = PdfDocument::from_bytes(&entry.raw_bytes)?;
        f(&mut doc)
    }

    pub fn add_change(&self, handle: u32, change: PdfChange) -> PdfResult<()> {
        let mut map = self
            .handles
            .lock()
            .map_err(|_| PdfError::InvalidOperation("DocumentRegistry lock poisoned".into()))?;

        let entry = map
            .get_mut(&handle)
            .ok_or_else(|| PdfError::InvalidSyntax(format!("Invalid document handle {handle}")))?;

        if entry.pending_changes.len() >= MAX_PENDING_CHANGES {
            return Err(PdfError::InvalidOperation(format!(
                "Maximum pending mutation count ({MAX_PENDING_CHANGES}) exceeded"
            )));
        }
        entry.pending_changes.push(change);
        Ok(())
    }

    pub fn export_and_apply_changes(&self, handle: u32) -> PdfResult<Vec<u8>> {
        let mut map = self
            .handles
            .lock()
            .map_err(|_| PdfError::InvalidOperation("DocumentRegistry lock poisoned".into()))?;

        let entry = map
            .get_mut(&handle)
            .ok_or_else(|| PdfError::InvalidSyntax(format!("Invalid document handle {handle}")))?;

        let mut doc = PdfDocument::from_bytes(&entry.raw_bytes)?;
        let plan = doc.apply_mutation(&entry.pending_changes)?;
        let status = plan.appearance_status;
        let new_bytes = doc.export_incremental(&plan)?;

        // Update handle state so subsequent mutations build incrementally
        entry.raw_bytes.clone_from(&new_bytes);
        entry.pending_changes.clear();
        entry.last_appearance_status = status;

        Ok(new_bytes)
    }

    pub fn last_appearance_status(&self, handle: u32) -> PdfResult<AppearanceStatus> {
        let map = self
            .handles
            .lock()
            .map_err(|_| PdfError::InvalidOperation("DocumentRegistry lock poisoned".into()))?;
        let entry = map
            .get(&handle)
            .ok_or_else(|| PdfError::InvalidSyntax(format!("Invalid document handle {handle}")))?;
        Ok(entry.last_appearance_status)
    }

    pub fn close(&self, handle: u32) -> PdfResult<bool> {
        let mut map = self
            .handles
            .lock()
            .map_err(|_| PdfError::InvalidOperation("DocumentRegistry lock poisoned".into()))?;

        Ok(map.remove(&handle).is_some())
    }

    pub fn clear(&self) -> PdfResult<()> {
        let mut map = self
            .handles
            .lock()
            .map_err(|_| PdfError::InvalidOperation("DocumentRegistry lock poisoned".into()))?;

        map.clear();
        Ok(())
    }
}

#[cfg(feature = "wasm")]
pub static REGISTRY: LazyLock<DocumentRegistry> = LazyLock::new(|| DocumentRegistry::new(16));
