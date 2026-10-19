//! Small custom widgets built directly on upstream PdfCraft design tokens.

use egui::{Align2, Color32, CornerRadius, Rect, Response, Sense, Stroke, Ui, Vec2, vec2};
use crate::theme::{self, Tokens};
use crate::icons;

pub fn mode_tab(ui: &mut Ui, label: &str, active: bool) -> Response {
    let t = Tokens::get(ui.ctx());
    let font = if active { theme::semibold(13.0) } else { theme::regular(13.0) };
    let text_w = ui.fonts_mut(|f| f.layout_no_wrap(label.to_owned(), font.clone(), Color32::WHITE).size().x);
    let width = text_w + 24.0;
    let height = 32.0;

    let (rect, resp) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    if ui.is_rect_visible(rect) {
        let _press = theme::Press::track(ui, &resp);
        let bg = if active {
            t.selected
        } else if resp.hovered() {
            t.hover
        } else {
            Color32::TRANSPARENT
        };

        if bg != Color32::TRANSPARENT {
            ui.painter().rect_filled(rect, CornerRadius::same(t.radius), bg);
        }

        let fg = if active { t.accent_text } else { t.text };
        ui.painter().text(rect.center(), Align2::CENTER_CENTER, label, font, fg);

        if active {
            // Blue bottom indicator bar
            let indicator = Rect::from_min_max(
                egui::pos2(rect.left() + 6.0, rect.bottom() - 2.0),
                egui::pos2(rect.right() - 6.0, rect.bottom()),
            );
            ui.painter().rect_filled(indicator, CornerRadius::same(1), t.accent);
        }
    }

    theme::hand(resp)
}

pub fn ghost_button(ui: &mut Ui, icon: &str, label: &str) -> Response {
    ghost_button_sized(ui, icon, label, 30.0)
}

pub fn ghost_button_sized(ui: &mut Ui, icon: &str, label: &str, height: f32) -> Response {
    let t = Tokens::get(ui.ctx());
    let font = theme::medium(13.0);
    let text_w = if label.is_empty() {
        0.0
    } else {
        ui.fonts_mut(|f| f.layout_no_wrap(label.to_owned(), font.clone(), Color32::WHITE).size().x) + 8.0
    };
    let width = 24.0 + text_w + 12.0;

    let (rect, resp) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    if ui.is_rect_visible(rect) {
        let press = theme::Press::track(ui, &resp);
        press.wash(ui, rect, CornerRadius::same(t.radius), false);

        let icon_rect = Rect::from_min_size(
            egui::pos2(rect.left() + 6.0, rect.center().y - 8.0),
            Vec2::splat(16.0),
        );
        icons::paint(ui, icon_rect, icon, 16.0, t.icon);

        if !label.is_empty() {
            ui.painter().text(
                egui::pos2(rect.left() + 26.0, rect.center().y),
                Align2::LEFT_CENTER,
                label,
                font,
                t.text,
            );
        }
    }

    theme::hand(resp)
}

pub fn search_box(ui: &mut Ui, placeholder: &str, width: f32) -> Response {
    let t = Tokens::get(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(vec2(width, 30.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let is_hovered = resp.hovered();
        let bg = if is_hovered { t.hover } else { t.card };
        ui.painter().rect_filled(rect, CornerRadius::same(6), bg);
        ui.painter().rect_stroke(rect, CornerRadius::same(6), Stroke::new(1.0, t.border), egui::StrokeKind::Inside);

        let icon_rect = Rect::from_min_size(egui::pos2(rect.left() + 8.0, rect.center().y - 7.0), Vec2::splat(14.0));
        icons::paint(ui, icon_rect, "search", 14.0, t.text_faint);

        ui.painter().text(
            egui::pos2(rect.left() + 28.0, rect.center().y),
            Align2::LEFT_CENTER,
            placeholder,
            theme::regular(12.0),
            t.text_faint,
        );

        // Shortcut pill (⌘K)
        let pill_rect = Rect::from_center_size(
            egui::pos2(rect.right() - 20.0, rect.center().y),
            vec2(24.0, 16.0),
        );
        ui.painter().rect_filled(pill_rect, CornerRadius::same(3), t.divider);
        ui.painter().text(
            pill_rect.center(),
            Align2::CENTER_CENTER,
            "⌘K",
            theme::medium(10.0),
            t.text_muted,
        );
    }
    theme::hand(resp)
}
