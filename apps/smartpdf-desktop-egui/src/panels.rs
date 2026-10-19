//! Side panels: Left Tool Panel, All Tools catalog, Thumbnails, and Right Rail.
//! Directly ported from upstream PdfCraft layout and widget geometry.

use egui::{
    Align2, Color32, CornerRadius, Rect, RichText, ScrollArea, Sense, Stroke, Ui, Vec2, pos2, vec2,
};
use starpdf_app_core::command::AppCommand;
use starpdf_app_core::session::SessionManager;

use crate::icons;
use crate::theme::{self, Tokens};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeftPanelMode {
    Closed,
    Thumbnails,
    Bookmarks,
    AllTools,
    Tool(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ActiveTool {
    Select,
    Hand,
    EditText,
    AddText,
    Highlight,
    StickyNote,
    DrawShape,
    FormField,
    Sign,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum AcrobatMode {
    AllTools,
    Read,
    Edit,
    Convert,
    Sign,
    Forms,
    Organize,
}

pub struct ToolItem {
    pub id: &'static str,
    pub icon: &'static str,
    pub label: &'static str,
    pub section: &'static str,
    pub ready: bool,
}

pub static EDIT_TOOLS: &[ToolItem] = &[
    ToolItem { id: "edit.text_images", icon: "pencil", label: "Edit text & images", section: "MODIFY PAGE", ready: true },
    ToolItem { id: "page.rotate", icon: "rotate-cw", label: "Rotate pages", section: "MODIFY PAGE", ready: true },
    ToolItem { id: "page.insert", icon: "file-plus", label: "Insert pages", section: "MODIFY PAGE", ready: true },
    ToolItem { id: "page.delete", icon: "trash-2", label: "Delete pages", section: "MODIFY PAGE", ready: true },
    ToolItem { id: "page.extract", icon: "file-output", label: "Extract pages", section: "MODIFY PAGE", ready: true },
    ToolItem { id: "page.organize", icon: "layout-grid", label: "Organize pages", section: "MODIFY PAGE", ready: true },
    ToolItem { id: "edit.add_text", icon: "type", label: "Text", section: "ADD CONTENT", ready: true },
    ToolItem { id: "edit.add_image", icon: "image-plus", label: "Image", section: "ADD CONTENT", ready: true },
    ToolItem { id: "edit.header_footer", icon: "rows-3", label: "Header and footer", section: "ADD CONTENT", ready: true },
    ToolItem { id: "edit.watermark", icon: "stamp", label: "Watermark", section: "ADD CONTENT", ready: true },
    ToolItem { id: "edit.link", icon: "link-2", label: "Link", section: "ADD CONTENT", ready: true },
    ToolItem { id: "edit.bates", icon: "hash", label: "Bates numbering", section: "ADD CONTENT", ready: true },
    ToolItem { id: "edit.background", icon: "paint-bucket", label: "Background", section: "ADD CONTENT", ready: true },
    ToolItem { id: "edit.attach", icon: "paperclip", label: "Attach file", section: "ADD CONTENT", ready: false },
];

pub static ALL_TOOLS_GROUPS: &[(&str, &str, &str, &[ToolItem])] = &[
    ("export", "file-output", "Export a PDF", &[]),
    ("fill_sign", "pen-line", "Fill & Sign", &[]),
    ("edit", "pencil", "Edit a PDF", EDIT_TOOLS),
    ("create", "file-plus", "Create a PDF", &[]),
    ("combine", "files", "Combine files", &[]),
    ("organize", "layout-grid", "Organize pages", &[]),
    ("comment", "message-square-text", "Add comments", &[]),
    ("ocr", "scan-text", "Scan & OCR", &[]),
    ("protect", "shield-check", "Protect a PDF", &[]),
    ("redact", "rectangle-horizontal", "Redact a PDF", &[]),
    ("compress", "file-down", "Compress a PDF", &[]),
    ("form", "square-dashed-mouse-pointer", "Prepare a form", &[]),
    ("stamp", "stamp", "Add a stamp", &[]),
    ("cert", "badge-check", "Use a certificate", &[]),
];

pub fn render_left_panel(
    ui: &mut Ui,
    manager: &mut SessionManager,
    left_panel: &mut LeftPanelMode,
    active_mode: &mut AcrobatMode,
    active_tool: &mut ActiveTool,
    thumbnails: &std::collections::HashMap<u32, egui::TextureHandle>,
) {
    let t = Tokens::get(ui.ctx());

    egui::Frame::NONE
        .fill(t.panel)
        .inner_margin(egui::Margin { left: 14, right: 10, top: 12, bottom: 10 })
        .stroke(Stroke::new(1.0, t.divider))
        .show(ui, |ui| {
            match *left_panel {
                LeftPanelMode::Closed => {}
                LeftPanelMode::Thumbnails => render_thumbnails_panel(ui, manager, thumbnails, left_panel),
                LeftPanelMode::Bookmarks => render_bookmarks_panel(ui, left_panel),
                LeftPanelMode::AllTools => render_all_tools_list(ui, left_panel),
                LeftPanelMode::Tool("edit") => render_edit_tool_panel(ui, manager, left_panel, active_mode, active_tool),
                LeftPanelMode::Tool(name) => render_generic_tool_panel(ui, name, left_panel),
            }
        });
}

fn panel_header(ui: &mut Ui, title: &str, back: bool) -> (bool, bool) {
    let t = Tokens::get(ui.ctx());
    let mut go_back = false;
    let mut close = false;

    ui.horizontal(|ui| {
        if back && icons::button(ui, "chevron-left", 26.0, false, "Back to all tools").clicked() {
            go_back = true;
        }

        let title_width = (ui.available_width() - 26.0 - ui.spacing().item_spacing.x).max(1.0);
        ui.add_sized(
            [title_width, 26.0],
            egui::Label::new(RichText::new(title).font(theme::semibold(15.5)).color(t.text)).truncate(),
        );

        if icons::button(ui, "x", 26.0, false, "Close panel").clicked() {
            close = true;
        }
    });

    ui.add_space(2.0);
    ui.separator();
    ui.add_space(4.0);
    (go_back, close)
}

pub fn render_all_tools_list(ui: &mut Ui, left_panel: &mut LeftPanelMode) {
    let t = Tokens::get(ui.ctx());
    let (_, close) = panel_header(ui, "All tools", false);
    if close {
        *left_panel = LeftPanelMode::Closed;
    }

    ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        for (id, icon, label, _) in ALL_TOOLS_GROUPS {
            let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::click());
            if ui.is_rect_visible(rect) {
                let press = theme::Press::track(ui, &resp);
                press.wash(ui, rect, CornerRadius::same(t.radius), false);

                let body = rect.translate(press.offset());
                icons::paint(
                    ui,
                    Rect::from_min_size(body.min + vec2(6.0, 8.0), Vec2::splat(18.0)),
                    icon,
                    17.0,
                    t.accent,
                );

                ui.painter().text(
                    body.left_center() + vec2(34.0, 0.0),
                    Align2::LEFT_CENTER,
                    *label,
                    theme::regular(13.0),
                    t.text,
                );
            }

            if resp.clicked() {
                if *id == "organize" {
                    *left_panel = LeftPanelMode::Thumbnails;
                } else {
                    *left_panel = LeftPanelMode::Tool(id);
                }
            }
        }
    });
}

pub fn render_edit_tool_panel(
    ui: &mut Ui,
    manager: &mut SessionManager,
    left_panel: &mut LeftPanelMode,
    active_mode: &mut AcrobatMode,
    active_tool: &mut ActiveTool,
) {
    let t = Tokens::get(ui.ctx());
    let (back, close) = panel_header(ui, "Edit a PDF", true);
    if back {
        *left_panel = LeftPanelMode::AllTools;
    }
    if close {
        *left_panel = LeftPanelMode::Closed;
    }

    // Planned milestone banner
    egui::Frame::NONE
        .fill(t.accent_soft)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new("Coming in milestone M7. Items marked Ready work today.")
                    .color(t.text)
                    .font(theme::regular(12.0)),
            );
        });

    ui.add_space(8.0);

    ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        let mut cur_section = "";

        for item in EDIT_TOOLS {
            if item.section != cur_section {
                cur_section = item.section;
                ui.add_space(10.0);
                ui.label(RichText::new(cur_section).font(theme::semibold(10.0)).color(t.text_faint));
                ui.add_space(2.0);
            }

            let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::click());
            if ui.is_rect_visible(rect) {
                let press = theme::Press::track(ui, &resp);
                press.wash(ui, rect, CornerRadius::same(t.radius), false);

                let body = rect.translate(press.offset());
                icons::paint(
                    ui,
                    Rect::from_min_size(body.min + vec2(6.0, 8.0), Vec2::splat(18.0)),
                    item.icon,
                    17.0,
                    if item.ready { t.accent } else { t.text_faint },
                );

                // Label
                let fg = if item.ready { t.text } else { t.text_muted };
                ui.painter().text(
                    body.left_center() + vec2(34.0, 0.0),
                    Align2::LEFT_CENTER,
                    item.label,
                    theme::regular(13.0),
                    fg,
                );

                // Ready chip
                let chip_rect = Rect::from_center_size(body.right_center() - vec2(28.0, 0.0), vec2(44.0, 16.0));
                if item.ready {
                    ui.painter().rect_filled(chip_rect, CornerRadius::same(4), Color32::from_rgb(0xDD, 0xF3, 0xE4));
                    ui.painter().text(chip_rect.center(), Align2::CENTER_CENTER, "Ready", theme::semibold(9.5), Color32::from_rgb(0x1E, 0x7B, 0x43));
                } else {
                    ui.painter().rect_filled(chip_rect, CornerRadius::same(4), t.pressed);
                    ui.painter().text(chip_rect.center(), Align2::CENTER_CENTER, "M12", theme::semibold(9.5), t.text_muted);
                }
            }

            if resp.clicked() && item.ready {
                if item.id == "edit.text_images" {
                    *active_mode = AcrobatMode::Edit;
                    *active_tool = ActiveTool::EditText;
                } else if item.id == "page.organize" {
                    *left_panel = LeftPanelMode::Thumbnails;
                } else if item.id == "page.rotate" {
                    // Page rotate action placeholder
                } else if item.id == "page.delete" {
                    if let Some(session) = manager.active_session() {
                        let cur = session.current_page;
                        let _ = manager.dispatch(AppCommand::DeletePage { page_index: cur });
                    }
                }
            }
        }
    });
}

fn render_generic_tool_panel(ui: &mut Ui, name: &str, left_panel: &mut LeftPanelMode) {
    let (_, close) = panel_header(ui, name, true);
    if close {
        *left_panel = LeftPanelMode::Closed;
    }
    ui.label(format!("Tool panel for '{name}' is active."));
}

pub fn render_thumbnails_panel(
    ui: &mut Ui,
    manager: &mut SessionManager,
    thumbnail_textures: &std::collections::HashMap<u32, egui::TextureHandle>,
    left_panel: &mut LeftPanelMode,
) {
    let t = Tokens::get(ui.ctx());
    let (cur_p, tot_p) = if let Some(s) = manager.active_session() {
        (s.current_page, s.total_pages)
    } else {
        (0, 0)
    };

    let (_, close) = panel_header(ui, &format!("Pages  {tot_p}"), false);
    if close {
        *left_panel = LeftPanelMode::Closed;
    }

    ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        for p in 0..tot_p {
            let is_current = p == cur_p;
            let card_w = ui.available_width() - 16.0;
            let card_h = 170.0;

            let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), card_h), Sense::click());
            if ui.is_rect_visible(rect) {
                let inner_rect = Rect::from_center_size(rect.center() - vec2(0.0, 10.0), vec2(card_w.min(130.0), 140.0));

                if is_current {
                    ui.painter().rect_stroke(
                        inner_rect.expand(3.0),
                        CornerRadius::same(4),
                        Stroke::new(2.0, t.accent),
                        egui::StrokeKind::Outside,
                    );
                }

                if let Some(texture) = thumbnail_textures.get(&p) {
                    ui.painter().image(
                        texture.id(),
                        inner_rect,
                        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                } else {
                    ui.painter().rect_filled(inner_rect, CornerRadius::same(2), Color32::WHITE);
                    ui.painter().rect_stroke(inner_rect, CornerRadius::same(2), Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
                }

                // Page number under thumbnail
                ui.painter().text(
                    pos2(rect.center().x, rect.bottom() - 6.0),
                    Align2::CENTER_CENTER,
                    format!("{}", p + 1),
                    theme::medium(12.0),
                    if is_current { t.accent_text } else { t.text },
                );
            }

            if resp.clicked() {
                let _ = manager.dispatch(AppCommand::SetPage { page_index: p });
            }
        }
    });
}

pub fn render_bookmarks_panel(ui: &mut Ui, left_panel: &mut LeftPanelMode) {
    let t = Tokens::get(ui.ctx());
    let (_, close) = panel_header(ui, "Bookmarks", false);
    if close {
        *left_panel = LeftPanelMode::Closed;
    }
    ui.label(RichText::new("No bookmarks found in document.").color(t.text_muted));
}

pub fn render_right_rail(
    ui: &mut Ui,
    left_panel: &mut LeftPanelMode,
) {
    let button_size = 32.0;

    ui.vertical(|ui| {
        ui.add_space(8.0);

        let is_thumbs = *left_panel == LeftPanelMode::Thumbnails;
        if icons::button(ui, "copy", button_size, is_thumbs, "Page thumbnails").clicked() {
            *left_panel = if is_thumbs { LeftPanelMode::Closed } else { LeftPanelMode::Thumbnails };
        }

        let is_bm = *left_panel == LeftPanelMode::Bookmarks;
        if icons::button(ui, "bookmark", button_size, is_bm, "Bookmarks").clicked() {
            *left_panel = if is_bm { LeftPanelMode::Closed } else { LeftPanelMode::Bookmarks };
        }

        if icons::button(ui, "message-square-text", button_size, false, "Comments").clicked() {
            // Comments panel
        }

        if icons::button(ui, "layers", button_size, false, "Layers").clicked() {
            // Layers panel
        }

        if icons::button(ui, "paperclip", button_size, false, "Attachments").clicked() {
            // Attachments
        }
    });
}
