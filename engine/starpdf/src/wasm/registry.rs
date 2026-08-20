#[cfg(feature = "wasm")]
use std::collections::HashMap;
#[cfg(feature = "wasm")]
use std::sync::atomic::{AtomicU32, Ordering};
#[cfg(feature = "wasm")]
use std::sync::{LazyLock, Mutex};

#[cfg(feature = "wasm")]
use crate::document::PdfDocument;
#[cfg(feature = "wasm")]
use crate::error::{PdfError, PdfResult};

#[cfg(feature = "wasm")]
pub struct DocumentHandleEntry {
    pub raw_bytes: Vec<u8>,
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
        map.insert(id, DocumentHandleEntry { raw_bytes: bytes });

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
