//! LuminarCore visual language: neutral charcoal surfaces in the style of
//! professional image editors, restrained blue for selection and focus, and
//! print-style crop marks on the canvas as the signature detail.
//!
//! Text colours are chosen for at least 4.5:1 contrast on VOID, PANEL and RAISED.

use std::sync::Arc;

use eframe::egui::{self, Color32, FontFamily, FontId, Rounding, Stroke, TextStyle};
use eframe::epaint::Shadow;

pub const VOID: Color32 = Color32::from_rgb(0x1E, 0x1E, 0x1E);
pub const PANEL: Color32 = Color32::from_rgb(0x2B, 0x2B, 0x2B);
pub const RAISED: Color32 = Color32::from_rgb(0x36, 0x36, 0x36);
pub const HOVER: Color32 = Color32::from_rgb(0x42, 0x42, 0x42);
pub const LINE: Color32 = Color32::from_rgb(0x47, 0x47, 0x47);
pub const TEXT: Color32 = Color32::from_rgb(0xE3, 0xE3, 0xE3);
pub const MUTED: Color32 = Color32::from_rgb(0xBD, 0xBD, 0xBD);
pub const FAINT: Color32 = Color32::from_rgb(0xA2, 0xA2, 0xA2);
pub const LUMEN: Color32 = Color32::from_rgb(0x5A, 0xA6, 0xF7);
pub const LUMEN_DEEP: Color32 = Color32::from_rgb(0x1A, 0x62, 0xC4);
pub const ON_LUMEN: Color32 = Color32::from_rgb(0x0B, 0x1B, 0x2E);
pub const VIOLET: Color32 = Color32::from_rgb(0xB7, 0x91, 0xF2);
pub const OK: Color32 = Color32::from_rgb(0x5C, 0xC4, 0x92);
pub const WARN: Color32 = Color32::from_rgb(0xE9, 0xA6, 0x3F);
pub const ERR: Color32 = Color32::from_rgb(0xF4, 0x7C, 0x7C);

/// Distinct outline colours for slice regions, in layout order.
pub const REGION_COLORS: [Color32; 4] = [LUMEN, VIOLET, OK, WARN];

pub fn region_color(i: usize) -> Color32 {
    REGION_COLORS[i % REGION_COLORS.len()]
}

pub fn rgb(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

pub fn small() -> FontId {
    FontId::proportional(11.5)
}

pub fn body() -> FontId {
    FontId::proportional(13.0)
}

pub fn apply(ctx: &egui::Context) {
    ctx.set_theme(egui::ThemePreference::Dark);
    let mut style = (*ctx.style()).clone();

    style.text_styles = [
        (TextStyle::Small, FontId::new(11.5, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(13.0, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(13.0, FontFamily::Proportional)),
        (TextStyle::Heading, FontId::new(17.0, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(12.0, FontFamily::Monospace)),
    ]
    .into();

    let s = &mut style.spacing;
    s.item_spacing = egui::vec2(8.0, 6.0);
    s.button_padding = egui::vec2(10.0, 5.0);
    s.interact_size = egui::vec2(36.0, 26.0);
    s.slider_width = 150.0;
    s.combo_width = 160.0;
    s.window_margin = egui::Margin::same(14.0);
    s.menu_margin = egui::Margin::same(6.0);
    s.scroll = egui::style::ScrollStyle::thin();

    let v = &mut style.visuals;
    *v = egui::Visuals::dark();
    v.dark_mode = true;
    v.panel_fill = PANEL;
    v.window_fill = RAISED;
    v.extreme_bg_color = VOID;
    v.faint_bg_color = RAISED;
    v.code_bg_color = VOID;
    v.hyperlink_color = LUMEN;
    v.window_stroke = Stroke::new(1.0, LINE);
    v.window_rounding = Rounding::same(10.0);
    v.menu_rounding = Rounding::same(8.0);
    v.window_shadow = Shadow { offset: egui::vec2(0.0, 12.0), blur: 32.0, spread: 0.0, color: Color32::from_black_alpha(140) };
    v.popup_shadow = Shadow { offset: egui::vec2(0.0, 6.0), blur: 18.0, spread: 0.0, color: Color32::from_black_alpha(120) };
    v.selection.bg_fill = LUMEN_DEEP;
    // Selected rows read as light text on blue, never blue on blue.
    v.selection.stroke = Stroke::new(1.0, TEXT);
    v.slider_trailing_fill = true;
    v.handle_shape = egui::style::HandleShape::Rect { aspect_ratio: 0.55 };
    v.striped = false;

    let r = Rounding::same(6.0);
    let w = &mut v.widgets;
    w.noninteractive.bg_fill = PANEL;
    w.noninteractive.weak_bg_fill = PANEL;
    w.noninteractive.bg_stroke = Stroke::new(1.0, LINE);
    w.noninteractive.fg_stroke = Stroke::new(1.0, MUTED);
    w.noninteractive.rounding = r;

    w.inactive.bg_fill = RAISED;
    w.inactive.weak_bg_fill = RAISED;
    w.inactive.bg_stroke = Stroke::new(1.0, LINE);
    w.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    w.inactive.rounding = r;

    w.hovered.bg_fill = HOVER;
    w.hovered.weak_bg_fill = HOVER;
    w.hovered.bg_stroke = Stroke::new(1.0, LUMEN.gamma_multiply(0.45));
    w.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    w.hovered.rounding = r;
    w.hovered.expansion = 0.0;

    w.active.bg_fill = LUMEN_DEEP;
    w.active.weak_bg_fill = LUMEN_DEEP;
    w.active.bg_stroke = Stroke::new(1.0, LUMEN);
    w.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    w.active.rounding = r;
    w.active.expansion = 0.0;

    w.open = w.hovered;

    ctx.set_style(style);
}

/// Procedural window icon: a luminous ring with a bright core.
pub fn app_icon() -> Arc<egui::IconData> {
    const N: u32 = 64;
    let mut rgba = Vec::with_capacity((N * N * 4) as usize);
    let c = (N as f32 - 1.0) / 2.0;
    for y in 0..N {
        for x in 0..N {
            let d = ((x as f32 - c).powi(2) + (y as f32 - c).powi(2)).sqrt() / c;
            let ring = (1.0 - ((d - 0.72).abs() / 0.09)).clamp(0.0, 1.0);
            let core = (1.0 - d / 0.26).clamp(0.0, 1.0);
            let glow = (1.0 - d).clamp(0.0, 1.0).powi(3) * 0.35;
            let bg = if d < 0.98 { 1.0 } else { 0.0 };
            let lum = (ring + core + glow).min(1.0);
            let base = [10.0, 14.0, 26.0];
            let light = [168.0, 212.0, 255.0];
            for i in 0..3 {
                rgba.push((base[i] + (light[i] - base[i]) * lum) as u8);
            }
            rgba.push((bg * 255.0) as u8);
        }
    }
    Arc::new(egui::IconData { rgba, width: N, height: N })
}
