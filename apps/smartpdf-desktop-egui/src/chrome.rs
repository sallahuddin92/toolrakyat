//! Top chrome (two-tier Acrobat bar), floating quick tools pill, and bottom status bar.

use egui::{
    Align, Align2, Color32, CornerRadius, Layout, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2,
};
use starpdf_app_core::command::AppCommand;
use starpdf_app_core::export::ExportProfile;
use starpdf_app_core::session::SessionManager;

use crate::icons;
use crate::panels::{AcrobatMode, ActiveTool, LeftPanelMode};
use crate::theme::{DesignTokens, ThemeMode};

pub fn render_top_tier1(
    ui: &mut Ui,
    tokens: &DesignTokens,
    manager: &mut SessionManager,
    theme_mode: &mut ThemeMode,
    gpu_enabled: &mut bool,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);

        // SmartPDF Brand Logo
        ui.label(
            RichText::new("SmartPDF")
                .strong()
                .size(15.0)
                .color(tokens.accent),
        );

        ui.separator();

        // Document Tabs
        let tabs = manager.tabs();

        for tab in tabs {
            let tab_bg = if tab.is_active { tokens.card_bg } else { Color32::TRANSPARENT };
            let tab_border = if tab.is_active { tokens.border } else { Color32::TRANSPARENT };
            let text_color = if tab.is_active { tokens.text_primary } else { tokens.text_secondary };

            let (rect, resp) = ui.allocate_exact_size(Vec2::new(140.0, 26.0), Sense::click());
            if ui.is_rect_visible(rect) {
                ui.painter().rect_filled(rect, CornerRadius::same(4), tab_bg);
                if tab.is_active {
                    ui.painter().rect_stroke(rect, CornerRadius::same(4), Stroke::new(1.0, tab_border), egui::StrokeKind::Inside);
                }

                // Title snippet
                let title_text = if tab.title.len() > 14 {
                    format!("{}...", &tab.title[..12])
                } else {
                    tab.title.clone()
                };

                ui.painter().text(
                    egui::pos2(rect.left() + 8.0, rect.center().y),
                    Align2::LEFT_CENTER,
                    title_text,
                    egui::FontId::proportional(12.0),
                    text_color,
                );

                // Close tab button on the right
                let close_rect = Rect::from_center_size(egui::pos2(rect.right() - 12.0, rect.center().y), Vec2::splat(12.0));
                let uri = format!("bytes://icons/{:p}", icons::X);
                let img = egui::Image::from_bytes(uri, icons::X)
                    .fit_to_exact_size(Vec2::splat(10.0))
                    .tint(tokens.text_faint);
                img.paint_at(ui, close_rect);
            }

            if resp.clicked() {
                let _ = manager.dispatch(AppCommand::SwitchTab { session_id: tab.session_id });
            }
        }

        // Add Tab (+) button
        if icons::icon_button(ui, icons::PLUS, tokens, 14.0, false, "Open another document").clicked() {
            if let Some(path) = rfd::FileDialog::new().add_filter("PDF", &["pdf"]).pick_file() {
                if let Ok(bytes) = std::fs::read(&path) {
                    let title = path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "document.pdf".to_string());
                    let _ = manager.dispatch(AppCommand::OpenDocument { path: Some(path), bytes, title });
                }
            }
        }

        // Global Actions on the right
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            // Theme toggle
            let is_dark = *theme_mode == ThemeMode::Dark;
            let theme_icon = if is_dark { icons::SUN } else { icons::MOON };
            if icons::icon_button(ui, theme_icon, tokens, 16.0, false, "Toggle Light / Dark Theme").clicked() {
                *theme_mode = if is_dark { ThemeMode::Light } else { ThemeMode::Dark };
            }

            // GPU Acceleration toggle
            let gpu_icon = icons::CPU;
            if icons::icon_button(ui, gpu_icon, tokens, 16.0, *gpu_enabled, "Toggle Metal / GPU Acceleration").clicked() {
                *gpu_enabled = !*gpu_enabled;
                if let Some(s) = manager.active_session() {
                    s.invalidate_render_cache();
                }
            }

            ui.separator();

            // Export
            if icons::icon_button(ui, icons::SHARE, tokens, 16.0, false, "Export Document...").clicked() {
                if let Some(path) = rfd::FileDialog::new().add_filter("PDF", &["pdf"]).set_file_name("exported.pdf").save_file() {
                    if let Ok(res) = manager.dispatch(AppCommand::ExportDocument {
                        profile: ExportProfile::StandardPdf,
                    }) {
                        if let Some(b) = res.exported_bytes {
                            let _ = std::fs::write(&path, b);
                        }
                    }
                }
            }

            // Save
            if icons::icon_button(ui, icons::SAVE, tokens, 16.0, false, "Save Document (Cmd+S)").clicked() {
                let existing_path = manager.active_session().and_then(|s| s.file_path.clone());
                let dest = existing_path.or_else(|| {
                    rfd::FileDialog::new().add_filter("PDF", &["pdf"]).set_file_name("document.pdf").save_file()
                });
                if let Some(p) = dest {
                    let _ = manager.dispatch(AppCommand::SaveDocument { destination_path: Some(p) });
                }
            }

            // Open
            if icons::icon_button(ui, icons::FOLDER_OPEN, tokens, 16.0, false, "Open PDF Document (Cmd+O)").clicked() {
                if let Some(path) = rfd::FileDialog::new().add_filter("PDF", &["pdf"]).pick_file() {
                    if let Ok(bytes) = std::fs::read(&path) {
                        let title = path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "document.pdf".to_string());
                        let _ = manager.dispatch(AppCommand::OpenDocument { path: Some(path), bytes, title });
                    }
                }
            }

            // Redo
            if icons::icon_button(ui, icons::REDO, tokens, 16.0, false, "Redo (Cmd+Shift+Z)").clicked() {
                let _ = manager.dispatch(AppCommand::Redo);
            }

            // Undo
            if icons::icon_button(ui, icons::UNDO, tokens, 16.0, false, "Undo (Cmd+Z)").clicked() {
                let _ = manager.dispatch(AppCommand::Undo);
            }
        });
    });
}

pub fn render_top_tier2(
    ui: &mut Ui,
    tokens: &DesignTokens,
    active_mode: &mut AcrobatMode,
    left_panel: &mut LeftPanelMode,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

        mode_tab(ui, tokens, "All Tools", *active_mode == AcrobatMode::AllTools, || {
            *active_mode = AcrobatMode::AllTools;
            *left_panel = LeftPanelMode::AllTools;
        });

        ui.separator();

        mode_tab(ui, tokens, "Edit", *active_mode == AcrobatMode::Edit, || {
            *active_mode = AcrobatMode::Edit;
        });

        mode_tab(ui, tokens, "Comment", *active_mode == AcrobatMode::Comment, || {
            *active_mode = AcrobatMode::Comment;
        });

        mode_tab(ui, tokens, "Forms", *active_mode == AcrobatMode::Forms, || {
            *active_mode = AcrobatMode::Forms;
        });

        mode_tab(ui, tokens, "Sign", *active_mode == AcrobatMode::Sign, || {
            *active_mode = AcrobatMode::Sign;
        });

        mode_tab(ui, tokens, "Organize", *active_mode == AcrobatMode::Organize, || {
            *active_mode = AcrobatMode::Organize;
            *left_panel = LeftPanelMode::Thumbnails;
        });
    });
}

fn mode_tab<F: FnOnce()>(ui: &mut Ui, tokens: &DesignTokens, label: &str, is_selected: bool, on_click: F) {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(76.0, 28.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let is_hovered = resp.hovered();
        let bg = if is_selected {
            tokens.selected_bg
        } else if is_hovered {
            tokens.hover_bg
        } else {
            Color32::TRANSPARENT
        };

        let text_color = if is_selected {
            tokens.accent
        } else {
            tokens.text_primary
        };

        ui.painter().rect_filled(rect, tokens.radius, bg);

        if is_selected {
            let indicator_rect = Rect::from_min_max(
                egui::pos2(rect.left() + 6.0, rect.bottom() - 2.0),
                egui::pos2(rect.right() - 6.0, rect.bottom()),
            );
            ui.painter().rect_filled(indicator_rect, CornerRadius::same(1), tokens.accent);
        }

        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(12.5),
            text_color,
        );
    }

    if resp.clicked() {
        on_click();
    }
}

pub fn render_floating_quick_pill(
    ui: &mut Ui,
    tokens: &DesignTokens,
    active_tool: &mut ActiveTool,
    pill_pos: Pos2,
) {
    let pill_rect = Rect::from_min_size(pill_pos, Vec2::new(260.0, 40.0));
    let painter = ui.painter();

    // Drop shadow
    let shadow_rect = pill_rect.translate(Vec2::new(0.0, 3.0));
    painter.rect_filled(shadow_rect, CornerRadius::same(20), tokens.page_shadow);

    // Pill background
    painter.rect_filled(pill_rect, CornerRadius::same(20), tokens.card_bg);
    painter.rect_stroke(pill_rect, CornerRadius::same(20), Stroke::new(1.0, tokens.border), egui::StrokeKind::Inside);

    // Put buttons inside pill
    let mut pill_ui = ui.new_child(egui::UiBuilder::new().max_rect(pill_rect));
    pill_ui.horizontal_centered(|ui| {
        ui.add_space(8.0);
        ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

        if icons::icon_button(ui, icons::MOUSE_POINTER, tokens, 16.0, *active_tool == ActiveTool::Select, "Select Tool (V)").clicked() {
            *active_tool = ActiveTool::Select;
        }

        if icons::icon_button(ui, icons::HAND, tokens, 16.0, *active_tool == ActiveTool::Hand, "Pan / Hand Tool (H)").clicked() {
            *active_tool = ActiveTool::Hand;
        }

        ui.separator();

        if icons::icon_button(ui, icons::HIGHLIGHTER, tokens, 16.0, *active_tool == ActiveTool::Highlight, "Highlight Annotation (U)").clicked() {
            *active_tool = ActiveTool::Highlight;
        }

        if icons::icon_button(ui, icons::STICKY_NOTE, tokens, 16.0, *active_tool == ActiveTool::StickyNote, "Sticky Note (N)").clicked() {
            *active_tool = ActiveTool::StickyNote;
        }

        if icons::icon_button(ui, icons::TYPE, tokens, 16.0, *active_tool == ActiveTool::AddText || *active_tool == ActiveTool::EditText, "Add / Edit Text (T)").clicked() {
            *active_tool = ActiveTool::EditText;
        }

        if icons::icon_button(ui, icons::PEN_TOOL, tokens, 16.0, *active_tool == ActiveTool::Sign, "Fill & Sign (S)").clicked() {
            *active_tool = ActiveTool::Sign;
        }
    });
}

pub fn render_bottom_status_bar(
    ui: &mut Ui,
    tokens: &DesignTokens,
    manager: &mut SessionManager,
    zoom_percent: &mut f32,
    render_telemetry: &str,
    status_message: &str,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(10.0, 0.0);

        // Status message
        ui.label(
            RichText::new(status_message)
                .size(11.0)
                .color(tokens.text_secondary),
        );

        // Controls on the right
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            // Engine badge (e.g. "⚡ Metal GPU (1.2 ms)" or "⚙ CPU (3.4 ms)")
            ui.label(
                RichText::new(render_telemetry)
                    .size(10.5)
                    .color(tokens.accent_text)
                    .strong(),
            );

            ui.separator();

            // Fit Page
            if icons::icon_button(ui, icons::SCAN, tokens, 14.0, false, "Fit Entire Page (Cmd+0)").clicked() {
                *zoom_percent = 100.0;
                let _ = manager.dispatch(AppCommand::SetZoom { percent: 100.0 });
            }

            // Fit Width
            if icons::icon_button(ui, icons::MAXIMIZE, tokens, 14.0, false, "Fit Width (Cmd+1)").clicked() {
                *zoom_percent = 135.0;
                let _ = manager.dispatch(AppCommand::SetZoom { percent: 135.0 });
            }

            // Zoom In (+)
            if icons::icon_button(ui, icons::PLUS, tokens, 12.0, false, "Zoom In (Cmd++)").clicked() {
                *zoom_percent = (*zoom_percent * 1.25).min(500.0);
                let _ = manager.dispatch(AppCommand::SetZoom { percent: *zoom_percent });
            }

            // Zoom Percent text
            ui.label(
                RichText::new(format!("{:.0}%", *zoom_percent))
                    .size(11.5)
                    .color(tokens.text_primary),
            );

            // Zoom Out (-)
            if icons::icon_button(ui, icons::MINUS, tokens, 12.0, false, "Zoom Out (Cmd+-)").clicked() {
                *zoom_percent = (*zoom_percent / 1.25).max(25.0);
                let _ = manager.dispatch(AppCommand::SetZoom { percent: *zoom_percent });
            }

            ui.separator();

            // Page Navigation (< Page X of Y >)
            let (cur_page, total_pages) = if let Some(s) = manager.active_session() {
                (s.current_page, s.total_pages)
            } else {
                (0, 0)
            };

            if icons::icon_button(ui, icons::CHEVRON_RIGHT, tokens, 14.0, false, "Next Page").clicked() {
                if cur_page + 1 < total_pages {
                    let _ = manager.dispatch(AppCommand::SetPage { page_index: cur_page + 1 });
                }
            }

            ui.label(
                RichText::new(format!("Page {} of {}", cur_page + 1, total_pages.max(1)))
                    .size(11.5)
                    .color(tokens.text_primary),
            );

            if icons::icon_button(ui, icons::CHEVRON_LEFT, tokens, 14.0, false, "Previous Page").clicked() {
                if cur_page > 0 {
                    let _ = manager.dispatch(AppCommand::SetPage { page_index: cur_page - 1 });
                }
            }
        });
    });
}
