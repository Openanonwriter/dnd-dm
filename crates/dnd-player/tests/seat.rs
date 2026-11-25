//! The real network worker against a real server: join by name, wait, get a
//! character pushed by the DM, lose it again.

use dnd_core::{Character, World};
use dnd_player::client::{Client, Cmd, Evt};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

fn wait_for<T>(c: &Client, what: &str, mut f: impl FnMut(Evt) -> Option<T>) -> T {
    let end = Instant::now() + Duration::from_secs(10);
    while Instant::now() < end {
        if let Ok(e) = c.rx.recv_timeout(Duration::from_millis(200)) {
            if let Some(v) = f(e) {
                return v;
            }
        }
    }
    panic!("timed out waiting for {what}");
}

#[test]
fn dm_assignment_reaches_the_phone() {
    let mut world = World::new(Vec::new());
    world.campaign.characters.push(Character::new("thia", "Thia"));
    let world = Arc::new(RwLock::new(world));
    let server = dnd_server::start_local(world.clone(), 0).unwrap();
    let url = format!("http://127.0.0.1:{}", server.addr.port());

    let client = Client::spawn(egui::Context::default());
    client.tx.send(Cmd::Connect { url, code: String::new(), device_id: "device-sam-0001".into(), name: "Sam".into() }).unwrap();
    let seat = wait_for(&client, "first seat", |e| match e {
        Evt::Seat { seat, fresh: true } => Some(seat),
        _ => None,
    });
    assert_eq!(seat.name, "Sam");
    assert_eq!(seat.character, None);
    assert!(world.read().unwrap().device_online("device-sam-0001"));

    // DM assigns on the dashboard; the websocket nudges the phone, no user action
    std::thread::sleep(Duration::from_millis(600));
    {
        let mut w = world.write().unwrap();
        assert!(w.campaign.assign_device("device-sam-0001", Some("thia")));
        w.touch();
    }
    let got = wait_for(&client, "assignment", |e| match e {
        Evt::Seat { seat, .. } if seat.character.is_some() => seat.character,
        _ => None,
    });
    assert_eq!(got, "thia");

    // the app then loads that sheet
    client.tx.send(Cmd::LoadCharacter(got)).unwrap();
    let name = wait_for(&client, "sheet", |e| match e {
        Evt::Character { character, .. } => Some(character.name),
        _ => None,
    });
    assert_eq!(name, "Thia");

    // and the DM can take it away again
    {
        let mut w = world.write().unwrap();
        w.campaign.assign_device("device-sam-0001", None);
        w.touch();
    }
    wait_for(&client, "unassignment", |e| match e {
        Evt::Seat { seat, .. } if seat.character.is_none() => Some(()),
        _ => None,
    });
    client.tx.send(Cmd::Shutdown).unwrap();
    server.stop();
}
