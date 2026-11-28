//! A practice table: a real server with the cellar fight on it, where the
//! goblins and the other hero play themselves. Connect a phone (or the
//! desktop player) and the first device to arrive plays Thia the wizard.
//! When the goblins are beaten, two more come. For trying the player app
//! through a whole fight without a DM.
//!
//! ```text
//! cargo run --release -p dnd-player --example table            # this machine only, port 8421
//! cargo run --release -p dnd-player --example table -- --lan   # reachable from the Wi-Fi
//! ```
//! Over USB: `adb reverse tcp:8421 tcp:8421`, then connect to 127.0.0.1:8421.

#[path = "shared/cellar.rs"]
mod cellar;
use dnd_core::model::CombatantKind;
use dnd_core::play::RollMode;
use std::sync::{Arc, RwLock};
use std::time::Duration;

fn main() {
    let lan = std::env::args().any(|a| a == "--lan");
    let port: u16 = std::env::var("DND_TABLE_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8421);
    let mut w = cellar::world();
    w.campaign.name = "Practice table".into();
    w.campaign.devices.clear();
    // everyone within reach and in plain sight, so the fight happens without anyone having to walk
    for (id, at) in [("goblin_1", dnd_core::grid::Pos::new(4, 4)), ("goblin_2", dnd_core::grid::Pos::new(4, 2))] {
        if let Some(c) = w.campaign.encounter.combatants.iter_mut().find(|c| c.id == id) {
            c.pos = Some(at);
        }
    }
    w.update_explored();
    let world = Arc::new(RwLock::new(w));
    let server = if lan { dnd_server::start(world.clone(), port) } else { dnd_server::start_local(world.clone(), port) }.expect("server");
    println!("practice table on port {} ({}); Ctrl-C to stop", server.addr.port(), if lan { "open to the network" } else { "this machine only" });

    loop {
        std::thread::sleep(Duration::from_millis(1800));
        let mut w = world.write().unwrap();
        // the first device without a character plays the wizard
        if !w.campaign.devices.iter().any(|d| d.character == "thia") {
            if let Some(id) = w.campaign.devices.iter().find(|d| d.character.is_empty()).map(|d| d.id.clone()) {
                w.campaign.assign_device(&id, Some("thia"));
                w.touch();
                println!("{id} plays Thia");
            }
            continue;
        }
        if !w.campaign.encounter.active {
            continue;
        }
        let Some(cur) = w.campaign.encounter.current().cloned() else { continue };
        let seated: Vec<String> = w.campaign.devices.iter().map(|d| d.character.clone()).filter(|c| !c.is_empty()).collect();
        let is_pc = matches!(cur.kind, CombatantKind::Pc { .. });
        if is_pc && seated.contains(&cur.id) {
            continue; // a person's turn: wait for them
        }
        let foes: Vec<String> = w.campaign.encounter.combatants.iter().filter(|c| !c.is_down() && matches!(c.kind, CombatantKind::Pc { .. }) != is_pc).map(|c| c.id.clone()).collect();
        if foes.is_empty() && is_pc {
            // the heroes won: bring the goblins back so the practice goes on
            for c in w.campaign.encounter.combatants.iter_mut().filter(|c| !matches!(c.kind, CombatantKind::Pc { .. })) {
                c.hp = c.hp_max;
                c.conditions.clear();
            }
            w.campaign.say("", "Two more goblins crawl out of the dark.", "turn");
            w.touch();
            continue;
        }
        if let Some(target) = foes.first() {
            let tries: &[usize] = if is_pc { &[0] } else { &[1, 0] };
            let mut done = false;
            for a in tries {
                let r = if is_pc { w.pc_attack(&cur.id, *a, target, RollMode::Normal) } else { w.creature_attack(&cur.id, *a, target, RollMode::Normal) };
                match r {
                    Ok(_) => {
                        done = true;
                        break;
                    }
                    Err(e) => println!("{} could not attack {target}: {e}", cur.name),
                }
            }
            if !done {
                w.campaign.say(&cur.name, "waits.", "turn");
            }
        }
        w.next_turn();
    }
}
