use eframe::egui::{self, Align2, Id, Order, RichText, Sense, Ui};

use super::{theme, widgets};
use crate::app::{Action, AppState};
use crate::assets::Slot;
use crate::export::slicer;
use crate::steam::config::{override_path, ConfigSource};

pub fn show(ctx: &egui::Context, s: &mut AppState) {
    settings_window(ctx, s);
    upload_guide(ctx, s);
    shortcuts(ctx, s);
    confirm_discard(ctx, s);
    error(ctx, s);
}

/// Dims and blocks everything below a modal window.
fn modal_backdrop(ctx: &egui::Context, id: &str) {
    egui::Area::new(Id::new(id))
        .order(Order::PanelResizeLine)
        .fixed_pos(egui::Pos2::ZERO)
        .show(ctx, |ui| {
            let r = ctx.screen_rect();
            ui.allocate_rect(r, Sense::click_and_drag());
            ui.painter().rect_filled(r, 0.0, egui::Color32::from_black_alpha(150));
        });
}

fn modal(title: &str) -> egui::Window<'_> {
    egui::Window::new(title)
        .order(Order::Foreground)
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .default_width(420.0)
}

fn confirm_discard(ctx: &egui::Context, s: &mut AppState) {
    let Some(pending) = s.confirm_discard.clone() else { return };
    modal_backdrop(ctx, "confirm_backdrop");
    let mut close = false;
    modal("Unsaved changes").show(ctx, |ui| {
        ui.label(RichText::new(format!("Save changes to “{}” first?", s.project.name)).color(theme::TEXT));
        ui.label(widgets::muted("If you don't save, your changes since the last save will be lost."));
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if widgets::primary_button(ui, "Save", true).clicked() {
                s.push(Action::SaveThen(Box::new(pending.clone())));
                close = true;
            }
            if ui.button("Don't save").clicked() {
                s.push(Action::Forced(Box::new(pending.clone())));
                close = true;
            }
            if ui.button("Cancel").clicked() {
                close = true;
            }
        });
    });
    if close || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        s.confirm_discard = None;
    }
}

fn error(ctx: &egui::Context, s: &mut AppState) {
    let Some(err) = s.error.clone() else { return };
    modal_backdrop(ctx, "error_backdrop");
    let mut close = false;
    modal(&err.title).show(ctx, |ui| {
        ui.label(RichText::new(&err.detail).color(theme::TEXT));
        if let Some(hint) = &err.hint {
            ui.add_space(4.0);
            ui.label(widgets::muted(hint));
        }
        ui.add_space(10.0);
        if widgets::primary_button(ui, "OK", true).clicked() {
            close = true;
        }
    });
    if close || ctx.input(|i| i.key_pressed(egui::Key::Escape) || i.key_pressed(egui::Key::Enter)) {
        s.error = None;
    }
}

fn settings_window(ctx: &egui::Context, s: &mut AppState) {
    let mut open = s.windows.settings;
    egui::Window::new("Settings").open(&mut open).collapsible(false).resizable(false).default_width(420.0).show(ctx, |ui| {
        widgets::section(ui, "Interface");
        let mut scale = s.settings.ui_scale;
        if widgets::value_row(ui, "Interface size", |ui| ui.add(egui::Slider::new(&mut scale, 0.8..=1.6).step_by(0.05))).drag_stopped()
            || (scale != s.settings.ui_scale && !ui.input(|i| i.pointer.any_down()))
        {
            s.settings.ui_scale = scale;
            ctx.set_zoom_factor(scale);
        }
        ui.checkbox(&mut s.canvas.snapping, "Snap while dragging");
        ui.checkbox(&mut s.canvas.show_guides, "Show centre guides");

        widgets::section(ui, "Privacy");
        ui.label(widgets::muted("Everything stays on this computer. The only stored history is your recent-project list."));
        let n = s.settings.recent.len();
        if ui.add_enabled(n > 0, egui::Button::new(format!("Clear recent projects and folders ({n})"))).clicked() {
            s.settings.recent.clear();
            s.settings.last_open_dir = None;
            s.settings.last_import_dir = None;
            s.settings.last_export_dir = None;
        }

        widgets::section(ui, "Steam settings");
        ui.label(widgets::faint(
            "All Steam sizes live in one JSON file. If Steam changes its layout, edit the file and reload. No rebuild needed.",
        ));
        let src = match &s.config_source {
            ConfigSource::Embedded => "Using built-in sizes".to_string(),
            ConfigSource::File(p) => format!("Using {}", p.display()),
        };
        ui.label(RichText::new(src).color(theme::TEXT));
        if let Some(w) = &s.config_warning {
            ui.label(RichText::new(format!("⚠ {w}")).color(theme::WARN));
        }
        ui.horizontal(|ui| {
            if ui.button("Create editable copy").on_hover_text("Writes steam_config.json next to the app").clicked() {
                s.push(Action::WriteConfigCopy);
            }
            if ui.button("Reload").clicked() {
                s.push(Action::ReloadConfig);
            }
            if let Some(dir) = override_path().and_then(|p| p.parent().map(|d| d.to_path_buf())) {
                if ui.button("Open app folder").clicked() {
                    s.push(Action::OpenPath(dir));
                }
            }
        });
    });
    s.windows.settings = open;
}

fn upload_guide(ctx: &egui::Context, s: &mut AppState) {
    let mut open = s.windows.upload_guide;
    egui::Window::new("Steam upload guide")
        .open(&mut open)
        .collapsible(false)
        .default_width(480.0)
        .default_height(560.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui| guide_body(ui, s));
        });
    s.windows.upload_guide = open;
}

fn guide_body(ui: &mut Ui, s: &mut AppState) {
    ui.label(widgets::muted(
        "Everything happens on Steam's own website, signed in by you. This app never asks for or stores your Steam password.",
    ));
    ui.add_space(6.0);

    let up = s.config.upload.clone();
    let layout = s.layout().clone();
    let scale = s.project.export.scale.resolve(s.project.artwork.as_ref().map(|a| a.transform.scale));
    let ext = s.project.export.format.extension();
    let plans = slicer::plan(&layout, s.project.art_height, scale);
    let exported = s.export.report.clone();
    let mut actions = Vec::new();
    let mut n = 0usize;

    let mut step = |ui: &mut Ui, done: &mut bool, title: &str, body: &mut dyn FnMut(&mut Ui)| {
        n += 1;
        ui.add_space(8.0);
        ui.horizontal_top(|ui| {
            ui.checkbox(done, "");
            ui.vertical(|ui| {
                let color = if *done { theme::FAINT } else { theme::TEXT };
                ui.label(RichText::new(format!("{n}. {title}")).color(color).strong());
                body(ui);
            });
        });
    };

    let mut done = std::mem::take(&mut s.upload_done);
    done.resize(8 + plans.len(), false);
    let mut i = 0;
    let mut next = || {
        i += 1;
        i - 1
    };

    let i0 = next();
    step(ui, &mut done[i0], "Export your artwork", &mut |ui| match &exported {
        Some(r) => {
            ui.label(widgets::faint(format!("Exported to {}", r.folder.display())));
            if ui.small_button("Open folder").clicked() {
                actions.push(Action::OpenPath(r.folder.clone()));
            }
        }
        None => {
            if ui.small_button("Export now").clicked() {
                actions.push(Action::StartExport);
            }
        }
    });
    let i1 = next();
    step(ui, &mut done[i1], "Open Steam's artwork upload page", &mut |ui| {
        ui.label(widgets::faint("Sign in on Steam if asked."));
        if ui.small_button("Open upload page").clicked() {
            actions.push(Action::OpenUrl(up.artwork_upload_url.clone()));
        }
    });
    for plan in &plans {
        let idx = next();
        let file = plan.file_name(ext);
        step(ui, &mut done[idx], &format!("Upload {file}"), &mut |ui| {
            ui.label(widgets::faint("Give it any title, agree to the terms, and keep visibility Public."));
            ui.label(widgets::faint(&up.snippet_note));
            // Never ask anyone to paste code they can't read first.
            ui.add(egui::Label::new(RichText::new(&up.long_image_snippet).monospace().color(theme::TEXT)).wrap());
            if ui.small_button("Copy console snippet").clicked() {
                actions.push(Action::Copy { text: up.long_image_snippet.clone(), what: "console snippet" });
            }
        });
    }
    let i2 = next();
    step(ui, &mut done[i2], "Open your profile's showcase settings", &mut |ui| {
        if ui.small_button("Open showcase settings").clicked() {
            actions.push(Action::OpenUrl(up.showcase_edit_url.clone()));
        }
    });
    let i3 = next();
    let assign = plans.iter().map(|p| format!("{} in the {} slot", p.file_name(ext), p.region.label.to_lowercase())).collect::<Vec<_>>().join(", ");
    step(ui, &mut done[i3], &format!("Add “{}” and choose your pieces", layout.showcase_title), &mut |ui| {
        ui.label(widgets::faint(format!("Put {assign}. Then save.")));
    });
    if s.project.avatar.is_some() && s.assets.get(Slot::Avatar).is_some() {
        let i4 = next();
        step(ui, &mut done[i4], "Optional: upload your avatar", &mut |ui| {
            if ui.small_button("Open avatar settings").clicked() {
                actions.push(Action::OpenUrl(up.avatar_edit_url.clone()));
            }
        });
    }
    let i5 = next();
    step(ui, &mut done[i5], "Check the result", &mut |ui| {
        ui.label(widgets::faint("Compare with Preview.png. If the pieces don't line up, re-upload with the console snippet."));
        if ui.small_button("Open my profile").clicked() {
            actions.push(Action::OpenUrl(up.profile_url.clone()));
        }
    });

    s.upload_done = done;
    s.actions.extend(actions);
}

fn shortcuts(ctx: &egui::Context, s: &mut AppState) {
    let mut open = s.windows.shortcuts;
    egui::Window::new("Keyboard shortcuts").open(&mut open).collapsible(false).resizable(false).show(ctx, |ui| {
        egui::Grid::new("keys").num_columns(2).spacing([24.0, 6.0]).show(ui, |ui| {
            for (k, what) in [
                ("Ctrl+N / O / S", "New, open, save"),
                ("Ctrl+Shift+S", "Save as"),
                ("Ctrl+I", "Import artwork"),
                ("Ctrl+E", "Export"),
                ("Ctrl+Z / Ctrl+Shift+Z", "Undo / redo"),
                ("Ctrl+1 … 5", "Jump to a workflow step"),
                ("Drag", "Move artwork"),
                ("Drag a corner", "Scale artwork"),
                ("Drag the knob", "Rotate (Shift snaps to 15°)"),
                ("Shift+scroll", "Scale artwork"),
                ("Scroll", "Zoom the canvas"),
                ("Middle-drag or Space+drag", "Pan the canvas"),
                ("Arrows (Shift for 10 px)", "Nudge artwork"),
                ("Alt while dragging", "Bypass snapping"),
                ("Double-click canvas", "Fill all pieces"),
            ] {
                ui.label(RichText::new(k).color(theme::TEXT).monospace());
                ui.label(widgets::muted(what));
                ui.end_row();
            }
        });
    });
    s.windows.shortcuts = open;
}
