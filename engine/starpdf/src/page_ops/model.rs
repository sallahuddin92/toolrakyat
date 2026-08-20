#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageOperationType {
    DeletePage,
    MovePage,
    DuplicatePage,
    InsertBlankPage,
    InsertImportedPage,
    ExtractPages,
    SplitDocument,
    MergeDocuments,
}

impl PageOperationType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DeletePage => "DELETE_PAGE",
            Self::MovePage => "MOVE_PAGE",
            Self::DuplicatePage => "DUPLICATE_PAGE",
            Self::InsertBlankPage => "INSERT_BLANK_PAGE",
            Self::InsertImportedPage => "INSERT_IMPORTED_PAGE",
            Self::ExtractPages => "EXTRACT_PAGES",
            Self::SplitDocument => "SPLIT_DOCUMENT",
            Self::MergeDocuments => "MERGE_DOCUMENTS",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageSource {
    pub document_index: usize,
    pub page_index: usize,
}

impl PageSource {
    pub const fn new(document_index: usize, page_index: usize) -> Self {
        Self {
            document_index,
            page_index,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageRange {
    pub start: usize,
    pub end_exclusive: usize,
}

impl PageRange {
    pub const fn new(start: usize, end_exclusive: usize) -> Self {
        Self {
            start,
            end_exclusive,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PageEdit {
    DeletePage {
        index: usize,
    },
    MovePage {
        from_index: usize,
        to_index: usize,
    },
    DuplicatePage {
        index: usize,
        insert_at: usize,
    },
    InsertBlankPage {
        index: usize,
        width: f64,
        height: f64,
        rotation: i32,
    },
}

impl PageEdit {
    pub const fn operation_type(&self) -> PageOperationType {
        match self {
            Self::DeletePage { .. } => PageOperationType::DeletePage,
            Self::MovePage { .. } => PageOperationType::MovePage,
            Self::DuplicatePage { .. } => PageOperationType::DuplicatePage,
            Self::InsertBlankPage { .. } => PageOperationType::InsertBlankPage,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PageOperationPlan {
    pub edits: Vec<PageEdit>,
}

impl PageOperationPlan {
    pub fn new(edit: PageEdit) -> Self {
        Self { edits: vec![edit] }
    }

    pub fn with_edits(edits: Vec<PageEdit>) -> Self {
        Self { edits }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyDisposition {
    SafeToShareImmutable,
    MustClone,
    MustRemap,
    UnsupportedDependency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentWriteMode {
    IncrementalDocumentUpdate,
    NewDocumentBuild,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageOperationLimits {
    pub max_input_documents: usize,
    pub max_total_pages: usize,
    pub max_selected_pages: usize,
    pub max_imported_objects: usize,
    pub max_dependency_depth: usize,
    pub max_remap_entries: usize,
    pub max_total_stream_bytes: usize,
    pub max_output_bytes: usize,
    pub max_annotations_per_page: usize,
    pub max_form_fields: usize,
    pub max_resources_per_page: usize,
}

impl Default for PageOperationLimits {
    fn default() -> Self {
        Self {
            max_input_documents: 16,
            max_total_pages: 10_000,
            max_selected_pages: 10_000,
            max_imported_objects: 100_000,
            max_dependency_depth: 64,
            max_remap_entries: 100_000,
            max_total_stream_bytes: 512 * 1024 * 1024,
            max_output_bytes: 768 * 1024 * 1024,
            max_annotations_per_page: 2_000,
            max_form_fields: 4_000,
            max_resources_per_page: 4_096,
        }
    }
}
