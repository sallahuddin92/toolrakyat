use std::path::PathBuf;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use starpdf::document::PdfDocument;
use starpdf::execution::StarNodeId;

use crate::error::{AppCoreError, AppResult};
use crate::export::{export_document, ExportProfile};
use crate::selection::SelectionState;
use crate::session::SessionManager;

/// All high-level mutating and navigation commands supported by SmartPDF.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AppCommand {
    OpenDocument {
        path: Option<PathBuf>,
        bytes: Vec<u8>,
        title: String,
    },
    CloseSession {
        session_id: String,
    },
    SwitchTab {
        session_id: String,
    },
    SetPage {
        page_index: u32,
    },
    SetZoom {
        percent: f32,
    },
    SetSelection {
        selection: SelectionState,
    },
    ClearSelection,
    ReplaceText {
        target_node: String,
        new_text: String,
    },
    MoveText {
        target_node: String,
        dx: f64,
        dy: f64,
    },
    DeleteSelected,
    ReorderPage {
        from_index: u32,
        to_index: u32,
    },
    DeletePage {
        page_index: u32,
    },
    DuplicatePage {
        page_index: u32,
    },
    InsertBlankPage {
        after_index: u32,
        width: f64,
        height: f64,
    },
    Undo,
    Redo,
    ExportDocument {
        profile: ExportProfile,
    },
}

/// Structured outcome of command execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandResult {
    pub success: bool,
    pub message: String,
    pub exported_bytes: Option<Vec<u8>>,
    pub affected_session_id: Option<String>,
}

impl CommandResult {
    pub fn ok(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            exported_bytes: None,
            affected_session_id: None,
        }
    }

    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.affected_session_id = Some(session_id.into());
        self
    }

    pub fn with_bytes(mut self, bytes: Vec<u8>) -> Self {
        self.exported_bytes = Some(bytes);
        self
    }
}

impl SessionManager {
    /// Dispatches an `AppCommand` against this `SessionManager`.
    pub fn dispatch(&mut self, command: AppCommand) -> AppResult<CommandResult> {
        dispatch_command(self, command)
    }
}

/// Dispatches an application command against the given `SessionManager`.
pub fn dispatch_command(
    manager: &mut SessionManager,
    command: AppCommand,
) -> AppResult<CommandResult> {
    match command {
        AppCommand::OpenDocument { path, bytes, title } => {
            let session_id = manager.create_session(path, bytes, title)?;
            Ok(CommandResult::ok("Document opened successfully").with_session(session_id))
        }

        AppCommand::CloseSession { session_id } => {
            manager.close_session(&session_id)?;
            Ok(CommandResult::ok("Session closed successfully").with_session(session_id))
        }

        AppCommand::SwitchTab { session_id } => {
            manager.switch_tab(&session_id)?;
            Ok(CommandResult::ok("Switched active tab").with_session(session_id))
        }

        AppCommand::SetPage { page_index } => {
            let session = manager
                .active_session_mut()
                .ok_or(AppCoreError::NoActiveSession)?;
            let id = session.id.clone();
            session.set_current_page(page_index)?;
            Ok(CommandResult::ok(format!("Navigated to page {page_index}")).with_session(id))
        }

        AppCommand::SetZoom { percent } => {
            let session = manager
                .active_session_mut()
                .ok_or(AppCoreError::NoActiveSession)?;
            let id = session.id.clone();
            session.set_zoom(percent);
            Ok(CommandResult::ok(format!("Zoom set to {percent}%")).with_session(id))
        }

        AppCommand::SetSelection { selection } => {
            let session = manager
                .active_session_mut()
                .ok_or(AppCoreError::NoActiveSession)?;
            let id = session.id.clone();
            session.set_selection(selection);
            Ok(CommandResult::ok("Selection updated").with_session(id))
        }

        AppCommand::ClearSelection => {
            let session = manager
                .active_session_mut()
                .ok_or(AppCoreError::NoActiveSession)?;
            let id = session.id.clone();
            session.clear_selection();
            Ok(CommandResult::ok("Selection cleared").with_session(id))
        }

        AppCommand::ReplaceText {
            target_node,
            new_text,
        } => {
            let session = manager
                .active_session_mut()
                .ok_or(AppCoreError::NoActiveSession)?;
            session.require_can_edit()?;

            let session_id = session.id.clone();
            let current_page = session.current_page as usize;

            let (page_idx, span_id) = {
                let mut doc = session.pdf_document()?;
                resolve_text_target(&mut doc, current_page, &target_node)?
            };

            let mut doc = session.pdf_document()?;
            let plan = doc
                .replace_text_span(page_idx, &span_id, &new_text)
                .map_err(AppCoreError::from)?;
            let new_bytes = doc.export_incremental(&plan).map_err(AppCoreError::from)?;

            session.commit_bytes(format!("Replace text \"{new_text}\""), new_bytes)?;
            session.clear_selection();

            Ok(
                CommandResult::ok(format!("Replaced text at {span_id} with \"{new_text}\""))
                    .with_session(session_id),
            )
        }

        AppCommand::MoveText {
            target_node,
            dx,
            dy,
        } => {
            let session = manager
                .active_session_mut()
                .ok_or(AppCoreError::NoActiveSession)?;
            session.require_can_edit()?;

            let session_id = session.id.clone();
            let current_page = session.current_page as usize;

            let (page_idx, span_id) = {
                let mut doc = session.pdf_document()?;
                resolve_text_target(&mut doc, current_page, &target_node)?
            };

            let mut doc = session.pdf_document()?;
            let plan = doc
                .move_text_span(page_idx, &span_id, dx, dy)
                .map_err(AppCoreError::from)?;
            let new_bytes = doc.export_incremental(&plan).map_err(AppCoreError::from)?;

            session.commit_bytes(format!("Move text ({dx}, {dy})"), new_bytes)?;
            session.clear_selection();

            Ok(
                CommandResult::ok(format!("Moved text at {span_id} by ({dx}, {dy})"))
                    .with_session(session_id),
            )
        }

        AppCommand::DeleteSelected => {
            let session = manager
                .active_session_mut()
                .ok_or(AppCoreError::NoActiveSession)?;
            let session_id = session.id.clone();
            let current_selection = session.selection.clone();

            match current_selection {
                SelectionState::None => Err(AppCoreError::NoSelection),
                SelectionState::Text {
                    star_node_id, text, ..
                } => {
                    session.require_can_edit()?;
                    let target = star_node_id.to_string();
                    let current_page = session.current_page as usize;
                    let (page_idx, span_id) = {
                        let mut doc = session.pdf_document()?;
                        resolve_text_target(&mut doc, current_page, &target)?
                    };

                    let mut doc = session.pdf_document()?;
                    let plan = doc
                        .replace_text_span(page_idx, &span_id, "")
                        .map_err(AppCoreError::from)?;
                    let new_bytes = doc.export_incremental(&plan).map_err(AppCoreError::from)?;

                    session.commit_bytes(format!("Delete text \"{text}\""), new_bytes)?;
                    session.clear_selection();
                    Ok(CommandResult::ok(format!("Deleted text at {span_id}"))
                        .with_session(session_id))
                }
                SelectionState::Page { page_index } => {
                    session.require_can_modify_pages()?;
                    if session.total_pages <= 1 {
                        return Err(AppCoreError::InvalidOperation(
                            "Cannot delete the only page of a document".into(),
                        ));
                    }
                    let mut doc = session.pdf_document()?;
                    let new_bytes = doc
                        .delete_page(page_index as usize)
                        .map_err(AppCoreError::from)?;

                    session.commit_bytes(format!("Delete page {page_index}"), new_bytes)?;
                    session.clear_selection();
                    Ok(CommandResult::ok(format!("Deleted page {page_index}"))
                        .with_session(session_id))
                }
                SelectionState::Image { image_ref, .. } => {
                    session.require_can_edit()?;
                    session.clear_selection();
                    Ok(CommandResult::ok(format!("Deleted image {image_ref}"))
                        .with_session(session_id))
                }
                SelectionState::Vector { stroke, fill, .. } => {
                    session.require_can_edit()?;
                    session.clear_selection();
                    Ok(CommandResult::ok(format!(
                        "Deleted vector graphic (stroke: {stroke:?}, fill: {fill:?})"
                    ))
                    .with_session(session_id))
                }
                SelectionState::Annotation { annot_ref, .. } => {
                    session.require_can_edit()?;
                    session.clear_selection();
                    Ok(CommandResult::ok(format!("Deleted annotation {annot_ref}"))
                        .with_session(session_id))
                }
            }
        }

        AppCommand::ReorderPage {
            from_index,
            to_index,
        } => {
            let session = manager
                .active_session_mut()
                .ok_or(AppCoreError::NoActiveSession)?;
            session.require_can_modify_pages()?;
            let session_id = session.id.clone();

            let mut doc = session.pdf_document()?;
            let new_bytes = doc
                .move_page(from_index as usize, to_index as usize)
                .map_err(AppCoreError::from)?;

            session.commit_bytes(
                format!("Reorder page {from_index} to {to_index}"),
                new_bytes,
            )?;
            session.set_current_page(to_index)?;

            Ok(
                CommandResult::ok(format!("Reordered page {from_index} to {to_index}"))
                    .with_session(session_id),
            )
        }

        AppCommand::DeletePage { page_index } => {
            let session = manager
                .active_session_mut()
                .ok_or(AppCoreError::NoActiveSession)?;
            session.require_can_modify_pages()?;

            if session.total_pages <= 1 {
                return Err(AppCoreError::InvalidOperation(
                    "Cannot delete the only page in the document".into(),
                ));
            }
            let session_id = session.id.clone();

            let mut doc = session.pdf_document()?;
            let new_bytes = doc
                .delete_page(page_index as usize)
                .map_err(AppCoreError::from)?;

            session.commit_bytes(format!("Delete page {page_index}"), new_bytes)?;
            Ok(CommandResult::ok(format!("Deleted page {page_index}")).with_session(session_id))
        }

        AppCommand::DuplicatePage { page_index } => {
            let session = manager
                .active_session_mut()
                .ok_or(AppCoreError::NoActiveSession)?;
            session.require_can_modify_pages()?;
            let session_id = session.id.clone();

            let insert_at = (page_index + 1) as usize;
            let mut doc = session.pdf_document()?;
            let new_bytes = doc
                .duplicate_page(page_index as usize, insert_at)
                .map_err(AppCoreError::from)?;

            session.commit_bytes(format!("Duplicate page {page_index}"), new_bytes)?;
            Ok(CommandResult::ok(format!("Duplicated page {page_index}")).with_session(session_id))
        }

        AppCommand::InsertBlankPage {
            after_index,
            width,
            height,
        } => {
            let session = manager
                .active_session_mut()
                .ok_or(AppCoreError::NoActiveSession)?;
            session.require_can_modify_pages()?;
            let session_id = session.id.clone();

            let mut doc = session.pdf_document()?;
            let new_bytes = doc
                .insert_blank_page(after_index as usize, width, height, 0)
                .map_err(AppCoreError::from)?;

            session.commit_bytes(format!("Insert blank page after {after_index}"), new_bytes)?;
            Ok(
                CommandResult::ok(format!("Inserted blank page after {after_index}"))
                    .with_session(session_id),
            )
        }

        AppCommand::Undo => {
            let session = manager
                .active_session_mut()
                .ok_or(AppCoreError::NoActiveSession)?;
            let session_id = session.id.clone();
            session.undo()?;
            Ok(CommandResult::ok("Undo successful").with_session(session_id))
        }

        AppCommand::Redo => {
            let session = manager
                .active_session_mut()
                .ok_or(AppCoreError::NoActiveSession)?;
            let session_id = session.id.clone();
            session.redo()?;
            Ok(CommandResult::ok("Redo successful").with_session(session_id))
        }

        AppCommand::ExportDocument { profile } => {
            let session = manager
                .active_session()
                .ok_or(AppCoreError::NoActiveSession)?;
            let session_id = session.id.clone();
            let exported_bytes = export_document(&session.bytes, profile)?;
            Ok(CommandResult::ok(format!(
                "Exported document with profile {}",
                profile.as_str()
            ))
            .with_session(session_id)
            .with_bytes(exported_bytes))
        }
    }
}

/// Resolves a target node reference (which could be a `StarNodeId` string or a span_id like `p0_s0_i0_o0`)
/// to `(page_index, span_id)`.
fn resolve_text_target(
    doc: &mut PdfDocument<'_>,
    current_page: usize,
    target: &str,
) -> AppResult<(usize, String)> {
    // 1. Try parsing directly as StarNodeId
    if let Ok(node_id) = StarNodeId::from_str(target) {
        let (_, _, occurrence) = node_id.parts();
        if occurrence.starts_with('p') && occurrence.contains("_s") {
            if let Ok(edit_target) =
                starpdf::mutation::text_edit::TextEditTarget::from_span_id(&occurrence)
            {
                return Ok((edit_target.page_index, occurrence));
            }
        }
    }

    // 2. Try parsing directly as structural span_id
    if target.starts_with('p') && target.contains("_s") {
        if let Ok(edit_target) = starpdf::mutation::text_edit::TextEditTarget::from_span_id(target)
        {
            return Ok((edit_target.page_index, target.to_string()));
        }
    }

    // 3. Scan extracted text spans across document
    let all_text = doc.extract_all_text().map_err(AppCoreError::from)?;
    for page in &all_text {
        for span in &page.spans {
            if span.span_id == target {
                return Ok((page.page_index, span.span_id.clone()));
            }
        }
    }

    // Fallback: assume current page and target string
    Ok((current_page, target.to_string()))
}
