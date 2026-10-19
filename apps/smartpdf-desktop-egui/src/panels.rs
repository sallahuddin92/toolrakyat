//! Side panels: Left Tool Panel, All Tools catalog, Thumbnails, Right Rail, and Selection Inspector.
//! Directly ported from upstream PdfCraft layout and widget geometry, fully wired to EguiEngineAdapter.

use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, Rect, RichText, ScrollArea, Sense, Stroke, Ui, Vec2,
};
use starpdf_app_core::selection::SelectionState;

use crate::adapter::EguiEngineAdapter;
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
pub enum RightPanelMode {
    Closed,
    Inspector,
    Bookmarks,
    Comments,
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
    ToolItem {
        id: "edit.text_images",
        icon: "pencil",
        label: "Edit text & images",
        section: "MODIFY PAGE",
        ready: true,
    },
    ToolItem {
        id: "page.rotate",
        icon: "rotate-cw",
        label: "Rotate pages",
        section: "MODIFY PAGE",
        ready: false,
    },
    ToolItem {
        id: "page.insert",
        icon: "file-plus",
        label: "Insert pages",
        section: "MODIFY PAGE",
        ready: true,
    },
    ToolItem {
        id: "page.duplicate",
        icon: "copy-plus",
        label: "Duplicate page",
        section: "MODIFY PAGE",
        ready: true,
    },
    ToolItem {
        id: "page.delete",
        icon: "trash-2",
        label: "Delete pages",
        section: "MODIFY PAGE",
        ready: true,
    },
    ToolItem {
        id: "page.extract",
        icon: "file-output",
        label: "Extract pages",
        section: "MODIFY PAGE",
        ready: false,
    },
    ToolItem {
        id: "page.organize",
        icon: "layout-grid",
        label: "Organize pages",
        section: "MODIFY PAGE",
        ready: true,
    },
    ToolItem {
        id: "edit.add_text",
        icon: "type",
        label: "Text",
        section: "ADD CONTENT",
        ready: true,
    },
    ToolItem {
        id: "edit.add_image",
        icon: "image-plus",
        label: "Image",
        section: "ADD CONTENT",
        ready: true,
    },
    ToolItem {
        id: "edit.header_footer",
        icon: "rows-3",
        label: "Header and footer",
        section: "ADD CONTENT",
        ready: false,
    },
    ToolItem {
        id: "edit.watermark",
        icon: "stamp",
        label: "Watermark",
        section: "ADD CONTENT",
        ready: false,
    },
    ToolItem {
        id: "edit.link",
        icon: "link-2",
        label: "Link",
        section: "ADD CONTENT",
        ready: false,
    },
    ToolItem {
        id: "edit.bates",
        icon: "hash",
        label: "Bates numbering",
        section: "ADD CONTENT",
        ready: false,
    },
    ToolItem {
        id: "edit.background",
        icon: "paint-bucket",
        label: "Background",
        section: "ADD CONTENT",
        ready: false,
    },
    ToolItem {
        id: "edit.attach",
        icon: "paperclip",
        label: "Attach file",
        section: "ADD CONTENT",
        ready: false,
    },
];

pub static ALL_TOOLS_GROUPS: &[(&str, &str, &str, bool, &[ToolItem])] = &[
    ("export", "file-output", "Export a PDF", true, &[]),
    ("fill_sign", "pen-line", "Fill & Sign", true, &[]),
    ("edit", "pencil", "Edit a PDF", true, EDIT_TOOLS),
    ("organize", "layout-grid", "Organize pages", true, &[]),
    ("comment", "message-square-text", "Add comments", true, &[]),
    (
        "form",
        "square-dashed-mouse-pointer",
        "Prepare a form",
        true,
        &[],
    ),
    ("create", "file-plus", "Create a PDF", false, &[]),
    ("combine", "files", "Combine files", false, &[]),
    ("ocr", "scan-text", "Scan & OCR", false, &[]),
    ("protect", "shield-check", "Protect a PDF", false, &[]),
    ("redact", "rectangle-horizontal", "Redact a PDF", false, &[]),
    ("compress", "file-down", "Compress a PDF", false, &[]),
    ("stamp", "stamp", "Add a stamp", false, &[]),
    ("cert", "badge-check", "Use a certificate", false, &[]),
];

pub fn render_left_panel(
    ui: &mut Ui,
    adapter: &mut EguiEngineAdapter,
    left_panel: &mut LeftPanelMode,
    active_mode: &mut AcrobatMode,
    active_tool: &mut ActiveTool,
    thumbnails: &std::collections::HashMap<u32, egui::TextureHandle>,
) {
    let t = Tokens::get(ui.ctx());

    egui::Frame::NONE
        .fill(t.panel)
        .inner_margin(egui::Margin {
            left: 14,
            right: 10,
            top: 12,
            bottom: 10,
        })
        .stroke(Stroke::new(1.0, t.divider))
        .show(ui, |ui| match *left_panel {
            LeftPanelMode::Closed => {}
            LeftPanelMode::Thumbnails => {
                render_thumbnails_panel(ui, adapter, thumbnails, left_panel)
            }
            LeftPanelMode::Bookmarks => render_bookmarks_panel(ui, left_panel),
            LeftPanelMode::AllTools => {
                render_all_tools_list(ui, adapter, left_panel, active_mode, active_tool)
            }
            LeftPanelMode::Tool("edit") => {
                render_edit_tool_panel(ui, adapter, left_panel, active_mode, active_tool)
            }
            LeftPanelMode::Tool(name) => render_generic_tool_panel(ui, adapter, name, left_panel),
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
            egui::Label::new(
                RichText::new(title)
                    .font(theme::semibold(15.5))
                    .color(t.text),
            )
            .truncate(),
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

pub fn render_all_tools_list(
    ui: &mut Ui,
    adapter: &mut EguiEngineAdapter,
    left_panel: &mut LeftPanelMode,
    active_mode: &mut AcrobatMode,
    active_tool: &mut ActiveTool,
) {
    let t = Tokens::get(ui.ctx());
    let (_, close) = panel_header(ui, "All tools", false);
    if close {
        *left_panel = LeftPanelMode::Closed;
    }

    ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        for (id, icon, label, supported, _) in ALL_TOOLS_GROUPS {
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
                    if *supported { t.accent } else { t.text_faint },
                );

                ui.painter().text(
                    body.left_center() + vec2(34.0, 0.0),
                    Align2::LEFT_CENTER,
                    *label,
                    theme::regular(13.0),
                    if *supported { t.text } else { t.text_muted },
                );

                if !*supported {
                    let chip_rect = Rect::from_center_size(body.right_center() - vec2(28.0, 0.0), vec2(46.0, 16.0));
                    ui.painter().rect_filled(chip_rect, CornerRadius::same(4), t.pressed);
                    ui.painter().text(chip_rect.center(), Align2::CENTER_CENTER, "Planned", theme::semibold(9.0), t.text_muted);
                }
            }

            if resp.clicked() {
                if !*supported {
                    let refusal = format!(
                        "Feature '{}' is planned for a future milestone and is not yet configured in this standalone StarPDF engine.",
                        label
                    );
                    adapter.status_message = refusal.clone();
                    adapter.error_banner = Some(refusal);
                } else if *id == "organize" {
                    *left_panel = LeftPanelMode::Thumbnails;
                    *active_mode = AcrobatMode::Organize;
                } else if *id == "edit" {
                    *left_panel = LeftPanelMode::Tool("edit");
                    *active_mode = AcrobatMode::Edit;
                    *active_tool = ActiveTool::EditText;
                } else if *id == "comment" {
                    *active_tool = ActiveTool::StickyNote;
                    adapter.status_message = "Sticky note tool selected. Click anywhere on page to place note.".to_string();
                } else if *id == "form" {
                    *active_mode = AcrobatMode::Forms;
                    *active_tool = ActiveTool::FormField;
                    adapter.status_message = "Form tools selected. Click on form fields on page to inspect or toggle.".to_string();
                } else if *id == "fill_sign" {
                    *active_mode = AcrobatMode::Sign;
                    *active_tool = ActiveTool::Sign;
                    adapter.status_message = "Fill & Sign: Click text or check boxes on document to fill. Use Add Image to place signature.".to_string();
                } else {
                    *left_panel = LeftPanelMode::Tool(id);
                }
            }
        }
    });
}

pub fn render_edit_tool_panel(
    ui: &mut Ui,
    adapter: &mut EguiEngineAdapter,
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
                let chip_rect = Rect::from_center_size(body.right_center() - vec2(28.0, 0.0), vec2(46.0, 16.0));
                if item.ready {
                    ui.painter().rect_filled(chip_rect, CornerRadius::same(4), Color32::from_rgb(0xDD, 0xF3, 0xE4));
                    ui.painter().text(chip_rect.center(), Align2::CENTER_CENTER, "Ready", theme::semibold(9.5), Color32::from_rgb(0x1E, 0x7B, 0x43));
                } else {
                    ui.painter().rect_filled(chip_rect, CornerRadius::same(4), t.pressed);
                    ui.painter().text(chip_rect.center(), Align2::CENTER_CENTER, "Planned", theme::semibold(9.0), t.text_muted);
                }
            }

            if resp.clicked() {
                if !item.ready {
                    let refusal = format!(
                        "Feature '{}' is planned for a future milestone and is not yet implemented in StarPDF engine.",
                        item.label
                    );
                    adapter.status_message = refusal.clone();
                    adapter.error_banner = Some(refusal);
                } else {
                    match item.id {
                        "edit.text_images" => {
                            *active_mode = AcrobatMode::Edit;
                            *active_tool = ActiveTool::EditText;
                            adapter.status_message = "Click any text or image on document to select and edit.".to_string();
                        }
                        "page.organize" => {
                            *left_panel = LeftPanelMode::Thumbnails;
                            *active_mode = AcrobatMode::Organize;
                        }
                        "page.insert" => {
                            let _ = adapter.insert_blank_page();
                        }
                        "page.duplicate" => {
                            let _ = adapter.duplicate_current_page();
                        }
                        "page.delete" => {
                            let _ = adapter.delete_current_page();
                        }
                        "edit.add_image" => {
                            let _ = adapter.add_sample_image(100.0, 500.0, 120.0, 80.0);
                        }
                        "edit.add_text" => {
                            *active_tool = ActiveTool::StickyNote;
                            adapter.status_message = "Text tool active: click on document to place text/note.".to_string();
                        }
                        _ => {}
                    }
                }
            }
        }
    });
}

fn render_generic_tool_panel(
    ui: &mut Ui,
    adapter: &mut EguiEngineAdapter,
    name: &str,
    left_panel: &mut LeftPanelMode,
) {
    let (_, close) = panel_header(ui, name, true);
    if close {
        *left_panel = LeftPanelMode::Closed;
    }
    let t = Tokens::get(ui.ctx());
    ui.label(RichText::new(format!("Tool panel for '{name}'")).color(t.text));
    ui.add_space(8.0);
    ui.label(
        RichText::new(
            "This category is scheduled for migration and will connect directly to StarPDF.",
        )
        .color(t.text_muted)
        .size(12.0),
    );
    if ui.button("Back to All Tools").clicked() {
        *left_panel = LeftPanelMode::AllTools;
    }
    let _ = adapter;
}

pub fn render_thumbnails_panel(
    ui: &mut Ui,
    adapter: &mut EguiEngineAdapter,
    thumbnail_textures: &std::collections::HashMap<u32, egui::TextureHandle>,
    left_panel: &mut LeftPanelMode,
) {
    let t = Tokens::get(ui.ctx());
    let (cur_p, tot_p) = if let Some(s) = adapter.manager.active_session() {
        (s.current_page, s.total_pages)
    } else {
        (0, 0)
    };

    let (_, close) = panel_header(ui, &format!("Pages ({tot_p})"), false);
    if close {
        *left_panel = LeftPanelMode::Closed;
    }

    // Page Assembly Action Toolbar
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(4.0, 4.0);

        if ui
            .button(RichText::new("+ Blank").size(11.0))
            .on_hover_text("Insert blank page after current")
            .clicked()
        {
            let _ = adapter.insert_blank_page();
        }

        if ui
            .button(RichText::new("Duplicate").size(11.0))
            .on_hover_text("Duplicate current page")
            .clicked()
        {
            let _ = adapter.duplicate_current_page();
        }

        if ui
            .button(RichText::new("↑ Move").size(11.0))
            .on_hover_text("Move current page up")
            .clicked()
        {
            let _ = adapter.move_current_page_up();
        }

        if ui
            .button(RichText::new("↓ Move").size(11.0))
            .on_hover_text("Move current page down")
            .clicked()
        {
            let _ = adapter.move_current_page_down();
        }

        if ui
            .button(
                RichText::new("🗑 Delete")
                    .size(11.0)
                    .color(Color32::from_rgb(220, 60, 60)),
            )
            .on_hover_text("Delete current page")
            .clicked()
        {
            let _ = adapter.delete_current_page();
        }
    });

    ui.add_space(6.0);
    ui.separator();
    ui.add_space(6.0);

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for p in 0..tot_p {
                let is_current = p == cur_p;
                let card_w = ui.available_width() - 16.0;
                let card_h = 170.0;

                let (rect, resp) =
                    ui.allocate_exact_size(vec2(ui.available_width(), card_h), Sense::click());
                if ui.is_rect_visible(rect) {
                    let inner_rect = Rect::from_center_size(
                        rect.center() - vec2(0.0, 10.0),
                        vec2(card_w.min(130.0), 140.0),
                    );

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
                        ui.painter()
                            .rect_filled(inner_rect, CornerRadius::same(2), Color32::WHITE);
                        ui.painter().rect_stroke(
                            inner_rect,
                            CornerRadius::same(2),
                            Stroke::new(1.0, t.border),
                            egui::StrokeKind::Inside,
                        );
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
                    let _ = adapter.set_page(p);
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

/// Dedicated Right-Rail Selection Inspector.
/// Exposes properties and operations for the currently selected element:
/// - Text: in-place replacement buffer, commit/cancel, delete, move steppers.
/// - Image: replace image, move steppers, resize steppers, delete image.
/// - Annotation: contents editor, update, delete.
/// - Form field: checkbox toggle, text field editor, update.
pub fn render_selection_inspector(ui: &mut Ui, adapter: &mut EguiEngineAdapter) {
    let t = Tokens::get(ui.ctx());

    let selection = if let Some(session) = adapter.manager.active_session() {
        session.selection.clone()
    } else {
        SelectionState::None
    };

    ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        match selection {
            SelectionState::Text {
                ref star_node_id,
                text: _,
                is_editable,
                ref font_info,
                ..
            } => {
                ui.label(RichText::new("Text Inspector").font(theme::semibold(14.0)).color(t.text));
                ui.add_space(4.0);

                let (_, _, occ) = star_node_id.parts();
                ui.label(RichText::new(format!("Target: {occ}")).size(11.0).color(t.text_muted));

                if let Some(ref fi) = font_info {
                    ui.label(RichText::new(format!("Font: {} ({:.1}pt)", fi.name, fi.size)).size(11.0).color(t.text_muted));
                }

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);

                if !is_editable {
                    ui.horizontal(|ui| {
                        icons::paint(ui, Rect::from_min_size(ui.cursor().min, Vec2::splat(14.0)), "triangle-alert", 14.0, Color32::from_rgb(220, 140, 20));
                        ui.add_space(18.0);
                        ui.label(RichText::new("Read-Only Native Text").color(Color32::from_rgb(220, 140, 20)).size(11.5));
                    });
                    ui.label(RichText::new("This text uses an unsupported font encoding or layout that cannot be safely modified without font subsetting corruption.").size(10.5).color(t.text_muted));
                    ui.add_space(6.0);
                } else {
                    ui.label(RichText::new("Edit Text Content:").font(theme::medium(12.0)).color(t.text));
                    ui.add_space(2.0);

                    ui.text_edit_multiline(&mut adapter.editing_text_buffer);

                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.button(RichText::new("Commit Changes").color(Color32::WHITE))
                            .on_hover_text("Commit text changes into document (Enter)")
                            .clicked()
                        {
                            let new_text = adapter.editing_text_buffer.clone();
                            let _ = adapter.commit_text_edit(&new_text);
                        }

                        if ui.button(RichText::new("Cancel").color(t.text))
                            .on_hover_text("Revert text to original")
                            .clicked()
                        {
                            adapter.cancel_text_edit();
                        }
                    });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    ui.label(RichText::new("Reposition Text:").font(theme::medium(12.0)).color(t.text));
                    ui.horizontal(|ui| {
                        if ui.button("← 5pt").clicked() {
                            let _ = adapter.move_selected(-5.0, 0.0);
                        }
                        if ui.button("→ 5pt").clicked() {
                            let _ = adapter.move_selected(5.0, 0.0);
                        }
                        if ui.button("↑ 5pt").clicked() {
                            let _ = adapter.move_selected(0.0, 5.0);
                        }
                        if ui.button("↓ 5pt").clicked() {
                            let _ = adapter.move_selected(0.0, -5.0);
                        }
                    });

                    ui.add_space(8.0);
                    if ui.button(RichText::new("🗑 Delete Text").color(Color32::from_rgb(220, 60, 60))).clicked() {
                        let _ = adapter.delete_selected();
                    }
                }
            }

            SelectionState::Image { ref image_ref, bounds, .. } => {
                ui.label(RichText::new("Image Inspector").font(theme::semibold(14.0)).color(t.text));
                ui.add_space(4.0);
                ui.label(RichText::new(format!("Image Ref: {image_ref}")).size(11.0).color(t.text_muted));
                ui.label(RichText::new(format!("Size: {:.0} × {:.0} pt", bounds.width, bounds.height)).size(11.0).color(t.text_muted));

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);

                if ui.button(RichText::new("Replace Image").color(t.text)).clicked() {
                    let _ = adapter.replace_selected_image(crate::adapter::DEFAULT_SAMPLE_JPEG.to_vec());
                }

                ui.add_space(8.0);
                ui.label(RichText::new("Move Image:").font(theme::medium(12.0)).color(t.text));
                ui.horizontal(|ui| {
                    if ui.button("← 10pt").clicked() {
                        let _ = adapter.move_selected(-10.0, 0.0);
                    }
                    if ui.button("→ 10pt").clicked() {
                        let _ = adapter.move_selected(10.0, 0.0);
                    }
                    if ui.button("↑ 10pt").clicked() {
                        let _ = adapter.move_selected(0.0, 10.0);
                    }
                    if ui.button("↓ 10pt").clicked() {
                        let _ = adapter.move_selected(0.0, -10.0);
                    }
                });

                ui.add_space(8.0);
                ui.label(RichText::new("Resize Image:").font(theme::medium(12.0)).color(t.text));
                ui.horizontal(|ui| {
                    if ui.button("+10 Size").clicked() {
                        let _ = adapter.resize_selected(10.0, 10.0);
                    }
                    if ui.button("-10 Size").clicked() {
                        let _ = adapter.resize_selected(-10.0, -10.0);
                    }
                });

                ui.add_space(8.0);
                if ui.button(RichText::new("🗑 Delete Image").color(Color32::from_rgb(220, 60, 60))).clicked() {
                    let _ = adapter.delete_selected();
                }
            }

            SelectionState::Annotation { ref annot_ref, ref subtype, .. } => {
                ui.label(RichText::new("Annotation Inspector").font(theme::semibold(14.0)).color(t.text));
                ui.add_space(4.0);
                ui.label(RichText::new(format!("Type: {subtype}")).size(11.0).color(t.text_muted));
                ui.label(RichText::new(format!("Ref: {annot_ref}")).size(11.0).color(t.text_muted));

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);

                ui.label(RichText::new("Annotation Contents:").font(theme::medium(12.0)).color(t.text));
                ui.text_edit_multiline(&mut adapter.editing_annotation_buffer);

                ui.add_space(6.0);
                if ui.button(RichText::new("Update Contents").color(Color32::WHITE)).clicked() {
                    let contents = adapter.editing_annotation_buffer.clone();
                    let _ = adapter.update_selected_annotation(Some(contents));
                }

                ui.add_space(8.0);
                ui.label(RichText::new("Move Annotation:").font(theme::medium(12.0)).color(t.text));
                ui.horizontal(|ui| {
                    if ui.button("← 10pt").clicked() {
                        let _ = adapter.move_selected(-10.0, 0.0);
                    }
                    if ui.button("→ 10pt").clicked() {
                        let _ = adapter.move_selected(10.0, 0.0);
                    }
                    if ui.button("↑ 10pt").clicked() {
                        let _ = adapter.move_selected(0.0, 10.0);
                    }
                    if ui.button("↓ 10pt").clicked() {
                        let _ = adapter.move_selected(0.0, -10.0);
                    }
                });

                ui.add_space(8.0);
                if ui.button(RichText::new("🗑 Delete Annotation").color(Color32::from_rgb(220, 60, 60))).clicked() {
                    let _ = adapter.delete_selected();
                }
            }

            SelectionState::FormField { ref field_name, ref field_type, .. } => {
                ui.label(RichText::new("Form Field Inspector").font(theme::semibold(14.0)).color(t.text));
                ui.add_space(4.0);
                ui.label(RichText::new(format!("Name: {field_name}")).size(11.0).color(t.text_muted));
                ui.label(RichText::new(format!("Type: {field_type}")).size(11.0).color(t.text_muted));

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);

                if field_type == "Checkbox" {
                    let is_checked = adapter.editing_form_value_buffer.eq_ignore_ascii_case("true")
                        || adapter.editing_form_value_buffer.eq_ignore_ascii_case("yes");
                    ui.label(RichText::new(format!("Current State: {}", if is_checked { "Checked" } else { "Unchecked" })).font(theme::medium(12.0)).color(t.text));
                    ui.add_space(4.0);
                    if ui.button(RichText::new(if is_checked { "Uncheck Box" } else { "Check Box" }).color(t.text)).clicked() {
                        let _ = adapter.toggle_selected_form_checkbox();
                    }
                } else {
                    ui.label(RichText::new("Field Value:").font(theme::medium(12.0)).color(t.text));
                    ui.text_edit_singleline(&mut adapter.editing_form_value_buffer);
                    ui.add_space(6.0);
                    if ui.button(RichText::new("Apply Value").color(Color32::WHITE)).clicked() {
                        let val = adapter.editing_form_value_buffer.clone();
                        let _ = adapter.set_selected_form_field_value(&val);
                    }
                }
            }

            SelectionState::None => {
                ui.label(RichText::new("Selection Inspector").font(theme::semibold(14.0)).color(t.text));
                ui.add_space(8.0);
                ui.label(RichText::new("Click on any element on the document canvas to inspect and edit:").size(12.0).color(t.text_muted));
                ui.add_space(4.0);
                ui.label(RichText::new("• Text Spans: In-place text replacement & movement\n• Images: Replace, resize, move, or delete\n• Annotations: Edit contents, move, or delete\n• Form Fields: Edit values or toggle checkboxes").size(11.0).color(t.text_faint));
            }

            _ => {
                ui.label(RichText::new("Selection Inspector").font(theme::semibold(14.0)).color(t.text));
                ui.label(RichText::new("Selected element does not expose property inspection.").size(12.0).color(t.text_muted));
            }
        }
    });
}

pub fn render_right_rail(
    ui: &mut Ui,
    left_panel: &mut LeftPanelMode,
    right_panel: &mut RightPanelMode,
) {
    let button_size = 32.0;

    ui.vertical(|ui| {
        ui.add_space(8.0);

        let is_inspector = *right_panel == RightPanelMode::Inspector;
        if icons::button(ui, "pencil", button_size, is_inspector, "Element Inspector").clicked() {
            *right_panel = if is_inspector {
                RightPanelMode::Closed
            } else {
                RightPanelMode::Inspector
            };
        }

        let is_thumbs = *left_panel == LeftPanelMode::Thumbnails;
        if icons::button(ui, "copy", button_size, is_thumbs, "Page thumbnails").clicked() {
            *left_panel = if is_thumbs {
                LeftPanelMode::Closed
            } else {
                LeftPanelMode::Thumbnails
            };
        }

        let is_bm = *right_panel == RightPanelMode::Bookmarks;
        if icons::button(ui, "bookmark", button_size, is_bm, "Bookmarks").clicked() {
            *right_panel = if is_bm {
                RightPanelMode::Closed
            } else {
                RightPanelMode::Bookmarks
            };
        }

        let is_comments = *right_panel == RightPanelMode::Comments;
        if icons::button(
            ui,
            "message-square-text",
            button_size,
            is_comments,
            "Comments",
        )
        .clicked()
        {
            *right_panel = if is_comments {
                RightPanelMode::Closed
            } else {
                RightPanelMode::Comments
            };
        }
    });
}
