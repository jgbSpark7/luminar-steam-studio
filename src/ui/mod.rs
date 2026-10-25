mod bottom_bar;
mod dialogs;
mod export_view;
mod import_view;
mod info_view;
mod inspector;
pub mod preview;
mod sidebar;
pub mod theme;
mod top_bar;
pub mod widgets;

use eframe::egui::{self, Align2, Frame, Id, LayerId, Order, RichText, Stroke};
use eframe::emath::vec2;

use crate::app::{Action, AppState, Step, ToastKind};
use crate::assets::image_loader::is_project_file;
use crate::assets::Slot;
use crate::editor::{canvas, paint};

pub fn draw(ctx: &egui::Context, s: &mut AppState) {
    top_bar::show(ctx, s);
    bottom_bar::show(ctx, s);
    sidebar::show(ctx, s);
    inspector::show(ctx, s);

    egui::CentralPanel::default().frame(Frame::none().fill(theme::VOID)).show(ctx, |ui| match s.step {
        _ if s.info_open => info_view::show(ui, s),
        Step::Import => import_view::show(ui, s),
        Step::Edit => canvas::show_editor(ui, &mut s.canvas, &mut s.project, &s.assets, &s.config, &mut s.actions),
        Step::Slice => canvas::show_slices(ui, &s.project, &s.assets, &s.config),
        Step::Preview => preview::show(ui, &mut s.preview, &s.project, &s.assets, &s.config),
        Step::Export => export_view::show(ui, s),
    });

    route_unclaimed_drops(ctx, s);
    dialogs::show(ctx, s);
    toasts(ctx, s);
    drop_overlay(ctx);
}

/// Anything dropped outside a specific target: projects open, images become the artwork.
fn route_unclaimed_drops(ctx: &egui::Context, s: &mut AppState) {
    if s.dropped.is_empty() {
        return;
    }
    let mut files = std::mem::take(&mut s.dropped);
    let first = files.remove(0);
    if is_project_file(&first) {
        s.push(Action::OpenProjectPath(first));
    } else {
        s.push(Action::ImportPath(Slot::Artwork, first));
    }
    if !files.is_empty() {
        s.toast(ctx, ToastKind::Info, "Only the first dropped file was used");
    }
}

fn toasts(ctx: &egui::Context, s: &AppState) {
    if s.toasts.is_empty() {
        return;
    }
    egui::Area::new(Id::new("toasts"))
        .order(Order::Tooltip)
        .anchor(Align2::RIGHT_BOTTOM, vec2(-338.0, -66.0))
        .interactable(false)
        .show(ctx, |ui| {
            for t in s.toasts.iter().rev().take(4) {
                let (glyph, color) = match t.kind {
                    ToastKind::Info => ("ℹ", theme::LUMEN),
                    ToastKind::Success => ("✔", theme::OK),
                    ToastKind::Warning => ("⚠", theme::WARN),
                };
                Frame::none()
                    .fill(theme::RAISED)
                    .stroke(Stroke::new(1.0, theme::LINE))
                    .rounding(8.0)
                    .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                    .shadow(ctx.style().visuals.popup_shadow)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(glyph).color(color));
                            ui.label(RichText::new(&t.text).color(theme::TEXT));
                        });
                    });
                ui.add_space(6.0);
            }
        });
}

fn drop_overlay(ctx: &egui::Context) {
    let hovering = ctx.input(|i| !i.raw.hovered_files.is_empty());
    if !hovering {
        return;
    }
    let painter = ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("drop_overlay")));
    let r = ctx.screen_rect();
    painter.rect_filled(r, 0.0, theme::VOID.gamma_multiply(0.55));
    paint::dashed_rect(&painter, r.shrink(18.0), Stroke::new(1.5, theme::LUMEN), 10.0, 6.0);
    painter.text(r.center(), Align2::CENTER_CENTER, "Drop to import", egui::FontId::proportional(22.0), theme::TEXT);
    painter.text(
        r.center() + vec2(0.0, 28.0),
        Align2::CENTER_CENTER,
        "Images become your artwork. On the Import screen, drop onto a card to choose.",
        theme::body(),
        theme::MUTED,
    );
}
