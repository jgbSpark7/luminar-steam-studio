#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod about;
mod app;
mod assets;
mod editor;
mod errors;
mod export;
mod project;
mod steam;
mod templates;
mod ui;

use std::path::PathBuf;

use eframe::egui;

fn main() -> eframe::Result<()> {
    // A project path passed on the command line (e.g. via file association) opens on launch.
    let open_on_start = std::env::args_os().nth(1).map(PathBuf::from);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(about::APP_NAME)
            .with_app_id("luminar-steam-studio")
            .with_inner_size([1480.0, 920.0])
            .with_min_inner_size([1120.0, 700.0])
            .with_drag_and_drop(true)
            .with_icon(ui::theme::app_icon()),
        ..Default::default()
    };
    eframe::run_native(
        about::APP_NAME,
        options,
        Box::new(move |cc| Ok(Box::new(app::LuminarApp::new(cc, open_on_start)))),
    )
}
