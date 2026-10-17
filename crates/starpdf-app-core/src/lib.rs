//! `starpdf-app-core`: Authoritative application-core engine for SmartPDF.
//!
//! Provides single-authority document session management, multi-tab coordination,
//! undo/redo delta history, provenance-preserving selection model, capability gating,
//! and command dispatch.

#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::must_use_candidate,
    clippy::missing_errors_doc,
    clippy::similar_names,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::struct_excessive_bools,
    clippy::format_push_string,
    clippy::doc_markdown,
    clippy::wildcard_imports,
    clippy::return_self_not_must_use,
    clippy::map_unwrap_or,
    clippy::unnecessary_map_or
)]

pub mod command;
pub mod editing;
pub mod error;
pub mod export;
pub mod history;
pub mod selection;
pub mod session;

// Re-export core execution types from starpdf
pub use starpdf::execution::{EvidenceLevel, RenderNodeId, StarExecutionState, StarNodeId};

// Re-export app-core types
pub use command::{dispatch_command, AppCommand, CommandResult};
pub use editing::TextEditingSession;
pub use error::{AppCoreError, AppResult};
pub use export::{export_document, ExportProfile};
pub use history::{HistoryRecord, HistoryStack};
pub use selection::{FontInfo, Rect, SelectionState};
pub use session::{
    DocumentSession, SessionManager, StarSecurityInfo, TabInfo, ZoomLevel, ZoomMode,
};
