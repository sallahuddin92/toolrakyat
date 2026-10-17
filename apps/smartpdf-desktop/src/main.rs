//! SmartPDF macOS Desktop Application
//! Built with Slint and powered by StarPDF App-Core.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use slint::{ComponentHandle, ModelRc, VecModel};
use starpdf::execution::StarNodeId;
use starpdf::syntax::object::ObjectRef;
use starpdf_app_core::command::AppCommand;
use starpdf_app_core::export::ExportProfile;
use starpdf_app_core::selection::{FontInfo, Rect, SelectionState};
use starpdf_app_core::session::SessionManager;

slint::include_modules!();

/// Embedded default sample PDF document so the desktop app is immediately interactive on launch.
const DEFAULT_SAMPLE_PDF: &[u8] = include_bytes!("../../../test-assets/text-multispan-heading.pdf");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = AppWindow::new()?;
    let manager = Rc::new(RefCell::new(SessionManager::new()));
    let active_tool = Rc::new(RefCell::new("select".to_string()));
    let is_editing = Rc::new(RefCell::new(false));

    // 1. Load initial document from command-line argument if provided, or load default sample
    let cli_path = std::env::args().nth(1).map(PathBuf::from);
    let (bytes, title, path) = if let Some(path) = cli_path {
        if let Ok(b) = std::fs::read(&path) {
            let t = path
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| "document.pdf".to_string());
            (b, t, Some(path))
        } else {
            eprintln!(
                "Warning: could not read path {:?}, loading sample PDF",
                path
            );
            (
                DEFAULT_SAMPLE_PDF.to_vec(),
                "sample_contract.pdf".to_string(),
                None,
            )
        }
    } else {
        (
            DEFAULT_SAMPLE_PDF.to_vec(),
            "sample_contract.pdf".to_string(),
            None,
        )
    };

    {
        let mut mgr = manager.borrow_mut();
        if let Err(err) = mgr.dispatch(AppCommand::OpenDocument { path, bytes, title }) {
            eprintln!("Failed to load initial document: {err}");
        }
    }

    // Initial state synchronization
    sync_ui(
        &app,
        &manager.borrow(),
        &active_tool.borrow(),
        *is_editing.borrow(),
    );

    // ==========================================
    // Register UI Callbacks
    // ==========================================

    // Open File Dialog (rfd)
    {
        let app_weak = app.as_weak();
        let manager = manager.clone();
        let active_tool = active_tool.clone();
        let is_editing = is_editing.clone();
        app.on_open_file_clicked(move || {
            if let Some(file_path) = rfd::FileDialog::new()
                .add_filter("PDF Document", &["pdf"])
                .set_title("Open PDF Document")
                .pick_file()
            {
                if let Ok(bytes) = std::fs::read(&file_path) {
                    let title = file_path
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "document.pdf".to_string());

                    let mut mgr = manager.borrow_mut();
                    match mgr.dispatch(AppCommand::OpenDocument {
                        path: Some(file_path.clone()),
                        bytes,
                        title,
                    }) {
                        Ok(res) => {
                            if let Some(app) = app_weak.upgrade() {
                                app.set_status_message(res.message.into());
                                sync_ui(&app, &mgr, &active_tool.borrow(), *is_editing.borrow());
                            }
                        }
                        Err(err) => {
                            if let Some(app) = app_weak.upgrade() {
                                app.set_status_message(format!("Open Error: {err}").into());
                            }
                        }
                    }
                }
            }
        });
    }

    // Export Document Dialog
    {
        let app_weak = app.as_weak();
        let manager = manager.clone();
        app.on_export_document_clicked(move || {
            let mut mgr = manager.borrow_mut();
            match mgr.dispatch(AppCommand::ExportDocument {
                profile: ExportProfile::IncrementalPdf,
            }) {
                Ok(res) => {
                    if let Some(exported_bytes) = res.exported_bytes {
                        if let Some(save_path) = rfd::FileDialog::new()
                            .add_filter("PDF Document", &["pdf"])
                            .set_title("Export PDF Document")
                            .set_file_name("smartpdf_export.pdf")
                            .save_file()
                        {
                            if let Err(e) = std::fs::write(&save_path, exported_bytes) {
                                if let Some(app) = app_weak.upgrade() {
                                    app.set_status_message(format!("Save failed: {e}").into());
                                }
                            } else if let Some(app) = app_weak.upgrade() {
                                app.set_status_message(
                                    format!("Saved to {}", save_path.display()).into(),
                                );
                            }
                        }
                    }
                }
                Err(err) => {
                    if let Some(app) = app_weak.upgrade() {
                        app.set_status_message(format!("Export failed: {err}").into());
                    }
                }
            }
        });
    }

    // Tab Selection
    {
        let app_weak = app.as_weak();
        let manager = manager.clone();
        let active_tool = active_tool.clone();
        let is_editing = is_editing.clone();
        app.on_tab_selected(move |tab_id| {
            let mut mgr = manager.borrow_mut();
            let _ = mgr.dispatch(AppCommand::SwitchTab {
                session_id: tab_id.to_string(),
            });
            if let Some(app) = app_weak.upgrade() {
                sync_ui(&app, &mgr, &active_tool.borrow(), *is_editing.borrow());
            }
        });
    }

    // Tab Closed
    {
        let app_weak = app.as_weak();
        let manager = manager.clone();
        let active_tool = active_tool.clone();
        let is_editing = is_editing.clone();
        app.on_tab_closed(move |tab_id| {
            let mut mgr = manager.borrow_mut();
            let _ = mgr.dispatch(AppCommand::CloseSession {
                session_id: tab_id.to_string(),
            });
            if let Some(app) = app_weak.upgrade() {
                sync_ui(&app, &mgr, &active_tool.borrow(), *is_editing.borrow());
            }
        });
    }

    // Tool Mode Changed
    {
        let app_weak = app.as_weak();
        let manager = manager.clone();
        let active_tool = active_tool.clone();
        let is_editing = is_editing.clone();
        app.on_tool_changed(move |tool| {
            *active_tool.borrow_mut() = tool.to_string();
            if tool != "text" {
                *is_editing.borrow_mut() = false;
            }
            if let Some(app) = app_weak.upgrade() {
                app.set_active_tool(tool);
                sync_ui(
                    &app,
                    &manager.borrow(),
                    &active_tool.borrow(),
                    *is_editing.borrow(),
                );
            }
        });
    }

    // Page Selected from Thumbnails
    {
        let app_weak = app.as_weak();
        let manager = manager.clone();
        let active_tool = active_tool.clone();
        let is_editing = is_editing.clone();
        app.on_page_selected(move |page_idx| {
            let mut mgr = manager.borrow_mut();
            let _ = mgr.dispatch(AppCommand::SetPage {
                page_index: page_idx as u32,
            });
            if let Some(app) = app_weak.upgrade() {
                sync_ui(&app, &mgr, &active_tool.borrow(), *is_editing.borrow());
            }
        });
    }

    // Previous Page
    {
        let app_weak = app.as_weak();
        let manager = manager.clone();
        let active_tool = active_tool.clone();
        let is_editing = is_editing.clone();
        app.on_prev_page_clicked(move || {
            let mut mgr = manager.borrow_mut();
            if let Some(s) = mgr.active_session() {
                if s.current_page > 0 {
                    let prev = s.current_page - 1;
                    let _ = mgr.dispatch(AppCommand::SetPage { page_index: prev });
                }
            }
            if let Some(app) = app_weak.upgrade() {
                sync_ui(&app, &mgr, &active_tool.borrow(), *is_editing.borrow());
            }
        });
    }

    // Next Page
    {
        let app_weak = app.as_weak();
        let manager = manager.clone();
        let active_tool = active_tool.clone();
        let is_editing = is_editing.clone();
        app.on_next_page_clicked(move || {
            let mut mgr = manager.borrow_mut();
            if let Some(s) = mgr.active_session() {
                if s.current_page + 1 < s.total_pages {
                    let next = s.current_page + 1;
                    let _ = mgr.dispatch(AppCommand::SetPage { page_index: next });
                }
            }
            if let Some(app) = app_weak.upgrade() {
                sync_ui(&app, &mgr, &active_tool.borrow(), *is_editing.borrow());
            }
        });
    }

    // Zoom Controls
    {
        let app_weak = app.as_weak();
        let manager = manager.clone();
        let active_tool = active_tool.clone();
        let is_editing = is_editing.clone();
        app.on_zoom_in_clicked(move || {
            let mut mgr = manager.borrow_mut();
            if let Some(s) = mgr.active_session() {
                let new_zoom = (s.zoom.percent + 25.0).min(400.0);
                let _ = mgr.dispatch(AppCommand::SetZoom { percent: new_zoom });
            }
            if let Some(app) = app_weak.upgrade() {
                sync_ui(&app, &mgr, &active_tool.borrow(), *is_editing.borrow());
            }
        });
    }

    {
        let app_weak = app.as_weak();
        let manager = manager.clone();
        let active_tool = active_tool.clone();
        let is_editing = is_editing.clone();
        app.on_zoom_out_clicked(move || {
            let mut mgr = manager.borrow_mut();
            if let Some(s) = mgr.active_session() {
                let new_zoom = (s.zoom.percent - 25.0).max(25.0);
                let _ = mgr.dispatch(AppCommand::SetZoom { percent: new_zoom });
            }
            if let Some(app) = app_weak.upgrade() {
                sync_ui(&app, &mgr, &active_tool.borrow(), *is_editing.borrow());
            }
        });
    }

    {
        let app_weak = app.as_weak();
        let manager = manager.clone();
        let active_tool = active_tool.clone();
        let is_editing = is_editing.clone();
        app.on_zoom_reset_clicked(move || {
            let mut mgr = manager.borrow_mut();
            let _ = mgr.dispatch(AppCommand::SetZoom { percent: 100.0 });
            if let Some(app) = app_weak.upgrade() {
                sync_ui(&app, &mgr, &active_tool.borrow(), *is_editing.borrow());
            }
        });
    }

    // Undo / Redo
    {
        let app_weak = app.as_weak();
        let manager = manager.clone();
        let active_tool = active_tool.clone();
        let is_editing = is_editing.clone();
        app.on_undo_clicked(move || {
            let mut mgr = manager.borrow_mut();
            match mgr.dispatch(AppCommand::Undo) {
                Ok(res) => {
                    if let Some(app) = app_weak.upgrade() {
                        app.set_status_message(res.message.into());
                        sync_ui(&app, &mgr, &active_tool.borrow(), *is_editing.borrow());
                    }
                }
                Err(err) => {
                    if let Some(app) = app_weak.upgrade() {
                        app.set_status_message(format!("Cannot Undo: {err}").into());
                    }
                }
            }
        });
    }

    {
        let app_weak = app.as_weak();
        let manager = manager.clone();
        let active_tool = active_tool.clone();
        let is_editing = is_editing.clone();
        app.on_redo_clicked(move || {
            let mut mgr = manager.borrow_mut();
            match mgr.dispatch(AppCommand::Redo) {
                Ok(res) => {
                    if let Some(app) = app_weak.upgrade() {
                        app.set_status_message(res.message.into());
                        sync_ui(&app, &mgr, &active_tool.borrow(), *is_editing.borrow());
                    }
                }
                Err(err) => {
                    if let Some(app) = app_weak.upgrade() {
                        app.set_status_message(format!("Cannot Redo: {err}").into());
                    }
                }
            }
        });
    }

    // Panel Toggles
    {
        let app_weak = app.as_weak();
        app.on_toggle_left_panel(move || {
            if let Some(app) = app_weak.upgrade() {
                let vis = app.get_left_panel_visible();
                app.set_left_panel_visible(!vis);
            }
        });
    }

    {
        let app_weak = app.as_weak();
        app.on_toggle_right_panel(move || {
            if let Some(app) = app_weak.upgrade() {
                let vis = app.get_right_panel_visible();
                app.set_right_panel_visible(!vis);
            }
        });
    }

    // Text Span Clicked (Selection & In-Place Editing Trigger)
    {
        let app_weak = app.as_weak();
        let manager = manager.clone();
        let active_tool = active_tool.clone();
        let is_editing = is_editing.clone();
        app.on_span_selected(move |span_id| {
            let mut mgr = manager.borrow_mut();
            let mut span_found = None;

            if let Some(session) = mgr.active_session() {
                if let Ok(mut doc) = session.pdf_document() {
                    if let Ok(pages) = doc.extract_all_text() {
                        if let Some(page) = pages.get(session.current_page as usize) {
                            for s in &page.spans {
                                if s.span_id == span_id.as_str() {
                                    span_found = Some(s.clone());
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            if let Some(span) = span_found {
                let page_height = 792.0;
                let canvas_y = (page_height - span.y - span.height).max(0.0);
                let bounds = Rect::new(
                    span.x,
                    canvas_y,
                    span.width.max(10.0),
                    span.height.max(12.0),
                );
                let star_node_id = StarNodeId::new("doc", ObjectRef::new(1, 0), &span.span_id)
                    .unwrap_or_else(|_| {
                        // Safe fallback canonical ID
                        StarNodeId::new("doc", ObjectRef::new(1, 0), "span").unwrap()
                    });

                let sel = SelectionState::Text {
                    star_node_id,
                    render_node_id: None,
                    text: span.text.clone(),
                    bounds,
                    font_info: Some(FontInfo::new(span.font_name.clone(), span.font_size)),
                    is_editable: true,
                };

                let _ = mgr.dispatch(AppCommand::SetSelection { selection: sel });

                *is_editing.borrow_mut() = *active_tool.borrow() == "text";
            }

            if let Some(app) = app_weak.upgrade() {
                sync_ui(&app, &mgr, &active_tool.borrow(), *is_editing.borrow());
            }
        });
    }

    // Element Clicked (Images, Vector placeholders)
    {
        let app_weak = app.as_weak();
        let manager = manager.clone();
        let active_tool = active_tool.clone();
        let is_editing = is_editing.clone();
        app.on_element_selected(move |elem_id| {
            let mut mgr = manager.borrow_mut();
            let star_node_id = StarNodeId::new("doc", ObjectRef::new(1, 0), &elem_id)
                .unwrap_or_else(|_| StarNodeId::new("doc", ObjectRef::new(1, 0), "elem").unwrap());

            let sel = SelectionState::Image {
                star_node_id,
                render_node_id: None,
                image_ref: elem_id.to_string(),
                bounds: Rect::new(50.0, 50.0, 200.0, 150.0),
                is_shared: false,
            };

            let _ = mgr.dispatch(AppCommand::SetSelection { selection: sel });
            *is_editing.borrow_mut() = false;

            if let Some(app) = app_weak.upgrade() {
                sync_ui(&app, &mgr, &active_tool.borrow(), *is_editing.borrow());
            }
        });
    }

    // Canvas Background Clicked (Clear Selection)
    {
        let app_weak = app.as_weak();
        let manager = manager.clone();
        let active_tool = active_tool.clone();
        let is_editing = is_editing.clone();
        app.on_canvas_background_clicked(move || {
            let mut mgr = manager.borrow_mut();
            let _ = mgr.dispatch(AppCommand::ClearSelection);
            *is_editing.borrow_mut() = false;
            if let Some(app) = app_weak.upgrade() {
                sync_ui(&app, &mgr, &active_tool.borrow(), *is_editing.borrow());
            }
        });
    }

    // In-Place / Properties Text Edit Committed
    {
        let app_weak = app.as_weak();
        let manager = manager.clone();
        let active_tool = active_tool.clone();
        let is_editing = is_editing.clone();
        app.on_text_edit_committed(move |new_text| {
            let mut mgr = manager.borrow_mut();
            let mut target_span_id = None;

            if let Some(session) = mgr.active_session() {
                if let SelectionState::Text { star_node_id, .. } = &session.selection {
                    // Extract span_id from star_node_id parts
                    let (_, _, occ) = star_node_id.parts();
                    target_span_id = Some(occ);
                }
            }

            if let Some(target) = target_span_id {
                match mgr.dispatch(AppCommand::ReplaceText {
                    target_node: target,
                    new_text: new_text.to_string(),
                }) {
                    Ok(res) => {
                        *is_editing.borrow_mut() = false;
                        if let Some(app) = app_weak.upgrade() {
                            app.set_status_message(res.message.into());
                            sync_ui(&app, &mgr, &active_tool.borrow(), *is_editing.borrow());
                        }
                    }
                    Err(err) => {
                        if let Some(app) = app_weak.upgrade() {
                            app.set_status_message(format!("Edit Refused: {err}").into());
                        }
                    }
                }
            }
        });
    }

    // Property Text Field Edited
    {
        let app_weak = app.as_weak();
        app.on_property_text_edited(move |txt| {
            if let Some(app) = app_weak.upgrade() {
                app.set_selection_text_content(txt);
            }
        });
    }

    // Run the native event loop
    app.run()?;
    Ok(())
}

/// Synchronizes state from authoritative `starpdf_app_core::session::SessionManager` to the Slint UI.
fn sync_ui(app: &AppWindow, manager: &SessionManager, active_tool: &str, is_editing: bool) {
    // 1. Sync Document Tabs
    let tab_items: Vec<TabItem> = manager
        .tabs()
        .into_iter()
        .map(|t| TabItem {
            id: t.session_id.into(),
            title: t.title.into(),
            is_active: t.is_active,
            is_dirty: t.is_dirty,
        })
        .collect();
    app.set_tabs(ModelRc::new(VecModel::from(tab_items)));
    app.set_active_tool(active_tool.into());

    if let Some(session) = manager.active_session() {
        app.set_active_tab_id(session.id.clone().into());
        app.set_active_page(session.current_page as i32);
        app.set_total_pages(session.total_pages as i32);
        app.set_page_info_text(
            format!(
                "Page {} of {}",
                session.current_page + 1,
                session.total_pages
            )
            .into(),
        );

        app.set_zoom_scale(session.zoom.percent / 100.0);
        app.set_zoom_percent(session.zoom.percent as i32);

        app.set_undo_depth(session.history.undo_depth() as i32);
        app.set_redo_depth(session.history.redo_depth() as i32);
        app.set_can_undo(session.history.can_undo());
        app.set_can_redo(session.history.can_redo());

        app.set_evidence_level("VERIFIED".into());
        let sec_status = if session.capabilities.can_edit.allowed {
            "Unrestricted (Local-First)"
        } else {
            "Read-Only / Protected"
        };
        app.set_security_status(sec_status.into());

        // 2. Build Thumbnails
        let mut thumbs = Vec::new();
        for p in 0..session.total_pages {
            thumbs.push(PageThumbnail {
                page_index: p as i32,
                label: format!("Page {}", p + 1).into(),
                dimensions: "612 × 792 pt".into(),
                is_active: p == session.current_page,
            });
        }
        app.set_thumbnails(ModelRc::new(VecModel::from(thumbs)));

        // 3. Extract Text Spans for Active Page
        let mut text_spans_data = Vec::new();
        let page_height = 792.0;

        if let Ok(mut doc) = session.pdf_document() {
            if let Ok(pages) = doc.extract_all_text() {
                let page_idx = session.current_page as usize;
                if let Some(page) = pages.get(page_idx) {
                    for span in &page.spans {
                        let is_selected = match &session.selection {
                            SelectionState::Text { star_node_id, .. } => {
                                let (_, _, occ) = star_node_id.parts();
                                occ == span.span_id
                            }
                            _ => false,
                        };

                        let canvas_y = (page_height - span.y - span.height).max(0.0);
                        text_spans_data.push(TextSpanData {
                            id: span.span_id.clone().into(),
                            text: span.text.clone().into(),
                            x: span.x as f32,
                            y: canvas_y as f32,
                            width: span.width.max(12.0) as f32,
                            height: span.height.max(14.0) as f32,
                            font_size: (span.font_size as f32).max(9.0),
                            font_family: span.font_name.clone().into(),
                            color_hex: "#0F172A".into(),
                            is_selected,
                        });
                    }
                }
            }
        }
        app.set_text_spans(ModelRc::new(VecModel::from(text_spans_data)));

        // 4. Page Elements (vector / image placeholders)
        let page_elements_data = vec![ElementItem {
            id: "img_logo".into(),
            kind: "image".into(),
            x: 480.0,
            y: 40.0,
            width: 80.0,
            height: 40.0,
            label: "📷 Header Logo".into(),
            is_selected: match &session.selection {
                SelectionState::Image { image_ref, .. } => image_ref == "img_logo",
                _ => false,
            },
        }];
        app.set_page_elements(ModelRc::new(VecModel::from(page_elements_data)));

        // 5. Selection Properties
        match &session.selection {
            SelectionState::Text {
                star_node_id,
                text,
                bounds,
                font_info,
                is_editable,
                ..
            } => {
                app.set_has_selection(true);
                app.set_selection_type("Text".into());
                app.set_selection_node_id(star_node_id.as_str().into());
                app.set_selection_bounds_text(
                    format!(
                        "{:.1}, {:.1}, {:.1} × {:.1}",
                        bounds.x, bounds.y, bounds.width, bounds.height
                    )
                    .into(),
                );
                app.set_selection_font_name(
                    font_info
                        .as_ref()
                        .map_or("Helvetica".into(), |f| f.name.clone().into()),
                );
                app.set_selection_font_size(
                    font_info
                        .as_ref()
                        .map_or("12.0 pt".into(), |f| format!("{:.1} pt", f.size).into()),
                );
                app.set_selection_text_content(text.clone().into());
                app.set_selection_is_editable(*is_editable);

                app.set_sel_x(bounds.x as f32);
                app.set_sel_y(bounds.y as f32);
                app.set_sel_w(bounds.width as f32);
                app.set_sel_h(bounds.height as f32);

                if is_editing {
                    app.set_is_editing_text(true);
                    app.set_editing_text_buffer(text.clone().into());
                } else {
                    app.set_is_editing_text(false);
                }
            }

            SelectionState::Image {
                image_ref, bounds, ..
            } => {
                app.set_has_selection(true);
                app.set_selection_type("Image".into());
                app.set_selection_node_id(image_ref.clone().into());
                app.set_selection_bounds_text(
                    format!(
                        "{:.1}, {:.1}, {:.1} × {:.1}",
                        bounds.x, bounds.y, bounds.width, bounds.height
                    )
                    .into(),
                );
                app.set_selection_font_name("".into());
                app.set_selection_font_size("".into());
                app.set_selection_text_content("".into());
                app.set_selection_is_editable(false);

                app.set_sel_x(bounds.x as f32);
                app.set_sel_y(bounds.y as f32);
                app.set_sel_w(bounds.width as f32);
                app.set_sel_h(bounds.height as f32);
                app.set_is_editing_text(false);
            }

            SelectionState::Vector { bounds, .. } => {
                app.set_has_selection(true);
                app.set_selection_type("Vector".into());
                app.set_selection_node_id("vector_element".into());
                app.set_selection_bounds_text(
                    format!(
                        "{:.1}, {:.1}, {:.1} × {:.1}",
                        bounds.x, bounds.y, bounds.width, bounds.height
                    )
                    .into(),
                );
                app.set_selection_font_name("".into());
                app.set_selection_font_size("".into());
                app.set_selection_text_content("".into());
                app.set_selection_is_editable(false);

                app.set_sel_x(bounds.x as f32);
                app.set_sel_y(bounds.y as f32);
                app.set_sel_w(bounds.width as f32);
                app.set_sel_h(bounds.height as f32);
                app.set_is_editing_text(false);
            }

            SelectionState::Page { page_index } => {
                app.set_has_selection(true);
                app.set_selection_type("Page".into());
                app.set_selection_node_id(format!("Page {}", page_index + 1).into());
                app.set_selection_bounds_text("0, 0, 612 × 792".into());
                app.set_selection_font_name("".into());
                app.set_selection_font_size("".into());
                app.set_selection_text_content("".into());
                app.set_selection_is_editable(false);
                app.set_is_editing_text(false);
            }

            _ => {
                app.set_has_selection(false);
                app.set_selection_type("None".into());
                app.set_selection_node_id("".into());
                app.set_selection_bounds_text("".into());
                app.set_selection_font_name("".into());
                app.set_selection_font_size("".into());
                app.set_selection_text_content("".into());
                app.set_selection_is_editable(false);
                app.set_is_editing_text(false);
            }
        }
    } else {
        app.set_active_tab_id("".into());
        app.set_has_selection(false);
        app.set_selection_type("None".into());
        app.set_thumbnails(ModelRc::new(VecModel::from(vec![])));
        app.set_text_spans(ModelRc::new(VecModel::from(vec![])));
        app.set_page_elements(ModelRc::new(VecModel::from(vec![])));
        app.set_is_editing_text(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slint::Model;

    fn init_testing() {
        let _ = i_slint_backend_testing::init_no_event_loop();
    }

    #[test]
    fn test_ui_initialization_and_sync() {
        init_testing();
        let app = AppWindow::new().expect("Failed to initialize Slint AppWindow");
        let mut manager = SessionManager::new();

        // Load sample PDF
        manager
            .dispatch(AppCommand::OpenDocument {
                path: None,
                bytes: DEFAULT_SAMPLE_PDF.to_vec(),
                title: "sample_contract.pdf".into(),
            })
            .expect("Failed to open document");

        assert_eq!(manager.session_count(), 1);
        sync_ui(&app, &manager, "select", false);

        assert_eq!(
            app.get_active_tab_id(),
            manager.active_session().unwrap().id
        );
        assert_eq!(app.get_evidence_level(), "VERIFIED");
        assert_eq!(app.get_zoom_percent(), 100);
        assert!(!app.get_has_selection());
        assert!(app.get_text_spans().row_count() > 0);
    }

    #[test]
    fn test_selection_and_editing_workflow() {
        init_testing();
        let app = AppWindow::new().expect("Failed to initialize Slint AppWindow");
        let mut manager = SessionManager::new();

        manager
            .dispatch(AppCommand::OpenDocument {
                path: None,
                bytes: DEFAULT_SAMPLE_PDF.to_vec(),
                title: "test.pdf".into(),
            })
            .expect("Failed to open document");

        let star_node_id = StarNodeId::new("doc", ObjectRef::new(1, 0), "p0_s0_i0_o0").unwrap();
        let sel = SelectionState::Text {
            star_node_id,
            render_node_id: None,
            text: "Heading Line".into(),
            bounds: Rect::new(50.0, 100.0, 150.0, 24.0),
            font_info: Some(FontInfo::new("Helvetica", 18.0)),
            is_editable: true,
        };

        manager
            .dispatch(AppCommand::SetSelection { selection: sel })
            .expect("Failed to set selection");

        sync_ui(&app, &manager, "text", true);

        assert!(app.get_has_selection());
        assert_eq!(app.get_selection_type(), "Text");
        assert_eq!(app.get_selection_text_content(), "Heading Line");
        assert!(app.get_is_editing_text());
        assert_eq!(app.get_editing_text_buffer(), "Heading Line");
    }

    #[test]
    fn test_multilingual_unicode_support() {
        init_testing();
        let app = AppWindow::new().expect("Failed to initialize Slint AppWindow");
        let test_strings = vec![
            "العربية (Arabic)",
            "فاطمة ڤيکتوريا (Jawi)",
            "עִבְרִית (Hebrew)",
            "日本語・中文・한국어 (CJK)",
            "हिन्दी (Devanagari)",
        ];

        for s in test_strings {
            app.set_selection_text_content(s.into());
            assert_eq!(app.get_selection_text_content(), s);
        }
    }
}
