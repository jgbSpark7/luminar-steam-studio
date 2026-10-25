use eframe::egui::{self, Align2, Color32, FontId, Rect, RichText, Sense, Stroke, Ui};
use eframe::emath::{pos2, vec2};

use super::{theme, widgets};
use crate::app::{Action, AppState};
use crate::assets::Slot;
use crate::editor::paint;
use crate::errors::display_name;

struct CardSpec {
    slot: Slot,
    title: &'static str,
    blurb: &'static str,
    badge: &'static str,
}

const CARDS: [CardSpec; 3] = [
    CardSpec { slot: Slot::Artwork, title: "Artwork", blurb: "The image you want across your showcase. Bigger is better.", badge: "Required" },
    CardSpec { slot: Slot::Background, title: "Profile background", blurb: "The Steam background you own, for previewing and lining things up.", badge: "Optional" },
    CardSpec { slot: Slot::Avatar, title: "Avatar", blurb: "Cropped square and exported at Steam's avatar size.", badge: "Optional" },
];

pub fn show(ui: &mut Ui, s: &mut AppState) {
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        let width = (ui.available_width() - 72.0).clamp(320.0, 1040.0);
        let side = ((ui.available_width() - width) / 2.0).max(0.0);
        ui.add_space(48.0);
        ui.horizontal(|ui| {
            ui.add_space(side);
            ui.vertical(|ui| {
                ui.set_width(width);
                ui.label(RichText::new("Start with your artwork").size(26.0).color(theme::TEXT));
                ui.add_space(4.0);
                ui.label(widgets::muted(
                    "Drop images onto a card or click to choose. Steam's sizes, slicing and alignment are handled for you.",
                ));
                ui.add_space(24.0);
                cards(ui, s, width);
                ui.add_space(28.0);
                recent(ui, s);
            });
        });
    });
}

fn cards(ui: &mut Ui, s: &mut AppState, width: f32) {
    let gap = 14.0;
    // Leave room for the row's own item spacing so the last card never clips.
    let width = width - 2.0 * ui.spacing().item_spacing.x;
    let w_main = (width - 2.0 * gap) * 0.42;
    let w_small = (width - 2.0 * gap - w_main) / 2.0;
    let files_hovering = ui.input(|i| !i.raw.hovered_files.is_empty());
    let pointer = ui.input(|i| i.pointer.latest_pos());

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        for (i, card) in CARDS.iter().enumerate() {
            let w = if i == 0 { w_main } else { w_small };
            let (rect, resp) = ui.allocate_exact_size(vec2(w, 280.0), Sense::click());
            let pointer_here = pointer.is_some_and(|p| rect.contains(p));
            let hot = resp.hovered() || (files_hovering && pointer_here);
            draw_card(ui, s, card, rect, hot, i == 0);
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                s.push(Action::ImportDialog(card.slot));
            }
            if pointer_here && !s.dropped.is_empty() {
                let path = s.dropped.remove(0);
                s.dropped.clear();
                s.push(Action::ImportPath(card.slot, path));
            }
        }
    });
}

fn draw_card(ui: &Ui, s: &AppState, card: &CardSpec, rect: Rect, hot: bool, primary: bool) {
    let p = ui.painter();
    p.rect_filled(rect, 10.0, if hot { theme::HOVER } else { theme::RAISED });
    let border = if hot { theme::LUMEN } else if primary { theme::LUMEN.gamma_multiply(0.35) } else { theme::LINE };
    paint::dashed_rect(p, rect.shrink(0.5), Stroke::new(1.0, border), 6.0, 4.0);

    let badge_color = if primary { theme::LUMEN } else { theme::FAINT };
    p.text(rect.right_top() + vec2(-14.0, 14.0), Align2::RIGHT_TOP, card.badge, theme::small(), badge_color);

    let preview_area = Rect::from_min_max(rect.min + vec2(16.0, 36.0), pos2(rect.max.x - 16.0, rect.max.y - 88.0));
    match s.assets.get(card.slot) {
        Some(asset) => {
            let size = asset.size();
            let k = (preview_area.width() / size.x).min(preview_area.height() / size.y);
            let img = Rect::from_center_size(preview_area.center(), size * k);
            p.image(asset.texture.id(), img, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            paint::crop_marks(p, img, theme::LUMEN.gamma_multiply(0.6), 8.0, 4.0);
        }
        None if s.assets.is_loading(card.slot) => {
            p.text(preview_area.center(), Align2::CENTER_CENTER, "Loading…", theme::body(), theme::MUTED);
        }
        None => {
            let c = preview_area.center();
            p.circle_stroke(c, 26.0, Stroke::new(1.2, if hot { theme::LUMEN } else { theme::FAINT }));
            p.text(c, Align2::CENTER_CENTER, "+", FontId::proportional(26.0), if hot { theme::LUMEN } else { theme::MUTED });
        }
    }

    let text_top = rect.max.y - 78.0;
    p.text(pos2(rect.left() + 16.0, text_top), Align2::LEFT_TOP, card.title, FontId::proportional(15.0), theme::TEXT);
    let detail = match (s.assets.get(card.slot), card.slot.path_in(&s.project)) {
        (Some(a), Some(path)) => format!("{}\n{}", display_name(path), a.info.summary()),
        _ => card.blurb.to_string(),
    };
    let galley = p.layout(detail, theme::small(), theme::MUTED, rect.width() - 32.0);
    p.galley(pos2(rect.left() + 16.0, text_top + 22.0), galley, theme::MUTED);
}

fn recent(ui: &mut Ui, s: &mut AppState) {
    ui.horizontal(|ui| {
        if ui.button("Open project…").clicked() {
            s.push(Action::OpenProjectDialog);
        }
        if ui.button("New project").clicked() {
            s.push(Action::NewProject);
        }
        if s.assets.get(Slot::Artwork).is_some() && widgets::primary_button(ui, "Continue to editor", true).clicked() {
            s.push(Action::GoTo(crate::app::Step::Edit));
        }
    });
    if s.settings.recent.is_empty() {
        return;
    }
    widgets::section(ui, "Recent projects");
    for path in s.settings.recent.clone() {
        let resp = ui
            .horizontal(|ui| {
                let r = ui.link(RichText::new(display_name(&path)).color(theme::TEXT));
                ui.label(widgets::faint(path.parent().map(|p| p.display().to_string()).unwrap_or_default()));
                r
            })
            .inner;
        if resp.clicked() {
            s.push(Action::OpenProjectPath(path));
        }
    }
}
