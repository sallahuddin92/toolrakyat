//! Vector icons directly embedded from upstream Lucide SVGs.
//! Fully compliant with upstream icon catalog and tinting mechanics.

use egui::{Color32, Image, Rect, Response, Sense, Stroke, Ui, Vec2};
use std::collections::HashMap;
use std::sync::OnceLock;

use crate::icon_data::ICONS;
use crate::theme::{self, Tokens};

struct IconCatalog {
    map: HashMap<&'static str, &'static [u8]>,
}

impl IconCatalog {
    fn new() -> Self {
        let mut map = HashMap::with_capacity(ICONS.len());
        for (name, bytes) in ICONS {
            map.insert(*name, *bytes);
        }
        Self { map }
    }

    fn get(&self, name: &str) -> Option<&'static [u8]> {
        self.map.get(name).copied()
    }
}

fn catalog() -> &'static IconCatalog {
    static CATALOG: OnceLock<IconCatalog> = OnceLock::new();
    CATALOG.get_or_init(IconCatalog::new)
}

#[allow(dead_code)]
pub fn exists(name: &str) -> bool {
    catalog().get(name).is_some()
}

pub fn get_icon_bytes(name: &str) -> &'static [u8] {
    catalog()
        .get(name)
        .or_else(|| catalog().get("square"))
        .unwrap_or(&[])
}

#[allow(dead_code)]
pub fn image(name: &str, size: f32, tint: Color32) -> Image<'static> {
    let bytes = get_icon_bytes(name);
    let uri = format!("bytes://icons/{name}.svg");
    Image::from_bytes(uri, bytes)
        .fit_to_exact_size(Vec2::splat(size))
        .tint(tint)
}

pub fn paint(ui: &Ui, rect: Rect, name: &str, size: f32, tint: Color32) {
    let bytes = get_icon_bytes(name);
    let uri = format!("bytes://icons/{name}.svg");
    Image::from_bytes(uri, bytes)
        .fit_to_exact_size(Vec2::splat(size))
        .tint(tint)
        .paint_at(ui, Rect::from_center_size(rect.center(), Vec2::splat(size)));
}

pub fn button(ui: &mut Ui, name: &str, box_size: f32, selected: bool, tooltip: &str) -> Response {
    let t = Tokens::get(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(box_size), Sense::click());

    if ui.is_rect_visible(rect) {
        let press = theme::Press::track(ui, &resp);
        let fill = press.wash_color(&t, selected);

        if fill.a() > 0 {
            ui.painter().rect_filled(rect, t.radius, fill);
        }

        if selected {
            ui.painter().rect_stroke(
                rect.shrink(0.5),
                t.radius,
                Stroke::new(1.0, t.accent.gamma_multiply(0.45)),
                egui::StrokeKind::Inside,
            );
        }

        let tint = if selected {
            t.accent_text
        } else if resp.hovered() {
            t.text
        } else {
            t.icon
        };

        let glyph_size = (box_size * 0.5).round() * (1.0 - 0.08 * press.down);
        paint(ui, rect.translate(press.offset()), name, glyph_size, tint);
    }

    let resp = theme::hand(resp);
    if tooltip.is_empty() {
        resp
    } else {
        resp.on_hover_text(tooltip)
    }
}
