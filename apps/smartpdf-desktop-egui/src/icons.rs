//! Icon registry and rendering helpers using embedded Lucide SVG assets.
#![allow(dead_code)]

use egui::{Color32, Image, Rect, Response, Sense, Stroke, Ui, Vec2};
use crate::theme::DesignTokens;

macro_rules! define_icons {
    ($(($name:ident, $file:literal)),* $(,)?) => {
        $(
            pub static $name: &[u8] = include_bytes!(concat!("../assets/icons/", $file));
        )*

        pub fn get_icon_bytes(name: &str) -> Option<&'static [u8]> {
            match name {
                $(
                    $file | stringify!($name) => Some($name),
                )*
                _ => None,
            }
        }
    };
}

define_icons! {
    (MOUSE_POINTER, "mouse-pointer-2.svg"),
    (HAND, "hand.svg"),
    (TYPE, "type.svg"),
    (HIGHLIGHTER, "highlighter.svg"),
    (STICKY_NOTE, "sticky-note.svg"),
    (PEN_TOOL, "pen-tool.svg"),
    (LAYOUT_GRID, "layout-grid.svg"),
    (PANEL_LEFT, "panel-left.svg"),
    (PANEL_RIGHT, "panel-right.svg"),
    (BOOKMARK, "bookmark.svg"),
    (LAYERS, "layers.svg"),
    (PAPERCLIP, "paperclip.svg"),
    (SAVE, "save.svg"),
    (SHARE, "share-2.svg"),
    (FOLDER_OPEN, "folder-open.svg"),
    (PRINTER, "printer.svg"),
    (UNDO, "undo-2.svg"),
    (REDO, "redo-2.svg"),
    (ZOOM_IN, "zoom-in.svg"),
    (ZOOM_OUT, "zoom-out.svg"),
    (MAXIMIZE, "maximize-2.svg"),
    (SCAN, "scan.svg"),
    (CHEVRON_LEFT, "chevron-left.svg"),
    (CHEVRON_RIGHT, "chevron-right.svg"),
    (CHEVRON_DOWN, "chevron-down.svg"),
    (CHEVRON_UP, "chevron-up.svg"),
    (PLUS, "plus.svg"),
    (MINUS, "minus.svg"),
    (X, "x.svg"),
    (SUN, "sun.svg"),
    (MOON, "moon.svg"),
    (CPU, "gauge.svg"),
    (SETTINGS, "settings.svg"),
    (CHECK_SQUARE, "check-circle-2.svg"),
    (CIRCLE_DOT, "circle-dot.svg"),
    (IMAGE, "image.svg"),
    (COPY, "copy.svg"),
    (TRASH, "trash-2.svg"),
    (ROTATE_CW, "rotate-cw.svg"),
    (SHIELD_CHECK, "shield-check.svg"),
    (PENCIL, "pencil.svg"),
    (SEARCH, "search.svg"),
    (SQUARE, "square.svg"),
    (CIRCLE, "circle.svg"),
}

pub fn render_icon(ui: &mut Ui, bytes: &'static [u8], size: Vec2, tint: Color32) {
    let uri = format!("bytes://icons/{:p}", bytes);
    let image = Image::from_bytes(uri, bytes)
        .fit_to_exact_size(size)
        .tint(tint);
    ui.add(image);
}

pub fn icon_button(
    ui: &mut Ui,
    bytes: &'static [u8],
    tokens: &DesignTokens,
    size: f32,
    selected: bool,
    tooltip: &str,
) -> Response {
    let button_size = Vec2::splat(size + 8.0);
    let (rect, resp) = ui.allocate_exact_size(button_size, Sense::click());

    if ui.is_rect_visible(rect) {
        let is_hovered = resp.hovered();
        let is_active = resp.is_pointer_button_down_on();

        let (bg_fill, icon_tint) = if selected {
            (tokens.selected_bg, tokens.accent)
        } else if is_active {
            (tokens.pressed_bg, tokens.text_primary)
        } else if is_hovered {
            (tokens.hover_bg, tokens.text_primary)
        } else {
            (Color32::TRANSPARENT, tokens.icon_tint)
        };

        if bg_fill != Color32::TRANSPARENT {
            ui.painter().rect_filled(rect, tokens.radius, bg_fill);
        }

        if selected {
            ui.painter().rect_stroke(
                rect,
                tokens.radius,
                Stroke::new(1.0, tokens.accent.gamma_multiply(0.5)),
                egui::StrokeKind::Inside,
            );
        }

        let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(size));
        let uri = format!("bytes://icons/{:p}", bytes);
        let image = Image::from_bytes(uri, bytes)
            .fit_to_exact_size(Vec2::splat(size))
            .tint(icon_tint);
        image.paint_at(ui, icon_rect);
    }

    if !tooltip.is_empty() {
        resp.on_hover_text(tooltip)
    } else {
        resp
    }
}
