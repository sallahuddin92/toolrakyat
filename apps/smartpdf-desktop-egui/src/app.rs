//! Main SmartPDF Desktop application struct and eframe::App implementation.
//! Fully integrates upstream PdfCraft design system, window chrome, and panels
//! with StarPDF authoritative document engine via EguiEngineAdapter.

use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::time::Instant;

use egui::{
    Align, Color32, ColorImage, Context, Frame, Key, Layout, Panel, Pos2, Rect, TextureHandle,
    TextureOptions, Ui, Vec2,
};
use starpdf_app_core::command::AppCommand;
use starpdf_app_core::export::ExportProfile;
use starpdf_app_core::selection::SelectionState;

use crate::adapter::EguiEngineAdapter;
use crate::canvas::{render_document_canvas, CanvasAction, CanvasState, PT};
use crate::chrome::{
    render_bottom_status_bar, render_floating_quick_pill, render_mode_bar, render_tab_strip,
    ModeBarAction, TabStripAction, MAC_TITLE_BAR,
};
use crate::icons;
use crate::panels::{
    render_left_panel, render_right_rail, render_selection_inspector, AcrobatMode, ActiveTool,
    LeftPanelMode, RightPanelMode,
};
use crate::theme::{self, ThemeKind, Tokens};

pub struct SmartPdfEguiApp {
    pub adapter: EguiEngineAdapter,
    pub active_tool: ActiveTool,
    pub active_mode: AcrobatMode,
    pub left_panel: LeftPanelMode,
    pub right_panel: RightPanelMode,
    pub theme_kind: ThemeKind,
    pub gpu_enabled: bool,
    pub gpu_renderer: Option<starpdf::render::WgpuRenderer>,
    pub zoom_percent: f32,
    pub canvas_state: CanvasState,
    pub page_texture: Option<TextureHandle>,
    pub thumbnail_textures: HashMap<u32, TextureHandle>,
    pub last_rendered_key: Option<(String, u64, u32, u32, bool)>, // (session_id, revision, page, zoom_bits, gpu)
    pub render_telemetry: String,
    pub is_loading: bool,
    pub open_dialog_rx: Option<Receiver<Option<PathBuf>>>,
    pub save_dialog_rx: Option<Receiver<Option<PathBuf>>>,
    pub export_dialog_rx: Option<Receiver<Option<PathBuf>>>,
    pub screenshot_target: Option<PathBuf>,
    pub frame_count: u64,
}

impl Deref for SmartPdfEguiApp {
    type Target = EguiEngineAdapter;
    fn deref(&self) -> &Self::Target {
        &self.adapter
    }
}

impl DerefMut for SmartPdfEguiApp {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.adapter
    }
}

impl SmartPdfEguiApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Install Inter and JetBrainsMono fonts
        cc.egui_ctx.set_fonts(theme::font_definitions());
        Self::create_internal()
    }

    #[allow(dead_code)]
    pub fn new_headless() -> Self {
        Self::create_internal()
    }

    fn create_internal() -> Self {
        // Initialize GPU renderer
        let gpu_renderer = starpdf::render::WgpuRenderer::new().ok();
        let telemetry = if let Some(ref r) = gpu_renderer {
            format!("⚡ {} GPU", r.adapter_name())
        } else {
            "⚙ Native CPU Raster".to_string()
        };

        Self {
            adapter: EguiEngineAdapter::new(),
            active_tool: ActiveTool::Select,
            active_mode: AcrobatMode::AllTools,
            left_panel: LeftPanelMode::AllTools,
            right_panel: RightPanelMode::Inspector,
            theme_kind: ThemeKind::Light,
            gpu_enabled: true,
            gpu_renderer,
            zoom_percent: 100.0,
            canvas_state: CanvasState::default(),
            page_texture: None,
            thumbnail_textures: HashMap::new(),
            last_rendered_key: None,
            render_telemetry: telemetry,
            is_loading: false,
            open_dialog_rx: None,
            save_dialog_rx: None,
            export_dialog_rx: None,
            screenshot_target: None,
            frame_count: 0,
        }
    }

    pub fn open_file_path(&mut self, path: impl AsRef<Path>) -> Result<(), String> {
        let res = self.adapter.open_file_path(path);
        if res.is_ok() {
            self.invalidate_render();
        }
        res
    }

    pub fn trigger_open_dialog(&mut self) {
        if self.open_dialog_rx.is_some() {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        self.open_dialog_rx = Some(rx);
        self.status_message = "Waiting for file selection...".to_string();
        std::thread::spawn(move || {
            let res = rfd::FileDialog::new()
                .add_filter("PDF Document (*.pdf)", &["pdf"])
                .pick_file();
            let _ = tx.send(res);
        });
    }

    pub fn trigger_save_dialog(&mut self) {
        if self.save_dialog_rx.is_some() {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        self.save_dialog_rx = Some(rx);
        self.status_message = "Waiting for save location...".to_string();
        std::thread::spawn(move || {
            let res = rfd::FileDialog::new()
                .add_filter("PDF Document (*.pdf)", &["pdf"])
                .set_file_name("document.pdf")
                .save_file();
            let _ = tx.send(res);
        });
    }

    pub fn trigger_export_dialog(&mut self) {
        if self.export_dialog_rx.is_some() {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        self.export_dialog_rx = Some(rx);
        self.status_message = "Waiting for export location...".to_string();
        std::thread::spawn(move || {
            let res = rfd::FileDialog::new()
                .add_filter("PDF Document (*.pdf)", &["pdf"])
                .set_file_name("exported_document.pdf")
                .save_file();
            let _ = tx.send(res);
        });
    }

    pub fn save_active_document(&mut self, dest: Option<PathBuf>) -> Result<PathBuf, String> {
        let has_path = dest.is_some()
            || self
                .manager
                .active_session()
                .and_then(|s| s.file_path.clone())
                .is_some();

        if !has_path {
            self.trigger_save_dialog();
            return Err("No destination path specified; triggered Save dialog".to_string());
        }

        self.adapter.save_active_document(dest)
    }

    pub fn load_document_bytes(&mut self, path: Option<PathBuf>, bytes: Vec<u8>, title: String) {
        let res = self
            .adapter
            .dispatch(AppCommand::OpenDocument { path, bytes, title });
        if let Ok(res) = res {
            self.status_message = res.message;
        }
        self.invalidate_render();
    }

    pub fn invalidate_render(&mut self) {
        self.last_rendered_key = None;
        self.page_texture = None;
        self.thumbnail_textures.clear();
        if let Some(s) = self.manager.active_session() {
            s.invalidate_render_cache();
        }
    }

    pub fn check_render_update(&mut self, ctx: &Context) {
        let (session_id, current_page, revision, total_pages) =
            if let Some(s) = self.manager.active_session() {
                (s.id.clone(), s.current_page, s.revision, s.total_pages)
            } else {
                self.page_texture = None;
                self.last_rendered_key = None;
                return;
            };

        let zoom_bits = self.zoom_percent.to_bits();
        let key = (
            session_id,
            revision,
            current_page,
            zoom_bits,
            self.gpu_enabled,
        );

        if self.last_rendered_key.as_ref() != Some(&key) {
            let start = Instant::now();
            let ppp = ctx.pixels_per_point();
            // High-DPI supersampling scale
            let scale = (self.zoom_percent / 100.0) * PT * ppp;

            let session = self.manager.active_session().expect("active session");

            // 1. Render main active page
            let (pixels, width, height, is_gpu) = if self.gpu_enabled {
                if let Some(ref r) = self.gpu_renderer {
                    match session.render_page_gpu(current_page, scale, r) {
                        Ok(gpu_res) => (gpu_res.rgba_pixels, gpu_res.width, gpu_res.height, true),
                        Err(_) => match session.render_page(current_page, scale) {
                            Ok(cpu_res) => {
                                (cpu_res.rgba_pixels, cpu_res.width, cpu_res.height, false)
                            }
                            Err(_) => return,
                        },
                    }
                } else {
                    match session.render_page(current_page, scale) {
                        Ok(cpu_res) => (cpu_res.rgba_pixels, cpu_res.width, cpu_res.height, false),
                        Err(_) => return,
                    }
                }
            } else {
                match session.render_page(current_page, scale) {
                    Ok(cpu_res) => (cpu_res.rgba_pixels, cpu_res.width, cpu_res.height, false),
                    Err(_) => return,
                }
            };

            let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
            self.render_telemetry = if is_gpu {
                format!("⚡ Metal GPU ({:.1} ms)", elapsed_ms)
            } else {
                format!("⚙ Native CPU ({:.1} ms)", elapsed_ms)
            };

            let color_image =
                ColorImage::from_rgba_unmultiplied([width as usize, height as usize], &pixels);

            self.page_texture = Some(ctx.load_texture(
                format!("page_{}_{}_{}", key.0, current_page, revision),
                color_image,
                TextureOptions::LINEAR,
            ));

            self.last_rendered_key = Some(key);
            self.adapter.sync_selection_buffers();
        }

        // Render thumbnails in background if missing
        if self.left_panel == LeftPanelMode::Thumbnails {
            let needed_pages: Vec<u32> = (0..total_pages)
                .filter(|p| !self.thumbnail_textures.contains_key(p))
                .collect();

            for p in needed_pages {
                let thumb_opt = {
                    let session = self.manager.active_session().expect("active session");
                    session.render_page(p, 0.35).ok()
                };
                if let Some(thumb_res) = thumb_opt {
                    let color_image = ColorImage::from_rgba_unmultiplied(
                        [thumb_res.width as usize, thumb_res.height as usize],
                        &thumb_res.rgba_pixels,
                    );
                    let tex = ctx.load_texture(
                        format!("thumb_{}_{}", p, revision),
                        color_image,
                        TextureOptions::LINEAR,
                    );
                    self.thumbnail_textures.insert(p, tex);
                }
            }
        }
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &Context) {
        let mut trigger_open = false;
        let mut trigger_save = false;
        let mut trigger_save_as = false;
        let mut trigger_export = false;
        let mut trigger_close = false;

        ctx.input(|i| {
            let cmd = i.modifiers.command || i.modifiers.ctrl;
            let shift = i.modifiers.shift;

            if cmd && i.key_pressed(Key::O) {
                trigger_open = true;
            } else if cmd && shift && i.key_pressed(Key::S) {
                trigger_save_as = true;
            } else if cmd && i.key_pressed(Key::S) {
                trigger_save = true;
            } else if cmd && i.key_pressed(Key::E) {
                trigger_export = true;
            } else if cmd && i.key_pressed(Key::W) {
                trigger_close = true;
            } else if cmd && shift && i.key_pressed(Key::Z) {
                let _ = self.adapter.redo();
            } else if cmd && i.key_pressed(Key::Z) {
                let _ = self.adapter.undo();
            } else if cmd && i.key_pressed(Key::Num0) {
                self.zoom_percent = 100.0;
                let _ = self.adapter.set_zoom(100.0);
            } else if cmd && (i.key_pressed(Key::Plus) || i.key_pressed(Key::Equals)) {
                if let Ok(z) = self.adapter.zoom_in() {
                    self.zoom_percent = z;
                }
            } else if cmd && i.key_pressed(Key::Minus) {
                if let Ok(z) = self.adapter.zoom_out() {
                    self.zoom_percent = z;
                }
            } else if !cmd {
                if i.key_pressed(Key::V) {
                    self.active_tool = ActiveTool::Select;
                } else if i.key_pressed(Key::H) {
                    self.active_tool = ActiveTool::Hand;
                } else if i.key_pressed(Key::T) {
                    self.active_tool = ActiveTool::EditText;
                    self.active_mode = AcrobatMode::Edit;
                } else if i.key_pressed(Key::U) {
                    self.active_tool = ActiveTool::Highlight;
                    self.active_mode = AcrobatMode::Edit;
                } else if i.key_pressed(Key::N) {
                    self.active_tool = ActiveTool::StickyNote;
                    self.active_mode = AcrobatMode::Edit;
                } else if (i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace))
                    && self
                        .adapter
                        .manager
                        .active_session()
                        .map(|s| s.selection != SelectionState::None)
                        .unwrap_or(false)
                {
                    // Only delete if not currently typing in a text field
                    if !ctx.egui_wants_keyboard_input() {
                        let _ = self.adapter.delete_selected();
                    }
                }
            }
        });

        if trigger_open {
            self.trigger_open_dialog();
        }
        if trigger_save_as {
            self.trigger_save_dialog();
        }
        if trigger_save {
            let has_path = self
                .manager
                .active_session()
                .and_then(|s| s.file_path.clone())
                .is_some();
            if has_path {
                let _ = self.save_active_document(None);
            } else {
                self.trigger_save_dialog();
            }
        }
        if trigger_export {
            self.trigger_export_dialog();
        }
        if trigger_close {
            if let Some(s) = self.manager.active_session() {
                let id = s.id.clone();
                let _ = self.adapter.close_session(&id);
                self.invalidate_render();
            }
        }
    }

    pub fn render_frame(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();

        // Apply theme tokens
        theme::apply(&ctx, self.theme_kind);
        let t = Tokens::get(&ctx);

        // Poll async file open receiver
        if let Some(ref rx) = self.open_dialog_rx {
            if let Ok(path_opt) = rx.try_recv() {
                self.open_dialog_rx = None;
                if let Some(path) = path_opt {
                    let _ = self.open_file_path(path);
                } else {
                    self.status_message = "Open cancelled".to_string();
                }
            }
        }

        // Poll async file save receiver
        if let Some(ref rx) = self.save_dialog_rx {
            if let Ok(path_opt) = rx.try_recv() {
                self.save_dialog_rx = None;
                if let Some(path) = path_opt {
                    let _ = self.save_active_document(Some(path));
                } else {
                    self.status_message = "Save cancelled".to_string();
                }
            }
        }

        // Poll async file export receiver
        if let Some(ref rx) = self.export_dialog_rx {
            if let Ok(path_opt) = rx.try_recv() {
                self.export_dialog_rx = None;
                if let Some(path) = path_opt {
                    let _ = self
                        .adapter
                        .export_active_document(ExportProfile::IncrementalPdf, path);
                } else {
                    self.status_message = "Export cancelled".to_string();
                }
            }
        }

        // Handle native drag and drop
        let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
        if !dropped_files.is_empty() {
            for dropped in dropped_files {
                let p = dropped.path();
                if !p.as_os_str().is_empty() {
                    let _ = self.open_file_path(p);
                }
            }
        }

        // Handle shortcuts
        self.handle_keyboard_shortcuts(&ctx);

        // Synchronize adapter cache invalidation flags
        if self.adapter.needs_render_invalidation {
            self.adapter.needs_render_invalidation = false;
            self.invalidate_render();
        }
        if self.adapter.needs_thumbnail_invalidation {
            self.adapter.needs_thumbnail_invalidation = false;
            self.thumbnail_textures.clear();
        }

        // Render check
        self.check_render_update(&ctx);

        self.frame_count += 1;

        // Screenshot capture logic
        if self.screenshot_target.is_some() {
            for event in ctx.input(|i| i.events.clone()) {
                if let egui::Event::Screenshot { image, .. } = event {
                    if let Some(target) = self.screenshot_target.take() {
                        let [w, h] = image.size;
                        let mut rgba = Vec::with_capacity(w * h * 4);
                        for px in &image.pixels {
                            rgba.extend_from_slice(&px.to_srgba_unmultiplied());
                        }
                        if let Some(parent) = target.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        if let Some(img_buf) = image::RgbaImage::from_raw(w as u32, h as u32, rgba)
                        {
                            if let Err(e) = img_buf.save(&target) {
                                eprintln!("[SCREENSHOT_ERROR] Failed to save screenshot: {e}");
                            } else {
                                println!("[SCREENSHOT_SAVED] {}", target.display());
                            }
                        }
                        std::process::exit(0);
                    }
                }
            }

            if self.frame_count >= 5 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(1)));
            }
            ctx.request_repaint();
        }

        // 1. Tab Strip (integrated title bar)
        let mac = cfg!(target_os = "macos");
        let mut tab_action = TabStripAction::None;
        Panel::top("tab_strip")
            .exact_size(if mac { MAC_TITLE_BAR } else { 38.0 })
            .frame(Frame::NONE.fill(t.titlebar).inner_margin(egui::Margin {
                left: 0,
                right: 10,
                top: 0,
                bottom: 0,
            }))
            .show(ui, |ui| {
                tab_action = render_tab_strip(ui, &mut self.adapter.manager, &mut self.theme_kind);
            });

        match tab_action {
            TabStripAction::OpenRequested => {
                self.trigger_open_dialog();
            }
            TabStripAction::SwitchTab(id) => {
                let _ = self.adapter.switch_tab(&id);
                self.invalidate_render();
            }
            TabStripAction::CloseTab(id) => {
                let _ = self.adapter.close_session(&id);
                self.invalidate_render();
            }
            TabStripAction::ToggleTheme => {
                self.theme_kind = if self.theme_kind == ThemeKind::Dark {
                    ThemeKind::Light
                } else {
                    ThemeKind::Dark
                };
            }
            TabStripAction::None => {}
        }

        // 2. Mode Bar
        let mut mode_action = ModeBarAction::None;
        let can_undo = self.adapter.can_undo();
        let can_redo = self.adapter.can_redo();

        Panel::top("mode_bar")
            .exact_size(48.0)
            .frame(
                Frame::NONE
                    .fill(t.chrome)
                    .inner_margin(egui::Margin::symmetric(10, 0))
                    .stroke(egui::Stroke::new(1.0, t.divider)),
            )
            .show(ui, |ui| {
                mode_action = render_mode_bar(ui, self.active_mode, can_undo, can_redo);
            });

        match mode_action {
            ModeBarAction::SaveRequested => {
                let _ = self.save_active_document(None);
            }
            ModeBarAction::ExportRequested => {
                self.trigger_export_dialog();
            }
            ModeBarAction::UndoRequested => {
                let _ = self.adapter.undo();
            }
            ModeBarAction::RedoRequested => {
                let _ = self.adapter.redo();
            }
            ModeBarAction::ToggleLeftPanel => {
                self.left_panel = if self.left_panel == LeftPanelMode::Closed {
                    LeftPanelMode::AllTools
                } else {
                    LeftPanelMode::Closed
                };
            }
            ModeBarAction::SelectMode(mode) => {
                self.active_mode = mode;
                match mode {
                    AcrobatMode::AllTools => {
                        self.left_panel = LeftPanelMode::AllTools;
                    }
                    AcrobatMode::Edit => {
                        self.left_panel = LeftPanelMode::Tool("edit");
                        self.active_tool = ActiveTool::EditText;
                        self.right_panel = RightPanelMode::Inspector;
                    }
                    AcrobatMode::Organize => {
                        self.left_panel = LeftPanelMode::Thumbnails;
                    }
                    AcrobatMode::Forms => {
                        self.active_tool = ActiveTool::FormField;
                        self.right_panel = RightPanelMode::Inspector;
                    }
                    AcrobatMode::Read => {
                        self.left_panel = LeftPanelMode::Closed;
                    }
                    AcrobatMode::Convert => {
                        let refusal = "Convert: External document conversion adapters (Office/Images) are not configured in this standalone environment.";
                        self.adapter.status_message = refusal.to_string();
                        self.adapter.error_banner = Some(refusal.to_string());
                    }
                    AcrobatMode::Sign => {
                        self.active_tool = ActiveTool::Sign;
                        let notice = "Fill & Sign mode: Use Edit to add signature images or click form fields to fill.";
                        self.adapter.status_message = notice.to_string();
                    }
                }
            }
            ModeBarAction::None => {}
        }

        // 2.5 Error Banner (if present)
        let mut dismiss_error = false;
        if let Some(ref err) = self.adapter.error_banner {
            Panel::top("error_banner")
                .exact_size(32.0)
                .frame(
                    Frame::NONE
                        .fill(Color32::from_rgb(195, 42, 42))
                        .inner_margin(egui::Margin::symmetric(14, 6)),
                )
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        icons::paint(
                            ui,
                            Rect::from_min_size(ui.cursor().min, Vec2::splat(16.0)),
                            "triangle-alert",
                            16.0,
                            Color32::WHITE,
                        );
                        ui.add_space(20.0);
                        ui.label(egui::RichText::new(err).color(Color32::WHITE).size(12.0));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui
                                .button(
                                    egui::RichText::new("Dismiss")
                                        .color(Color32::WHITE)
                                        .size(11.0),
                                )
                                .clicked()
                            {
                                dismiss_error = true;
                            }
                        });
                    });
                });
        }
        if dismiss_error {
            self.adapter.error_banner = None;
        }

        // 3. Bottom Status Bar
        Panel::bottom("bottom_status")
            .frame(
                Frame::NONE
                    .fill(t.chrome)
                    .inner_margin(egui::Margin::symmetric(14, 6))
                    .stroke(egui::Stroke::new(1.0, t.divider)),
            )
            .show(ui, |ui| {
                render_bottom_status_bar(
                    ui,
                    &mut self.adapter.manager,
                    &mut self.zoom_percent,
                    &self.render_telemetry,
                    &self.adapter.status_message,
                );
            });

        // 4. Right Rail (Inspector, Thumbnails, Bookmarks, Comments toggle strip)
        Panel::right("right_rail")
            .resizable(false)
            .exact_size(44.0)
            .frame(
                Frame::NONE
                    .fill(t.chrome)
                    .stroke(egui::Stroke::new(1.0, t.divider)),
            )
            .show(ui, |ui| {
                render_right_rail(ui, &mut self.left_panel, &mut self.right_panel);
            });

        // 4.5 Right Inspector Panel (if open)
        if self.right_panel == RightPanelMode::Inspector {
            Panel::right("right_inspector")
                .resizable(false)
                .exact_size(260.0)
                .frame(
                    Frame::NONE
                        .fill(t.panel)
                        .inner_margin(egui::Margin::same(12))
                        .stroke(egui::Stroke::new(1.0, t.divider)),
                )
                .show(ui, |ui| {
                    render_selection_inspector(ui, &mut self.adapter);
                });
        }

        // 5. Left Tool Panel (if open)
        if self.left_panel != LeftPanelMode::Closed {
            Panel::left("left_tool_panel")
                .resizable(false)
                .exact_size(272.0)
                .frame(Frame::NONE.fill(t.panel))
                .show(ui, |ui| {
                    render_left_panel(
                        ui,
                        &mut self.adapter,
                        &mut self.left_panel,
                        &mut self.active_mode,
                        &mut self.active_tool,
                        &self.thumbnail_textures,
                    );
                });
        }

        // 6. Central Document Canvas & Floating Quick Tools Pill
        egui::CentralPanel::default()
            .frame(Frame::NONE.fill(t.pasteboard))
            .show(ui, |ui| {
                let page_dims = if let Some(session) = self.manager.active_session() {
                    session
                        .compile_render_page(session.current_page)
                        .map(|ir| (ir.width, ir.height))
                        .unwrap_or((595.0, 842.0))
                } else {
                    (595.0, 842.0)
                };

                let (canvas_resp, canvas_action) = render_document_canvas(
                    ui,
                    &mut self.adapter,
                    &mut self.canvas_state,
                    &mut self.active_tool,
                    &mut self.active_mode,
                    self.page_texture.as_ref(),
                    page_dims,
                    self.zoom_percent,
                );

                if canvas_action == CanvasAction::OpenRequested {
                    self.trigger_open_dialog();
                }

                // If element selected, ensure inspector is visible
                if let Some(session) = self.manager.active_session() {
                    if session.selection != SelectionState::None {
                        self.right_panel = RightPanelMode::Inspector;
                    }
                }

                // Floating vertical quick tools pill docked to the left of the canvas (only if session active)
                if self.manager.active_session().is_some() {
                    let pill_x = canvas_resp.rect.left() + 14.0;
                    let pill_y = canvas_resp.rect.top() + 16.0;
                    render_floating_quick_pill(
                        ui,
                        &mut self.active_tool,
                        Pos2::new(pill_x, pill_y),
                    );
                }
            });
    }
}

impl eframe::App for SmartPdfEguiApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.render_frame(ui);
    }
}
