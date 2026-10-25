use eframe::egui::{self, Align2, Color32, DragValue, Frame, Margin, RichText, Sense, Slider, Stroke, Ui};
use eframe::emath::{pos2, vec2, Rect};

use super::{theme, widgets};
use crate::app::{Action, AppState, Panel, Step};
use crate::assets::Slot;
use crate::editor::paint;
use crate::errors::display_name;
use crate::export::slicer;
use crate::project::transform::FitMode;
use crate::project::{Crop, ExportFormat, ExportScale};
use crate::steam::config::ConfigSource;
use crate::steam::validation::Severity;
use crate::templates::TEMPLATES;

pub fn show(ctx: &egui::Context, s: &mut AppState) {
    let frame = Frame::none()
        .fill(theme::PANEL)
        .inner_margin(Margin::symmetric(14.0, 12.0))
        .stroke(Stroke::new(1.0, theme::LINE));
    egui::SidePanel::right("inspector").exact_width(320.0).resizable(false).frame(frame).show(ctx, |ui| {
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let mut actions = Vec::new();
            if s.step == Step::Export {
                export_settings(ui, s, &mut actions);
            } else {
                match s.panel {
                    Panel::Artwork => artwork(ui, s, &mut actions),
                    Panel::Background => background(ui, s, &mut actions),
                    Panel::Avatar => avatar(ui, s, &mut actions),
                    Panel::Showcase => showcase(ui, s, &mut actions),
                    Panel::Templates => templates(ui, s, &mut actions),
                    Panel::Profile => profile(ui, s),
                }
            }
            checks(ui, s, &mut actions);
            s.actions.extend(actions);
        });
    });
}

fn header(ui: &mut Ui, title: &str, subtitle: &str) {
    ui.label(RichText::new(title).size(18.0).color(theme::TEXT).strong());
    ui.label(widgets::muted(subtitle));
    ui.add_space(4.0);
}

fn empty_state(ui: &mut Ui, text: &str, button: &str, slot: Slot, actions: &mut Vec<Action>) {
    widgets::card().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(widgets::muted(text));
        ui.add_space(6.0);
        if widgets::primary_button(ui, button, true).clicked() {
            actions.push(Action::ImportDialog(slot));
        }
        ui.label(widgets::faint("Or drop a file onto the window."));
    });
}

fn asset_row(ui: &mut Ui, s: &AppState, slot: Slot, actions: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        match s.assets.get(slot) {
            Some(a) => {
                widgets::thumbnail(ui, &a.texture, vec2(72.0, 72.0));
            }
            None => {
                let (r, _) = ui.allocate_exact_size(vec2(72.0, 72.0), Sense::hover());
                ui.painter().rect_filled(r, 6.0, theme::VOID);
                let msg = if s.assets.has_failed(slot) { "missing" } else { "loading…" };
                ui.painter().text(r.center(), Align2::CENTER_CENTER, msg, theme::small(), theme::FAINT);
            }
        }
        ui.vertical(|ui| {
            if let Some(path) = slot.path_in(&s.project) {
                ui.add(egui::Label::new(RichText::new(display_name(path)).color(theme::TEXT)).truncate())
                    .on_hover_text(path.display().to_string());
            }
            if let Some(a) = s.assets.get(slot) {
                ui.label(widgets::faint(format!("{}  {:.1} MB", a.info.summary(), a.info.file_bytes as f64 / 1_048_576.0)));
            }
            ui.horizontal(|ui| {
                if ui.small_button("Replace…").clicked() {
                    actions.push(Action::ImportDialog(slot));
                }
                if s.assets.has_failed(slot) && ui.small_button("Retry").clicked() {
                    actions.push(Action::ReloadAsset(slot));
                }
                if ui.small_button("Remove").clicked() {
                    actions.push(Action::RemoveAsset(slot));
                }
            });
        });
    });
}

fn artwork(ui: &mut Ui, s: &mut AppState, actions: &mut Vec<Action>) {
    header(ui, "Artwork", "Position it once. The pieces are cut for you.");
    if s.project.artwork.is_none() {
        empty_state(ui, "No artwork yet. Choose the image you want across your showcase.", "Choose artwork…", Slot::Artwork, actions);
        return;
    }
    asset_row(ui, s, Slot::Artwork, actions);
    let (min_h, max_h) = {
        let l = s.layout();
        (l.min_height, l.max_height)
    };
    let art = s.project.artwork.as_mut().expect("checked above");

    widgets::section(ui, "Position");
    let t = &mut art.transform;
    widgets::value_row(ui, "Center X", |ui| ui.add(DragValue::new(&mut t.center[0]).speed(1.0).suffix(" px").max_decimals(1)));
    widgets::value_row(ui, "Center Y", |ui| ui.add(DragValue::new(&mut t.center[1]).speed(1.0).suffix(" px").max_decimals(1)));
    let mut pct = t.scale * 100.0;
    if widgets::value_row(ui, "Scale", |ui| ui.add(DragValue::new(&mut pct).speed(0.25).range(0.5..=5000.0).suffix(" %").max_decimals(1))).changed() {
        t.scale = pct / 100.0;
    }
    widgets::value_row(ui, "Rotation", |ui| {
        let r = ui.add(DragValue::new(&mut t.rotation_deg).speed(0.25).range(-180.0..=180.0).suffix("°").max_decimals(1));
        if ui.small_button("−90").clicked() {
            t.rotation_deg = wrap_deg(t.rotation_deg - 90.0);
        }
        if ui.small_button("+90").clicked() {
            t.rotation_deg = wrap_deg(t.rotation_deg + 90.0);
        }
        r
    });
    ui.checkbox(&mut t.flip_x, "Flip horizontally");

    widgets::section(ui, "Fit");
    egui::Grid::new("fit_grid").num_columns(3).spacing(vec2(6.0, 6.0)).show(ui, |ui| {
        for (i, (label, action, tip)) in [
            ("Fill", Action::Fit(FitMode::Cover), "Cover every piece with no empty edges"),
            ("Fit width", Action::Fit(FitMode::FitWidth), "Match the layout's width"),
            ("Fit height", Action::Fit(FitMode::FitHeight), "Match the showcase height"),
            ("Show all", Action::Fit(FitMode::Contain), "Fit the whole image inside"),
            ("Center", Action::CenterArtwork, "Center at the current size"),
            ("Reset", Action::ResetArtwork, "Clear crop, rotation and flip"),
        ]
        .into_iter()
        .enumerate()
        {
            if ui.add_sized(vec2(88.0, 26.0), egui::Button::new(label)).on_hover_text(tip).clicked() {
                actions.push(action);
            }
            if i % 3 == 2 {
                ui.end_row();
            }
        }
    });

    widgets::section(ui, "Crop");
    ui.label(widgets::faint("Trim the source without changing its position."));
    let c = &mut art.crop;
    for (label, v) in [("Left", &mut c.left), ("Top", &mut c.top), ("Right", &mut c.right), ("Bottom", &mut c.bottom)] {
        let mut p = *v * 100.0;
        if widgets::value_row(ui, label, |ui| ui.add(Slider::new(&mut p, 0.0..=Crop::MAX_SIDE * 100.0).suffix(" %").max_decimals(1))).changed() {
            *v = p / 100.0;
        }
    }
    if !c.is_none() && ui.small_button("Clear crop").clicked() {
        *c = Crop::default();
    }

    widgets::section(ui, "Showcase height");
    ui.label(widgets::faint("Shared by every piece. You can also drag the handle under the canvas."));
    widgets::value_row(ui, "Height", |ui| {
        ui.add(DragValue::new(&mut s.project.art_height).speed(2.0).range(min_h..=max_h).suffix(" px").max_decimals(0))
    });
    s.project.art_height = s.project.art_height.round();
    ui.horizontal(|ui| {
        if ui.button("Match image").on_hover_text("Use the image's own proportions").clicked() {
            actions.push(Action::MatchHeightToArtwork);
        }
        for preset in [600.0, 800.0, 1000.0] {
            if ui.small_button(format!("{preset}")).clicked() {
                s.project.art_height = preset;
            }
        }
    });
}

fn wrap_deg(d: f32) -> f32 {
    let d = (d + 180.0).rem_euclid(360.0) - 180.0;
    if d == -180.0 { 180.0 } else { d }
}

fn background(ui: &mut Ui, s: &mut AppState, actions: &mut Vec<Action>) {
    header(ui, "Background", "Preview your art on the profile background you own.");
    if s.project.background.is_none() {
        empty_state(ui, "Add a screenshot or export of your Steam profile background.", "Choose background…", Slot::Background, actions);
        ui.label(widgets::faint("Steam backgrounds come from the Points Shop or Market and can't be uploaded. This is for previewing and alignment."));
        return;
    }
    asset_row(ui, s, Slot::Background, actions);
    let bg = s.project.background.as_mut().expect("checked above");
    widgets::section(ui, "Placement");
    let mut pct = bg.scale * 100.0;
    if widgets::value_row(ui, "Scale", |ui| ui.add(DragValue::new(&mut pct).speed(0.25).range(5.0..=800.0).suffix(" %").max_decimals(1))).changed() {
        bg.scale = pct / 100.0;
    }
    widgets::value_row(ui, "Offset X", |ui| ui.add(DragValue::new(&mut bg.offset[0]).speed(1.0).suffix(" px")));
    widgets::value_row(ui, "Offset Y", |ui| ui.add(DragValue::new(&mut bg.offset[1]).speed(1.0).suffix(" px")));
    widgets::value_row(ui, "Darken", |ui| ui.add(Slider::new(&mut bg.dim, 0.0..=0.9).show_value(true)));
    ui.checkbox(&mut bg.show_in_editor, "Show behind the editor canvas");
    if ui.button("Fit to 1920 px wide").on_hover_text("Steam backgrounds are usually 1920 px wide").clicked() {
        actions.push(Action::FitBackgroundToReference);
    }
}

fn avatar(ui: &mut Ui, s: &mut AppState, actions: &mut Vec<Action>) {
    header(ui, "Avatar", "Optional. Exported square, ready to upload.");
    if s.project.avatar.is_none() {
        empty_state(ui, "Add an avatar to see it in the profile preview.", "Choose avatar…", Slot::Avatar, actions);
        return;
    }
    asset_row(ui, s, Slot::Avatar, actions);
    let export_size = s.config.avatar.export_size;
    let av = s.project.avatar.as_mut().expect("checked above");

    if let Some(asset) = s.assets.get(Slot::Avatar) {
        ui.add_space(8.0);
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 132.0), Sense::hover());
        let box_rect = Rect::from_center_size(rect.center(), vec2(124.0, 124.0));
        let crop = av.crop_rect(asset.size());
        let uv = Rect::from_min_max(
            pos2(crop.min.x / asset.size().x, crop.min.y / asset.size().y),
            pos2(crop.max.x / asset.size().x, crop.max.y / asset.size().y),
        );
        if av.circular {
            paint::textured_circle(ui.painter(), asset.texture.id(), box_rect.center(), 62.0, uv, Color32::WHITE);
        } else {
            ui.painter().image(asset.texture.id(), box_rect, uv, Color32::WHITE);
            ui.painter().rect_stroke(box_rect, 0.0, Stroke::new(1.0, theme::LINE));
        }
    }
    widgets::section(ui, "Framing");
    widgets::value_row(ui, "Zoom", |ui| ui.add(Slider::new(&mut av.zoom, 1.0..=4.0).max_decimals(2)));
    widgets::value_row(ui, "Horizontal", |ui| ui.add(Slider::new(&mut av.offset[0], -1.0..=1.0).max_decimals(2)));
    widgets::value_row(ui, "Vertical", |ui| ui.add(Slider::new(&mut av.offset[1], -1.0..=1.0).max_decimals(2)));
    ui.checkbox(&mut av.circular, "Circular preview");
    ui.label(widgets::faint(format!("Exports as {export_size} × {export_size} px.")));
}

fn showcase(ui: &mut Ui, s: &mut AppState, actions: &mut Vec<Action>) {
    header(ui, "Showcase", "Choose the Steam layout. Sizes are handled for you.");
    for (li, layout) in s.config.layouts.iter().enumerate() {
        let selected = layout.id == s.project.layout_id;
        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 104.0), Sense::click());
        let p = ui.painter();
        p.rect_filled(rect, 8.0, if selected { theme::HOVER } else { theme::RAISED });
        p.rect_stroke(rect, 8.0, Stroke::new(1.0, if selected { theme::LUMEN } else if resp.hovered() { theme::FAINT } else { theme::LINE }));
        // Proportional mini diagram of the regions.
        let diagram = Rect::from_min_size(rect.min + vec2(12.0, 12.0), vec2(64.0, 80.0));
        let k = diagram.width() / layout.width();
        for (i, (_, r)) in layout.region_rects(1.0).enumerate() {
            let rr = Rect::from_min_size(pos2(diagram.left() + r.left() * k, diagram.top()), vec2(r.width() * k, diagram.height()));
            p.rect_filled(rr, 1.0, theme::region_color(i).gamma_multiply(if selected { 0.55 } else { 0.25 }));
        }
        let tx = diagram.right() + 12.0;
        p.text(pos2(tx, rect.top() + 14.0), Align2::LEFT_TOP, &layout.name, theme::body(), theme::TEXT);
        p.text(pos2(tx, rect.top() + 34.0), Align2::LEFT_TOP, layout.widths_summary(), theme::small(), theme::LUMEN);
        let galley = p.layout(layout.description.clone(), theme::small(), theme::MUTED, rect.right() - tx - 12.0);
        p.galley(pos2(tx, rect.top() + 54.0), galley, theme::MUTED);
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            actions.push(Action::SetLayout(layout.id.clone()));
        }
        if li + 1 < s.config.layouts.len() {
            ui.add_space(6.0);
        }
    }

    widgets::section(ui, "Pieces");
    let scale = s.project.export.scale.resolve(s.project.artwork.as_ref().map(|a| a.transform.scale));
    let ext = s.project.export.format.extension();
    egui::Grid::new("pieces").num_columns(2).spacing(vec2(10.0, 4.0)).show(ui, |ui| {
        for plan in slicer::plan(s.layout(), s.project.art_height, scale) {
            ui.label(RichText::new(plan.file_name(ext)).color(theme::TEXT));
            ui.label(widgets::faint(format!("{} × {} px", plan.pixel_width, plan.pixel_height)));
            ui.end_row();
        }
    });
    ui.label(widgets::faint(format!("At {scale}× export. Steam shows each piece at its layout width.")));

    widgets::section(ui, "Steam settings");
    let src = match &s.config_source {
        ConfigSource::Embedded => "Built-in sizes".to_string(),
        ConfigSource::File(p) => format!("Custom file: {}", display_name(p)),
    };
    ui.label(widgets::muted(src));
    if ui.small_button("Manage in Settings").clicked() {
        s.windows.settings = true;
    }
}

fn templates(ui: &mut Ui, s: &mut AppState, actions: &mut Vec<Action>) {
    header(ui, "Templates", "Starting points for layout and colour. Everything stays editable.");
    let w = (ui.available_width() - 8.0) / 2.0;
    egui::Grid::new("templates").num_columns(2).spacing(vec2(8.0, 8.0)).show(ui, |ui| {
        for (i, t) in TEMPLATES.iter().enumerate() {
            let selected = s.project.style.template_id == t.id;
            let (rect, resp) = ui.allocate_exact_size(vec2(w, 76.0), Sense::click());
            let p = ui.painter();
            p.rect_filled(rect, 8.0, theme::rgb(t.backdrop));
            let accent = theme::rgb(t.accent);
            p.circle_filled(rect.right_top() + vec2(-18.0, 18.0), 7.0, accent);
            p.rect_filled(Rect::from_min_size(rect.left_bottom() + vec2(10.0, -14.0), vec2(w - 20.0, 3.0)), 1.5, accent.gamma_multiply(0.5));
            p.text(rect.left_top() + vec2(10.0, 10.0), Align2::LEFT_TOP, t.name, theme::body(), Color32::WHITE);
            p.text(rect.left_top() + vec2(10.0, 30.0), Align2::LEFT_TOP, t.tagline, theme::small(), Color32::from_white_alpha(150));
            let stroke = if selected { Stroke::new(1.5, theme::LUMEN) } else if resp.hovered() { Stroke::new(1.0, theme::FAINT) } else { Stroke::new(1.0, theme::LINE) };
            p.rect_stroke(rect, 8.0, stroke);
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                actions.push(Action::ApplyTemplate(t.id));
            }
            if i % 2 == 1 {
                ui.end_row();
            }
        }
    });

    widgets::section(ui, "Colours");
    let st = &mut s.project.style;
    ui.horizontal(|ui| {
        widgets::color_swatch(ui, &mut st.accent, "Accent");
        ui.label(widgets::muted("Accent"));
        ui.add_space(8.0);
        widgets::color_swatch(ui, &mut st.backdrop, "Page colour");
        ui.label(widgets::muted("Page"));
        ui.add_space(8.0);
        widgets::color_swatch(ui, &mut st.canvas_fill, "Fill for empty export areas");
        ui.label(widgets::muted("Fill"));
    });
    widgets::value_row(ui, "Panel opacity", |ui| ui.add(Slider::new(&mut st.panel_alpha, 0.0..=1.0).max_decimals(2)));
}

fn profile(ui: &mut Ui, s: &mut AppState) {
    header(ui, "Profile", "Details used by the preview only.");
    widgets::section(ui, "Project");
    widgets::value_row(ui, "Name", |ui| ui.text_edit_singleline(&mut s.project.name));
    widgets::section(ui, "Preview details");
    let pr = &mut s.project.profile;
    widgets::value_row(ui, "Display name", |ui| ui.text_edit_singleline(&mut pr.display_name));
    widgets::value_row(ui, "Level", |ui| ui.add(DragValue::new(&mut pr.level).range(0..=5000)));
    widgets::value_row(ui, "Status", |ui| {
        egui::ComboBox::from_id_salt("status")
            .selected_text(pr.status.clone())
            .show_ui(ui, |ui| {
                for opt in ["Currently Online", "Currently Offline", "Currently In-Game", "Away"] {
                    ui.selectable_value(&mut pr.status, opt.to_string(), opt);
                }
            })
            .response
    });
}

fn export_settings(ui: &mut Ui, s: &mut AppState, actions: &mut Vec<Action>) {
    header(ui, "Export settings", "Where and how your pieces are saved.");
    widgets::section(ui, "Destination");
    let folder = s.project.export.folder.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "Not chosen yet".into());
    ui.add(egui::Label::new(widgets::muted(folder)).wrap());
    if ui.button("Choose folder…").clicked() {
        actions.push(Action::ChooseExportFolder);
    }

    let has_avatar = s.assets.get(Slot::Avatar).is_some();
    let has_bg = s.assets.get(Slot::Background).is_some();
    let auto_scale = ExportScale::Auto.resolve(s.project.artwork.as_ref().map(|a| a.transform.scale));
    let e = &mut s.project.export;

    widgets::section(ui, "Format");
    widgets::segmented(ui, &mut e.format, &[(ExportFormat::Png, "PNG"), (ExportFormat::Jpeg, "JPEG")]);
    if e.format == ExportFormat::Jpeg {
        widgets::value_row(ui, "Quality", |ui| ui.add(Slider::new(&mut e.jpeg_quality, 60..=100)));
        ui.label(widgets::faint("JPEG has no transparency; empty areas are always filled."));
    }

    widgets::section(ui, "Resolution");
    let opts: Vec<(ExportScale, &str)> = ExportScale::ALL.iter().map(|s| (*s, s.label())).collect();
    widgets::segmented(ui, &mut e.scale, &opts);
    ui.label(widgets::faint(format!(
        "Auto uses the most detail your image supports (currently {auto_scale}×). Whole-number scales keep every piece aligned."
    )));

    widgets::section(ui, "Include");
    ui.add_enabled(has_avatar, egui::Checkbox::new(&mut e.include_avatar, "Avatar"));
    ui.checkbox(&mut e.include_preview, "Preview image");
    ui.checkbox(&mut e.include_project, "Project file");
    ui.add_enabled(has_bg, egui::Checkbox::new(&mut e.include_background, "Background reference"));
    ui.checkbox(&mut e.fill_transparent, "Fill empty and transparent areas");
    ui.checkbox(&mut e.make_zip, "Also create a ZIP");
}

fn checks(ui: &mut Ui, s: &AppState, actions: &mut Vec<Action>) {
    widgets::section(ui, "Steam artwork check");
    for c in &s.checks {
        if c.severity == Severity::Ok && s.step != Step::Export && s.panel != Panel::Showcase {
            // Keep the list short while editing; full list on export.
            continue;
        }
        let (glyph, color) = widgets::severity_glyph(c.severity);
        ui.horizontal_top(|ui| {
            ui.label(RichText::new(glyph).color(color));
            ui.vertical(|ui| {
                ui.label(RichText::new(&c.title).color(theme::TEXT));
                ui.add(egui::Label::new(widgets::faint(&c.detail)).wrap());
                if let Some(fix) = c.fix {
                    if ui.small_button(fix.label()).clicked() {
                        actions.push(Action::Fix(fix));
                    }
                }
            });
        });
        ui.add_space(2.0);
    }
    let ok = s.checks.iter().filter(|c| c.severity == Severity::Ok).count();
    if ok > 0 && s.step != Step::Export && s.panel != Panel::Showcase {
        ui.label(widgets::faint(format!("✔ {ok} checks passed")));
    }
}
