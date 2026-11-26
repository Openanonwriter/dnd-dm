//! The map over the wire: a player gets only what their character has seen,
//! walks their own token where the way is clear, and opens doors.

use dnd_core::grid::Pos;
use dnd_core::model::*;
use dnd_core::view::MapView;
use dnd_core::World;
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

fn next_map(c: &Client) -> MapView {
    wait_for(c, "map", |e| match e {
        Evt::Map(m) => Some(*m),
        _ => None,
    })
}

#[test]
fn player_sees_walks_and_opens_doors() {
    let mut w = World::default();
    let mut m = MapState::new("Cellar", 7, 5);
    m.cells = vec!["#######".into(), "#..#..#".into(), "#.....#".into(), "#..#..#".into(), "#######".into()];
    m.walls.push(Wall { x1: 3.0, y1: 2.0, x2: 3.0, y2: 3.0, kind: WallKind::Door, ..Default::default() });
    m.props.push(Prop { kind: "chest".into(), x: 5.5, y: 1.5, ..Default::default() });
    m.fog = FogMode::Vision;
    w.campaign.map = m;
    w.campaign.characters.push(Character::new("ann", "Ann"));
    w.campaign.encounter.combatants.push(Combatant { id: "ann".into(), name: "Ann".into(), kind: CombatantKind::Pc { id: "ann".into() }, hp: 8, hp_max: 8, pos: Some(Pos::new(1, 2)), visible: true, ..Default::default() });
    w.campaign.encounter.combatants.push(Combatant { id: "rat".into(), name: "Rat".into(), kind: CombatantKind::Creature { template: "rat".into() }, hp: 1, hp_max: 1, pos: Some(Pos::new(5, 2)), visible: true, ..Default::default() });
    w.campaign.register_device("device-ann-0001", "Ann");
    assert!(w.campaign.assign_device("device-ann-0001", Some("ann")));
    w.update_explored();
    let world = Arc::new(RwLock::new(w));
    let server = dnd_server::start_local(world.clone(), 0).unwrap();
    let url = format!("http://127.0.0.1:{}", server.addr.port());

    // nobody is handed the whole map any more, with or without a device id
    let enc: dnd_core::api::PlayerEncounter = ureq::get(&format!("{url}/api/player/encounter")).call().unwrap().into_json().unwrap();
    assert!(enc.map.cells.is_empty() && enc.map.walls.is_empty());
    assert!(enc.combatants.iter().find(|c| c.id == "rat").unwrap().pos.is_none(), "the rat's place is not given away");
    let party: MapView = ureq::get(&format!("{url}/api/player/map")).call().unwrap().into_json().unwrap();
    assert_eq!(party.tile(5, 2), tile::UNKNOWN);
    assert!(!party.dm && party.props.is_empty());

    let ann = Client::spawn(egui::Context::default());
    ann.tx.send(Cmd::Connect { url: url.clone(), code: String::new(), device_id: "device-ann-0001".into(), name: "Ann".into() }).unwrap();
    wait_for(&ann, "seat", |e| match e {
        Evt::Seat { seat, .. } => seat.character.filter(|c| c == "ann"),
        _ => None,
    });
    ann.tx.send(Cmd::LoadCharacter("ann".into())).unwrap();
    let v = next_map(&ann);
    assert_eq!(v.name, "Cellar");
    assert_eq!(v.tile(1, 1), tile::FLOOR);
    assert_eq!(v.tile(5, 2), tile::UNKNOWN);
    assert_eq!(v.tokens.len(), 1);
    assert!(v.tokens[0].mine);

    // the door is shut: no way through, and the server says why
    ann.tx.send(Cmd::MapMove { x: 4, y: 2 }).unwrap();
    let why = wait_for(&ann, "refusal", |e| match e {
        Evt::MapRefused(w) => Some(w),
        _ => None,
    });
    assert!(why.contains("cannot get there"), "{why}");
    // too far away to reach the handle
    ann.tx.send(Cmd::MapDoor { x: 3.0, y: 2.5 }).unwrap();
    let why = wait_for(&ann, "too far", |e| match e {
        Evt::MapRefused(w) => Some(w),
        _ => None,
    });
    assert!(why.contains("too far"), "{why}");
    next_map(&ann);

    // walk up to it, open it, and the far room with its rat and chest appears
    ann.tx.send(Cmd::MapMove { x: 2, y: 2 }).unwrap();
    // (a background refresh may deliver an older picture of the map first)
    wait_for(&ann, "the map with Ann moved", |e| match e {
        Evt::Map(m) if m.tokens[0].x == 2 => Some(()),
        _ => None,
    });
    assert_eq!(wait_for(&ann, "walked", |e| if let Evt::Walked(f) = e { Some(f) } else { None }), 5);
    ann.tx.send(Cmd::MapDoor { x: 3.0, y: 2.5 }).unwrap();
    let v = wait_for(&ann, "the map with the door open", |e| match e {
        Evt::Map(m) if m.walls.first().map(|w| w.open).unwrap_or(false) => Some(*m),
        _ => None,
    });
    assert!(v.walls[0].open);
    assert_eq!(v.tile(5, 2), tile::FLOOR);
    assert!(v.tokens.iter().any(|t| t.id == "rat" && !t.mine));
    assert!(v.props.iter().any(|p| p.kind == "chest"));
    assert!(world.read().unwrap().campaign.log.iter().any(|l| l.text == "Ann opens a door"));

    // a fight starts and it is the rat's turn: Ann has to wait
    {
        let mut w = world.write().unwrap();
        w.campaign.encounter.active = true;
        w.campaign.encounter.turn = 1;
        w.touch();
    }
    ann.tx.send(Cmd::MapMove { x: 3, y: 2 }).unwrap();
    let why = wait_for(&ann, "not your turn", |e| match e {
        Evt::MapRefused(w) => Some(w),
        _ => None,
    });
    assert!(why.contains("your turn"), "{why}");
    // someone else's device cannot move Ann at all
    let r = ureq::post(&format!("{url}/api/player/map/move")).send_json(ureq::json!({ "device_id": "device-bob-0002", "x": 3, "y": 2 }));
    assert!(matches!(r, Err(ureq::Error::Status(403, _))));
    assert_eq!(world.read().unwrap().campaign.encounter.find("ann").unwrap().pos, Some(Pos::new(2, 2)));

    ann.tx.send(Cmd::Shutdown).unwrap();
    server.stop();
}
