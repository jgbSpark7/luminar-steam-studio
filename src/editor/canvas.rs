use eframe::egui::{self, Align2, Color32, CursorIcon, PointerButton, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use eframe::emath::{pos2, vec2};

use super::guides::{Guides, SnapLine};
use super::paint::{self, View};
use crate::app::Action;
use crate::assets::{AssetStore, Slot};
use crate::export::slicer;
use crate::project::transform::{bounds, point_in_quad, quad, FitMode};
use crate::project::Project;
use crate::steam::SteamConfig;
use crate::ui::{theme, widgets};

const MIN_ZOOM: f32 = 0.05;
const MAX_ZOOM: f32 = 8.0;
const HANDLE_RADIUS: f32 = 10.0;
const ROTATE_KNOB_OFFSET: f32 = 28.0;

pub struct CanvasState {
    pub zoom: f32,
    pub pan: Vec2,
    pub needs_fit: bool,
    pub snapping: bool,
    pub show_guides: bool,
    pub show_background: bool,
    /// Pointer position in composition space, for the status bar.
    pub hover: Option<Pos2>,
    drag: Option<DragOp>,
    snap_lines: Vec<SnapLine>,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: Vec2::ZERO,
            needs_fit: true,
            snapping: true,
            show_guides: true,
            show_background: true,
            hover: None,
            drag: None,
            snap_lines: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum DragOp {
    Move { start_center: Pos2, start_pointer: Pos2 },
    Scale { start_scale: f32, start_dist: f32 },
    Rotate { start_rotation: f32, start_angle: f32 },
    Height { start_height: f32, start_pointer_y: f32 },
    Pan { start_pan: Vec2, start_pointer: Pos2 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Hit {
    Nothing,
    Body,
    Corner,
    Rotate,
    Height,
}

fn view_for(st: &CanvasState, rect: Rect, comp: Vec2) -> View {
    View { origin: rect.center() + st.pan - comp * st.zoom / 2.0, zoom: st.zoom }
}

pub fn fit_view(st: &mut CanvasState, rect: Rect, comp: Vec2) {
    let margin = vec2(120.0, 110.0);
    let avail = (rect.size() - margin).max(Vec2::splat(50.0));
    st.zoom = (avail.x / comp.x).min(avail.y / comp.y).clamp(MIN_ZOOM, 2.0);
    st.pan = vec2(0.0, 6.0);
}

fn zoom_about(st: &mut CanvasState, rect: Rect, comp: Vec2, pointer: Pos2, factor: f32) {
    let view = view_for(st, rect, comp);
    let space = view.to_space(pointer);
    let zoom = (st.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
    let origin = pointer - space.to_vec2() * zoom;
    st.pan = origin - rect.center() + comp * zoom / 2.0;
    st.zoom = zoom;
}

fn rotate_knob(q: &[Pos2; 4]) -> Pos2 {
    let top_mid = q[0] + (q[1] - q[0]) / 2.0;
    let centroid = pos2((q[0].x + q[2].x) / 2.0, (q[0].y + q[2].y) / 2.0);
    let dir = (top_mid - centroid).normalized();
    top_mid + dir * ROTATE_KNOB_OFFSET
}

fn height_handle(view: &View, comp: Vec2) -> Rect {
    Rect::from_center_size(view.to_screen(pos2(comp.x / 2.0, comp.y)), vec2(52.0, 14.0))
}

fn hit_test(view: &View, comp: Vec2, art_quad: Option<&[Pos2; 4]>, p: Pos2) -> Hit {
    if height_handle(view, comp).expand(2.0).contains(p) {
        return Hit::Height;
    }
    if let Some(q) = art_quad {
        if q.iter().any(|c| c.distance(p) <= HANDLE_RADIUS) {
            return Hit::Corner;
        }
        if rotate_knob(q).distance(p) <= HANDLE_RADIUS {
            return Hit::Rotate;
        }
        if point_in_quad(q, p) {
            return Hit::Body;
        }
    }
    Hit::Nothing
}

pub fn show_editor(
    ui: &mut Ui,
    st: &mut CanvasState,
    project: &mut Project,
    assets: &AssetStore,
    config: &SteamConfig,
    actions: &mut Vec<Action>,
) {
    let art_asset = assets.get(Slot::Artwork).filter(|_| project.artwork.is_some());
    toolbar(ui, st, project, config, actions, art_asset.is_some());

    let (rect, resp) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
    let layout = config.layout(&project.layout_id);
    let comp = layout.size(project.art_height);
    if st.needs_fit {
        fit_view(st, rect, comp);
        st.needs_fit = false;
    }

    let (scroll, zoom_delta, mods, hover_pos, space_down) = ui.input(|i| {
        (i.smooth_scroll_delta, i.zoom_delta(), i.modifiers, i.pointer.hover_pos(), i.key_down(egui::Key::Space))
    });

    // Wheel: zoom the view. Shift+wheel: scale the artwork.
    if resp.hovered() {
        if mods.shift {
            if let Some(art) = project.artwork.as_mut().filter(|_| art_asset.is_some()) {
                let amount = scroll.x + scroll.y;
                if amount != 0.0 {
                    art.transform.scale = (art.transform.scale * (amount * 0.002).exp()).clamp(0.005, 50.0);
                }
            }
        } else if let Some(p) = hover_pos {
            let factor = zoom_delta * (scroll.y * 0.0018).exp();
            if (factor - 1.0).abs() > 1e-4 {
                zoom_about(st, rect, comp, p, factor);
            }
        }
    }

    let view = view_for(st, rect, comp);
    let image_size = art_asset.map(|a| a.size());
    let screen_quad = |p: &Project| -> Option<[Pos2; 4]> {
        let a = p.artwork.as_ref()?;
        Some(quad(&a.transform, image_size?, &a.crop).map(|c| view.to_screen(c)))
    };

    // Start a gesture.
    if resp.drag_started() {
        let pointer = ui.input(|i| i.pointer.press_origin()).or(resp.interact_pointer_pos()).unwrap_or(rect.center());
        let pan_gesture = resp.dragged_by(PointerButton::Middle) || (space_down && resp.dragged_by(PointerButton::Primary));
        st.drag = if pan_gesture {
            Some(DragOp::Pan { start_pan: st.pan, start_pointer: pointer })
        } else if resp.dragged_by(PointerButton::Primary) {
            let q = screen_quad(project);
            match (hit_test(&view, comp, q.as_ref(), pointer), &project.artwork, image_size) {
                (Hit::Height, _, _) => Some(DragOp::Height { start_height: project.art_height, start_pointer_y: pointer.y }),
                (Hit::Corner, Some(a), Some(_)) => {
                    let c = view.to_screen(a.transform.center_pos());
                    Some(DragOp::Scale { start_scale: a.transform.scale, start_dist: c.distance(pointer).max(1.0) })
                }
                (Hit::Rotate, Some(a), Some(_)) => {
                    let c = view.to_screen(a.transform.center_pos());
                    Some(DragOp::Rotate { start_rotation: a.transform.rotation_deg, start_angle: (pointer - c).angle() })
                }
                // With a single layer, dragging anywhere moves it: no need to aim.
                (Hit::Body | Hit::Nothing, Some(a), Some(_)) => {
                    Some(DragOp::Move { start_center: a.transform.center_pos(), start_pointer: pointer })
                }
                _ => None,
            }
        } else {
            None
        };
    }

    // Continue a gesture.
    if let (Some(op), Some(p)) = (st.drag, resp.interact_pointer_pos()) {
        st.snap_lines.clear();
        let snap_enabled = st.snapping && !mods.alt;
        match op {
            DragOp::Pan { start_pan, start_pointer } => st.pan = start_pan + (p - start_pointer),
            DragOp::Height { start_height, start_pointer_y } => {
                let raw = start_height + (p.y - start_pointer_y) / view.zoom;
                let stepped = if snap_enabled { (raw / 10.0).round() * 10.0 } else { raw.round() };
                let new_h = layout.clamp_height(stepped);
                // Keep the top edge still while the bottom moves.
                st.pan.y += (new_h - project.art_height) * view.zoom / 2.0;
                project.art_height = new_h;
            }
            DragOp::Move { start_center, start_pointer } => {
                if let (Some(a), Some(size)) = (project.artwork.as_mut(), image_size) {
                    let mut center = start_center + (p - start_pointer) / view.zoom;
                    a.transform.set_center(center);
                    if snap_enabled {
                        let b = bounds(&quad(&a.transform, size, &a.crop));
                        let guides = Guides::for_layout(layout, project.art_height);
                        let (delta, lines) = guides.snap(b, 7.0 / view.zoom);
                        center += delta;
                        st.snap_lines = lines;
                    }
                    a.transform.set_center(center);
                }
            }
            DragOp::Scale { start_scale, start_dist } => {
                if let Some(a) = project.artwork.as_mut() {
                    let c = view.to_screen(a.transform.center_pos());
                    a.transform.scale = (start_scale * c.distance(p) / start_dist).clamp(0.005, 50.0);
                }
            }
            DragOp::Rotate { start_rotation, start_angle } => {
                if let Some(a) = project.artwork.as_mut() {
                    let c = view.to_screen(a.transform.center_pos());
                    let mut deg = start_rotation + ((p - c).angle() - start_angle).to_degrees();
                    if mods.shift {
                        deg = (deg / 15.0).round() * 15.0;
                    } else if snap_enabled {
                        let nearest = (deg / 90.0).round() * 90.0;
                        if (deg - nearest).abs() < 2.5 {
                            deg = nearest;
                        }
                    }
                    a.transform.rotation_deg = normalize_degrees(deg);
                }
            }
        }
    }
    if resp.drag_stopped() {
        st.drag = None;
        st.snap_lines.clear();
    }

    if resp.double_clicked() && art_asset.is_some() {
        actions.push(Action::Fit(FitMode::Cover));
    } else if resp.clicked() && art_asset.is_none() && project.artwork.is_none() {
        actions.push(Action::ImportDialog(Slot::Artwork));
    }

    // Arrow-key nudge.
    if resp.hovered() && !ui.ctx().wants_keyboard_input() {
        if let Some(a) = project.artwork.as_mut() {
            let step = if mods.shift { 10.0 } else { 1.0 };
            let mut d = Vec2::ZERO;
            ui.input(|i| {
                if i.key_pressed(egui::Key::ArrowLeft) { d.x -= step; }
                if i.key_pressed(egui::Key::ArrowRight) { d.x += step; }
                if i.key_pressed(egui::Key::ArrowUp) { d.y -= step; }
                if i.key_pressed(egui::Key::ArrowDown) { d.y += step; }
            });
            if d != Vec2::ZERO {
                a.transform.set_center(a.transform.center_pos() + d);
            }
        }
    }

    // Cursor feedback.
    let hover_hit = hover_pos.filter(|p| rect.contains(*p)).map(|p| hit_test(&view, comp, screen_quad(project).as_ref(), p));
    st.hover = hover_pos.filter(|p| rect.contains(*p)).map(|p| view.to_space(p));
    if resp.hovered() || st.drag.is_some() {
        let icon = match (st.drag, hover_hit) {
            (Some(DragOp::Pan { .. }) | Some(DragOp::Move { .. }), _) => CursorIcon::Grabbing,
            (Some(DragOp::Height { .. }), _) | (None, Some(Hit::Height)) => CursorIcon::ResizeVertical,
            (Some(DragOp::Scale { .. }), _) | (None, Some(Hit::Corner)) => CursorIcon::ResizeNwSe,
            (Some(DragOp::Rotate { .. }), _) | (None, Some(Hit::Rotate)) => CursorIcon::Crosshair,
            _ if space_down => CursorIcon::Grab,
            (None, _) if art_asset.is_some() => CursorIcon::Grab,
            (None, _) if project.artwork.is_none() => CursorIcon::PointingHand,
            _ => CursorIcon::Default,
        };
        ui.ctx().set_cursor_icon(icon);
    }

    // ---- Drawing --------------------------------------------------------------
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, theme::VOID);
    paint::dot_grid(&painter, rect, 22.0, view.origin, theme::LINE.gamma_multiply(0.7));

    let comp_rect = view.rect(Rect::from_min_size(Pos2::ZERO, comp));

    // Profile background, mapped from page space, so you can line art up with it.
    if st.show_background {
        if let (Some(bg), Some(bg_asset)) = (&project.background, assets.get(Slot::Background)) {
            if bg.show_in_editor {
                let arranged = config.profile.arrange(comp);
                let page_view = view.shifted(-arranged.comp_origin.to_vec2() * view.zoom);
                let t = bg.transform(bg_asset.size());
                let tint = Color32::from_gray(((1.0 - bg.dim) * 150.0) as u8);
                paint::placed_image(&painter, &page_view, bg_asset.texture.id(), bg_asset.size(), &t, &Default::default(), tint);
            }
        }
    }

    let fill = theme::rgb(project.style.canvas_fill);
    if project.export.fill_transparent {
        painter.rect_filled(comp_rect, 0.0, fill);
    } else {
        painter.rect_filled(comp_rect, 0.0, theme::VOID.gamma_multiply(0.6));
        paint::hatch(&painter, comp_rect, theme::LINE.gamma_multiply(0.6), 10.0);
    }

    if let (Some(a), Some(asset)) = (&project.artwork, art_asset) {
        // Ghost of the full image so you can see what falls outside the pieces.
        paint::placed_image(&painter, &view, asset.texture.id(), asset.size(), &a.transform, &a.crop, Color32::WHITE.gamma_multiply(0.26));
        for (_, r) in layout.region_rects(project.art_height) {
            let clip = painter.with_clip_rect(view.rect(r).intersect(rect));
            paint::placed_image(&clip, &view, asset.texture.id(), asset.size(), &a.transform, &a.crop, Color32::WHITE);
        }
    }

    for gap in layout.gaps(project.art_height) {
        let g = view.rect(gap);
        painter.rect_filled(g, 0.0, theme::VOID.gamma_multiply(0.75));
        paint::hatch(&painter, g, theme::LINE, 6.0);
    }

    for (i, (region, r)) in layout.region_rects(project.art_height).enumerate() {
        let sr = view.rect(r);
        let color = theme::region_color(i);
        painter.rect_stroke(sr, 0.0, Stroke::new(1.0, color.gamma_multiply(0.85)));
        paint::crop_marks(&painter, sr, color, 12.0, 5.0);
        painter.text(sr.left_top() + vec2(0.0, -10.0), Align2::LEFT_BOTTOM, &region.label, theme::small(), color);
        painter.text(
            sr.left_bottom() + vec2(0.0, 10.0),
            Align2::LEFT_TOP,
            format!("{} × {}", r.width() as u32, r.height() as u32),
            theme::small(),
            theme::MUTED,
        );
        if st.show_guides {
            let cx = sr.center().x;
            paint::dashed_line(&painter, pos2(cx, sr.top()), pos2(cx, sr.bottom()), Stroke::new(1.0, theme::FAINT.gamma_multiply(0.5)));
        }
    }
    if st.show_guides {
        let cy = comp_rect.center().y;
        paint::dashed_line(&painter, pos2(comp_rect.left(), cy), pos2(comp_rect.right(), cy), Stroke::new(1.0, theme::FAINT.gamma_multiply(0.5)));
    }

    for line in &st.snap_lines {
        let s = Stroke::new(1.0, theme::LUMEN);
        match *line {
            SnapLine::Vertical(x) => {
                let sx = view.to_screen(pos2(x, 0.0)).x;
                painter.line_segment([pos2(sx, rect.top()), pos2(sx, rect.bottom())], s);
            }
            SnapLine::Horizontal(y) => {
                let sy = view.to_screen(pos2(0.0, y)).y;
                painter.line_segment([pos2(rect.left(), sy), pos2(rect.right(), sy)], s);
            }
        }
    }

    // Selection frame and handles.
    if let Some(q) = screen_quad(project) {
        let active = st.drag.is_some() || resp.hovered();
        let stroke = Stroke::new(1.0, theme::LUMEN.gamma_multiply(if active { 0.9 } else { 0.45 }));
        painter.add(egui::Shape::closed_line(q.to_vec(), stroke));
        let knob = rotate_knob(&q);
        let top_mid = q[0] + (q[1] - q[0]) / 2.0;
        painter.line_segment([top_mid, knob], stroke);
        painter.circle(knob, 5.0, theme::VOID, Stroke::new(1.5, theme::LUMEN));
        for c in q {
            let hot = hover_hit == Some(Hit::Corner) && hover_pos.is_some_and(|h| h.distance(c) <= HANDLE_RADIUS);
            painter.rect(Rect::from_center_size(c, Vec2::splat(if hot { 10.0 } else { 8.0 })), 1.5, theme::VOID, Stroke::new(1.5, theme::LUMEN));
        }
        if let (Some(DragOp::Rotate { .. }), Some(a)) = (st.drag, &project.artwork) {
            paint::pill(&painter, knob + vec2(12.0, 0.0), Align2::LEFT_CENTER, &format!("{:.1}°", a.transform.rotation_deg), theme::TEXT, theme::RAISED);
        }
        if let (Some(DragOp::Scale { .. }), Some(a)) = (st.drag, &project.artwork) {
            paint::pill(&painter, q[2] + vec2(10.0, 10.0), Align2::LEFT_TOP, &format!("{:.0}%", a.transform.scale * 100.0), theme::TEXT, theme::RAISED);
        }
    }

    // Height handle: drag the bottom edge to make the showcase taller or shorter.
    let hh = height_handle(&view, comp);
    let hot = matches!(st.drag, Some(DragOp::Height { .. })) || hover_hit == Some(Hit::Height);
    painter.rect(
        Rect::from_center_size(hh.center(), vec2(36.0, 7.0)),
        3.5,
        if hot { theme::LUMEN } else { theme::RAISED },
        Stroke::new(1.0, if hot { theme::LUMEN } else { theme::FAINT }),
    );
    if hot {
        paint::pill(&painter, hh.center() + vec2(26.0, 0.0), Align2::LEFT_CENTER, &format!("{} px tall", project.art_height as u32), theme::TEXT, theme::RAISED);
    }

    if project.artwork.is_none() {
        let c = comp_rect.center();
        painter.text(c + vec2(0.0, -12.0), Align2::CENTER_CENTER, "Drop artwork here", egui::FontId::proportional(18.0), theme::TEXT);
        painter.text(c + vec2(0.0, 14.0), Align2::CENTER_CENTER, "or click to choose a file", theme::body(), theme::MUTED);
    } else if art_asset.is_none() {
        painter.text(comp_rect.center(), Align2::CENTER_CENTER, "Loading artwork…", theme::body(), theme::MUTED);
        ui.ctx().request_repaint();
    }

    let hint = "Drag to move  •  Corners scale  •  Knob rotates  •  Scroll zooms  •  Middle-drag pans";
    painter.text(rect.left_bottom() + vec2(14.0, -12.0), Align2::LEFT_BOTTOM, hint, theme::small(), theme::FAINT);
}

fn normalize_degrees(d: f32) -> f32 {
    let mut d = d % 360.0;
    if d > 180.0 {
        d -= 360.0;
    } else if d <= -180.0 {
        d += 360.0;
    }
    d
}

fn toolbar(ui: &mut Ui, st: &mut CanvasState, project: &Project, config: &SteamConfig, actions: &mut Vec<Action>, has_art: bool) {
    widgets::toolbar_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            let layout = config.layout(&project.layout_id);
            egui::ComboBox::from_id_salt("canvas_layout")
                .selected_text(&layout.name)
                .width(190.0)
                .show_ui(ui, |ui| {
                    for l in &config.layouts {
                        if ui.selectable_label(l.id == project.layout_id, &l.name).clicked() {
                            actions.push(Action::SetLayout(l.id.clone()));
                        }
                    }
                });
            ui.separator();
            ui.add_enabled_ui(has_art, |ui| {
                if ui.button("Fill").on_hover_text("Scale so every piece is covered (double-click the canvas)").clicked() {
                    actions.push(Action::Fit(FitMode::Cover));
                }
                if ui.button("Fit width").clicked() {
                    actions.push(Action::Fit(FitMode::FitWidth));
                }
                if ui.button("Show all").on_hover_text("Fit the whole image inside the layout").clicked() {
                    actions.push(Action::Fit(FitMode::Contain));
                }
                if ui.button("Center").clicked() {
                    actions.push(Action::CenterArtwork);
                }
                if ui.button("Reset").on_hover_text("Clear crop, rotation and flip, then fill").clicked() {
                    actions.push(Action::ResetArtwork);
                }
            });
            ui.separator();
            ui.toggle_value(&mut st.snapping, "Snap").on_hover_text("Snap to edges and centres. Hold Alt to bypass.");
            ui.toggle_value(&mut st.show_guides, "Guides");
            ui.toggle_value(&mut st.show_background, "Background");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Fit view").on_hover_text("Scroll to zoom, middle-drag to pan").clicked() {
                    st.needs_fit = true;
                }
            });
        });
    });
}

/// Exploded view: every exported piece side by side, exactly as it will be saved.
pub fn show_slices(ui: &mut Ui, project: &Project, assets: &AssetStore, config: &SteamConfig) {
    let layout = config.layout(&project.layout_id);
    let art = assets.get(Slot::Artwork).filter(|_| project.artwork.is_some());
    let scale = project.export.scale.resolve(project.artwork.as_ref().map(|a| a.transform.scale));
    let plans = slicer::plan(layout, project.art_height, scale);

    widgets::toolbar_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("{} pieces at {scale}× export", plans.len())).color(theme::TEXT));
            ui.label(
                egui::RichText::new("Every piece is rendered from the same composition with a shared height, so they line up on Steam.")
                    .color(theme::MUTED),
            );
        });
    });

    let (rect, resp) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, theme::VOID);

    let h = project.art_height.round();
    let n = plans.len() as f32;
    let spacing = 64.0;
    let (top, bottom) = (52.0, 82.0);
    let sum_w: f32 = plans.iter().map(|p| p.display.width()).sum();
    let zoom = ((rect.width() - 120.0 - spacing * (n - 1.0)) / sum_w)
        .min((rect.height() - top - bottom - 20.0) / h)
        .clamp(0.02, 3.0);
    let total_w = sum_w * zoom + spacing * (n - 1.0);
    let mut x = rect.center().x - total_w / 2.0;
    let y = rect.top() + top + ((rect.height() - top - bottom) - h * zoom).max(0.0) / 2.0;
    let ext = project.export.format.extension();
    let fill = theme::rgb(project.style.canvas_fill);
    let mut first_left = None;
    let mut last_right = x;

    for (i, plan) in plans.iter().enumerate() {
        let r = plan.display;
        let screen = Rect::from_min_size(pos2(x, y), vec2(r.width() * zoom, h * zoom));
        first_left.get_or_insert(screen.left());
        last_right = screen.right();
        let view = View { origin: screen.min - r.min.to_vec2() * zoom, zoom };
        painter.rect_filled(screen, 0.0, if project.export.fill_transparent { fill } else { theme::PANEL });
        if let (Some(a), Some(asset)) = (&project.artwork, art) {
            let clip = painter.with_clip_rect(screen.intersect(rect));
            paint::placed_image(&clip, &view, asset.texture.id(), asset.size(), &a.transform, &a.crop, Color32::WHITE);
        }
        let color = theme::region_color(i);
        paint::crop_marks(&painter, screen, color, 14.0, 6.0);
        painter.text(screen.left_top() + vec2(0.0, -20.0), Align2::LEFT_BOTTOM, &plan.region.label, theme::body(), color);
        let below = screen.left_bottom() + vec2(0.0, 16.0);
        painter.text(below, Align2::LEFT_TOP, plan.file_name(ext), theme::body(), theme::TEXT);
        painter.text(below + vec2(0.0, 18.0), Align2::LEFT_TOP, format!("{} × {} px", plan.pixel_width, plan.pixel_height), theme::small(), theme::MUTED);
        painter.text(
            below + vec2(0.0, 34.0),
            Align2::LEFT_TOP,
            format!("Shows at {} × {}", r.width() as u32, h as u32),
            theme::small(),
            theme::FAINT,
        );
        x = screen.right() + spacing;
    }

    // Alignment guide follows the pointer across every piece.
    if let (Some(p), Some(left)) = (resp.hover_pos(), first_left) {
        if p.y >= y && p.y <= y + h * zoom && plans.len() > 1 {
            let s = Stroke::new(1.0, theme::LUMEN.gamma_multiply(0.8));
            paint::dashed_line(&painter, pos2(left - 10.0, p.y), pos2(last_right + 10.0, p.y), s);
            let comp_y = ((p.y - y) / zoom).round();
            paint::pill(&painter, pos2(last_right + 16.0, p.y), Align2::LEFT_CENTER, &format!("y {comp_y} px in every piece"), theme::TEXT, theme::RAISED);
        }
    }
    if art.is_none() {
        painter.text(rect.center(), Align2::CENTER_CENTER, "Import artwork to see your pieces", theme::body(), theme::MUTED);
    }
}
