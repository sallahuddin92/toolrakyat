use std::collections::BTreeMap;

use crate::syntax::object::{ObjectRef, PdfObject};

/// Precise appearance update status after a field mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppearanceStatus {
    /// Logical field value (/V) was updated; appearance state (/AS) was also updated.
    AppearanceStateUpdated,
    /// Logical field value was updated, but widget lacks distinct appearance states.
    LogicalOnlyUpdated,
    /// Appearance stream (/AP) was preserved as-is without regeneration.
    AppearanceStreamPreserved,
}

/// Result of evaluating a set of mutations on a PDF document.
#[derive(Debug, Clone, PartialEq)]
pub struct MutationPlan {
    pub modified_objects: BTreeMap<ObjectRef, PdfObject>,
    pub appearance_status: AppearanceStatus,
}
