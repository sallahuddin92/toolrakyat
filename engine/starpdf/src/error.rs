use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum PdfError {
    UnexpectedEof,
    InvalidToken(String),
    InvalidSyntax(String),
    InvalidHeader,
    InvalidXref(String),
    UnrecoverableXref(String),
    RecoveredXrefExport(String),
    ObjectNotFound {
        number: u64,
        generation: u16,
    },
    TypeMismatch {
        expected: &'static str,
        actual: &'static str,
    },
    PageNotFound(usize),
    RecursionLimitExceeded,
    CircularReference(String),
    MalformedInput(String),
    UnsupportedCompositeMapping(String),
    CffDetectedUnsupported,
    Cff2DetectedUnsupported,
    UnknownFontProgram,
    MalformedSignature(String),
    EncryptedDocumentUnsupported(String),
    SignatureMutationUnsupported(String),
    AmbiguousFieldGraph(String),
    PageOperation(String),
    PageResourceLimit(String),
    UnsupportedPageDependency(String),
    PartialFieldImport(String),
    ExcludedPageTarget(String),
    UnsupportedFontEncoding(String),
    UnsupportedComplexScript(String),
    UnsupportedLayout(String),
    TargetTextNotFound(String),
    TextEditRefused(String),
    UnsupportedImageFormat(String),
    ImageNotFound(String),
    NestedFormXObjectRefusal(String),
    VectorGraphicNotFound(String),
    VectorEditRefused(String),
    UnsupportedVectorOperator(String),
    ComplexClippingRefusal(String),
    Serialization(String),
    Compression(String),
    InvalidObject(String),
    InvalidOperation(String),
}

impl fmt::Display for PdfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEof => write!(f, "Unexpected end of file while parsing PDF"),
            Self::InvalidToken(msg) => write!(f, "Invalid PDF token: {msg}"),
            Self::InvalidSyntax(msg) => write!(f, "Invalid PDF syntax: {msg}"),
            Self::InvalidHeader => write!(f, "Invalid or missing PDF header signature (%PDF-)"),
            Self::InvalidXref(msg) => write!(f, "Invalid cross-reference table/stream: {msg}"),
            Self::UnrecoverableXref(msg) => {
                write!(f, "XREF_STATUS_UNRECOVERABLE: {msg}")
            }
            Self::RecoveredXrefExport(msg) => {
                write!(f, "XREF_RECOVERED_EXPORT_REFUSED: {msg}")
            }
            Self::ObjectNotFound { number, generation } => {
                write!(
                    f,
                    "Indirect object {number} {generation} R not found in document"
                )
            }
            Self::TypeMismatch { expected, actual } => {
                write!(
                    f,
                    "PDF object type mismatch: expected {expected}, found {actual}"
                )
            }
            Self::PageNotFound(idx) => write!(f, "Page index {idx} out of range"),
            Self::RecursionLimitExceeded => write!(f, "Parser recursion depth limit exceeded"),
            Self::CircularReference(msg) => write!(f, "Circular reference detected: {msg}"),
            Self::MalformedInput(msg) => write!(f, "Malformed PDF data: {msg}"),
            Self::UnsupportedCompositeMapping(msg) => {
                write!(f, "UNSUPPORTED_COMPOSITE_MAPPING: {msg}")
            }
            Self::CffDetectedUnsupported => write!(f, "CFF_DETECTED_UNSUPPORTED"),
            Self::Cff2DetectedUnsupported => write!(f, "CFF2_DETECTED_UNSUPPORTED"),
            Self::UnknownFontProgram => write!(f, "UNKNOWN_FONT_PROGRAM"),
            Self::MalformedSignature(msg) => write!(f, "SIGNED_STRUCTURE_MALFORMED: {msg}"),
            Self::EncryptedDocumentUnsupported(msg) => {
                write!(f, "ENCRYPTED_DOCUMENT_MUTATION_UNSUPPORTED: {msg}")
            }
            Self::SignatureMutationUnsupported(msg) => {
                write!(f, "SIGNATURE_MUTATION_UNSUPPORTED: {msg}")
            }
            Self::AmbiguousFieldGraph(msg) => write!(f, "AMBIGUOUS_FIELD_GRAPH: {msg}"),
            Self::PageOperation(msg) => write!(f, "PAGE_OPERATION_INVALID: {msg}"),
            Self::PageResourceLimit(msg) => write!(f, "PAGE_OPERATION_LIMIT: {msg}"),
            Self::UnsupportedPageDependency(msg) => {
                write!(f, "UNSUPPORTED_PAGE_DEPENDENCY: {msg}")
            }
            Self::PartialFieldImport(msg) => write!(f, "PARTIAL_FIELD_IMPORT_REFUSED: {msg}"),
            Self::ExcludedPageTarget(msg) => write!(f, "EXCLUDED_PAGE_TARGET: {msg}"),
            Self::UnsupportedFontEncoding(msg) => {
                write!(f, "UNSUPPORTED_FONT_ENCODING: {msg}")
            }
            Self::UnsupportedComplexScript(msg) => {
                write!(f, "UNSUPPORTED_COMPLEX_SCRIPT: {msg}")
            }
            Self::UnsupportedLayout(msg) => write!(f, "UNSUPPORTED_LAYOUT: {msg}"),
            Self::TargetTextNotFound(msg) => write!(f, "TARGET_TEXT_NOT_FOUND: {msg}"),
            Self::TextEditRefused(msg) => write!(f, "TEXT_EDIT_REFUSED: {msg}"),
            Self::UnsupportedImageFormat(msg) => write!(f, "UNSUPPORTED_IMAGE_FORMAT: {msg}"),
            Self::ImageNotFound(msg) => write!(f, "IMAGE_NOT_FOUND: {msg}"),
            Self::NestedFormXObjectRefusal(msg) => {
                write!(f, "NESTED_FORM_XOBJECT_REFUSAL: {msg}")
            }
            Self::VectorGraphicNotFound(msg) => write!(f, "VECTOR_GRAPHIC_NOT_FOUND: {msg}"),
            Self::VectorEditRefused(msg) => write!(f, "VECTOR_EDIT_REFUSED: {msg}"),
            Self::UnsupportedVectorOperator(msg) => {
                write!(f, "UNSUPPORTED_VECTOR_OPERATOR: {msg}")
            }
            Self::ComplexClippingRefusal(msg) => write!(f, "COMPLEX_CLIPPING_REFUSAL: {msg}"),
            Self::Serialization(msg) => write!(f, "SERIALIZATION_ERROR: {msg}"),
            Self::Compression(msg) => write!(f, "COMPRESSION_ERROR: {msg}"),
            Self::InvalidObject(msg) => write!(f, "INVALID_OBJECT: {msg}"),
            Self::InvalidOperation(msg) => write!(f, "Invalid PDF operation: {msg}"),
        }
    }
}

impl std::error::Error for PdfError {}

pub type PdfResult<T> = Result<T, PdfError>;
