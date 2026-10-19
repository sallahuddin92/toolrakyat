//! Main SmartPDF Desktop application struct and eframe::App implementation.
#![allow(dead_code)]

use std::collections::HashMap;
use std::time::Instant;

use egui::{
    ColorImage, Context, Frame, Key, Panel, Pos2, TextureHandle, TextureOptions, Ui,
};
use starpdf_app_core::command::AppCommand;
use starpdf_app_core::selection::SelectionState;
use starpdf_app_core::session::SessionManager;

use crate::canvas::{render_document_canvas, CanvasState};
use crate::chrome::{
    render_bottom_status_bar, render_floating_quick_pill, render_top_tier1, render_top_tier2,
};
use crate::panels::{
    render_all_tools_catalog, render_left_rail, render_right_inspector, render_thumbnails_panel,
    AcrobatMode, ActiveTool, LeftPanelMode,
};
use crate::theme::{DesignTokens, ThemeMode};

pub struct SmartPdfEguiApp {
    pub manager: SessionManager,
    pub active_tool: ActiveTool,
    pub active_mode: AcrobatMode,
    pub left_panel: LeftPanelMode,
    pub right_panel_open: bool,
    pub tokens: DesignTokens,
    pub theme_mode: ThemeMode,
    pub gpu_enabled: bool,
    pub gpu_renderer: Option<starpdf::render::WgpuRenderer>,
    pub zoom_percent: f32,
    pub canvas_state: CanvasState,
    pub page_texture: Option<TextureHandle>,
    pub thumbnail_textures: HashMap<u32, TextureHandle>,
    pub last_rendered_key: Option<(u64, u32, u32, bool)>, // (revision, page, zoom_bits, gpu)
    pub render_telemetry: String,
    pub status_message: String,
    pub editing_text_buffer: String,
    pub initial_sample_loaded: bool,
}

impl SmartPdfEguiApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // Initialize GPU renderer
        let gpu_renderer = starpdf::render::WgpuRenderer::new().ok();
        let telemetry = if let Some(ref r) = gpu_renderer {
            format!("⚡ {} GPU", r.adapter_name())
        } else {
            "⚙ Native CPU Raster".to_string()
        };

        Self {
            manager: SessionManager::new(),
            active_tool: ActiveTool::Select,
            active_mode: AcrobatMode::Edit,
            left_panel: LeftPanelMode::Closed,
            right_panel_open: true,
            tokens: DesignTokens::light(),
            theme_mode: ThemeMode::Light,
            gpu_enabled: true,
            gpu_renderer,
            zoom_percent: 100.0,
            canvas_state: CanvasState::default(),
            page_texture: None,
            thumbnail_textures: HashMap::new(),
            last_rendered_key: None,
            render_telemetry: telemetry,
            status_message: "Ready".to_string(),
            editing_text_buffer: String::new(),
            initial_sample_loaded: false,
        }
    }

    pub fn load_document_bytes(&mut self, path: Option<std::path::PathBuf>, bytes: Vec<u8>, title: String) {
        let res = self.manager.dispatch(AppCommand::OpenDocument { path, bytes, title });
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

    fn check_render_update(&mut self, ctx: &Context) {
        let (current_page, revision, total_pages) = if let Some(s) = self.manager.active_session() {
            (s.current_page, s.revision, s.total_pages)
        } else {
            return;
        };

        let zoom_bits = self.zoom_percent.to_bits();
        let key = (revision, current_page, zoom_bits, self.gpu_enabled);

        if self.last_rendered_key != Some(key) {
            let start = Instant::now();
            let scale = self.zoom_percent / 100.0;

            let session = self.manager.active_session().expect("active session");

            // 1. Render main active page
            let (pixels, width, height, is_gpu) = if self.gpu_enabled && self.gpu_renderer.is_some() {
                let r = self.gpu_renderer.as_ref().unwrap();
                match session.render_page_gpu(current_page, scale, r) {
                    Ok(gpu_res) => (gpu_res.rgba_pixels, gpu_res.width, gpu_res.height, true),
                    Err(_) => match session.render_page(current_page, scale) {
                        Ok(cpu_res) => (cpu_res.rgba_pixels, cpu_res.width, cpu_res.height, false),
                        Err(_) => return,
                    },
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

            let color_image = ColorImage::from_rgba_unmultiplied(
                [width as usize, height as usize],
                &pixels,
            );

            self.page_texture = Some(ctx.load_texture(
                format!("page_{}_{}", current_page, revision),
                color_image,
                TextureOptions::LINEAR,
            ));

            self.last_rendered_key = Some(key);

            // Sync editing text buffer with current selection
            match &session.selection {
                SelectionState::Text { text, .. } => {
                    self.editing_text_buffer = text.clone();
                }
                SelectionState::FormField { field_name, .. } => {
                    if let Ok(mut doc) = session.pdf_document() {
                        if let Ok(fields) = doc.form_fields() {
                            if let Some(f) = fields.iter().find(|x| &x.partial_name == field_name || &x.fully_qualified_name == field_name) {
                                let val_str = match &f.value {
                                    starpdf::forms::FieldValue::Text(s) => s.clone(),
                                    starpdf::forms::FieldValue::Boolean(b) => if *b { "true".into() } else { "false".into() },
                                    starpdf::forms::FieldValue::Name(s) => s.clone(),
                                    starpdf::forms::FieldValue::Choice(v) => v.first().cloned().unwrap_or_default(),
                                    starpdf::forms::FieldValue::None => "".into(),
                                };
                                self.editing_text_buffer = val_str;
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // Render thumbnails in background if missing
        if self.left_panel == LeftPanelMode::Thumbnails {
            let session = self.manager.active_session().expect("active session");
            for p in 0..total_pages {
                if !self.thumbnail_textures.contains_key(&p) {
                    if let Ok(thumb_res) = session.render_page(p, 0.25) {
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
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &Context) {
        ctx.input(|i| {
            let cmd = i.modifiers.command || i.modifiers.ctrl;
            let shift = i.modifiers.shift;

            if cmd && i.key_pressed(Key::O) {
                if let Some(path) = rfd::FileDialog::new().add_filter("PDF", &["pdf"]).pick_file() {
                    if let Ok(bytes) = std::fs::read(&path) {
                        let title = path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "document.pdf".to_string());
                        let _ = self.manager.dispatch(AppCommand::OpenDocument { path: Some(path), bytes, title });
                        self.invalidate_render();
                    }
                }
            } else if cmd && i.key_pressed(Key::S) {
                let existing_path = self.manager.active_session().and_then(|s| s.file_path.clone());
                let dest = existing_path.or_else(|| {
                    rfd::FileDialog::new().add_filter("PDF", &["pdf"]).set_file_name("document.pdf").save_file()
                });
                if let Some(p) = dest {
                    let _ = self.manager.dispatch(AppCommand::SaveDocument { destination_path: Some(p) });
                }
            } else if cmd && shift && i.key_pressed(Key::Z) {
                let _ = self.manager.dispatch(AppCommand::Redo);
            } else if cmd && i.key_pressed(Key::Z) {
                let _ = self.manager.dispatch(AppCommand::Undo);
            } else if cmd && i.key_pressed(Key::Num0) {
                self.zoom_percent = 100.0;
                let _ = self.manager.dispatch(AppCommand::SetZoom { percent: 100.0 });
            } else if cmd && i.key_pressed(Key::Num1) {
                self.zoom_percent = 135.0;
                let _ = self.manager.dispatch(AppCommand::SetZoom { percent: 135.0 });
            } else if cmd && (i.key_pressed(Key::Plus) || i.key_pressed(Key::Equals)) {
                self.zoom_percent = (self.zoom_percent * 1.25).min(500.0);
                let _ = self.manager.dispatch(AppCommand::SetZoom { percent: self.zoom_percent });
            } else if cmd && i.key_pressed(Key::Minus) {
                self.zoom_percent = (self.zoom_percent / 1.25).max(25.0);
                let _ = self.manager.dispatch(AppCommand::SetZoom { percent: self.zoom_percent });
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
                    self.active_mode = AcrobatMode::Comment;
                } else if i.key_pressed(Key::N) {
                    self.active_tool = ActiveTool::StickyNote;
                    self.active_mode = AcrobatMode::Comment;
                }
            }
        });
    }
}

impl eframe::App for SmartPdfEguiApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // Apply theme tokens
        self.tokens = match self.theme_mode {
            ThemeMode::Light => DesignTokens::light(),
            ThemeMode::Dark => DesignTokens::dark(),
        };
        self.tokens.apply_to_context(&ctx);

        // Handle shortcuts
        self.handle_keyboard_shortcuts(&ctx);

        // Render check
        self.check_render_update(&ctx);

        // 1. Top Bar (Two-Tier Chrome)
        Panel::top("top_chrome")
            .frame(Frame::NONE.fill(self.tokens.chrome_bg))
            .show(ui, |ui| {
                ui.add_space(6.0);
                render_top_tier1(
                    ui,
                    &self.tokens,
                    &mut self.manager,
                    &mut self.theme_mode,
                    &mut self.gpu_enabled,
                );
                ui.add_space(4.0);
                ui.separator();
                ui.add_space(2.0);
                render_top_tier2(
                    ui,
                    &self.tokens,
                    &mut self.active_mode,
                    &mut self.left_panel,
                );
                ui.add_space(4.0);
            });

        // 2. Bottom Status Bar
        Panel::bottom("bottom_status")
            .frame(Frame::NONE.fill(self.tokens.panel_bg))
            .show(ui, |ui| {
                ui.add_space(4.0);
                render_bottom_status_bar(
                    ui,
                    &self.tokens,
                    &mut self.manager,
                    &mut self.zoom_percent,
                    &self.render_telemetry,
                    &self.status_message,
                );
                ui.add_space(4.0);
            });

        // 3. Left Rail (Icon strip)
        Panel::left("left_rail")
            .resizable(false)
            .exact_size(40.0)
            .frame(Frame::NONE.fill(self.tokens.panel_bg))
            .show(ui, |ui| {
                render_left_rail(ui, &self.tokens, &mut self.left_panel);
            });

        // 4. Left Drawer / Panel (if open)
        if self.left_panel != LeftPanelMode::Closed {
            Panel::left("left_drawer")
                .resizable(true)
                .default_size(220.0)
                .size_range(180.0..=360.0)
                .frame(Frame::NONE.fill(self.tokens.chrome_bg))
                .show(ui, |ui| {
                    ui.add_space(8.0);
                    match self.left_panel {
                        LeftPanelMode::Thumbnails => {
                            render_thumbnails_panel(
                                ui,
                                &self.tokens,
                                &mut self.manager,
                                &self.thumbnail_textures,
                            );
                        }
                        LeftPanelMode::AllTools => {
                            render_all_tools_catalog(
                                ui,
                                &self.tokens,
                                &mut self.active_mode,
                                &mut self.active_tool,
                                &mut self.left_panel,
                            );
                        }
                        LeftPanelMode::Bookmarks => {
                            ui.label(egui::RichText::new("Bookmarks").strong().color(self.tokens.text_primary));
                            ui.separator();
                            ui.label(egui::RichText::new("No bookmarks found in document.").color(self.tokens.text_secondary));
                        }
                        LeftPanelMode::Closed => {}
                    }
                });
        }

        // 5. Right Inspector Panel (if open)
        if self.right_panel_open {
            Panel::right("right_inspector")
                .resizable(true)
                .default_size(240.0)
                .size_range(200.0..=380.0)
                .frame(Frame::NONE.fill(self.tokens.chrome_bg))
                .show(ui, |ui| {
                    ui.add_space(8.0);
                    render_right_inspector(
                        ui,
                        &self.tokens,
                        &mut self.manager,
                        &mut self.editing_text_buffer,
                    );
                });
        }

        // 6. Central Document Canvas & Floating Quick Tools Pill
        egui::CentralPanel::default()
            .frame(Frame::NONE.fill(self.tokens.canvas_bg))
            .show(ui, |ui| {
                let page_dims = if let Some(session) = self.manager.active_session() {
                    session
                        .compile_render_page(session.current_page)
                        .map(|ir| (ir.width as f32, ir.height as f32))
                        .unwrap_or((612.0, 792.0))
                } else {
                    (612.0, 792.0)
                };

                let canvas_resp = render_document_canvas(
                    ui,
                    &self.tokens,
                    &mut self.manager,
                    &mut self.canvas_state,
                    &mut self.active_tool,
                    &mut self.active_mode,
                    self.page_texture.as_ref(),
                    page_dims,
                    self.zoom_percent,
                    &mut self.editing_text_buffer,
                );

                // Floating quick tools pill at top center of canvas
                let pill_x = (canvas_resp.rect.left() + canvas_resp.rect.right()) / 2.0 - 130.0;
                let pill_y = canvas_resp.rect.top() + 16.0;
                render_floating_quick_pill(
                    ui,
                    &self.tokens,
                    &mut self.active_tool,
                    Pos2::new(pill_x, pill_y),
                );
            });
    }
}
