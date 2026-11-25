//! Loads the real `content/` pack shipped with the repo and builds a few
//! characters from it, so a broken TOML file fails CI instead of a session.

use dnd_core::model::*;
use dnd_core::{derive, Library};
use std::path::PathBuf;

fn content_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content")
}

#[test]
fn base_pack_loads_without_warnings() {
    let lib = Library::load_packs(&[content_dir()]);
    assert!(lib.warnings.is_empty(), "{:#?}", lib.warnings);
    assert!(lib.species.len() >= 5);
    assert!(lib.classes.len() >= 4);
    assert!(lib.items.len() >= 30);
    assert!(lib.spells.len() >= 15);
    assert!(lib.creatures.len() >= 9);
    assert_eq!(lib.rules.skills.len(), 18);
}

#[test]
fn dwarf_fighter_sheet() {
    let lib = Library::load_packs(&[content_dir()]);
    let mut ch = Character::new("bruenor", "Bruenor");
    ch.species = "dwarf".into();
    ch.background = "soldier".into();
    ch.classes = vec![ClassLevel { class: "fighter".into(), subclass: Some("champion".into()), level: 4, ..Default::default() }];
    ch.base_abilities = Abilities { str: 16, dex: 12, con: 14, int: 8, wis: 10, cha: 10 };
    ch.skill_proficiencies = vec!["perception".into(), "survival".into()];
    ch.advancements = vec![Advancement { level: 4, ability_increases: [("str".to_string(), 2)].into_iter().collect(), feat: None }];
    ch.inventory = vec![
        InventoryItem { item: "chain_mail".into(), equipped: true, qty: 1, ..Default::default() },
        InventoryItem { item: "shield".into(), equipped: true, qty: 1, ..Default::default() },
        InventoryItem { item: "longsword".into(), equipped: true, qty: 1, ..Default::default() },
        InventoryItem { item: "handaxe".into(), equipped: true, qty: 2, ..Default::default() },
        InventoryItem { item: "ring_of_protection".into(), equipped: true, attuned: true, qty: 1, ..Default::default() },
    ];
    let s = derive(&ch, &lib);
    assert!(s.warnings.is_empty(), "{:?}", s.warnings);
    assert_eq!(s.abilities.str, 18); // 16 + 2 ASI
    assert_eq!(s.abilities.con, 16); // dwarf +2
    assert_eq!(s.abilities.wis, 11); // hill dwarf +1
    assert_eq!(s.proficiency_bonus, 2);
    // chain mail 16 + shield 2 + Defense 1 + ring 1
    assert_eq!(s.ac, 20);
    // L1 10+3, L2..4 (6+3)*3 = 40, + Dwarven Toughness 4 = 44
    assert_eq!(s.hp_max, 44);
    assert_eq!(s.speed, 25);
    let sword = s.attacks.iter().find(|a| a.name == "Longsword").unwrap();
    assert_eq!(sword.bonus, 6); // +4 str +2 pb
    assert_eq!(sword.damage, "1d8 + 4");
    let str_save = s.saves.iter().find(|x| x.key == "str").unwrap();
    assert_eq!(str_save.bonus, 7); // +4 +2 prof +1 ring
    assert!(str_save.proficient);
    let athletics = s.skills.iter().find(|x| x.key == "athletics").unwrap();
    assert!(athletics.proficient); // from soldier background
    assert_eq!(athletics.bonus, 6);
    assert!(s.features.iter().any(|f| f.name == "Improved Critical"));
    assert!(s.features.iter().any(|f| f.name == "Action Surge" && f.uses == Some(1)));
    assert!(s.proficiencies.weapons.contains(&"martial".to_string()));
    assert!(s.proficiencies.languages.contains(&"Dwarvish".to_string()));
}

#[test]
fn elf_wizard_spellcasting() {
    let lib = Library::load_packs(&[content_dir()]);
    let mut ch = Character::new("aria", "Aria");
    ch.species = "elf".into();
    ch.background = "sage".into();
    ch.classes = vec![ClassLevel { class: "wizard".into(), subclass: Some("evocation".into()), level: 5, ..Default::default() }];
    ch.base_abilities = Abilities { str: 8, dex: 14, con: 13, int: 16, wis: 12, cha: 10 };
    ch.inventory = vec![InventoryItem { item: "dagger".into(), equipped: true, qty: 1, ..Default::default() }];
    ch.feats = vec!["tough".into()];
    let s = derive(&ch, &lib);
    assert!(s.warnings.is_empty(), "{:?}", s.warnings);
    assert_eq!(s.abilities.int, 17);
    assert_eq!(s.proficiency_bonus, 3);
    let sc = &s.spellcasting[0];
    assert_eq!(sc.attack_bonus, 6);
    assert_eq!(sc.save_dc, 14);
    assert_eq!(sc.prepared_max, Some(8)); // 3 + 5
    assert_eq!(sc.cantrips_known, 4);
    assert_eq!(s.spell_slots, vec![4, 3, 2, 0, 0, 0, 0, 0, 0]);
    // dex 16 -> +3; dagger finesse uses dex; proficient via class list
    let dagger = s.attacks.iter().find(|a| a.name == "Dagger").unwrap();
    assert_eq!(dagger.bonus, 6);
    // HP: 6+1 + (4+1)*4 = 27 + tough 10 = 37
    assert_eq!(s.hp_max, 37);
    assert_eq!(s.ac, 13);
    let perception = s.skills.iter().find(|x| x.key == "perception").unwrap();
    assert!(perception.proficient); // Keen Senses grant
    assert_eq!(s.darkvision, 60);
    assert!(s.features.iter().any(|f| f.name == "Sculpt Spells"));
}

#[test]
fn creatures_become_combatants() {
    let mut world = dnd_core::World::new(vec![content_dir()]);
    let c = world.combatant_from_creature("ogre", None).unwrap();
    assert_eq!(c.hp, 59);
    assert_eq!(c.ac, 11);
    assert_eq!(c.size, "Large");
    world.campaign.encounter.combatants.push(c);
    let c2 = world.combatant_from_creature("ogre", None).unwrap();
    assert_eq!(c2.id, "ogre_2");
    let html = dnd_core::export::creature_html(world.library.creatures.get("giant_spider").unwrap());
    assert!(html.contains("Spider Climb"));
}
