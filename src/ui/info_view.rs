use eframe::egui::{self, Frame, Margin, RichText, Sense, Stroke, Ui};
use eframe::emath::vec2;

use super::{theme, widgets};
use crate::about::{self, Credit};
use crate::app::{Action, AppState};

pub fn show(ui: &mut Ui, s: &mut AppState) {
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        let width = (ui.available_width() - 72.0).clamp(320.0, 760.0);
        let side = ((ui.available_width() - width) / 2.0).max(0.0);
        ui.add_space(40.0);
        ui.horizontal(|ui| {
            ui.add_space(side);
            ui.vertical(|ui| {
                ui.set_width(width);
                body(ui, s);
                ui.add_space(40.0);
            });
        });
    });
}

fn body(ui: &mut Ui, s: &mut AppState) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::hover());
        ui.painter().circle_stroke(r.center(), 12.0, Stroke::new(2.2, theme::LUMEN));
        ui.painter().circle_filled(r.center(), 4.0, theme::LUMEN);
        ui.label(
            RichText::new(about::APP_NAME.to_uppercase())
                .size(22.0)
                .color(theme::TEXT)
                .strong()
                .extra_letter_spacing(3.0),
        );
    });
    ui.add_space(6.0);
    ui.label(RichText::new(about::TAGLINE).size(14.0).color(theme::MUTED));
    ui.add_space(12.0);

    // Free status: stated in words, with colour only as reinforcement.
    Frame::none()
        .fill(theme::OK.gamma_multiply(0.12))
        .stroke(Stroke::new(1.0, theme::OK))
        .rounding(14.0)
        .inner_margin(Margin::symmetric(12.0, 5.0))
        .show(ui, |ui| {
            ui.label(RichText::new(format!("✔ {}", about::PRICING)).color(theme::OK).strong());
        });
    ui.add_space(4.0);
    ui.label(RichText::new(about::PRICING_DETAIL).color(theme::MUTED));

    widgets::section(ui, "Details");
    widgets::card().show(ui, |ui| {
        ui.set_width(ui.available_width());
        egui::Grid::new("about_details").num_columns(2).spacing(vec2(28.0, 10.0)).show(ui, |ui| {
            for (label, value) in [
                ("Made by", about::CREATOR),
                ("Studio", about::STUDIO),
                ("Version", about::VERSION),
            ] {
                ui.label(RichText::new(label).color(theme::MUTED));
                ui.label(RichText::new(value).color(theme::TEXT).strong());
                ui.end_row();
            }
            ui.label(RichText::new("GitHub").color(theme::MUTED));
            ui.horizontal(|ui| {
                link(ui, s, about::GITHUB_URL, about::GITHUB_URL);
                ui.label(RichText::new("opens in your browser").color(theme::MUTED).size(11.5));
            });
            ui.end_row();
        });
    });

    widgets::section(ui, "Privacy");
    widgets::card().show(ui, |ui| {
        ui.set_width(ui.available_width());
        for fact in about::PRIVACY_FACTS {
            ui.horizontal_top(|ui| {
                ui.label(RichText::new("✔").color(theme::OK));
                ui.add(egui::Label::new(RichText::new(*fact).color(theme::TEXT)).wrap());
            });
        }
    });

    widgets::section(ui, "Third-party software");
    ui.label(RichText::new("Luminar Steam Studio is built with these open-source projects. They are owned by their authors and used under their licences.").color(theme::MUTED));
    ui.add_space(6.0);
    credits_table(ui, s, "about_libraries", about::LIBRARIES);

    widgets::section(ui, "Fonts");
    ui.label(RichText::new("Bundled through egui's default font set.").color(theme::MUTED));
    ui.add_space(6.0);
    credits_table(ui, s, "about_fonts", about::FONTS);

    ui.add_space(10.0);
    ui.add(egui::Label::new(RichText::new(about::TRANSITIVE_NOTE).color(theme::MUTED).size(12.0)).wrap());
    ui.add_space(6.0);
    ui.add(egui::Label::new(RichText::new(about::TRADEMARK_NOTICE).color(theme::MUTED).size(12.0)).wrap());
}

fn credits_table(ui: &mut Ui, s: &mut AppState, id: &str, credits: &[Credit]) {
    widgets::card().show(ui, |ui| {
        ui.set_width(ui.available_width());
        egui::Grid::new(id).num_columns(4).spacing(vec2(18.0, 8.0)).striped(false).show(ui, |ui| {
            for h in ["Name", "Used for", "Licence", "Project"] {
                ui.label(RichText::new(h).color(theme::MUTED).size(12.0).strong());
            }
            ui.end_row();
            for c in credits {
                ui.label(RichText::new(c.name).color(theme::TEXT));
                ui.label(RichText::new(c.purpose).color(theme::MUTED));
                ui.label(RichText::new(c.license).color(theme::TEXT));
                link(ui, s, c.url.trim_start_matches("https://"), c.url);
                ui.end_row();
            }
        });
    });
}

/// Keyboard-focusable link that opens through the app's own https-only path.
fn link(ui: &mut Ui, s: &mut AppState, text: &str, url: &str) {
    let resp = ui.link(RichText::new(text).color(theme::LUMEN)).on_hover_text(url);
    if resp.clicked() {
        s.push(Action::OpenUrl(url.to_string()));
    }
}
