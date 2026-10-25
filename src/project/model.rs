use std::path::PathBuf;

use eframe::emath::{vec2, Rect, Vec2};
use serde::{Deserialize, Serialize};

use crate::steam::layouts::LayoutSpec;

/// Everything needed to reproduce a design. Holds paths, never pixels, so it
/// stays cheap to clone for undo history.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Project {
    pub name: String,
    pub layout_id: String,
    /// Composition height in Steam display px. Shared by every exported piece.
    pub art_height: f32,
    pub artwork: Option<ArtworkLayer>,
    pub background: Option<BackgroundLayer>,
    pub avatar: Option<AvatarLayer>,
    pub profile: ProfileInfo,
    pub style: Style,
    pub export: ExportSettings,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            name: "Untitled profile".into(),
            layout_id: "artwork_showcase".into(),
            art_height: 800.0,
            artwork: None,
            background: None,
            avatar: None,
            profile: ProfileInfo::default(),
            style: Style::default(),
            export: ExportSettings::default(),
        }
    }
}

impl Project {
    pub fn new_for(layout: &LayoutSpec) -> Self {
        Self {
            layout_id: layout.id.clone(),
            art_height: layout.default_height,
            ..Self::default()
        }
    }
}

/// Places a source image in composition space. `center` is where the centre of the
/// *uncropped* source lands; `scale` is display px per source px.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Transform {
    pub center: [f32; 2],
    pub scale: f32,
    pub rotation_deg: f32,
    pub flip_x: bool,
}

impl Default for Transform {
    fn default() -> Self {
        Self { center: [0.0, 0.0], scale: 1.0, rotation_deg: 0.0, flip_x: false }
    }
}

/// Non-destructive crop, as fractions trimmed from each edge of the source.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Crop {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Crop {
    pub const MAX_SIDE: f32 = 0.45;

    pub fn is_none(&self) -> bool {
        *self == Crop::default()
    }

    pub fn sanitized(self) -> Self {
        let c = |v: f32| v.clamp(0.0, Self::MAX_SIDE);
        Self { left: c(self.left), top: c(self.top), right: c(self.right), bottom: c(self.bottom) }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtworkLayer {
    pub source: PathBuf,
    #[serde(default)]
    pub transform: Transform,
    #[serde(default)]
    pub crop: Crop,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BackgroundLayer {
    pub source: PathBuf,
    /// 1.0 = native pixel size, which is how Steam draws backgrounds.
    pub scale: f32,
    /// Page-space offset from Steam's default top-centre placement.
    pub offset: [f32; 2],
    /// 0..1 darkening overlay applied in previews.
    pub dim: f32,
    pub show_in_editor: bool,
}

impl Default for BackgroundLayer {
    fn default() -> Self {
        Self { source: PathBuf::new(), scale: 1.0, offset: [0.0, 0.0], dim: 0.0, show_in_editor: true }
    }
}

impl BackgroundLayer {
    /// Background placement expressed as a page-space transform.
    pub fn transform(&self, image_size: Vec2) -> Transform {
        Transform {
            center: [self.offset[0], self.offset[1] + image_size.y * self.scale / 2.0],
            scale: self.scale,
            rotation_deg: 0.0,
            flip_x: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AvatarLayer {
    pub source: PathBuf,
    /// 1.0 shows the largest centred square; higher values zoom in.
    pub zoom: f32,
    /// -1..1 on each axis, relative to the available slack.
    pub offset: [f32; 2],
    pub circular: bool,
}

impl Default for AvatarLayer {
    fn default() -> Self {
        Self { source: PathBuf::new(), zoom: 1.0, offset: [0.0, 0.0], circular: false }
    }
}

impl AvatarLayer {
    /// Square crop in source pixels.
    pub fn crop_rect(&self, image_size: Vec2) -> Rect {
        let side = image_size.x.min(image_size.y) / self.zoom.max(1.0);
        let slack = (image_size - Vec2::splat(side)) / 2.0;
        let center = image_size / 2.0 + vec2(self.offset[0] * slack.x, self.offset[1] * slack.y);
        Rect::from_center_size(center.to_pos2(), Vec2::splat(side))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProfileInfo {
    pub display_name: String,
    pub level: u32,
    pub status: String,
}

impl Default for ProfileInfo {
    fn default() -> Self {
        Self { display_name: "Your Name".into(), level: 100, status: "Currently Online".into() }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Style {
    pub template_id: String,
    pub accent: [u8; 3],
    /// Page colour when no background image is set.
    pub backdrop: [u8; 3],
    /// Fill for any uncovered/transparent export area.
    pub canvas_fill: [u8; 3],
    /// Opacity of the profile's dark panels in the preview.
    pub panel_alpha: f32,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            template_id: "luminarcore".into(),
            accent: [168, 212, 255],
            backdrop: [12, 16, 32],
            canvas_fill: [8, 9, 12],
            panel_alpha: 0.72,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    Png,
    Jpeg,
}

impl ExportFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportScale {
    Auto,
    X1,
    X2,
    X3,
    X4,
}

impl ExportScale {
    pub const ALL: [Self; 5] = [Self::Auto, Self::X1, Self::X2, Self::X3, Self::X4];

    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::X1 => "1×",
            Self::X2 => "2×",
            Self::X3 => "3×",
            Self::X4 => "4×",
        }
    }

    /// Always an integer: integer factors are what guarantee every piece
    /// displays at exactly the same height after Steam rescales it.
    pub fn resolve(self, artwork_scale: Option<f32>) -> u32 {
        match self {
            Self::Auto => artwork_scale
                .filter(|s| *s > 0.0)
                .map(|s| (1.0 / s).floor().clamp(1.0, 3.0) as u32)
                .unwrap_or(2),
            Self::X1 => 1,
            Self::X2 => 2,
            Self::X3 => 3,
            Self::X4 => 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExportSettings {
    pub folder: Option<PathBuf>,
    pub format: ExportFormat,
    pub jpeg_quality: u8,
    pub scale: ExportScale,
    pub make_zip: bool,
    pub include_avatar: bool,
    pub include_background: bool,
    pub include_preview: bool,
    pub include_project: bool,
    pub fill_transparent: bool,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            folder: None,
            format: ExportFormat::Png,
            jpeg_quality: 92,
            scale: ExportScale::Auto,
            make_zip: true,
            include_avatar: true,
            include_background: false,
            include_preview: true,
            include_project: true,
            fill_transparent: true,
        }
    }
}
