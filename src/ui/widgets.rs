use eframe::egui::{self, Align2, Color32, Frame, Margin, Response, RichText, Rounding, Sense, Stroke, Ui, Vec2};
use eframe::emath::{pos2, vec2};

use super::theme;
use crate::steam::validation::Severity;

pub fn toolbar_frame() -> Frame {
    Frame::none()
        .fill(theme::PANEL)
        .inner_margin(Margin::symmetric(12.0, 7.0))
        .stroke(Stroke::new(1.0, theme::LINE))
}

pub fn card() -> Frame {
    Frame::none()
        .fill(theme::RAISED)
        .rounding(Rounding::same(8.0))
        .stroke(Stroke::new(1.0, theme::LINE))
        .inner_margin(Margin::same(12.0))
}

/// Sentence-case section heading followed by a hairline.
pub fn section(ui: &mut Ui, title: &str) {
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).size(12.0).color(theme::MUTED).strong());
        let rest = ui.available_rect_before_wrap();
        let y = rest.center().y + 1.0;
        ui.painter().line_segment([pos2(rest.left() + 4.0, y), pos2(rest.right(), y)], Stroke::new(1.0, theme::LINE));
    });
    ui.add_space(4.0);
}

pub fn primary_button(ui: &mut Ui, text: &str, enabled: bool) -> Response {
    let fill = if enabled { theme::LUMEN } else { theme::RAISED };
    let fg = if enabled { theme::ON_LUMEN } else { theme::FAINT };
    ui.add_enabled(
        enabled,
        egui::Button::new(RichText::new(text).color(fg).strong())
            .fill(fill)
            .stroke(Stroke::NONE)
            .rounding(6.0)
            .min_size(vec2(0.0, 32.0)),
    )
}

pub fn severity_glyph(s: Severity) -> (&'static str, Color32) {
    match s {
        Severity::Ok => ("✔", theme::OK),
        Severity::Info => ("ℹ", theme::LUMEN),
        Severity::Warning => ("⚠", theme::WARN),
        Severity::Error => ("✖", theme::ERR),
    }
}

#[derive(Debug, Clone, Copy)]
pub enum NavIcon {
    Artwork,
    Background,
    Avatar,
    Showcase,
    Templates,
    Profile,
    Info,
}

/// Line icons drawn with the painter, so they never depend on font coverage.
fn paint_icon(p: &egui::Painter, icon: NavIcon, c: egui::Pos2, color: Color32) {
    use egui::Rect;
    let s = Stroke::new(1.3, color);
    let r = |w: f32, h: f32| Rect::from_center_size(c, vec2(w, h));
    match icon {
        NavIcon::Artwork => {
            let f = r(15.0, 12.0);
            p.rect_stroke(f, 1.5, s);
            p.line_segment([f.left_bottom() + vec2(2.0, -2.0), c + vec2(-1.0, 0.0)], s);
            p.line_segment([c + vec2(-1.0, 0.0), f.right_bottom() + vec2(-2.0, -2.0)], s);
            p.circle_filled(f.right_top() + vec2(-4.0, 3.5), 1.4, color);
        }
        NavIcon::Background => {
            p.rect_stroke(r(15.0, 11.0).translate(vec2(1.5, 1.5)), 1.5, s);
            p.rect_filled(r(15.0, 11.0).translate(vec2(-1.5, -1.5)), 1.5, theme::PANEL);
            p.rect_stroke(r(15.0, 11.0).translate(vec2(-1.5, -1.5)), 1.5, s);
        }
        NavIcon::Avatar => {
            p.rect_stroke(r(14.0, 14.0), 2.0, s);
            p.circle_stroke(c + vec2(0.0, -1.5), 2.6, s);
            p.line_segment([c + vec2(-4.0, 5.0), c + vec2(4.0, 5.0)], s);
        }
        NavIcon::Showcase => {
            // The layout itself: a wide piece and a narrow strip.
            p.rect_stroke(Rect::from_min_size(c + vec2(-8.0, -7.0), vec2(10.0, 14.0)), 1.0, s);
            p.rect_stroke(Rect::from_min_size(c + vec2(4.0, -7.0), vec2(4.0, 14.0)), 1.0, s);
        }
        NavIcon::Templates => {
            for (dx, dy) in [(-4.0, -4.0), (4.0, -4.0), (-4.0, 4.0), (4.0, 4.0)] {
                p.rect_stroke(Rect::from_center_size(c + vec2(dx, dy), vec2(6.0, 6.0)), 1.0, s);
            }
        }
        NavIcon::Info => {
            p.circle_stroke(c, 7.0, s);
            p.circle_filled(c + vec2(0.0, -3.2), 1.1, color);
            p.line_segment([c + vec2(0.0, -0.8), c + vec2(0.0, 3.8)], s);
        }
        NavIcon::Profile => {
            p.circle_stroke(c + vec2(-3.5, -3.0), 2.8, s);
            for (i, w) in [7.0, 12.0].iter().enumerate() {
                let y = c.y + 2.5 + i as f32 * 3.5;
                p.line_segment([pos2(c.x - 7.0, y), pos2(c.x - 7.0 + w, y)], s);
            }
        }
    }
}

/// Left-nav row with a status detail on the right.
pub fn nav_item(ui: &mut Ui, icon: NavIcon, label: &str, detail: &str, selected: bool, filled: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::click());
    let p = ui.painter();
    if selected {
        p.rect_filled(rect, 6.0, theme::HOVER);
        p.rect_filled(egui::Rect::from_min_size(rect.min + vec2(0.0, 8.0), vec2(2.5, 18.0)), 1.0, theme::LUMEN);
    } else if resp.hovered() {
        p.rect_filled(rect, 6.0, theme::RAISED);
    }
    let fg = if selected { theme::TEXT } else { theme::MUTED };
    paint_icon(p, icon, rect.left_center() + vec2(20.0, 0.0), fg);
    p.text(rect.left_center() + vec2(38.0, 0.0), Align2::LEFT_CENTER, label, theme::body(), fg);
    let dot = if filled { theme::OK } else { theme::FAINT };
    p.circle_filled(rect.right_center() - vec2(12.0, 0.0), 3.0, dot);
    if !detail.is_empty() {
        p.text(rect.right_center() - vec2(22.0, 0.0), Align2::RIGHT_CENTER, detail, theme::small(), theme::FAINT);
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Labelled numeric row used throughout the inspector.
pub fn value_row(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui) -> Response) -> Response {
    ui.horizontal(|ui| {
        ui.add_sized(vec2(84.0, 22.0), egui::Label::new(RichText::new(label).color(theme::MUTED)).truncate());
        add(ui)
    })
    .inner
}

/// Segmented control. Returns true if the value changed.
pub fn segmented<T: PartialEq + Copy>(ui: &mut Ui, value: &mut T, options: &[(T, &str)]) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for (v, label) in options {
            let on = *value == *v;
            let text = RichText::new(*label).color(if on { theme::ON_LUMEN } else { theme::TEXT });
            let b = egui::Button::new(text).fill(if on { theme::LUMEN } else { theme::RAISED }).rounding(5.0);
            if ui.add(b).clicked() && !on {
                *value = *v;
                changed = true;
            }
        }
    });
    changed
}

pub fn color_swatch(ui: &mut Ui, rgb: &mut [u8; 3], tooltip: &str) -> Response {
    let mut c = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
    let r = ui.color_edit_button_srgba(&mut c).on_hover_text(tooltip);
    *rgb = [c.r(), c.g(), c.b()];
    r
}

/// Small image thumbnail that keeps aspect ratio inside `max`.
pub fn thumbnail(ui: &mut Ui, tex: &egui::TextureHandle, max: Vec2) -> Response {
    let size = tex.size_vec2();
    let k = (max.x / size.x).min(max.y / size.y);
    let (rect, resp) = ui.allocate_exact_size(max, Sense::hover());
    let img_rect = egui::Rect::from_center_size(rect.center(), size * k);
    ui.painter().rect_filled(rect, 6.0, theme::VOID);
    ui.painter().image(tex.id(), img_rect, egui::Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
    ui.painter().rect_stroke(rect, 6.0, Stroke::new(1.0, theme::LINE));
    resp
}

pub fn muted(text: impl Into<String>) -> RichText {
    RichText::new(text.into()).color(theme::MUTED)
}

pub fn faint(text: impl Into<String>) -> RichText {
    RichText::new(text.into()).color(theme::FAINT).size(11.5)
}
