use eframe::egui::{self, Align2, Frame, Margin, Response, RichText, Sense, Stroke, Ui};
use eframe::emath::{pos2, vec2};

use super::{theme, widgets};
use crate::app::{Action, AppState, Step};
use crate::assets::Slot;

fn step_chip(ui: &mut Ui, number: usize, label: &str, active: bool, complete: bool) -> Response {
    let galley = ui.painter().layout_no_wrap(label.to_owned(), theme::body(), theme::TEXT);
    let size = vec2(galley.size().x + 50.0, 32.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let p = ui.painter();
    if active {
        p.rect_filled(rect, 16.0, theme::HOVER);
        p.rect_stroke(rect, 16.0, Stroke::new(1.0, theme::LUMEN.gamma_multiply(0.6)));
    } else if resp.hovered() {
        p.rect_filled(rect, 16.0, theme::RAISED);
    }
    let dot = pos2(rect.left() + 17.0, rect.center().y);
    let (fill, fg, glyph) = match (complete, active) {
        (_, true) => (theme::LUMEN, theme::ON_LUMEN, number.to_string()),
        (true, false) => (theme::OK.gamma_multiply(0.18), theme::OK, "✔".to_string()),
        (false, false) => (theme::RAISED, theme::MUTED, number.to_string()),
    };
    p.circle_filled(dot, 10.0, fill);
    p.text(dot, Align2::CENTER_CENTER, glyph, egui::FontId::proportional(11.0), fg);
    let text_color = if active { theme::TEXT } else { theme::MUTED };
    p.text(pos2(rect.left() + 34.0, rect.center().y), Align2::LEFT_CENTER, label, theme::body(), text_color);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn show(ctx: &egui::Context, s: &mut AppState) {
    let frame = Frame::none()
        .fill(theme::PANEL)
        .inner_margin(Margin::symmetric(14.0, 0.0))
        .stroke(Stroke::new(1.0, theme::LINE));
    egui::TopBottomPanel::bottom("workflow").exact_height(54.0).frame(frame).show(ctx, |ui| {
        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            for (i, step) in Step::ALL.iter().enumerate() {
                if i > 0 {
                    let (r, _) = ui.allocate_exact_size(vec2(18.0, 2.0), Sense::hover());
                    ui.painter().line_segment([r.left_center(), r.right_center()], Stroke::new(1.0, theme::LINE));
                }
                if step_chip(ui, i + 1, step.label(), s.step == *step && !s.info_open, s.step_complete(*step)).clicked() {
                    s.push(Action::GoTo(*step));
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                let can_export = s.assets.get(Slot::Artwork).is_some() && !s.export.running();
                if widgets::primary_button(ui, "  Export profile artwork  ", can_export)
                    .on_hover_text("Ctrl+E")
                    .clicked()
                {
                    s.push(Action::StartExport);
                }
                if s.export.running() {
                    ui.add(egui::ProgressBar::new(s.export.progress).desired_width(180.0).text(s.export.label.clone()).animate(true));
                } else if matches!(s.step, Step::Edit) {
                    if let Some(p) = s.canvas.hover {
                        ui.label(widgets::faint(format!("x {:.0}  y {:.0}", p.x, p.y)));
                    }
                    ui.label(widgets::faint(format!("{:.0}%", s.canvas.zoom * 100.0)));
                }
                let height = s.project.art_height as u32;
                let layout = s.layout();
                ui.label(RichText::new(format!("{}  {} × {}", layout.name, layout.width() as u32, height)).color(theme::FAINT).size(11.5));
            });
        });
    });
}
