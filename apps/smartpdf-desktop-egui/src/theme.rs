//! Design tokens and styling for SmartPDF egui desktop interface.
//! Closely aligns with Adobe Acrobat Pro's modern desktop workspace design grammar.
#![allow(dead_code)]

use egui::{Color32, CornerRadius, Stroke, Visuals};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ThemeMode {
    #[default]
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug)]
pub struct DesignTokens {
    pub mode: ThemeMode,
    /// Top menu / titlebar
    pub titlebar: Color32,
    /// Two-tier chrome background
    pub chrome_bg: Color32,
    /// Side panels / drawers background
    pub panel_bg: Color32,
    /// Canvas / pasteboard background behind PDF page
    pub canvas_bg: Color32,
    /// Card background
    pub card_bg: Color32,
    /// Subtle divider line
    pub divider: Color32,
    /// Strong border line
    pub border: Color32,
    /// Primary text
    pub text_primary: Color32,
    /// Muted secondary text
    pub text_secondary: Color32,
    /// Tertiary / faint text
    pub text_faint: Color32,
    /// Icon tint
    pub icon_tint: Color32,
    /// Interactive hover background
    pub hover_bg: Color32,
    /// Interactive pressed background
    pub pressed_bg: Color32,
    /// Selected item background
    pub selected_bg: Color32,
    /// Acrobat blue accent
    pub accent: Color32,
    /// Accent text / focus
    pub accent_text: Color32,
    /// Accent soft wash
    pub accent_soft: Color32,
    /// Page shadow
    pub page_shadow: Color32,
    /// Selection box highlight stroke
    pub selection_stroke: Color32,
    /// Selection fill tint
    pub selection_fill: Color32,
    /// Control corner radius
    pub radius: CornerRadius,
}

impl DesignTokens {
    pub fn light() -> Self {
        Self {
            mode: ThemeMode::Light,
            titlebar: Color32::from_rgb(0xF5, 0xF6, 0xF8),
            chrome_bg: Color32::from_rgb(0xFF, 0xFF, 0xFF),
            panel_bg: Color32::from_rgb(0xFA, 0xFA, 0xFB),
            // Adobe Acrobat neutral gray canvas
            canvas_bg: Color32::from_rgb(0x32, 0x36, 0x39),
            card_bg: Color32::from_rgb(0xFF, 0xFF, 0xFF),
            divider: Color32::from_rgb(0xE6, 0xE7, 0xEB),
            border: Color32::from_rgb(0xD5, 0xD7, 0xDC),
            text_primary: Color32::from_rgb(0x22, 0x24, 0x2A),
            text_secondary: Color32::from_rgb(0x60, 0x64, 0x70),
            text_faint: Color32::from_rgb(0x8D, 0x92, 0x9E),
            icon_tint: Color32::from_rgb(0x40, 0x44, 0x50),
            hover_bg: Color32::from_rgb(0xF0, 0xF2, 0xF6),
            pressed_bg: Color32::from_rgb(0xE4, 0xE7, 0xEE),
            selected_bg: Color32::from_rgb(0xE7, 0xF0, 0xFD),
            accent: Color32::from_rgb(0x14, 0x73, 0xE6), // Acrobat Blue
            accent_text: Color32::from_rgb(0x0F, 0x5B, 0xBA),
            accent_soft: Color32::from_rgb(0xE5, 0xF1, 0xFD),
            page_shadow: Color32::from_black_alpha(80),
            selection_stroke: Color32::from_rgb(0x14, 0x73, 0xE6),
            selection_fill: Color32::from_rgba_premultiplied(20, 115, 230, 35),
            radius: CornerRadius::same(5),
        }
    }

    pub fn dark() -> Self {
        Self {
            mode: ThemeMode::Dark,
            titlebar: Color32::from_rgb(0x1E, 0x20, 0x23),
            chrome_bg: Color32::from_rgb(0x25, 0x28, 0x2C),
            panel_bg: Color32::from_rgb(0x21, 0x23, 0x27),
            canvas_bg: Color32::from_rgb(0x15, 0x16, 0x18),
            card_bg: Color32::from_rgb(0x2B, 0x2E, 0x33),
            divider: Color32::from_rgb(0x38, 0x3C, 0x44),
            border: Color32::from_rgb(0x42, 0x47, 0x52),
            text_primary: Color32::from_rgb(0xEA, 0xEC, 0xF0),
            text_secondary: Color32::from_rgb(0x9E, 0xA4, 0xB2),
            text_faint: Color32::from_rgb(0x6E, 0x74, 0x82),
            icon_tint: Color32::from_rgb(0xC8, 0xCC, 0xD6),
            hover_bg: Color32::from_rgb(0x32, 0x36, 0x3E),
            pressed_bg: Color32::from_rgb(0x3D, 0x42, 0x4C),
            selected_bg: Color32::from_rgb(0x1A, 0x3A, 0x66),
            accent: Color32::from_rgb(0x26, 0x80, 0xEB),
            accent_text: Color32::from_rgb(0x52, 0x9B, 0xF0),
            accent_soft: Color32::from_rgb(0x1A, 0x3A, 0x66),
            page_shadow: Color32::from_black_alpha(140),
            selection_stroke: Color32::from_rgb(0x52, 0x9B, 0xF0),
            selection_fill: Color32::from_rgba_premultiplied(82, 155, 240, 45),
            radius: CornerRadius::same(5),
        }
    }

    pub fn apply_to_context(&self, ctx: &egui::Context) {
        let mut visuals = match self.mode {
            ThemeMode::Light => Visuals::light(),
            ThemeMode::Dark => Visuals::dark(),
        };

        visuals.window_fill = self.panel_bg;
        visuals.panel_fill = self.chrome_bg;
        visuals.window_stroke = Stroke::new(1.0, self.border);
        visuals.widgets.noninteractive.bg_fill = self.chrome_bg;
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, self.text_primary);
        visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, self.text_primary);
        visuals.widgets.hovered.bg_fill = self.hover_bg;
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, self.text_primary);
        visuals.widgets.active.bg_fill = self.pressed_bg;
        visuals.widgets.active.fg_stroke = Stroke::new(1.0, self.text_primary);
        visuals.selection.bg_fill = self.selected_bg;
        visuals.selection.stroke = Stroke::new(1.0, self.accent);

        ctx.set_visuals(visuals);
    }
}
