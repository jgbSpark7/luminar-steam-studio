//! Geometry for placing a source image in composition space. Pure math, no UI,
//! so the editor, validation and the export renderer all agree exactly.

use eframe::emath::{pos2, vec2, Pos2, Rect, Vec2};

use super::model::{Crop, Transform};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitMode {
    /// Fill the target completely, cropping overflow.
    Cover,
    /// Show the whole image inside the target.
    Contain,
    FitWidth,
    FitHeight,
}

impl Transform {
    pub fn rotation_rad(&self) -> f32 {
        self.rotation_deg.to_radians()
    }

    pub fn center_pos(&self) -> Pos2 {
        pos2(self.center[0], self.center[1])
    }

    pub fn set_center(&mut self, p: Pos2) {
        self.center = [p.x, p.y];
    }

    pub fn source_to_comp(&self, image: Vec2, p: Pos2) -> Pos2 {
        let mut l = p.to_vec2() - image / 2.0;
        if self.flip_x {
            l.x = -l.x;
        }
        let (s, c) = self.rotation_rad().sin_cos();
        let r = vec2(l.x * c - l.y * s, l.x * s + l.y * c);
        self.center_pos() + r * self.scale
    }

    pub fn comp_to_source(&self, image: Vec2, q: Pos2) -> Pos2 {
        let d = (q - self.center_pos()) / self.scale.max(1e-6);
        let (s, c) = self.rotation_rad().sin_cos();
        let mut l = vec2(d.x * c + d.y * s, -d.x * s + d.y * c);
        if self.flip_x {
            l.x = -l.x;
        }
        (image / 2.0 + l).to_pos2()
    }
}

/// Visible part of the source after cropping, in source pixels.
pub fn visible_rect(image: Vec2, crop: &Crop) -> Rect {
    let c = crop.sanitized();
    Rect::from_min_max(
        pos2(c.left * image.x, c.top * image.y),
        pos2((1.0 - c.right) * image.x, (1.0 - c.bottom) * image.y),
    )
}

/// Corners of the visible image (TL, TR, BR, BL of the source) in composition space.
pub fn quad(t: &Transform, image: Vec2, crop: &Crop) -> [Pos2; 4] {
    let v = visible_rect(image, crop);
    [v.left_top(), v.right_top(), v.right_bottom(), v.left_bottom()].map(|p| t.source_to_comp(image, p))
}

/// Matching normalised texture coordinates for [`quad`].
pub fn quad_uv(image: Vec2, crop: &Crop) -> [Pos2; 4] {
    let v = visible_rect(image, crop);
    let n = |p: Pos2| pos2(p.x / image.x.max(1.0), p.y / image.y.max(1.0));
    [n(v.left_top()), n(v.right_top()), n(v.right_bottom()), n(v.left_bottom())]
}

pub fn bounds(points: &[Pos2]) -> Rect {
    let mut r = Rect::NOTHING;
    for p in points {
        r.extend_with(*p);
    }
    r
}

/// True if `p` lies inside (or on the edge of) the convex quad.
pub fn point_in_quad(q: &[Pos2; 4], p: Pos2) -> bool {
    let mut sign = 0.0f32;
    for i in 0..4 {
        let a = q[i];
        let b = q[(i + 1) % 4];
        let cross = (b - a).x * (p - a).y - (b - a).y * (p - a).x;
        if cross.abs() < 1e-3 {
            continue;
        }
        if sign == 0.0 {
            sign = cross.signum();
        } else if cross.signum() != sign {
            return false;
        }
    }
    true
}

/// True if the image fully covers `region`.
pub fn covers(t: &Transform, image: Vec2, crop: &Crop, region: Rect) -> bool {
    let q = quad(t, image, crop);
    let shrunk = region.shrink(0.25);
    [shrunk.left_top(), shrunk.right_top(), shrunk.right_bottom(), shrunk.left_bottom()]
        .iter()
        .all(|p| point_in_quad(&q, *p))
}

/// Builds a transform that fits the visible image into `target`, keeping rotation and flip.
pub fn fit(image: Vec2, crop: &Crop, rotation_deg: f32, flip_x: bool, target: Rect, mode: FitMode) -> Transform {
    let v = visible_rect(image, crop);
    let (s, c) = rotation_deg.to_radians().sin_cos();
    let bw = v.width() * c.abs() + v.height() * s.abs();
    let bh = v.width() * s.abs() + v.height() * c.abs();
    let sx = target.width() / bw.max(1.0);
    let sy = target.height() / bh.max(1.0);
    let scale = match mode {
        // The target, rotated into the image's frame, must fit inside the image.
        FitMode::Cover => {
            let (w, h) = (target.width(), target.height());
            let need_w = w * c.abs() + h * s.abs();
            let need_h = w * s.abs() + h * c.abs();
            (need_w / v.width().max(1.0)).max(need_h / v.height().max(1.0))
        }
        FitMode::Contain => sx.min(sy),
        FitMode::FitWidth => sx,
        FitMode::FitHeight => sy,
    };
    let mut t = Transform { center: [0.0, 0.0], scale, rotation_deg, flip_x };
    let offset = t.source_to_comp(image, v.center()).to_vec2();
    t.set_center(target.center() - offset);
    t
}

/// Composition height at which the visible image, fitted to `width`, keeps its aspect.
pub fn natural_height(image: Vec2, crop: &Crop, rotation_deg: f32, width: f32) -> f32 {
    let v = visible_rect(image, crop);
    let (s, c) = rotation_deg.to_radians().sin_cos();
    let bw = v.width() * c.abs() + v.height() * s.abs();
    let bh = v.width() * s.abs() + v.height() * c.abs();
    width * bh / bw.max(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_with_rotation_and_flip() {
        let img = vec2(400.0, 300.0);
        let t = Transform { center: [120.0, 80.0], scale: 0.7, rotation_deg: 33.0, flip_x: true };
        let p = pos2(37.0, 211.0);
        let back = t.comp_to_source(img, t.source_to_comp(img, p));
        assert!((back - p).length() < 1e-3);
    }

    #[test]
    fn cover_fit_covers_target() {
        let img = vec2(1920.0, 1080.0);
        let target = Rect::from_min_size(Pos2::ZERO, vec2(610.0, 800.0));
        let t = fit(img, &Crop::default(), 0.0, false, target, FitMode::Cover);
        assert!(covers(&t, img, &Crop::default(), target));
        let t2 = fit(img, &Crop::default(), 15.0, false, target, FitMode::Cover);
        assert!(covers(&t2, img, &Crop::default(), target));
    }
}
