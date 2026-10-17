use thiserror::Error;

/// Core error types for the `starpdf-app-core` application subsystem.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum AppCoreError {
    #[error("No active document session")]
    NoActiveSession,

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Document capability refused: {reason} (code: {code})")]
    Refused { code: String, reason: String },

    #[error("Invalid page index: {index} (page count: {total})")]
    InvalidPageIndex { index: u32, total: u32 },

    #[error("No element currently selected")]
    NoSelection,

    #[error("Selection type mismatch: expected {expected}, actual {actual}")]
    SelectionMismatch {
        expected: &'static str,
        actual: &'static str,
    },

    #[error("Target node not found: {0}")]
    TargetNotFound(String),

    #[error("Cannot undo: history stack is at earliest state")]
    CannotUndo,

    #[error("Cannot redo: history stack is at latest state")]
    CannotRedo,

    #[error("Underlying PDF engine error: {0}")]
    PdfError(String),

    #[error("Export failed: {0}")]
    ExportError(String),

    #[error("I/O error: {0}")]
    Io(String),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}

impl From<starpdf::error::PdfError> for AppCoreError {
    fn from(err: starpdf::error::PdfError) -> Self {
        Self::PdfError(err.to_string())
    }
}

pub type AppResult<T> = Result<T, AppCoreError>;
