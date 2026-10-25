//! Templates are starting points: a layout preset plus styling. Applying one never
//! touches the artwork itself, and everything stays editable afterwards.

use crate::project::Project;
use crate::steam::SteamConfig;

pub struct Template {
    pub id: &'static str,
    pub name: &'static str,
    pub tagline: &'static str,
    pub layout: &'static str,
    pub art_height: f32,
    pub accent: [u8; 3],
    pub backdrop: [u8; 3],
    pub canvas_fill: [u8; 3],
    pub panel_alpha: f32,
}

pub const TEMPLATES: &[Template] = &[
    Template { id: "luminarcore", name: "LuminarCore", tagline: "Ice-lit split art", layout: "artwork_showcase", art_height: 800.0, accent: [168, 212, 255], backdrop: [12, 16, 32], canvas_fill: [8, 9, 12], panel_alpha: 0.72 },
    Template { id: "monochrome", name: "Monochrome", tagline: "Ink and starlight", layout: "artwork_showcase", art_height: 860.0, accent: [236, 236, 236], backdrop: [6, 6, 7], canvas_fill: [0, 0, 0], panel_alpha: 0.80 },
    Template { id: "anime", name: "Anime", tagline: "Tall character focus", layout: "artwork_showcase", art_height: 940.0, accent: [255, 158, 199], backdrop: [26, 16, 32], canvas_fill: [18, 10, 22], panel_alpha: 0.66 },
    Template { id: "gothic", name: "Gothic", tagline: "Deep crimson, heavy shadow", layout: "artwork_showcase", art_height: 1000.0, accent: [196, 58, 74], backdrop: [14, 8, 9], canvas_fill: [8, 4, 5], panel_alpha: 0.84 },
    Template { id: "minimal", name: "Minimal", tagline: "One quiet banner", layout: "featured_artwork", art_height: 420.0, accent: [220, 222, 226], backdrop: [20, 21, 23], canvas_fill: [20, 21, 23], panel_alpha: 0.60 },
    Template { id: "cyberpunk", name: "Cyberpunk", tagline: "Neon over violet night", layout: "artwork_showcase", art_height: 760.0, accent: [44, 246, 232], backdrop: [18, 10, 31], canvas_fill: [10, 4, 20], panel_alpha: 0.70 },
    Template { id: "fps", name: "FPS", tagline: "Wide tactical banner", layout: "featured_artwork", art_height: 360.0, accent: [255, 106, 43], backdrop: [15, 18, 16], canvas_fill: [10, 12, 10], panel_alpha: 0.78 },
    Template { id: "dark", name: "Dark", tagline: "Low-key and steel blue", layout: "artwork_showcase", art_height: 720.0, accent: [124, 139, 166], backdrop: [9, 11, 16], canvas_fill: [5, 6, 9], panel_alpha: 0.86 },
    Template { id: "retro", name: "Retro", tagline: "Warm amber CRT", layout: "featured_artwork", art_height: 640.0, accent: [255, 179, 71], backdrop: [27, 20, 16], canvas_fill: [20, 14, 10], panel_alpha: 0.70 },
    Template { id: "futuristic", name: "Futuristic", tagline: "Mint on deep teal", layout: "artwork_showcase", art_height: 780.0, accent: [125, 249, 193], backdrop: [6, 20, 26], canvas_fill: [3, 12, 16], panel_alpha: 0.68 },
];

pub fn find(id: &str) -> Option<&'static Template> {
    TEMPLATES.iter().find(|t| t.id == id)
}

pub fn apply(t: &Template, project: &mut Project, config: &SteamConfig) {
    project.style.template_id = t.id.into();
    project.style.accent = t.accent;
    project.style.backdrop = t.backdrop;
    project.style.canvas_fill = t.canvas_fill;
    project.style.panel_alpha = t.panel_alpha;
    if config.has_layout(t.layout) {
        let layout = config.layout(t.layout);
        project.layout_id = layout.id.clone();
        project.art_height = layout.clamp_height(t.art_height);
    }
}
