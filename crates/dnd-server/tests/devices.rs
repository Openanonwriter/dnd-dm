//! Player devices register by name; the DM assigns each one a character.

use dnd_core::api::{ClaimReq, RegisterReq, Seat};
use dnd_core::{Character, World};
use std::sync::{Arc, RwLock};

fn register(base: &str, id: &str, name: &str) -> Result<Seat, ureq::Error> {
    Ok(ureq::post(&format!("{base}/api/player/register")).send_json(RegisterReq { device_id: id.into(), name: name.into() })?.into_json().unwrap())
}

fn status(r: Result<ureq::Response, ureq::Error>) -> u16 {
    match r {
        Ok(r) => r.status(),
        Err(ureq::Error::Status(c, _)) => c,
        Err(e) => panic!("{e}"),
    }
}

#[test]
fn register_assign_and_guard() {
    let mut world = World::new(Vec::new());
    world.campaign.characters.push(Character::new("thia", "Thia"));
    world.campaign.characters.push(Character::new("borin", "Borin"));
    let world = Arc::new(RwLock::new(world));
    let server = dnd_server::start_local(world.clone(), 0).unwrap();
    let base = format!("http://127.0.0.1:{}", server.addr.port());

    // a phone joins by name: known to the DM, online, nothing assigned yet
    let seat = register(&base, "device-sam-0001", "  Sam ").unwrap();
    assert_eq!(seat.name, "Sam");
    assert_eq!(seat.character, None);
    assert!(!seat.can_choose);
    {
        let w = world.read().unwrap();
        assert_eq!(w.campaign.devices.len(), 1);
        assert!(w.device_online("device-sam-0001"));
    }
    // no name, no seat
    assert_eq!(status(ureq::post(&format!("{base}/api/player/register")).send_json(RegisterReq { device_id: "device-xx-0002".into(), name: " ".into() })), 400);

    // picking is off, so the player can neither claim nor edit a sheet
    let claim = ClaimReq { device_id: "device-sam-0001".into(), character: "thia".into() };
    assert_eq!(status(ureq::post(&format!("{base}/api/player/claim")).send_json(&claim)), 403);
    let patch = |ch: &str| status(ureq::request("PATCH", &format!("{base}/api/player/characters/{ch}")).set("X-Device-Id", "device-sam-0001").send_json(ureq::json!({ "notes": "hi" })));
    assert_eq!(patch("thia"), 403);

    // the DM assigns; the next check-in tells the phone
    let rev = {
        let mut w = world.write().unwrap();
        assert!(w.campaign.assign_device("device-sam-0001", Some("thia")));
        w.touch();
        w.revision
    };
    let seat = register(&base, "device-sam-0001", "Sam").unwrap();
    assert_eq!(seat.character.as_deref(), Some("thia"));
    assert_eq!(seat.character_name, "Thia");
    assert_eq!(world.read().unwrap().revision, rev, "a plain heartbeat must not bump the revision");
    assert_eq!(world.read().unwrap().campaign.character("thia").unwrap().player, "Sam");
    assert_eq!(patch("thia"), 200);
    assert_eq!(patch("borin"), 403);

    // DM remote: off without a dm_code, then list + reassign + forget
    assert_eq!(status(ureq::get(&format!("{base}/api/dm/devices")).call()), 401);
    world.write().unwrap().campaign.settings.dm_code = "dm".into();
    let list: serde_json::Value = ureq::get(&format!("{base}/api/dm/devices?dm_code=dm")).call().unwrap().into_json().unwrap();
    assert_eq!(list[0]["name"], "Sam");
    assert_eq!(list[0]["online"], true);
    assert_eq!(list[0]["character"], "thia");
    let seat: Seat = ureq::put(&format!("{base}/api/dm/devices/device-sam-0001?dm_code=dm")).send_json(ureq::json!({ "character": "borin" })).unwrap().into_json().unwrap();
    assert_eq!(seat.character.as_deref(), Some("borin"));
    assert_eq!(status(ureq::put(&format!("{base}/api/dm/devices/device-sam-0001?dm_code=dm")).send_json(ureq::json!({ "character": "nobody" }))), 404);

    // a deleted character leaves the device unassigned rather than dangling
    world.write().unwrap().campaign.characters.retain(|c| c.id != "borin");
    assert_eq!(register(&base, "device-sam-0001", "Sam").unwrap().character, None);

    // with picking allowed the player can claim, but not a character someone else holds
    world.write().unwrap().campaign.settings.players_choose_character = true;
    register(&base, "device-kim-0003", "Kim").unwrap();
    let seat: Seat = ureq::post(&format!("{base}/api/player/claim")).send_json(&claim).unwrap().into_json().unwrap();
    assert_eq!(seat.character.as_deref(), Some("thia"));
    let kim = ClaimReq { device_id: "device-kim-0003".into(), character: "thia".into() };
    assert_eq!(status(ureq::post(&format!("{base}/api/player/claim")).send_json(&kim)), 409);

    assert_eq!(status(ureq::delete(&format!("{base}/api/dm/devices/device-kim-0003?dm_code=dm")).call()), 204);
    assert_eq!(world.read().unwrap().campaign.devices.len(), 1);
    server.stop();
}

#[test]
fn devices_survive_save_and_auto_assign_by_name() {
    let mut camp = dnd_core::Campaign::new("T");
    let mut c = Character::new("thia", "Thia");
    c.player = "sam".into();
    camp.characters.push(c);
    camp.characters.push(Character::new("borin", "Borin"));
    camp.register_device("device-sam-0001", "Sam");
    camp.register_device("device-bor-0002", "borin");
    camp.register_device("device-zed-0003", "Zed");
    assert_eq!(camp.auto_assign_devices(), 2);
    assert_eq!(camp.device_character("device-sam-0001").unwrap().id, "thia");
    assert_eq!(camp.device_character("device-bor-0002").unwrap().id, "borin");
    assert!(camp.device_character("device-zed-0003").is_none());

    let dir = std::env::temp_dir().join(format!("dnd-dm-devtest-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dnd_core::save::save_campaign(&mut camp, &dir).unwrap();
    let loaded = dnd_core::save::load_campaign(&dir).unwrap();
    assert_eq!(loaded.devices, camp.devices);
    let _ = std::fs::remove_dir_all(&dir);
}
