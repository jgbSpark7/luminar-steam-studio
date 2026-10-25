use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub const STORAGE_KEY: &str = "luminar_steam_studio.settings";
const MAX_RECENT: usize = 8;

/// App-wide preferences, persisted by eframe between sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub recent: Vec<PathBuf>,
    pub ui_scale: f32,
    pub snapping: bool,
    pub show_guides: bool,
    pub last_open_dir: Option<PathBuf>,
    pub last_import_dir: Option<PathBuf>,
    pub last_export_dir: Option<PathBuf>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            recent: Vec::new(),
            ui_scale: 1.0,
            snapping: true,
            show_guides: true,
            last_open_dir: None,
            last_import_dir: None,
            last_export_dir: None,
        }
    }
}

impl Settings {
    pub fn push_recent(&mut self, path: PathBuf) {
        self.recent.retain(|p| p != &path);
        self.recent.insert(0, path);
        self.recent.truncate(MAX_RECENT);
    }
}
