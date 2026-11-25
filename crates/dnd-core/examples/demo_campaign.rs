//! Writes a small campaign to try the maps with: a dungeon in play with a
//! party and some goblins, and a cave and a forest road on the shelf.
//!
//! ```text
//! cargo run -p dnd-core --example demo_campaign -- content saves
//! ```

use dnd_core::grid::Pos;
use dnd_core::model::*;
use dnd_core::{mapgen, save, Rng, World};
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let content = PathBuf::from(args.next().unwrap_or_else(|| "content".into()));
    let saves = PathBuf::from(args.next().unwrap_or_else(|| "saves".into()));
    let mut w = World::new(vec![content]);
    w.campaign = Campaign::new("Demo - The Sunless Cellar");
    let mut rng = Rng::seeded(7);

    for (id, name, class) in [("thia", "Thia", "wizard"), ("borin", "Borin", "fighter"), ("mist", "Mist", "rogue")] {
        let mut c = Character::new(id, name);
        c.species = "human".into();
        c.classes.push(ClassLevel { class: class.into(), level: 3, ..Default::default() });
        let (abilities, gear): (&[(&str, i32)], &[&str]) = match class {
            "wizard" => (&[("int", 16), ("dex", 14), ("con", 13)], &["quarterstaff", "dagger", "spellbook"]),
            "fighter" => (&[("str", 16), ("con", 15), ("dex", 12)], &["longsword", "chain_mail", "shield", "light_crossbow"]),
            _ => (&[("dex", 16), ("con", 13), ("int", 12)], &["rapier", "shortbow", "leather_armor", "thieves_tools"]),
        };
        for (k, v) in abilities {
            c.base_abilities.set(k, *v);
        }
        c.inventory = gear.iter().map(|g| InventoryItem { item: g.to_string(), qty: 1, equipped: true, ..Default::default() }).collect();
        if class == "wizard" {
            c.spells.known = ["fire_bolt", "light", "mage_hand", "magic_missile", "shield", "mage_armor", "sleep", "burning_hands", "detect_magic", "scorching_ray", "misty_step", "web", "shatter"].iter().map(|s| s.to_string()).collect();
            c.spells.prepared = ["magic_missile", "shield", "sleep", "burning_hands", "scorching_ray", "misty_step"].iter().map(|s| s.to_string()).collect();
        }
        c.hp.current = dnd_core::derive(&c, &w.library).hp_max;
        w.campaign.characters.push(c);
    }

    let mut dungeon = mapgen::dungeon("The Sunless Cellar", 44, 30, &mut rng);
    dungeon.chapter = "Chapter 1".into();
    w.campaign.map = dungeon;
    w.campaign.map.id = "the_sunless_cellar".into();
    for id in ["thia", "borin", "mist"] {
        if let Some(mut c) = w.combatant_from_character(id) {
            c.visible = true;
            c.hp = c.hp_max;
            if id == "borin" {
                c.light = 20;
            }
            w.campaign.encounter.combatants.push(c);
        }
    }
    let rules = w.library.rules.clone();
    w.campaign.place_party(|s| rules.size_squares(s));
    // goblins somewhere else in the dungeon, well away from the way in
    let start = w.campaign.map.start.unwrap_or_default();
    let mut far: Vec<Pos> = (0..w.campaign.map.height as i32)
        .flat_map(|y| (0..w.campaign.map.width as i32).map(move |x| Pos::new(x, y)))
        .filter(|p| !w.campaign.map.solid(p.x, p.y) && (p.x - start.x).abs() + (p.y - start.y).abs() > 14)
        .collect();
    for i in 0..5 {
        if far.is_empty() {
            break;
        }
        let p = far.remove((rng.next_u64() % far.len() as u64) as usize);
        let made = w.combatant_from_creature("goblin", Some(&format!("Goblin {}", i + 1)));
        let mut c = made.unwrap_or(Combatant { id: format!("goblin_{}", i + 1), name: format!("Goblin {}", i + 1), hp: 7, hp_max: 7, ac: 15, ..Default::default() });
        c.visible = true;
        c.pos = Some(p);
        w.campaign.encounter.combatants.push(c);
    }
    w.update_explored();

    let mut cave = mapgen::cave("Spider Caves", 44, 30, &mut rng);
    cave.chapter = "Chapter 1".into();
    w.campaign.add_map(cave);
    let mut road = mapgen::wilderness("The Forest Road", 44, 30, &mut rng);
    road.chapter = "Chapter 2".into();
    w.campaign.add_map(road);

    let dir = saves.join(Character::slug(&w.campaign.name));
    save::save_campaign(&mut w.campaign, &dir)?;
    println!("wrote {}", dir.display());
    Ok(())
}
