use std::collections::BTreeMap;

use crate::document::object_store::ObjectStore;
use crate::document::pages::PageTree;
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

        // 1. Verify header signature %PDF-
        let header_pos = source
            .find_from(0, b"%PDF-")
            .ok_or(PdfError::InvalidHeader)?;

        if header_pos > 1024 {
            return Err(PdfError::InvalidHeader);
        }

        // Extract version string (e.g. "1.7")
        let version_slice = source.get_slice(header_pos + 5, 3).unwrap_or(b"1.7");
        let version = String::from_utf8_lossy(version_slice).to_string();

        // 2. Locate and parse XRef table and Trailer
        let xref_table = XrefResolver::load_xref_and_trailer_with_limits(source, &limits)?;

        // 3. Initialize Lazy Object Store
        let mut store = ObjectStore::new_with_limits(source, xref_table, limits);

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
            source,
            version,
            store,
            catalog_ref,
            root_pages_ref,
        })
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
}
