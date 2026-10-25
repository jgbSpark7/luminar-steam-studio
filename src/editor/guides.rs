use eframe::emath::{Rect, Vec2};

use crate::steam::LayoutSpec;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SnapLine {
    Vertical(f32),
    Horizontal(f32),
}

pub struct Guides {
    pub xs: Vec<f32>,
    pub ys: Vec<f32>,
}

impl Guides {
    /// Edges and centres of the composition and of every region.
    pub fn for_layout(layout: &LayoutSpec, height: f32) -> Self {
        let w = layout.width();
        let mut xs = vec![0.0, w, w / 2.0];
        for (_, r) in layout.region_rects(height) {
            xs.extend([r.left(), r.right(), r.center().x]);
        }
        xs.sort_by(f32::total_cmp);
        xs.dedup_by(|a, b| (*a - *b).abs() < 0.01);
        Self { xs, ys: vec![0.0, height, height / 2.0] }
    }

    /// Returns the correction to apply so `bounds` snaps to the nearest guide
    /// within `threshold` on each axis, plus the lines that engaged.
    pub fn snap(&self, bounds: Rect, threshold: f32) -> (Vec2, Vec<SnapLine>) {
        let best = |cands: [f32; 3], guides: &[f32]| -> Option<(f32, f32)> {
            let mut best: Option<(f32, f32)> = None;
            for c in cands {
                for g in guides {
                    let d = g - c;
                    if d.abs() <= threshold && best.map_or(true, |(bd, _)| d.abs() < bd.abs()) {
                        best = Some((d, *g));
                    }
                }
            }
            best
        };
        let mut delta = Vec2::ZERO;
        let mut lines = Vec::new();
        if let Some((d, g)) = best([bounds.left(), bounds.center().x, bounds.right()], &self.xs) {
            delta.x = d;
            lines.push(SnapLine::Vertical(g));
        }
        if let Some((d, g)) = best([bounds.top(), bounds.center().y, bounds.bottom()], &self.ys) {
            delta.y = d;
            lines.push(SnapLine::Horizontal(g));
        }
        (delta, lines)
    }
}
