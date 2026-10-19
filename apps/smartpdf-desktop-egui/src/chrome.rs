//! Top chrome and window controls directly adopting upstream PdfCraft geometry and styling.

use egui::{
    Align, Align2, CornerRadius, Layout, Pos2, Rect, Sense, Stroke, Ui, Vec2, vec2,
};
use starpdf_app_core::command::AppCommand;
use starpdf_app_core::session::SessionManager;

use crate::icons;
use crate::panels::{AcrobatMode, ActiveTool};
use crate::theme::{self, ThemeKind, Tokens};
use crate::widgets;

pub const MAC_TITLE_BAR: f32 = 28.0;
pub const MAC_LIGHT: f32 = 12.0;
pub const MAC_LIGHT_PITCH: f32 = 20.0;
pub const MAC_LIGHTS_LEFT: f32 = 10.0;
pub const MAC_CONTENT_LEFT: f32 = MAC_LIGHTS_LEFT + 2.0 * MAC_LIGHT_PITCH + MAC_LIGHT + (MAC_TITLE_BAR - MAC_LIGHT) / 2.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TabStripAction {
    None,
    OpenRequested,
    SwitchTab(String),
    CloseTab(String),
    ToggleTheme,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModeBarAction {
    None,
    SaveRequested,
    ToggleLeftPanel,
    SelectMode(AcrobatMode),
}

pub fn render_tab_strip(
    ui: &mut Ui,
    manager: &mut SessionManager,
    theme_kind: &mut ThemeKind,
) -> TabStripAction {
    let t = Tokens::get(ui.ctx());
    let mac = cfg!(target_os = "macos");
    let left = if mac { MAC_CONTENT_LEFT } else { 8.0 };
    let tab_h = MAC_TITLE_BAR - 2.0;
    let button_size = 24.0;
    let mut action = TabStripAction::None;

    ui.horizontal(|ui| {
        ui.add_space(left);
        ui.spacing_mut().item_spacing.x = 4.0;

        // Home button
        if icons::button(ui, "house", button_size, false, "Home").clicked() {
            // Home view
        }

        // Open button
        if widgets::ghost_button_sized(ui, "plus", "Open", tab_h).clicked() {
            action = TabStripAction::OpenRequested;
        }

        ui.add_space(4.0);

        // Document tabs
        let tabs = manager.tabs();
        for tab in tabs {
            let active = tab.is_active;
            let tab_width = 180.0;
            let (rect, resp) = ui.allocate_exact_size(vec2(tab_width, tab_h), Sense::click());

            if ui.is_rect_visible(rect) {
                let _press = theme::Press::track(ui, &resp);
                let bg = if active {
                    t.chrome
                } else if resp.hovered() {
                    t.hover
                } else {
                    t.titlebar
                };

                ui.painter().rect_filled(rect, CornerRadius { nw: t.radius, ne: t.radius, sw: 0, se: 0 }, bg);
                if active {
                    ui.painter().rect_stroke(
                        rect,
                        CornerRadius { nw: t.radius, ne: t.radius, sw: 0, se: 0 },
                        Stroke::new(1.0, t.border),
                        egui::StrokeKind::Inside,
                    );
                }

                // File icon
                let icon_rect = Rect::from_min_size(egui::pos2(rect.left() + 8.0, rect.center().y - 7.0), Vec2::splat(14.0));
                icons::paint(ui, icon_rect, "file-text", 14.0, if active { t.accent } else { t.text_muted });

                // Title label
                let mut title = tab.title.clone();
                if title.chars().count() > 18 {
                    title = format!("{}…", &title.chars().take(17).collect::<String>());
                }
                ui.painter().text(
                    egui::pos2(rect.left() + 26.0, rect.center().y),
                    Align2::LEFT_CENTER,
                    title,
                    if active { theme::medium(12.5) } else { theme::regular(12.5) },
                    if active { t.text } else { t.text_muted },
                );

                // Close button
                let close_rect = Rect::from_center_size(egui::pos2(rect.right() - 14.0, rect.center().y), Vec2::splat(16.0));
                let close_resp = ui.interact(close_rect, ui.id().with(("close_tab", &tab.session_id)), Sense::click());
                if close_resp.hovered() {
                    ui.painter().rect_filled(close_rect, CornerRadius::same(3), t.hover);
                }
                icons::paint(ui, close_rect, "x", 12.0, if close_resp.hovered() { t.text } else { t.text_faint });

                if close_resp.clicked() {
                    action = TabStripAction::CloseTab(tab.session_id.clone());
                }
            }

            if action == TabStripAction::None && resp.clicked() {
                action = TabStripAction::SwitchTab(tab.session_id.clone());
            }
        }

        // Right side utilities (Theme toggle, shortcuts)
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            let theme_ico = if *theme_kind == ThemeKind::Dark { "sun" } else { "moon" };
            if icons::button(ui, theme_ico, button_size, false, "Toggle Light/Dark Theme").clicked() {
                action = TabStripAction::ToggleTheme;
            }

            if icons::button(ui, "circle-help", button_size, false, "Keyboard Shortcuts").clicked() {
                // Dialog
            }
        });
    });

    action
}

pub fn render_mode_bar(
    ui: &mut Ui,
    active_mode: AcrobatMode,
) -> ModeBarAction {
    let t = Tokens::get(ui.ctx());
    let mut action = ModeBarAction::None;

    ui.horizontal_centered(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;

        // Menu button
        if widgets::ghost_button(ui, "panel-left", "Menu").clicked() {
            action = ModeBarAction::ToggleLeftPanel;
        }

        ui.add_space(6.0);
        ui.painter().vline(ui.cursor().left(), ui.max_rect().y_range().shrink(12.0), Stroke::new(1.0, t.divider));
        ui.add_space(10.0);

        // Acrobat mode tabs
        let modes = [
            (AcrobatMode::AllTools, "All tools"),
            (AcrobatMode::Read, "Read"),
            (AcrobatMode::Edit, "Edit"),
            (AcrobatMode::Convert, "Convert"),
            (AcrobatMode::Sign, "E-Sign"),
        ];

        for (mode, label) in modes {
            if widgets::mode_tab(ui, label, active_mode == mode).clicked() {
                action = ModeBarAction::SelectMode(mode);
            }
        }

        // Right side quick actions
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;

            if icons::button(ui, "printer", 32.0, false, "Print (⌘P)").clicked() {
                // Print command
            }

            if icons::button(ui, "save", 32.0, false, "Save (⌘S)").clicked() {
                action = ModeBarAction::SaveRequested;
            }

            if icons::button(ui, "info", 32.0, false, "Document Properties (⌘D)").clicked() {
                // Properties dialog
            }

            ui.add_space(8.0);
            if widgets::search_box(ui, "Find tools and commands", 260.0).clicked() {
                // Palette
            }
        });
    });

    action
}

pub fn render_floating_quick_pill(
    ui: &mut Ui,
    active_tool: &mut ActiveTool,
    pos: Pos2,
) {
    let t = Tokens::get(ui.ctx());
    let pill_rect = Rect::from_min_size(pos, Vec2::new(42.0, 230.0));

    // Shadow
    ui.painter().rect_filled(
        pill_rect.translate(Vec2::new(2.0, 3.0)),
        CornerRadius::same(12),
        t.page_shadow,
    );

    // Pill background
    ui.painter().rect_filled(pill_rect, CornerRadius::same(12), t.card);
    ui.painter().rect_stroke(pill_rect, CornerRadius::same(12), Stroke::new(1.0, t.border), egui::StrokeKind::Inside);

    let tools = [
        ("mouse-pointer-2", ActiveTool::Select, "Select tool (V)"),
        ("hand", ActiveTool::Hand, "Hand tool (H)"),
        ("message-square-plus", ActiveTool::StickyNote, "Add comment"),
        ("highlighter", ActiveTool::Highlight, "Highlight text (U)"),
        ("pencil", ActiveTool::DrawShape, "Draw / Markup"),
        ("sticky-note", ActiveTool::StickyNote, "Sticky note (N)"),
    ];

    let mut start_y = pill_rect.top() + 6.0;
    for (icon, tool, tip) in tools {
        let is_active = *active_tool == tool;
        let btn_rect = Rect::from_min_size(
            Pos2::new(pill_rect.left() + 5.0, start_y),
            Vec2::splat(32.0),
        );
        let resp = ui.interact(btn_rect, ui.id().with(("quick_tool", icon)), Sense::click());

        if is_active {
            ui.painter().rect_filled(btn_rect, CornerRadius::same(8), t.selected);
            ui.painter().rect_stroke(btn_rect, CornerRadius::same(8), Stroke::new(1.0, t.accent), egui::StrokeKind::Inside);
        } else if resp.hovered() {
            ui.painter().rect_filled(btn_rect, CornerRadius::same(8), t.hover);
        }

        let tint = if is_active { t.accent_text } else if resp.hovered() { t.text } else { t.icon };
        icons::paint(ui, btn_rect, icon, 16.0, tint);

        if resp.clicked() {
            *active_tool = tool;
        }

        let _ = theme::hand(resp).on_hover_text(tip);
        start_y += 36.0;
    }
}

pub fn render_bottom_status_bar(
    ui: &mut Ui,
    manager: &mut SessionManager,
    zoom_percent: &mut f32,
    telemetry: &str,
    status_msg: &str,
) {
    let t = Tokens::get(ui.ctx());

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;

        ui.label(
            egui::RichText::new(status_msg)
                .size(11.5)
                .color(t.text_muted),
        );

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 6.0;

            // Telemetry
            ui.label(
                egui::RichText::new(telemetry)
                    .size(11.0)
                    .color(t.accent_text),
            );

            ui.separator();

            // Zoom Steppers
            if icons::button(ui, "plus", 20.0, false, "Zoom In").clicked() {
                *zoom_percent = (*zoom_percent * 1.25).min(500.0);
                let _ = manager.dispatch(AppCommand::SetZoom { percent: *zoom_percent });
            }

            ui.label(
                egui::RichText::new(format!("{:.0}%", *zoom_percent))
                    .size(11.5)
                    .color(t.text),
            );

            if icons::button(ui, "minus", 20.0, false, "Zoom Out").clicked() {
                *zoom_percent = (*zoom_percent / 1.25).max(25.0);
                let _ = manager.dispatch(AppCommand::SetZoom { percent: *zoom_percent });
            }

            ui.separator();

            // Page Navigation
            let (cur_p, tot_p) = if let Some(s) = manager.active_session() {
                (s.current_page + 1, s.total_pages)
            } else {
                (1, 1)
            };

            if icons::button(ui, "chevron-left", 20.0, false, "Previous Page").clicked() {
                if let Some(s) = manager.active_session() {
                    if s.current_page > 0 {
                        let _ = manager.dispatch(AppCommand::SetPage { page_index: s.current_page - 1 });
                    }
                }
            }

            ui.label(
                egui::RichText::new(format!("Page {cur_p} of {tot_p}"))
                    .size(11.5)
                    .color(t.text),
            );

            if icons::button(ui, "chevron-right", 20.0, false, "Next Page").clicked() {
                if let Some(s) = manager.active_session() {
                    if s.current_page + 1 < s.total_pages {
                        let _ = manager.dispatch(AppCommand::SetPage { page_index: s.current_page + 1 });
                    }
                }
            }
        });
    });
}
