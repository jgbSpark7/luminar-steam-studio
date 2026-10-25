use eframe::egui::{self, epaint::Vertex, Color32, Mesh, Painter, Pos2, Rect, Shape, Stroke, TextureId};
use eframe::emath::{pos2, vec2, Vec2};

use crate::project::transform::{quad, quad_uv};
use crate::project::{Crop, Transform};

/// Maps a 2D space (composition or page) onto the screen.
#[derive(Debug, Clone, Copy)]
pub struct View {
    /// Screen position of space point (0, 0).
    pub origin: Pos2,
    pub zoom: f32,
}

impl View {
    pub fn to_screen(&self, p: Pos2) -> Pos2 {
        self.origin + p.to_vec2() * self.zoom
    }

    pub fn to_space(&self, s: Pos2) -> Pos2 {
        ((s - self.origin) / self.zoom).to_pos2()
    }

    pub fn rect(&self, r: Rect) -> Rect {
        Rect::from_min_max(self.to_screen(r.min), self.to_screen(r.max))
    }

    pub fn shifted(&self, by: Vec2) -> Self {
        Self { origin: self.origin + by, zoom: self.zoom }
    }
}

pub fn textured_quad(painter: &Painter, tex: TextureId, corners: [Pos2; 4], uv: [Pos2; 4], tint: Color32) {
    let mut mesh = Mesh::with_texture(tex);
    for i in 0..4 {
        mesh.vertices.push(Vertex { pos: corners[i], uv: uv[i], color: tint });
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(Shape::mesh(mesh));
}

/// Draws a placed image (with transform and crop) through `view`.
pub fn placed_image(painter: &Painter, view: &View, tex: TextureId, image: Vec2, t: &Transform, crop: &Crop, tint: Color32) {
    let corners = quad(t, image, crop).map(|p| view.to_screen(p));
    textured_quad(painter, tex, corners, quad_uv(image, crop), tint);
}

pub fn textured_circle(painter: &Painter, tex: TextureId, center: Pos2, radius: f32, uv: Rect, tint: Color32) {
    const SEGMENTS: u32 = 64;
    let mut mesh = Mesh::with_texture(tex);
    mesh.vertices.push(Vertex { pos: center, uv: uv.center(), color: tint });
    for i in 0..=SEGMENTS {
        let a = i as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
        let dir = vec2(a.cos(), a.sin());
        mesh.vertices.push(Vertex {
            pos: center + dir * radius,
            uv: uv.center() + vec2(dir.x * uv.width(), dir.y * uv.height()) * 0.5,
            color: tint,
        });
    }
    for i in 1..=SEGMENTS {
        mesh.add_triangle(0, i, i + 1);
    }
    painter.add(Shape::mesh(mesh));
}

/// Print-style crop marks just outside each corner of `rect`.
pub fn crop_marks(painter: &Painter, rect: Rect, color: Color32, len: f32, gap: f32) {
    let s = Stroke::new(1.0, color);
    for (corner, dx, dy) in [
        (rect.left_top(), -1.0, -1.0),
        (rect.right_top(), 1.0, -1.0),
        (rect.right_bottom(), 1.0, 1.0),
        (rect.left_bottom(), -1.0, 1.0),
    ] {
        let h0 = corner + vec2(dx * gap, 0.0);
        painter.line_segment([h0, h0 + vec2(dx * len, 0.0)], s);
        let v0 = corner + vec2(0.0, dy * gap);
        painter.line_segment([v0, v0 + vec2(0.0, dy * len)], s);
    }
}

pub fn hatch(painter: &Painter, rect: Rect, color: Color32, spacing: f32) {
    let p = painter.with_clip_rect(rect.intersect(painter.clip_rect()));
    let s = Stroke::new(1.0, color);
    let h = rect.height();
    let mut x = rect.left() - h;
    while x < rect.right() {
        p.line_segment([pos2(x, rect.bottom()), pos2(x + h, rect.top())], s);
        x += spacing;
    }
}

pub fn dashed_rect(painter: &Painter, rect: Rect, stroke: Stroke, dash: f32, gap: f32) {
    let pts = [rect.left_top(), rect.right_top(), rect.right_bottom(), rect.left_bottom(), rect.left_top()];
    painter.extend(Shape::dashed_line(&pts, stroke, dash, gap));
}

pub fn dashed_line(painter: &Painter, a: Pos2, b: Pos2, stroke: Stroke) {
    painter.extend(Shape::dashed_line(&[a, b], stroke, 5.0, 4.0));
}

pub fn dot_grid(painter: &Painter, rect: Rect, spacing: f32, anchor: Pos2, color: Color32) {
    let start_x = rect.left() + (anchor.x - rect.left()).rem_euclid(spacing);
    let start_y = rect.top() + (anchor.y - rect.top()).rem_euclid(spacing);
    let mut y = start_y;
    while y < rect.bottom() {
        let mut x = start_x;
        while x < rect.right() {
            painter.rect_filled(Rect::from_center_size(pos2(x, y), Vec2::splat(1.4)), 0.0, color);
            x += spacing;
        }
        y += spacing;
    }
}

/// Small pill with text, used for on-canvas readouts.
pub fn pill(painter: &Painter, anchor: Pos2, align: egui::Align2, text: &str, fg: Color32, bg: Color32) -> Rect {
    let galley = painter.layout_no_wrap(text.to_owned(), egui::FontId::proportional(11.5), fg);
    let size = galley.size() + vec2(12.0, 6.0);
    let rect = align.anchor_size(anchor, size);
    painter.rect_filled(rect, 4.0, bg);
    painter.galley(rect.min + vec2(6.0, 3.0), galley, fg);
    rect
}
