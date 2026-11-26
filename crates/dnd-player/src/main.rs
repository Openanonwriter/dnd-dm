//! Desktop entry point: the same app in a phone-sized window, handy for
//! testing against a DM server on this machine.

fn main() -> eframe::Result {
    env_logger::init();
    let data_dir = std::env::var("DND_PLAYER_DATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| dirs_fallback().join("dnd-player"));
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([430.0, 880.0]).with_min_inner_size([360.0, 600.0]).with_title("dnd-player"),
        ..Default::default()
    };
    eframe::run_native("dnd-player", options, Box::new(move |cc| Ok(Box::new(dnd_player::PlayerApp::new(cc, data_dir)))))
}

fn dirs_fallback() -> std::path::PathBuf {
    if let Ok(p) = std::env::var("APPDATA") {
        return std::path::PathBuf::from(p);
    }
    if let Some(p) = std::env::var_os("XDG_DATA_HOME").filter(|p| !p.is_empty()) {
        return std::path::PathBuf::from(p);
    }
    if let Ok(p) = std::env::var("HOME") {
        return std::path::PathBuf::from(p).join(".local/share");
    }
    std::path::PathBuf::from(".")
}
