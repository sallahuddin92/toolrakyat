use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum PdfError {
    UnexpectedEof,
    InvalidToken(String),
    InvalidSyntax(String),
    InvalidHeader,
    InvalidXref(String),
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
            Self::InvalidOperation(msg) => write!(f, "Invalid PDF operation: {msg}"),
        }
    }
}

impl std::error::Error for PdfError {}

pub type PdfResult<T> = Result<T, PdfError>;
