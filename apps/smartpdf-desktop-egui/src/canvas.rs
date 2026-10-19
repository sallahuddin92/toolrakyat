//! Central document canvas: pan/zoom viewport, high-DPI page rendering, interactive selection and overlays.

use egui::{
    Color32, CornerRadius, Pos2, Rect, Response, Sense, Stroke, TextureHandle, Ui, Vec2,
};
use starpdf::execution::StarNodeId;
use starpdf::syntax::object::ObjectRef;
use starpdf_app_core::command::AppCommand;
use starpdf_app_core::selection::{FontInfo, Rect as CoreRect, SelectionState};
use starpdf_app_core::session::SessionManager;

use crate::panels::{AcrobatMode, ActiveTool};
use crate::theme::DesignTokens;

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

pub fn render_document_canvas(
    ui: &mut Ui,
    tokens: &DesignTokens,
    manager: &mut SessionManager,
    state: &mut CanvasState,
    active_tool: &mut ActiveTool,
    active_mode: &mut AcrobatMode,
    page_texture: Option<&TextureHandle>,
    page_dims: (f32, f32), // in points (e.g. 612.0, 792.0)
    zoom_percent: f32,
    editing_text_buffer: &mut String,
) -> Response {
    let available_rect = ui.available_rect_before_wrap();
    let (response, painter) = ui.allocate_painter(available_rect.size(), Sense::click_and_drag());

    // Canvas background
    painter.rect_filled(available_rect, CornerRadius::ZERO, tokens.canvas_bg);

    let scale = zoom_percent / 100.0;
    let page_width_px = page_dims.0 * scale;
    let page_height_px = page_dims.1 * scale;

    // Center page on canvas plus user pan offset
    let center_x = available_rect.center().x + state.pan_offset.x;
    let start_y = (available_rect.top() + 30.0 + state.pan_offset.y).max(available_rect.top() + 10.0);

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

    // 1. Draw Page Shadow
    let shadow_rect = page_rect.translate(Vec2::new(0.0, 6.0));
    painter.rect_filled(shadow_rect, CornerRadius::same(2), tokens.page_shadow);

    // 2. Draw Rendered Page Image
    if let Some(texture) = page_texture {
        painter.image(
            texture.id(),
            page_rect,
            Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
    } else {
        // Fallback white paper sheet while rendering
        painter.rect_filled(page_rect, CornerRadius::same(2), Color32::WHITE);
    }

    // Draw thin document border
    painter.rect_stroke(page_rect, CornerRadius::same(2), Stroke::new(1.0, tokens.border), egui::StrokeKind::Outside);

    // 3. Interactive Selection Overlays and Handles
    if let Some(session) = manager.active_session() {
        match &session.selection {
            SelectionState::Text { bounds, .. } => {
                let overlay_rect = to_canvas_rect(bounds, page_rect, scale);
                draw_selection_box(ui, tokens, overlay_rect, true);

                // In-place text editing overlay if in edit mode
                if *active_mode == AcrobatMode::Edit {
                    let mut text_val = editing_text_buffer.clone();
                    let edit_resp = ui.put(
                        overlay_rect.expand(2.0),
                        egui::TextEdit::multiline(&mut text_val)
                            .font(egui::TextStyle::Body)
                            .text_color(tokens.text_primary),
                    );
                    if edit_resp.changed() {
                        *editing_text_buffer = text_val;
                    }
                }
            }
            SelectionState::Annotation { bounds, .. } => {
                let overlay_rect = to_canvas_rect(bounds, page_rect, scale);
                draw_selection_box(ui, tokens, overlay_rect, true);
            }
            SelectionState::FormField { bounds, .. } => {
                let overlay_rect = to_canvas_rect(bounds, page_rect, scale);
                draw_selection_box(ui, tokens, overlay_rect, true);
            }
            SelectionState::Image { bounds, .. } => {
                let overlay_rect = to_canvas_rect(bounds, page_rect, scale);
                draw_selection_box(ui, tokens, overlay_rect, true);
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

    response
}

fn to_canvas_rect(core_rect: &CoreRect, page_rect: Rect, scale: f32) -> Rect {
    let left = page_rect.left() + (core_rect.x as f32) * scale;
    let top = page_rect.top() + (core_rect.y as f32) * scale;
    let width = (core_rect.width as f32) * scale;
    let height = (core_rect.height as f32) * scale;
    Rect::from_min_size(Pos2::new(left, top), Vec2::new(width, height))
}

fn draw_selection_box(ui: &Ui, tokens: &DesignTokens, rect: Rect, with_handles: bool) {
    let painter = ui.painter();

    // Fill wash
    painter.rect_filled(rect, CornerRadius::same(2), tokens.selection_fill);

    // Outline stroke
    painter.rect_stroke(
        rect,
        CornerRadius::same(2),
        Stroke::new(1.5, tokens.selection_stroke),
        egui::StrokeKind::Outside,
    );

    if with_handles {
        let handle_size = Vec2::splat(6.0);
        let handle_color = Color32::WHITE;
        let handle_stroke = Stroke::new(1.5, tokens.selection_stroke);

        let corners = [
            rect.left_top(),
            rect.right_top(),
            rect.left_bottom(),
            rect.right_bottom(),
        ];

        for c in corners {
            let h_rect = Rect::from_center_size(c, handle_size);
            painter.rect_filled(h_rect, CornerRadius::same(1), handle_color);
            painter.rect_stroke(h_rect, CornerRadius::same(1), handle_stroke, egui::StrokeKind::Outside);
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
    let current_page = manager.active_session().map(|s| s.current_page).unwrap_or(0);

    match *active_tool {
        ActiveTool::Select | ActiveTool::EditText => {
            // Hit test forms first
            if let Some(session) = manager.active_session() {
                if let Ok(mut doc) = session.pdf_document() {
                    if let Ok(fields) = doc.form_fields() {
                        for f in &fields {
                            for w in &f.widgets {
                                if x >= w.rect[0] && x <= w.rect[2] && y >= w.rect[1] && y <= w.rect[3] {
                                    let is_cb = matches!(f.field_type, starpdf::forms::FieldType::Checkbox);
                                    let is_radio = matches!(f.field_type, starpdf::forms::FieldType::RadioButtonGroup);
                                    if is_cb {
                                        let _ = manager.dispatch(AppCommand::SetFormField {
                                            field_name: f.partial_name.clone(),
                                            value: "toggle".into(),
                                        });
                                        return;
                                    } else if is_radio {
                                        let _ = manager.dispatch(AppCommand::SetFormField {
                                            field_name: f.partial_name.clone(),
                                            value: "Choice1".into(),
                                        });
                                        return;
                                    } else {
                                        let canvas_y = (page_height - w.rect[3]).max(0.0);
                                        let bounds = CoreRect::new(
                                            w.rect[0],
                                            canvas_y,
                                            (w.rect[2] - w.rect[0]).abs().max(16.0),
                                            (w.rect[3] - w.rect[1]).abs().max(16.0),
                                        );
                                        let val_str = match &f.value {
                                            starpdf::forms::FieldValue::Text(s) => s.clone(),
                                            starpdf::forms::FieldValue::Boolean(b) => if *b { "true".into() } else { "false".into() },
                                            starpdf::forms::FieldValue::Name(s) => s.clone(),
                                            starpdf::forms::FieldValue::Choice(v) => v.first().cloned().unwrap_or_default(),
                                            starpdf::forms::FieldValue::None => "".into(),
                                        };
                                        let sel = SelectionState::FormField {
                                            field_ref: format!("{} 0 R", f.object_ref.number),
                                            field_name: f.partial_name.clone(),
                                            field_type: "Text".into(),
                                            value: val_str.clone(),
                                            bounds,
                                        };
                                        let _ = manager.dispatch(AppCommand::SetSelection { selection: sel });
                                        *editing_text_buffer = val_str;
                                        return;
                                    }
                                }
                            }
                        }
                    }

                    // Hit test annotations
                    if let Ok(annots) = doc.page_annotations(session.current_page as usize) {
                        for a in &annots {
                            if x >= a.rect[0] && x <= a.rect[2] && y >= a.rect[1] && y <= a.rect[3] {
                                let canvas_y = (page_height - a.rect[3]).max(0.0);
                                let bounds = CoreRect::new(
                                    a.rect[0],
                                    canvas_y,
                                    (a.rect[2] - a.rect[0]).abs().max(16.0),
                                    (a.rect[3] - a.rect[1]).abs().max(16.0),
                                );
                                let sel = SelectionState::Annotation {
                                    annot_ref: format!("{} 0 R", a.object_ref.number),
                                    subtype: format!("{:?}", a.subtype),
                                    contents: a.contents.clone(),
                                    bounds,
                                };
                                let _ = manager.dispatch(AppCommand::SetSelection { selection: sel });
                                return;
                            }
                        }
                    }

                    // Hit test images
                    if let Ok(images) = doc.enumerate_images(session.current_page as usize) {
                        for img in &images {
                            if x >= img.rect[0] && x <= img.rect[2] && y >= img.rect[1] && y <= img.rect[3] {
                                let canvas_y = (page_height - img.rect[3]).max(0.0);
                                let bounds = CoreRect::new(
                                    img.rect[0],
                                    canvas_y,
                                    (img.rect[2] - img.rect[0]).abs().max(16.0),
                                    (img.rect[3] - img.rect[1]).abs().max(16.0),
                                );
                                let star_node_id = StarNodeId::new("doc", img.object_ref, &img.image_id)
                                    .unwrap_or_else(|_| StarNodeId::new("doc", ObjectRef::new(1, 0), "img").unwrap());
                                let sel = SelectionState::Image {
                                    star_node_id,
                                    render_node_id: None,
                                    image_ref: format!("{} 0 R", img.object_ref.number),
                                    bounds,
                                    is_shared: false,
                                };
                                let _ = manager.dispatch(AppCommand::SetSelection { selection: sel });
                                return;
                            }
                        }
                    }
                }

                // Hit test text spans
                if let Ok(spans) = session.extract_page_text_spans(session.current_page) {
                    for span in spans {
                        let span_top = span.y;
                        let span_bottom = span.y + span.height;
                        if x >= span.x && x <= span.x + span.width && y >= span_top && y <= span_bottom {
                            let canvas_y = (page_height - span.y - span.height).max(0.0);
                            let bounds = CoreRect::new(span.x, canvas_y, span.width.max(16.0), span.height.max(12.0));
                            let star_node_id = StarNodeId::new("doc", ObjectRef::new(1, 0), &span.span_id)
                                .unwrap_or_else(|_| StarNodeId::new("doc", ObjectRef::new(1, 0), "span").unwrap());
                            let sel = SelectionState::Text {
                                star_node_id,
                                render_node_id: None,
                                text: span.text.clone(),
                                bounds,
                                font_info: Some(FontInfo::new(span.font_name.clone(), span.font_size)),
                                is_editable: true,
                            };
                            let _ = manager.dispatch(AppCommand::SetSelection { selection: sel });
                            *editing_text_buffer = span.text;
                            return;
                        }
                    }
                }
            }

            // Clicked empty space -> clear selection
            let _ = manager.dispatch(AppCommand::ClearSelection);
        }
        ActiveTool::Highlight => {
            let _ = manager.dispatch(AppCommand::AddAnnotation {
                page_index: current_page,
                subtype: "Highlight".into(),
                rect: [x, y, x + 120.0, y + 16.0],
                contents: Some("Highlight markup".into()),
                color: Some(vec![1.0, 0.9, 0.2]),
            });
        }
        ActiveTool::StickyNote => {
            let _ = manager.dispatch(AppCommand::AddAnnotation {
                page_index: current_page,
                subtype: "Text".into(),
                rect: [x, y, x + 24.0, y + 24.0],
                contents: Some("Sticky note comment".into()),
                color: Some(vec![0.2, 0.6, 1.0]),
            });
        }
        ActiveTool::Sign => {
            let _ = manager.dispatch(AppCommand::AddAnnotation {
                page_index: current_page,
                subtype: "FreeText".into(),
                rect: [x, y, x + 140.0, y + 30.0],
                contents: Some("Digitally Signed by SmartPDF".into()),
                color: Some(vec![0.1, 0.4, 0.8]),
            });
        }
        _ => {}
    }
}
