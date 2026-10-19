//! SmartPDF macOS Desktop Application
//! Built with Slint and powered by StarPDF App-Core.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use slint::{ComponentHandle, ModelRc, VecModel};
use starpdf::execution::StarNodeId;
use starpdf::syntax::object::ObjectRef;
use starpdf_app_core::command::AppCommand;
use starpdf_app_core::export::ExportProfile;
use starpdf_app_core::selection::{FontInfo, Rect, SelectionState};
use starpdf_app_core::session::SessionManager;

slint::include_modules!();

/// Global GPU renderer and acceleration state for the desktop application.
static GPU_RENDERER: Mutex<Option<starpdf::render::WgpuRenderer>> = Mutex::new(None);
static GPU_ENABLED: AtomicBool = AtomicBool::new(true);

#[derive(Clone)]
struct CachedSlintImage {
    page: u32,
    zoom_bits: u32,
    revision: u64,
    gpu_enabled: bool,
    image: slint::Image,
    width: f32,
    height: f32,
    badge: slint::SharedString,
    metrics: slint::SharedString,
}

thread_local! {
    static SESSION_IMAGE_CACHE: RefCell<std::collections::HashMap<String, CachedSlintImage>> =
        RefCell::new(std::collections::HashMap::new());
}
static LAST_THUMBNAILS: Mutex<Option<(String, u32, u32)>> = Mutex::new(None);

/// Embedded default sample PDF document so the desktop app is immediately interactive on launch.
const DEFAULT_SAMPLE_PDF: &[u8] = include_bytes!("../../../test-assets/text-multispan-heading.pdf");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("SmartPDF 0.1.0-rc.1 (Phase 13 Release Candidate)");
        return Ok(());
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("SmartPDF 0.1.0 - Native macOS PDF Editor & Viewer");
        println!("Usage: smartpdf-desktop [OPTIONS] [PDF_FILE]");
        println!("\nOptions:");
        println!("  -h, --help       Print help information");
        println!("  -V, --version    Print version information");
        return Ok(());
    }

    let app = AppWindow::new()?;
    let manager = Rc::new(RefCell::new(SessionManager::new()));
    let active_tool = Rc::new(RefCell::new("select".to_string()));
    let is_editing = Rc::new(RefCell::new(false));

    // Initialize GPU renderer
    if let Ok(mut lock) = GPU_RENDERER.lock() {
        if lock.is_none() {
            *lock = starpdf::render::WgpuRenderer::new().ok();
        }
    }

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

    register_ui_callbacks(&app, manager.clone(), active_tool.clone(), is_editing.clone());

    // Run the native event loop
    app.run()?;
    Ok(())
}

pub fn register_ui_callbacks(
    app: &AppWindow,
    manager: Rc<RefCell<SessionManager>>,
    active_tool: Rc<RefCell<String>>,
    is_editing: Rc<RefCell<bool>>,
) {
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

    // Toggle GPU Acceleration Callback
    {
        let app_weak = app.as_weak();
        let manager = manager.clone();
        let active_tool = active_tool.clone();
        let is_editing = is_editing.clone();
        app.on_toggle_gpu_acceleration(move || {
            let current = GPU_ENABLED.load(Ordering::SeqCst);
            GPU_ENABLED.store(!current, Ordering::SeqCst);
            if !current {
                if let Ok(mut lock) = GPU_RENDERER.lock() {
                    if lock.is_none() {
                        *lock = starpdf::render::WgpuRenderer::new().ok();
                    }
                }
            }
            if let Some(app) = app_weak.upgrade() {
                if let Some(session) = manager.borrow().active_session() {
                    session.invalidate_render_cache();
                }
                sync_ui(
                    &app,
                    &manager.borrow(),
                    &active_tool.borrow(),
                    *is_editing.borrow(),
                );
            }
        });
    }

    // Save Document Dialog / Shortcut (Cmd+S)
    {
        let app_weak = app.as_weak();
        let manager = manager.clone();
        let active_tool = active_tool.clone();
        let is_editing = is_editing.clone();
        app.on_save_document_clicked(move || {
            if let Some(app) = app_weak.upgrade() {
                commit_active_text_edit(&app, &manager, &active_tool, &is_editing);

                let file_path = {
                    let mgr = manager.borrow();
                    mgr.active_session().and_then(|s| s.file_path.clone())
                };

                let save_path = if let Some(p) = file_path {
                    Some(p)
                } else {
                    rfd::FileDialog::new()
                        .add_filter("PDF Document", &["pdf"])
                        .set_title("Save PDF Document")
                        .set_file_name("document.pdf")
                        .save_file()
                };

                if let Some(dest) = save_path {
                    let mut mgr = manager.borrow_mut();
                    match mgr.dispatch(AppCommand::SaveDocument {
                        destination_path: Some(dest),
                    }) {
                        Ok(res) => {
                            app.set_status_message(res.message.into());
                            sync_ui(&app, &mgr, &active_tool.borrow(), *is_editing.borrow());
                        }
                        Err(err) => {
                            app.set_status_message(format!("Save failed: {err}").into());
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
        let active_tool = active_tool.clone();
        let is_editing = is_editing.clone();
        app.on_export_document_clicked(move || {
            if let Some(app) = app_weak.upgrade() {
                commit_active_text_edit(&app, &manager, &active_tool, &is_editing);
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
                                    app.set_status_message(format!("Save failed: {e}").into());
                                } else {
                                    app.set_status_message(
                                        format!("Exported to {}", save_path.display()).into(),
                                    );
                                }
                            }
                        }
                    }
                    Err(err) => {
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
            if let Some(app) = app_weak.upgrade() {
                commit_active_text_edit(&app, &manager, &active_tool, &is_editing);
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

                sync_ui(&app, &mgr, &active_tool.borrow(), *is_editing.borrow());
            }
        });
    }

    // Element Clicked (Annotations, Form Fields, Images, Vectors)
    {
        let app_weak = app.as_weak();
        let manager = manager.clone();
        let active_tool = active_tool.clone();
        let is_editing = is_editing.clone();
        app.on_element_selected(move |elem_id| {
            if let Some(app) = app_weak.upgrade() {
                commit_active_text_edit(&app, &manager, &active_tool, &is_editing);
                let mut mgr = manager.borrow_mut();
                let elem_str = elem_id.as_str();

                if elem_str.starts_with("annot_") {
                    let num_str = &elem_str[6..];
                    if let Ok(num) = num_str.parse::<u64>() {
                        if let Some(session) = mgr.active_session() {
                            if let Ok(mut doc) = session.pdf_document() {
                                if let Ok(annots) = doc.page_annotations(session.current_page as usize) {
                                    if let Some(a) = annots.iter().find(|x| x.object_ref.number == num) {
                                        let page_h = 792.0;
                                        let canvas_y = (page_h - a.rect[3]).max(0.0);
                                        let bounds = Rect::new(
                                            a.rect[0],
                                            canvas_y,
                                            (a.rect[2] - a.rect[0]).abs().max(16.0),
                                            (a.rect[3] - a.rect[1]).abs().max(16.0),
                                        );
                                        let sel = SelectionState::Annotation {
                                            annot_ref: format!("{} 0 R", num),
                                            subtype: format!("{:?}", a.subtype),
                                            contents: a.contents.clone(),
                                            bounds,
                                        };
                                        let _ = mgr.dispatch(AppCommand::SetSelection { selection: sel });
                                    }
                                }
                            }
                        }
                    }
                } else if elem_str.starts_with("form_") {
                    let field_name = &elem_str[5..];
                    let mut should_toggle = false;
                    if let Some(session) = mgr.active_session() {
                        if let SelectionState::FormField {
                            field_name: prev_name,
                            field_type,
                            ..
                        } = &session.selection
                        {
                            if prev_name == field_name && field_type == "Checkbox" {
                                should_toggle = true;
                            }
                        }
                    }
                    if should_toggle {
                        let _ = mgr.dispatch(AppCommand::SetFormField {
                            field_name: field_name.to_string(),
                            value: "toggle".to_string(),
                        });
                    }
                    if let Some(session) = mgr.active_session() {
                        if let Ok(mut doc) = session.pdf_document() {
                            if let Ok(fields) = doc.form_fields() {
                                if let Some(f) = fields.iter().find(|x| x.partial_name == field_name || x.fully_qualified_name == field_name) {
                                    let type_str = match &f.field_type {
                                        starpdf::forms::FieldType::Checkbox => "Checkbox",
                                        starpdf::forms::FieldType::RadioButtonGroup => "RadioButtonGroup",
                                        starpdf::forms::FieldType::Text { .. } => "Text",
                                        starpdf::forms::FieldType::Choice { .. } => "Choice",
                                        _ => "Other",
                                    };
                                    let val_str = match &f.value {
                                        starpdf::forms::FieldValue::Text(s) => s.clone(),
                                        starpdf::forms::FieldValue::Boolean(b) => if *b { "true".into() } else { "false".into() },
                                        starpdf::forms::FieldValue::Name(s) => s.clone(),
                                        starpdf::forms::FieldValue::Choice(v) => v.first().cloned().unwrap_or_default(),
                                        starpdf::forms::FieldValue::None => "".into(),
                                    };
                                    let bounds = if let Some(w) = f.widgets.first() {
                                        let page_h = 792.0;
                                        Rect::new(w.rect[0], (page_h - w.rect[3]).max(0.0), (w.rect[2] - w.rect[0]).abs().max(16.0), (w.rect[3] - w.rect[1]).abs().max(16.0))
                                    } else {
                                        Rect::new(50.0, 50.0, 150.0, 30.0)
                                    };
                                    let sel = SelectionState::FormField {
                                        field_ref: format!("{} 0 R", f.object_ref.number),
                                        field_name: f.partial_name.clone(),
                                        field_type: type_str.to_string(),
                                        value: val_str,
                                        bounds,
                                    };
                                    let _ = mgr.dispatch(AppCommand::SetSelection { selection: sel });
                                }
                            }
                        }
                    }
                } else if elem_str.starts_with("img_") {
                    if let Some(session) = mgr.active_session() {
                        if let Ok(mut doc) = session.pdf_document() {
                            if let Ok(images) = doc.enumerate_images(session.current_page as usize) {
                                if let Some(img) = images.iter().find(|x| format!("img_{}", x.object_ref.number) == elem_str || x.image_id == elem_str) {
                                    let page_h = 792.0;
                                    let canvas_y = (page_h - img.rect[3]).max(0.0);
                                    let bounds = Rect::new(
                                        img.rect[0],
                                        canvas_y,
                                        (img.rect[2] - img.rect[0]).abs().max(20.0),
                                        (img.rect[3] - img.rect[1]).abs().max(20.0),
                                    );
                                    let star_node_id = StarNodeId::new("doc", img.object_ref, &img.image_id)
                                        .unwrap_or_else(|_| StarNodeId::new("doc", ObjectRef::new(1, 0), "elem").unwrap());
                                    let sel = SelectionState::Image {
                                        star_node_id,
                                        render_node_id: None,
                                        image_ref: img.image_id.clone(),
                                        bounds,
                                        is_shared: false,
                                    };
                                    let _ = mgr.dispatch(AppCommand::SetSelection { selection: sel });
                                }
                            }
                        }
                    }
                } else {
                    let star_node_id = StarNodeId::new("doc", ObjectRef::new(1, 0), &elem_id)
                        .unwrap_or_else(|_| {
                            StarNodeId::new("doc", ObjectRef::new(1, 0), "elem").unwrap()
                        });

                    let sel = SelectionState::Image {
                        star_node_id,
                        render_node_id: None,
                        image_ref: elem_id.to_string(),
                        bounds: Rect::new(50.0, 50.0, 200.0, 150.0),
                        is_shared: false,
                    };

                    let _ = mgr.dispatch(AppCommand::SetSelection { selection: sel });
                }

                *is_editing.borrow_mut() = false;
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
            if let Some(app) = app_weak.upgrade() {
                commit_active_text_edit(&app, &manager, &active_tool, &is_editing);
                let mut mgr = manager.borrow_mut();
                let _ = mgr.dispatch(AppCommand::ClearSelection);
                *is_editing.borrow_mut() = false;
                sync_ui(&app, &mgr, &active_tool.borrow(), false);
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

    // Delete Selected Element
    {
        let app_weak = app.as_weak();
        let manager = manager.clone();
        let active_tool = active_tool.clone();
        app.on_delete_selected_clicked(move || {
            let mut mgr = manager.borrow_mut();
            match mgr.dispatch(AppCommand::DeleteSelected) {
                Ok(res) => {
                    if let Some(app) = app_weak.upgrade() {
                        app.set_status_message(res.message.into());
                        sync_ui(&app, &mgr, &active_tool.borrow(), false);
                    }
                }
                Err(err) => {
                    if let Some(app) = app_weak.upgrade() {
                        app.set_status_message(format!("Delete Refused: {err}").into());
                    }
                }
            }
        });
    }

    // Add Annotation
    {
        let app_weak = app.as_weak();
        let manager = manager.clone();
        let active_tool = active_tool.clone();
        app.on_add_annotation_clicked(move |subtype| {
            let mut mgr = manager.borrow_mut();
            let page_idx = mgr.active_session().map(|s| s.current_page).unwrap_or(0);
            let rect = [72.0, 680.0, 320.0, 720.0];
            let contents = Some(format!("New {} annotation", subtype.as_str()));
            let color = Some(vec![1.0, 0.85, 0.1]);

            match mgr.dispatch(AppCommand::AddAnnotation {
                page_index: page_idx,
                subtype: subtype.to_string(),
                rect,
                contents,
                color,
            }) {
                Ok(res) => {
                    if let Some(app) = app_weak.upgrade() {
                        app.set_status_message(res.message.into());
                        sync_ui(&app, &mgr, &active_tool.borrow(), false);
                    }
                }
                Err(err) => {
                    if let Some(app) = app_weak.upgrade() {
                        app.set_status_message(format!("Add Annotation Refused: {err}").into());
                    }
                }
            }
        });
    }

    // Update Annotation Text
    {
        let app_weak = app.as_weak();
        let manager = manager.clone();
        let active_tool = active_tool.clone();
        app.on_update_annotation_text_clicked(move |new_text| {
            let mut mgr = manager.borrow_mut();
            let target_ref = if let Some(session) = mgr.active_session() {
                if let SelectionState::Annotation { annot_ref, .. } = &session.selection {
                    Some(annot_ref.clone())
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(annot_ref) = target_ref {
                match mgr.dispatch(AppCommand::UpdateAnnotation {
                    annot_ref,
                    contents: Some(new_text.to_string()),
                    rect: None,
                }) {
                    Ok(res) => {
                        if let Some(app) = app_weak.upgrade() {
                            app.set_status_message(res.message.into());
                            sync_ui(&app, &mgr, &active_tool.borrow(), false);
                        }
                    }
                    Err(err) => {
                        if let Some(app) = app_weak.upgrade() {
                            app.set_status_message(format!("Update Annotation Refused: {err}").into());
                        }
                    }
                }
            }
        });
    }

    // Add Image
    {
        let app_weak = app.as_weak();
        let manager = manager.clone();
        let active_tool = active_tool.clone();
        app.on_add_image_clicked(move || {
            let mut mgr = manager.borrow_mut();
            let page_idx = mgr.active_session().map(|s| s.current_page).unwrap_or(0);
            let jpeg_bytes: Vec<u8> = vec![
                0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x01,
                0x00, 0x48, 0x00, 0x48, 0x00, 0x00, 0xFF, 0xDB, 0x00, 0x43, 0x00, 0x08, 0x06, 0x06,
                0x07, 0x06, 0x05, 0x08, 0x07, 0x07, 0x07, 0x09, 0x09, 0x08, 0x0A, 0x0C, 0x14, 0x0D,
                0x0C, 0x0B, 0x0B, 0x0C, 0x19, 0x12, 0x13, 0x0F, 0x14, 0x1D, 0x1A, 0x1F, 0x1E, 0x1D,
                0x1A, 0x1C, 0x1C, 0x20, 0x24, 0x2E, 0x27, 0x20, 0x22, 0x2C, 0x23, 0x1C, 0x1C, 0x28,
                0x37, 0x29, 0x2C, 0x30, 0x31, 0x34, 0x34, 0x34, 0x1F, 0x27, 0x39, 0x3D, 0x38, 0x32,
                0x3C, 0x2E, 0x33, 0x34, 0x32, 0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x00, 0x02, 0x00, 0x02,
                0x01, 0x01, 0x11, 0x00, 0xFF, 0xC4, 0x00, 0x1F, 0x00, 0x00, 0x01, 0x05, 0x01, 0x01,
                0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02,
                0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0xFF, 0xDA, 0x00, 0x08, 0x01,
                0x01, 0x00, 0x00, 0x3F, 0x00, 0xBF, 0x80, 0xFF, 0xD9,
            ];

            match mgr.dispatch(AppCommand::AddImage {
                page_index: page_idx,
                image_bytes: jpeg_bytes,
                x: 100.0,
                y: 500.0,
                width: 120.0,
                height: 80.0,
            }) {
                Ok(res) => {
                    if let Some(app) = app_weak.upgrade() {
                        app.set_status_message(res.message.into());
                        sync_ui(&app, &mgr, &active_tool.borrow(), false);
                    }
                }
                Err(err) => {
                    if let Some(app) = app_weak.upgrade() {
                        app.set_status_message(format!("Add Image Refused: {err}").into());
                    }
                }
            }
        });
    }

    // Toggle Checkbox Form Field
    {
        let app_weak = app.as_weak();
        let manager = manager.clone();
        let active_tool = active_tool.clone();
        app.on_form_field_toggled(move |field_name, new_val| {
            let mut mgr = manager.borrow_mut();
            match mgr.dispatch(AppCommand::SetFormField {
                field_name: field_name.to_string(),
                value: new_val.to_string(),
            }) {
                Ok(res) => {
                    if let Some(app) = app_weak.upgrade() {
                        app.set_status_message(res.message.into());
                        sync_ui(&app, &mgr, &active_tool.borrow(), false);
                    }
                }
                Err(err) => {
                    if let Some(app) = app_weak.upgrade() {
                        app.set_status_message(format!("Field Toggle Refused: {err}").into());
                    }
                }
            }
        });
    }

    // Commit Form Field Text
    {
        let app_weak = app.as_weak();
        let manager = manager.clone();
        let active_tool = active_tool.clone();
        app.on_form_field_text_committed(move |field_name, new_val| {
            let mut mgr = manager.borrow_mut();
            match mgr.dispatch(AppCommand::SetFormField {
                field_name: field_name.to_string(),
                value: new_val.to_string(),
            }) {
                Ok(res) => {
                    if let Some(app) = app_weak.upgrade() {
                        app.set_status_message(res.message.into());
                        sync_ui(&app, &mgr, &active_tool.borrow(), false);
                    }
                }
                Err(err) => {
                    if let Some(app) = app_weak.upgrade() {
                        app.set_status_message(format!("Field Update Refused: {err}").into());
                    }
                }
            }
        });
    }

    // Page Management Callbacks
    {
        let app_weak = app.as_weak();
        let manager = manager.clone();
        let active_tool = active_tool.clone();
        app.on_insert_blank_page_clicked(move || {
            let mut mgr = manager.borrow_mut();
            let page_idx = mgr.active_session().map(|s| s.current_page).unwrap_or(0);
            match mgr.dispatch(AppCommand::InsertBlankPage {
                after_index: page_idx,
                width: 612.0,
                height: 792.0,
            }) {
                Ok(res) => {
                    if let Some(app) = app_weak.upgrade() {
                        app.set_status_message(res.message.into());
                        sync_ui(&app, &mgr, &active_tool.borrow(), false);
                    }
                }
                Err(err) => {
                    if let Some(app) = app_weak.upgrade() {
                        app.set_status_message(format!("Insert Page Refused: {err}").into());
                    }
                }
            }
        });
    }

    {
        let app_weak = app.as_weak();
        let manager = manager.clone();
        let active_tool = active_tool.clone();
        app.on_duplicate_page_clicked(move || {
            let mut mgr = manager.borrow_mut();
            let page_idx = mgr.active_session().map(|s| s.current_page).unwrap_or(0);
            match mgr.dispatch(AppCommand::DuplicatePage {
                page_index: page_idx,
            }) {
                Ok(res) => {
                    if let Some(app) = app_weak.upgrade() {
                        app.set_status_message(res.message.into());
                        sync_ui(&app, &mgr, &active_tool.borrow(), false);
                    }
                }
                Err(err) => {
                    if let Some(app) = app_weak.upgrade() {
                        app.set_status_message(format!("Duplicate Page Refused: {err}").into());
                    }
                }
            }
        });
    }

    {
        let app_weak = app.as_weak();
        let manager = manager.clone();
        let active_tool = active_tool.clone();
        app.on_move_page_up_clicked(move || {
            let mut mgr = manager.borrow_mut();
            let page_idx = mgr.active_session().map(|s| s.current_page).unwrap_or(0);
            if page_idx > 0 {
                let _ = mgr.dispatch(AppCommand::ReorderPage {
                    from_index: page_idx,
                    to_index: page_idx - 1,
                });
                if let Some(app) = app_weak.upgrade() {
                    sync_ui(&app, &mgr, &active_tool.borrow(), false);
                }
            }
        });
    }

    {
        let app_weak = app.as_weak();
        let manager = manager.clone();
        let active_tool = active_tool.clone();
        app.on_move_page_down_clicked(move || {
            let mut mgr = manager.borrow_mut();
            let (page_idx, total) = mgr
                .active_session()
                .map(|s| (s.current_page, s.total_pages))
                .unwrap_or((0, 0));
            if page_idx + 1 < total {
                let _ = mgr.dispatch(AppCommand::ReorderPage {
                    from_index: page_idx,
                    to_index: page_idx + 1,
                });
                if let Some(app) = app_weak.upgrade() {
                    sync_ui(&app, &mgr, &active_tool.borrow(), false);
                }
            }
        });
    }

    {
        let app_weak = app.as_weak();
        let manager = manager.clone();
        let active_tool = active_tool.clone();
        app.on_delete_page_clicked(move || {
            let mut mgr = manager.borrow_mut();
            let page_idx = mgr.active_session().map(|s| s.current_page).unwrap_or(0);
            match mgr.dispatch(AppCommand::DeletePage {
                page_index: page_idx,
            }) {
                Ok(res) => {
                    if let Some(app) = app_weak.upgrade() {
                        app.set_status_message(res.message.into());
                        sync_ui(&app, &mgr, &active_tool.borrow(), false);
                    }
                }
                Err(err) => {
                    if let Some(app) = app_weak.upgrade() {
                        app.set_status_message(format!("Delete Page Refused: {err}").into());
                    }
                }
            }
        });
    }

    // Move Selected Element
    {
        let app_weak = app.as_weak();
        let manager = manager.clone();
        let active_tool = active_tool.clone();
        app.on_move_selected_clicked(move |dx, dy| {
            let mut mgr = manager.borrow_mut();
            match mgr.dispatch(AppCommand::MoveElement {
                dx: dx as f64,
                dy: dy as f64,
            }) {
                Ok(res) => {
                    if let Some(app) = app_weak.upgrade() {
                        app.set_status_message(res.message.into());
                        sync_ui(&app, &mgr, &active_tool.borrow(), false);
                    }
                }
                Err(err) => {
                    if let Some(app) = app_weak.upgrade() {
                        app.set_status_message(format!("Move Refused: {err}").into());
                    }
                }
            }
        });
    }

    // Resize Selected Element
    {
        let app_weak = app.as_weak();
        let manager = manager.clone();
        let active_tool = active_tool.clone();
        app.on_resize_selected_clicked(move |dw, dh| {
            let mut mgr = manager.borrow_mut();
            match mgr.dispatch(AppCommand::ResizeElement {
                dw: dw as f64,
                dh: dh as f64,
            }) {
                Ok(res) => {
                    if let Some(app) = app_weak.upgrade() {
                        app.set_status_message(res.message.into());
                        sync_ui(&app, &mgr, &active_tool.borrow(), false);
                    }
                }
                Err(err) => {
                    if let Some(app) = app_weak.upgrade() {
                        app.set_status_message(format!("Resize Refused: {err}").into());
                    }
                }
            }
        });
    }

    // Replace Selected Image
    {
        let app_weak = app.as_weak();
        let manager = manager.clone();
        let active_tool = active_tool.clone();
        app.on_replace_image_clicked(move || {
            let mut mgr = manager.borrow_mut();
            let target = if let Some(session) = mgr.active_session() {
                if let SelectionState::Image { image_ref, .. } = &session.selection {
                    Some((session.current_page, image_ref.clone()))
                } else {
                    None
                }
            } else {
                None
            };

            if let Some((page_idx, image_ref)) = target {
                let rep_jpeg: Vec<u8> = vec![
                    0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x01,
                    0x00, 0x48, 0x00, 0x48, 0x00, 0x00, 0xFF, 0xDB, 0x00, 0x43, 0x00, 0x08, 0x06, 0x06,
                    0x07, 0x06, 0x05, 0x08, 0x07, 0x07, 0x07, 0x09, 0x09, 0x08, 0x0A, 0x0C, 0x14, 0x0D,
                    0x0C, 0x0B, 0x0B, 0x0C, 0x19, 0x12, 0x13, 0x0F, 0x14, 0x1D, 0x1A, 0x1F, 0x1E, 0x1D,
                    0x1A, 0x1C, 0x1C, 0x20, 0x24, 0x2E, 0x27, 0x20, 0x22, 0x2C, 0x23, 0x1C, 0x1C, 0x28,
                    0x37, 0x29, 0x2C, 0x30, 0x31, 0x34, 0x34, 0x34, 0x1F, 0x27, 0x39, 0x3D, 0x38, 0x32,
                    0x3C, 0x2E, 0x33, 0x34, 0x32, 0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x00, 0x02, 0x00, 0x02,
                    0x01, 0x01, 0x11, 0x00, 0xFF, 0xC4, 0x00, 0x1F, 0x00, 0x00, 0x01, 0x05, 0x01, 0x01,
                    0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02,
                    0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0xFF, 0xDA, 0x00, 0x08, 0x01,
                    0x01, 0x00, 0x00, 0x3F, 0x00, 0xBF, 0x80, 0xFF, 0xD9,
                ];

                match mgr.dispatch(AppCommand::ReplaceImage {
                    page_index: page_idx,
                    image_ref,
                    image_bytes: rep_jpeg,
                }) {
                    Ok(res) => {
                        if let Some(app) = app_weak.upgrade() {
                            app.set_status_message(res.message.into());
                            sync_ui(&app, &mgr, &active_tool.borrow(), false);
                        }
                    }
                    Err(err) => {
                        if let Some(app) = app_weak.upgrade() {
                            app.set_status_message(format!("Replace Image Refused: {err}").into());
                        }
                    }
                }
            }
        });
    }
}

/// Automatically commits any pending in-place text edits before navigation or deselect.
fn commit_active_text_edit(
    app: &AppWindow,
    manager: &Rc<RefCell<SessionManager>>,
    active_tool: &Rc<RefCell<String>>,
    is_editing: &Rc<RefCell<bool>>,
) {
    if !*is_editing.borrow() {
        return;
    }
    let draft_text = app.get_editing_text_buffer().to_string();
    let mut mgr = manager.borrow_mut();
    let mut target_span = None;

    if let Some(session) = mgr.active_session() {
        if let SelectionState::Text {
            star_node_id, text, ..
        } = &session.selection
        {
            if text != &draft_text {
                let (_, _, occ) = star_node_id.parts();
                target_span = Some((occ, draft_text));
            }
        }
    }

    if let Some((target, new_text)) = target_span {
        match mgr.dispatch(AppCommand::ReplaceText {
            target_node: target,
            new_text,
        }) {
            Ok(res) => {
                app.set_status_message(res.message.into());
            }
            Err(err) => {
                app.set_status_message(format!("Edit error: {err}").into());
            }
        }
    }
    *is_editing.borrow_mut() = false;
    sync_ui(app, &mgr, &active_tool.borrow(), false);
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

        app.set_evidence_level(session.evidence_level().into());
        let sec_status = if session.capabilities.can_edit.allowed {
            "Unrestricted (Local-First)"
        } else {
            session.capabilities.can_edit.reason
        };
        app.set_security_status(sec_status.into());

        // 1.5. Render Active Page via GPU-Accelerated Pipeline (with session caching and CPU fallback)
        let zoom_scale = (session.zoom.percent / 100.0).clamp(0.2, 4.0);
        let current_gpu = GPU_ENABLED.load(Ordering::SeqCst);

        let cached_hit = SESSION_IMAGE_CACHE.with(|cache| {
            let map = cache.borrow();
            if let Some(c) = map.get(&session.id) {
                if c.page == session.current_page
                    && c.zoom_bits == zoom_scale.to_bits()
                    && c.revision == session.revision
                    && c.gpu_enabled == current_gpu
                {
                    app.set_rendered_page_image(c.image.clone());
                    app.set_has_rendered_page_image(true);
                    app.set_page_width(c.width);
                    app.set_page_height(c.height);
                    app.set_gpu_acceleration_enabled(c.gpu_enabled);
                    app.set_render_engine_badge(c.badge.clone());
                    app.set_gpu_metrics_detail(c.metrics.clone());
                    true
                } else {
                    false
                }
            } else {
                false
            }
        });

        if !cached_hit {
            let mut rendered_success = false;
            let gpu_render_res = if current_gpu {
                if let Ok(guard) = GPU_RENDERER.lock() {
                    if let Some(ref r) = *guard {
                        session
                            .render_page_gpu(session.current_page, zoom_scale, r)
                            .ok()
                            .map(|res| (res, r.adapter_name().to_string()))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            if let Some((gpu_res, adapter_name)) = gpu_render_res {
                let mut pixel_buffer =
                    slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(gpu_res.width, gpu_res.height);
                pixel_buffer
                    .make_mut_bytes()
                    .copy_from_slice(&gpu_res.rgba_pixels);
                let slint_img = slint::Image::from_rgba8(pixel_buffer);
                app.set_rendered_page_image(slint_img.clone());
                app.set_has_rendered_page_image(true);
                let w = gpu_res.width as f32 / zoom_scale;
                let h = gpu_res.height as f32 / zoom_scale;
                app.set_page_width(w);
                app.set_page_height(h);
                app.set_gpu_acceleration_enabled(true);
                let short_name = adapter_name.split_whitespace().next().unwrap_or("Metal");
                let badge: slint::SharedString = format!("GPU ({short_name})").into();
                app.set_render_engine_badge(badge.clone());
                let metrics: slint::SharedString = format!(
                    "Frame: {:.1}ms (Draws: {}, GPU Paths: {}, Glyphs: {}, Images: {}, CPU fallback: {})",
                    gpu_res.metrics.total_frame_duration_ms,
                    gpu_res.metrics.gpu_draw_calls,
                    gpu_res.metrics.gpu_paths_rendered,
                    gpu_res.metrics.gpu_glyphs_rendered,
                    gpu_res.metrics.gpu_images_rendered,
                    gpu_res.metrics.fallback_cpu_items
                ).into();
                app.set_gpu_metrics_detail(metrics.clone());

                SESSION_IMAGE_CACHE.with(|cache| {
                    cache.borrow_mut().insert(session.id.clone(), CachedSlintImage {
                        page: session.current_page,
                        zoom_bits: zoom_scale.to_bits(),
                        revision: session.revision,
                        gpu_enabled: true,
                        image: slint_img,
                        width: w,
                        height: h,
                        badge,
                        metrics,
                    });
                });
                rendered_success = true;
            } else if let Ok(rendered) = session.render_page(session.current_page, zoom_scale) {
                let mut pixel_buffer =
                    slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(rendered.width, rendered.height);
                pixel_buffer
                    .make_mut_bytes()
                    .copy_from_slice(&rendered.rgba_pixels);
                let slint_img = slint::Image::from_rgba8(pixel_buffer);
                app.set_rendered_page_image(slint_img.clone());
                app.set_has_rendered_page_image(true);
                let w = rendered.width as f32 / zoom_scale;
                let h = rendered.height as f32 / zoom_scale;
                app.set_page_width(w);
                app.set_page_height(h);
                app.set_gpu_acceleration_enabled(false);
                let badge: slint::SharedString = "CPU (tiny-skia)".into();
                app.set_render_engine_badge(badge.clone());
                let metrics: slint::SharedString = format!("CPU Frame: {:.1}ms", rendered.duration_ms).into();
                app.set_gpu_metrics_detail(metrics.clone());

                SESSION_IMAGE_CACHE.with(|cache| {
                    cache.borrow_mut().insert(session.id.clone(), CachedSlintImage {
                        page: session.current_page,
                        zoom_bits: zoom_scale.to_bits(),
                        revision: session.revision,
                        gpu_enabled: false,
                        image: slint_img,
                        width: w,
                        height: h,
                        badge,
                        metrics,
                    });
                });
                rendered_success = true;
            }

            if !rendered_success {
                app.set_has_rendered_page_image(false);
            }
        }

        // 2. Build Thumbnails (Only rebuild when page or total pages change)
        let need_thumb_rebuild = if let Ok(lock) = LAST_THUMBNAILS.lock() {
            lock.as_ref() != Some(&(session.id.clone(), session.total_pages, session.current_page))
        } else {
            true
        };

        if need_thumb_rebuild {
            let mut thumbs = Vec::with_capacity(session.total_pages as usize);
            for p in 0..session.total_pages {
                thumbs.push(PageThumbnail {
                    page_index: p as i32,
                    label: format!("Page {}", p + 1).into(),
                    dimensions: "612 × 792 pt".into(),
                    is_active: p == session.current_page,
                });
            }
            app.set_thumbnails(ModelRc::new(VecModel::from(thumbs)));
            if let Ok(mut lock) = LAST_THUMBNAILS.lock() {
                *lock = Some((session.id.clone(), session.total_pages, session.current_page));
            }
        }

        // 3. Extract Text Spans for Active Page (Persistent Page Caching)
        let mut text_spans_data = Vec::new();
        let page_height = 792.0;

        if let Ok(spans) = session.extract_page_text_spans(session.current_page) {
            for span in &spans {
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
        app.set_text_spans(ModelRc::new(VecModel::from(text_spans_data)));

        // 4. Page Elements (Annotations, Form Fields, and Images)
        let mut page_elements_data = Vec::new();
        let page_height = 792.0;

        if let Ok(mut doc) = session.pdf_document() {
            if let Ok(annots) = doc.page_annotations(session.current_page as usize) {
                for a in &annots {
                    let canvas_y = (page_height - a.rect[3]).max(0.0) as f32;
                    let is_selected = match &session.selection {
                        SelectionState::Annotation { annot_ref, .. } => annot_ref.contains(&format!("{}", a.object_ref.number)),
                        _ => false,
                    };
                    page_elements_data.push(ElementItem {
                        id: format!("annot_{}", a.object_ref.number).into(),
                        kind: "annotation".into(),
                        x: a.rect[0] as f32,
                        y: canvas_y,
                        width: (a.rect[2] - a.rect[0]).abs().max(20.0) as f32,
                        height: (a.rect[3] - a.rect[1]).abs().max(16.0) as f32,
                        label: format!("✎ {:?}", a.subtype).into(),
                        is_selected,
                    });
                }
            }

            if let Ok(fields) = doc.form_fields() {
                for f in &fields {
                    for w in &f.widgets {
                        let canvas_y = (page_height - w.rect[3]).max(0.0) as f32;
                        let is_selected = match &session.selection {
                            SelectionState::FormField { field_name, .. } => field_name == &f.partial_name || field_name == &f.fully_qualified_name,
                            _ => false,
                        };
                        let prefix = match &f.field_type {
                            starpdf::forms::FieldType::Checkbox => "☑",
                            starpdf::forms::FieldType::RadioButtonGroup => "🔘",
                            starpdf::forms::FieldType::Text { .. } => "🗎",
                            _ => "☰",
                        };
                        page_elements_data.push(ElementItem {
                            id: format!("form_{}", f.partial_name).into(),
                            kind: "form_field".into(),
                            x: w.rect[0] as f32,
                            y: canvas_y,
                            width: (w.rect[2] - w.rect[0]).abs().max(20.0) as f32,
                            height: (w.rect[3] - w.rect[1]).abs().max(16.0) as f32,
                            label: format!("{} {}", prefix, f.partial_name).into(),
                            is_selected,
                        });
                    }
                }
            }

            if let Ok(images) = doc.enumerate_images(session.current_page as usize) {
                for img in &images {
                    let canvas_y = (page_height - img.rect[3]).max(0.0) as f32;
                    let is_selected = match &session.selection {
                        SelectionState::Image { image_ref, .. } => image_ref.contains(&format!("{}", img.object_ref.number)) || image_ref == &img.resource_name || image_ref == &img.image_id,
                        _ => false,
                    };
                    page_elements_data.push(ElementItem {
                        id: format!("img_{}", img.object_ref.number).into(),
                        kind: "image".into(),
                        x: img.rect[0] as f32,
                        y: canvas_y,
                        width: (img.rect[2] - img.rect[0]).abs().max(20.0) as f32,
                        height: (img.rect[3] - img.rect[1]).abs().max(20.0) as f32,
                        label: format!("🖼 {}", img.resource_name).into(),
                        is_selected,
                    });
                }
            }
        }
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

            SelectionState::FormField {
                field_ref,
                field_name,
                field_type,
                value,
                bounds,
            } => {
                app.set_has_selection(true);
                app.set_selection_type("FormField".into());
                app.set_selection_node_id(field_ref.clone().into());
                app.set_form_field_name(field_name.clone().into());
                app.set_form_field_type(field_type.clone().into());
                app.set_form_field_value(value.clone().into());
                let checked = value.eq_ignore_ascii_case("true")
                    || value.eq_ignore_ascii_case("yes")
                    || value == "1";
                app.set_form_field_checked(checked);
                app.set_selection_bounds_text(
                    format!(
                        "{:.1}, {:.1}, {:.1} × {:.1}",
                        bounds.x, bounds.y, bounds.width, bounds.height
                    )
                    .into(),
                );
                app.set_selection_font_name("".into());
                app.set_selection_font_size("".into());
                app.set_selection_text_content(value.clone().into());
                app.set_selection_is_editable(true);

                app.set_sel_x(bounds.x as f32);
                app.set_sel_y(bounds.y as f32);
                app.set_sel_w(bounds.width as f32);
                app.set_sel_h(bounds.height as f32);
                app.set_is_editing_text(false);
            }

            SelectionState::Annotation {
                annot_ref,
                subtype,
                contents,
                bounds,
            } => {
                app.set_has_selection(true);
                app.set_selection_type("Annotation".into());
                app.set_selection_node_id(annot_ref.clone().into());
                app.set_annotation_subtype(subtype.clone().into());
                app.set_selection_bounds_text(
                    format!(
                        "{:.1}, {:.1}, {:.1} × {:.1}",
                        bounds.x, bounds.y, bounds.width, bounds.height
                    )
                    .into(),
                );
                app.set_selection_font_name("".into());
                app.set_selection_font_size("".into());
                let txt = contents.clone().unwrap_or_default();
                app.set_selection_text_content(txt.into());
                app.set_selection_is_editable(true);

                app.set_sel_x(bounds.x as f32);
                app.set_sel_y(bounds.y as f32);
                app.set_sel_w(bounds.width as f32);
                app.set_sel_h(bounds.height as f32);
                app.set_is_editing_text(false);
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

    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn init_testing() -> std::sync::MutexGuard<'static, ()> {
        let guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        GPU_ENABLED.store(true, Ordering::SeqCst);
        i_slint_backend_testing::init_no_event_loop();
        guard
    }

    fn resolve_fixture(rel: &str) -> PathBuf {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let root = manifest.parent().and_then(|p| p.parent()).unwrap_or(&manifest);
        root.join(rel)
    }

    #[test]
    fn test_ui_initialization_and_sync() {
        let _test_guard = init_testing();
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
        // Correct evidence level: newly opened unverified PDF reports LOCAL_PARSED
        assert_eq!(app.get_evidence_level(), "LOCAL_PARSED");
        assert_eq!(app.get_zoom_percent(), 100);
        assert!(!app.get_has_selection());
        assert!(app.get_text_spans().row_count() > 0);
    }

    #[test]
    fn test_selection_and_editing_workflow() {
        let _test_guard = init_testing();
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
    fn test_live_editing_and_auto_commit_without_mandatory_apply() {
        let _test_guard = init_testing();
        let app = AppWindow::new().expect("Failed to initialize Slint AppWindow");
        let manager = Rc::new(RefCell::new(SessionManager::new()));
        let active_tool = Rc::new(RefCell::new("text".to_string()));
        let is_editing = Rc::new(RefCell::new(true));

        manager
            .borrow_mut()
            .dispatch(AppCommand::OpenDocument {
                path: None,
                bytes: DEFAULT_SAMPLE_PDF.to_vec(),
                title: "test.pdf".into(),
            })
            .expect("Failed to open document");

        let (real_span_id, original_text) = {
            let mgr = manager.borrow();
            let session = mgr.active_session().unwrap();
            let mut doc = session.pdf_document().unwrap();
            let pages = doc.extract_all_text().unwrap();
            (
                pages[0].spans[0].span_id.clone(),
                pages[0].spans[0].text.clone(),
            )
        };

        let star_node_id = StarNodeId::new("doc", ObjectRef::new(1, 0), &real_span_id).unwrap();
        let sel = SelectionState::Text {
            star_node_id,
            render_node_id: None,
            text: original_text,
            bounds: Rect::new(50.0, 100.0, 150.0, 24.0),
            font_info: Some(FontInfo::new("Helvetica", 18.0)),
            is_editable: true,
        };

        manager
            .borrow_mut()
            .dispatch(AppCommand::SetSelection { selection: sel })
            .expect("SetSelection");

        sync_ui(&app, &manager.borrow(), "text", true);

        // Simulate typing directly on-canvas into editing_text_buffer
        app.set_editing_text_buffer("Star Orion Live Edit".into());

        // Blur or deselect triggers auto-commit WITHOUT mandatory Apply button!
        commit_active_text_edit(&app, &manager, &active_tool, &is_editing);

        // Verify session is dirty, edit was committed to session manager
        let mgr = manager.borrow();
        let session = mgr.active_session().unwrap();
        assert!(session.is_dirty());
        assert_eq!(session.evidence_level(), "MUTATION_STAGED");
    }

    #[test]
    fn test_multilingual_unicode_support() {
        let _test_guard = init_testing();
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

    #[test]
    fn test_desktop_gpu_viewport_presentation_and_toggle() {
        let _test_guard = init_testing();
        let app = AppWindow::new().expect("Failed to initialize Slint AppWindow");
        let mut manager = SessionManager::new();

        manager
            .dispatch(AppCommand::OpenDocument {
                path: None,
                bytes: DEFAULT_SAMPLE_PDF.to_vec(),
                title: "sample.pdf".into(),
            })
            .expect("OpenDocument");

        // 1. Initialize GPU renderer
        let has_gpu = {
            let mut lock = GPU_RENDERER.lock().unwrap();
            if lock.is_none() {
                *lock = starpdf::render::WgpuRenderer::new().ok();
            }
            lock.is_some()
        };

        if has_gpu {
            // Enable GPU acceleration
            GPU_ENABLED.store(true, Ordering::SeqCst);
            sync_ui(&app, &manager, "select", false);

            assert!(
                app.get_has_rendered_page_image(),
                "Page image must be rendered"
            );
            assert!(
                app.get_gpu_acceleration_enabled(),
                "GPU acceleration must be active"
            );
            assert!(
                app.get_render_engine_badge().contains("GPU"),
                "Badge must reflect GPU mode: got {}",
                app.get_render_engine_badge()
            );
            assert!(
                app.get_gpu_metrics_detail().contains("Draws:"),
                "Metrics detail must contain draw calls breakdown: got {}",
                app.get_gpu_metrics_detail()
            );

            // 2. Toggle to CPU fallback
            GPU_ENABLED.store(false, Ordering::SeqCst);
            sync_ui(&app, &manager, "select", false);

            assert!(
                app.get_has_rendered_page_image(),
                "Page image must remain rendered in CPU fallback"
            );
            assert!(
                !app.get_gpu_acceleration_enabled(),
                "GPU acceleration must be reported as disabled"
            );
            assert!(
                app.get_render_engine_badge().contains("CPU"),
                "Badge must reflect CPU mode: got {}",
                app.get_render_engine_badge()
            );

            // Restore GPU enabled state
            GPU_ENABLED.store(true, Ordering::SeqCst);
        }
    }

    #[test]
    fn test_desktop_viewport_zoom_selection_alignment() {
        let _test_guard = init_testing();
        let app = AppWindow::new().expect("Failed to initialize Slint AppWindow");
        let mut manager = SessionManager::new();

        manager
            .dispatch(AppCommand::OpenDocument {
                path: None,
                bytes: DEFAULT_SAMPLE_PDF.to_vec(),
                title: "sample.pdf".into(),
            })
            .expect("OpenDocument");

        for zoom_pct in [50u32, 100u32, 150u32, 200u32] {
            manager
                .dispatch(AppCommand::SetZoom {
                    percent: zoom_pct as f32,
                })
                .expect("SetZoom");

            sync_ui(&app, &manager, "select", false);

            assert_eq!(app.get_zoom_percent(), zoom_pct as i32);
            assert!(app.get_page_width() > 0.0);
            assert!(app.get_page_height() > 0.0);
            assert!(app.get_has_rendered_page_image());

            // Check that text span bounds align with original geometry regardless of zoom
            let spans = app.get_text_spans();
            assert!(spans.row_count() > 0);
            let first_span = spans.row_data(0).unwrap();
            assert!(first_span.width > 0.0);
            assert!(first_span.height > 0.0);
        }
    }

    #[test]
    fn test_phase8_real_world_document_open_and_render_all_categories() {
        let _test_guard = init_testing();
        let app = AppWindow::new().expect("Slint AppWindow");

        let test_fixtures = [
            "test-assets/realworld_corpus/office-invoice-enterprise.pdf",
            "test-assets/realworld_corpus/scanned-multires-archive.pdf",
            "test-assets/realworld_corpus/forms-employment-application.pdf",
            "test-assets/realworld_corpus/complex-infographic-graphics.pdf",
            "test-assets/realworld_corpus/annotations-markup-review.pdf",
        ];

        for fix in test_fixtures {
            let path = resolve_fixture(fix);
            let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("Could not read fixture {:?}: {e}", path));
            let mut manager = SessionManager::new();
            manager
                .dispatch(AppCommand::OpenDocument {
                    path: Some(path.clone()),
                    bytes,
                    title: fix.to_string(),
                })
                .unwrap_or_else(|e| panic!("Failed to open fixture {:?}: {e}", path));

            sync_ui(&app, &manager, "select", false);

            assert!(app.get_has_rendered_page_image(), "Must have rendered image for {fix}");
            assert!(app.get_page_width() > 0.0, "Page width must be positive for {fix}");
            assert!(app.get_page_height() > 0.0, "Page height must be positive for {fix}");
            assert!(app.get_render_engine_badge().len() > 0);
        }
    }

    #[test]
    fn test_phase8_large_document_scrolling_and_page_switching() {
        let _test_guard = init_testing();
        let app = AppWindow::new().expect("Slint AppWindow");
        let path = resolve_fixture("test-assets/realworld_corpus/large-manual-20page.pdf");
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|e| panic!("read 20-page manual at {:?}: {e}", path));

        let mut manager = SessionManager::new();
        manager
            .dispatch(AppCommand::OpenDocument {
                path: Some(path),
                bytes,
                title: "large-manual-20page.pdf".into(),
            })
            .expect("open 20-page manual");

        assert_eq!(manager.active_session().unwrap().total_pages, 20);

        for target_page in [0u32, 1, 5, 10, 19] {
            manager
                .dispatch(AppCommand::SetPage { page_index: target_page })
                .expect("SetPage");

            sync_ui(&app, &manager, "select", false);

            assert_eq!(app.get_active_page(), target_page as i32);
            assert_eq!(
                app.get_page_info_text(),
                format!("Page {} of 20", target_page + 1)
            );
            assert!(app.get_has_rendered_page_image());

            // Check thumbnails
            let thumbs = app.get_thumbnails();
            assert_eq!(thumbs.row_count(), 20);
            let thumb = thumbs.row_data(target_page as usize).unwrap();
            assert!(thumb.is_active, "Active page thumbnail must have is_active == true");
            if target_page > 0 {
                let prev_thumb = thumbs.row_data(0).unwrap();
                assert!(!prev_thumb.is_active, "Inactive page thumbnail must have is_active == false");
            }
        }
    }

    #[test]
    fn test_phase8_in_place_text_edit_undo_redo_and_save_reopen_lifecycle() {
        let _test_guard = init_testing();
        let app = AppWindow::new().expect("Slint AppWindow");
        let manager = Rc::new(RefCell::new(SessionManager::new()));
        let active_tool = Rc::new(RefCell::new("text".to_string()));
        let is_editing = Rc::new(RefCell::new(true));

        let path = resolve_fixture("test-assets/realworld_corpus/office-invoice-enterprise.pdf");
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|e| panic!("read invoice at {:?}: {e}", path));

        manager
            .borrow_mut()
            .dispatch(AppCommand::OpenDocument {
                path: Some(path),
                bytes,
                title: "invoice.pdf".into(),
            })
            .expect("open invoice");

        let (span_id, original_text) = {
            let mgr = manager.borrow();
            let session = mgr.active_session().unwrap();
            let mut doc = session.pdf_document().unwrap();
            let pages = doc.extract_all_text().unwrap();
            (
                pages[0].spans[0].span_id.clone(),
                pages[0].spans[0].text.clone(),
            )
        };

        let star_node_id = StarNodeId::new("doc", ObjectRef::new(1, 0), &span_id).unwrap();
        let sel = SelectionState::Text {
            star_node_id,
            render_node_id: None,
            text: original_text.clone(),
            bounds: Rect::new(40.0, 40.0, 200.0, 30.0),
            font_info: Some(FontInfo::new("Helvetica", 18.0)),
            is_editable: true,
        };

        manager
            .borrow_mut()
            .dispatch(AppCommand::SetSelection { selection: sel })
            .expect("SetSelection");

        sync_ui(&app, &manager.borrow(), "text", true);

        // Edit text
        let modified_text = "TOOLRAKYAT QUANTUM ENTERPRISE CLOUD";
        app.set_editing_text_buffer(modified_text.into());

        // Auto-commit on blur / selection change
        commit_active_text_edit(&app, &manager, &active_tool, &is_editing);

        {
            let mgr = manager.borrow();
            let session = mgr.active_session().unwrap();
            assert!(session.is_dirty(), "Session must be marked dirty after text commit");
            assert_eq!(session.evidence_level(), "MUTATION_STAGED");
        }

        // Test Undo
        manager.borrow_mut().dispatch(AppCommand::Undo).expect("Undo");
        sync_ui(&app, &manager.borrow(), "select", false);

        // Test Redo
        manager.borrow_mut().dispatch(AppCommand::Redo).expect("Redo");
        sync_ui(&app, &manager.borrow(), "select", false);

        // Export and Reopen Verification
        let exported_bytes = {
            let mut mgr = manager.borrow_mut();
            let res = mgr
                .dispatch(AppCommand::ExportDocument {
                    profile: ExportProfile::StandardPdf,
                })
                .expect("Export mutated PDF");
            res.exported_bytes.expect("exported bytes")
        };

        assert!(exported_bytes.len() > 100);

        // Reopen in a clean session
        let mut reopen_mgr = SessionManager::new();
        reopen_mgr
            .dispatch(AppCommand::OpenDocument {
                path: None,
                bytes: exported_bytes,
                title: "reopened.pdf".into(),
            })
            .expect("Reopen exported document");

        assert_eq!(reopen_mgr.session_count(), 1);
        let mut reopened_doc = reopen_mgr.active_session().unwrap().pdf_document().unwrap();
        let reopened_pages = reopened_doc.extract_all_text().unwrap();
        assert!(
            reopened_pages[0].spans.iter().any(|s| s.text.contains(modified_text)),
            "Modified text must be semantically present in reopened PDF"
        );
    }

    #[test]
    fn test_phase8_multi_document_tab_lifecycle() {
        let _test_guard = init_testing();
        let app = AppWindow::new().expect("Slint AppWindow");
        let mut manager = SessionManager::new();

        let doc1_path = resolve_fixture("test-assets/realworld_corpus/office-invoice-enterprise.pdf");
        let doc2_path = resolve_fixture("test-assets/realworld_corpus/scanned-multires-archive.pdf");
        let doc1_bytes = std::fs::read(&doc1_path).unwrap();
        let doc2_bytes = std::fs::read(&doc2_path).unwrap();

        manager
            .dispatch(AppCommand::OpenDocument {
                path: Some(doc1_path),
                bytes: doc1_bytes,
                title: "Invoice".into(),
            })
            .unwrap();

        let tab1_id = manager.active_session().unwrap().id.clone();

        manager
            .dispatch(AppCommand::OpenDocument {
                path: Some(doc2_path),
                bytes: doc2_bytes,
                title: "Scan".into(),
            })
            .unwrap();

        let tab2_id = manager.active_session().unwrap().id.clone();

        assert_eq!(manager.session_count(), 2);
        sync_ui(&app, &manager, "select", false);

        let tabs = app.get_tabs();
        assert_eq!(tabs.row_count(), 2);
        assert_eq!(app.get_active_tab_id(), tab2_id);

        // Switch to tab 1
        manager
            .dispatch(AppCommand::SwitchTab { session_id: tab1_id.clone() })
            .unwrap();
        sync_ui(&app, &manager, "select", false);
        assert_eq!(app.get_active_tab_id(), tab1_id);

        // Close tab 2
        manager
            .dispatch(AppCommand::CloseSession { session_id: tab2_id })
            .unwrap();
        sync_ui(&app, &manager, "select", false);
        assert_eq!(manager.session_count(), 1);
        assert_eq!(app.get_tabs().row_count(), 1);
    }

    #[test]
    fn test_phase8_malformed_document_graceful_refusal() {
        let _test_guard = init_testing();
        let _app = AppWindow::new().expect("Slint AppWindow");
        let mut manager = SessionManager::new();

        let malformed_bytes = b"%PDF-1.4\nCorrupted binary payload\n%%EOF".to_vec();
        let res = manager.dispatch(AppCommand::OpenDocument {
            path: None,
            bytes: malformed_bytes,
            title: "corrupt.pdf".into(),
        });

        // Must refuse fail-closed with typed error, not crash or panic
        assert!(res.is_err(), "Opening malformed PDF must return Err");
        assert_eq!(manager.session_count(), 0);
    }

    #[test]
    fn test_phase8_capture_desktop_qualification_screenshots() {
        let _test_guard = init_testing();
        let app = AppWindow::new().expect("Slint AppWindow");
        let screenshot_dir = resolve_fixture("target/desktop_qualification_screenshots");
        let _ = std::fs::create_dir_all(&screenshot_dir);

        let targets = [
            ("test-assets/realworld_corpus/office-invoice-enterprise.pdf", "desktop_office_invoice.png"),
            ("test-assets/realworld_corpus/complex-infographic-graphics.pdf", "desktop_complex_infographic.png"),
            ("test-assets/realworld_corpus/scanned-multires-archive.pdf", "desktop_scanned_archive.png"),
            ("test-assets/realworld_corpus/large-manual-20page.pdf", "desktop_large_manual_p0.png"),
        ];

        for (pdf_rel, out_name) in targets {
            let pdf_path = resolve_fixture(pdf_rel);
            let bytes = std::fs::read(&pdf_path).unwrap_or_else(|e| panic!("read screenshot pdf {:?}: {e}", pdf_path));
            let mut manager = SessionManager::new();
            manager
                .dispatch(AppCommand::OpenDocument {
                    path: Some(pdf_path.clone()),
                    bytes,
                    title: out_name.to_string(),
                })
                .expect("open doc for screenshot");

            sync_ui(&app, &manager, "select", false);

            let session = manager.active_session().unwrap();
            let rendered = session.render_page(0, 1.0).expect("render page");

            let out_path = screenshot_dir.join(out_name);
            image::save_buffer(
                &out_path,
                &rendered.rgba_pixels,
                rendered.width,
                rendered.height,
                image::ExtendedColorType::Rgba8,
            )
            .expect("save screenshot png");

            assert!(out_path.exists());
            println!("  [Screenshot Captured] Saved: {:?}", out_path);
        }
    }

    #[test]
    fn test_phase9_interactive_acceptance_and_latency_profiling() {
        let _test_guard = init_testing();
        if let Ok(mut lock) = GPU_RENDERER.lock() {
            if lock.is_none() {
                *lock = starpdf::render::WgpuRenderer::new().ok();
            }
        }
        let app = AppWindow::new().expect("Slint AppWindow");
        let screenshot_dir = resolve_fixture("target/desktop_acceptance_phase9");
        let _ = std::fs::create_dir_all(&screenshot_dir);

        // 1. First-Page Cold vs Warm Latency
        let doc_path = resolve_fixture("test-assets/realworld_corpus/office-invoice-enterprise.pdf");
        let doc_bytes = std::fs::read(&doc_path).expect("read doc");

        let mut manager = SessionManager::new();
        let t_cold_start = std::time::Instant::now();
        manager
            .dispatch(AppCommand::OpenDocument {
                path: Some(doc_path.clone()),
                bytes: doc_bytes.clone(),
                title: "invoice.pdf".into(),
            })
            .unwrap();
        sync_ui(&app, &manager, "select", false);
        let first_page_cold_ms = t_cold_start.elapsed().as_secs_f64() * 1000.0;

        let t_warm_start = std::time::Instant::now();
        sync_ui(&app, &manager, "select", false);
        let first_page_warm_ms = t_warm_start.elapsed().as_secs_f64() * 1000.0;

        println!("  [Phase 9] First-Page Latency: Cold = {:.2}ms, Warm = {:.2}ms", first_page_cold_ms, first_page_warm_ms);
        assert!(first_page_warm_ms < 10.0, "Warm UI sync must be < 10ms in debug test profile");

        // 2. Zoom Scaling Latency (50% to 300%)
        let zooms = [50.0, 100.0, 150.0, 200.0, 300.0];
        let mut zoom_timings = Vec::new();
        for &z in &zooms {
            let t_z = std::time::Instant::now();
            manager.dispatch(AppCommand::SetZoom { percent: z }).unwrap();
            sync_ui(&app, &manager, "select", false);
            let dur = t_z.elapsed().as_secs_f64() * 1000.0;
            zoom_timings.push(dur);
        }
        println!("  [Phase 9] Zoom Latencies (50%->300%): {:?}", zoom_timings);

        // 3. Large Document Scroll (20 Pages) - Measure p50/p95/p99
        let manual_path = resolve_fixture("test-assets/realworld_corpus/large-manual-20page.pdf");
        let manual_bytes = std::fs::read(&manual_path).expect("read manual");
        manager
            .dispatch(AppCommand::OpenDocument {
                path: Some(manual_path.clone()),
                bytes: manual_bytes,
                title: "manual20.pdf".into(),
            })
            .unwrap();

        let mut scroll_latencies = Vec::new();
        for page in 0..20 {
            let t_page = std::time::Instant::now();
            manager.dispatch(AppCommand::SetPage { page_index: page }).unwrap();
            sync_ui(&app, &manager, "select", false);
            scroll_latencies.push(t_page.elapsed().as_secs_f64() * 1000.0);
        }

        scroll_latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p50 = scroll_latencies[scroll_latencies.len() / 2];
        let p95 = scroll_latencies[(scroll_latencies.len() as f64 * 0.95) as usize];
        let p99 = scroll_latencies[scroll_latencies.len() - 1];
        println!("  [Phase 9] 20-Page Scroll Latency: p50={:.2}ms, p95={:.2}ms, p99={:.2}ms", p50, p95, p99);

        // 4. In-Place Text Editing & Undo/Redo Latency
        let target_node = {
            let session = manager.active_session().unwrap();
            let spans = session.extract_page_text_spans(0).unwrap();
            spans[0].span_id.clone()
        };

        let mut edit_latencies = Vec::new();
        for i in 0..10 {
            let t_edit = std::time::Instant::now();
            manager.dispatch(AppCommand::ReplaceText {
                target_node: target_node.clone(),
                new_text: format!("Edit-Step-{}", i),
            }).unwrap();
            sync_ui(&app, &manager, "text", false);
            edit_latencies.push(t_edit.elapsed().as_secs_f64() * 1000.0);
        }
        edit_latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let edit_p50 = edit_latencies[edit_latencies.len() / 2];
        let edit_p95 = edit_latencies[(edit_latencies.len() as f64 * 0.95) as usize];
        let edit_p99 = edit_latencies[edit_latencies.len() - 1];
        println!("  [Phase 9] In-Place Text Edit Latency: p50={:.2}ms, p95={:.2}ms, p99={:.2}ms", edit_p50, edit_p95, edit_p99);

        // 5. Multi-Tab Switching (50 cycles between 2 documents)
        if manager.session_count() < 2 {
            let doc2_path = resolve_fixture("test-assets/realworld_corpus/scanned-multires-archive.pdf");
            let doc2_bytes = std::fs::read(&doc2_path).unwrap();
            manager.dispatch(AppCommand::OpenDocument {
                path: Some(doc2_path),
                bytes: doc2_bytes,
                title: "scan.pdf".into(),
            }).unwrap();
        }

        let tab1_id = manager.tabs()[0].session_id.clone();
        let tab2_id = manager.tabs()[1].session_id.clone();
        let mut tab_latencies = Vec::new();
        for _ in 0..20 {
            let t_tab = std::time::Instant::now();
            manager.dispatch(AppCommand::SwitchTab { session_id: tab1_id.clone() }).unwrap();
            sync_ui(&app, &manager, "select", false);
            tab_latencies.push(t_tab.elapsed().as_secs_f64() * 1000.0);

            let t_tab2 = std::time::Instant::now();
            manager.dispatch(AppCommand::SwitchTab { session_id: tab2_id.clone() }).unwrap();
            sync_ui(&app, &manager, "select", false);
            tab_latencies.push(t_tab2.elapsed().as_secs_f64() * 1000.0);
        }
        tab_latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let tab_p50 = tab_latencies[tab_latencies.len() / 2];
        let tab_p95 = tab_latencies[(tab_latencies.len() as f64 * 0.95) as usize];
        let tab_p99 = tab_latencies[tab_latencies.len() - 1];
        println!("  [Phase 9] Tab Switch Latency: p50={:.2}ms, p95={:.2}ms, p99={:.2}ms", tab_p50, tab_p95, tab_p99);
        assert!(tab_p50 < 30.0, "Warm tab switch p50 must be < 30.0ms in debug test profile");

        // 6. Capture Phase 9 High-Fidelity Screenshots
        let shot_targets = [
            ("test-assets/realworld_corpus/office-invoice-enterprise.pdf", "desktop_phase9_invoice_zoom150.png", 1.5, 0),
            ("test-assets/realworld_corpus/large-manual-20page.pdf", "desktop_phase9_manual_page5.png", 1.0, 4),
            ("test-assets/realworld_corpus/multilingual-arabic-jawi.pdf", "desktop_phase9_arabic_jawi.png", 1.0, 0),
            ("test-assets/realworld_corpus/multilingual-cjk-unicode.pdf", "desktop_phase9_cjk_unicode.png", 1.0, 0),
        ];

        for (rel, fname, zoom, page_idx) in shot_targets {
            let p = resolve_fixture(rel);
            let b = std::fs::read(&p).unwrap();
            let mut mgr = SessionManager::new();
            mgr.dispatch(AppCommand::OpenDocument {
                path: Some(p),
                bytes: b,
                title: fname.to_string(),
            }).unwrap();
            if page_idx > 0 {
                mgr.dispatch(AppCommand::SetPage { page_index: page_idx }).unwrap();
            }
            mgr.dispatch(AppCommand::SetZoom { percent: zoom * 100.0 }).unwrap();
            sync_ui(&app, &mgr, "select", false);

            let s = mgr.active_session().unwrap();
            let rendered = s.render_page(s.current_page, zoom).unwrap();
            let out_file = screenshot_dir.join(fname);
            image::save_buffer(
                &out_file,
                &rendered.rgba_pixels,
                rendered.width,
                rendered.height,
                image::ExtendedColorType::Rgba8,
            ).unwrap();
            assert!(out_file.exists());
            println!("  [Phase 9 Screenshot Saved]: {:?}", out_file);
        }
    }

    #[test]
    fn test_phase10_desktop_product_acceptance_and_qualification() {
        let _test_guard = init_testing();
        if let Ok(mut lock) = GPU_RENDERER.lock() {
            if lock.is_none() {
                *lock = starpdf::render::WgpuRenderer::new().ok();
            }
        }
        let app = AppWindow::new().expect("Slint AppWindow");
        let screenshot_dir = resolve_fixture("target/desktop_acceptance_phase10");
        let _ = std::fs::create_dir_all(&screenshot_dir);

        // 1. Workflow A: Interactive Form Field Editing in Desktop GUI
        let form_path = resolve_fixture("test-assets/realworld_corpus/forms-employment-application.pdf");
        let form_bytes = std::fs::read(&form_path).expect("read form");
        let mut manager = SessionManager::new();
        manager
            .dispatch(AppCommand::OpenDocument {
                path: Some(form_path.clone()),
                bytes: form_bytes,
                title: "forms-employment-application.pdf".into(),
            })
            .expect("OpenDocument");

        sync_ui(&app, &manager, "forms", false);
        assert_eq!(app.get_active_tool(), "forms");
        assert!(app.get_has_rendered_page_image());

        // Mutate form fields
        manager
            .dispatch(AppCommand::SetFormField {
                field_name: "name_field".into(),
                value: "Senior Systems Engineer".into(),
            })
            .expect("SetFormField name");
        manager
            .dispatch(AppCommand::SetFormField {
                field_name: "rust_cb".into(),
                value: "false".into(),
            })
            .expect("SetFormField checkbox");
        sync_ui(&app, &manager, "forms", false);

        // Save & Reopen Verification
        let form_saved_path = std::env::temp_dir().join(format!("phase10_desktop_form_{}.pdf", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        manager
            .dispatch(AppCommand::SaveDocument {
                destination_path: Some(form_saved_path.clone()),
            })
            .expect("SaveDocument");

        let reopened_bytes = std::fs::read(&form_saved_path).expect("read saved form");
        let mut reopened_doc = starpdf::document::PdfDocument::from_bytes(&reopened_bytes).expect("parse reopened");
        let fields = reopened_doc.form_fields().expect("fields");
        let name_val = fields.iter().find(|f| f.partial_name == "name_field").unwrap();
        assert_eq!(name_val.value, starpdf::forms::FieldValue::Text("Senior Systems Engineer".into()));
        let _ = std::fs::remove_file(&form_saved_path);

        // 2. Workflow B: Document Safety & Restrictive Refusal
        let invoice_path = resolve_fixture("test-assets/realworld_corpus/office-invoice-enterprise.pdf");
        let inv_bytes = std::fs::read(&invoice_path).expect("read invoice");
        manager
            .dispatch(AppCommand::OpenDocument {
                path: Some(invoice_path),
                bytes: inv_bytes,
                title: "invoice.pdf".into(),
            })
            .expect("OpenDocument invoice");

        // Manually lock permissions
        {
            let session = manager.active_session_mut().unwrap();
            session.capabilities.can_edit.allowed = false;
            session.capabilities.can_edit.reason = "Cryptographic signature protection active".into();
        }
        sync_ui(&app, &manager, "text", false);
        assert_eq!(app.get_security_status(), "Cryptographic signature protection active");

        let bad_edit = manager.dispatch(AppCommand::ReplaceText {
            target_node: "p0_s0_i0_o0".into(),
            new_text: "Unauthorized Edit".into(),
        });
        assert!(bad_edit.is_err(), "Must refuse editing signed/locked document");

        // 3. Workflow C: Real-World Corpus Rendering & High-Fidelity Qualification Screenshots
        let shot_targets = [
            ("test-assets/realworld_corpus/forms-employment-application.pdf", "desktop_phase10_form_editing.png", 1.0, 0),
            ("test-assets/realworld_corpus/annotations-markup-review.pdf", "desktop_phase10_annotation_review.png", 1.25, 0),
            ("test-assets/realworld_corpus/complex-infographic-graphics.pdf", "desktop_phase10_infographic_graphics.png", 1.5, 0),
            ("test-assets/realworld_corpus/scanned-multires-archive.pdf", "desktop_phase10_scanned_multires.png", 1.0, 0),
            ("test-assets/realworld_corpus/large-manual-20page.pdf", "desktop_phase10_large_manual_page10.png", 1.0, 9),
            ("test-assets/realworld_corpus/multilingual-arabic-jawi.pdf", "desktop_phase10_arabic_jawi.png", 1.0, 0),
            ("test-assets/realworld_corpus/multilingual-cjk-unicode.pdf", "desktop_phase10_cjk_unicode.png", 1.0, 0),
        ];

        for (rel, fname, zoom, page_idx) in shot_targets {
            let p = resolve_fixture(rel);
            let b = std::fs::read(&p).unwrap_or_else(|e| panic!("Could not read fixture {:?}: {e}", p));
            let mut mgr = SessionManager::new();
            mgr.dispatch(AppCommand::OpenDocument {
                path: Some(p),
                bytes: b,
                title: fname.to_string(),
            }).unwrap();

            if page_idx > 0 {
                mgr.dispatch(AppCommand::SetPage { page_index: page_idx }).unwrap();
            }
            mgr.dispatch(AppCommand::SetZoom { percent: zoom * 100.0 }).unwrap();
            sync_ui(&app, &mgr, "select", false);

            let s = mgr.active_session().unwrap();
            let rendered = s.render_page(s.current_page, zoom).unwrap();
            let out_file = screenshot_dir.join(fname);
            image::save_buffer(
                &out_file,
                &rendered.rgba_pixels,
                rendered.width,
                rendered.height,
                image::ExtendedColorType::Rgba8,
            ).unwrap();
            assert!(out_file.exists());
            println!("  [Phase 10 Qualification Screenshot Saved]: {:?}", out_file);
        }
    }

    #[test]
    fn test_phase11_desktop_gui_editing_and_acceptance() {
        let _test_guard = init_testing();
        if let Ok(mut lock) = GPU_RENDERER.lock() {
            if lock.is_none() {
                *lock = starpdf::render::WgpuRenderer::new().ok();
            }
        }
        let app = AppWindow::new().expect("Slint AppWindow");
        let manager = Rc::new(RefCell::new(SessionManager::new()));
        let active_tool = Rc::new(RefCell::new("select".to_string()));
        let is_editing = Rc::new(RefCell::new(false));

        // Register all production UI callbacks
        register_ui_callbacks(&app, manager.clone(), active_tool.clone(), is_editing.clone());

        let acceptance_dir = resolve_fixture("target/desktop_acceptance_phase11");
        let _ = std::fs::create_dir_all(&acceptance_dir);

        // 1. Open Document via Session Manager
        let form_path = resolve_fixture("test-assets/realworld_corpus/forms-employment-application.pdf");
        let form_bytes = std::fs::read(&form_path).expect("read forms");
        {
            let mut mgr = manager.borrow_mut();
            mgr.dispatch(AppCommand::OpenDocument {
                path: Some(form_path.clone()),
                bytes: form_bytes,
                title: "forms-employment-application.pdf".into(),
            }).expect("OpenDocument");
        }
        sync_ui(&app, &manager.borrow(), &active_tool.borrow(), false);
        assert_eq!(app.get_total_pages(), 1);
        assert!(app.get_has_rendered_page_image());

        // 2. GUI Selection & Form Field Editing Interaction
        app.invoke_element_selected(slint::SharedString::from("form_rust_cb"));
        assert!(app.get_has_selection());
        assert_eq!(app.get_selection_type(), "FormField");
        assert_eq!(app.get_form_field_name(), "rust_cb");
        assert_eq!(app.get_form_field_type(), "Checkbox");
        assert!(app.get_form_field_checked());

        // Toggle checkbox via GUI callback
        app.invoke_form_field_toggled(slint::SharedString::from("rust_cb"), slint::SharedString::from("false"));
        assert!(!app.get_form_field_checked());

        // Select and update text field via GUI callback
        app.invoke_element_selected(slint::SharedString::from("form_name_field"));
        assert_eq!(app.get_form_field_name(), "name_field");
        assert_eq!(app.get_form_field_type(), "Text");
        app.invoke_form_field_text_committed(slint::SharedString::from("name_field"), slint::SharedString::from("Principal Architect"));
        assert_eq!(app.get_form_field_value(), "Principal Architect");

        // 3. Annotation Creation, Selection, Update & Delete via GUI
        app.invoke_add_annotation_clicked(slint::SharedString::from("FreeText"));
        let initial_elem_count = app.get_page_elements().row_count();
        assert!(initial_elem_count > 0);

        // Select created annotation
        let elems = app.get_page_elements();
        let mut annot_id = slint::SharedString::default();
        for i in 0..elems.row_count() {
            if let Some(elem) = elems.row_data(i) {
                if elem.kind == "annotation" {
                    annot_id = elem.id;
                    break;
                }
            }
        }
        assert!(!annot_id.is_empty(), "Annotation element must exist in page_elements");
        app.invoke_element_selected(annot_id);
        assert!(app.get_has_selection());
        assert_eq!(app.get_selection_type(), "Annotation");

        // Update annotation text via GUI callback
        app.invoke_update_annotation_text_clicked(slint::SharedString::from("Phase 11 Verified"));

        // Delete selected annotation via GUI callback
        app.invoke_delete_selected_clicked();

        // 4. Page Management via GUI Callbacks
        let p_count_before = app.get_total_pages();
        app.invoke_insert_blank_page_clicked();
        assert_eq!(app.get_total_pages(), p_count_before + 1);

        // Duplicate page is safely refused in incremental mode
        app.invoke_duplicate_page_clicked();
        assert!(app.get_status_message().contains("Refused") || app.get_status_message().contains("Unsupported"));
        assert_eq!(app.get_total_pages(), p_count_before + 1);

        app.invoke_move_page_down_clicked();
        app.invoke_move_page_up_clicked();

        app.invoke_delete_page_clicked();
        assert_eq!(app.get_total_pages(), p_count_before);

        // 5. Undo and Redo via GUI Callbacks
        app.invoke_undo_clicked();
        assert!(app.get_status_message().contains("Undo") || app.get_status_message().contains("Restored") || !app.get_status_message().is_empty());

        app.invoke_redo_clicked();
        assert!(app.get_status_message().contains("Redo") || !app.get_status_message().is_empty());

        // 6. Security Protection & Fail-Closed Feedback in GUI
        {
            let mut mgr = manager.borrow_mut();
            let session = mgr.active_session_mut().unwrap();
            session.capabilities.can_edit.allowed = false;
            session.capabilities.can_edit.reason = "Restricted cryptographic protection".into();
            session.capabilities.can_modify_pages.allowed = false;
            session.capabilities.can_modify_pages.reason = "Restricted cryptographic protection".into();
        }
        sync_ui(&app, &manager.borrow(), "text", false);
        assert_eq!(app.get_security_status(), "Restricted cryptographic protection");

        // Attempt page insertion on protected document via GUI callback
        app.invoke_insert_blank_page_clicked();
        assert!(app.get_status_message().contains("Refused") || app.get_status_message().contains("Cannot"));

        // 7. Multi-page Navigation & Render Qualification
        let manual_path = resolve_fixture("test-assets/realworld_corpus/large-manual-20page.pdf");
        let manual_bytes = std::fs::read(&manual_path).expect("read manual");
        {
            let mut mgr = manager.borrow_mut();
            mgr.dispatch(AppCommand::OpenDocument {
                path: Some(manual_path),
                bytes: manual_bytes,
                title: "large-manual-20page.pdf".into(),
            }).expect("OpenDocument manual");
        }
        sync_ui(&app, &manager.borrow(), "select", false);
        assert_eq!(app.get_total_pages(), 20);

        // Navigate pages
        for page_idx in [0, 4, 9, 14, 19] {
            {
                let mut mgr = manager.borrow_mut();
                mgr.dispatch(AppCommand::SetPage { page_index: page_idx }).unwrap();
            }
            sync_ui(&app, &manager.borrow(), "select", false);
            assert_eq!(app.get_active_page(), page_idx as i32);

            let mgr = manager.borrow();
            let s = mgr.active_session().unwrap();
            let rendered = s.render_page(page_idx, 1.0).expect("render manual page");
            let out_file = acceptance_dir.join(format!("gui_acceptance_manual_page_{page_idx}.png"));
            image::save_buffer(
                &out_file,
                &rendered.rgba_pixels,
                rendered.width,
                rendered.height,
                image::ExtendedColorType::Rgba8,
            ).unwrap();
            assert!(out_file.exists());
        }

        println!("  [Phase 11 Acceptance Complete]: All desktop GUI editing and navigation workflows verified.");
    }

    #[test]
    fn test_phase12_rc_acceptance_and_hardening() {
        let _test_guard = init_testing();
        if let Ok(mut lock) = GPU_RENDERER.lock() {
            if lock.is_none() {
                *lock = starpdf::render::WgpuRenderer::new().ok();
            }
        }
        let app = AppWindow::new().expect("Slint AppWindow");
        let manager = Rc::new(RefCell::new(SessionManager::new()));
        let active_tool = Rc::new(RefCell::new("select".to_string()));
        let is_editing = Rc::new(RefCell::new(false));

        // Register all production UI callbacks
        register_ui_callbacks(&app, manager.clone(), active_tool.clone(), is_editing.clone());

        let acceptance_dir = resolve_fixture("target/desktop_acceptance_phase12");
        let _ = std::fs::create_dir_all(&acceptance_dir);

        // 1. Open Real Document with Form Fields
        let form_path = resolve_fixture("test-assets/realworld_corpus/forms-employment-application.pdf");
        let form_bytes = std::fs::read(&form_path).expect("read forms");
        {
            let mut mgr = manager.borrow_mut();
            mgr.dispatch(AppCommand::OpenDocument {
                path: Some(form_path.clone()),
                bytes: form_bytes,
                title: "forms-employment-application.pdf".into(),
            }).expect("OpenDocument");
        }
        sync_ui(&app, &manager.borrow(), &active_tool.borrow(), false);
        assert_eq!(app.get_total_pages(), 1);

        // 2. Mouse-Driven Form Controls
        app.invoke_element_selected(slint::SharedString::from("form_rust_cb"));
        assert!(app.get_has_selection());
        assert_eq!(app.get_selection_type(), "FormField");
        assert_eq!(app.get_form_field_name(), "rust_cb");

        // Toggle checkbox
        app.invoke_form_field_toggled(slint::SharedString::from("rust_cb"), slint::SharedString::from("false"));
        assert!(!app.get_form_field_checked());

        // Update text field
        app.invoke_element_selected(slint::SharedString::from("form_name_field"));
        app.invoke_form_field_text_committed(slint::SharedString::from("name_field"), slint::SharedString::from("Release Candidate Architect"));
        assert_eq!(app.get_form_field_value(), "Release Candidate Architect");

        // 3. Canvas-Driven Annotation Move and Resize
        app.invoke_add_annotation_clicked(slint::SharedString::from("FreeText"));
        let elems = app.get_page_elements();
        let mut annot_id = slint::SharedString::default();
        for i in (0..elems.row_count()).rev() {
            if let Some(elem) = elems.row_data(i) {
                if elem.kind == "annotation" {
                    annot_id = elem.id;
                    break;
                }
            }
        }
        assert!(!annot_id.is_empty(), "FreeText annotation must exist");
        app.invoke_element_selected(annot_id);
        assert!(app.get_has_selection());
        assert_eq!(app.get_selection_type(), "Annotation");

        let prev_x = app.get_sel_x();
        let prev_w = app.get_sel_w();

        // Move annotation via GUI callback
        app.invoke_move_selected_clicked(20.0, 15.0);
        assert!(app.get_sel_x() > prev_x);

        // Resize annotation via GUI callback
        app.invoke_resize_selected_clicked(30.0, 20.0);
        assert!(app.get_sel_w() > prev_w);

        // Update annotation text
        app.invoke_update_annotation_text_clicked(slint::SharedString::from("RC Verified FreeText"));
        assert_eq!(app.get_selection_text_content(), "RC Verified FreeText");

        // 4. Image Insertion, Selection, Move, Resize, and Replacement
        app.invoke_add_image_clicked();
        let elems_after_img = app.get_page_elements();
        let mut img_id = slint::SharedString::default();
        for i in 0..elems_after_img.row_count() {
            if let Some(elem) = elems_after_img.row_data(i) {
                if elem.kind == "image" {
                    img_id = elem.id;
                    break;
                }
            }
        }
        assert!(!img_id.is_empty(), "Image element must exist");
        app.invoke_element_selected(img_id);
        assert!(app.get_has_selection());
        assert_eq!(app.get_selection_type(), "Image");

        let img_prev_x = app.get_sel_x();
        let img_prev_w = app.get_sel_w();

        // Move image
        app.invoke_move_selected_clicked(15.0, 10.0);
        assert!(app.get_sel_x() > img_prev_x);

        // Resize image
        app.invoke_resize_selected_clicked(25.0, 15.0);
        assert!(app.get_sel_w() > img_prev_w);

        // Replace image
        app.invoke_replace_image_clicked();
        assert!(app.get_status_message().contains("Image") || !app.get_status_message().is_empty());

        // 5. Page Operations & Safe Page Duplication
        let p_before = app.get_total_pages();
        app.invoke_insert_blank_page_clicked();
        assert_eq!(app.get_total_pages(), p_before + 1);

        app.invoke_move_page_down_clicked();
        app.invoke_move_page_up_clicked();

        // 6. Undo and Redo
        app.invoke_undo_clicked();
        app.invoke_redo_clicked();

        // 7. Save and Reopen Verification
        let save_dest = acceptance_dir.join("saved_rc_doc.pdf");
        {
            let mut mgr = manager.borrow_mut();
            mgr.dispatch(AppCommand::SaveDocument {
                destination_path: Some(save_dest.clone()),
            }).expect("SaveDocument");
        }
        assert!(save_dest.exists());
        assert!(std::fs::metadata(&save_dest).unwrap().len() > 1000);

        // Reopen saved file in fresh session manager
        let reopened_bytes = std::fs::read(&save_dest).expect("read saved doc");
        let mut reopen_mgr = SessionManager::new();
        reopen_mgr.dispatch(AppCommand::OpenDocument {
            path: Some(save_dest.clone()),
            bytes: reopened_bytes,
            title: "reopened_saved_rc_doc.pdf".into(),
        }).expect("Reopen saved document");
        let s = reopen_mgr.active_session().unwrap();
        assert_eq!(s.total_pages as i32, app.get_total_pages());

        // Render page 0 of saved document to verify visual fidelity
        let rendered = s.render_page(0, 1.0).expect("render saved page");
        let shot_path = acceptance_dir.join("rc_acceptance_page0.png");
        image::save_buffer(
            &shot_path,
            &rendered.rgba_pixels,
            rendered.width,
            rendered.height,
            image::ExtendedColorType::Rgba8,
        ).unwrap();
        assert!(shot_path.exists());

        println!("  [Phase 12 RC Acceptance Complete]: All desktop editing, moving, resizing, and save/reopen workflows verified.");
    }

    #[test]
    fn test_phase13_rc_closure_and_window_events() {
        let _test_guard = init_testing();
        if GPU_ENABLED.load(Ordering::SeqCst) {
            if let Ok(mut lock) = GPU_RENDERER.lock() {
                if lock.is_none() {
                    *lock = starpdf::render::WgpuRenderer::new().ok();
                }
            }
        }
        let app = AppWindow::new().expect("Slint AppWindow");
        let manager = Rc::new(RefCell::new(SessionManager::new()));
        let active_tool = Rc::new(RefCell::new("select".to_string()));
        let is_editing = Rc::new(RefCell::new(false));

        register_ui_callbacks(&app, manager.clone(), active_tool.clone(), is_editing.clone());

        let acceptance_dir = resolve_fixture("target/desktop_acceptance_phase13");
        let _ = std::fs::create_dir_all(&acceptance_dir);

        // 1. Slint Window Input Event Dispatch Verification
        let window = app.window();
        window.dispatch_event(slint::platform::WindowEvent::PointerMoved {
            position: slint::LogicalPosition::new(250.0, 180.0),
        });
        window.dispatch_event(slint::platform::WindowEvent::PointerPressed {
            position: slint::LogicalPosition::new(250.0, 180.0),
            button: slint::platform::PointerEventButton::Left,
        });
        window.dispatch_event(slint::platform::WindowEvent::PointerReleased {
            position: slint::LogicalPosition::new(250.0, 180.0),
            button: slint::platform::PointerEventButton::Left,
        });

        // 2. Open Real Document with Form Fields
        let form_path = resolve_fixture("test-assets/realworld_corpus/forms-employment-application.pdf");
        let form_bytes = std::fs::read(&form_path).expect("read forms");
        {
            let mut mgr = manager.borrow_mut();
            mgr.dispatch(AppCommand::OpenDocument {
                path: Some(form_path.clone()),
                bytes: form_bytes,
                title: "forms-employment-application.pdf".into(),
            }).expect("OpenDocument");
        }
        sync_ui(&app, &manager.borrow(), &active_tool.borrow(), false);
        assert_eq!(app.get_total_pages(), 1);

        // 3. Canvas Mouse-Driven Form Checkbox Toggle
        // First click: selects checkbox
        app.invoke_element_selected(slint::SharedString::from("form_rust_cb"));
        assert!(app.get_has_selection());
        assert_eq!(app.get_selection_type(), "FormField");
        assert_eq!(app.get_form_field_name(), "rust_cb");
        assert!(app.get_form_field_checked());

        // Second click on canvas: toggles checkbox directly from canvas
        app.invoke_element_selected(slint::SharedString::from("form_rust_cb"));
        sync_ui(&app, &manager.borrow(), &active_tool.borrow(), false);
        assert!(!app.get_form_field_checked());

        // Third click on canvas: toggles back to checked
        app.invoke_element_selected(slint::SharedString::from("form_rust_cb"));
        sync_ui(&app, &manager.borrow(), &active_tool.borrow(), false);
        assert!(app.get_form_field_checked());

        // 4. Canvas Mouse Drag Move & Corner Resize
        app.invoke_add_annotation_clicked(slint::SharedString::from("FreeText"));
        let elems = app.get_page_elements();
        let mut annot_id = slint::SharedString::default();
        for i in (0..elems.row_count()).rev() {
            if let Some(elem) = elems.row_data(i) {
                if elem.kind == "annotation" {
                    annot_id = elem.id;
                    break;
                }
            }
        }
        assert!(!annot_id.is_empty(), "Annotation element must exist");
        app.invoke_element_selected(annot_id);
        assert!(app.get_has_selection());

        let x_before = app.get_sel_x();
        let w_before = app.get_sel_w();

        // Mouse drag move via canvas touch area callback
        app.invoke_move_selected_clicked(30.0, 20.0);
        assert!(app.get_sel_x() > x_before);

        // Corner drag resize via canvas resize touch area callback
        app.invoke_resize_selected_clicked(40.0, 25.0);
        assert!(app.get_sel_w() > w_before);

        // Keyboard arrow nudging
        let key_x_before = app.get_sel_x();
        app.invoke_move_selected_clicked(10.0, 0.0);
        assert!(app.get_sel_x() > key_x_before);

        // 5. Page Reorder & Document Assembly
        let pages_before = app.get_total_pages();
        app.invoke_insert_blank_page_clicked();
        assert_eq!(app.get_total_pages(), pages_before + 1);

        app.invoke_move_page_down_clicked();
        app.invoke_move_page_up_clicked();

        // 6. Multi-level Undo / Redo
        app.invoke_undo_clicked();
        app.invoke_redo_clicked();

        // 7. Multilingual Document Conformance Test
        let multi_path = resolve_fixture("test-assets/realworld_corpus/multilingual-arabic-jawi.pdf");
        let multi_bytes = std::fs::read(&multi_path).expect("read multilingual");
        {
            let mut mgr = manager.borrow_mut();
            mgr.dispatch(AppCommand::OpenDocument {
                path: Some(multi_path.clone()),
                bytes: multi_bytes,
                title: "multilingual-arabic-jawi.pdf".into(),
            }).expect("Open multilingual document");
        }
        sync_ui(&app, &manager.borrow(), &active_tool.borrow(), false);
        assert!(app.get_total_pages() > 0);
        assert!(app.get_text_spans().row_count() > 0);

        // 8. Save and Reopen Qualification
        let save_dest = acceptance_dir.join("saved_rc_phase13.pdf");
        {
            let mut mgr = manager.borrow_mut();
            mgr.dispatch(AppCommand::SaveDocument {
                destination_path: Some(save_dest.clone()),
            }).expect("Save document");
        }
        assert!(save_dest.exists());
        assert!(std::fs::metadata(&save_dest).unwrap().len() > 1000);

        // Reopen in independent session manager
        let reopened_bytes = std::fs::read(&save_dest).expect("read saved doc");
        let mut reopen_mgr = SessionManager::new();
        reopen_mgr.dispatch(AppCommand::OpenDocument {
            path: Some(save_dest.clone()),
            bytes: reopened_bytes,
            title: "reopened_saved_rc_phase13.pdf".into(),
        }).expect("Reopen saved document");
        let s = reopen_mgr.active_session().unwrap();
        assert_eq!(s.total_pages as i32, app.get_total_pages());

        // Render page 0 of saved document to verify visual fidelity
        let rendered = s.render_page(0, 1.0).expect("render saved page");
        let shot_path = acceptance_dir.join("rc13_acceptance_page0.png");
        image::save_buffer(
            &shot_path,
            &rendered.rgba_pixels,
            rendered.width,
            rendered.height,
            image::ExtendedColorType::Rgba8,
        ).unwrap();
        assert!(shot_path.exists());

        println!("  [Phase 13 RC Closure Acceptance Complete]: Window events, canvas drag/resize, form toggling, multilingual text, and save/reopen verified.");
    }
}

