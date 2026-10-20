//! Standardized UI-Engine Adapter for SmartPDF Desktop egui.
//! Wraps starpdf-app-core SessionManager and AppCommand dispatching with
//! authoritative state synchronization, selection lifecycle, and error propagation.

use starpdf::execution::StarNodeId;
use starpdf::syntax::object::ObjectRef;
use starpdf_app_core::command::{AppCommand, CommandResult};
use starpdf_app_core::export::ExportProfile;
use starpdf_app_core::selection::{FontInfo, Rect as CoreRect, SelectionState};
use starpdf_app_core::session::SessionManager;
use std::path::{Path, PathBuf};

/// Default sample 2x2 JPEG bytes for image operations.
pub static DEFAULT_SAMPLE_JPEG: &[u8] = &[
    0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x01, 0x00, 0x48,
    0x00, 0x48, 0x00, 0x00, 0xFF, 0xDB, 0x00, 0x43, 0x00, 0x08, 0x06, 0x06, 0x07, 0x06, 0x05, 0x08,
    0x07, 0x07, 0x07, 0x09, 0x09, 0x08, 0x0A, 0x0C, 0x14, 0x0D, 0x0C, 0x0B, 0x0B, 0x0C, 0x19, 0x12,
    0x13, 0x0F, 0x14, 0x1D, 0x1A, 0x1F, 0x1E, 0x1D, 0x1A, 0x1C, 0x1C, 0x20, 0x24, 0x2E, 0x27, 0x20,
    0x22, 0x2C, 0x23, 0x1C, 0x1C, 0x28, 0x37, 0x29, 0x2C, 0x30, 0x31, 0x34, 0x34, 0x34, 0x1F, 0x27,
    0x39, 0x3D, 0x38, 0x32, 0x3C, 0x2E, 0x33, 0x34, 0x32, 0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x00, 0x02,
    0x00, 0x02, 0x01, 0x01, 0x11, 0x00, 0xFF, 0xC4, 0x00, 0x1F, 0x00, 0x00, 0x01, 0x05, 0x01, 0x01,
    0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04,
    0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0xFF, 0xDA, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x3F,
    0x00, 0xBF, 0x80, 0xFF, 0xD9,
];

/// The standardized UI-to-engine adapter.
/// Acts as the single point of contact between egui presentation and StarPDF engine.
pub struct EguiEngineAdapter {
    pub manager: SessionManager,
    pub status_message: String,
    pub error_banner: Option<String>,
    pub editing_text_buffer: String,
    pub editing_annotation_buffer: String,
    pub editing_form_value_buffer: String,
    pub needs_render_invalidation: bool,
    pub needs_thumbnail_invalidation: bool,
}

impl Default for EguiEngineAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl EguiEngineAdapter {
    pub fn new() -> Self {
        Self {
            manager: SessionManager::new(),
            status_message: "Ready".to_string(),
            error_banner: None,
            editing_text_buffer: String::new(),
            editing_annotation_buffer: String::new(),
            editing_form_value_buffer: String::new(),
            needs_render_invalidation: false,
            needs_thumbnail_invalidation: false,
        }
    }

    /// Dispatches an authoritative AppCommand with centralized error handling,
    /// notification, and cache invalidation.
    pub fn dispatch(&mut self, command: AppCommand) -> Result<CommandResult, String> {
        let is_page_structure_mutation = matches!(
            command,
            AppCommand::DeletePage { .. }
                | AppCommand::DuplicatePage { .. }
                | AppCommand::InsertBlankPage { .. }
                | AppCommand::ReorderPage { .. }
        );

        let is_mutating = is_page_structure_mutation
            || matches!(
                command,
                AppCommand::ReplaceText { .. }
                    | AppCommand::MoveText { .. }
                    | AppCommand::DeleteSelected
                    | AppCommand::AddAnnotation { .. }
                    | AppCommand::UpdateAnnotation { .. }
                    | AppCommand::SetFormField { .. }
                    | AppCommand::AddImage { .. }
                    | AppCommand::ReplaceImage { .. }
                    | AppCommand::MoveElement { .. }
                    | AppCommand::ResizeElement { .. }
                    | AppCommand::Undo
                    | AppCommand::Redo
                    | AppCommand::OpenDocument { .. }
                    | AppCommand::CloseSession { .. }
                    | AppCommand::SwitchTab { .. }
                    | AppCommand::SetPage { .. }
            );

        match self.manager.dispatch(command) {
            Ok(res) => {
                self.status_message = res.message.clone();
                self.error_banner = None;
                if is_mutating {
                    self.needs_render_invalidation = true;
                }
                if is_page_structure_mutation {
                    self.needs_thumbnail_invalidation = true;
                }
                self.sync_selection_buffers();
                Ok(res)
            }
            Err(err) => {
                let err_msg = format!("{err}");
                self.status_message = format!("Refused: {err_msg}");
                self.error_banner = Some(err_msg.clone());
                Err(err_msg)
            }
        }
    }

    /// Synchronizes internal editing text/form/annotation buffers from current selection.
    pub fn sync_selection_buffers(&mut self) {
        if let Some(session) = self.manager.active_session() {
            match &session.selection {
                SelectionState::Text { text, .. } => {
                    self.editing_text_buffer = text.clone();
                }
                SelectionState::Annotation { contents, .. } => {
                    self.editing_annotation_buffer = contents.clone().unwrap_or_default();
                }
                SelectionState::FormField { value, .. } => {
                    self.editing_form_value_buffer = value.clone();
                }
                _ => {}
            }
        }
    }

    // --- WORKFLOW 1: File Operations ---

    pub fn open_file_path(&mut self, path: impl AsRef<Path>) -> Result<(), String> {
        let p = path.as_ref();
        match std::fs::read(p) {
            Ok(bytes) => {
                let title = p
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "document.pdf".to_string());
                let res = self.dispatch(AppCommand::OpenDocument {
                    path: Some(p.to_path_buf()),
                    bytes,
                    title,
                })?;
                self.needs_thumbnail_invalidation = true;
                self.status_message = res.message;
                Ok(())
            }
            Err(e) => {
                let err = format!("Cannot read file {:?}: {e}", p);
                self.error_banner = Some(err.clone());
                self.status_message = err.clone();
                Err(err)
            }
        }
    }

    pub fn close_session(&mut self, session_id: &str) -> Result<(), String> {
        self.dispatch(AppCommand::CloseSession {
            session_id: session_id.to_string(),
        })?;
        self.needs_thumbnail_invalidation = true;
        Ok(())
    }

    pub fn switch_tab(&mut self, session_id: &str) -> Result<(), String> {
        self.auto_commit_draft_if_needed()?;
        self.dispatch(AppCommand::SwitchTab {
            session_id: session_id.to_string(),
        })?;
        self.needs_thumbnail_invalidation = true;
        Ok(())
    }

    pub fn save_active_document(&mut self, dest: Option<PathBuf>) -> Result<PathBuf, String> {
        self.auto_commit_draft_if_needed()?;
        let path = match dest.or_else(|| {
            self.manager
                .active_session()
                .and_then(|s| s.file_path.clone())
        }) {
            Some(p) => p,
            None => {
                return Err("No destination path provided for save".to_string());
            }
        };

        self.dispatch(AppCommand::SaveDocument {
            destination_path: Some(path.clone()),
        })?;
        Ok(path)
    }

    pub fn export_active_document(
        &mut self,
        profile: ExportProfile,
        dest: PathBuf,
    ) -> Result<PathBuf, String> {
        self.auto_commit_draft_if_needed()?;
        let res = self.dispatch(AppCommand::ExportDocument { profile })?;
        if let Some(bytes) = res.exported_bytes {
            std::fs::write(&dest, bytes).map_err(|e| format!("Failed to write export: {e}"))?;
            self.status_message = format!("Exported successfully to {}", dest.display());
            Ok(dest)
        } else {
            Err("Engine did not return exported bytes".to_string())
        }
    }

    // --- WORKFLOW 2: Navigation & Zoom ---

    pub fn set_page(&mut self, page_index: u32) -> Result<(), String> {
        self.auto_commit_draft_if_needed()?;
        self.dispatch(AppCommand::SetPage { page_index })?;
        Ok(())
    }

    pub fn next_page(&mut self) -> Result<(), String> {
        if let Some(s) = self.manager.active_session() {
            if s.current_page + 1 < s.total_pages {
                return self.set_page(s.current_page + 1);
            }
        }
        Ok(())
    }

    pub fn prev_page(&mut self) -> Result<(), String> {
        if let Some(s) = self.manager.active_session() {
            if s.current_page > 0 {
                return self.set_page(s.current_page - 1);
            }
        }
        Ok(())
    }

    pub fn set_zoom(&mut self, percent: f32) -> Result<(), String> {
        let clamped = percent.clamp(25.0, 500.0);
        self.dispatch(AppCommand::SetZoom { percent: clamped })?;
        Ok(())
    }

    pub fn zoom_in(&mut self) -> Result<f32, String> {
        let cur = self
            .manager
            .active_session()
            .map(|s| s.zoom.percent)
            .unwrap_or(100.0);
        let next = (cur * 1.25).min(500.0);
        self.set_zoom(next)?;
        Ok(next)
    }

    pub fn zoom_out(&mut self) -> Result<f32, String> {
        let cur = self
            .manager
            .active_session()
            .map(|s| s.zoom.percent)
            .unwrap_or(100.0);
        let next = (cur / 1.25).max(25.0);
        self.set_zoom(next)?;
        Ok(next)
    }

    pub fn zoom_reset(&mut self) -> Result<f32, String> {
        self.set_zoom(100.0)?;
        Ok(100.0)
    }

    // --- WORKFLOW 3 & 4: Hit Testing, Selection & Text Editing ---

    /// Unified hit testing on the active document page.
    /// Evaluates text spans, images, annotations, and form fields in coordinate space.
    pub fn hit_test(&self, page_x: f64, page_y: f64, page_h: f64) -> Option<SelectionState> {
        let session = self.manager.active_session()?;
        let target_page = session.current_page;

        // 1. Text spans hit test (with 2pt horizontal & 3pt vertical tolerance)
        if let Ok(spans) = session.extract_page_text_spans(target_page) {
            if let Some(span) = spans.iter().find(|s| {
                page_x >= (s.x - 2.0)
                    && page_x <= (s.x + s.width + 2.0)
                    && page_y >= (s.y - 3.0)
                    && page_y <= (s.y + s.height + 3.0)
            }) {
                let star_node_id = StarNodeId::new("doc", ObjectRef::new(1, 0), &span.span_id)
                    .unwrap_or_else(|_| {
                        StarNodeId::new("doc", ObjectRef::new(1, 0), "span").unwrap()
                    });
                let canvas_y = (page_h - span.y - span.height).max(0.0);
                return Some(SelectionState::Text {
                    star_node_id,
                    render_node_id: None,
                    text: span.text.clone(),
                    bounds: CoreRect::new(
                        span.x,
                        canvas_y,
                        span.width.max(12.0),
                        span.height.max(14.0),
                    ),
                    font_info: Some(FontInfo::new(span.font_name.clone(), span.font_size)),
                    is_editable: span.is_editable,
                });
            }
        }

        // 2. Images hit test
        if let Ok(mut doc) = session.pdf_document() {
            if let Ok(images) = doc.enumerate_images(target_page as usize) {
                if let Some(img) = images.iter().find(|i| {
                    page_x >= i.rect[0]
                        && page_x <= i.rect[2]
                        && page_y >= i.rect[1]
                        && page_y <= i.rect[3]
                }) {
                    let canvas_y = (page_h - img.rect[3]).max(0.0);
                    let bounds = CoreRect::new(
                        img.rect[0],
                        canvas_y,
                        (img.rect[2] - img.rect[0]).abs().max(16.0),
                        (img.rect[3] - img.rect[1]).abs().max(16.0),
                    );
                    let star_node_id = StarNodeId::new("doc", img.object_ref, &img.image_id)
                        .unwrap_or_else(|_| {
                            StarNodeId::new("doc", ObjectRef::new(1, 0), "elem").unwrap()
                        });
                    return Some(SelectionState::Image {
                        star_node_id,
                        render_node_id: None,
                        image_ref: img.image_id.clone(),
                        bounds,
                        is_shared: false,
                    });
                }
            }
        }

        // 3. Annotations hit test
        if let Ok(mut doc) = session.pdf_document() {
            if let Ok(annots) = doc.page_annotations(target_page as usize) {
                if let Some(a) = annots.iter().find(|x| {
                    if format!("{:?}", x.subtype) == "Widget" {
                        return false;
                    }
                    let min_x = x.rect[0].min(x.rect[2]);
                    let max_x = x.rect[0].max(x.rect[2]);
                    let min_y = x.rect[1].min(x.rect[3]);
                    let max_y = x.rect[1].max(x.rect[3]);
                    page_x >= min_x && page_x <= max_x && page_y >= min_y && page_y <= max_y
                }) {
                    let min_x = a.rect[0].min(a.rect[2]);
                    let max_x = a.rect[0].max(a.rect[2]);
                    let max_y = a.rect[1].max(a.rect[3]);
                    let canvas_y = (page_h - max_y).max(0.0);
                    let bounds = CoreRect::new(
                        min_x,
                        canvas_y,
                        (max_x - min_x).abs().max(16.0),
                        (a.rect[3] - a.rect[1]).abs().max(16.0),
                    );
                    return Some(SelectionState::Annotation {
                        annot_ref: format!("{} 0 R", a.object_ref.number),
                        subtype: format!("{:?}", a.subtype),
                        contents: a.contents.clone(),
                        bounds,
                    });
                }
            }
        }

        // 4. Form fields hit test
        if let Ok(mut doc) = session.pdf_document() {
            if let Ok(fields) = doc.form_fields() {
                for f in &fields {
                    for w in &f.widgets {
                        let min_x = w.rect[0].min(w.rect[2]);
                        let max_x = w.rect[0].max(w.rect[2]);
                        let min_y = w.rect[1].min(w.rect[3]);
                        let max_y = w.rect[1].max(w.rect[3]);
                        if page_x >= min_x && page_x <= max_x && page_y >= min_y && page_y <= max_y
                        {
                            let type_str = match &f.field_type {
                                starpdf::forms::FieldType::Checkbox => "Checkbox",
                                starpdf::forms::FieldType::RadioButtonGroup => "RadioButtonGroup",
                                starpdf::forms::FieldType::Text { .. } => "Text",
                                starpdf::forms::FieldType::Choice { .. } => "Choice",
                                _ => "Other",
                            };
                            let val_str = match &f.value {
                                starpdf::forms::FieldValue::Text(s) => s.clone(),
                                starpdf::forms::FieldValue::Boolean(b) => {
                                    if *b {
                                        "true".into()
                                    } else {
                                        "false".into()
                                    }
                                }
                                starpdf::forms::FieldValue::Name(s) => s.clone(),
                                starpdf::forms::FieldValue::Choice(v) => {
                                    v.first().cloned().unwrap_or_default()
                                }
                                starpdf::forms::FieldValue::None => "".into(),
                            };
                            let canvas_y = (page_h - max_y).max(0.0);
                            let bounds = CoreRect::new(
                                min_x,
                                canvas_y,
                                (max_x - min_x).abs().max(16.0),
                                (max_y - min_y).abs().max(16.0),
                            );
                            return Some(SelectionState::FormField {
                                field_ref: format!("{} 0 R", f.object_ref.number),
                                field_name: f.partial_name.clone(),
                                field_type: type_str.to_string(),
                                value: val_str,
                                bounds,
                            });
                        }
                    }
                }
            }
        }

        None
    }

    pub fn select(&mut self, selection: SelectionState) -> Result<(), String> {
        self.auto_commit_draft_if_needed()?;
        self.dispatch(AppCommand::SetSelection { selection })?;
        Ok(())
    }

    pub fn clear_selection(&mut self) -> Result<(), String> {
        self.auto_commit_draft_if_needed()?;
        self.dispatch(AppCommand::ClearSelection)?;
        self.editing_text_buffer.clear();
        self.editing_annotation_buffer.clear();
        self.editing_form_value_buffer.clear();
        Ok(())
    }

    /// Whether the active text selection has uncommitted draft modifications.
    pub fn has_unsaved_text(&self) -> bool {
        if let Some(session) = self.manager.active_session() {
            if let SelectionState::Text { text, .. } = &session.selection {
                return text != &self.editing_text_buffer;
            }
        }
        false
    }

    /// Automatically commits any pending draft text changes before switching or clearing selection.
    /// Guarantees that if auto-commit fails, the draft text in `editing_text_buffer` is NEVER lost.
    pub fn auto_commit_draft_if_needed(&mut self) -> Result<bool, String> {
        if self.has_unsaved_text() {
            let draft = self.editing_text_buffer.clone();
            match self.commit_text_edit(&draft) {
                Ok(()) => Ok(true),
                Err(err) => {
                    self.editing_text_buffer = draft;
                    let msg = format!("Auto-commit draft text failed: {err}");
                    self.error_banner = Some(msg.clone());
                    self.status_message = format!("Refused: {msg}");
                    Err(msg)
                }
            }
        } else {
            Ok(false)
        }
    }

    pub fn commit_text_edit(&mut self, new_text: &str) -> Result<(), String> {
        let (target_node, is_single_line) = {
            let session = self.manager.active_session().ok_or("No active session")?;
            match &session.selection {
                SelectionState::Text {
                    star_node_id,
                    is_editable,
                    text,
                    ..
                } => {
                    if !is_editable {
                        return Err("Selected text is read-only (unsupported font encoding or complex layout)".to_string());
                    }
                    let (_, _, occ) = star_node_id.parts();
                    (occ, !text.contains('\n'))
                }
                _ => return Err("No text selected for editing".to_string()),
            }
        };

        let sanitized = if is_single_line {
            new_text.trim_end_matches(['\r', '\n']).to_string()
        } else {
            new_text.to_string()
        };

        self.dispatch(AppCommand::ReplaceText {
            target_node,
            new_text: sanitized,
        })?;
        Ok(())
    }

    pub fn cancel_text_edit(&mut self) {
        if let Some(session) = self.manager.active_session() {
            if let SelectionState::Text { text, .. } = &session.selection {
                self.editing_text_buffer = text.clone();
            }
        }
    }

    // --- WORKFLOW 5: Image Operations ---

    pub fn add_sample_image(
        &mut self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> Result<(), String> {
        let cur_page = self
            .manager
            .active_session()
            .map(|s| s.current_page)
            .unwrap_or(0);
        self.dispatch(AppCommand::AddImage {
            page_index: cur_page,
            image_bytes: DEFAULT_SAMPLE_JPEG.to_vec(),
            x,
            y,
            width,
            height,
        })?;
        Ok(())
    }

    pub fn add_image_bytes(
        &mut self,
        bytes: Vec<u8>,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> Result<(), String> {
        let cur_page = self
            .manager
            .active_session()
            .map(|s| s.current_page)
            .unwrap_or(0);
        self.dispatch(AppCommand::AddImage {
            page_index: cur_page,
            image_bytes: bytes,
            x,
            y,
            width,
            height,
        })?;
        Ok(())
    }

    pub fn replace_selected_image(&mut self, new_bytes: Vec<u8>) -> Result<(), String> {
        let (page_idx, image_ref) = {
            let session = self.manager.active_session().ok_or("No active session")?;
            match &session.selection {
                SelectionState::Image { image_ref, .. } => {
                    (session.current_page, image_ref.clone())
                }
                _ => return Err("No image selected to replace".to_string()),
            }
        };

        self.dispatch(AppCommand::ReplaceImage {
            page_index: page_idx,
            image_ref,
            image_bytes: new_bytes,
        })?;
        Ok(())
    }

    pub fn move_selected(&mut self, dx: f64, dy: f64) -> Result<(), String> {
        self.dispatch(AppCommand::MoveElement { dx, dy })?;
        Ok(())
    }

    pub fn resize_selected(&mut self, dw: f64, dh: f64) -> Result<(), String> {
        self.dispatch(AppCommand::ResizeElement { dw, dh })?;
        Ok(())
    }

    pub fn delete_selected(&mut self) -> Result<(), String> {
        self.dispatch(AppCommand::DeleteSelected)?;
        Ok(())
    }

    // --- WORKFLOW 6: Annotation Operations ---

    pub fn add_annotation(
        &mut self,
        subtype: &str,
        rect: [f64; 4],
        contents: Option<String>,
        color: Option<Vec<f64>>,
    ) -> Result<(), String> {
        let cur_page = self
            .manager
            .active_session()
            .map(|s| s.current_page)
            .unwrap_or(0);
        self.dispatch(AppCommand::AddAnnotation {
            page_index: cur_page,
            subtype: subtype.to_string(),
            rect,
            contents,
            color,
        })?;
        Ok(())
    }

    pub fn update_selected_annotation(&mut self, contents: Option<String>) -> Result<(), String> {
        let annot_ref = {
            let session = self.manager.active_session().ok_or("No active session")?;
            match &session.selection {
                SelectionState::Annotation { annot_ref, .. } => annot_ref.clone(),
                _ => return Err("No annotation selected to update".to_string()),
            }
        };

        self.dispatch(AppCommand::UpdateAnnotation {
            annot_ref,
            contents,
            rect: None,
        })?;
        Ok(())
    }

    // --- WORKFLOW 7: Form Operations ---

    pub fn toggle_selected_form_checkbox(&mut self) -> Result<(), String> {
        let field_name = {
            let session = self.manager.active_session().ok_or("No active session")?;
            match &session.selection {
                SelectionState::FormField { field_name, .. } => field_name.clone(),
                _ => return Err("No form field selected to toggle".to_string()),
            }
        };

        self.dispatch(AppCommand::SetFormField {
            field_name,
            value: "toggle".to_string(),
        })?;
        Ok(())
    }

    pub fn set_selected_form_field_value(&mut self, value: &str) -> Result<(), String> {
        let field_name = {
            let session = self.manager.active_session().ok_or("No active session")?;
            match &session.selection {
                SelectionState::FormField { field_name, .. } => field_name.clone(),
                _ => return Err("No form field selected to update".to_string()),
            }
        };

        self.dispatch(AppCommand::SetFormField {
            field_name,
            value: value.to_string(),
        })?;
        Ok(())
    }

    // --- WORKFLOW 8: Page Assembly Operations ---

    pub fn insert_blank_page(&mut self) -> Result<(), String> {
        let cur_page = self
            .manager
            .active_session()
            .map(|s| s.current_page)
            .unwrap_or(0);
        self.dispatch(AppCommand::InsertBlankPage {
            after_index: cur_page,
            width: 612.0,
            height: 792.0,
        })?;
        self.needs_thumbnail_invalidation = true;
        Ok(())
    }

    pub fn duplicate_current_page(&mut self) -> Result<(), String> {
        let cur_page = self
            .manager
            .active_session()
            .map(|s| s.current_page)
            .unwrap_or(0);
        self.dispatch(AppCommand::DuplicatePage {
            page_index: cur_page,
        })?;
        self.needs_thumbnail_invalidation = true;
        Ok(())
    }

    pub fn delete_current_page(&mut self) -> Result<(), String> {
        let cur_page = self
            .manager
            .active_session()
            .map(|s| s.current_page)
            .unwrap_or(0);
        self.dispatch(AppCommand::DeletePage {
            page_index: cur_page,
        })?;
        self.needs_thumbnail_invalidation = true;
        Ok(())
    }

    pub fn move_current_page_up(&mut self) -> Result<(), String> {
        let cur_page = self
            .manager
            .active_session()
            .map(|s| s.current_page)
            .unwrap_or(0);
        if cur_page > 0 {
            self.dispatch(AppCommand::ReorderPage {
                from_index: cur_page,
                to_index: cur_page - 1,
            })?;
            self.needs_thumbnail_invalidation = true;
        }
        Ok(())
    }

    pub fn move_current_page_down(&mut self) -> Result<(), String> {
        let (cur_page, total) = self
            .manager
            .active_session()
            .map(|s| (s.current_page, s.total_pages))
            .unwrap_or((0, 0));
        if cur_page + 1 < total {
            self.dispatch(AppCommand::ReorderPage {
                from_index: cur_page,
                to_index: cur_page + 1,
            })?;
            self.needs_thumbnail_invalidation = true;
        }
        Ok(())
    }

    // --- WORKFLOW 9: History Operations ---

    pub fn undo(&mut self) -> Result<(), String> {
        self.dispatch(AppCommand::Undo)?;
        self.needs_thumbnail_invalidation = true;
        Ok(())
    }

    pub fn redo(&mut self) -> Result<(), String> {
        self.dispatch(AppCommand::Redo)?;
        self.needs_thumbnail_invalidation = true;
        Ok(())
    }

    pub fn can_undo(&self) -> bool {
        self.manager
            .active_session()
            .map(|s| s.history.can_undo())
            .unwrap_or(false)
    }

    pub fn can_redo(&self) -> bool {
        self.manager
            .active_session()
            .map(|s| s.history.can_redo())
            .unwrap_or(false)
    }
}
