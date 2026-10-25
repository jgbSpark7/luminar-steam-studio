//! Simulated Steam profile. Structure and proportions follow Steam so artwork can be
//! judged in context; styling is an approximation, not a copy of Steam's UI.

use eframe::egui::{self, Align2, Color32, FontId, Mesh, Painter, Pos2, Rect, Sense, Stroke, Ui};
use eframe::emath::{pos2, vec2};

use super::{theme, widgets};
use crate::assets::{AssetStore, Slot};
use crate::editor::paint::{self, View};
use crate::export::slicer;
use crate::project::Project;
use crate::steam::profile::ProfileLayout;
use crate::steam::SteamConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewZoom {
    Fit,
    Actual,
}

pub struct PreviewState {
    pub zoom: PreviewZoom,
    pub outlines: bool,
}

impl Default for PreviewState {
    fn default() -> Self {
        Self { zoom: PreviewZoom::Fit, outlines: false }
    }
}

struct Ctx<'a> {
    painter: &'a Painter,
    view: View,
}

impl Ctx<'_> {
    fn rect(&self, r: Rect) -> Rect {
        self.view.rect(r)
    }
    fn text(&self, p: Pos2, align: Align2, text: &str, size: f32, color: Color32) {
        self.painter.text(self.view.to_screen(p), align, text, FontId::proportional(size * self.view.zoom), color);
    }
    fn fill(&self, r: Rect, rounding: f32, color: Color32) {
        self.painter.rect_filled(self.rect(r), rounding * self.view.zoom, color);
    }
}

pub fn show(ui: &mut Ui, st: &mut PreviewState, project: &Project, assets: &AssetStore, config: &SteamConfig) {
    widgets::toolbar_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            widgets::segmented(ui, &mut st.zoom, &[(PreviewZoom::Fit, "Fit"), (PreviewZoom::Actual, "100%")]);
            ui.toggle_value(&mut st.outlines, "Show pieces");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(widgets::faint("Simulated profile: layout and proportions follow Steam; styling is approximate."));
            });
        });
    });

    let layout = config.layout(&project.layout_id);
    let comp = layout.size(project.art_height.round());
    let pl = config.profile.arrange(comp);

    egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
        let avail = ui.available_width();
        let fit_width = pl.content.width() + 160.0;
        let zoom = match st.zoom {
            PreviewZoom::Fit => (avail / fit_width).min(1.25),
            PreviewZoom::Actual => 1.0,
        };
        let size = vec2(avail.max(fit_width * zoom), pl.page.height() * zoom);
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let painter = ui.painter_at(rect);
        let view = View { origin: pos2(rect.center().x, rect.top()), zoom };
        let c = Ctx { painter: &painter, view };
        draw_page(&c, rect, &pl, project, assets, config, st.outlines);
    });
}

fn draw_page(c: &Ctx, rect: Rect, pl: &ProfileLayout, p: &Project, assets: &AssetStore, config: &SteamConfig, outlines: bool) {
    let accent = theme::rgb(p.style.accent);
    let backdrop = theme::rgb(p.style.backdrop);
    let panel = Color32::from_black_alpha((p.style.panel_alpha * 255.0) as u8);
    let z = c.view.zoom;

    c.painter.rect_filled(rect, 0.0, backdrop);
    match (&p.background, assets.get(Slot::Background)) {
        (Some(bg), Some(asset)) => {
            let tint = Color32::from_gray(((1.0 - bg.dim) * 255.0) as u8);
            paint::placed_image(c.painter, &c.view, asset.texture.id(), asset.size(), &bg.transform(asset.size()), &Default::default(), tint);
        }
        _ => vertical_glow(c.painter, rect, accent.gamma_multiply(0.10), backdrop),
    }

    // Header
    c.fill(pl.header, 0.0, panel);
    let av = c.rect(pl.avatar);
    c.painter.rect_filled(av.expand(3.0 * z), 2.0, accent.gamma_multiply(0.55));
    match (&p.avatar, assets.get(Slot::Avatar)) {
        (Some(a), Some(asset)) => {
            let crop = a.crop_rect(asset.size());
            let uv = Rect::from_min_max(
                pos2(crop.min.x / asset.size().x, crop.min.y / asset.size().y),
                pos2(crop.max.x / asset.size().x, crop.max.y / asset.size().y),
            );
            if a.circular {
                c.painter.rect_filled(av.expand(3.0 * z), 2.0, panel);
                paint::textured_circle(c.painter, asset.texture.id(), av.center(), av.width() / 2.0, uv, Color32::WHITE);
            } else {
                c.painter.image(asset.texture.id(), av, uv, Color32::WHITE);
            }
        }
        _ => {
            c.painter.rect_filled(av, 0.0, theme::RAISED);
            let initial = p.profile.display_name.chars().next().unwrap_or('?').to_uppercase().to_string();
            c.painter.text(av.center(), Align2::CENTER_CENTER, initial, FontId::proportional(64.0 * z), theme::FAINT);
        }
    }
    c.text(pl.name_pos, Align2::LEFT_TOP, &p.profile.display_name, 24.0, Color32::WHITE);
    for (i, w) in [220.0, 160.0].iter().enumerate() {
        let r = Rect::from_min_size(pl.name_pos + vec2(0.0, 44.0 + i as f32 * 16.0), vec2(*w, 7.0));
        c.fill(r, 3.0, Color32::from_white_alpha(18));
    }
    c.text(pl.level_pos, Align2::LEFT_TOP, "Level", 22.0, Color32::WHITE);
    let ring = c.view.to_screen(pl.level_pos + vec2(86.0, 14.0));
    c.painter.circle(ring, 15.0 * z, Color32::from_black_alpha(80), Stroke::new(2.0 * z, accent));
    c.painter.text(ring, Align2::CENTER_CENTER, p.profile.level.to_string(), FontId::proportional(13.0 * z), Color32::WHITE);
    let edit = Rect::from_min_size(pl.level_pos + vec2(0.0, 120.0), vec2(140.0, 30.0));
    c.fill(edit, 2.0, Color32::from_white_alpha(22));
    c.text(edit.center(), Align2::CENTER_CENTER, "Edit Profile", 12.0, theme::MUTED);

    // Showcase
    let layout = config.layout(&p.layout_id);
    c.fill(pl.showcase_header, 0.0, panel);
    c.text(pl.showcase_header.left_center() + vec2(12.0, 0.0), Align2::LEFT_CENTER, &layout.showcase_title, 14.0, theme::TEXT);
    c.fill(Rect::from_min_max(pos2(pl.showcase.min.x, pl.showcase_header.max.y), pl.showcase.max), 0.0, panel.gamma_multiply(0.7));

    let comp_view = View { origin: c.view.to_screen(pl.comp_origin), zoom: z };
    let art = assets.get(Slot::Artwork).filter(|_| p.artwork.is_some());
    let ext = p.export.format.extension();
    let scale = p.export.scale.resolve(p.artwork.as_ref().map(|a| a.transform.scale));
    for (i, plan) in slicer::plan(layout, p.art_height, scale).iter().enumerate() {
        let sr = comp_view.rect(plan.display);
        if p.export.fill_transparent {
            c.painter.rect_filled(sr, 0.0, theme::rgb(p.style.canvas_fill));
        }
        match (&p.artwork, art) {
            (Some(a), Some(asset)) => {
                let clip = c.painter.with_clip_rect(sr.intersect(c.painter.clip_rect()));
                paint::placed_image(&clip, &comp_view, asset.texture.id(), asset.size(), &a.transform, &a.crop, Color32::WHITE);
            }
            _ => {
                c.painter.rect_filled(sr, 0.0, theme::RAISED.gamma_multiply(0.8));
                c.painter.text(sr.center(), Align2::CENTER_CENTER, &plan.region.label, FontId::proportional(12.0 * z), theme::FAINT);
            }
        }
        if outlines {
            let color = theme::region_color(i);
            paint::dashed_rect(c.painter, sr, Stroke::new(1.5, color), 6.0, 4.0);
            paint::pill(c.painter, sr.left_top() + vec2(6.0, 6.0), Align2::LEFT_TOP, &plan.file_name(ext), theme::ON_LUMEN, color);
        }
    }

    // Recent activity placeholder
    let act = pl.activity;
    c.fill(Rect::from_min_size(act.min, vec2(act.width(), 34.0)), 0.0, panel);
    c.text(act.min + vec2(12.0, 17.0), Align2::LEFT_CENTER, "Recent Activity", 14.0, theme::TEXT);
    c.fill(Rect::from_min_max(act.min + vec2(0.0, 34.0), act.max), 0.0, panel.gamma_multiply(0.7));
    for i in 0..3 {
        let top = act.min + vec2(12.0, 48.0 + i as f32 * 60.0);
        c.fill(Rect::from_min_size(top, vec2(92.0, 44.0)), 2.0, Color32::from_white_alpha(14));
        c.fill(Rect::from_min_size(top + vec2(108.0, 6.0), vec2(180.0 - i as f32 * 30.0, 8.0)), 3.0, Color32::from_white_alpha(26));
        c.fill(Rect::from_min_size(top + vec2(108.0, 24.0), vec2(110.0, 6.0)), 3.0, Color32::from_white_alpha(14));
    }

    draw_right_column(c, pl, p, accent, panel);
}

fn draw_right_column(c: &Ctx, pl: &ProfileLayout, p: &Project, accent: Color32, panel: Color32) {
    let rc = pl.right_col;
    c.fill(rc, 0.0, panel);
    let x = rc.min.x + 16.0;
    let mut y = rc.min.y + 18.0;

    c.text(pos2(x, y), Align2::LEFT_TOP, &p.profile.status, 17.0, accent);
    y += 52.0;

    let header = |label: &str, count: &str, y: f32| {
        c.text(pos2(x, y), Align2::LEFT_TOP, label, 13.0, theme::TEXT);
        if !count.is_empty() {
            let w = label.len() as f32 * 7.2 + 8.0;
            c.text(pos2(x + w, y - 3.0), Align2::LEFT_TOP, count, 17.0, theme::FAINT);
        }
    };

    header("Profile Awards", "8", y);
    y += 26.0;
    for i in 0..4 {
        let center = c.view.to_screen(pos2(x + 14.0 + i as f32 * 38.0, y + 14.0));
        c.painter.circle_filled(center, 12.0 * c.view.zoom, accent.gamma_multiply(0.35 + i as f32 * 0.12));
    }
    y += 46.0;

    header("Badges", "12", y);
    y += 26.0;
    for i in 0..4 {
        let r = Rect::from_min_size(pos2(x + i as f32 * 42.0, y), vec2(32.0, 32.0));
        c.fill(r, 3.0, Color32::from_white_alpha(20 + i * 10));
        c.painter.rect_stroke(c.rect(r), 3.0 * c.view.zoom, Stroke::new(1.0, accent.gamma_multiply(0.4)));
    }
    y += 60.0;

    for (label, count) in [
        ("Games", "4"),
        ("Inventory", ""),
        ("Screenshots", "4"),
        ("Videos", ""),
        ("Workshop Items", "19"),
        ("Reviews", "1"),
        ("Guides", ""),
        ("Artwork", "39"),
    ] {
        header(label, count, y);
        y += 26.0;
    }
    y += 22.0;
    header("Groups", "2", y);
    y += 30.0;
    for (name, members) in [("LuminarCore", "1,204 Members"), ("Artwork Profiles", "38,500 Members")] {
        c.fill(Rect::from_min_size(pos2(x, y), vec2(32.0, 32.0)), 2.0, accent.gamma_multiply(0.3));
        c.text(pos2(x + 42.0, y + 2.0), Align2::LEFT_TOP, name, 12.0, theme::TEXT);
        c.text(pos2(x + 42.0, y + 18.0), Align2::LEFT_TOP, members, 11.0, theme::FAINT);
        y += 44.0;
    }
    y += 12.0;
    header("Friends", "17", y);
}

fn vertical_glow(painter: &Painter, rect: Rect, top: Color32, bottom: Color32) {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.right_top(), top);
    let mid = rect.top() + rect.height().min(900.0);
    mesh.colored_vertex(pos2(rect.right(), mid), bottom);
    mesh.colored_vertex(pos2(rect.left(), mid), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(egui::Shape::mesh(mesh));
}
