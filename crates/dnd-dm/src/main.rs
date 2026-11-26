#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod paths;
mod shot;
mod ui;

fn main() -> eframe::Result {
    let paths = paths::Paths::discover();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size(shot::size_from_env().unwrap_or([1360.0, 860.0])).with_min_inner_size([900.0, 600.0]).with_title("dnd-dm"),
        ..Default::default()
    };
    eframe::run_native("dnd-dm", options, Box::new(move |cc| Ok(Box::new(app::DmApp::new(cc, paths)))))
}
