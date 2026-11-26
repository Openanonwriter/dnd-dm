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
    assert!(lib.species.len() >= 17);
    assert_eq!(lib.classes.len(), 13);
    assert!(lib.subclasses.len() >= 12);
    assert!(lib.items.len() >= 170);
    assert!(lib.spells.len() >= 335);
    assert!(lib.creatures.len() >= 9);
    assert_eq!(lib.rules.skills.len(), 18);
}

#[test]
fn every_class_has_a_subclass_and_every_reference_resolves() {
    let lib = Library::load_packs(&[content_dir()]);
    for c in lib.classes.values() {
        assert!(lib.subclasses.values().any(|s| s.class == c.id), "{} has no subclass", c.id);
        for f in &c.features {
            assert!(f.level >= 1 && f.level <= 20, "{}: {}", c.id, f.name);
        }
    }
    let mut features: Vec<(String, &Feature)> = Vec::new();
    for c in lib.classes.values() {
        features.extend(c.features.iter().map(|f| (c.id.clone(), f)));
    }
    for s in lib.subclasses.values() {
        assert!(lib.classes.contains_key(&s.class), "{}: unknown class {}", s.id, s.class);
        features.extend(s.features.iter().map(|f| (s.id.clone(), f)));
    }
    for s in lib.species.values() {
        features.extend(s.traits.iter().map(|f| (s.id.clone(), f)));
    }
    for (owner, f) in &features {
        for sp in &f.spells {
            assert!(lib.spells.contains_key(sp), "{owner}: {} grants unknown spell {sp}", f.name);
        }
        if !f.options.is_empty() {
            assert!(lib.feats.values().any(|x| x.tags.contains(&f.options)), "{owner}: {} has no '{}' options", f.name, f.options);
        }
    }
    for feat in lib.feats.values() {
        for sp in &feat.spells {
            assert!(lib.spells.contains_key(sp), "feat {} grants unknown spell {sp}", feat.id);
        }
    }
    // every spell sits on at least one class list, and every list id is a class or subclass
    for s in lib.spells.values() {
        assert!(!s.classes.is_empty(), "{} is on no class list", s.id);
        for c in &s.classes {
            assert!(lib.classes.contains_key(c) || lib.subclasses.contains_key(c), "{}: unknown list {c}", s.id);
        }
    }
}

#[test]
fn choices_and_granted_spells_work_on_real_characters() {
    let lib = Library::load_packs(&[content_dir()]);

    // half-elf: picked +1s and skills apply; too many picks warn
    let mut ch = Character::new("h", "H");
    ch.species = "half_elf".into();
    ch.classes = vec![ClassLevel { class: "bard".into(), level: 1, ..Default::default() }];
    ch.choices.insert("Ability Score Increase".into(), vec!["ability_plus_dex".into(), "ability_plus_con".into()]);
    ch.choices.insert("Skill Versatility".into(), vec!["skill_stealth".into(), "skill_arcana".into()]);
    let s = derive(&ch, &lib);
    assert!(s.warnings.is_empty(), "{:?}", s.warnings);
    assert_eq!((s.abilities.cha, s.abilities.dex, s.abilities.con, s.abilities.str), (12, 11, 11, 10));
    assert!(s.skills.iter().find(|k| k.key == "stealth").unwrap().proficient);
    let line = s.features.iter().find(|f| f.name == "Ability Score Increase").unwrap();
    assert_eq!((line.options.as_str(), line.picks, line.chosen.len()), ("ability_plus_one", 2, 2));
    ch.choices.get_mut("Skill Versatility").unwrap().push("skill_history".into());
    assert!(derive(&ch, &lib).warnings.iter().any(|w| w.contains("Skill Versatility")));

    // life cleric 5: domain spells up to 3rd level are granted and castable without preparing
    let mut cl = Character::new("c", "C");
    cl.species = "human".into();
    cl.classes = vec![ClassLevel { class: "cleric".into(), subclass: Some("life_domain".into()), level: 5, ..Default::default() }];
    let s = derive(&cl, &lib);
    assert!(s.spells_granted.contains(&"revivify".to_string()));
    let avail = dnd_core::play::spells_available(&lib, &cl, &s);
    assert!(avail.contains(&"revivify".to_string()) && !avail.contains(&"death_ward".to_string()));

    // fiend warlock: patron spells are on its list to learn, plain warlocks can't
    let mut wl = Character::new("w", "W");
    wl.classes = vec![ClassLevel { class: "warlock".into(), subclass: Some("fiend".into()), level: 1, ..Default::default() }];
    assert!(dnd_core::play::choose_spells(&lib, &mut wl, &["burning_hands".into()], false, false).is_ok());
    wl.classes[0].subclass = None;
    wl.spells.known.clear();
    assert!(dnd_core::play::choose_spells(&lib, &mut wl, &["burning_hands".into()], false, false).is_err());

    // tiefling: racial spells unlock by character level
    let mut tf = Character::new("t", "T");
    tf.species = "tiefling".into();
    tf.classes = vec![ClassLevel { class: "fighter".into(), level: 3, ..Default::default() }];
    let s = derive(&tf, &lib);
    assert!(s.spells_granted.contains(&"hellish_rebuke".to_string()) && !s.spells_granted.contains(&"darkness".to_string()));

    // monk unarmored defense + movement, barbarian unarmored defense
    let mut mk = Character::new("m", "M");
    mk.classes = vec![ClassLevel { class: "monk".into(), level: 6, ..Default::default() }];
    mk.base_abilities = Abilities { str: 10, dex: 16, con: 12, int: 10, wis: 16, cha: 8 };
    let s = derive(&mk, &lib);
    assert_eq!(s.ac, 16);
    assert_eq!(s.speed, 45);
}

#[test]
fn dwarf_fighter_sheet() {
    let lib = Library::load_packs(&[content_dir()]);
    let mut ch = Character::new("bruenor", "Bruenor");
    ch.species = "dwarf".into();
    ch.background = "soldier".into();
    ch.classes = vec![ClassLevel { class: "fighter".into(), subclass: Some("champion".into()), level: 4, ..Default::default() }];
    ch.choices.insert("Fighting Style".into(), vec!["style_defense".into()]);
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

#[test]
fn artificer_casts_from_first_level_and_rounds_up() {
    let lib = Library::load_packs(&[content_dir()]);
    let mut ch = Character::new("a", "A");
    ch.species = "dwarf_warding".into();
    ch.base_abilities = Abilities { str: 10, dex: 14, con: 14, int: 16, wis: 10, cha: 8 };
    for (lvl, slots) in [(1u32, vec![2, 0, 0]), (3, vec![3, 0, 0]), (5, vec![4, 2, 0]), (9, vec![4, 3, 2])] {
        ch.classes = vec![ClassLevel { class: "artificer".into(), subclass: Some("alchemist".into()), level: lvl, ..Default::default() }];
        let s = derive(&ch, &lib);
        assert!(s.warnings.is_empty(), "{:?}", s.warnings);
        assert_eq!(&s.spell_slots[..3], &slots[..], "level {lvl}");
    }
    let s = derive(&ch, &lib);
    assert_eq!(s.spellcasting[0].prepared_max, Some(3 + 4)); // int 17 -> +3, half of 9 = 4
    assert!(s.spells_granted.contains(&"gaseous_form".to_string()));
    assert!(dnd_core::play::spells_available(&lib, &ch, &s).contains(&"booming_blade".to_string()) == false); // cantrips are learned, not prepared
}

#[test]
fn everything_a_player_can_pick_explains_itself() {
    let lib = Library::load_packs(&[content_dir()]);
    for kind in [Kind::Species, Kind::Class, Kind::Subclass, Kind::Background, Kind::Feat, Kind::Item, Kind::Spell] {
        for (id, name) in lib.list(kind) {
            let text = dnd_core::describe::describe(&lib, kind, &id).unwrap_or_default();
            assert!(text.len() > 15, "{kind:?} {name} has no real explanation: {text:?}");
        }
    }
    for b in lib.backgrounds.values() {
        let fills: Vec<&str> = b.suggestions.iter().map(|t| t.fills.as_str()).collect();
        for want in ["personality", "ideals", "bonds", "flaws"] {
            assert!(fills.contains(&want), "{} has no {want} suggestions", b.id);
        }
        for t in &b.suggestions {
            assert!(dnd_core::builder::suggestion_field_label(&t.fills) != "", "{}: bad fills {}", b.id, t.fills);
            assert!(t.options.len() >= 6, "{}: {} has only {} options", b.id, t.name, t.options.len());
        }
    }
    // picking a suggestion fills the field; picking it again takes it out
    let mut ch = Character::new("x", "X");
    dnd_core::builder::toggle_suggestion(&mut ch, "ideals", "Tradition. Keep the rites. (Lawful)");
    dnd_core::builder::toggle_suggestion(&mut ch, "ideals", "Mercy. Help. (Good)");
    assert_eq!(ch.ideals.lines().count(), 2);
    dnd_core::builder::toggle_suggestion(&mut ch, "ideals", "Tradition. Keep the rites. (Lawful)");
    assert_eq!(ch.ideals, "Mercy. Help. (Good)");
}

#[test]
fn rule_checks_flag_what_a_character_does_not_qualify_for() {
    use dnd_core::requirements::{check, Area};
    let lib = Library::load_packs(&[content_dir()]);
    let mut ch = Character::new("w", "W");
    ch.species = "tiefling".into();
    ch.base_abilities = Abilities { str: 8, dex: 14, con: 13, int: 10, wis: 12, cha: 15 };
    ch.classes = vec![ClassLevel { class: "warlock".into(), subclass: Some("fiend".into()), level: 3, ..Default::default() }];
    ch.choices.insert("Eldritch Invocations".into(), vec!["inv_thirsting_blade".into()]);
    ch.choices.insert("Pact Boon".into(), vec!["pact_blade".into()]);
    let p = check(&lib, &ch);
    assert!(p.iter().any(|x| x.area == Area::Choice && x.key == "inv_thirsting_blade" && x.message.contains("5th level")), "{p:#?}");
    ch.classes[0].level = 5;
    assert!(!check(&lib, &ch).iter().any(|x| x.key == "inv_thirsting_blade"), "qualifies at 5 with the blade pact");

    // a subclass too early, a spell too high, and a multiclass without the scores
    let mut f = Character::new("f", "F");
    f.base_abilities = Abilities { str: 15, dex: 10, con: 14, int: 8, wis: 10, cha: 10 };
    f.classes = vec![ClassLevel { class: "fighter".into(), subclass: Some("champion".into()), level: 1, ..Default::default() }];
    assert!(check(&lib, &f).iter().any(|x| x.area == Area::Subclass));
    f.classes.push(ClassLevel { class: "wizard".into(), level: 1, ..Default::default() });
    f.spells.known = vec!["fireball".into()];
    let p = check(&lib, &f);
    assert!(p.iter().any(|x| x.area == Area::Class && x.key == "wizard"), "{p:#?}");
    assert!(p.iter().any(|x| x.area == Area::Spell && x.key == "fireball"), "{p:#?}");

    // spells: locked for the player unless the DM allows rule breaks
    let mut wiz = Character::new("z", "Z");
    wiz.classes = vec![ClassLevel { class: "wizard".into(), level: 1, ..Default::default() }];
    assert!(dnd_core::play::choose_spells(&lib, &mut wiz, &["fireball".into()], false, false).is_err());
    assert!(dnd_core::play::choose_spells(&lib, &mut wiz, &["fireball".into()], false, true).is_ok());
}

#[test]
fn players_cannot_upload_rule_breaks_unless_the_dm_allows() {
    use dnd_core::builder::{prepare_player_character, UploadContext};
    let lib = Library::load_packs(&[content_dir()]);
    let mut camp = Campaign::new("T");
    let mut up = Character::new("", "Hexblade");
    up.species = "human".into();
    up.ability_method = "standard_array".into();
    up.base_abilities = Abilities { str: 10, dex: 13, con: 14, int: 8, wis: 12, cha: 15 };
    up.classes = vec![ClassLevel { class: "warlock".into(), subclass: Some("fiend".into()), level: 5, ..Default::default() }];
    up.choices.insert("Eldritch Invocations".into(), vec!["inv_thirsting_blade".into()]);
    up.choices.insert("Pact Boon".into(), vec!["pact_chain".into()]);
    let cx = UploadContext { start_level: 5, ..Default::default() };
    let e = prepare_player_character(&lib, &camp, "Sam", &up, &cx).unwrap_err();
    assert!(e.contains("Thirsting Blade") && e.contains("DM"), "{e}");
    camp.settings.allow_rule_breaks = true;
    let ch = prepare_player_character(&lib, &camp, "Sam", &up, &cx).unwrap();
    assert_eq!(ch.choices.get("Eldritch Invocations"), Some(&vec!["inv_thirsting_blade".to_string()]));
}

#[test]
fn the_dead_stay_dead_until_magic_brings_them_back() {
    let mut w = dnd_core::World::new(vec![content_dir()]);
    for (id, class) in [("ash", "cleric"), ("bo", "fighter")] {
        let mut ch = Character::new(id, id);
        ch.species = "human".into();
        ch.base_abilities = Abilities { str: 14, dex: 10, con: 14, int: 10, wis: 16, cha: 10 };
        ch.classes = vec![ClassLevel { class: class.into(), level: 5, ..Default::default() }];
        ch.hp.current = 30;
        w.campaign.characters.push(ch);
        let mut c = w.combatant_from_character(id).unwrap();
        c.visible = true;
        w.campaign.encounter.combatants.push(c);
    }
    // bo goes down and fails three death saves
    {
        let bo = w.campaign.character_mut("bo").unwrap();
        bo.hp.current = 0;
        bo.death_saves.failures = 3;
    }
    w.sync_combatant_from_pc("bo");
    assert!(w.campaign.character("bo").unwrap().is_dead());
    assert!(w.rest("bo", true).unwrap_err().contains("dead"));
    assert!(w.rest("bo", false).unwrap_err().contains("dead"));
    assert!(w.spend_hit_die("bo", "d10").is_err());
    assert!(w.death_save("bo").is_err());
    let token = w.pc_combatant("bo").unwrap();
    assert_eq!(w.heal(&token, 10), 0, "healing does nothing for the dead");
    // a player's own sheet can't heal them either
    let mut patch = dnd_core::api::PlayerPatch::default();
    patch.hp = Some(HitPoints { current: 20, ..Default::default() });
    patch.death_saves = Some(DeathSaves::default());
    w.campaign.character_mut("bo").unwrap().apply_player_patch(patch);
    assert!(w.campaign.character("bo").unwrap().is_dead());
    // at 0 HP but alive: no long rest either
    w.campaign.character_mut("ash").unwrap().hp.current = 0;
    assert!(w.rest("ash", true).unwrap_err().contains("at least 1 hit point"));
    w.campaign.character_mut("ash").unwrap().hp.current = 30;
    // revivify brings bo back with 1 hit point
    w.campaign.character_mut("ash").unwrap().spells.prepared = vec!["revivify".into()];
    let r = w.pc_cast("ash", "revivify", 3, &[token.clone()], dnd_core::play::RollMode::Normal).unwrap();
    assert!(r.text.contains("returns to life"), "{}", r.text);
    let bo = w.campaign.character("bo").unwrap();
    assert!(!bo.is_dead() && bo.hp.current == 1);
    assert!(w.rest("bo", true).is_ok(), "alive again, so resting works");
}
