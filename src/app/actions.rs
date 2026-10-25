//! Everything that does more than tweak a value goes through an [`Action`]. UI code
//! only records intent; this module performs it.

use std::path::PathBuf;

use eframe::egui;
use eframe::emath::{pos2, Rect};

use super::state::{AppState, Panel, Step, ToastKind};
use crate::assets::image_loader::{self, SUPPORTED_EXTENSIONS};
use crate::assets::Slot;
use crate::errors::{display_name, UserError};
use crate::export::pipeline::{self, ExportJob};
use crate::project::serialization::{self, EXTENSION};
use crate::project::transform::{fit, natural_height, visible_rect, FitMode};
use crate::project::{ArtworkLayer, AvatarLayer, BackgroundLayer, ExportScale, Project};
use crate::steam::config::{self, override_path};
use crate::steam::validation::QuickFix;
use crate::steam::SteamConfig;
use crate::templates;

#[derive(Debug, Clone)]
pub enum Action {
    NewProject,
    OpenProjectDialog,
    OpenProjectPath(PathBuf),
    Save,
    SaveAs,
    /// Runs the inner action without the unsaved-changes prompt.
    Forced(Box<Action>),
    /// Saves, then runs the inner action only if the save succeeded.
    SaveThen(Box<Action>),
    Quit,

    ImportDialog(Slot),
    ImportPath(Slot, PathBuf),
    RemoveAsset(Slot),
    ReloadAsset(Slot),

    Fit(FitMode),
    CenterArtwork,
    ResetArtwork,
    MatchHeightToArtwork,
    FitBackgroundToReference,
    SetLayout(String),
    ApplyTemplate(&'static str),
    Fix(QuickFix),

    ChooseExportFolder,
    StartExport,
    OpenPath(PathBuf),

    Undo,
    Redo,
    GoTo(Step),
    Focus(Panel),
    ShowInfo,
    OpenUrl(String),
    Copy { text: String, what: &'static str },
    ReloadConfig,
    WriteConfigCopy,
}

impl AppState {
    pub fn apply(&mut self, action: Action, ctx: &egui::Context) {
        match action {
            Action::NewProject | Action::OpenProjectDialog | Action::OpenProjectPath(_) | Action::Quit
                if self.is_dirty() =>
            {
                self.confirm_discard = Some(action);
            }
            Action::Forced(inner) => self.run_unguarded(*inner, ctx),
            Action::SaveThen(inner) => {
                if self.save(false, ctx) {
                    self.run_unguarded(*inner, ctx);
                }
            }
            other => self.run_unguarded(other, ctx),
        }
    }

    fn run_unguarded(&mut self, action: Action, ctx: &egui::Context) {
        match action {
            Action::NewProject => {
                let mut fresh = Project::new_for(&self.config.layouts[0]);
                fresh.export.folder = self.project.export.folder.clone();
                self.replace_project(fresh, None);
                self.step = Step::Import;
                self.toast(ctx, ToastKind::Info, "New project");
            }
            Action::OpenProjectDialog => {
                let mut dialog = rfd::FileDialog::new().add_filter("Luminar Steam Studio project", &[EXTENSION]);
                if let Some(dir) = &self.settings.last_open_dir {
                    dialog = dialog.set_directory(dir);
                }
                if let Some(path) = dialog.pick_file() {
                    self.run_unguarded(Action::OpenProjectPath(path), ctx);
                }
            }
            Action::OpenProjectPath(path) => match serialization::load(&path) {
                Ok(mut project) => {
                    if !self.config.has_layout(&project.layout_id) {
                        self.toast(ctx, ToastKind::Warning, "This project's layout isn't available; switched to the default.");
                        project.layout_id = self.config.layouts[0].id.clone();
                    }
                    self.settings.last_open_dir = path.parent().map(PathBuf::from);
                    self.settings.push_recent(path.clone());
                    self.replace_project(project, Some(path));
                    self.step = if self.project.artwork.is_some() { Step::Edit } else { Step::Import };
                    self.toast(ctx, ToastKind::Success, format!("Opened {}", self.project.name));
                }
                Err(e) => {
                    self.settings.recent.retain(|p| p != &path);
                    self.error = Some(e);
                }
            },
            Action::Save => {
                self.save(false, ctx);
            }
            Action::SaveAs => {
                self.save(true, ctx);
            }
            Action::Forced(inner) | Action::SaveThen(inner) => self.run_unguarded(*inner, ctx),
            Action::Quit => {
                self.quit_confirmed = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }

            Action::ImportDialog(slot) => {
                let mut dialog = rfd::FileDialog::new()
                    .set_title(format!("Choose {}", slot.label().to_lowercase()))
                    .add_filter("Images", SUPPORTED_EXTENSIONS);
                if let Some(dir) = &self.settings.last_import_dir {
                    dialog = dialog.set_directory(dir);
                }
                if let Some(path) = dialog.pick_file() {
                    self.import(slot, path, ctx);
                }
            }
            Action::ImportPath(slot, path) => self.import(slot, path, ctx),
            Action::RemoveAsset(slot) => match slot {
                Slot::Artwork => self.project.artwork = None,
                Slot::Background => self.project.background = None,
                Slot::Avatar => self.project.avatar = None,
            },
            Action::ReloadAsset(slot) => self.assets.reload(slot, &self.project, ctx),

            Action::Fit(mode) => self.fit_artwork(mode),
            Action::CenterArtwork => {
                let comp = self.layout().size(self.project.art_height);
                if let (Some(art), Some(asset)) = (&mut self.project.artwork, self.assets.get(Slot::Artwork)) {
                    // Centre the *visible* (cropped) image, at the current scale and rotation.
                    let size = asset.size();
                    let vis_center = visible_rect(size, &art.crop).center();
                    let offset = art.transform.source_to_comp(size, vis_center) - art.transform.center_pos();
                    art.transform.set_center((comp / 2.0).to_pos2() - offset);
                }
            }
            Action::ResetArtwork => {
                if let Some(art) = &mut self.project.artwork {
                    art.crop = Default::default();
                    art.transform.rotation_deg = 0.0;
                    art.transform.flip_x = false;
                }
                self.fit_artwork(FitMode::Cover);
            }
            Action::MatchHeightToArtwork => {
                let width = self.layout().width();
                if let (Some(art), Some(asset)) = (&self.project.artwork, self.assets.get(Slot::Artwork)) {
                    let h = natural_height(asset.size(), &art.crop, art.transform.rotation_deg, width);
                    self.project.art_height = self.layout().clamp_height(h);
                    self.fit_artwork(FitMode::Cover);
                }
            }
            Action::FitBackgroundToReference => {
                let reference = self.config.background.reference_width;
                if let (Some(bg), Some(asset)) = (&mut self.project.background, self.assets.get(Slot::Background)) {
                    bg.scale = reference / asset.size().x.max(1.0);
                    bg.offset = [0.0, 0.0];
                }
            }
            Action::SetLayout(id) => {
                if id != self.project.layout_id && self.config.has_layout(&id) {
                    self.project.layout_id = id;
                    let layout = self.layout().clone();
                    self.project.art_height = layout.clamp_height(self.project.art_height);
                    self.fit_artwork(FitMode::Cover);
                    self.canvas.needs_fit = true;
                }
            }
            Action::ApplyTemplate(id) => {
                if let Some(t) = templates::find(id) {
                    let before = self.project.layout_id.clone();
                    templates::apply(t, &mut self.project, &self.config);
                    if before != self.project.layout_id || self.project.artwork.is_some() {
                        self.fit_artwork(FitMode::Cover);
                    }
                    self.canvas.needs_fit = true;
                    self.toast(ctx, ToastKind::Info, format!("Applied {} template", t.name));
                }
            }
            Action::Fix(fix) => match fix {
                QuickFix::ImportArtwork => self.run_unguarded(Action::ImportDialog(Slot::Artwork), ctx),
                QuickFix::CoverRegions => self.fit_artwork(FitMode::Cover),
                QuickFix::ClampHeight => {
                    let h = self.layout().clamp_height(self.project.art_height);
                    self.project.art_height = h;
                }
                QuickFix::ChooseExportFolder => self.run_unguarded(Action::ChooseExportFolder, ctx),
                QuickFix::UseOneX => self.project.export.scale = ExportScale::X1,
            },

            Action::ChooseExportFolder => {
                self.choose_export_folder();
            }
            Action::StartExport => self.start_export(ctx),
            Action::OpenPath(path) => {
                if let Err(e) = open::that_detached(&path) {
                    self.error = Some(UserError::from_io("open", &path, &e));
                }
            }

            Action::Undo => {
                if self.history.undo(&mut self.project) {
                    self.toast(ctx, ToastKind::Info, "Undo");
                }
            }
            Action::Redo => {
                if self.history.redo(&mut self.project) {
                    self.toast(ctx, ToastKind::Info, "Redo");
                }
            }
            Action::GoTo(step) => {
                self.info_open = false;
                self.step = step;
                if step == Step::Edit || step == Step::Slice {
                    self.canvas.needs_fit = true;
                }
            }
            Action::Focus(panel) => {
                self.info_open = false;
                self.panel = panel;
            }
            Action::ShowInfo => self.info_open = true,
            Action::OpenUrl(url) => {
                // Only web links: URLs can come from a user-editable config file, and
                // handing the OS a file path or custom scheme could launch a program.
                if !url.starts_with("https://") {
                    self.error = Some(UserError::new("Link blocked", "Only https:// web links can be opened from the app.")
                        .hint(format!("The link was: {url}")));
                } else if open::that_detached(&url).is_err() {
                    ctx.copy_text(url);
                    self.toast(ctx, ToastKind::Warning, "Couldn't open your browser; the link was copied instead.");
                }
            }
            Action::Copy { text, what } => {
                ctx.copy_text(text);
                self.toast(ctx, ToastKind::Success, format!("Copied {what}"));
            }
            Action::ReloadConfig => {
                let loaded = config::load();
                self.config = loaded.config;
                self.config_source = loaded.source;
                self.config_warning = loaded.warning;
                if !self.config.has_layout(&self.project.layout_id) {
                    self.project.layout_id = self.config.layouts[0].id.clone();
                }
                self.canvas.needs_fit = true;
                self.toast(ctx, ToastKind::Success, "Steam settings reloaded");
            }
            Action::WriteConfigCopy => match override_path() {
                Some(path) if path.exists() => {
                    self.toast(ctx, ToastKind::Info, "An editable copy already exists");
                    self.run_unguarded(Action::OpenPath(path), ctx);
                }
                Some(path) => match std::fs::write(&path, SteamConfig::embedded_text()) {
                    Ok(()) => self.toast(ctx, ToastKind::Success, format!("Created {}", path.display())),
                    Err(e) => {
                        self.error = Some(UserError::from_io("create", &path, &e).hint(
                            "The app folder may be read-only. Move the app to a folder you own, such as Documents.",
                        ))
                    }
                },
                None => {
                    self.error = Some(UserError::new("Couldn't find the app folder", "The location of the program couldn't be determined."))
                }
            },
        }
    }

    fn replace_project(&mut self, project: Project, path: Option<PathBuf>) {
        self.project = project;
        self.saved = self.project.clone();
        self.project_path = path;
        self.history.reset(&self.project);
        self.export.report = None;
        self.canvas.needs_fit = true;
        self.upload_done.iter_mut().for_each(|d| *d = false);
        self.visited = [true, false, false, false, false];
    }

    /// Returns true if the project ended up saved.
    fn save(&mut self, force_dialog: bool, ctx: &egui::Context) -> bool {
        let path = match (&self.project_path, force_dialog) {
            (Some(p), false) => p.clone(),
            _ => {
                let mut dialog = rfd::FileDialog::new()
                    .add_filter("Luminar Steam Studio project", &[EXTENSION])
                    .set_file_name(format!("{}.{EXTENSION}", pipeline::sanitize_folder_name(&self.project.name)));
                if let Some(dir) = &self.settings.last_open_dir {
                    dialog = dialog.set_directory(dir);
                }
                match dialog.save_file() {
                    Some(p) if p.extension().is_none() => p.with_extension(EXTENSION),
                    Some(p) => p,
                    None => return false,
                }
            }
        };
        match serialization::save(&self.project, &path) {
            Ok(()) => {
                self.saved = self.project.clone();
                self.settings.last_open_dir = path.parent().map(PathBuf::from);
                self.settings.push_recent(path.clone());
                self.toast(ctx, ToastKind::Success, format!("Saved {}", display_name(&path)));
                self.project_path = Some(path);
                true
            }
            Err(e) => {
                self.error = Some(e);
                false
            }
        }
    }

    fn import(&mut self, slot: Slot, path: PathBuf, ctx: &egui::Context) {
        if image_loader::is_project_file(&path) {
            self.apply(Action::OpenProjectPath(path), ctx);
            return;
        }
        let (w, h) = match image_loader::probe_dimensions(&path) {
            Ok(d) => d,
            Err(e) => {
                self.error = Some(e);
                return;
            }
        };
        self.settings.last_import_dir = path.parent().map(PathBuf::from);
        let size = eframe::emath::vec2(w as f32, h as f32);
        match slot {
            Slot::Artwork => {
                let first = self.project.artwork.is_none();
                let layout = self.layout().clone();
                if first {
                    // Size the showcase to the image's own proportions so nothing is cropped by default.
                    let natural = natural_height(size, &Default::default(), 0.0, layout.width());
                    self.project.art_height = layout.clamp_height(natural);
                }
                let comp = Rect::from_min_size(pos2(0.0, 0.0), layout.size(self.project.art_height));
                let transform = fit(size, &Default::default(), 0.0, false, comp, FitMode::Cover);
                self.project.artwork = Some(ArtworkLayer { source: path.clone(), transform, crop: Default::default() });
                if self.project.name == Project::default().name {
                    if let Some(stem) = path.file_stem() {
                        self.project.name = stem.to_string_lossy().into_owned();
                    }
                }
                self.panel = Panel::Artwork;
                if self.step == Step::Import {
                    self.step = Step::Edit;
                }
                self.canvas.needs_fit = true;
                self.toast(ctx, ToastKind::Success, "Artwork placed. Drag to reposition, scroll to zoom.");
            }
            Slot::Background => {
                let scale = self.config.background.reference_width / size.x.max(1.0);
                self.project.background = Some(BackgroundLayer { source: path, scale, ..Default::default() });
                self.panel = Panel::Background;
                self.toast(ctx, ToastKind::Success, "Background added");
            }
            Slot::Avatar => {
                self.project.avatar = Some(AvatarLayer { source: path, ..Default::default() });
                self.panel = Panel::Avatar;
                self.toast(ctx, ToastKind::Success, "Avatar added");
            }
        }
    }

    pub fn fit_artwork(&mut self, mode: FitMode) {
        let comp = Rect::from_min_size(pos2(0.0, 0.0), self.layout().size(self.project.art_height));
        if let (Some(art), Some(asset)) = (&mut self.project.artwork, self.assets.get(Slot::Artwork)) {
            art.transform = fit(asset.size(), &art.crop, art.transform.rotation_deg, art.transform.flip_x, comp, mode);
        }
    }

    fn choose_export_folder(&mut self) -> bool {
        let mut dialog = rfd::FileDialog::new().set_title("Choose where to export");
        if let Some(dir) = self.project.export.folder.as_ref().or(self.settings.last_export_dir.as_ref()) {
            dialog = dialog.set_directory(dir);
        }
        match dialog.pick_folder() {
            Some(folder) => {
                self.settings.last_export_dir = Some(folder.clone());
                self.project.export.folder = Some(folder);
                true
            }
            None => false,
        }
    }

    fn start_export(&mut self, ctx: &egui::Context) {
        if self.export.running() {
            return;
        }
        self.step = Step::Export;
        let Some(artwork) = self.assets.get(Slot::Artwork).map(|a| a.full.clone()) else {
            self.error = Some(if self.project.artwork.is_some() {
                UserError::new("Artwork is still loading", "Wait a moment and try exporting again.")
            } else {
                UserError::new("Nothing to export yet", "Add artwork before exporting.").hint("Drop an image onto the window or press Ctrl+I.")
            });
            return;
        };
        if self.project.export.folder.is_none() && !self.choose_export_folder() {
            return;
        }
        let base = self.project.export.folder.clone().expect("folder chosen above");
        let folder = base.join(pipeline::sanitize_folder_name(&self.project.name));
        let project_json = if self.project.export.include_project {
            serialization::to_shareable_json(&self.project).ok()
        } else {
            None
        };
        let job = ExportJob {
            project: self.project.clone(),
            layout: self.layout().clone(),
            profile: self.config.profile.clone(),
            avatar_size: self.config.avatar.export_size,
            size_warning_bytes: (self.config.file_size_warning_mb * 1_048_576.0) as u64,
            artwork,
            background: self.assets.get(Slot::Background).map(|a| a.full.clone()),
            avatar: self.assets.get(Slot::Avatar).map(|a| a.full.clone()),
            folder,
            project_json,
        };
        self.export.report = None;
        self.export.progress = 0.0;
        self.export.label = "Starting".into();
        self.export.rx = Some(pipeline::spawn(job, ctx.clone()));
    }
}
