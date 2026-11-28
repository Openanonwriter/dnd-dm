//! The player app against a little game running inside this same program:
//! a wizard and a fighter in a cellar with two goblins, the fight already on.
//! Nothing is opened to the network (the server listens on 127.0.0.1 only).
//!
//! ```text
//! cargo run --release -p dnd-player --example demo
//! ```
//! `DND_PLAYER_TAB` = stats, combat, spells, items, notes, map, dice opens
//! on that tab; `DND_PLAYER_SHOT=<file.bmp>` saves a picture and exits.

#[path = "shared/cellar.rs"]
mod cellar;
use cellar::{world, DEVICE};
use std::sync::{Arc, RwLock};

fn main() -> eframe::Result {
    let world = Arc::new(RwLock::new(world()));
    let server = dnd_server::start_local(world.clone(), 0).expect("server");
    let data_dir = std::env::temp_dir().join(format!("dnd-player-demo-{}", std::process::id()));
    let store = dnd_player::store::Store::new(data_dir.clone());
    store.save_config(&dnd_player::store::Config {
        server_url: format!("http://127.0.0.1:{}", server.addr.port()),
        player_name: "Demo player".into(),
        device_id: DEVICE.into(),
        character_id: "thia".into(),
        ..Default::default()
    });
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([430.0, 880.0]).with_title("dnd-player demo"),
        ..Default::default()
    };
    let r = eframe::run_native("dnd-player demo", options, Box::new(move |cc| Ok(Box::new(dnd_player::PlayerApp::new(cc, data_dir)))));
    drop(server);
    r
}
