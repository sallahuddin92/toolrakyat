use std::collections::{BTreeMap, BTreeSet};

use crate::document::object_store::ObjectStore;
use crate::error::{PdfError, PdfResult};
use crate::syntax::object::{ObjectRef, PdfObject};

pub struct PageTree;

pub const MAX_PAGE_TREE_DEPTH: usize = 32;
pub const MAX_DOCUMENT_PAGES: usize = 10_000;

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

    /// Validates the complete page tree and returns page references in logical order.
    /// Unlike the ordinary lookup fast path, this verifies every `/Count`, `/Kids`, node type,
    /// and parent relationship before page operations are planned.
    pub fn validate_and_collect(
        store: &mut ObjectStore<'_>,
        root_pages_ref: ObjectRef,
    ) -> PdfResult<Vec<ObjectRef>> {
        let mut pages = Vec::new();
        let mut visited = BTreeSet::new();
        let total =
            Self::collect_validated(store, root_pages_ref, None, 0, &mut visited, &mut pages)?;
        if total == 0 {
            return Err(PdfError::PageOperation(
                "document page tree contains no pages".into(),
            ));
        }
        Ok(pages)
    }

    fn collect_validated(
        store: &mut ObjectStore<'_>,
        node_ref: ObjectRef,
        expected_parent: Option<ObjectRef>,
        depth: usize,
        visited: &mut BTreeSet<ObjectRef>,
        pages: &mut Vec<ObjectRef>,
    ) -> PdfResult<usize> {
        if depth > MAX_PAGE_TREE_DEPTH {
            return Err(PdfError::RecursionLimitExceeded);
        }
        if !visited.insert(node_ref) {
            return Err(PdfError::CircularReference(format!(
                "cycle in page tree at {node_ref}"
            )));
        }
        let object = store.resolve(node_ref)?.clone();
        let dict = object.as_dict().ok_or_else(|| PdfError::TypeMismatch {
            expected: "page-tree dictionary",
            actual: object.type_name(),
        })?;
        let parent = dict.get("Parent").and_then(PdfObject::as_reference);
        if expected_parent.is_some() && parent != expected_parent {
            return Err(PdfError::PageOperation(format!(
                "page-tree node {node_ref} has an inconsistent /Parent"
            )));
        }

        match dict.get("Type").and_then(PdfObject::as_name) {
            Some("Page") => {
                if pages.len() >= MAX_DOCUMENT_PAGES {
                    return Err(PdfError::PageResourceLimit(format!(
                        "page count exceeds {MAX_DOCUMENT_PAGES}"
                    )));
                }
                pages.push(node_ref);
                Ok(1)
            }
            Some("Pages") => {
                let kids = dict
                    .get("Kids")
                    .and_then(PdfObject::as_array)
                    .ok_or_else(|| {
                        PdfError::PageOperation(format!(
                            "page-tree node {node_ref} is missing a /Kids array"
                        ))
                    })?;
                if kids.len() > MAX_DOCUMENT_PAGES {
                    return Err(PdfError::PageResourceLimit(format!(
                        "page-tree /Kids count exceeds {MAX_DOCUMENT_PAGES}"
                    )));
                }
                let mut actual = 0usize;
                for kid in kids {
                    let kid_ref = kid.as_reference().ok_or_else(|| {
                        PdfError::PageOperation(format!(
                            "page-tree node {node_ref} contains a direct or malformed child"
                        ))
                    })?;
                    actual = actual
                        .checked_add(Self::collect_validated(
                            store,
                            kid_ref,
                            Some(node_ref),
                            depth + 1,
                            visited,
                            pages,
                        )?)
                        .ok_or_else(|| {
                            PdfError::PageResourceLimit("page count arithmetic overflow".into())
                        })?;
                }
                let declared = dict
                    .get("Count")
                    .and_then(PdfObject::as_integer)
                    .and_then(|value| usize::try_from(value).ok())
                    .ok_or_else(|| {
                        PdfError::PageOperation(format!(
                            "page-tree node {node_ref} has an invalid /Count"
                        ))
                    })?;
                if declared != actual {
                    return Err(PdfError::PageOperation(format!(
                        "page-tree node {node_ref} declares /Count {declared}, resolved {actual}"
                    )));
                }
                Ok(actual)
            }
            other => Err(PdfError::PageOperation(format!(
                "object {node_ref} has invalid page-tree /Type {other:?}"
            ))),
        }
    }
}
