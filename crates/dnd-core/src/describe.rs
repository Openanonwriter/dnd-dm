//! Plain-text summaries of content entries for the (i) popups and hover
//! tips: everything a new player needs to know about a class, subclass,
//! species, background, feat, item or spell before picking it.

use crate::glossary;
use crate::library::Library;
use crate::model::*;

/// A readable summary of one entry, or None if it doesn't exist.
pub fn describe(lib: &Library, kind: Kind, id: &str) -> Option<String> {
    Some(match kind {
        Kind::Species => species(lib.species.get(id)?),
        Kind::Class => class(lib, lib.classes.get(id)?),
        Kind::Subclass => subclass(lib, lib.subclasses.get(id)?),
        Kind::Background => background(lib, lib.backgrounds.get(id)?),
        Kind::Feat => feat(lib.feats.get(id)?),
        Kind::Item => item(lib.items.get(id)?),
        Kind::Spell => spell(lib.spells.get(id)?),
        Kind::Creature => creature(lib.creatures.get(id)?),
    })
}

/// The first sentence or so of a description, for a short hover tip.
pub fn short(text: &str) -> String {
    let t = text.trim();
    let cut = t.char_indices().find(|(i, c)| *c == '.' && *i > 40).map(|(i, _)| i + 1).unwrap_or(t.len());
    let s = &t[..cut];
    if s.chars().count() > 220 {
        format!("{}…", s.chars().take(220).collect::<String>())
    } else {
        s.to_string()
    }
}

fn push(out: &mut String, line: impl AsRef<str>) {
    let l = line.as_ref().trim();
    if l.is_empty() {
        return;
    }
    if !out.is_empty() {
        out.push('\n');
    }
    out.push_str(l);
}

fn ability_name(k: &str) -> &'static str {
    match k {
        "str" => "Strength",
        "dex" => "Dexterity",
        "con" => "Constitution",
        "int" => "Intelligence",
        "wis" => "Wisdom",
        "cha" => "Charisma",
        _ => "",
    }
}

fn abilities(list: &[String]) -> String {
    list.iter().map(|a| ability_name(a).to_string()).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(", ")
}

fn nice(s: &str) -> String {
    s.replace('_', " ")
}

fn skill_names(lib: &Library, keys: &[String]) -> String {
    keys.iter().map(|k| lib.rules.skill(k).map(|s| s.name.clone()).unwrap_or_else(|| nice(k))).collect::<Vec<_>>().join(", ")
}

fn ability_bonuses(mods: &[Modifier]) -> String {
    mods.iter()
        .filter_map(|m| {
            let k = m.target.strip_prefix("ability.")?;
            Some(format!("{} +{}", ability_name(k), m.value))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn features(out: &mut String, list: &[Feature], with_level: bool) {
    for f in list {
        let lvl = if with_level { format!("Level {}: ", f.level) } else if f.level > 1 { format!("(from level {}) ", f.level) } else { String::new() };
        let uses = if f.uses.is_empty() { String::new() } else { format!(" [limited uses, back after a {}]", nice(&f.recharge)) };
        push(out, format!("• {lvl}{}: {}{uses}", f.name, f.description));
    }
}

fn species(s: &Species) -> String {
    let mut o = String::new();
    push(&mut o, &s.description);
    let mut facts = vec![s.size.clone(), format!("speed {} ft", s.speed)];
    if s.darkvision > 0 {
        facts.push(format!("darkvision {} ft", s.darkvision));
    }
    push(&mut o, facts.join(" · "));
    let b = ability_bonuses(&s.modifiers);
    if !b.is_empty() {
        push(&mut o, format!("Ability scores: {b}"));
    }
    if !s.languages.is_empty() {
        push(&mut o, format!("Languages: {}", s.languages.join(", ")));
    }
    if !s.traits.is_empty() {
        push(&mut o, "Traits:");
        features(&mut o, &s.traits, false);
    }
    o
}

fn class(lib: &Library, c: &Class) -> String {
    let mut o = String::new();
    push(&mut o, &c.description);
    let mut facts = vec![format!("Hit die d{}", c.hit_die)];
    if !c.primary_abilities.is_empty() {
        facts.push(format!("main ability {}", abilities(&c.primary_abilities)));
    }
    if !c.saving_throws.is_empty() {
        facts.push(format!("saves {}", abilities(&c.saving_throws)));
    }
    push(&mut o, facts.join(" · "));
    if !c.armor_proficiencies.is_empty() {
        push(&mut o, format!("Armor: {}", c.armor_proficiencies.join(", ")));
    }
    if !c.weapon_proficiencies.is_empty() {
        push(&mut o, format!("Weapons: {}", c.weapon_proficiencies.iter().map(|w| nice(w)).collect::<Vec<_>>().join(", ")));
    }
    if c.skill_choices.count > 0 {
        let from = if c.skill_choices.from.is_empty() { "any skills".to_string() } else { skill_names(lib, &c.skill_choices.from) };
        push(&mut o, format!("Skills: choose {} from {from}", c.skill_choices.count));
    }
    if let Some(sc) = &c.spellcasting {
        let how = if !sc.spells_known.is_empty() {
            "knows a set number of spells"
        } else if sc.spellbook_start > 0 {
            "keeps a spellbook and prepares from it"
        } else {
            "prepares spells each day from the whole class list"
        };
        push(&mut o, format!("Spellcasting with {}: {how}.", ability_name(&sc.ability)));
    }
    let subs: Vec<String> = lib.subclasses_of(&c.id).iter().map(|s| s.name.clone()).collect();
    let label = if c.subclass_label.is_empty() { "Subclass" } else { &c.subclass_label };
    push(&mut o, format!("{label} at level {}: {}", c.subclass_level, if subs.is_empty() { "none in the library yet".to_string() } else { subs.join(", ") }));
    if !c.features.is_empty() {
        push(&mut o, "Features:");
        features(&mut o, &c.features, true);
    }
    o
}

fn subclass(lib: &Library, s: &Subclass) -> String {
    let mut o = String::new();
    let parent = lib.classes.get(&s.class).map(|c| c.name.clone()).unwrap_or_else(|| s.class.clone());
    push(&mut o, format!("A {parent} specialty."));
    push(&mut o, &s.description);
    if !s.features.is_empty() {
        push(&mut o, "Features:");
        features(&mut o, &s.features, true);
    }
    o
}

fn background(lib: &Library, b: &Background) -> String {
    let mut o = String::new();
    push(&mut o, &b.description);
    if !b.skill_proficiencies.is_empty() {
        push(&mut o, format!("Skills: {}", skill_names(lib, &b.skill_proficiencies)));
    }
    if !b.tool_proficiencies.is_empty() {
        push(&mut o, format!("Tools: {}", b.tool_proficiencies.join(", ")));
    }
    if b.language_choices > 0 || !b.languages.is_empty() {
        let mut l = b.languages.clone();
        if b.language_choices > 0 {
            l.push(format!("{} of your choice", b.language_choices));
        }
        push(&mut o, format!("Languages: {}", l.join(", ")));
    }
    if !b.starting_equipment.is_empty() {
        push(&mut o, format!("Equipment: {}", b.starting_equipment.join(", ")));
    }
    if let Some(f) = &b.feature {
        push(&mut o, format!("Feature — {}: {}", f.name, f.description));
    }
    for v in &b.variants {
        push(&mut o, format!("Variant — {}: {}", v.name, v.description));
    }
    if !b.suggestions.is_empty() {
        push(&mut o, format!("Suggested characteristics: {}.", b.suggestions.iter().map(|t| t.name.to_lowercase()).collect::<Vec<_>>().join(", ")));
    }
    o
}

fn feat(f: &Feat) -> String {
    let mut o = String::new();
    if !f.prerequisite.is_empty() {
        push(&mut o, format!("Needs: {}", f.prerequisite));
    }
    push(&mut o, &f.description);
    o
}

fn item(i: &Item) -> String {
    let mut o = String::new();
    let mut facts = vec![nice(&i.kind)];
    if !i.cost.is_empty() {
        facts.push(i.cost.clone());
    }
    if i.weight > 0.0 {
        facts.push(format!("{} lb", i.weight));
    }
    if !i.rarity.is_empty() {
        facts.push(i.rarity.clone());
    }
    if i.requires_attunement {
        facts.push("needs attunement".into());
    }
    push(&mut o, facts.join(" · "));
    if let Some(w) = &i.weapon {
        let bonus = if w.bonus != 0 { format!(" (+{} magic)", w.bonus) } else { String::new() };
        let mut line = format!("{} {} weapon: {} {}{bonus}", nice(&w.category), w.kind, w.damage, w.damage_type);
        if !w.versatile.is_empty() {
            line.push_str(&format!(", {} with two hands", w.versatile));
        }
        if !w.range.is_empty() {
            line.push_str(&format!(", range {} ft", w.range));
        }
        push(&mut o, line);
        for p in &w.properties {
            match glossary::explain(p) {
                Some(t) => push(&mut o, format!("• {}: {t}", nice(p))),
                None => push(&mut o, format!("• {}", nice(p))),
            }
        }
    }
    if let Some(a) = &i.armor {
        let line = match (a.category.as_str(), a.dex_cap) {
            ("shield", _) => format!("Shield: +{} AC while carried", a.base_ac + a.bonus),
            (_, Some(0)) => format!("{} armor: AC {}", a.category, a.base_ac + a.bonus),
            (_, Some(cap)) => format!("{} armor: AC {} + Dexterity modifier (max +{cap})", a.category, a.base_ac + a.bonus),
            (_, None) => format!("{} armor: AC {} + Dexterity modifier", a.category, a.base_ac + a.bonus),
        };
        push(&mut o, line);
        if a.strength_requirement > 0 {
            push(&mut o, format!("Needs Strength {} or your speed drops by 10 ft.", a.strength_requirement));
        }
        if a.stealth_disadvantage {
            push(&mut o, "Noisy: disadvantage on Stealth checks while worn.");
        }
    }
    push(&mut o, &i.description);
    o
}

fn spell(s: &Spell) -> String {
    let mut o = String::new();
    let lvl = if s.level == 0 { format!("{} cantrip", s.school) } else { format!("Level {} {}", s.level, s.school) };
    let tags: Vec<&str> = [if s.ritual { Some("ritual") } else { None }, if s.concentration { Some("concentration") } else { None }].into_iter().flatten().collect();
    push(&mut o, if tags.is_empty() { lvl } else { format!("{lvl} ({})", tags.join(", ")) });
    push(&mut o, format!("{} · {} · {} · {}", s.casting_time, s.range, s.components, s.duration));
    let classes: Vec<String> = s.classes.iter().map(|c| nice(c)).collect();
    if !classes.is_empty() {
        push(&mut o, format!("Classes: {}", classes.join(", ")));
    }
    push(&mut o, &s.description);
    if !s.higher_levels.is_empty() {
        push(&mut o, format!("Higher levels: {}", s.higher_levels));
    }
    o
}

fn creature(c: &Creature) -> String {
    let mut o = String::new();
    push(&mut o, format!("{} {} · AC {} · {} HP · CR {}", c.size, c.kind, c.ac, c.hp, c.cr));
    push(&mut o, &c.description);
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_takes_the_first_sentence() {
        assert_eq!(short("A priestly champion who wields divine magic. More text here."), "A priestly champion who wields divine magic.");
    }
}
