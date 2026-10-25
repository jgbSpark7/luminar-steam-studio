use std::path::PathBuf;
use std::sync::mpsc::Receiver;

use eframe::egui;

use super::actions::Action;
use super::history::History;
use super::settings::Settings;
use crate::assets::{AssetStore, Slot};
use crate::editor::canvas::CanvasState;
use crate::errors::UserError;
use crate::export::pipeline::{ExportEvent, ExportReport};
use crate::project::Project;
use crate::steam::config::{self, ConfigSource};
use crate::steam::validation::{self, ArtworkState, Check, Facts};
use crate::steam::{LayoutSpec, SteamConfig};
use crate::ui::preview::PreviewState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Import,
    Edit,
    Slice,
    Preview,
    Export,
}

impl Step {
    pub const ALL: [Step; 5] = [Step::Import, Step::Edit, Step::Slice, Step::Preview, Step::Export];

    pub fn label(self) -> &'static str {
        match self {
            Step::Import => "Import",
            Step::Edit => "Edit",
            Step::Slice => "Slice",
            Step::Preview => "Preview",
            Step::Export => "Export",
        }
    }

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|s| *s == self).unwrap_or(0)
    }
}

/// What the inspector on the right is focused on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Artwork,
    Background,
    Avatar,
    Showcase,
    Templates,
    Profile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Success,
    Warning,
}

pub struct Toast {
    pub text: String,
    pub kind: ToastKind,
    pub born: f64,
}

#[derive(Default)]
pub struct ExportState {
    pub rx: Option<Receiver<ExportEvent>>,
    pub progress: f32,
    pub label: String,
    pub report: Option<ExportReport>,
}

impl ExportState {
    pub fn running(&self) -> bool {
        self.rx.is_some()
    }
}

#[derive(Default)]
pub struct Windows {
    pub settings: bool,
    pub upload_guide: bool,
    pub shortcuts: bool,
}

pub struct AppState {
    pub project: Project,
    pub project_path: Option<PathBuf>,
    pub saved: Project,
    pub config: SteamConfig,
    pub config_source: ConfigSource,
    pub config_warning: Option<String>,
    pub assets: AssetStore,
    pub history: History,
    pub step: Step,
    pub panel: Panel,
    /// The Info tab replaces the central view while open.
    pub info_open: bool,
    pub canvas: CanvasState,
    pub preview: PreviewState,
    pub export: ExportState,
    pub settings: Settings,
    pub toasts: Vec<Toast>,
    pub error: Option<UserError>,
    pub confirm_discard: Option<Action>,
    pub windows: Windows,
    pub actions: Vec<Action>,
    pub checks: Vec<Check>,
    pub upload_done: Vec<bool>,
    pub visited: [bool; 5],
    pub dropped: Vec<PathBuf>,
    pub quit_confirmed: bool,
    title: String,
}

impl AppState {
    pub fn new(settings: Settings) -> Self {
        let loaded = config::load();
        let project = Project::new_for(&loaded.config.layouts[0]);
        let mut canvas = CanvasState::default();
        canvas.snapping = settings.snapping;
        canvas.show_guides = settings.show_guides;
        Self {
            history: History::new(&project),
            saved: project.clone(),
            project,
            project_path: None,
            config: loaded.config,
            config_source: loaded.source,
            config_warning: loaded.warning,
            assets: AssetStore::default(),
            step: Step::Import,
            panel: Panel::Artwork,
            info_open: false,
            canvas,
            preview: PreviewState::default(),
            export: ExportState::default(),
            settings,
            toasts: Vec::new(),
            error: None,
            confirm_discard: None,
            windows: Windows::default(),
            actions: Vec::new(),
            checks: Vec::new(),
            upload_done: vec![false; 12],
            visited: [true, false, false, false, false],
            dropped: Vec::new(),
            quit_confirmed: false,
            title: String::new(),
        }
    }

    pub fn layout(&self) -> &LayoutSpec {
        self.config.layout(&self.project.layout_id)
    }

    pub fn is_dirty(&self) -> bool {
        self.project != self.saved
    }

    pub fn toast(&mut self, ctx: &egui::Context, kind: ToastKind, text: impl Into<String>) {
        let born = ctx.input(|i| i.time);
        self.toasts.push(Toast { text: text.into(), kind, born });
    }

    pub fn push(&mut self, action: Action) {
        self.actions.push(action);
    }

    pub fn step_complete(&self, step: Step) -> bool {
        match step {
            Step::Import => self.assets.get(Slot::Artwork).is_some(),
            Step::Export => self.export.report.is_some(),
            other => self.visited[other.index()] && self.assets.get(Slot::Artwork).is_some(),
        }
    }

    pub fn begin_frame(&mut self, ctx: &egui::Context) {
        if let Some(err) = self.assets.poll(ctx).pop() {
            self.error = Some(err);
        }
        self.assets.sync(&self.project, ctx);
        self.poll_export(ctx);

        self.dropped = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
        self.handle_shortcuts(ctx);
        self.visited[self.step.index()] = true;
        self.refresh_checks();
    }

    pub fn refresh_checks(&mut self) {
        let artwork = match self.assets.get(Slot::Artwork) {
            Some(a) if self.project.artwork.is_some() => ArtworkState::Ready {
                size: a.size(),
                summary: a.info.summary(),
                animated: match a.info.frames {
                    crate::assets::animation::FrameInfo::Animated { frames } => Some(frames),
                    _ => None,
                },
                has_alpha: a.info.has_alpha,
            },
            _ if self.project.artwork.is_none() => ArtworkState::Missing,
            _ if self.assets.has_failed(Slot::Artwork) => ArtworkState::Failed,
            _ => ArtworkState::Loading,
        };
        let facts = Facts {
            artwork,
            background_loaded: self.assets.get(Slot::Background).is_some(),
            avatar_loaded: self.assets.get(Slot::Avatar).is_some(),
        };
        let layout = self.config.layout(&self.project.layout_id);
        self.checks = validation::run(&self.project, layout, &self.config, &facts);
    }

    fn poll_export(&mut self, ctx: &egui::Context) {
        let Some(rx) = &self.export.rx else { return };
        let mut finished = None;
        while let Ok(event) = rx.try_recv() {
            match event {
                ExportEvent::Progress { fraction, label } => {
                    self.export.progress = fraction;
                    self.export.label = label;
                }
                ExportEvent::Finished(result) => finished = Some(result),
            }
        }
        if let Some(result) = finished {
            self.export.rx = None;
            match result {
                Ok(report) => {
                    let n = report.files.len();
                    self.export.report = Some(report);
                    self.toast(ctx, ToastKind::Success, format!("Exported {n} files"));
                }
                Err(err) => self.error = Some(err),
            }
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        use egui::{Key, KeyboardShortcut, Modifiers};
        let cmd = |k| KeyboardShortcut::new(Modifiers::COMMAND, k);
        let cmd_shift = |k| KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, k);
        let fire = |s: KeyboardShortcut| ctx.input_mut(|i| i.consume_shortcut(&s));

        // Order matters: shift variants must be consumed before their plain versions.
        if fire(cmd_shift(Key::S)) {
            self.push(Action::SaveAs);
        }
        if fire(cmd_shift(Key::Z)) || fire(cmd(Key::Y)) {
            self.push(Action::Redo);
        }
        if fire(cmd(Key::Z)) {
            self.push(Action::Undo);
        }
        if fire(cmd(Key::S)) {
            self.push(Action::Save);
        }
        if fire(cmd(Key::O)) {
            self.push(Action::OpenProjectDialog);
        }
        if fire(cmd(Key::N)) {
            self.push(Action::NewProject);
        }
        if fire(cmd(Key::I)) {
            self.push(Action::ImportDialog(Slot::Artwork));
        }
        if fire(cmd(Key::E)) {
            self.push(Action::StartExport);
        }
        for (key, step) in [Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5].into_iter().zip(Step::ALL) {
            if fire(cmd(key)) {
                self.push(Action::GoTo(step));
            }
        }
    }

    pub fn process_actions(&mut self, ctx: &egui::Context) {
        // Actions may enqueue follow-ups, so drain until quiet.
        for _ in 0..8 {
            let actions = std::mem::take(&mut self.actions);
            if actions.is_empty() {
                break;
            }
            for a in actions {
                self.apply(a, ctx);
            }
        }
    }

    pub fn end_frame(&mut self, ctx: &egui::Context) {
        let interacting = ctx.input(|i| i.pointer.any_down()) || ctx.wants_keyboard_input();
        self.history.observe(&self.project, interacting);

        let now = ctx.input(|i| i.time);
        self.toasts.retain(|t| now - t.born < 4.0);
        if !self.toasts.is_empty() || self.export.running() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }

        let title = format!(
            "{}{} — {}",
            if self.is_dirty() { "● " } else { "" },
            self.project.name,
            crate::about::APP_NAME
        );
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }

        if ctx.input(|i| i.viewport().close_requested()) && self.is_dirty() && !self.quit_confirmed {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.confirm_discard = Some(Action::Quit);
        }
    }
}
