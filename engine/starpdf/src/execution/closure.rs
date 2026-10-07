//! Explicit structural dependencies for the initially supported mutation families.
use super::{ClosureCompleteness, DocumentDelta, ResourceKey, StructuralClosure};
use crate::syntax::object::ObjectRef;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SupportedEdit {
    FreeTextStyle,
    FreeTextContent,
    NativeTextStyle,
    ImageDelete,
    ImageReplace,
}

#[derive(Debug, Clone, Default)]
pub struct DependencyGraph {
    /// An entry means dependencies were explicitly inspected, including an empty set.
    pub object_edges: BTreeMap<ObjectRef, BTreeSet<ObjectRef>>,
    pub resource_edges: BTreeMap<ObjectRef, BTreeSet<ResourceKey>>,
    pub inspected_for: BTreeMap<SupportedEdit, BTreeSet<ObjectRef>>,
}

impl DependencyGraph {
    pub fn inspect(
        &mut self,
        object: ObjectRef,
        objects: impl IntoIterator<Item = ObjectRef>,
        resources: impl IntoIterator<Item = ResourceKey>,
    ) {
        self.object_edges
            .insert(object, objects.into_iter().collect());
        self.resource_edges
            .insert(object, resources.into_iter().collect());
    }

    pub fn inspect_for(
        &mut self,
        operation: SupportedEdit,
        object: ObjectRef,
        objects: impl IntoIterator<Item = ObjectRef>,
        resources: impl IntoIterator<Item = ResourceKey>,
    ) {
        self.inspect(object, objects, resources);
        self.inspected_for
            .entry(operation)
            .or_default()
            .insert(object);
    }

    pub fn closure(&self, operation: SupportedEdit, delta: &DocumentDelta) -> StructuralClosure {
        let mut closure = StructuralClosure::default();
        let mut pending: Vec<_> = delta
            .added_objects
            .union(&delta.modified_objects)
            .copied()
            .collect();
        pending.extend(delta.removed_references.iter().map(|edge| edge.to));
        pending.extend(
            delta
                .resource_changes
                .values()
                .flat_map(|change| [change.before, change.after].into_iter().flatten()),
        );
        while let Some(object) = pending.pop() {
            if !closure.objects.insert(object) {
                continue;
            }
            match (
                self.object_edges.get(&object),
                self.resource_edges.get(&object),
            ) {
                (Some(edges), Some(resources)) => {
                    pending.extend(edges.iter().copied());
                    closure.resources.extend(resources.iter().cloned());
                }
                _ => closure.completeness = ClosureCompleteness::Incomplete,
            }
        }
        closure
            .resources
            .extend(delta.resource_changes.keys().cloned());
        closure
            .source_ranges
            .extend(delta.source_range_edits.keys().cloned());
        if closure.objects.iter().all(|object| {
            self.object_edges.contains_key(object)
                && self.resource_edges.contains_key(object)
                && self
                    .inspected_for
                    .get(&operation)
                    .is_some_and(|set| set.contains(object))
        }) {
            closure.completeness = ClosureCompleteness::Complete;
        }
        closure
    }
}
