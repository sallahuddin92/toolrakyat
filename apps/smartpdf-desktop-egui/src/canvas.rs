//! Central document canvas: pan/zoom viewport, high-DPI page rendering, interactive selection and overlays.
//! Adopts upstream PdfCraft canvas metrics (PT: 96/72 dpi scale, drop-shadow, and layout margins).

use egui::{Color32, CornerRadius, Pos2, Rect, Response, Sense, Stroke, TextureHandle, Ui, Vec2};
use starpdf_app_core::selection::{Rect as CoreRect, SelectionState};

use crate::adapter::EguiEngineAdapter;
use crate::panels::{AcrobatMode, ActiveTool};
use crate::theme::Tokens;

/// Logical pixels per PDF point at 100% (96 dpi, like browsers and upstream PdfCraft).
pub const PT: f32 = 96.0 / 72.0;
#[allow(dead_code)]
pub const GAP: f32 = 14.0;
pub const MARGIN: f32 = 28.0;

pub struct CanvasState {
    pub pan_offset: Vec2,
    pub is_panning: bool,
    pub last_drag_pos: Option<Pos2>,
    pub dragging_selection: bool,
    pub drag_start_page_pt: Option<[f64; 2]>,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            pan_offset: Vec2::ZERO,
            is_panning: false,
            last_drag_pos: None,
            dragging_selection: false,
            drag_start_page_pt: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasAction {
    None,
    OpenRequested,
}

#[allow(clippy::too_many_arguments)]
pub fn render_document_canvas(
    ui: &mut Ui,
    adapter: &mut EguiEngineAdapter,
    state: &mut CanvasState,
    active_tool: &mut ActiveTool,
    active_mode: &mut AcrobatMode,
    page_texture: Option<&TextureHandle>,
    page_dims: (f32, f32), // in points (e.g. 595.0, 842.0)
    zoom_percent: f32,
) -> (Response, CanvasAction) {
    let t = Tokens::get(ui.ctx());
    let available_rect = ui.available_rect_before_wrap();
    let (response, painter) = ui.allocate_painter(available_rect.size(), Sense::click_and_drag());

    // Canvas background (Acrobat neutral pasteboard)
    painter.rect_filled(available_rect, CornerRadius::ZERO, t.pasteboard);

    if adapter.manager.active_session().is_none() {
        let mut action = CanvasAction::None;
        let center = available_rect.center();
        let card_w = 440.0_f32.min(available_rect.width() - 40.0);
        let card_h = 240.0;
        let card_rect = Rect::from_center_size(center, Vec2::new(card_w, card_h));

        painter.rect_filled(card_rect, CornerRadius::same(12), t.card);
        painter.rect_stroke(
            card_rect,
            CornerRadius::same(12),
            Stroke::new(1.0, t.border),
            egui::StrokeKind::Inside,
        );

        let icon_rect = Rect::from_center_size(
            Pos2::new(center.x, card_rect.top() + 45.0),
            Vec2::splat(36.0),
        );
        crate::icons::paint(ui, icon_rect, "file-text", 36.0, t.accent);

        painter.text(
            Pos2::new(center.x, card_rect.top() + 85.0),
            egui::Align2::CENTER_CENTER,
            "No Document Open",
            crate::theme::semibold(16.0),
            t.text,
        );

        painter.text(
            Pos2::new(center.x, card_rect.top() + 112.0),
            egui::Align2::CENTER_CENTER,
            "Open a PDF or drag and drop a file anywhere on this window",
            crate::theme::regular(13.0),
            t.text_muted,
        );

        let btn_w = 160.0;
        let btn_h = 36.0;
        let btn_rect = Rect::from_center_size(
            Pos2::new(center.x, card_rect.top() + 165.0),
            Vec2::new(btn_w, btn_h),
        );
        let btn_resp = ui.interact(btn_rect, ui.id().with("empty_open_btn"), Sense::click());
        let btn_bg = if btn_resp.hovered() {
            t.accent_soft
        } else {
            t.accent
        };
        painter.rect_filled(btn_rect, CornerRadius::same(8), btn_bg);

        let btn_icon_rect = Rect::from_min_size(
            Pos2::new(btn_rect.left() + 20.0, btn_rect.center().y - 8.0),
            Vec2::splat(16.0),
        );
        crate::icons::paint(ui, btn_icon_rect, "plus", 16.0, Color32::WHITE);
        painter.text(
            Pos2::new(btn_rect.left() + 44.0, btn_rect.center().y),
            egui::Align2::LEFT_CENTER,
            "Open Document",
            crate::theme::medium(13.5),
            Color32::WHITE,
        );

        if btn_resp.clicked() {
            action = CanvasAction::OpenRequested;
        }

        return (response, action);
    }

    let scale = (zoom_percent / 100.0) * PT;
    let page_width_px = page_dims.0 * scale;
    let page_height_px = page_dims.1 * scale;

    // Center page on canvas plus user pan offset
    let center_x = available_rect.center().x + state.pan_offset.x;
    let start_y =
        (available_rect.top() + MARGIN + state.pan_offset.y).max(available_rect.top() + 10.0);

    let page_rect = Rect::from_min_size(
        Pos2::new(center_x - page_width_px / 2.0, start_y),
        Vec2::new(page_width_px, page_height_px),
    );

    // Mouse drag handling (Pan vs Move selected)
    if response.drag_started() {
        if *active_tool == ActiveTool::Hand || ui.input(|i| i.key_down(egui::Key::Space)) {
            state.is_panning = true;
            state.last_drag_pos = response.interact_pointer_pos();
        } else if let Some(ptr) = response.interact_pointer_pos() {
            if page_rect.contains(ptr) {
                let page_x = ((ptr.x - page_rect.left()) / scale) as f64;
                let page_y = ((page_rect.bottom() - ptr.y) / scale) as f64;
                state.drag_start_page_pt = Some([page_x, page_y]);
            }
        }
    }

    if response.dragged() {
        if state.is_panning {
            let delta = response.drag_delta();
            state.pan_offset += delta;
        } else if let Some(start_pt) = state.drag_start_page_pt {
            if let Some(ptr) = response.interact_pointer_pos() {
                let current_page_x = ((ptr.x - page_rect.left()) / scale) as f64;
                let current_page_y = ((page_rect.bottom() - ptr.y) / scale) as f64;
                let dx = current_page_x - start_pt[0];
                let dy = current_page_y - start_pt[1];
                if dx.abs() > 2.0 || dy.abs() > 2.0 {
                    state.dragging_selection = true;
                }
            }
        }
    }

    if response.drag_stopped() {
        if state.dragging_selection {
            if let (Some(start_pt), Some(ptr)) =
                (state.drag_start_page_pt, response.interact_pointer_pos())
            {
                let current_page_x = ((ptr.x - page_rect.left()) / scale) as f64;
                let current_page_y = ((page_rect.bottom() - ptr.y) / scale) as f64;
                let dx = current_page_x - start_pt[0];
                let dy = current_page_y - start_pt[1];
                let _ = adapter.move_selected(dx, dy);
            }
        }
        state.is_panning = false;
        state.dragging_selection = false;
        state.drag_start_page_pt = None;
    }

    // Wheel zooming with Cmd/Ctrl
    if ui.rect_contains_pointer(available_rect) {
        let scroll_delta = ui.input(|i| i.smooth_scroll_delta);
        if ui.input(|i| i.modifiers.command || i.modifiers.ctrl) && scroll_delta.y.abs() > 0.0 {
            let factor = if scroll_delta.y > 0.0 { 1.1 } else { 0.9 };
            let new_zoom = (zoom_percent * factor).clamp(25.0, 500.0);
            let _ = adapter.set_zoom(new_zoom);
        } else if scroll_delta.y.abs() > 0.0 || scroll_delta.x.abs() > 0.0 {
            state.pan_offset += scroll_delta;
        }
    }

    // 1. Draw Page Shadow (Upstream soft shadow)
    let shadow_rect = page_rect.translate(Vec2::new(0.0, 4.0));
    painter.rect_filled(shadow_rect, CornerRadius::same(2), t.page_shadow);

    // 2. Draw Rendered Page Image
    if let Some(texture) = page_texture {
        painter.image(
            texture.id(),
            page_rect,
            Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
    } else {
        painter.rect_filled(page_rect, CornerRadius::same(2), Color32::WHITE);
    }

    // Draw thin document border
    painter.rect_stroke(
        page_rect,
        CornerRadius::same(2),
        Stroke::new(1.0, t.border),
        egui::StrokeKind::Outside,
    );

    // 3. Interactive Selection Overlays and Handles
    if let Some(session) = adapter.manager.active_session() {
        match &session.selection {
            SelectionState::Text {
                bounds,
                is_editable,
                ..
            } => {
                let overlay_rect = to_canvas_rect(bounds, page_rect, scale);
                draw_selection_box(ui, overlay_rect, true);

                if *active_mode == AcrobatMode::Edit && *is_editable {
                    let mut text_val = adapter.editing_text_buffer.clone();
                    let edit_resp = ui.put(
                        overlay_rect.expand(2.0),
                        egui::TextEdit::multiline(&mut text_val)
                            .font(egui::TextStyle::Body)
                            .text_color(t.text),
                    );
                    if edit_resp.changed() {
                        adapter.editing_text_buffer = text_val.clone();
                    }

                    // Mini on-canvas action buttons directly below selection
                    let commit_rect = Rect::from_min_size(
                        Pos2::new(overlay_rect.left(), overlay_rect.bottom() + 4.0),
                        Vec2::new(60.0, 22.0),
                    );
                    let cancel_rect = Rect::from_min_size(
                        Pos2::new(overlay_rect.left() + 66.0, overlay_rect.bottom() + 4.0),
                        Vec2::new(60.0, 22.0),
                    );

                    let commit_btn = ui.put(
                        commit_rect,
                        egui::Button::new(
                            egui::RichText::new("Commit")
                                .size(11.0)
                                .color(Color32::WHITE),
                        ),
                    );
                    if commit_btn
                        .on_hover_text("Commit text changes (Enter)")
                        .clicked()
                    {
                        let _ = adapter.commit_text_edit(&text_val);
                    }

                    let cancel_btn = ui.put(
                        cancel_rect,
                        egui::Button::new(egui::RichText::new("Cancel").size(11.0)),
                    );
                    if cancel_btn
                        .on_hover_text("Cancel text changes (Esc)")
                        .clicked()
                    {
                        adapter.cancel_text_edit();
                    }

                    // Keyboard shortcuts for inline editing: Enter (without Shift) commits, Esc cancels
                    if edit_resp.has_focus() {
                        let commit_key =
                            ui.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.shift);
                        let cancel_key = ui.input(|i| i.key_pressed(egui::Key::Escape));
                        if commit_key {
                            let _ = adapter.commit_text_edit(&text_val);
                        } else if cancel_key {
                            adapter.cancel_text_edit();
                        }
                    }
                }
            }
            SelectionState::Annotation { bounds, .. }
            | SelectionState::FormField { bounds, .. }
            | SelectionState::Image { bounds, .. } => {
                let overlay_rect = to_canvas_rect(bounds, page_rect, scale);
                draw_selection_box(ui, overlay_rect, true);
            }
            _ => {}
        }
    }

    // 4. Click Handling (Hit Testing on Page)
    if response.clicked() {
        if let Some(ptr) = response.interact_pointer_pos() {
            if page_rect.contains(ptr) {
                let page_x = ((ptr.x - page_rect.left()) / scale) as f64;
                let page_y = ((page_rect.bottom() - ptr.y) / scale) as f64;

                let hit_opt = adapter.hit_test(page_x, page_y, page_dims.1 as f64);

                // Auto-commit uncommitted draft text changes before switching or clearing selection.
                // If auto-commit fails, refuse to change selection or clear draft so user edits are NEVER lost.
                if let Err(_) = adapter.auto_commit_draft_if_needed() {
                    return (response, CanvasAction::None);
                }

                if let Some(sel) = hit_opt {
                    let is_checkbox = match &sel {
                        SelectionState::FormField { field_type, .. } => field_type == "Checkbox",
                        _ => false,
                    };
                    let _ = adapter.select(sel);
                    if is_checkbox {
                        let _ = adapter.toggle_selected_form_checkbox();
                    }
                } else {
                    match *active_tool {
                        ActiveTool::Highlight => {
                            let _ = adapter.add_annotation(
                                "Highlight",
                                [
                                    page_x,
                                    (page_dims.1 as f64 - page_y - 16.0).max(0.0),
                                    page_x + 120.0,
                                    page_dims.1 as f64 - page_y,
                                ],
                                Some("Highlight".to_string()),
                                Some(vec![1.0, 0.9, 0.2]),
                            );
                        }
                        ActiveTool::StickyNote => {
                            let _ = adapter.add_annotation(
                                "FreeText",
                                [
                                    page_x,
                                    (page_dims.1 as f64 - page_y - 24.0).max(0.0),
                                    page_x + 120.0,
                                    page_dims.1 as f64 - page_y,
                                ],
                                Some("Note".to_string()),
                                Some(vec![1.0, 0.8, 0.0]),
                            );
                        }
                        _ => {
                            let _ = adapter.clear_selection();
                        }
                    }
                }
            }
        }
    }

    (response, CanvasAction::None)
}

fn to_canvas_rect(core_rect: &CoreRect, page_rect: Rect, scale: f32) -> Rect {
    let left = page_rect.left() + (core_rect.x as f32) * scale;
    let top = page_rect.top() + (core_rect.y as f32) * scale;
    let width = (core_rect.width as f32) * scale;
    let height = (core_rect.height as f32) * scale;
    Rect::from_min_size(Pos2::new(left, top), Vec2::new(width, height))
}

fn draw_selection_box(ui: &Ui, rect: Rect, with_handles: bool) {
    let t = Tokens::get(ui.ctx());
    let painter = ui.painter();

    painter.rect_filled(
        rect,
        CornerRadius::same(2),
        t.accent_soft.gamma_multiply(0.4),
    );
    painter.rect_stroke(
        rect,
        CornerRadius::same(2),
        Stroke::new(1.5, t.accent),
        egui::StrokeKind::Outside,
    );

    if with_handles {
        let handle_size = Vec2::splat(6.0);
        let corners = [
            rect.left_top(),
            rect.right_top(),
            rect.left_bottom(),
            rect.right_bottom(),
        ];

        for c in corners {
            let h_rect = Rect::from_center_size(c, handle_size);
            painter.rect_filled(h_rect, CornerRadius::same(1), Color32::WHITE);
            painter.rect_stroke(
                h_rect,
                CornerRadius::same(1),
                Stroke::new(1.5, t.accent),
                egui::StrokeKind::Outside,
            );
        }
    }
}
