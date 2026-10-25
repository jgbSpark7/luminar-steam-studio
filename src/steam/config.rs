use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::layouts::LayoutSpec;
use super::profile::ProfileGeometry;

const EMBEDDED: &str = include_str!("steam_config.json");
pub const OVERRIDE_FILE_NAME: &str = "steam_config.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvatarSpec {
    pub export_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundSpec {
    pub reference_width: f32,
    pub reference_height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadSpec {
    pub artwork_upload_url: String,
    pub profile_url: String,
    pub showcase_edit_url: String,
    pub avatar_edit_url: String,
    pub long_image_snippet: String,
    pub snippet_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamConfig {
    pub schema: u32,
    #[serde(default)]
    pub notes: Vec<String>,
    pub file_size_warning_mb: f32,
    pub profile: ProfileGeometry,
    pub avatar: AvatarSpec,
    pub background: BackgroundSpec,
    pub upload: UploadSpec,
    pub layouts: Vec<LayoutSpec>,
}

impl SteamConfig {
    pub fn embedded() -> Self {
        serde_json::from_str(EMBEDDED).expect("embedded steam_config.json must be valid")
    }

    pub fn embedded_text() -> &'static str {
        EMBEDDED
    }

    /// Falls back to the first layout so a stale project can never crash the editor.
    pub fn layout(&self, id: &str) -> &LayoutSpec {
        self.layouts
            .iter()
            .find(|l| l.id == id)
            .unwrap_or(&self.layouts[0])
    }

    pub fn has_layout(&self, id: &str) -> bool {
        self.layouts.iter().any(|l| l.id == id)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.layouts.is_empty() {
            return Err("No layouts are defined.".into());
        }
        for l in &self.layouts {
            l.validate()?;
        }
        if self.avatar.export_size == 0 {
            return Err("Avatar export size must be greater than zero.".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConfigSource {
    Embedded,
    File(PathBuf),
}

pub struct LoadedConfig {
    pub config: SteamConfig,
    pub source: ConfigSource,
    pub warning: Option<String>,
}

pub fn override_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    Some(exe.parent()?.join(OVERRIDE_FILE_NAME))
}

pub fn load() -> LoadedConfig {
    let embedded = LoadedConfig {
        config: SteamConfig::embedded(),
        source: ConfigSource::Embedded,
        warning: None,
    };
    let Some(path) = override_path().filter(|p| p.exists()) else {
        return embedded;
    };
    let parsed = std::fs::read_to_string(&path)
        .map_err(|e| format!("Couldn't read it ({e})."))
        .and_then(|text| {
            serde_json::from_str::<SteamConfig>(&text)
                .map_err(|e| format!("It isn't valid: line {}, column {}.", e.line(), e.column()))
        })
        .and_then(|cfg| cfg.validate().map(|_| cfg));

    match parsed {
        Ok(config) => LoadedConfig {
            config,
            source: ConfigSource::File(path),
            warning: None,
        },
        Err(why) => LoadedConfig {
            warning: Some(format!(
                "Your custom steam_config.json was ignored. {why} The built-in Steam settings are being used instead."
            )),
            ..embedded
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_config_is_valid() {
        let cfg = SteamConfig::embedded();
        cfg.validate().unwrap();
        let art = cfg.layout("artwork_showcase");
        assert_eq!(art.regions[0].width, 506.0);
        assert_eq!(art.regions[1].width, 100.0);
        assert_eq!(cfg.layout("featured_artwork").width(), 630.0);
        assert_eq!(art.gaps(800.0).len(), 1);
    }

    #[test]
    fn rejects_unsafe_export_names() {
        let mut cfg = SteamConfig::embedded();
        cfg.layouts[0].regions[0].file_stem = "../../Desktop/evil".into();
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn unknown_layout_falls_back() {
        let cfg = SteamConfig::embedded();
        assert_eq!(cfg.layout("does-not-exist").id, cfg.layouts[0].id);
    }
}
