//! dnd-player: a character sheet in the player's pocket. Talks to the DM's
//! `dnd-dm` server over Wi-Fi, caches the last sheet for offline reading, and
//! lets the player track HP, slots, conditions, items and notes.

pub mod app;
pub mod client;
pub mod shot;
pub mod store;
pub mod theme;
pub mod ui;

pub use app::PlayerApp;

/// The Android activity handle, kept so the UI can ask for the content rect
/// (the area not covered by the status bar / navigation bar).
#[cfg(target_os = "android")]
pub static ANDROID_APP: std::sync::OnceLock<winit::platform::android::activity::AndroidApp> = std::sync::OnceLock::new();

/// (top, bottom) insets in logical points that the system bars cover.
pub fn system_insets(ctx: &egui::Context) -> (f32, f32) {
    #[cfg(target_os = "android")]
    {
        if let Some(app) = ANDROID_APP.get() {
            let r = app.content_rect();
            let ppp = ctx.pixels_per_point().max(0.5);
            let screen_h = ctx.screen_rect().height();
            let top = (r.top as f32 / ppp).max(0.0);
            let bottom = (screen_h - r.bottom as f32 / ppp).max(0.0);
            if top < 200.0 && bottom < 200.0 {
                return (top, bottom);
            }
        }
        (24.0, 16.0)
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = ctx;
        (0.0, 0.0)
    }
}

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(android_app: winit::platform::android::activity::AndroidApp) {
    android_logger::init_once(android_logger::Config::default().with_max_level(log::LevelFilter::Info).with_tag("dnd-player"));
    let _ = ANDROID_APP.set(android_app.clone());
    let data_dir = android_app.internal_data_path().unwrap_or_else(|| std::path::PathBuf::from("/data/local/tmp"));
    let options = eframe::NativeOptions { android_app: Some(android_app), ..Default::default() };
    if let Err(e) = eframe::run_native("dnd-player", options, Box::new(move |cc| Ok(Box::new(PlayerApp::new(cc, data_dir))))) {
        log::error!("eframe exited: {e}");
    }
}
