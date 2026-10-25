use eframe::egui::{self, Frame, Margin, RichText, Stroke};

use super::widgets::NavIcon;
use super::{theme, widgets};
use crate::app::{Action, AppState, Panel, Step};
use crate::assets::Slot;
use crate::steam::validation::Severity;
use crate::templates;

pub fn show(ctx: &egui::Context, s: &mut AppState) {
    let frame = Frame::none()
        .fill(theme::PANEL)
        .inner_margin(Margin::symmetric(10.0, 12.0))
        .stroke(Stroke::new(1.0, theme::LINE));
    egui::SidePanel::left("sidebar").exact_width(236.0).resizable(false).frame(frame).show(ctx, |ui| {
        widgets::card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(&s.project.name).size(15.0).color(theme::TEXT).strong());
            let layout = s.layout();
            ui.label(widgets::muted(&layout.name));
            ui.label(widgets::faint(format!("{} wide, {} px tall", layout.widths_summary(), s.project.art_height as u32)));
        });

        widgets::section(ui, "Design");
        let detail_for = |slot: Slot| -> (String, bool) {
            match s.assets.get(slot) {
                Some(a) => (format!("{}×{}", a.info.width, a.info.height), true),
                None if s.assets.is_loading(slot) => ("loading".into(), false),
                None => ("add".into(), false),
            }
        };
        let rows: [(Panel, NavIcon, &str, String, bool); 6] = [
            { let (d, f) = detail_for(Slot::Artwork); (Panel::Artwork, NavIcon::Artwork, "Artwork", d, f) },
            { let (d, f) = detail_for(Slot::Background); (Panel::Background, NavIcon::Background, "Background", d, f) },
            { let (d, f) = detail_for(Slot::Avatar); (Panel::Avatar, NavIcon::Avatar, "Avatar", d, f) },
            (Panel::Showcase, NavIcon::Showcase, "Showcase", s.layout().widths_summary(), true),
            (
                Panel::Templates,
                NavIcon::Templates,
                "Templates",
                templates::find(&s.project.style.template_id).map(|t| t.name.to_string()).unwrap_or_default(),
                true,
            ),
            (Panel::Profile, NavIcon::Profile, "Profile", format!("Lv {}", s.project.profile.level), true),
        ];
        for (panel, icon, label, detail, filled) in rows {
            if widgets::nav_item(ui, icon, label, &detail, s.panel == panel && !s.info_open, filled).clicked() {
                s.push(Action::Focus(panel));
                if s.step == Step::Export {
                    s.push(Action::GoTo(Step::Edit));
                }
            }
        }

        widgets::section(ui, "App");
        let info = widgets::nav_item(ui, NavIcon::Info, "Info", crate::about::VERSION, s.info_open, true)
            .on_hover_text("About, credits and privacy");
        if info.clicked() {
            s.push(Action::ShowInfo);
        }

        widgets::section(ui, "Checks");
        let count = |sev: Severity| s.checks.iter().filter(|c| c.severity == sev).count();
        ui.horizontal(|ui| {
            for sev in [Severity::Ok, Severity::Warning, Severity::Error] {
                let (glyph, color) = widgets::severity_glyph(sev);
                ui.label(RichText::new(format!("{glyph} {}", count(sev))).color(color));
                ui.add_space(6.0);
            }
        });
        let issues: Vec<_> = s.checks.iter().filter(|c| c.severity >= Severity::Warning).take(3).cloned().collect();
        if issues.is_empty() {
            ui.label(widgets::faint("Nothing needs attention."));
        }
        for c in issues {
            let (glyph, color) = widgets::severity_glyph(c.severity);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(glyph).color(color));
                ui.label(RichText::new(&c.title).color(theme::TEXT).size(12.0));
            });
        }

        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            ui.label(widgets::faint("A LuminarCore utility"));
            ui.add_space(4.0);
            if ui.button("Steam upload guide").clicked() {
                s.windows.upload_guide = true;
            }
        });
    });
}
