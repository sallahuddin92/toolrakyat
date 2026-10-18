use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use starpdf::document::PdfDocument;
use starpdf::execution::StarExecutionState;
use starpdf::security::{CapabilityDecision, DocumentCapabilities, DocumentSecurityInfo};
use uuid::Uuid;

use crate::editing::TextEditingSession;
use crate::error::{AppCoreError, AppResult};
use crate::history::HistoryStack;
use crate::selection::SelectionState;

/// Type alias aligning with canonical project architecture.
pub type StarSecurityInfo = DocumentSecurityInfo;

/// Viewport zoom mode.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ZoomMode {
    FitWidth,
    FitPage,
    ActualSize,
    Custom(f32),
}

/// Viewport zoom configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZoomLevel {
    pub percent: f32,
    pub mode: ZoomMode,
}

impl Default for ZoomLevel {
    fn default() -> Self {
        Self {
            percent: 100.0,
            mode: ZoomMode::FitPage,
        }
    }
}

/// Data transfer object representing a document tab for the UI layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabInfo {
    pub session_id: String,
    pub title: String,
    pub is_active: bool,
    pub is_dirty: bool,
    pub page_count: u32,
}

/// Authoritative document session representing an open PDF document in memory.
pub struct DocumentSession {
    pub id: String,
    pub title: String,
    pub file_path: Option<PathBuf>,
    pub bytes: Vec<u8>,
    pub current_page: u32,
    pub total_pages: u32,
    pub zoom: ZoomLevel,
    pub selection: SelectionState,
    pub history: HistoryStack,
    pub editing_session: Option<TextEditingSession>,
    pub security_info: Option<StarSecurityInfo>,
    pub capabilities: DocumentCapabilities,
    pub execution_state: StarExecutionState,
}

impl DocumentSession {
    /// Opens and initializes a new document session from raw PDF bytes.
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        file_path: Option<PathBuf>,
        bytes: Vec<u8>,
    ) -> AppResult<Self> {
        let id_str = id.into();
        let title_str = title.into();

        // 1. Verify and parse document via starpdf
        let mut doc = PdfDocument::from_bytes(&bytes).map_err(AppCoreError::from)?;
        let total_pages = u32::try_from(doc.page_count().map_err(AppCoreError::from)?).unwrap_or(1);
        let sec_info = doc.security_info().ok();
        let capabilities = sec_info.as_ref().map_or_else(
            || DocumentCapabilities {
                can_view: CapabilityDecision {
                    allowed: true,
                    reason: "UNENCRYPTED_STANDARD",
                },
                can_edit: CapabilityDecision {
                    allowed: true,
                    reason: "UNENCRYPTED_STANDARD",
                },
                can_extract: CapabilityDecision {
                    allowed: true,
                    reason: "UNENCRYPTED_STANDARD",
                },
                can_annotate: CapabilityDecision {
                    allowed: true,
                    reason: "UNENCRYPTED_STANDARD",
                },
                can_modify_pages: CapabilityDecision {
                    allowed: true,
                    reason: "UNENCRYPTED_STANDARD",
                },
            },
            |s| s.capabilities(true),
        );

        if !capabilities.can_view.allowed {
            return Err(AppCoreError::Refused {
                code: "CANNOT_VIEW".into(),
                reason: capabilities.can_view.reason.to_string(),
            });
        }

        let execution_state = StarExecutionState::from_document(&doc, 0);
        let history = HistoryStack::new(bytes.clone(), HistoryStack::DEFAULT_MAX_DEPTH);

        Ok(Self {
            id: id_str,
            title: title_str,
            file_path,
            bytes,
            current_page: 0,
            total_pages,
            zoom: ZoomLevel::default(),
            selection: SelectionState::None,
            history,
            editing_session: None,
            security_info: sec_info,
            capabilities,
            execution_state,
        })
    }

    /// Instantiates an active `PdfDocument` engine view over the current authoritative bytes.
    pub fn pdf_document(&self) -> AppResult<PdfDocument<'_>> {
        PdfDocument::from_bytes(&self.bytes).map_err(AppCoreError::from)
    }

    /// Compiles the specified 0-based page into typed Render IR.
    pub fn compile_render_page(&self, page: u32) -> AppResult<starpdf::render::RenderPage> {
        let mut doc = PdfDocument::from_bytes(&self.bytes).map_err(AppCoreError::from)?;
        doc.compile_render_page(page as usize).map_err(AppCoreError::from)
    }

    /// Renders the specified 0-based page to an RGBA pixmap at the given scale factor.
    pub fn render_page(&self, page: u32, scale: f32) -> AppResult<starpdf::render::RenderedPage> {
        let mut doc = PdfDocument::from_bytes(&self.bytes).map_err(AppCoreError::from)?;
        doc.render_page(page as usize, scale).map_err(AppCoreError::from)
    }

    /// Whether the document currently has unsaved modifications.
    pub fn is_dirty(&self) -> bool {
        self.history.is_dirty()
    }

    /// Sets the current active page (0-based index).
    pub fn set_current_page(&mut self, page: u32) -> AppResult<()> {
        if page >= self.total_pages {
            return Err(AppCoreError::InvalidPageIndex {
                index: page,
                total: self.total_pages,
            });
        }
        self.current_page = page;
        Ok(())
    }

    /// Sets viewport zoom percent.
    pub fn set_zoom(&mut self, percent: f32) {
        self.zoom.percent = percent.clamp(10.0, 1000.0);
        self.zoom.mode = ZoomMode::Custom(self.zoom.percent);
    }

    /// Updates active selection state.
    pub fn set_selection(&mut self, selection: SelectionState) {
        self.selection = selection;
    }

    /// Clears active selection.
    pub fn clear_selection(&mut self) {
        self.selection = SelectionState::None;
        if let Some(session) = &mut self.editing_session {
            session.cancel();
        }
        self.editing_session = None;
    }

    /// Pre-check: rejects if editing content is forbidden by security / capabilities.
    pub fn require_can_edit(&self) -> AppResult<()> {
        if !self.capabilities.can_edit.allowed {
            return Err(AppCoreError::Refused {
                code: "MUTATION_REFUSED".into(),
                reason: self.capabilities.can_edit.reason.to_string(),
            });
        }
        Ok(())
    }

    /// Pre-check: rejects if page manipulation is forbidden by security / capabilities.
    pub fn require_can_modify_pages(&self) -> AppResult<()> {
        if !self.capabilities.can_modify_pages.allowed {
            return Err(AppCoreError::Refused {
                code: "PAGE_MUTATION_REFUSED".into(),
                reason: self.capabilities.can_modify_pages.reason.to_string(),
            });
        }
        Ok(())
    }

    /// Commits mutated bytes to document state, refreshes metadata, and updates history.
    pub fn commit_bytes(
        &mut self,
        description: impl Into<String>,
        new_bytes: Vec<u8>,
    ) -> AppResult<()> {
        let mut doc = PdfDocument::from_bytes(&new_bytes).map_err(AppCoreError::from)?;
        let total_pages = u32::try_from(doc.page_count().map_err(AppCoreError::from)?).unwrap_or(1);
        let sec_info = doc.security_info().ok();
        let capabilities = sec_info
            .as_ref()
            .map(|s| s.capabilities(true))
            .unwrap_or_else(|| self.capabilities.clone());

        self.total_pages = total_pages;
        if self.current_page >= self.total_pages {
            self.current_page = self.total_pages.saturating_sub(1);
        }
        self.security_info = sec_info;
        self.capabilities = capabilities;

        let revision = (self.history.current_index() + 1) as u64;
        self.execution_state = StarExecutionState::from_document(&doc, revision);

        self.history.push(description, new_bytes.clone());
        self.bytes = new_bytes;
        Ok(())
    }

    /// Restores previous document state from undo history.
    pub fn undo(&mut self) -> AppResult<bool> {
        self.require_can_edit()?;
        if let Some(restored) = self.history.undo() {
            let bytes = restored.to_vec();
            let mut doc = PdfDocument::from_bytes(&bytes).map_err(AppCoreError::from)?;
            self.total_pages =
                u32::try_from(doc.page_count().map_err(AppCoreError::from)?).unwrap_or(1);
            if self.current_page >= self.total_pages {
                self.current_page = self.total_pages.saturating_sub(1);
            }
            self.bytes = bytes;
            self.clear_selection();
            Ok(true)
        } else {
            Err(AppCoreError::CannotUndo)
        }
    }

    /// Restores undone document state from redo history.
    pub fn redo(&mut self) -> AppResult<bool> {
        self.require_can_edit()?;
        if let Some(restored) = self.history.redo() {
            let bytes = restored.to_vec();
            let mut doc = PdfDocument::from_bytes(&bytes).map_err(AppCoreError::from)?;
            self.total_pages =
                u32::try_from(doc.page_count().map_err(AppCoreError::from)?).unwrap_or(1);
            if self.current_page >= self.total_pages {
                self.current_page = self.total_pages.saturating_sub(1);
            }
            self.bytes = bytes;
            self.clear_selection();
            Ok(true)
        } else {
            Err(AppCoreError::CannotRedo)
        }
    }

    /// Determines the authoritative, accurate evidence confidence level for this session.
    pub fn evidence_level(&self) -> &'static str {
        let has_verified = self
            .execution_state
            .effective_object_index
            .values()
            .any(|o| {
                matches!(
                    o.evidence.level,
                    starpdf::execution::EvidenceLevel::Verified
                        | starpdf::execution::EvidenceLevel::RecoveredVerified
                )
            });

        if has_verified {
            "VERIFIED"
        } else if self.is_dirty() {
            "MUTATION_STAGED"
        } else if self
            .security_info
            .as_ref()
            .is_some_and(|s| s.encryption_state != starpdf::security::EncryptionState::NotEncrypted)
        {
            "RESTRICTED"
        } else {
            "LOCAL_PARSED"
        }
    }

    /// Generates tab DTO for UI presentation.
    pub fn to_tab_info(&self, is_active: bool) -> TabInfo {
        TabInfo {
            session_id: self.id.clone(),
            title: self.title.clone(),
            is_active,
            is_dirty: self.is_dirty(),
            page_count: self.total_pages,
        }
    }
}

/// Manages multiple document tabs and active session focus.
pub struct SessionManager {
    sessions: Vec<DocumentSession>,
    active_tab_index: Option<usize>,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            active_tab_index: None,
        }
    }

    /// Creates and switches to a new document session.
    pub fn create_session(
        &mut self,
        file_path: Option<PathBuf>,
        bytes: Vec<u8>,
        title: String,
    ) -> AppResult<String> {
        let id = Uuid::new_v4().to_string();
        let session = DocumentSession::new(id.clone(), title, file_path, bytes)?;
        self.sessions.push(session);
        self.active_tab_index = Some(self.sessions.len() - 1);
        Ok(id)
    }

    /// Closes a document session by ID, updating active tab focus.
    pub fn close_session(&mut self, session_id: &str) -> AppResult<()> {
        let index = self
            .sessions
            .iter()
            .position(|s| s.id == session_id)
            .ok_or_else(|| AppCoreError::SessionNotFound(session_id.to_string()))?;

        self.sessions.remove(index);

        if self.sessions.is_empty() {
            self.active_tab_index = None;
        } else if let Some(active) = self.active_tab_index {
            if active >= self.sessions.len() || active == index {
                self.active_tab_index = Some(self.sessions.len().saturating_sub(1));
            } else if active > index {
                self.active_tab_index = Some(active - 1);
            }
        }

        Ok(())
    }

    /// Switches active tab to the specified session ID.
    pub fn switch_tab(&mut self, session_id: &str) -> AppResult<()> {
        let index = self
            .sessions
            .iter()
            .position(|s| s.id == session_id)
            .ok_or_else(|| AppCoreError::SessionNotFound(session_id.to_string()))?;

        self.active_tab_index = Some(index);
        Ok(())
    }

    /// Switches active tab to the specified 0-based tab index.
    pub fn switch_tab_index(&mut self, index: usize) -> AppResult<()> {
        if index >= self.sessions.len() {
            return Err(AppCoreError::InvalidOperation(format!(
                "Tab index {index} out of bounds (total tabs: {})",
                self.sessions.len()
            )));
        }
        self.active_tab_index = Some(index);
        Ok(())
    }

    /// Returns a reference to the active document session, if any.
    pub fn active_session(&self) -> Option<&DocumentSession> {
        self.active_tab_index.and_then(|idx| self.sessions.get(idx))
    }

    /// Returns a mutable reference to the active document session, if any.
    pub fn active_session_mut(&mut self) -> Option<&mut DocumentSession> {
        self.active_tab_index
            .and_then(|idx| self.sessions.get_mut(idx))
    }

    /// Returns a reference to a session by ID.
    pub fn get_session(&self, session_id: &str) -> Option<&DocumentSession> {
        self.sessions.iter().find(|s| s.id == session_id)
    }

    /// Returns a mutable reference to a session by ID.
    pub fn get_session_mut(&mut self, session_id: &str) -> Option<&mut DocumentSession> {
        self.sessions.iter_mut().find(|s| s.id == session_id)
    }

    /// Returns list of TabInfo objects for UI tab rendering.
    pub fn tabs(&self) -> Vec<TabInfo> {
        self.sessions
            .iter()
            .enumerate()
            .map(|(idx, s)| s.to_tab_info(Some(idx) == self.active_tab_index))
            .collect()
    }

    /// Returns current number of open document sessions.
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Returns active tab index.
    pub fn active_tab_index(&self) -> Option<usize> {
        self.active_tab_index
    }

    /// Renders the currently active page of the active session.
    pub fn render_active_page(&self, scale: f32) -> AppResult<starpdf::render::RenderedPage> {
        let session = self.active_session().ok_or(AppCoreError::NoActiveSession)?;
        session.render_page(session.current_page, scale)
    }
}
