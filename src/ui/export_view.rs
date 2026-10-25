use eframe::egui::{self, RichText, Ui};

use super::{theme, widgets};
use crate::app::{Action, AppState};
use crate::assets::Slot;
use crate::export::pipeline::sanitize_folder_name;
use crate::export::slicer;
use crate::steam::validation::Severity;

pub fn show(ui: &mut Ui, s: &mut AppState) {
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        let width = (ui.available_width() - 72.0).clamp(320.0, 680.0);
        let side = ((ui.available_width() - width) / 2.0).max(0.0);
        ui.add_space(40.0);
        ui.horizontal(|ui| {
            ui.add_space(side);
            ui.vertical(|ui| {
                ui.set_width(width);
                body(ui, s);
            });
        });
    });
}

fn body(ui: &mut Ui, s: &mut AppState) {
    ui.label(RichText::new("Export profile artwork").size(26.0).color(theme::TEXT));
    ui.label(widgets::muted("Every piece Steam needs, cut, named and checked."));
    ui.add_space(18.0);

    let p = &s.project;
    let scale = p.export.scale.resolve(p.artwork.as_ref().map(|a| a.transform.scale));
    let ext = p.export.format.extension();
    let plans = slicer::plan(s.layout(), p.art_height, scale);

    widgets::card().show(ui, |ui| {
        ui.set_width(ui.available_width());
        let dest = match &p.export.folder {
            Some(f) => f.join(sanitize_folder_name(&p.name)).display().to_string(),
            None => "You'll choose a folder when you export".into(),
        };
        ui.label(widgets::faint("Destination"));
        ui.add(egui::Label::new(RichText::new(dest).color(theme::TEXT)).wrap());
        ui.add_space(10.0);
        ui.label(widgets::faint("Files"));
        egui::Grid::new("export_files").num_columns(3).spacing(egui::vec2(18.0, 6.0)).show(ui, |ui| {
            for plan in &plans {
                ui.label(RichText::new(plan.file_name(ext)).color(theme::TEXT));
                ui.label(widgets::muted(format!("{} × {} px", plan.pixel_width, plan.pixel_height)));
                ui.label(widgets::faint(format!("Upload as artwork ({})", plan.region.label.to_lowercase())));
                ui.end_row();
            }
            if p.export.include_avatar && s.assets.get(Slot::Avatar).is_some() {
                let a = s.config.avatar.export_size;
                ui.label(RichText::new(format!("Avatar.{ext}")).color(theme::TEXT));
                ui.label(widgets::muted(format!("{a} × {a} px")));
                ui.label(widgets::faint("Upload as your avatar"));
                ui.end_row();
            }
            if p.export.include_preview {
                ui.label(RichText::new("Preview.png").color(theme::TEXT));
                ui.label(widgets::muted("Showcase strip"));
                ui.label(widgets::faint("For sharing and checking alignment"));
                ui.end_row();
            }
            if p.export.include_background && s.assets.get(Slot::Background).is_some() {
                ui.label(RichText::new("Background.png").color(theme::TEXT));
                ui.label(widgets::muted("Original size"));
                ui.label(widgets::faint("Reference only"));
                ui.end_row();
            }
            if p.export.include_project {
                ui.label(RichText::new("Project.lss").color(theme::TEXT));
                ui.label(widgets::muted("Editable"));
                ui.label(widgets::faint("Reopen this design later"));
                ui.end_row();
            }
        });
        if p.export.make_zip {
            ui.add_space(4.0);
            ui.label(widgets::faint("A ZIP of the folder is created alongside it."));
        }
    });

    ui.add_space(16.0);
    let ready = s.assets.get(Slot::Artwork).is_some() && !s.export.running();
    ui.horizontal(|ui| {
        if widgets::primary_button(ui, "   Export profile artwork   ", ready).clicked() {
            s.push(Action::StartExport);
        }
        if s.project.export.folder.is_some() && ui.button("Change folder…").clicked() {
            s.push(Action::ChooseExportFolder);
        }
    });
    if s.project.artwork.is_none() {
        ui.label(widgets::faint("Import artwork first."));
    }
    let blocking = s.checks.iter().filter(|c| c.severity == Severity::Warning).count();
    if blocking > 0 {
        ui.label(RichText::new(format!("⚠ {blocking} warning(s) in the checks panel. You can still export.")).color(theme::WARN).size(12.0));
    }

    if s.export.running() {
        ui.add_space(12.0);
        ui.add(egui::ProgressBar::new(s.export.progress).text(s.export.label.clone()).animate(true));
    }

    if let Some(report) = s.export.report.clone() {
        ui.add_space(18.0);
        widgets::card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            let (title, color) = match report.worst() {
                Severity::Error => ("Export finished with problems", theme::ERR),
                Severity::Warning => ("Exported with warnings", theme::WARN),
                _ => ("Export complete", theme::OK),
            };
            ui.label(RichText::new(title).size(16.0).color(color).strong());
            ui.label(widgets::faint(format!("Rendered at {}× into {}", report.scale, report.folder.display())));
            ui.add_space(6.0);
            for line in &report.lines {
                let (glyph, c) = widgets::severity_glyph(line.severity);
                ui.horizontal_top(|ui| {
                    ui.label(RichText::new(glyph).color(c));
                    ui.add(egui::Label::new(RichText::new(&line.text).color(theme::TEXT)).wrap());
                });
            }
            ui.add_space(8.0);
            egui::Grid::new("written_files").num_columns(3).spacing(egui::vec2(18.0, 4.0)).show(ui, |ui| {
                for f in &report.files {
                    ui.label(RichText::new(&f.name).color(theme::TEXT));
                    ui.label(widgets::muted(if f.width > 0 { format!("{} × {} px", f.width, f.height) } else { String::new() }));
                    ui.label(widgets::faint(format!("{:.0} KB", f.bytes as f64 / 1024.0)));
                    ui.end_row();
                }
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if let Some(zip) = &report.zip {
                    if ui.button("Show ZIP").on_hover_text(zip.display().to_string()).clicked() {
                        if let Some(dir) = zip.parent() {
                            s.push(Action::OpenPath(dir.to_path_buf()));
                        }
                    }
                }
                if widgets::primary_button(ui, "Open folder", true).clicked() {
                    s.push(Action::OpenPath(report.folder.clone()));
                }
                if ui.button("Steam upload guide").clicked() {
                    s.windows.upload_guide = true;
                }
            });
        });
    }
}
