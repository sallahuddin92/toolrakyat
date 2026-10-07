use super::{
    ContentHash, ContractRefusal, ContractResult, PageBounds, RefusalCode, RenderNodeId, StarNodeId,
};
use crate::syntax::object::ObjectRef;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SourceSpace {
    FileBytes,
    DecodedStream,
}

/// Half-open [start,end); an insertion has start == end. Offsets belong to one revision/space.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceRange {
    pub owner: ObjectRef,
    pub revision: u64,
    pub space: SourceSpace,
    pub start: u64,
    pub end: u64,
}

impl SourceRange {
    pub fn contains(&self, other: &Self) -> bool {
        self.start <= self.end
            && other.start <= other.end
            && self.owner == other.owner
            && self.revision == other.revision
            && self.space == other.space
            && self.start <= other.start
            && self.end >= other.end
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReferenceEdge {
    pub from: ObjectRef,
    pub to: ObjectRef,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ResourceKey {
    pub owner: ObjectRef,
    pub category: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueChange<T> {
    pub before: T,
    pub after: T,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SemanticChange {
    pub text: Option<ValueChange<String>>,
    pub font: Option<ValueChange<ContentHash>>,
    pub geometry: Option<ValueChange<PageBounds>>,
    /// Future semantic kinds are explicit and cannot be silently certified by v1.
    pub other: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DocumentDelta {
    pub added_objects: BTreeSet<ObjectRef>,
    pub modified_objects: BTreeSet<ObjectRef>,
    pub removed_references: BTreeSet<ReferenceEdge>,
    pub source_range_edits: BTreeMap<SourceRange, ValueChange<ContentHash>>,
    pub resource_changes: BTreeMap<ResourceKey, ValueChange<Option<ObjectRef>>>,
    pub semantic_changes: BTreeMap<StarNodeId, SemanticChange>,
    pub affected_pages: BTreeSet<u32>,
    pub before_hashes: BTreeMap<ObjectRef, ContentHash>,
    pub after_hashes: BTreeMap<ObjectRef, ContentHash>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ClosureCompleteness {
    Complete,
    #[default]
    Incomplete,
}

/// Cs(Δ): a supplied structural dependency closure, not a graph traversal implementation.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StructuralClosure {
    pub objects: BTreeSet<ObjectRef>,
    pub resources: BTreeSet<ResourceKey>,
    pub source_ranges: BTreeSet<SourceRange>,
    pub completeness: ClosureCompleteness,
}

/// Cv(Δ): supplied affected visual bounds, including old and new coverage.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VisualClosure {
    pub bounds: BTreeMap<RenderNodeId, PageBounds>,
    pub completeness: ClosureCompleteness,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MutationEnvelope {
    pub target_nodes: BTreeSet<StarNodeId>,
    pub allowed_pdf_objects: BTreeSet<ObjectRef>,
    pub allowed_resources: BTreeSet<ResourceKey>,
    pub allowed_source_ranges: BTreeSet<SourceRange>,
    pub allowed_bounds: Vec<PageBounds>,
    pub expected_text: BTreeMap<StarNodeId, ValueChange<String>>,
    pub expected_font_changes: BTreeMap<StarNodeId, ValueChange<ContentHash>>,
    pub expected_geometry_changes: BTreeMap<StarNodeId, ValueChange<PageBounds>>,
    pub forbidden_dependencies: BTreeSet<ObjectRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
pub enum CertificateClassification {
    Explained,
    Unexplained,
    Refused,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MutationCertificate {
    pub classification: CertificateClassification,
    pub unexplained_changes: BTreeSet<String>,
    pub refusal: Option<ContractRefusal>,
}

/// Audit details over the stable v1 certificate contract.
#[derive(Debug, Clone, PartialEq)]
pub struct CertificateReport {
    pub certificate: MutationCertificate,
    pub expected_changes: BTreeSet<String>,
    pub actual_changes: BTreeSet<String>,
    pub explained_changes: BTreeSet<String>,
    pub forbidden_dependency_touches: BTreeSet<ObjectRef>,
}

impl MutationCertificate {
    pub fn require_explained(&self) -> ContractResult<()> {
        match self.classification {
            CertificateClassification::Explained
                if self.unexplained_changes.is_empty() && self.refusal.is_none() =>
            {
                Ok(())
            }
            CertificateClassification::Explained => Err(ContractRefusal::new(
                RefusalCode::InvalidDelta,
                "inconsistent explained certificate",
            )),
            CertificateClassification::Unexplained => Err(ContractRefusal::new(
                RefusalCode::UnexplainedChange,
                "UNEXPLAINED CHANGE must equal zero",
            )),
            CertificateClassification::Refused => Err(self.refusal.clone().unwrap_or_else(|| {
                ContractRefusal::new(RefusalCode::InvalidDelta, "refused certificate")
            })),
        }
    }
}

impl MutationEnvelope {
    pub fn evaluate(
        &self,
        delta: &DocumentDelta,
        structural: &StructuralClosure,
        visual: &VisualClosure,
    ) -> CertificateReport {
        let certificate = self.compare(delta, structural, visual);
        let mut expected_changes = BTreeSet::new();
        let mut actual_changes = BTreeSet::new();
        for object in &self.allowed_pdf_objects {
            expected_changes.insert(format!("object:{object}"));
        }
        for node in self.expected_text.keys() {
            expected_changes.insert(format!("text:{node}"));
        }
        for node in self.expected_font_changes.keys() {
            expected_changes.insert(format!("font:{node}"));
        }
        for node in self.expected_geometry_changes.keys() {
            expected_changes.insert(format!("geometry:{node}"));
        }
        for object in delta.added_objects.union(&delta.modified_objects) {
            actual_changes.insert(format!("object:{object}"));
        }
        for edge in &delta.removed_references {
            actual_changes.insert(format!("reference:{}->{}", edge.from, edge.to));
        }
        for resource in delta.resource_changes.keys() {
            actual_changes.insert(format!("resource:{resource:?}"));
        }
        for range in delta.source_range_edits.keys() {
            actual_changes.insert(format!("source-range:{range:?}"));
        }
        for (node, change) in &delta.semantic_changes {
            if change.text.is_some() {
                actual_changes.insert(format!("text:{node}"));
            }
            if change.font.is_some() {
                actual_changes.insert(format!("font:{node}"));
            }
            if change.geometry.is_some() {
                actual_changes.insert(format!("geometry:{node}"));
            }
            for kind in &change.other {
                actual_changes.insert(format!("semantic:{node}:{kind}"));
            }
        }
        let explained_changes =
            if certificate.classification == CertificateClassification::Explained {
                actual_changes.clone()
            } else {
                actual_changes
                    .intersection(&expected_changes)
                    .filter(|item| !certificate.unexplained_changes.contains(*item))
                    .cloned()
                    .collect()
            };
        CertificateReport {
            certificate,
            expected_changes,
            actual_changes,
            explained_changes,
            forbidden_dependency_touches: self
                .forbidden_dependencies
                .intersection(&structural.objects)
                .copied()
                .collect(),
        }
    }
    /// Compares declared facts only. This does not verify actual exported bytes or raster output.
    pub fn compare(
        &self,
        delta: &DocumentDelta,
        structural: &StructuralClosure,
        visual: &VisualClosure,
    ) -> MutationCertificate {
        if let Err(refusal) = self.validate_comparison(delta, structural, visual) {
            return MutationCertificate {
                classification: CertificateClassification::Refused,
                unexplained_changes: BTreeSet::new(),
                refusal: Some(refusal),
            };
        }
        let mut unexplained = BTreeSet::new();
        for object in delta.added_objects.union(&delta.modified_objects) {
            if !self.allowed_pdf_objects.contains(object) {
                unexplained.insert(format!("object:{object}"));
            }
        }
        for edge in &delta.removed_references {
            if !self.allowed_pdf_objects.contains(&edge.from)
                || !self.allowed_pdf_objects.contains(&edge.to)
            {
                unexplained.insert(format!("reference:{}->{}", edge.from, edge.to));
            }
        }
        for resource in delta.resource_changes.keys() {
            if !self.allowed_resources.contains(resource)
                || !self.allowed_pdf_objects.contains(&resource.owner)
            {
                unexplained.insert(format!("resource:{resource:?}"));
            }
        }
        for range in delta.source_range_edits.keys() {
            if !self
                .allowed_source_ranges
                .iter()
                .any(|allowed| allowed.contains(range))
                || !self.allowed_pdf_objects.contains(&range.owner)
            {
                unexplained.insert(format!("source-range:{range:?}"));
            }
        }
        for (render, bounds) in &visual.bounds {
            if !self.target_nodes.contains(&render.star_node())
                || !self
                    .allowed_bounds
                    .iter()
                    .any(|allowed| allowed.contains(*bounds))
            {
                unexplained.insert(format!("visual:{render}"));
            }
        }
        let mut actual_text = BTreeMap::new();
        let mut actual_font = BTreeMap::new();
        let mut actual_geometry = BTreeMap::new();
        for (node, change) in &delta.semantic_changes {
            if !self.target_nodes.contains(node) {
                unexplained.insert(format!("target:{node}"));
            }
            if let Some(text) = &change.text {
                actual_text.insert(node.clone(), text.clone());
            }
            if let Some(font) = &change.font {
                actual_font.insert(node.clone(), font.clone());
            }
            if let Some(geometry) = &change.geometry {
                actual_geometry.insert(node.clone(), geometry.clone());
                for bounds in [geometry.before, geometry.after] {
                    if !self
                        .allowed_bounds
                        .iter()
                        .any(|allowed| allowed.contains(bounds))
                    {
                        unexplained.insert(format!("geometry-bounds:{node}"));
                    }
                }
            }
            for kind in &change.other {
                unexplained.insert(format!("semantic:{node}:{kind}"));
            }
        }
        if actual_text != self.expected_text {
            unexplained.insert("expected-text".into());
        }
        if actual_font != self.expected_font_changes {
            unexplained.insert("expected-font".into());
        }
        if actual_geometry != self.expected_geometry_changes {
            unexplained.insert("expected-geometry".into());
        }
        MutationCertificate {
            classification: if unexplained.is_empty() {
                CertificateClassification::Explained
            } else {
                CertificateClassification::Unexplained
            },
            unexplained_changes: unexplained,
            refusal: None,
        }
    }

    fn validate_comparison(
        &self,
        delta: &DocumentDelta,
        structural: &StructuralClosure,
        visual: &VisualClosure,
    ) -> ContractResult<()> {
        if structural.completeness != ClosureCompleteness::Complete
            || visual.completeness != ClosureCompleteness::Complete
        {
            return Err(ContractRefusal::new(
                RefusalCode::IncompleteClosure,
                "Cs(delta) and Cv(delta) must be complete",
            ));
        }
        if !self.forbidden_dependencies.is_disjoint(&structural.objects) {
            return Err(ContractRefusal::new(
                RefusalCode::ForbiddenDependency,
                "structural closure contains a forbidden dependency",
            ));
        }
        let changed: BTreeSet<_> = delta
            .added_objects
            .union(&delta.modified_objects)
            .copied()
            .collect();
        let before: BTreeSet<_> = delta.before_hashes.keys().copied().collect();
        let after: BTreeSet<_> = delta.after_hashes.keys().copied().collect();
        let invalid = || {
            ContractRefusal::new(
                RefusalCode::InvalidDelta,
                "inconsistent delta, envelope or closure facts",
            )
        };
        if !delta.added_objects.is_disjoint(&delta.modified_objects)
            || before != delta.modified_objects
            || after != changed
        {
            return Err(invalid());
        }
        if !changed.is_subset(&structural.objects)
            || delta.removed_references.iter().any(|e| {
                !changed.contains(&e.from)
                    || !structural.objects.contains(&e.from)
                    || !structural.objects.contains(&e.to)
            })
            || delta.resource_changes.iter().any(|(r, values)| {
                !changed.contains(&r.owner)
                    || !structural.resources.contains(r)
                    || !structural.objects.contains(&r.owner)
                    || [values.before, values.after]
                        .into_iter()
                        .flatten()
                        .any(|reference| !structural.objects.contains(&reference))
            })
            || delta.source_range_edits.keys().any(|r| {
                r.start > r.end
                    || !delta.modified_objects.contains(&r.owner)
                    || !structural.objects.contains(&r.owner)
                    || !structural.source_ranges.iter().any(|s| s.contains(r))
            })
        {
            return Err(invalid());
        }
        if self.allowed_source_ranges.iter().any(|r| r.start > r.end)
            || self.allowed_bounds.iter().any(|b| !b.is_valid())
            || self
                .expected_geometry_changes
                .values()
                .any(|g| !g.before.is_valid() || !g.after.is_valid())
            || self
                .expected_text
                .keys()
                .chain(self.expected_font_changes.keys())
                .chain(self.expected_geometry_changes.keys())
                .any(|n| !self.target_nodes.contains(n))
        {
            return Err(invalid());
        }
        let visual_pages: BTreeSet<_> = visual.bounds.values().map(|b| b.page_index).collect();
        if visual_pages != delta.affected_pages
            || visual.bounds.values().any(|b| !b.is_valid())
            || delta
                .semantic_changes
                .iter()
                .filter_map(|(node, c)| c.geometry.as_ref().map(|g| (node, g)))
                .any(|(node, g)| {
                    !g.before.is_valid()
                        || !g.after.is_valid()
                        || !delta.affected_pages.contains(&g.before.page_index)
                        || !delta.affected_pages.contains(&g.after.page_index)
                        || [g.before, g.after].iter().any(|bounds| {
                            !visual.bounds.iter().any(|(render, visual_bounds)| {
                                &render.star_node() == node && visual_bounds.contains(*bounds)
                            })
                        })
                })
        {
            return Err(invalid());
        }
        Ok(())
    }
}
