//! A player builds a character between sessions and uploads it; once the
//! game is live, new characters come from the DM only.

use dnd_core::{Abilities, Advancement, Character, ClassLevel, World};
use dnd_player::client::{Client, Cmd, Evt};
use std::path::PathBuf;
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

fn draft(name: &str) -> Character {
    let mut c = Character::new("", name);
    c.species = "dwarf".into();
    c.background = "soldier".into();
    c.classes = vec![ClassLevel { class: "fighter".into(), level: 7, ..Default::default() }];
    c.base_abilities = Abilities { str: 15, dex: 12, con: 14, int: 8, wis: 13, cha: 10 };
    c.ability_method = "standard_array".into();
    c.skill_proficiencies = vec!["perception".into(), "survival".into()];
    c.dm_notes = "sneaky".into();
    c
}

fn create(c: &Client, ch: Character, gear: Vec<usize>) -> Result<dnd_core::api::Seat, String> {
    c.tx.send(Cmd::Create { character: Box::new(ch), gear }).unwrap();
    wait_for(c, "upload answer", |e| match e {
        Evt::Created(s) => Some(Ok(s)),
        Evt::CreateFailed(e) => Some(Err(e)),
        _ => None,
    })
}

fn status(r: Result<ureq::Response, ureq::Error>) -> u16 {
    match r {
        Ok(r) => r.status(),
        Err(ureq::Error::Status(c, _)) => c,
        Err(e) => panic!("{e}"),
    }
}

#[test]
fn player_builds_uploads_and_owns_a_character() {
    let content = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content");
    let world = Arc::new(RwLock::new(World::new(vec![content])));
    assert!(world.read().unwrap().library.classes.contains_key("fighter"), "sample content pack should load");
    let server = dnd_server::start_local(world.clone(), 0).unwrap();
    let url = format!("http://127.0.0.1:{}", server.addr.port());

    // Sam joins between sessions: the app gets the build options and may create
    let sam = Client::spawn(egui::Context::default());
    sam.tx.send(Cmd::Connect { url: url.clone(), code: String::new(), device_id: "device-sam-0001".into(), name: "Sam".into() }).unwrap();
    let mut seat = None;
    let opts = wait_for(&sam, "builder options", |e| match e {
        Evt::Seat { seat: s, .. } => {
            seat = Some(s);
            None
        }
        Evt::Builder(o) => Some(o),
        _ => None,
    });
    let seat = seat.expect("seat arrives before the options");
    assert!(seat.can_create && !seat.game_live && !seat.made_character);
    assert_eq!(seat.start_level, 1);
    assert!(opts.classes.iter().any(|c| c.id == "fighter"));
    assert!(opts.species.iter().any(|s| s.id == "dwarf"));
    assert!(opts.subclasses.iter().any(|s| s.id == "champion"));
    assert!(opts.items.contains_key("chain_mail"), "{:?}", opts.items.keys().collect::<Vec<_>>());

    // the same numbers the phone previews are what the server will compute
    let lib = dnd_core::builder::library_from_options(&opts);
    let mut at_one = draft("Borin Stonefist");
    at_one.classes[0].level = 1;
    let preview = dnd_core::derive(&at_one, &lib);

    // first line of fighter gear: option 0 is chain mail
    let seat = create(&sam, draft("Borin Stonefist"), vec![0]).expect("upload");
    assert_eq!(seat.character.as_deref(), Some("borin_stonefist"));
    assert!(seat.made_character && seat.can_create, "the player may rebuild what they made");
    {
        let w = world.read().unwrap();
        let ch = w.campaign.character("borin_stonefist").unwrap();
        assert_eq!(ch.player, "Sam");
        assert_eq!(ch.total_level(), 1, "uploads start at the campaign's level whatever was sent");
        assert!(ch.dm_notes.is_empty());
        assert_eq!(w.campaign.device_character("device-sam-0001").unwrap().id, "borin_stonefist");
        let sheet = w.sheet("borin_stonefist").unwrap();
        assert_eq!(ch.hp.current, sheet.hp_max);
        assert_eq!(sheet.hp_max, preview.hp_max);
        assert_eq!(sheet.abilities, preview.abilities);
        assert!(ch.inventory.iter().any(|i| i.item == "chain_mail" && !i.custom), "starting gear: {:?}", ch.inventory);
        assert!(!ch.inventory.iter().any(|i| i.item == "leather_armor"));
        assert_eq!(ch.currency.gp, 10, "the soldier's 10 gp");
    }

    // the server rolls ability scores, once, and only those numbers are accepted
    sam.tx.send(Cmd::RollAbilities).unwrap();
    let rolls = wait_for(&sam, "rolls", |e| match e {
        Evt::Seat { seat, .. } if seat.rolled.len() == 6 => Some(seat.rolled),
        _ => None,
    });
    assert!(rolls.iter().all(|r| (3..=18).contains(r)));
    assert_eq!(world.read().unwrap().campaign.device("device-sam-0001").unwrap().rolled, rolls);
    sam.tx.send(Cmd::RollAbilities).unwrap();
    let again = wait_for(&sam, "second roll", |e| match e {
        Evt::Seat { seat, .. } if seat.rolled.len() == 6 => Some(seat.rolled),
        _ => None,
    });
    assert_eq!(again, rolls, "no re-rolling until the DM allows it");
    let mut cheat = draft("Borin Stonefist");
    cheat.ability_method = "rolled".into();
    cheat.base_abilities = Abilities { str: 18, dex: 18, con: 18, int: 18, wis: 18, cha: 17 };
    if rolls != vec![18, 18, 18, 18, 18, 17] {
        assert!(create(&sam, cheat, vec![]).unwrap_err().contains("400"));
    }

    // the DM raises the starting level; Sam rebuilds: same seat, the old character is replaced
    world.write().unwrap().campaign.settings.player_start_level = 4;
    let mut honest = draft("Borin Stonefist");
    honest.ability_method = "rolled".into();
    honest.base_abilities = Abilities { str: rolls[0], dex: rolls[1], con: rolls[2], int: rolls[3], wis: rolls[4], cha: rolls[5] };
    assert!(create(&sam, honest.clone(), vec![1]).unwrap_err().contains("400"), "a level 4 fighter needs a subclass");
    honest.classes[0].subclass = Some("champion".into());
    honest.advancements = vec![Advancement { level: 4, ability_increases: [("str".to_string(), 1), ("con".to_string(), 1)].into_iter().collect(), feat: None }];
    let seat = create(&sam, honest, vec![1]).expect("rebuild");
    assert_eq!(seat.character.as_deref(), Some("borin_stonefist"));
    {
        let w = world.read().unwrap();
        assert_eq!(w.campaign.characters.len(), 1, "replaced, not added");
        let ch = w.campaign.character("borin_stonefist").unwrap();
        assert_eq!(ch.total_level(), 4);
        assert_eq!(ch.classes[0].subclass.as_deref(), Some("champion"));
        assert_eq!(ch.advancements.len(), 1);
        assert!(ch.inventory.iter().any(|i| i.item == "leather_armor"), "{:?}", ch.inventory);
        assert!(!ch.inventory.iter().any(|i| i.item == "chain_mail"));
    }

    // the wholesale character routes are for the DM, not for anyone with the join code
    assert_eq!(status(ureq::post(&format!("{url}/api/characters")).send_json(ureq::json!({ "name": "Intruder" }))), 401);
    assert_eq!(status(ureq::put(&format!("{url}/api/characters/borin_stonefist")).send_json(ureq::json!({ "name": "Hacked" }))), 401);

    // the DM starts the game: Kim arrives late and cannot upload or roll
    {
        let mut w = world.write().unwrap();
        w.campaign.settings.game_live = true;
        w.touch();
    }
    let kim = Client::spawn(egui::Context::default());
    kim.tx.send(Cmd::Connect { url, code: String::new(), device_id: "device-kim-0002".into(), name: "Kim".into() }).unwrap();
    let seat = wait_for(&kim, "kim's seat", |e| match e {
        Evt::Seat { seat, fresh: true } => Some(seat),
        _ => None,
    });
    assert!(seat.game_live && !seat.can_create && seat.character.is_none());
    assert!(create(&kim, draft("Late Hero"), vec![]).unwrap_err().contains("403"));
    assert!(create(&sam, draft("Borin Again"), vec![]).unwrap_err().contains("403"), "no rebuilding mid-game either");
    assert_eq!(world.read().unwrap().campaign.characters.len(), 1);

    // so the DM gives Kim a spare character, which Kim may not swap out afterwards
    {
        let mut w = world.write().unwrap();
        w.campaign.characters.push(Character::new("spare", "Spare Hero"));
        assert!(w.campaign.assign_device("device-kim-0002", Some("spare")));
        w.touch();
    }
    let got = wait_for(&kim, "assignment", |e| match e {
        Evt::Seat { seat, .. } if seat.character.is_some() => Some(seat),
        _ => None,
    });
    assert_eq!(got.character.as_deref(), Some("spare"));
    assert!(!got.made_character);
    world.write().unwrap().campaign.settings.game_live = false;
    assert!(create(&kim, draft("Kim's Own"), vec![]).unwrap_err().contains("409"));

    sam.tx.send(Cmd::Shutdown).unwrap();
    kim.tx.send(Cmd::Shutdown).unwrap();
    server.stop();
}
