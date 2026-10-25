//! Turns one composition into the separate pieces Steam needs.
//!
//! Each piece is rendered directly from the source with the shared composition
//! transform, so pieces line up by construction rather than by cropping math.

use eframe::emath::Rect;
use image::RgbaImage;

use super::renderer::{render_layer, Placement};
use crate::steam::layouts::{LayoutSpec, RegionSpec};

#[derive(Debug, Clone)]
pub struct SlicePlan {
    pub region: RegionSpec,
    /// Rectangle in composition space (Steam display px).
    pub display: Rect,
    pub pixel_width: u32,
    pub pixel_height: u32,
}

impl SlicePlan {
    pub fn file_name(&self, ext: &str) -> String {
        format!("{}.{ext}", self.region.file_stem)
    }
}

/// `scale` must be an integer so every piece maps back to exactly the same
/// display height on Steam (h·s / (w·s) · w = h for every piece).
pub fn plan(layout: &LayoutSpec, height: f32, scale: u32) -> Vec<SlicePlan> {
    let h = height.round().max(1.0);
    layout
        .region_rects(h)
        .map(|(region, rect)| SlicePlan {
            region: region.clone(),
            display: rect,
            pixel_width: rect.width().round() as u32 * scale,
            pixel_height: h as u32 * scale,
        })
        .collect()
}

pub fn render(plan: &SlicePlan, scale: u32, layer: Option<&Placement>) -> RgbaImage {
    let mut img = RgbaImage::new(plan.pixel_width, plan.pixel_height);
    if let Some(layer) = layer {
        render_layer(&mut img, plan.display.min, scale as f32, layer);
    }
    img
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{Crop, Transform};
    use crate::steam::SteamConfig;

    /// A horizontal gradient spanning the whole composition must continue across
    /// the gap: the side piece's first column equals where the main piece would be
    /// had it extended through the gap.
    #[test]
    fn pieces_align_across_the_gap() {
        let cfg = SteamConfig::embedded();
        let layout = cfg.layout("artwork_showcase");
        let comp_w = layout.width();
        let src_w = comp_w as u32; // 1 source px per display px
        let src = RgbaImage::from_fn(src_w, 10, |x, _| image::Rgba([(x % 256) as u8, (x / 256) as u8, 0, 255]));
        let layer = Placement {
            image: &src,
            transform: Transform { center: [comp_w / 2.0, 5.0], scale: 1.0, rotation_deg: 0.0, flip_x: false },
            crop: Crop::default(),
        };
        for scale in [1u32, 2, 3] {
            let plans = plan(layout, 10.0, scale);
            assert_eq!(plans[0].pixel_width, 506 * scale);
            assert_eq!(plans[1].pixel_width, 100 * scale);
            assert_eq!(plans[0].pixel_height, plans[1].pixel_height, "pieces must share a height");
            let side = render(&plans[1], scale, Some(&layer));
            let p = side.get_pixel(0, 0);
            let decoded = p[0] as u32 + p[1] as u32 * 256;
            assert_eq!(decoded, plans[1].region.x as u32, "side piece starts at its composition x");
        }
    }
}
