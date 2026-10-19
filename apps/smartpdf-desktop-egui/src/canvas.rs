//! Central document canvas: pan/zoom viewport, high-DPI page rendering, interactive selection and overlays.
//! Adopts upstream PdfCraft canvas metrics (PT: 96/72 dpi scale, drop-shadow, and layout margins).

use egui::{
    Color32, CornerRadius, Pos2, Rect, Response, Sense, Stroke, TextureHandle, Ui, Vec2,
};
use starpdf_app_core::command::AppCommand;
use starpdf_app_core::selection::{Rect as CoreRect, SelectionState};
use starpdf_app_core::session::SessionManager;

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

pub fn render_document_canvas(
    ui: &mut Ui,
    manager: &mut SessionManager,
    state: &mut CanvasState,
    active_tool: &mut ActiveTool,
    active_mode: &mut AcrobatMode,
    page_texture: Option<&TextureHandle>,
    page_dims: (f32, f32), // in points (e.g. 595.0, 842.0)
    zoom_percent: f32,
    editing_text_buffer: &mut String,
) -> (Response, CanvasAction) {
    let t = Tokens::get(ui.ctx());
    let available_rect = ui.available_rect_before_wrap();
    let (response, painter) = ui.allocate_painter(available_rect.size(), Sense::click_and_drag());

    // Canvas background (Acrobat neutral pasteboard)
    painter.rect_filled(available_rect, CornerRadius::ZERO, t.pasteboard);

    if manager.active_session().is_none() {
        let mut action = CanvasAction::None;
        let center = available_rect.center();
        let card_w = 440.0_f32.min(available_rect.width() - 40.0);
        let card_h = 240.0;
        let card_rect = Rect::from_center_size(center, Vec2::new(card_w, card_h));

        painter.rect_filled(card_rect, CornerRadius::same(12), t.card);
        painter.rect_stroke(card_rect, CornerRadius::same(12), Stroke::new(1.0, t.border), egui::StrokeKind::Inside);

        let icon_rect = Rect::from_center_size(Pos2::new(center.x, card_rect.top() + 45.0), Vec2::splat(36.0));
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
        let btn_rect = Rect::from_center_size(Pos2::new(center.x, card_rect.top() + 165.0), Vec2::new(btn_w, btn_h));
        let btn_resp = ui.interact(btn_rect, ui.id().with("empty_open_btn"), Sense::click());
        let btn_bg = if btn_resp.hovered() { t.accent_soft } else { t.accent };
        painter.rect_filled(btn_rect, CornerRadius::same(8), btn_bg);

        let btn_icon_rect = Rect::from_min_size(Pos2::new(btn_rect.left() + 20.0, btn_rect.center().y - 8.0), Vec2::splat(16.0));
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
    let start_y = (available_rect.top() + MARGIN + state.pan_offset.y).max(available_rect.top() + 10.0);

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
            if let (Some(start_pt), Some(ptr)) = (state.drag_start_page_pt, response.interact_pointer_pos()) {
                let current_page_x = ((ptr.x - page_rect.left()) / scale) as f64;
                let current_page_y = ((page_rect.bottom() - ptr.y) / scale) as f64;
                let dx = current_page_x - start_pt[0];
                let dy = current_page_y - start_pt[1];
                let _ = manager.dispatch(AppCommand::MoveElement { dx, dy });
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
            let _ = manager.dispatch(AppCommand::SetZoom { percent: new_zoom });
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
    painter.rect_stroke(page_rect, CornerRadius::same(2), Stroke::new(1.0, t.border), egui::StrokeKind::Outside);

    // 3. Interactive Selection Overlays and Handles
    if let Some(session) = manager.active_session() {
        match &session.selection {
            SelectionState::Text { bounds, .. } => {
                let overlay_rect = to_canvas_rect(bounds, page_rect, scale);
                draw_selection_box(ui, overlay_rect, true);

                if *active_mode == AcrobatMode::Edit {
                    let mut text_val = editing_text_buffer.clone();
                    let edit_resp = ui.put(
                        overlay_rect.expand(2.0),
                        egui::TextEdit::multiline(&mut text_val)
                            .font(egui::TextStyle::Body)
                            .text_color(t.text),
                    );
                    if edit_resp.changed() {
                        *editing_text_buffer = text_val;
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

                handle_canvas_click(
                    manager,
                    active_tool,
                    page_x,
                    page_y,
                    page_dims.1 as f64,
                    editing_text_buffer,
                );
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

    painter.rect_filled(rect, CornerRadius::same(2), t.accent_soft.gamma_multiply(0.4));
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
            painter.rect_stroke(h_rect, CornerRadius::same(1), Stroke::new(1.5, t.accent), egui::StrokeKind::Outside);
        }
    }
}

fn handle_canvas_click(
    manager: &mut SessionManager,
    active_tool: &mut ActiveTool,
    x: f64,
    y: f64,
    page_height: f64,
    editing_text_buffer: &mut String,
) {
    let session = match manager.active_session() {
        Some(s) => s,
        None => return,
    };

    let target_page = session.current_page;

    // Check hit elements via text spans
    if let Ok(spans) = session.extract_page_text_spans(target_page) {
        if let Some(span) = spans.iter().find(|s| {
            x >= s.x && x <= s.x + s.width &&
            y >= s.y && y <= s.y + s.height
        }) {
            *editing_text_buffer = span.text.clone();
            if let Ok(node_id) = starpdf::execution::StarNodeId::new(
                "doc",
                starpdf::syntax::object::ObjectRef::new(target_page as u64 + 1, 0),
                &format!("span-{}", span.text),
            ) {
                let _ = manager.dispatch(AppCommand::SetSelection {
                    selection: SelectionState::Text {
                        star_node_id: node_id,
                        render_node_id: None,
                        text: span.text.clone(),
                        bounds: CoreRect::new(span.x, span.y, span.width, span.height),
                        font_info: None,
                        is_editable: true,
                    },
                });
            }
            return;
        }
    }

    match *active_tool {
        ActiveTool::Highlight => {
            let _ = manager.dispatch(AppCommand::AddAnnotation {
                page_index: target_page,
                subtype: "Highlight".to_string(),
                rect: [x, page_height - y - 16.0, x + 120.0, page_height - y],
                contents: Some("Highlight".to_string()),
                color: Some(vec![1.0, 0.9, 0.2]),
            });
        }
        ActiveTool::StickyNote => {
            let _ = manager.dispatch(AppCommand::AddAnnotation {
                page_index: target_page,
                subtype: "Text".to_string(),
                rect: [x, page_height - y - 24.0, x + 24.0, page_height - y],
                contents: Some("Note".to_string()),
                color: Some(vec![1.0, 0.8, 0.0]),
            });
        }
        _ => {
            let _ = manager.dispatch(AppCommand::ClearSelection);
            editing_text_buffer.clear();
        }
    }
}
