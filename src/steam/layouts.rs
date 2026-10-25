use eframe::emath::{pos2, vec2, Rect, Vec2};
use serde::{Deserialize, Serialize};

/// One exported artwork piece, positioned in composition space (Steam display px).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegionSpec {
    pub id: String,
    pub label: String,
    pub file_stem: String,
    pub x: f32,
    pub width: f32,
}

/// A Steam showcase layout. All pieces share the composition's height and top edge,
/// which is what keeps them visually continuous on the profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutSpec {
    pub id: String,
    pub name: String,
    pub description: String,
    pub showcase_title: String,
    pub default_height: f32,
    pub min_height: f32,
    pub max_height: f32,
    pub regions: Vec<RegionSpec>,
}

impl LayoutSpec {
    pub fn width(&self) -> f32 {
        self.regions
            .iter()
            .map(|r| r.x + r.width)
            .fold(0.0, f32::max)
    }

    pub fn size(&self, height: f32) -> Vec2 {
        vec2(self.width(), height)
    }

    pub fn region_rect(region: &RegionSpec, height: f32) -> Rect {
        Rect::from_min_size(pos2(region.x, 0.0), vec2(region.width, height))
    }

    pub fn region_rects(&self, height: f32) -> impl Iterator<Item = (&RegionSpec, Rect)> + '_ {
        self.regions
            .iter()
            .map(move |r| (r, Self::region_rect(r, height)))
    }

    /// Spans between pieces. Image content there is never visible on Steam.
    pub fn gaps(&self, height: f32) -> Vec<Rect> {
        let mut spans: Vec<(f32, f32)> = self.regions.iter().map(|r| (r.x, r.x + r.width)).collect();
        spans.sort_by(|a, b| a.0.total_cmp(&b.0));
        spans
            .windows(2)
            .filter(|w| w[1].0 > w[0].1)
            .map(|w| Rect::from_min_max(pos2(w[0].1, 0.0), pos2(w[1].0, height)))
            .collect()
    }

    pub fn widths_summary(&self) -> String {
        self.regions
            .iter()
            .map(|r| format!("{}", r.width as u32))
            .collect::<Vec<_>>()
            .join(" + ")
            + " px"
    }

    pub fn clamp_height(&self, h: f32) -> f32 {
        h.round().clamp(self.min_height, self.max_height)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.regions.is_empty() {
            return Err(format!("Layout '{}' has no regions.", self.id));
        }
        for r in &self.regions {
            if r.width < 1.0 || r.x < 0.0 {
                return Err(format!("Region '{}' in '{}' has an invalid size.", r.id, self.id));
            }
            let safe_name = !r.file_stem.is_empty()
                && r.file_stem.len() <= 64
                && r.file_stem.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
            if !safe_name {
                return Err(format!(
                    "Region '{}' in '{}' needs a file_stem of letters, digits, '-' or '_' only.",
                    r.id, self.id
                ));
            }
            if r.x.fract() != 0.0 || r.width.fract() != 0.0 {
                return Err(format!(
                    "Region '{}' in '{}' must use whole-pixel x and width.",
                    r.id, self.id
                ));
            }
        }
        let mut spans: Vec<(f32, f32)> = self.regions.iter().map(|r| (r.x, r.x + r.width)).collect();
        spans.sort_by(|a, b| a.0.total_cmp(&b.0));
        if spans.windows(2).any(|w| w[1].0 < w[0].1) {
            return Err(format!("Regions in '{}' overlap.", self.id));
        }
        if !(self.min_height > 0.0 && self.min_height <= self.default_height && self.default_height <= self.max_height) {
            return Err(format!("Height limits in '{}' are inconsistent.", self.id));
        }
        Ok(())
    }
}
