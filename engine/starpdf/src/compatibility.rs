//! Bounded, read-only inspection of catalog compatibility structures.
use std::collections::{BTreeMap, BTreeSet};

use crate::document::ObjectStore;
use crate::error::{PdfError, PdfResult};
use crate::syntax::object::{ObjectRef, PdfObject};

const MAX_ITEMS: usize = 4096;
const MAX_DEPTH: usize = 64;
const MAX_ATTACHMENT_BYTES: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub struct OutlineItem {
    pub title: String,
    pub destination: Option<PdfObject>,
    pub action: Option<PdfObject>,
    pub children: Vec<OutlineItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedFile {
    pub name: String,
    pub file_spec: ObjectRef,
    pub stream_ref: ObjectRef,
    pub size: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptionalContentGroup {
    pub reference: ObjectRef,
    pub name: Option<String>,
    pub default_visible: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionalContentMembership {
    pub reference: ObjectRef,
    pub groups: Vec<ObjectRef>,
    pub policy: Option<String>,
    /// Unknown for unsupported visibility expressions or incomplete group state.
    pub default_visible: Option<bool>,
}

fn dictionary(
    store: &mut ObjectStore<'_>,
    object: &PdfObject,
) -> PdfResult<BTreeMap<String, PdfObject>> {
    store
        .resolve_object(object)?
        .as_dict()
        .cloned()
        .ok_or_else(|| PdfError::InvalidSyntax("Expected compatibility dictionary".into()))
}

pub fn outlines(
    store: &mut ObjectStore<'_>,
    catalog: &BTreeMap<String, PdfObject>,
) -> PdfResult<Vec<OutlineItem>> {
    let Some(root) = catalog.get("Outlines") else {
        return Ok(Vec::new());
    };
    let root = dictionary(store, root)?;
    let mut seen = BTreeSet::new();
    let mut count = 0;
    walk_outline(store, root.get("First"), &mut seen, &mut count, 0)
}

fn walk_outline(
    store: &mut ObjectStore<'_>,
    first: Option<&PdfObject>,
    seen: &mut BTreeSet<ObjectRef>,
    count: &mut usize,
    depth: usize,
) -> PdfResult<Vec<OutlineItem>> {
    if depth >= MAX_DEPTH {
        return Err(PdfError::InvalidOperation(
            "Outline depth limit exceeded".into(),
        ));
    }
    let mut result = Vec::new();
    let mut next = first.cloned();
    while let Some(object) = next {
        let reference = object
            .as_reference()
            .ok_or_else(|| PdfError::InvalidSyntax("Outline item must be indirect".into()))?;
        if !seen.insert(reference) {
            return Err(PdfError::CircularReference("Outline cycle".into()));
        }
        *count += 1;
        if *count > MAX_ITEMS {
            return Err(PdfError::InvalidOperation(
                "Outline item limit exceeded".into(),
            ));
        }
        let dict = dictionary(store, &object)?;
        let title = dict
            .get("Title")
            .and_then(PdfObject::as_string_lossy)
            .ok_or_else(|| PdfError::InvalidSyntax("Outline title missing".into()))?;
        let children = walk_outline(store, dict.get("First"), seen, count, depth + 1)?;
        result.push(OutlineItem {
            title,
            destination: dict.get("Dest").cloned(),
            action: dict.get("A").cloned(),
            children,
        });
        next = dict.get("Next").cloned();
    }
    Ok(result)
}

pub fn embedded_files(
    store: &mut ObjectStore<'_>,
    catalog: &BTreeMap<String, PdfObject>,
) -> PdfResult<Vec<EmbeddedFile>> {
    let Some(names) = catalog.get("Names") else {
        return Ok(Vec::new());
    };
    let names = dictionary(store, names)?;
    let Some(tree) = names.get("EmbeddedFiles") else {
        return Ok(Vec::new());
    };
    let mut seen = BTreeSet::new();
    let mut result = Vec::new();
    walk_names(store, tree, &mut seen, &mut result, 0)?;
    Ok(result)
}

fn walk_names(
    store: &mut ObjectStore<'_>,
    node: &PdfObject,
    seen: &mut BTreeSet<ObjectRef>,
    result: &mut Vec<EmbeddedFile>,
    depth: usize,
) -> PdfResult<()> {
    if depth >= MAX_DEPTH {
        return Err(PdfError::InvalidOperation(
            "Name tree depth limit exceeded".into(),
        ));
    }
    if let Some(reference) = node.as_reference() {
        if !seen.insert(reference) {
            return Err(PdfError::CircularReference(
                "Embedded file name tree cycle".into(),
            ));
        }
    }
    let dict = dictionary(store, node)?;
    if let Some(names) = dict.get("Names") {
        let values = store.resolve_object(names)?;
        let values = values
            .as_array()
            .ok_or_else(|| PdfError::InvalidSyntax("Embedded file names must be an array".into()))?
            .to_vec();
        if values.len() % 2 != 0 {
            return Err(PdfError::InvalidSyntax(
                "Unpaired embedded file name".into(),
            ));
        }
        for pair in values.chunks_exact(2) {
            if result.len() >= MAX_ITEMS {
                return Err(PdfError::InvalidOperation(
                    "Embedded file count limit exceeded".into(),
                ));
            }
            let name = pair[0]
                .as_string_lossy()
                .ok_or_else(|| PdfError::InvalidSyntax("Invalid embedded file name".into()))?;
            let file_spec = pair[1].as_reference().ok_or_else(|| {
                PdfError::InvalidSyntax("Embedded file specification must be indirect".into())
            })?;
            let spec = dictionary(store, &pair[1])?;
            let ef = spec
                .get("EF")
                .ok_or_else(|| PdfError::InvalidSyntax("Embedded file /EF missing".into()))?;
            let ef = dictionary(store, ef)?;
            let stream_ref = ef
                .get("UF")
                .or_else(|| ef.get("F"))
                .and_then(PdfObject::as_reference)
                .ok_or_else(|| {
                    PdfError::InvalidSyntax("Embedded file stream reference missing".into())
                })?;
            let stream = store.resolve(stream_ref)?.as_stream().ok_or_else(|| {
                PdfError::InvalidSyntax("Embedded file target is not a stream".into())
            })?;
            if stream.data.len() > MAX_ATTACHMENT_BYTES {
                return Err(PdfError::InvalidOperation(
                    "Embedded file size limit exceeded".into(),
                ));
            }
            result.push(EmbeddedFile {
                name,
                file_spec,
                stream_ref,
                size: stream.data.len(),
            });
        }
    }
    if let Some(kids) = dict.get("Kids") {
        let values = store
            .resolve_object(kids)?
            .as_array()
            .ok_or_else(|| PdfError::InvalidSyntax("Name tree kids must be an array".into()))?
            .to_vec();
        for kid in values {
            walk_names(store, &kid, seen, result, depth + 1)?;
        }
    }
    Ok(())
}

/// Only unfiltered bytes are returned. No attachment content is interpreted or executed.
pub fn extract_embedded_file(
    store: &mut ObjectStore<'_>,
    entry: &EmbeddedFile,
) -> PdfResult<Vec<u8>> {
    let stream = store
        .resolve(entry.stream_ref)?
        .as_stream()
        .ok_or_else(|| PdfError::InvalidSyntax("Embedded file target is not a stream".into()))?;
    if stream.dict.contains_key("Filter") || stream.dict.contains_key("DecodeParms") {
        return Err(PdfError::InvalidOperation(
            "Filtered embedded file extraction is unsupported".into(),
        ));
    }
    if stream.data.len() > MAX_ATTACHMENT_BYTES || stream.data.len() != entry.size {
        return Err(PdfError::InvalidOperation(
            "Embedded file size changed or exceeded limit".into(),
        ));
    }
    Ok(stream.data.clone())
}

pub fn optional_content_groups(
    store: &mut ObjectStore<'_>,
    catalog: &BTreeMap<String, PdfObject>,
) -> PdfResult<Vec<OptionalContentGroup>> {
    let Some(oc) = catalog.get("OCProperties") else {
        return Ok(Vec::new());
    };
    let oc = dictionary(store, oc)?;
    let groups = oc
        .get("OCGs")
        .ok_or_else(|| PdfError::InvalidSyntax("OCProperties missing OCGs".into()))?;
    let groups = store
        .resolve_object(groups)?
        .as_array()
        .ok_or_else(|| PdfError::InvalidSyntax("OCGs must be an array".into()))?
        .to_vec();
    if groups.len() > MAX_ITEMS {
        return Err(PdfError::InvalidOperation(
            "OCG count limit exceeded".into(),
        ));
    }
    let default = oc.get("D").map(|d| dictionary(store, d)).transpose()?;
    let base = default
        .as_ref()
        .and_then(|d| d.get("BaseState"))
        .and_then(PdfObject::as_name);
    let on = default
        .as_ref()
        .and_then(|d| d.get("ON"))
        .map(|v| store.resolve_object(v))
        .transpose()?;
    let off = default
        .as_ref()
        .and_then(|d| d.get("OFF"))
        .map(|v| store.resolve_object(v))
        .transpose()?;
    let on = on.as_ref().and_then(PdfObject::as_array);
    let off = off.as_ref().and_then(PdfObject::as_array);
    let mut seen = BTreeSet::new();
    groups
        .into_iter()
        .map(|object| {
            let reference = object
                .as_reference()
                .ok_or_else(|| PdfError::InvalidSyntax("OCG must be indirect".into()))?;
            if !seen.insert(reference) {
                return Err(PdfError::InvalidSyntax("Duplicate OCG".into()));
            }
            let dict = dictionary(store, &object)?;
            if dict.get("Type").and_then(PdfObject::as_name) != Some("OCG") {
                return Err(PdfError::InvalidSyntax("Invalid OCG type".into()));
            }
            let visible = if on.is_some_and(|list| list.contains(&object)) {
                Some(true)
            } else if off.is_some_and(|list| list.contains(&object)) {
                Some(false)
            } else {
                match base {
                    Some("ON") | None => Some(true),
                    Some("OFF") => Some(false),
                    _ => None,
                }
            };
            Ok(OptionalContentGroup {
                reference,
                name: dict.get("Name").and_then(PdfObject::as_string_lossy),
                default_visible: visible,
            })
        })
        .collect()
}

pub fn optional_content_memberships(
    store: &mut ObjectStore<'_>,
    groups: &[OptionalContentGroup],
) -> PdfResult<Vec<OptionalContentMembership>> {
    let refs: Vec<_> = store
        .xref()
        .entries
        .iter()
        .filter(|(_, entry)| entry.is_in_use())
        .map(|(number, entry)| ObjectRef::new(*number, entry.generation()))
        .collect();
    if refs.len() > crate::security::MAX_SECURITY_OBJECTS_INSPECTED {
        return Err(PdfError::InvalidOperation(
            "Optional content object limit exceeded".into(),
        ));
    }
    let mut result = Vec::new();
    for reference in refs {
        let object = store.resolve(reference)?.clone();
        let Some(dict) = object.as_dict() else {
            continue;
        };
        if dict.get("Type").and_then(PdfObject::as_name) != Some("OCMD") {
            continue;
        }
        if result.len() >= MAX_ITEMS {
            return Err(PdfError::InvalidOperation(
                "OCMD count limit exceeded".into(),
            ));
        }
        let members = match dict.get("OCGs") {
            Some(PdfObject::Reference(one)) => vec![*one],
            Some(PdfObject::Array(items)) => items
                .iter()
                .map(|item| {
                    item.as_reference().ok_or_else(|| {
                        PdfError::InvalidSyntax("OCMD group must be indirect".into())
                    })
                })
                .collect::<PdfResult<Vec<_>>>()?,
            _ => {
                return Err(PdfError::InvalidSyntax(
                    "OCMD OCGs missing or malformed".into(),
                ))
            }
        };
        let policy = dict
            .get("P")
            .and_then(PdfObject::as_name)
            .unwrap_or("AnyOn")
            .to_string();
        let states: Option<Vec<bool>> = members
            .iter()
            .map(|member| {
                groups
                    .iter()
                    .find(|group| group.reference == *member)
                    .and_then(|group| group.default_visible)
            })
            .collect();
        let default_visible = if dict.contains_key("VE") {
            None
        } else {
            states.and_then(|states| match policy.as_str() {
                "AllOn" => Some(states.iter().all(|value| *value)),
                "AnyOn" => Some(states.iter().any(|value| *value)),
                "AllOff" => Some(states.iter().all(|value| !*value)),
                "AnyOff" => Some(states.iter().any(|value| !*value)),
                _ => None,
            })
        };
        result.push(OptionalContentMembership {
            reference,
            groups: members,
            policy: Some(policy),
            default_visible,
        });
    }
    Ok(result)
}
