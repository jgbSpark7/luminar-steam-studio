mod actions;
mod history;
pub mod settings;
pub mod state;

use std::path::PathBuf;

use eframe::egui;

pub use actions::Action;
pub use state::{AppState, Panel, Step, ToastKind};

pub struct LuminarApp {
    state: AppState,
}

impl LuminarApp {
    pub fn new(cc: &eframe::CreationContext<'_>, open_on_start: Option<PathBuf>) -> Self {
        crate::ui::theme::apply(&cc.egui_ctx);
        let settings: settings::Settings = cc
            .storage
            .and_then(|s| eframe::get_value(s, settings::STORAGE_KEY))
            .unwrap_or_default();
        cc.egui_ctx.set_zoom_factor(settings.ui_scale);
        let mut state = AppState::new(settings);
        if let Some(path) = open_on_start {
            state.push(Action::OpenProjectPath(path));
        }
        Self { state }
    }
}

impl eframe::App for LuminarApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let s = &mut self.state;
        s.begin_frame(ctx);
        crate::ui::draw(ctx, s);
        s.process_actions(ctx);
        s.end_frame(ctx);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.state.settings.snapping = self.state.canvas.snapping;
        self.state.settings.show_guides = self.state.canvas.show_guides;
        eframe::set_value(storage, settings::STORAGE_KEY, &self.state.settings);
    }
}
