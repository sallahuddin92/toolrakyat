use std::collections::BTreeMap;

use crate::document::object_store::ObjectStore;
use crate::error::{PdfError, PdfResult};
use crate::syntax::object::{ObjectRef, PdfObject};

pub struct PageTree;

impl PageTree {
    /// Counts total pages starting from the root /Pages node.
    pub fn count_pages(store: &mut ObjectStore<'_>, node_ref: ObjectRef) -> PdfResult<usize> {
        let node_obj = store.resolve(node_ref)?.clone();
        let dict = node_obj.as_dict().ok_or_else(|| PdfError::TypeMismatch {
            expected: "dictionary",
            actual: node_obj.type_name(),
        })?;

        // Fast path: use /Count if present and valid
        if let Some(count_val) = dict.get("Count").and_then(|v| v.as_i64()) {
            if count_val >= 0 {
                return Ok(count_val as usize);
            }
        }

        // Slow path: recursively count
        Self::recursive_count(store, node_ref, 0)
    }

    fn recursive_count(
        store: &mut ObjectStore<'_>,
        node_ref: ObjectRef,
        depth: usize,
    ) -> PdfResult<usize> {
        if depth > 32 {
            return Err(PdfError::RecursionLimitExceeded);
        }

        let node_obj = store.resolve(node_ref)?.clone();
        let dict = match node_obj.as_dict() {
            Some(d) => d,
            None => return Ok(0),
        };

        let node_type = dict.get("Type").and_then(|v| v.as_name());
        if node_type == Some("Page") {
            return Ok(1);
        }

        let kids = match dict.get("Kids").and_then(|v| v.as_array()) {
            Some(k) => k.to_vec(),
            None => return Ok(0),
        };

        let mut total = 0;
        for kid in kids {
            if let Some(kid_ref) = kid.as_reference() {
                total += Self::recursive_count(store, kid_ref, depth + 1)?;
            }
        }

        Ok(total)
    }

    /// Resolves a 0-indexed page reference by walking the /Kids tree.
    pub fn get_page_ref(
        store: &mut ObjectStore<'_>,
        node_ref: ObjectRef,
        target_index: usize,
    ) -> PdfResult<ObjectRef> {
        Self::find_page(store, node_ref, target_index, 0)
    }

    fn find_page(
        store: &mut ObjectStore<'_>,
        node_ref: ObjectRef,
        mut target_index: usize,
        depth: usize,
    ) -> PdfResult<ObjectRef> {
        if depth > 32 {
            return Err(PdfError::RecursionLimitExceeded);
        }

        let node_obj = store.resolve(node_ref)?.clone();
        let dict = node_obj.as_dict().ok_or_else(|| PdfError::TypeMismatch {
            expected: "dictionary",
            actual: node_obj.type_name(),
        })?;

        let node_type = dict.get("Type").and_then(|v| v.as_name());
        if node_type == Some("Page") {
            if target_index == 0 {
                return Ok(node_ref);
            } else {
                return Err(PdfError::PageNotFound(target_index));
            }
        }

        let kids = dict
            .get("Kids")
            .and_then(|v| v.as_array())
            .ok_or_else(|| PdfError::InvalidSyntax("Pages node missing /Kids array".into()))?
            .to_vec();

        for kid in kids {
            let kid_ref = match kid.as_reference() {
                Some(r) => r,
                None => continue,
            };

            let kid_obj = store.resolve(kid_ref)?.clone();
            let kid_dict = match kid_obj.as_dict() {
                Some(d) => d,
                None => continue,
            };

            let kid_type = kid_dict.get("Type").and_then(|v| v.as_name());

            if kid_type == Some("Page") {
                if target_index == 0 {
                    return Ok(kid_ref);
                }
                target_index -= 1;
            } else {
                // Nested Pages node
                let subtree_count = if let Some(c) = kid_dict.get("Count").and_then(|v| v.as_i64())
                {
                    if c >= 0 {
                        c as usize
                    } else {
                        Self::recursive_count(store, kid_ref, depth + 1)?
                    }
                } else {
                    Self::recursive_count(store, kid_ref, depth + 1)?
                };

                if target_index < subtree_count {
                    return Self::find_page(store, kid_ref, target_index, depth + 1);
                } else {
                    target_index -= subtree_count;
                }
            }
        }

        Err(PdfError::PageNotFound(target_index))
    }

    /// Loads the resolved dictionary of a 0-indexed page.
    pub fn get_page_dict(
        store: &mut ObjectStore<'_>,
        root_pages_ref: ObjectRef,
        page_index: usize,
    ) -> PdfResult<BTreeMap<String, PdfObject>> {
        let page_ref = Self::get_page_ref(store, root_pages_ref, page_index)?;
        let page_obj = store.resolve(page_ref)?.clone();
        page_obj
            .as_dict()
            .cloned()
            .ok_or_else(|| PdfError::TypeMismatch {
                expected: "dictionary",
                actual: page_obj.type_name(),
            })
    }
}
