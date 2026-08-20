use std::collections::{BTreeMap, BTreeSet};

use crate::document::{PageTree, PdfDocument};
use crate::error::{PdfError, PdfResult};
use crate::page_ops::inherited::materialize_page_inheritance;
use crate::page_ops::PageOperationLimits;
use crate::security::{EncryptionState, SignatureState};
use crate::syntax::{ObjectRef, PdfObject, StreamObject};
use crate::validate::StructuralValidator;
use crate::writer::CompleteWriter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BuildPage {
    document_index: usize,
    page_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct ImportKey {
    document_index: usize,
    source_ref: ObjectRef,
    scope: u32,
}

pub struct DocumentBuilder;

impl DocumentBuilder {
    pub fn duplicate_page(
        input: &[u8],
        page_index: usize,
        insert_at: usize,
        limits: &PageOperationLimits,
    ) -> PdfResult<Vec<u8>> {
        let mut document = PdfDocument::from_bytes(input)?;
        let root_pages_ref = document.root_pages_ref();
        let pages = PageTree::validate_and_collect(document.store_mut(), root_pages_ref)?;
        if page_index >= pages.len() {
            return Err(PdfError::PageNotFound(page_index));
        }
        if insert_at > pages.len() {
            return Err(PdfError::PageNotFound(insert_at));
        }
        let mut selection: Vec<BuildPage> = (0..pages.len())
            .map(|index| BuildPage {
                document_index: 0,
                page_index: index,
            })
            .collect();
        selection.insert(
            insert_at,
            BuildPage {
                document_index: 0,
                page_index,
            },
        );
        Self::build(&[input], &selection, limits)
    }

    pub fn extract_pages(
        input: &[u8],
        page_indices: &[usize],
        limits: &PageOperationLimits,
    ) -> PdfResult<Vec<u8>> {
        if page_indices.is_empty() {
            return Err(PdfError::PageOperation(
                "page extraction selection must not be empty".into(),
            ));
        }
        let selection = page_indices
            .iter()
            .copied()
            .map(|page_index| BuildPage {
                document_index: 0,
                page_index,
            })
            .collect::<Vec<_>>();
        Self::build(&[input], &selection, limits)
    }

    fn build(
        inputs: &[&[u8]],
        selection: &[BuildPage],
        limits: &PageOperationLimits,
    ) -> PdfResult<Vec<u8>> {
        if inputs.len() != 1 {
            return Err(PdfError::PageOperation(
                "v0.12A standalone builds accept exactly one source document".into(),
            ));
        }
        if selection.is_empty() || selection.len() > limits.max_selected_pages {
            return Err(PdfError::PageResourceLimit(format!(
                "selected page count must be between 1 and {}",
                limits.max_selected_pages
            )));
        }

        let mut documents = Vec::with_capacity(inputs.len());
        for input in inputs {
            documents.push(PdfDocument::from_bytes(input)?);
        }
        let mut source_pages = Vec::with_capacity(documents.len());
        let mut force_shared = BTreeSet::new();
        let mut source_has_forms = Vec::with_capacity(documents.len());
        let mut total_source_pages = 0usize;
        for (document_index, document) in documents.iter_mut().enumerate() {
            Self::ensure_allowed(document)?;
            let root_pages_ref = document.root_pages_ref();
            let pages = PageTree::validate_and_collect(document.store_mut(), root_pages_ref)?;
            total_source_pages = total_source_pages.checked_add(pages.len()).ok_or_else(|| {
                PdfError::PageResourceLimit("total source page count overflow".into())
            })?;
            if total_source_pages > limits.max_total_pages {
                return Err(PdfError::PageResourceLimit(format!(
                    "total source pages exceed {}",
                    limits.max_total_pages
                )));
            }
            let fields = document.form_fields()?;
            if fields.len() > limits.max_form_fields {
                return Err(PdfError::PageResourceLimit(format!(
                    "form field count exceeds {}",
                    limits.max_form_fields
                )));
            }
            for field in &fields {
                if field.object_ref.number != 0 {
                    force_shared.insert((document_index, field.object_ref));
                }
                if let Some(parent) = field.parent_ref {
                    force_shared.insert((document_index, parent));
                }
                for widget in &field.widgets {
                    if widget.object_ref.number != 0 {
                        force_shared.insert((document_index, widget.object_ref));
                    }
                }
            }
            source_has_forms.push(!fields.is_empty());
            source_pages.push(pages);
        }

        let mut selected_counts: Vec<Vec<usize>> = source_pages
            .iter()
            .map(|pages| vec![0usize; pages.len()])
            .collect();
        for page in selection {
            let counts = selected_counts
                .get_mut(page.document_index)
                .ok_or_else(|| {
                    PdfError::PageOperation(format!(
                        "selection references missing document {}",
                        page.document_index
                    ))
                })?;
            let count = counts
                .get_mut(page.page_index)
                .ok_or(PdfError::PageNotFound(page.page_index))?;
            *count = count.checked_add(1).ok_or_else(|| {
                PdfError::PageResourceLimit("page selection count overflow".into())
            })?;
        }
        for (document_index, has_forms) in source_has_forms.iter().copied().enumerate() {
            if has_forms
                && !selected_counts[document_index]
                    .iter()
                    .all(|count| *count == 1)
            {
                return Err(PdfError::PartialFieldImport(format!(
                    "document {document_index} contains form fields; v0.12A copies its field graph only when every source page is selected exactly once"
                )));
            }
        }

        let catalog_ref = ObjectRef::new(1, 0);
        let pages_ref = ObjectRef::new(2, 0);
        let page_count_u64 = u64::try_from(selection.len()).map_err(|_| {
            PdfError::PageResourceLimit("selected page count conversion overflow".into())
        })?;
        let next_object = page_count_u64.checked_add(3).ok_or_else(|| {
            PdfError::PageResourceLimit("destination page allocation overflow".into())
        })?;
        let mut assembler = Assembler {
            limits,
            objects: BTreeMap::new(),
            remap: BTreeMap::new(),
            page_map: BTreeMap::new(),
            all_page_refs: source_pages
                .iter()
                .map(|pages| pages.iter().copied().collect())
                .collect(),
            force_shared,
            source_catalogs: documents.iter().map(PdfDocument::catalog_ref).collect(),
            source_page_roots: documents.iter().map(PdfDocument::root_pages_ref).collect(),
            next_object,
            total_stream_bytes: 0,
        };

        let mut destination_pages = Vec::with_capacity(selection.len());
        for (output_index, page) in selection.iter().enumerate() {
            let number = u64::try_from(output_index)
                .ok()
                .and_then(|value| value.checked_add(3))
                .ok_or_else(|| {
                    PdfError::PageResourceLimit("destination page number overflow".into())
                })?;
            let destination_ref = ObjectRef::new(number, 0);
            destination_pages.push(destination_ref);
            let source_ref = source_pages[page.document_index][page.page_index];
            assembler
                .page_map
                .entry((page.document_index, source_ref))
                .or_insert(destination_ref);
        }

        for (output_index, page) in selection.iter().enumerate() {
            let destination_ref = destination_pages[output_index];
            let source_ref = source_pages[page.document_index][page.page_index];
            let mut page_dict = materialize_page_inheritance(
                documents[page.document_index].store_mut(),
                source_ref,
                limits,
            )?;
            let mut imported = BTreeMap::new();
            for (key, value) in std::mem::take(&mut page_dict) {
                if key == "Parent" {
                    continue;
                }
                let scope = if key == "Annots" {
                    u32::try_from(output_index)
                        .ok()
                        .and_then(|value| value.checked_add(1))
                        .ok_or_else(|| {
                            PdfError::PageResourceLimit("annotation clone scope overflow".into())
                        })?
                } else {
                    0
                };
                let remapped = assembler.remap_object(
                    &mut documents,
                    page.document_index,
                    value,
                    scope,
                    Some((source_ref, destination_ref)),
                    0,
                )?;
                imported.insert(key, remapped);
            }
            imported.insert("Type".into(), PdfObject::Name("Page".into()));
            imported.insert("Parent".into(), PdfObject::Reference(pages_ref));
            assembler
                .objects
                .insert(destination_ref, PdfObject::Dictionary(imported));
        }

        let pages_dict = BTreeMap::from([
            ("Type".into(), PdfObject::Name("Pages".into())),
            (
                "Kids".into(),
                PdfObject::Array(
                    destination_pages
                        .iter()
                        .copied()
                        .map(PdfObject::Reference)
                        .collect(),
                ),
            ),
            (
                "Count".into(),
                PdfObject::Integer(i64::try_from(destination_pages.len()).map_err(|_| {
                    PdfError::PageResourceLimit("destination page count overflow".into())
                })?),
            ),
        ]);
        assembler
            .objects
            .insert(pages_ref, PdfObject::Dictionary(pages_dict));

        let acroform =
            assembler.build_acroform(&mut documents, &source_has_forms, &selected_counts)?;
        let mut catalog = BTreeMap::from([
            ("Type".into(), PdfObject::Name("Catalog".into())),
            ("Pages".into(), PdfObject::Reference(pages_ref)),
        ]);
        assembler.import_primary_catalog_entries(&mut documents, &selected_counts, &mut catalog)?;
        if let Some(acroform_ref) = acroform {
            catalog.insert("AcroForm".into(), PdfObject::Reference(acroform_ref));
        }
        assembler
            .objects
            .insert(catalog_ref, PdfObject::Dictionary(catalog));

        let trailer = assembler.import_primary_trailer(&mut documents)?;
        let version = documents
            .first()
            .map_or("1.7", |document| document.version());
        let output = CompleteWriter::write(
            version,
            &assembler.objects,
            catalog_ref,
            &trailer,
            limits.max_output_bytes,
        )?;
        Self::verify_output(&output, selection.len())?;
        Ok(output)
    }

    fn ensure_allowed(document: &mut PdfDocument<'_>) -> PdfResult<()> {
        let security = document.security_info()?;
        if security.encryption_state != EncryptionState::NotEncrypted {
            return Err(PdfError::EncryptedDocumentUnsupported(
                "complete page-graph rebuilding does not rewrite encrypted objects".into(),
            ));
        }
        if security.signature_state != SignatureState::Unsigned {
            return Err(PdfError::SignatureMutationUnsupported(
                "complete page-graph rebuilding is refused for structurally signed documents"
                    .into(),
            ));
        }
        Ok(())
    }

    fn verify_output(bytes: &[u8], expected_pages: usize) -> PdfResult<()> {
        let mut reopened = PdfDocument::from_bytes(bytes)?;
        StructuralValidator::validate(&mut reopened)?;
        let root_pages_ref = reopened.root_pages_ref();
        let pages = PageTree::validate_and_collect(reopened.store_mut(), root_pages_ref)?;
        if pages.len() != expected_pages {
            return Err(PdfError::PageOperation(format!(
                "reopened output has {} pages, expected {expected_pages}",
                pages.len()
            )));
        }
        Ok(())
    }
}

struct Assembler<'a> {
    limits: &'a PageOperationLimits,
    objects: BTreeMap<ObjectRef, PdfObject>,
    remap: BTreeMap<ImportKey, ObjectRef>,
    page_map: BTreeMap<(usize, ObjectRef), ObjectRef>,
    all_page_refs: Vec<BTreeSet<ObjectRef>>,
    force_shared: BTreeSet<(usize, ObjectRef)>,
    source_catalogs: Vec<ObjectRef>,
    source_page_roots: Vec<ObjectRef>,
    next_object: u64,
    total_stream_bytes: usize,
}

impl Assembler<'_> {
    fn remap_object(
        &mut self,
        documents: &mut [PdfDocument<'_>],
        document_index: usize,
        object: PdfObject,
        scope: u32,
        current_page: Option<(ObjectRef, ObjectRef)>,
        depth: usize,
    ) -> PdfResult<PdfObject> {
        if depth > self.limits.max_dependency_depth {
            return Err(PdfError::PageResourceLimit(format!(
                "dependency traversal depth exceeds {}",
                self.limits.max_dependency_depth
            )));
        }
        match object {
            PdfObject::Reference(reference) => self
                .import_reference(
                    documents,
                    document_index,
                    reference,
                    scope,
                    current_page,
                    depth,
                )
                .map(PdfObject::Reference),
            PdfObject::Array(values) => {
                if values.len() > self.limits.max_imported_objects {
                    return Err(PdfError::PageResourceLimit(
                        "direct array exceeds dependency node limit".into(),
                    ));
                }
                let mut remapped = Vec::with_capacity(values.len());
                for value in values {
                    remapped.push(self.remap_object(
                        documents,
                        document_index,
                        value,
                        scope,
                        current_page,
                        depth + 1,
                    )?);
                }
                Ok(PdfObject::Array(remapped))
            }
            PdfObject::Dictionary(values) => {
                if values.len() > self.limits.max_imported_objects {
                    return Err(PdfError::PageResourceLimit(
                        "direct dictionary exceeds dependency node limit".into(),
                    ));
                }
                let mut remapped = BTreeMap::new();
                for (key, value) in values {
                    let mapped = if key == "P" {
                        if let Some((source_page, destination_page)) = current_page {
                            if value.as_reference() == Some(source_page) {
                                PdfObject::Reference(destination_page)
                            } else {
                                self.remap_object(
                                    documents,
                                    document_index,
                                    value,
                                    scope,
                                    current_page,
                                    depth + 1,
                                )?
                            }
                        } else {
                            self.remap_object(
                                documents,
                                document_index,
                                value,
                                scope,
                                current_page,
                                depth + 1,
                            )?
                        }
                    } else {
                        self.remap_object(
                            documents,
                            document_index,
                            value,
                            scope,
                            current_page,
                            depth + 1,
                        )?
                    };
                    remapped.insert(key, mapped);
                }
                Ok(PdfObject::Dictionary(remapped))
            }
            PdfObject::Stream(stream) => {
                self.total_stream_bytes = self
                    .total_stream_bytes
                    .checked_add(stream.data.len())
                    .ok_or_else(|| {
                        PdfError::PageResourceLimit("imported stream byte count overflow".into())
                    })?;
                if self.total_stream_bytes > self.limits.max_total_stream_bytes {
                    return Err(PdfError::PageResourceLimit(format!(
                        "imported stream bytes exceed {}",
                        self.limits.max_total_stream_bytes
                    )));
                }
                let dict = match self.remap_object(
                    documents,
                    document_index,
                    PdfObject::Dictionary(stream.dict),
                    scope,
                    current_page,
                    depth + 1,
                )? {
                    PdfObject::Dictionary(dict) => dict,
                    _ => {
                        return Err(PdfError::PageOperation(
                            "stream dictionary remapping produced a non-dictionary".into(),
                        ))
                    }
                };
                Ok(PdfObject::Stream(StreamObject {
                    dict,
                    stream_length: stream.data.len(),
                    data: stream.data,
                    stream_offset: 0,
                }))
            }
            direct => Ok(direct),
        }
    }

    fn import_reference(
        &mut self,
        documents: &mut [PdfDocument<'_>],
        document_index: usize,
        source_ref: ObjectRef,
        requested_scope: u32,
        current_page: Option<(ObjectRef, ObjectRef)>,
        depth: usize,
    ) -> PdfResult<ObjectRef> {
        if let Some((source_page, destination_page)) = current_page {
            if source_ref == source_page {
                return Ok(destination_page);
            }
        }
        if let Some(destination) = self.page_map.get(&(document_index, source_ref)) {
            return Ok(*destination);
        }
        if self
            .all_page_refs
            .get(document_index)
            .is_some_and(|pages| pages.contains(&source_ref))
        {
            return Err(PdfError::ExcludedPageTarget(format!(
                "object graph references excluded page {source_ref} from document {document_index}"
            )));
        }
        if self.source_catalogs.get(document_index) == Some(&source_ref)
            || self.source_page_roots.get(document_index) == Some(&source_ref)
        {
            return Err(PdfError::UnsupportedPageDependency(format!(
                "imported dependency references source catalog or page-tree root {source_ref}"
            )));
        }
        let scope = if self.force_shared.contains(&(document_index, source_ref)) {
            0
        } else {
            requested_scope
        };
        let key = ImportKey {
            document_index,
            source_ref,
            scope,
        };
        if let Some(destination) = self.remap.get(&key) {
            return Ok(*destination);
        }
        if self.remap.len() >= self.limits.max_remap_entries
            || self.objects.len() >= self.limits.max_imported_objects
        {
            return Err(PdfError::PageResourceLimit(format!(
                "object remap exceeds {} entries",
                self.limits.max_remap_entries
            )));
        }
        let destination = self.allocate_object()?;
        self.remap.insert(key, destination);
        let source = documents
            .get_mut(document_index)
            .ok_or_else(|| {
                PdfError::PageOperation(format!(
                    "dependency references missing document {document_index}"
                ))
            })?
            .store_mut()
            .resolve(source_ref)?
            .clone();
        let imported = self.remap_object(
            documents,
            document_index,
            source,
            scope,
            current_page,
            depth + 1,
        )?;
        self.objects.insert(destination, imported);
        Ok(destination)
    }

    fn allocate_object(&mut self) -> PdfResult<ObjectRef> {
        if self.objects.len() >= self.limits.max_imported_objects {
            return Err(PdfError::PageResourceLimit(format!(
                "destination object count exceeds {}",
                self.limits.max_imported_objects
            )));
        }
        let reference = ObjectRef::new(self.next_object, 0);
        self.next_object = self.next_object.checked_add(1).ok_or_else(|| {
            PdfError::PageResourceLimit("destination object number overflow".into())
        })?;
        Ok(reference)
    }

    fn build_acroform(
        &mut self,
        documents: &mut [PdfDocument<'_>],
        source_has_forms: &[bool],
        selected_counts: &[Vec<usize>],
    ) -> PdfResult<Option<ObjectRef>> {
        let mut combined_fields = Vec::new();
        let mut combined = BTreeMap::new();
        let mut found = false;
        for document_index in 0..documents.len() {
            if !source_has_forms[document_index]
                || !selected_counts[document_index]
                    .iter()
                    .all(|count| *count == 1)
            {
                continue;
            }
            let catalog_ref = documents[document_index].catalog_ref();
            let catalog = documents[document_index]
                .store_mut()
                .resolve(catalog_ref)?
                .clone();
            let catalog_dict = catalog.as_dict().ok_or_else(|| PdfError::TypeMismatch {
                expected: "catalog dictionary",
                actual: catalog.type_name(),
            })?;
            let Some(acroform_value) = catalog_dict.get("AcroForm").cloned() else {
                continue;
            };
            let acroform = documents[document_index]
                .store_mut()
                .resolve_object(&acroform_value)?;
            let acroform_dict =
                acroform
                    .as_dict()
                    .cloned()
                    .ok_or_else(|| PdfError::TypeMismatch {
                        expected: "AcroForm dictionary",
                        actual: acroform.type_name(),
                    })?;
            if acroform_dict.contains_key("XFA") {
                return Err(PdfError::UnsupportedPageDependency(
                    "XFA form extraction is not supported by v0.12A page operations".into(),
                ));
            }
            let fields_source = acroform_dict
                .get("Fields")
                .cloned()
                .unwrap_or_else(|| PdfObject::Array(Vec::new()));
            let fields_source = documents[document_index]
                .store_mut()
                .resolve_object(&fields_source)?;
            let fields = fields_source.as_array().ok_or_else(|| {
                PdfError::PageOperation("AcroForm /Fields must resolve to an array".into())
            })?;
            if combined_fields.len().checked_add(fields.len()).is_none()
                || combined_fields.len() + fields.len() > self.limits.max_form_fields
            {
                return Err(PdfError::PageResourceLimit(format!(
                    "standalone form field count exceeds {}",
                    self.limits.max_form_fields
                )));
            }

            let imported_dr = if let Some(value) = acroform_dict.get("DR").cloned() {
                Some(self.remap_object(documents, document_index, value, 0, None, 0)?)
            } else {
                None
            };
            let inherited_da = acroform_dict.get("DA").cloned();
            for field in fields {
                let imported =
                    self.remap_object(documents, document_index, field.clone(), 0, None, 0)?;
                if let Some(field_ref) = imported.as_reference() {
                    if let Some(field_object) = self.objects.get_mut(&field_ref) {
                        if let Some(field_dict) = field_object.as_dict_mut() {
                            if let Some(dr) = imported_dr.clone() {
                                field_dict.entry("DR".into()).or_insert(dr);
                            }
                            if let Some(da) = inherited_da.clone() {
                                field_dict.entry("DA".into()).or_insert(da);
                            }
                        }
                    }
                }
                combined_fields.push(imported);
            }

            if !found {
                for (key, value) in acroform_dict {
                    if matches!(key.as_str(), "Fields" | "DR" | "DA") {
                        continue;
                    }
                    combined.insert(
                        key,
                        self.remap_object(documents, document_index, value, 0, None, 0)?,
                    );
                }
                if let Some(dr) = imported_dr {
                    combined.insert("DR".into(), dr);
                }
                if let Some(da) = inherited_da {
                    combined.insert("DA".into(), da);
                }
            }
            found = true;
        }
        if !found {
            return Ok(None);
        }
        combined.insert("Fields".into(), PdfObject::Array(combined_fields));
        let reference = self.allocate_object()?;
        self.objects
            .insert(reference, PdfObject::Dictionary(combined));
        Ok(Some(reference))
    }

    fn import_primary_catalog_entries(
        &mut self,
        documents: &mut [PdfDocument<'_>],
        selected_counts: &[Vec<usize>],
        destination: &mut BTreeMap<String, PdfObject>,
    ) -> PdfResult<()> {
        let primary = documents
            .first_mut()
            .ok_or_else(|| PdfError::PageOperation("primary document is missing".into()))?;
        let catalog_ref = primary.catalog_ref();
        let catalog = primary.store_mut().resolve(catalog_ref)?.clone();
        let catalog_dict = catalog.as_dict().ok_or_else(|| PdfError::TypeMismatch {
            expected: "catalog dictionary",
            actual: catalog.type_name(),
        })?;
        for key in [
            "Lang",
            "Metadata",
            "ViewerPreferences",
            "PageMode",
            "PageLayout",
            "MarkInfo",
            "OutputIntents",
            "OCProperties",
        ] {
            if let Some(value) = catalog_dict.get(key).cloned() {
                destination.insert(
                    key.into(),
                    self.remap_object(documents, 0, value, 0, None, 0)?,
                );
            }
        }
        let primary_complete = selected_counts
            .first()
            .is_some_and(|counts| counts.iter().all(|count| *count == 1));
        if primary_complete {
            for key in ["Names", "Outlines", "OpenAction"] {
                if let Some(value) = catalog_dict.get(key).cloned() {
                    destination.insert(
                        key.into(),
                        self.remap_object(documents, 0, value, 0, None, 0)?,
                    );
                }
            }
        }
        Ok(())
    }

    fn import_primary_trailer(
        &mut self,
        documents: &mut [PdfDocument<'_>],
    ) -> PdfResult<BTreeMap<String, PdfObject>> {
        let primary = documents
            .first()
            .ok_or_else(|| PdfError::PageOperation("primary document is missing".into()))?;
        let info = primary.trailer().get("Info").cloned();
        let id = primary.trailer().get("ID").cloned();
        let mut trailer = BTreeMap::new();
        if let Some(info) = info {
            trailer.insert(
                "Info".into(),
                self.remap_object(documents, 0, info, 0, None, 0)?,
            );
        }
        if let Some(id) = id {
            trailer.insert(
                "ID".into(),
                self.remap_object(documents, 0, id, 0, None, 0)?,
            );
        }
        Ok(trailer)
    }
}
