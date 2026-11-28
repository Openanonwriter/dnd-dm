//! The little game the examples share: a wizard and a fighter in a cellar
//! with two goblins, the fight already on.

use dnd_core::grid::Pos;
use dnd_core::model::*;
use dnd_core::play::RollMode;
use dnd_core::World;
use std::path::PathBuf;

pub const DEVICE: &str = "demo-device-0001";

pub fn world() -> World {
    let content = std::env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../content")));
    let mut w = World::new(vec![content]);
    w.campaign = Campaign::new("The Sunless Cellar");

    let mut thia = Character::new("thia", "Thia");
    thia.species = "elf".into();
    thia.classes = vec![ClassLevel { class: "wizard".into(), subclass: Some("evocation".into()), level: 5, ..Default::default() }];
    for (k, v) in [("int", 17), ("dex", 14), ("con", 14), ("wis", 12), ("cha", 10), ("str", 8)] {
        thia.base_abilities.set(k, v);
    }
    thia.inventory = vec![
        InventoryItem { item: "quarterstaff".into(), qty: 1, equipped: true, ..Default::default() },
        InventoryItem { item: "dagger".into(), qty: 2, equipped: true, ..Default::default() },
        InventoryItem { item: "spellbook".into(), qty: 1, ..Default::default() },
        InventoryItem { item: "potion_of_healing".into(), qty: 2, ..Default::default() },
    ];
    thia.spells.known = ["fire_bolt", "light", "mage_hand", "ray_of_frost", "magic_missile", "shield", "mage_armor", "sleep", "burning_hands", "detect_magic", "scorching_ray", "misty_step", "web", "fireball", "counterspell", "fly"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    thia.spells.prepared = ["magic_missile", "shield", "burning_hands", "detect_magic", "scorching_ray", "misty_step", "fireball", "counterspell"].iter().map(|s| s.to_string()).collect();
    thia.spells.slots_used = vec![1, 0, 1];

    let mut borin = Character::new("borin", "Borin");
    borin.species = "dwarf".into();
    borin.classes = vec![ClassLevel { class: "fighter".into(), subclass: Some("champion".into()), level: 5, ..Default::default() }];
    for (k, v) in [("str", 17), ("con", 16), ("dex", 12), ("wis", 12), ("int", 10), ("cha", 8)] {
        borin.base_abilities.set(k, v);
    }
    borin.inventory = vec![
        InventoryItem { item: "longsword".into(), qty: 1, equipped: true, ..Default::default() },
        InventoryItem { item: "chain_mail".into(), qty: 1, equipped: true, ..Default::default() },
        InventoryItem { item: "shield".into(), qty: 1, equipped: true, ..Default::default() },
    ];
    for mut c in [thia, borin] {
        c.hp.current = dnd_core::derive(&c, &w.library).hp_max;
        w.campaign.characters.push(c);
    }

    let mut m = MapState::new("The Sunless Cellar", 14, 10);
    m.cells = vec![
        "##############".into(),
        "#.....#......#".into(),
        "#.....#......#".into(),
        "#............#".into(),
        "#.....#......#".into(),
        "#.....#..~~..#".into(),
        "###.####.~~..#".into(),
        "#......#.....#".into(),
        "#......#.....#".into(),
        "##############".into(),
    ];
    m.walls.push(Wall { x1: 6.0, y1: 3.0, x2: 6.0, y2: 4.0, kind: WallKind::Door, open: true, ..Default::default() });
    m.lights.push(Light { x: 9.5, y: 1.5, bright: 20, ..Default::default() });
    m.props.push(Prop { kind: "chest".into(), x: 12.5, y: 1.5, ..Default::default() });
    m.props.push(Prop { kind: "barrel".into(), x: 1.5, y: 1.5, ..Default::default() });
    m.fog = FogMode::Vision;
    m.start = Some(Pos::new(3, 3));
    w.campaign.map = m;

    for (id, at, light) in [("thia", Pos::new(3, 3), 0), ("borin", Pos::new(5, 3), 20)] {
        let mut c = w.combatant_from_character(id).unwrap();
        c.pos = Some(at);
        c.light = light;
        c.visible = true;
        c.initiative = if id == "thia" { 19 } else { 12 };
        w.campaign.encounter.combatants.push(c);
    }
    for (i, at) in [Pos::new(8, 3), Pos::new(10, 2)].into_iter().enumerate() {
        let mut c = w.combatant_from_creature("goblin", Some(&format!("Goblin {}", i + 1))).unwrap();
        c.pos = Some(at);
        c.visible = true;
        c.initiative = 15 - i as i32 * 5;
        w.campaign.encounter.combatants.push(c);
    }
    w.campaign.register_device(DEVICE, "Demo player");
    w.campaign.assign_device(DEVICE, Some("thia"));
    w.campaign.settings.game_live = true;
    w.update_explored();
    w.start_encounter();
    // a little history so the table feed has something in it
    let _ = w.pc_cast("thia", "fire_bolt", 0, &["goblin_1".into()], RollMode::Normal);
    w.next_turn();
    let _ = w.creature_attack("goblin_1", 1, "borin", RollMode::Normal);
    w.next_turn();
    let _ = w.pc_attack("borin", 0, "goblin_1", RollMode::Normal);
    for _ in 0..8 {
        w.next_turn();
        if w.campaign.encounter.current().map(|c| c.id == "thia").unwrap_or(false) {
            break;
        }
    }
    w
}
