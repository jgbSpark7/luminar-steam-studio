use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

use eframe::egui::{self, TextureHandle, TextureOptions};
use eframe::emath::{vec2, Vec2};
use image::RgbaImage;

use super::image_loader::{decode, DecodedImage, ImageInfo};
use crate::errors::UserError;
use crate::project::Project;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Slot {
    Artwork,
    Background,
    Avatar,
}

impl Slot {
    pub const ALL: [Slot; 3] = [Slot::Artwork, Slot::Background, Slot::Avatar];

    pub fn label(self) -> &'static str {
        match self {
            Slot::Artwork => "Artwork",
            Slot::Background => "Background",
            Slot::Avatar => "Avatar",
        }
    }

    pub fn path_in(self, p: &Project) -> Option<&Path> {
        match self {
            Slot::Artwork => p.artwork.as_ref().map(|a| a.source.as_path()),
            Slot::Background => p.background.as_ref().map(|a| a.source.as_path()),
            Slot::Avatar => p.avatar.as_ref().map(|a| a.source.as_path()),
        }
    }
}

pub struct LoadedAsset {
    pub path: PathBuf,
    /// Full-resolution pixels, shared with export threads without copying.
    pub full: Arc<RgbaImage>,
    /// Downscaled GPU texture used only for on-screen drawing.
    pub texture: TextureHandle,
    pub info: ImageInfo,
}

impl LoadedAsset {
    pub fn size(&self) -> Vec2 {
        vec2(self.info.width as f32, self.info.height as f32)
    }
}

type LoadResult = (Slot, PathBuf, Result<DecodedImage, UserError>);

/// Keeps decoded images in sync with whatever paths the project references.
/// Undo, open and import all just change the project; this notices and reloads.
pub struct AssetStore {
    loaded: HashMap<Slot, LoadedAsset>,
    pending: HashMap<Slot, PathBuf>,
    failed: HashMap<Slot, PathBuf>,
    tx: Sender<LoadResult>,
    rx: Receiver<LoadResult>,
}

impl Default for AssetStore {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self { loaded: HashMap::new(), pending: HashMap::new(), failed: HashMap::new(), tx, rx }
    }
}

impl AssetStore {
    pub fn get(&self, slot: Slot) -> Option<&LoadedAsset> {
        self.loaded.get(&slot)
    }

    pub fn is_loading(&self, slot: Slot) -> bool {
        self.pending.contains_key(&slot)
    }

    pub fn has_failed(&self, slot: Slot) -> bool {
        self.failed.contains_key(&slot)
    }

    pub fn sync(&mut self, project: &Project, ctx: &egui::Context) {
        for slot in Slot::ALL {
            match slot.path_in(project) {
                None => {
                    self.loaded.remove(&slot);
                    self.pending.remove(&slot);
                    self.failed.remove(&slot);
                }
                Some(want) => {
                    let up_to_date = self.loaded.get(&slot).is_some_and(|a| a.path == want)
                        || self.pending.get(&slot).is_some_and(|p| p == want)
                        || self.failed.get(&slot).is_some_and(|p| p == want);
                    if !up_to_date {
                        self.request(slot, want.to_path_buf(), ctx);
                    }
                }
            }
        }
    }

    /// Forces a reload even if the path is unchanged (e.g. the file was edited on disk).
    pub fn reload(&mut self, slot: Slot, project: &Project, ctx: &egui::Context) {
        if let Some(p) = slot.path_in(project) {
            self.request(slot, p.to_path_buf(), ctx);
        }
    }

    fn request(&mut self, slot: Slot, path: PathBuf, ctx: &egui::Context) {
        self.pending.insert(slot, path.clone());
        self.failed.remove(&slot);
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        std::thread::Builder::new()
            .name(format!("load-{}", slot.label()))
            .spawn(move || {
                let result = decode(&path);
                let _ = tx.send((slot, path, result));
                ctx.request_repaint();
            })
            .expect("failed to spawn image loader thread");
    }

    /// Installs finished loads and returns any load failures.
    pub fn poll(&mut self, ctx: &egui::Context) -> Vec<UserError> {
        let mut errors = Vec::new();
        while let Ok((slot, path, result)) = self.rx.try_recv() {
            // Ignore results for a path the project has since moved away from.
            if self.pending.get(&slot) != Some(&path) {
                continue;
            }
            self.pending.remove(&slot);
            match result {
                Ok(decoded) => {
                    let options = TextureOptions {
                        magnification: egui::TextureFilter::Linear,
                        minification: egui::TextureFilter::Linear,
                        wrap_mode: egui::TextureWrapMode::ClampToEdge,
                        mipmap_mode: Some(egui::TextureFilter::Linear),
                    };
                    let texture = ctx.load_texture(format!("asset-{}", slot.label()), decoded.preview, options);
                    self.loaded.insert(
                        slot,
                        LoadedAsset { path: decoded.path, full: decoded.full, texture, info: decoded.info },
                    );
                }
                Err(err) => {
                    self.loaded.remove(&slot);
                    self.failed.insert(slot, path);
                    errors.push(err);
                }
            }
        }
        errors
    }
}
