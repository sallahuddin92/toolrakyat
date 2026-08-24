use std::collections::BTreeMap;

use crate::document::object_store::ObjectStore;
use crate::document::pages::PageTree;
use crate::document::recovery::{RecoveryKind, RecoveryTracker};
use crate::error::{PdfError, PdfResult};
use crate::io::source::ByteSource;
use crate::syntax::object::{ObjectRef, PdfObject};
use crate::xref::resolver::XrefResolver;

pub struct PdfDocument<'a> {
    source: ByteSource<'a>,
    version: String,
    store: ObjectStore<'a>,
    catalog_ref: ObjectRef,
    root_pages_ref: ObjectRef,
    pub recovery_tracker: RecoveryTracker,
}

impl<'a> PdfDocument<'a> {
    /// Opens and validates a PDF document from an in-memory byte slice (default limits).
    pub fn from_bytes(bytes: &'a [u8]) -> PdfResult<Self> {
        Self::from_bytes_with_limits(bytes, crate::filter::limits::DecompressLimits::default())
    }

    /// Opens and validates a PDF document from an in-memory byte slice with custom limits.
    pub fn from_bytes_with_limits(
        bytes: &'a [u8],
        limits: crate::filter::limits::DecompressLimits,
    ) -> PdfResult<Self> {
        let source = ByteSource::new(bytes);
        let mut recovery_tracker = RecoveryTracker::new();

        // 1. Verify header signature %PDF-
        let header_pos = source
            .find_from(0, b"%PDF-")
            .ok_or(PdfError::InvalidHeader)?;

        if header_pos > 4096 {
            return Err(PdfError::InvalidHeader);
        }

        if header_pos > 0 {
            recovery_tracker.record(
                RecoveryKind::ProducerCompatibilityPath,
                format!("Header preceded by {header_pos} leading bytes/BOM"),
            );
        }

        // Extract version string (e.g. "1.7")
        let version_slice = source.get_slice(header_pos + 5, 3).unwrap_or(b"1.7");
        let version = String::from_utf8_lossy(version_slice).to_string();

        let effective_source = if header_pos > 0 {
            ByteSource::new(&bytes[header_pos..])
        } else {
            source
        };

        // 2. Locate and parse XRef table and Trailer
        let (xref_table, store_source) =
            match XrefResolver::load_xref_and_trailer_with_limits(effective_source, &limits) {
                Ok(table) => (table, effective_source),
                Err(_) => (
                    XrefResolver::load_xref_and_trailer_with_limits(source, &limits)?,
                    source,
                ),
            };

        // 3. Initialize Lazy Object Store
        let mut store = ObjectStore::new_with_limits(store_source, xref_table, limits);

        // 4. Resolve /Root Catalog
        let catalog_ref = store
            .trailer()
            .get("Root")
            .and_then(|v| v.as_reference())
            .ok_or_else(|| {
                PdfError::InvalidSyntax("Trailer missing /Root catalog reference".into())
            })?;

        let catalog_obj = store.resolve(catalog_ref)?.clone();
        let catalog_dict = catalog_obj
            .as_dict()
            .ok_or_else(|| PdfError::TypeMismatch {
                expected: "dictionary",
                actual: catalog_obj.type_name(),
            })?;

        // 5. Resolve /Pages root node
        let root_pages_ref = catalog_dict
            .get("Pages")
            .and_then(|v| v.as_reference())
            .ok_or_else(|| PdfError::InvalidSyntax("Catalog missing /Pages reference".into()))?;

        Ok(Self {
            source: store_source,
            version,
            store,
            catalog_ref,
            root_pages_ref,
            recovery_tracker,
        })
    }

    #[inline]
    pub fn recovery_status(&self) -> RecoveryKind {
        self.recovery_tracker.primary_status()
    }

    #[inline]
    pub fn recovery_tracker(&self) -> &RecoveryTracker {
        &self.recovery_tracker
    }

    #[inline]
    pub fn version(&self) -> &str {
        &self.version
    }

    #[inline]
    pub fn source(&self) -> ByteSource<'a> {
        self.source
    }

    #[inline]
    pub fn store(&self) -> &ObjectStore<'a> {
        &self.store
    }

    #[inline]
    pub fn store_mut(&mut self) -> &mut ObjectStore<'a> {
        &mut self.store
    }

    #[inline]
    pub fn trailer(&self) -> &BTreeMap<String, PdfObject> {
        self.store.trailer()
    }

    #[inline]
    pub fn catalog_ref(&self) -> ObjectRef {
        self.catalog_ref
    }

    #[inline]
    pub fn root_pages_ref(&self) -> ObjectRef {
        self.root_pages_ref
    }

    /// Returns the total number of pages in the document.
    pub fn page_count(&mut self) -> PdfResult<usize> {
        PageTree::count_pages(&mut self.store, self.root_pages_ref)
    }

    /// Resolves the 0-indexed page object reference.
    pub fn page_ref(&mut self, page_index: usize) -> PdfResult<ObjectRef> {
        PageTree::get_page_ref(&mut self.store, self.root_pages_ref, page_index)
    }

    /// Returns the resolved dictionary of a 0-indexed page.
    pub fn page_dict(&mut self, page_index: usize) -> PdfResult<BTreeMap<String, PdfObject>> {
        PageTree::get_page_dict(&mut self.store, self.root_pages_ref, page_index)
    }

    /// Extracts coordinate-aware text spans from a single 0-indexed page.
    pub fn extract_page_text(
        &mut self,
        page_index: usize,
    ) -> PdfResult<crate::text::span::PageText> {
        let page_dict = self.page_dict(page_index)?;
        let resources =
            crate::font::resource::PageResources::resolve_for_page(&page_dict, &mut self.store)?;

        let mut decompressed_streams = Vec::new();
        if let Some(contents_obj) = page_dict.get("Contents") {
            let resolved_contents = self.store.resolve_object(contents_obj)?;
            match resolved_contents {
                PdfObject::Stream(stream) => {
                    let decompressed = self.decompress_stream_data(&stream)?;
                    decompressed_streams.push(decompressed);
                }
                PdfObject::Array(streams_arr) => {
                    for stream_ref in streams_arr {
                        let stream_obj = self.store.resolve_object(&stream_ref)?;
                        if let Some(stream) = stream_obj.as_stream() {
                            let decompressed = self.decompress_stream_data(stream)?;
                            decompressed_streams.push(decompressed);
                        }
                    }
                }
                _ => {}
            }
        }

        let stream_slices: Vec<&[u8]> = decompressed_streams.iter().map(Vec::as_slice).collect();
        crate::text::extractor::TextExtractor::extract_from_streams(
            page_index,
            &stream_slices,
            &resources,
        )
    }

    /// Extracts text across all pages of the document.
    pub fn extract_all_text(&mut self) -> PdfResult<Vec<crate::text::span::PageText>> {
        let count = self.page_count()?;
        let mut results = Vec::with_capacity(count);
        for i in 0..count {
            results.push(self.extract_page_text(i)?);
        }
        Ok(results)
    }

    /// Builds a full document search index.
    pub fn build_search_index(&mut self) -> PdfResult<crate::search::DocumentSearchIndex> {
        let pages_text = self.extract_all_text()?;
        Ok(crate::search::DocumentSearchIndex::new(pages_text))
    }

    /// Performs text search across all pages with given search options.
    pub fn search(
        &mut self,
        query: &str,
        options: &crate::search::SearchOptions,
    ) -> PdfResult<Vec<crate::search::SearchResult>> {
        let index = self.build_search_index()?;
        Ok(index.search(query, options))
    }

    /// Returns all page object references in document order.
    pub fn page_refs(&mut self) -> PdfResult<Vec<ObjectRef>> {
        let count = self.page_count()?;
        let mut refs = Vec::with_capacity(count);
        for i in 0..count {
            refs.push(self.page_ref(i)?);
        }
        Ok(refs)
    }

    /// Parses and returns the document's AcroForm structure, if present.
    pub fn acroform(&mut self) -> PdfResult<Option<crate::forms::AcroForm>> {
        let catalog_obj = self.store.resolve(self.catalog_ref)?.clone();
        let catalog_dict = catalog_obj
            .as_dict()
            .ok_or_else(|| PdfError::TypeMismatch {
                expected: "dictionary",
                actual: catalog_obj.type_name(),
            })?
            .clone();

        let page_refs = self.page_refs()?;
        let parser = crate::forms::AcroFormParser::new(&mut self.store, &page_refs);
        parser.parse_catalog_acroform(&catalog_dict)
    }

    /// Returns all interactive form fields detected in the AcroForm hierarchy.
    pub fn form_fields(&mut self) -> PdfResult<Vec<crate::forms::FormField>> {
        match self.acroform()? {
            Some(af) => Ok(af.fields),
            None => Ok(Vec::new()),
        }
    }

    /// Parses and returns all annotations present on a specific page.
    pub fn page_annotations(
        &mut self,
        page_index: usize,
    ) -> PdfResult<Vec<crate::annotation::Annotation>> {
        let page_dict = self.page_dict(page_index)?;
        let mut parser = crate::annotation::AnnotationParser::new(&mut self.store);
        parser.parse_page_annotations(&page_dict, page_index)
    }

    /// Prepares a validated mutation plan for a set of field/widget changes.
    pub fn apply_mutation(
        &mut self,
        changes: &[crate::mutation::PdfChange],
    ) -> PdfResult<crate::mutation::MutationPlan> {
        let security = self.security_info()?;
        if security.encryption_state != crate::security::EncryptionState::NotEncrypted {
            return Err(PdfError::EncryptedDocumentUnsupported(
                security.encryption_state.as_str().to_string(),
            ));
        }
        if security.signature_state == crate::security::SignatureState::SignedStructureMalformed {
            return Err(PdfError::SignatureMutationUnsupported(
                "signature structure or ByteRange is malformed".into(),
            ));
        }
        let fields = self.form_fields()?;
        for change in changes {
            let target = match change {
                crate::mutation::PdfChange::SetTextField { field_ref, .. }
                | crate::mutation::PdfChange::SetCheckbox { field_ref, .. }
                | crate::mutation::PdfChange::SetChoice { field_ref, .. }
                | crate::mutation::PdfChange::SetChoiceValues { field_ref, .. } => Some(*field_ref),
                crate::mutation::PdfChange::SetRadio { parent_ref, .. } => Some(*parent_ref),
                _ => None,
            };
            if target.is_some_and(|reference| {
                fields.iter().any(|field| {
                    field.object_ref == reference
                        && matches!(field.field_type, crate::forms::FieldType::Signature)
                })
            }) {
                return Err(PdfError::SignatureMutationUnsupported(
                    "signature fields are inspection-only".into(),
                ));
            }
            if matches!(change, crate::mutation::PdfChange::SetRadio { .. })
                && target.is_some_and(|reference| {
                    fields.iter().any(|field| {
                        field.object_ref == reference
                            && matches!(
                                field.graph_classification,
                                crate::forms::FieldGraphClassification::AmbiguousWidgetGroup
                                    | crate::forms::FieldGraphClassification::MalformedFieldGraph
                            )
                    })
                })
            {
                return Err(PdfError::AmbiguousFieldGraph(
                    "radio group membership is malformed or not proven by object relationships"
                        .into(),
                ));
            }
        }
        let page_refs = self.page_refs()?;
        let mut engine = crate::mutation::MutationEngine::new(&mut self.store, &page_refs);
        engine.prepare_plan(changes)
    }

    pub fn security_info(&mut self) -> PdfResult<crate::security::DocumentSecurityInfo> {
        crate::security::DocumentSecurityInfo::inspect(&mut self.store, self.source.len())
    }

    /// Writes an incremental update based on a prepared MutationPlan.
    pub fn export_incremental(
        &mut self,
        plan: &crate::mutation::MutationPlan,
    ) -> PdfResult<Vec<u8>> {
        let prev_startxref = self.store.xref().startxref_offset as usize;
        let trailer_dict = self.store.trailer().clone();
        let source_bytes = self.source.as_bytes();

        crate::writer::incremental::IncrementalWriter::write_update(
            source_bytes,
            &plan.modified_objects,
            prev_startxref,
            &trailer_dict,
        )
    }

    /// Mutates the document with the specified changes and exports an incrementally updated PDF.
    pub fn mutate_and_export(
        &mut self,
        changes: &[crate::mutation::PdfChange],
    ) -> PdfResult<Vec<u8>> {
        let plan = self.apply_mutation(changes)?;
        self.export_incremental(&plan)
    }

    /// Plans a native text replacement without mutating the document state.
    pub fn plan_text_replacement(
        &mut self,
        page_index: usize,
        target: &crate::mutation::text_edit::TextEditTarget,
        replacement: &str,
        requested_style: Option<&crate::font::font::FontStyle>,
    ) -> PdfResult<crate::font::planner::TextReplacementPlan> {
        crate::font::planner::TextPlanner::plan(
            self,
            page_index,
            target,
            replacement,
            requested_style,
        )
    }

    /// Replaces native existing content-stream text at a specific target.
    pub fn replace_text(
        &mut self,
        page_index: usize,
        target: &crate::mutation::text_edit::TextEditTarget,
        replacement: &str,
    ) -> PdfResult<crate::mutation::MutationPlan> {
        self.apply_mutation(&[crate::mutation::PdfChange::ReplaceText {
            page_index,
            target: target.clone(),
            replacement: replacement.to_string(),
        }])
    }

    /// Replaces native existing content-stream text specified by a structural span ID.
    pub fn replace_text_span(
        &mut self,
        page_index: usize,
        span_id: &str,
        replacement: &str,
    ) -> PdfResult<crate::mutation::MutationPlan> {
        let target = crate::mutation::text_edit::TextEditTarget::from_span_id(span_id)?;
        if target.page_index != page_index {
            return Err(PdfError::TargetTextNotFound(format!(
                "Span ID page mismatch: target is page {}, requested page {page_index}",
                target.page_index
            )));
        }
        self.replace_text(page_index, &target, replacement)
    }

    /// Replaces a group of native existing content-stream text spans atomically.
    pub fn replace_text_group(
        &mut self,
        page_index: usize,
        span_ids: &[String],
        replacement: &str,
    ) -> PdfResult<crate::mutation::MutationPlan> {
        if span_ids.is_empty() {
            return Err(PdfError::TargetTextNotFound(
                "No span IDs provided for text group replacement".into(),
            ));
        }
        if span_ids.len() == 1 {
            return self.replace_text_span(page_index, &span_ids[0], replacement);
        }

        let mut targets = Vec::with_capacity(span_ids.len());
        for span_id in span_ids {
            let target = crate::mutation::text_edit::TextEditTarget::from_span_id(span_id)?;
            if target.page_index != page_index {
                return Err(PdfError::TargetTextNotFound(format!(
                    "Span ID page mismatch: target is page {}, requested page {page_index}",
                    target.page_index
                )));
            }
            targets.push(target);
        }

        self.apply_mutation(&[crate::mutation::PdfChange::ReplaceTextGroup {
            page_index,
            targets,
            replacement: replacement.to_string(),
        }])
    }

    /// Moves native existing content-stream text at a specific target.
    pub fn move_text(
        &mut self,
        page_index: usize,
        target: &crate::mutation::text_edit::TextEditTarget,
        dx: f64,
        dy: f64,
    ) -> PdfResult<crate::mutation::MutationPlan> {
        self.apply_mutation(&[crate::mutation::PdfChange::MoveText {
            page_index,
            target: target.clone(),
            dx,
            dy,
        }])
    }

    /// Moves native existing content-stream text specified by a structural span ID.
    pub fn move_text_span(
        &mut self,
        page_index: usize,
        span_id: &str,
        dx: f64,
        dy: f64,
    ) -> PdfResult<crate::mutation::MutationPlan> {
        let target = crate::mutation::text_edit::TextEditTarget::from_span_id(span_id)?;
        if target.page_index != page_index {
            return Err(PdfError::TargetTextNotFound(format!(
                "Span ID page mismatch: target is page {}, requested page {page_index}",
                target.page_index
            )));
        }
        self.move_text(page_index, &target, dx, dy)
    }

    /// Moves a group of native existing content-stream text spans atomically.
    pub fn move_text_group(
        &mut self,
        page_index: usize,
        span_ids: &[String],
        dx: f64,
        dy: f64,
    ) -> PdfResult<crate::mutation::MutationPlan> {
        if span_ids.is_empty() {
            return Err(PdfError::TargetTextNotFound(
                "No span IDs provided for text group move".into(),
            ));
        }
        if span_ids.len() == 1 {
            return self.move_text_span(page_index, &span_ids[0], dx, dy);
        }

        let mut targets = Vec::with_capacity(span_ids.len());
        for span_id in span_ids {
            let target = crate::mutation::text_edit::TextEditTarget::from_span_id(span_id)?;
            if target.page_index != page_index {
                return Err(PdfError::TargetTextNotFound(format!(
                    "Span ID page mismatch: target is page {}, requested page {page_index}",
                    target.page_index
                )));
            }
            targets.push(target);
        }

        self.apply_mutation(&[crate::mutation::PdfChange::MoveTextGroup {
            page_index,
            targets,
            dx,
            dy,
        }])
    }

    /// Discovers all Image XObjects on a specific page.
    pub fn enumerate_images(
        &mut self,
        page_index: usize,
    ) -> PdfResult<Vec<crate::image::ImageXObjectInfo>> {
        let mut extractor = crate::image::ImageExtractor::new(self)?;
        extractor.extract_page_images(page_index)
    }

    /// Discovers all Image XObjects across the entire document.
    pub fn enumerate_all_images(&mut self) -> PdfResult<Vec<crate::image::ImageXObjectInfo>> {
        let mut extractor = crate::image::ImageExtractor::new(self)?;
        extractor.extract_all_images()
    }

    /// Replaces an existing Image XObject's content stream and dictionary.
    pub fn replace_image(
        &mut self,
        spec: &crate::image::ReplaceImageSpec,
    ) -> PdfResult<crate::mutation::MutationPlan> {
        self.apply_mutation(&[crate::mutation::PdfChange::ReplaceImage { spec: spec.clone() }])
    }

    /// Adds a new Image XObject to a page with content-stream draw operator.
    pub fn add_image(
        &mut self,
        spec: &crate::image::AddImageSpec,
    ) -> PdfResult<crate::mutation::MutationPlan> {
        self.apply_mutation(&[crate::mutation::PdfChange::AddImage { spec: spec.clone() }])
    }

    /// Moves and/or resizes an existing Image XObject on a page.
    pub fn update_image(
        &mut self,
        spec: &crate::image::UpdateImageSpec,
    ) -> PdfResult<crate::mutation::MutationPlan> {
        self.apply_mutation(&[crate::mutation::PdfChange::UpdateImage { spec: spec.clone() }])
    }

    /// Removes an Image XObject draw operation from a page.
    pub fn remove_image(
        &mut self,
        spec: &crate::image::RemoveImageSpec,
    ) -> PdfResult<crate::mutation::MutationPlan> {
        self.apply_mutation(&[crate::mutation::PdfChange::RemoveImage { spec: spec.clone() }])
    }

    /// Discovers and enumerates all vector graphics on a single page.
    pub fn enumerate_graphics(
        &mut self,
        page_index: usize,
    ) -> PdfResult<Vec<crate::vector::VectorGraphicInfo>> {
        let all = self.enumerate_all_graphics()?;
        Ok(all
            .into_iter()
            .filter(|g| g.page_index == page_index)
            .collect())
    }

    /// Discovers and enumerates all vector graphics across the entire document.
    pub fn enumerate_all_graphics(&mut self) -> PdfResult<Vec<crate::vector::VectorGraphicInfo>> {
        let mut extractor = crate::vector::VectorExtractor::new(self)?;
        extractor.extract_all_graphics()
    }

    /// Updates an existing vector graphic or path object on a page.
    pub fn update_graphic(
        &mut self,
        spec: &crate::vector::UpdateVectorGraphicSpec,
    ) -> PdfResult<crate::mutation::MutationPlan> {
        self.apply_mutation(&[crate::mutation::PdfChange::UpdateVectorGraphic {
            spec: spec.clone(),
        }])
    }

    /// Adds a new vector graphic (Rectangle or Line) to a page.
    pub fn add_graphic(
        &mut self,
        spec: &crate::vector::AddVectorGraphicSpec,
    ) -> PdfResult<crate::mutation::MutationPlan> {
        self.apply_mutation(&[crate::mutation::PdfChange::AddVectorGraphic { spec: spec.clone() }])
    }

    /// Deletes an existing vector graphic from a page.
    pub fn delete_graphic(
        &mut self,
        spec: &crate::vector::DeleteVectorGraphicSpec,
    ) -> PdfResult<crate::mutation::MutationPlan> {
        self.apply_mutation(&[crate::mutation::PdfChange::DeleteVectorGraphic {
            spec: spec.clone(),
        }])
    }

    /// Applies an ordered, atomic page-operation plan. Intermediate outputs remain private and
    /// are returned only after the complete plan reopens successfully.
    pub fn apply_page_operations(
        &mut self,
        plan: &crate::page_ops::PageOperationPlan,
        limits: &crate::page_ops::PageOperationLimits,
    ) -> PdfResult<Vec<u8>> {
        let mut current = self.source.as_bytes().to_vec();
        for edit in &plan.edits {
            let next = match edit {
                crate::page_ops::PageEdit::DuplicatePage { index, insert_at } => {
                    crate::page_ops::DocumentBuilder::duplicate_page(
                        &current, *index, *insert_at, limits,
                    )?
                }
                other => {
                    let mut document = PdfDocument::from_bytes(&current)?;
                    crate::page_ops::IncrementalPageEditor::apply(
                        &mut document,
                        &crate::page_ops::PageOperationPlan::new(other.clone()),
                        limits,
                    )?
                }
            };
            current = next;
        }
        {
            let mut reopened = PdfDocument::from_bytes(&current)?;
            crate::validate::StructuralValidator::validate(&mut reopened)?;
        }
        Ok(current)
    }

    pub fn delete_page(&mut self, page_index: usize) -> PdfResult<Vec<u8>> {
        self.apply_page_operations(
            &crate::page_ops::PageOperationPlan::new(crate::page_ops::PageEdit::DeletePage {
                index: page_index,
            }),
            &crate::page_ops::PageOperationLimits::default(),
        )
    }

    pub fn move_page(&mut self, from_index: usize, to_index: usize) -> PdfResult<Vec<u8>> {
        self.apply_page_operations(
            &crate::page_ops::PageOperationPlan::new(crate::page_ops::PageEdit::MovePage {
                from_index,
                to_index,
            }),
            &crate::page_ops::PageOperationLimits::default(),
        )
    }

    pub fn duplicate_page(&mut self, page_index: usize, insert_at: usize) -> PdfResult<Vec<u8>> {
        self.apply_page_operations(
            &crate::page_ops::PageOperationPlan::new(crate::page_ops::PageEdit::DuplicatePage {
                index: page_index,
                insert_at,
            }),
            &crate::page_ops::PageOperationLimits::default(),
        )
    }

    pub fn insert_blank_page(
        &mut self,
        index: usize,
        width: f64,
        height: f64,
        rotation: i32,
    ) -> PdfResult<Vec<u8>> {
        self.apply_page_operations(
            &crate::page_ops::PageOperationPlan::new(crate::page_ops::PageEdit::InsertBlankPage {
                index,
                width,
                height,
                rotation,
            }),
            &crate::page_ops::PageOperationLimits::default(),
        )
    }

    pub fn extract_pages(&mut self, page_indices: &[usize]) -> PdfResult<Vec<u8>> {
        crate::page_ops::DocumentBuilder::extract_pages(
            self.source.as_bytes(),
            page_indices,
            &crate::page_ops::PageOperationLimits::default(),
        )
    }

    pub fn insert_page_from(
        &mut self,
        imported_document: &PdfDocument<'_>,
        imported_page_index: usize,
        insert_at: usize,
    ) -> PdfResult<Vec<u8>> {
        crate::page_ops::DocumentBuilder::insert_page(
            self.source.as_bytes(),
            imported_document.source.as_bytes(),
            imported_page_index,
            insert_at,
            &crate::page_ops::PageOperationLimits::default(),
        )
    }

    pub fn merge_documents(inputs: &[&[u8]]) -> PdfResult<Vec<u8>> {
        crate::page_ops::DocumentBuilder::merge_documents(
            inputs,
            &crate::page_ops::PageOperationLimits::default(),
        )
    }

    pub fn merge_selected(
        inputs: &[&[u8]],
        page_sources: &[crate::page_ops::PageSource],
    ) -> PdfResult<Vec<u8>> {
        crate::page_ops::DocumentBuilder::merge_selected(
            inputs,
            page_sources,
            &crate::page_ops::PageOperationLimits::default(),
        )
    }

    pub fn split_document(
        &mut self,
        ranges: &[crate::page_ops::PageRange],
    ) -> PdfResult<Vec<Vec<u8>>> {
        crate::page_ops::DocumentBuilder::split_document(
            self.source.as_bytes(),
            ranges,
            &crate::page_ops::PageOperationLimits::default(),
        )
    }

    fn decompress_stream_data(
        &self,
        stream: &crate::syntax::object::StreamObject,
    ) -> PdfResult<Vec<u8>> {
        if let Some(filter) = stream.dict.get("Filter").and_then(|v| v.as_name()) {
            if filter == "FlateDecode" {
                return crate::filter::flate::FlateDecoder::decode(
                    &stream.data,
                    &crate::filter::limits::DecompressLimits::default(),
                );
            }
        }
        Ok(stream.data.clone())
    }
}
