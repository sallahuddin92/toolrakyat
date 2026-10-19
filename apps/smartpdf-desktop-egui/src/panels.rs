//! Side panels: Left Rail (Thumbnails, Bookmarks, All Tools) and Right Inspector.
#![allow(dead_code)]

use egui::{
    Align, Color32, CornerRadius, Layout, RichText, ScrollArea, Stroke, Ui, Vec2,
};
use starpdf_app_core::command::AppCommand;
use starpdf_app_core::selection::SelectionState;
use starpdf_app_core::session::SessionManager;

use crate::icons;
use crate::theme::DesignTokens;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeftPanelMode {
    Closed,
    Thumbnails,
    Bookmarks,
    AllTools,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
pub enum AcrobatMode {
    AllTools,
    Edit,
    Comment,
    Forms,
    Sign,
    Organize,
}

pub fn render_left_rail(
    ui: &mut Ui,
    tokens: &DesignTokens,
    panel_mode: &mut LeftPanelMode,
) {
    ui.vertical(|ui| {
        ui.add_space(8.0);

        // Thumbnails (Pages)
        let is_thumbs = *panel_mode == LeftPanelMode::Thumbnails;
        if icons::icon_button(ui, icons::LAYOUT_GRID, tokens, 18.0, is_thumbs, "Page Thumbnails").clicked() {
            *panel_mode = if is_thumbs { LeftPanelMode::Closed } else { LeftPanelMode::Thumbnails };
        }

        ui.add_space(4.0);

        // Bookmarks
        let is_bookmarks = *panel_mode == LeftPanelMode::Bookmarks;
        if icons::icon_button(ui, icons::BOOKMARK, tokens, 18.0, is_bookmarks, "Bookmarks").clicked() {
            *panel_mode = if is_bookmarks { LeftPanelMode::Closed } else { LeftPanelMode::Bookmarks };
        }

        ui.add_space(4.0);

        // All Tools
        let is_tools = *panel_mode == LeftPanelMode::AllTools;
        if icons::icon_button(ui, icons::PANEL_LEFT, tokens, 18.0, is_tools, "All Tools Catalog").clicked() {
            *panel_mode = if is_tools { LeftPanelMode::Closed } else { LeftPanelMode::AllTools };
        }

        ui.add_space(4.0);

        // Attachments
        if icons::icon_button(ui, icons::PAPERCLIP, tokens, 18.0, false, "Attachments").clicked() {
            // Future panel
        }
    });
}

pub fn render_thumbnails_panel(
    ui: &mut Ui,
    tokens: &DesignTokens,
    manager: &mut SessionManager,
    thumbnail_textures: &std::collections::HashMap<u32, egui::TextureHandle>,
) {
    ui.vertical(|ui| {
        // Panel Header
        ui.horizontal(|ui| {
            ui.label(RichText::new("Page Thumbnails").strong().size(13.0).color(tokens.text_primary));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if icons::icon_button(ui, icons::COPY, tokens, 14.0, false, "Duplicate Page").clicked() {
                    if let Some(session) = manager.active_session() {
                        let cur = session.current_page;
                        let _ = manager.dispatch(AppCommand::DuplicatePage { page_index: cur });
                    }
                }
            });
        });

        ui.separator();

        let (total_pages, current_page) = if let Some(session) = manager.active_session() {
            (session.total_pages, session.current_page)
        } else {
            (0, 0)
        };

        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.add_space(4.0);
            for p in 0..total_pages {
                let is_current = p == current_page;
                let bg_color = if is_current { tokens.selected_bg } else { Color32::TRANSPARENT };
                let border_stroke = if is_current {
                    Stroke::new(2.0, tokens.accent)
                } else {
                    Stroke::new(1.0, tokens.border)
                };

                let card_resp = ui.vertical(|ui| {
                    let item_rect = ui.available_rect_before_wrap();
                    let thumb_height = 130.0;
                    let (rect, resp) = ui.allocate_exact_size(Vec2::new(item_rect.width(), thumb_height), egui::Sense::click());

                    if ui.is_rect_visible(rect) {
                        // Card background
                        ui.painter().rect_filled(rect, CornerRadius::same(4), bg_color);
                        ui.painter().rect_stroke(rect, CornerRadius::same(4), border_stroke, egui::StrokeKind::Inside);

                        // Thumbnail image if rendered
                        if let Some(texture) = thumbnail_textures.get(&p) {
                            let img_size = texture.size_vec2();
                            let aspect = img_size.x / img_size.y.max(1.0);
                            let target_h = thumb_height - 28.0;
                            let target_w = (target_h * aspect).min(rect.width() - 16.0);
                            let img_rect = egui::Rect::from_center_size(
                                egui::pos2(rect.center().x, rect.top() + 8.0 + target_h / 2.0),
                                Vec2::new(target_w, target_h),
                            );
                            ui.painter().image(
                                texture.id(),
                                img_rect,
                                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                Color32::WHITE,
                            );
                        } else {
                            // Fallback page placeholder
                            let page_box = egui::Rect::from_center_size(
                                egui::pos2(rect.center().x, rect.top() + 10.0 + (thumb_height - 28.0) / 2.0),
                                Vec2::new(75.0, 95.0),
                            );
                            ui.painter().rect_filled(page_box, CornerRadius::same(2), Color32::WHITE);
                            ui.painter().rect_stroke(page_box, CornerRadius::same(2), Stroke::new(1.0, tokens.border), egui::StrokeKind::Inside);
                        }

                        // Page label
                        let label_rect = egui::Rect::from_min_max(
                            egui::pos2(rect.left(), rect.bottom() - 20.0),
                            egui::pos2(rect.right(), rect.bottom()),
                        );
                        let text_color = if is_current { tokens.accent_text } else { tokens.text_secondary };
                        ui.painter().text(
                            label_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            format!("Page {}", p + 1),
                            egui::FontId::proportional(11.0),
                            text_color,
                        );
                    }

                    resp
                }).inner;

                if card_resp.clicked() {
                    let _ = manager.dispatch(AppCommand::SetPage { page_index: p });
                }

                ui.add_space(8.0);
            }
        });
    });
}

pub fn render_all_tools_catalog(
    ui: &mut Ui,
    tokens: &DesignTokens,
    active_mode: &mut AcrobatMode,
    active_tool: &mut ActiveTool,
    panel_mode: &mut LeftPanelMode,
) {
    ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(4.0);
        ui.label(RichText::new("All Tools").strong().size(15.0).color(tokens.text_primary));
        ui.label(RichText::new("Choose an Acrobat Pro toolset:").size(11.0).color(tokens.text_secondary));
        ui.add_space(10.0);

        // Category 1: Edit & Review
        ui.label(RichText::new("EDIT & REVIEW").size(10.0).strong().color(tokens.text_faint));
        ui.add_space(4.0);

        if catalog_item(ui, tokens, icons::TYPE, "Edit Text & Images", "Modify text, fonts, colors and pictures").clicked() {
            *active_mode = AcrobatMode::Edit;
            *active_tool = ActiveTool::EditText;
            *panel_mode = LeftPanelMode::Closed;
        }

        if catalog_item(ui, tokens, icons::HIGHLIGHTER, "Comment & Markup", "Highlight, underline, sticky notes, freehand drawing").clicked() {
            *active_mode = AcrobatMode::Comment;
            *active_tool = ActiveTool::Highlight;
            *panel_mode = LeftPanelMode::Closed;
        }

        if catalog_item(ui, tokens, icons::LAYOUT_GRID, "Organize Pages", "Reorder, duplicate, rotate, extract and delete pages").clicked() {
            *active_mode = AcrobatMode::Organize;
            *panel_mode = LeftPanelMode::Thumbnails;
        }

        ui.add_space(12.0);

        // Category 2: Forms & Sign
        ui.label(RichText::new("FORMS & SIGN").size(10.0).strong().color(tokens.text_faint));
        ui.add_space(4.0);

        if catalog_item(ui, tokens, icons::CHECK_SQUARE, "Fill & Sign", "Fill checkboxes, radios, input text and apply signatures").clicked() {
            *active_mode = AcrobatMode::Forms;
            *active_tool = ActiveTool::FormField;
            *panel_mode = LeftPanelMode::Closed;
        }

        if catalog_item(ui, tokens, icons::PEN_TOOL, "Add Digital Signature", "Place a cryptographic or visual signature line").clicked() {
            *active_mode = AcrobatMode::Sign;
            *active_tool = ActiveTool::Sign;
            *panel_mode = LeftPanelMode::Closed;
        }

        ui.add_space(12.0);

        // Category 3: Convert & Protect
        ui.label(RichText::new("CONVERT & PROTECT").size(10.0).strong().color(tokens.text_faint));
        ui.add_space(4.0);

        if catalog_item(ui, tokens, icons::SHARE, "Export PDF", "Export high-fidelity images or flattened documents").clicked() {
            // Handled in top bar
        }

        if catalog_item(ui, tokens, icons::SHIELD_CHECK, "Protect Document", "Set permissions and secure PDF").clicked() {
            // Handled in top bar
        }
    });
}

fn catalog_item(ui: &mut Ui, tokens: &DesignTokens, icon: &'static [u8], title: &str, desc: &str) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 44.0), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let is_hovered = resp.hovered();
        let bg = if is_hovered { tokens.hover_bg } else { Color32::TRANSPARENT };
        ui.painter().rect_filled(rect, tokens.radius, bg);

        let icon_rect = egui::Rect::from_min_size(egui::pos2(rect.left() + 8.0, rect.top() + 10.0), Vec2::splat(20.0));
        let uri = format!("bytes://icons/{:p}", icon);
        let image = egui::Image::from_bytes(uri, icon)
            .fit_to_exact_size(Vec2::splat(18.0))
            .tint(if is_hovered { tokens.accent } else { tokens.icon_tint });
        image.paint_at(ui, icon_rect);

        let text_left = rect.left() + 36.0;
        ui.painter().text(
            egui::pos2(text_left, rect.top() + 8.0),
            egui::Align2::LEFT_TOP,
            title,
            egui::FontId::proportional(12.0),
            tokens.text_primary,
        );
        ui.painter().text(
            egui::pos2(text_left, rect.top() + 24.0),
            egui::Align2::LEFT_TOP,
            desc,
            egui::FontId::proportional(10.0),
            tokens.text_secondary,
        );
    }
    resp
}

pub fn render_right_inspector(
    ui: &mut Ui,
    tokens: &DesignTokens,
    manager: &mut SessionManager,
    editing_text_buffer: &mut String,
) {
    ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(4.0);
        let (sel_type, title) = if let Some(session) = manager.active_session() {
            match &session.selection {
                SelectionState::None => ("none", "Document Properties"),
                SelectionState::Text { .. } => ("text", "Edit Text"),
                SelectionState::Annotation { .. } => ("annot", "Annotation Properties"),
                SelectionState::FormField { .. } => ("form", "Form Field Properties"),
                SelectionState::Image { .. } => ("image", "Image Properties"),
                SelectionState::Page { .. } => ("page", "Page Properties"),
                SelectionState::Vector { .. } => ("vector", "Vector Graphic"),
            }
        } else {
            ("none", "Document Properties")
        };

        ui.label(RichText::new(title).strong().size(13.0).color(tokens.text_primary));
        ui.separator();

        match sel_type {
            "text" => {
                ui.label(RichText::new("TEXT FORMATTING").size(10.0).strong().color(tokens.text_faint));
                ui.add_space(4.0);

                let mut target_node_id = None;
                if let Some(session) = manager.active_session() {
                    if let SelectionState::Text { ref star_node_id, ref font_info, .. } = session.selection {
                        let (_, _, occ) = star_node_id.parts();
                        target_node_id = Some(occ.to_string());
                        if let Some(font) = font_info {
                            ui.label(RichText::new(format!("Font: {}", font.name)).size(11.0).color(tokens.text_secondary));
                            ui.label(RichText::new(format!("Size: {:.1} pt", font.size)).size(11.0).color(tokens.text_secondary));
                        }
                    }
                }

                ui.add_space(8.0);
                ui.label(RichText::new("Content:").size(11.0).strong().color(tokens.text_primary));
                ui.text_edit_multiline(editing_text_buffer);

                ui.add_space(8.0);
                if ui.button(RichText::new("Commit Text Edit").color(Color32::WHITE)).clicked() {
                    if let Some(target_node) = target_node_id {
                        let new_text = editing_text_buffer.clone();
                        let _ = manager.dispatch(AppCommand::ReplaceText { target_node, new_text });
                    }
                }
            }
            "annot" => {
                ui.label(RichText::new("MARKUP DETAILS").size(10.0).strong().color(tokens.text_faint));
                ui.add_space(4.0);
                ui.label(RichText::new("Type: Highlight / FreeText Note").size(11.0).color(tokens.text_secondary));
                ui.label(RichText::new("Author: SmartPDF User").size(11.0).color(tokens.text_secondary));

                ui.add_space(10.0);
                if ui.button(RichText::new("Delete Annotation").color(Color32::from_rgb(220, 50, 50))).clicked() {
                    let _ = manager.dispatch(AppCommand::DeleteSelected);
                }
            }
            "form" => {
                ui.label(RichText::new("FORM FIELD").size(10.0).strong().color(tokens.text_faint));
                ui.add_space(4.0);
                let mut field_name_opt = None;
                if let Some(session) = manager.active_session() {
                    if let SelectionState::FormField { ref field_name, .. } = session.selection {
                        ui.label(RichText::new(format!("Name: {}", field_name)).size(11.0).color(tokens.text_primary));
                        field_name_opt = Some(field_name.clone());
                    }
                }
                ui.add_space(8.0);
                ui.label(RichText::new("Value:").size(11.0).strong().color(tokens.text_primary));
                ui.text_edit_singleline(editing_text_buffer);

                ui.add_space(8.0);
                if ui.button(RichText::new("Update Form Field").color(Color32::WHITE)).clicked() {
                    if let Some(field_name) = field_name_opt {
                        let val = editing_text_buffer.clone();
                        let _ = manager.dispatch(AppCommand::SetFormField { field_name, value: val });
                    }
                }
            }
            "image" => {
                ui.label(RichText::new("IMAGE OBJECT").size(10.0).strong().color(tokens.text_faint));
                ui.add_space(8.0);
                let (current_page, image_ref_opt) = if let Some(session) = manager.active_session() {
                    let img_ref = if let SelectionState::Image { ref image_ref, .. } = session.selection {
                        Some(image_ref.clone())
                    } else {
                        None
                    };
                    (session.current_page, img_ref)
                } else {
                    (0, None)
                };

                if ui.button("Replace Image...").clicked() {
                    if let Some(image_ref) = image_ref_opt {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Image", &["png", "jpg", "jpeg"])
                            .pick_file()
                        {
                            if let Ok(bytes) = std::fs::read(&path) {
                                let _ = manager.dispatch(AppCommand::ReplaceImage {
                                    page_index: current_page,
                                    image_ref,
                                    image_bytes: bytes,
                                });
                            }
                        }
                    }
                }
                ui.add_space(4.0);
                if ui.button(RichText::new("Delete Image").color(Color32::from_rgb(220, 50, 50))).clicked() {
                    let _ = manager.dispatch(AppCommand::DeleteSelected);
                }
            }
            _ => {
                // Document Info
                ui.label(RichText::new("DOCUMENT OVERVIEW").size(10.0).strong().color(tokens.text_faint));
                ui.add_space(6.0);
                if let Some(session) = manager.active_session() {
                    ui.label(RichText::new(format!("Title: {}", session.title)).size(11.0).color(tokens.text_primary));
                    ui.label(RichText::new(format!("Total Pages: {}", session.total_pages)).size(11.0).color(tokens.text_secondary));
                    ui.label(RichText::new(format!("File Size: {:.1} KB", session.bytes.len() as f64 / 1024.0)).size(11.0).color(tokens.text_secondary));
                    ui.label(RichText::new(format!("Revision: #{}", session.revision)).size(11.0).color(tokens.text_secondary));
                    ui.label(RichText::new("PDF Standard: ISO 32000-1").size(11.0).color(tokens.text_secondary));
                    ui.label(RichText::new("Engine: StarPDF Rust Engine").size(11.0).color(tokens.accent_text));
                } else {
                    ui.label(RichText::new("No document loaded.").size(11.0).color(tokens.text_secondary));
                }
            }
        }
    });
}
