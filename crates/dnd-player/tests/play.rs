//! A whole evening in small: a player builds a wizard with spells, casts at
//! a goblin, rolls where the table sees it, learns, rests and ends a turn,
//! all through the real server with the real content pack.

use dnd_core::model::*;
use dnd_core::play::{ActionResult, RollMode};
use dnd_core::World;
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

fn action(c: &Client) -> Result<ActionResult, String> {
    wait_for(c, "action result", |e| match e {
        Evt::Action(r) => Some(r),
        _ => None,
    })
}

#[test]
fn a_wizard_plays_a_session() {
    let content = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../content"));
    let mut w = World::new(vec![content]);
    assert!(w.library.spells.len() > 60, "spell list loaded: {}", w.library.spells.len());
    w.rng = dnd_core::Rng::seeded(11);
    let world = Arc::new(RwLock::new(w));
    let server = dnd_server::start_local(world.clone(), 0).unwrap();
    let url = format!("http://127.0.0.1:{}", server.addr.port());

    let mira = Client::spawn(egui::Context::default());
    mira.tx.send(Cmd::Connect { url, code: String::new(), device_id: "device-mira-0001".into(), name: "Mira".into() }).unwrap();
    let options = wait_for(&mira, "builder options", |e| match e {
        Evt::Builder(o) => Some(*o),
        _ => None,
    });
    assert!(options.spells.iter().any(|s| s.id == "mage_armor"), "the builder is sent the spells");

    // ---- build: three cantrips, six spells in the book, some prepared
    let mut c = Character::new("", "Mira");
    c.species = "elf".into();
    c.classes = vec![ClassLevel { class: "wizard".into(), level: 1, ..Default::default() }];
    c.ability_method = "standard_array".into();
    for (k, v) in [("int", 15), ("dex", 14), ("con", 13), ("wis", 12), ("cha", 10), ("str", 8)] {
        c.base_abilities.set(k, v);
    }
    let book = ["magic_missile", "shield", "mage_armor", "sleep", "burning_hands", "detect_magic"];
    c.spells.known = ["fire_bolt", "light", "mage_hand"].iter().chain(book.iter()).map(|s| s.to_string()).collect();
    c.spells.prepared = vec!["magic_missile".into(), "shield".into(), "sleep".into(), "cure_wounds".into()];
    let mut greedy = c.clone();
    greedy.spells.known.push("fireball".into());
    mira.tx.send(Cmd::Create { character: Box::new(greedy), gear: vec![] }).unwrap();
    let refused = wait_for(&mira, "refusal", |e| match e {
        Evt::CreateFailed(why) => Some(why),
        _ => None,
    });
    assert!(refused.contains("Fireball"), "a level 1 wizard cannot take fireball: {refused}");
    mira.tx.send(Cmd::Create { character: Box::new(c), gear: vec![] }).unwrap();
    let id = wait_for(&mira, "created", |e| match e {
        Evt::Created(seat) => seat.character,
        _ => None,
    });
    {
        let w = world.read().unwrap();
        let ch = w.campaign.character(&id).unwrap();
        assert_eq!(ch.spells.known.len(), 9);
        assert_eq!(ch.spells.prepared, vec!["magic_missile".to_string(), "shield".into(), "sleep".into()], "cure wounds is no wizard spell");
    }

    // ---- a goblin walks in
    {
        let mut w = world.write().unwrap();
        let pc = w.combatant_from_character(&id).unwrap();
        w.campaign.encounter.combatants.push(pc);
        let mut g = w.combatant_from_creature("goblin", None).unwrap();
        g.visible = true;
        w.campaign.encounter.combatants.push(g);
        w.touch();
    }
    mira.tx.send(Cmd::LoadCharacter(id.clone())).unwrap();

    mira.tx.send(Cmd::Cast { spell: "fireball".into(), slot: 3, targets: vec!["goblin".into()], mode: RollMode::Normal }).unwrap();
    assert!(action(&mira).is_err(), "not a spell she knows");
    mira.tx.send(Cmd::Cast { spell: "burning_hands".into(), slot: 1, targets: vec!["goblin".into()], mode: RollMode::Normal }).unwrap();
    assert!(action(&mira).unwrap_err().contains("not prepared"));

    mira.tx.send(Cmd::Cast { spell: "magic_missile".into(), slot: 1, targets: vec!["goblin".into()], mode: RollMode::Normal }).unwrap();
    let r = action(&mira).unwrap();
    assert!(r.text.contains("Magic Missile") && r.damage >= 6, "{r:?}");
    {
        let w = world.read().unwrap();
        assert_eq!(w.campaign.encounter.find("goblin").unwrap().hp, (7 - r.damage).max(0));
        assert_eq!(w.campaign.character(&id).unwrap().spells.slots_used[0], 1);
        let said = w.campaign.table.last().unwrap();
        assert_eq!(said.who, "Mira");
        assert!(said.text.contains("Goblin"));
    }
    // cantrips cost nothing and are rolled against armor class
    mira.tx.send(Cmd::Cast { spell: "fire_bolt".into(), slot: 0, targets: vec!["goblin".into()], mode: RollMode::Advantage }).unwrap();
    assert!(action(&mira).unwrap().text.contains("spell attack"));

    // ---- a roll the whole table sees
    mira.tx.send(Cmd::Roll { formula: "1d20 + 5".into(), label: "Arcana".into(), mode: RollMode::Normal }).unwrap();
    let r = action(&mira).unwrap();
    assert!((6..=25).contains(&r.total), "{r:?}");
    let table = wait_for(&mira, "table", |e| match e {
        Evt::Table(t) if t.iter().any(|x| x.text.contains("Arcana")) => Some(t),
        _ => None,
    });
    assert!(table.iter().any(|x| x.kind == "spell"));

    // ---- the patch a phone sends never changes which spells are known
    let mut patch = dnd_core::api::PlayerPatch::default();
    patch.spells = Some(SpellState { known: vec!["fireball".into()], prepared: vec!["magic_missile".into()], slots_used: vec![1], ..Default::default() });
    mira.tx.send(Cmd::Patch { id: id.clone(), patch }).unwrap();
    wait_for(&mira, "patched", |e| match e {
        Evt::Patched(c) => Some(c),
        _ => None,
    });
    assert_eq!(world.read().unwrap().campaign.character(&id).unwrap().spells.known.len(), 9);

    // ---- between games she may swap a spell; during one, only add
    let mut known = world.read().unwrap().campaign.character(&id).unwrap().spells.known.clone();
    known.retain(|k| k != "detect_magic");
    known.push("grease".into());
    mira.tx.send(Cmd::ChooseSpells(known.clone())).unwrap();
    let c = wait_for(&mira, "spells chosen", |e| match e {
        Evt::SpellsChosen(c) => Some(c),
        _ => None,
    });
    assert!(c.spells.known.contains(&"grease".to_string()) && !c.spells.known.contains(&"detect_magic".to_string()));
    world.write().unwrap().campaign.settings.game_live = true;
    known.retain(|k| k != "grease");
    mira.tx.send(Cmd::ChooseSpells(known)).unwrap();
    assert!(action(&mira).unwrap_err().contains("add"));

    // ---- rest, then a fight with turns
    mira.tx.send(Cmd::Do { action: "long_rest".into(), arg: String::new() }).unwrap();
    action(&mira).unwrap();
    assert!(world.read().unwrap().campaign.character(&id).unwrap().spells.slots_used.iter().all(|u| *u == 0));
    {
        let mut w = world.write().unwrap();
        for c in &mut w.campaign.encounter.combatants {
            c.initiative = if matches!(c.kind, CombatantKind::Pc { .. }) { 20 } else { 3 };
            c.hp = c.hp_max;
        }
        w.start_encounter();
    }
    mira.tx.send(Cmd::Do { action: "end_turn".into(), arg: String::new() }).unwrap();
    action(&mira).unwrap();
    mira.tx.send(Cmd::Do { action: "end_turn".into(), arg: String::new() }).unwrap();
    assert!(action(&mira).unwrap_err().contains("not your turn"));
    {
        let mut w = world.write().unwrap();
        assert_eq!(w.campaign.encounter.current().unwrap().id, "goblin");
        // the goblin answers: the DM's attack lands on her sheet too
        let before = w.campaign.character(&id).unwrap().hp.current;
        let mut hit = false;
        for _ in 0..12 {
            let r = w.creature_attack("goblin", 0, &id, RollMode::Advantage).unwrap();
            if r.hit == Some(true) {
                hit = true;
                break;
            }
        }
        assert!(hit);
        assert!(w.campaign.character(&id).unwrap().hp.current < before);
    }
    mira.tx.send(Cmd::Shutdown).unwrap();
    drop(server);
}
