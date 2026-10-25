use eframe::egui::{self, Frame, Margin, RichText, Sense, Stroke, Ui};
use eframe::emath::vec2;

use super::theme;
use crate::app::{Action, AppState, Step};
use crate::assets::Slot;
use crate::errors::display_name;
use crate::project::transform::FitMode;

fn item(ui: &mut Ui, label: &str, shortcut: &str, enabled: bool) -> bool {
    let clicked = ui
        .add_enabled(enabled, egui::Button::new(label).shortcut_text(shortcut).min_size(vec2(220.0, 0.0)))
        .clicked();
    if clicked {
        ui.close_menu();
    }
    clicked
}

pub fn show(ctx: &egui::Context, s: &mut AppState) {
    let frame = Frame::none()
        .fill(theme::VOID)
        .inner_margin(Margin::symmetric(14.0, 0.0))
        .stroke(Stroke::new(1.0, theme::LINE));
    egui::TopBottomPanel::top("top_bar").exact_height(42.0).frame(frame).show(ctx, |ui| {
        ui.horizontal_centered(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
            ui.painter().circle_stroke(r.center(), 7.0, Stroke::new(1.6, theme::LUMEN));
            ui.painter().circle_filled(r.center(), 2.4, theme::LUMEN);
            ui.label(RichText::new("LUMINAR STEAM STUDIO").size(12.0).color(theme::TEXT).strong().extra_letter_spacing(2.2));
            ui.add_space(18.0);
            menus(ui, s);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("⚙ Settings").clicked() {
                    s.windows.settings = !s.windows.settings;
                }
                if ui.button("Upload guide").on_hover_text("Step-by-step help for putting your pieces on Steam").clicked() {
                    s.windows.upload_guide = !s.windows.upload_guide;
                }
                ui.separator();
                if ui.add_enabled(s.history.can_redo(), egui::Button::new("Redo")).on_hover_text("Redo (Ctrl+Shift+Z)").clicked() {
                    s.push(Action::Redo);
                }
                if ui.add_enabled(s.history.can_undo(), egui::Button::new("Undo")).on_hover_text("Undo (Ctrl+Z)").clicked() {
                    s.push(Action::Undo);
                }
                ui.separator();
                let location = s
                    .project_path
                    .as_ref()
                    .map(|p| display_name(p))
                    .unwrap_or_else(|| "Not saved yet".into());
                ui.label(RichText::new(location).color(theme::FAINT).size(11.5));
                if s.is_dirty() {
                    ui.label(RichText::new("●").color(theme::WARN).size(10.0)).on_hover_text("Unsaved changes");
                }
                ui.label(RichText::new(&s.project.name).color(theme::TEXT));
            });
        });
    });
}

fn menus(ui: &mut Ui, s: &mut AppState) {
    ui.menu_button("File", |ui| {
        if item(ui, "New project", "Ctrl+N", true) {
            s.push(Action::NewProject);
        }
        if item(ui, "Open project…", "Ctrl+O", true) {
            s.push(Action::OpenProjectDialog);
        }
        ui.add_enabled_ui(!s.settings.recent.is_empty(), |ui| {
            ui.menu_button("Open recent", |ui| {
                for path in s.settings.recent.clone() {
                    if ui.button(display_name(&path)).on_hover_text(path.display().to_string()).clicked() {
                        s.push(Action::OpenProjectPath(path));
                        ui.close_menu();
                    }
                }
            });
        });
        if item(ui, "Save", "Ctrl+S", true) {
            s.push(Action::Save);
        }
        if item(ui, "Save as…", "Ctrl+Shift+S", true) {
            s.push(Action::SaveAs);
        }
        ui.separator();
        if item(ui, "Import artwork…", "Ctrl+I", true) {
            s.push(Action::ImportDialog(Slot::Artwork));
        }
        if item(ui, "Import background…", "", true) {
            s.push(Action::ImportDialog(Slot::Background));
        }
        if item(ui, "Import avatar…", "", true) {
            s.push(Action::ImportDialog(Slot::Avatar));
        }
        ui.separator();
        if item(ui, "Export profile artwork", "Ctrl+E", !s.export.running()) {
            s.push(Action::StartExport);
        }
        ui.separator();
        if item(ui, "Quit", "", true) {
            s.push(Action::Quit);
        }
    });
    ui.menu_button("Edit", |ui| {
        if item(ui, "Undo", "Ctrl+Z", s.history.can_undo()) {
            s.push(Action::Undo);
        }
        if item(ui, "Redo", "Ctrl+Shift+Z", s.history.can_redo()) {
            s.push(Action::Redo);
        }
        ui.separator();
        let has_art = s.project.artwork.is_some();
        if item(ui, "Fill all pieces", "", has_art) {
            s.push(Action::Fit(FitMode::Cover));
        }
        if item(ui, "Center artwork", "", has_art) {
            s.push(Action::CenterArtwork);
        }
        if item(ui, "Match height to image", "", has_art) {
            s.push(Action::MatchHeightToArtwork);
        }
        if item(ui, "Reset transform", "", has_art) {
            s.push(Action::ResetArtwork);
        }
    });
    ui.menu_button("View", |ui| {
        for (i, step) in Step::ALL.iter().enumerate() {
            if item(ui, step.label(), &format!("Ctrl+{}", i + 1), true) {
                s.push(Action::GoTo(*step));
            }
        }
    });
    ui.menu_button("Help", |ui| {
        if item(ui, "Steam upload guide", "", true) {
            s.windows.upload_guide = true;
        }
        if item(ui, "Keyboard shortcuts", "", true) {
            s.windows.shortcuts = true;
        }
        ui.separator();
        if item(ui, "About Luminar Steam Studio", "", true) {
            s.push(Action::ShowInfo);
        }
    });
}
