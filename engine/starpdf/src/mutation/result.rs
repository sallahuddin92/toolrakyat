use std::collections::BTreeMap;

use crate::appearance::status::AppearanceStatus;
use crate::syntax::object::{ObjectRef, PdfObject};

/// Result of evaluating an atomic set of mutations on a PDF document.
#[derive(Debug, Clone, PartialEq)]
pub struct MutationPlan {
    pub modified_objects: BTreeMap<ObjectRef, PdfObject>,
    pub appearance_status: AppearanceStatus,
}
