//! Main SmartPDF Desktop application struct and eframe::App implementation.
//! Fully integrates upstream PdfCraft design system, window chrome, and panels
//! with StarPDF authoritative document engine.

use std::collections::HashMap;
use std::time::Instant;

use egui::{
    ColorImage, Context, Frame, Key, Panel, Pos2, TextureHandle, TextureOptions, Ui,
};
use starpdf_app_core::command::AppCommand;
use starpdf_app_core::selection::SelectionState;
use starpdf_app_core::session::SessionManager;

use crate::canvas::{render_document_canvas, CanvasState, PT};
use crate::chrome::{
    render_bottom_status_bar, render_floating_quick_pill, render_mode_bar, render_tab_strip,
    MAC_TITLE_BAR,
};
use crate::panels::{
    render_left_panel, render_right_rail, AcrobatMode, ActiveTool, LeftPanelMode,
};
use crate::theme::{self, ThemeKind, Tokens};

pub struct SmartPdfEguiApp {
    pub manager: SessionManager,
    pub active_tool: ActiveTool,
    pub active_mode: AcrobatMode,
    pub left_panel: LeftPanelMode,
    pub theme_kind: ThemeKind,
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
    pub screenshot_target: Option<std::path::PathBuf>,
    pub frame_count: u64,
}

impl SmartPdfEguiApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Install Inter and JetBrainsMono fonts
        cc.egui_ctx.set_fonts(theme::font_definitions());

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
            active_mode: AcrobatMode::AllTools,
            left_panel: LeftPanelMode::AllTools,
            theme_kind: ThemeKind::Light,
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
            screenshot_target: None,
            frame_count: 0,
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
            let ppp = ctx.pixels_per_point();
            // High-DPI supersampling scale
            let scale = (self.zoom_percent / 100.0) * PT * ppp;

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
            if let SelectionState::Text { text, .. } = &session.selection {
                self.editing_text_buffer = text.clone();
            }
        }

        // Render thumbnails in background if missing
        if self.left_panel == LeftPanelMode::Thumbnails {
            let session = self.manager.active_session().expect("active session");
            for p in 0..total_pages {
                if !self.thumbnail_textures.contains_key(&p) {
                    if let Ok(thumb_res) = session.render_page(p, 0.35) {
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
                    self.active_mode = AcrobatMode::Edit;
                } else if i.key_pressed(Key::N) {
                    self.active_tool = ActiveTool::StickyNote;
                    self.active_mode = AcrobatMode::Edit;
                }
            }
        });
    }
}

impl eframe::App for SmartPdfEguiApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // Apply theme tokens
        theme::apply(&ctx, self.theme_kind);
        let t = Tokens::get(&ctx);

        // Handle shortcuts
        self.handle_keyboard_shortcuts(&ctx);

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
                        if let Some(img_buf) = image::RgbaImage::from_raw(w as u32, h as u32, rgba) {
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
        Panel::top("tab_strip")
            .exact_size(if mac { MAC_TITLE_BAR } else { 38.0 })
            .frame(Frame::NONE.fill(t.titlebar).inner_margin(egui::Margin { left: 0, right: 10, top: 0, bottom: 0 }))
            .show(ui, |ui| {
                render_tab_strip(ui, &mut self.manager, &mut self.theme_kind);
            });

        // 2. Mode Bar
        Panel::top("mode_bar")
            .exact_size(48.0)
            .frame(Frame::NONE.fill(t.chrome).inner_margin(egui::Margin::symmetric(10, 0)).stroke(egui::Stroke::new(1.0, t.divider)))
            .show(ui, |ui| {
                render_mode_bar(ui, &mut self.manager, &mut self.active_mode, &mut self.left_panel);
            });

        // 3. Bottom Status Bar
        Panel::bottom("bottom_status")
            .frame(Frame::NONE.fill(t.chrome).inner_margin(egui::Margin::symmetric(14, 6)).stroke(egui::Stroke::new(1.0, t.divider)))
            .show(ui, |ui| {
                render_bottom_status_bar(
                    ui,
                    &mut self.manager,
                    &mut self.zoom_percent,
                    &self.render_telemetry,
                    &self.status_message,
                );
            });

        // 4. Right Rail (Thumbnails, Bookmarks, Comments toggle strip)
        Panel::right("right_rail")
            .resizable(false)
            .exact_size(44.0)
            .frame(Frame::NONE.fill(t.chrome).stroke(egui::Stroke::new(1.0, t.divider)))
            .show(ui, |ui| {
                render_right_rail(ui, &mut self.left_panel);
            });

        // 5. Left Tool Panel (if open)
        if self.left_panel != LeftPanelMode::Closed {
            Panel::left("left_tool_panel")
                .resizable(false)
                .exact_size(272.0)
                .frame(Frame::NONE.fill(t.panel))
                .show(ui, |ui| {
                    render_left_panel(
                        ui,
                        &mut self.manager,
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
                        .map(|ir| (ir.width as f32, ir.height as f32))
                        .unwrap_or((595.0, 842.0))
                } else {
                    (595.0, 842.0)
                };

                let canvas_resp = render_document_canvas(
                    ui,
                    &mut self.manager,
                    &mut self.canvas_state,
                    &mut self.active_tool,
                    &mut self.active_mode,
                    self.page_texture.as_ref(),
                    page_dims,
                    self.zoom_percent,
                    &mut self.editing_text_buffer,
                );

                // Floating vertical quick tools pill docked to the left of the canvas
                let pill_x = canvas_resp.rect.left() + 14.0;
                let pill_y = canvas_resp.rect.top() + 16.0;
                render_floating_quick_pill(
                    ui,
                    &mut self.active_tool,
                    Pos2::new(pill_x, pill_y),
                );
            });
    }
}
