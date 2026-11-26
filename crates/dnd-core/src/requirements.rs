//! Rule checks: does a character qualify for what it has picked? The DM may
//! give a character anything; the apps show a red outline and the reason.
//! Players may only pick what their character qualifies for, unless the DM
//! allows rule breaks for the whole campaign or for that character.

use crate::expr::{self, Env};
use crate::library::Library;
use crate::model::*;
use crate::sheet::Sheet;

/// What part of the character a problem is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Area {
    Class,
    Subclass,
    Choice,
    Feat,
    Advancement,
    Skill,
    Spell,
    Item,
}

/// One broken rule. `key` names the thing (a feat, spell, skill or item id,
/// a class id, or a feature name for a choice count).
#[derive(Debug, Clone, PartialEq)]
pub struct Problem {
    pub area: Area,
    pub key: String,
    pub message: String,
}

/// May this character's player pick things the character doesn't qualify for?
pub fn may_break_rules(campaign: &Campaign, ch: &Character) -> bool {
    campaign.settings.allow_rule_breaks || ch.allow_rule_breaks
}

/// The variables a requirement formula can use for this character: the
/// sheet's own (levels, scores, class levels...) plus 1/0 flags for feats,
/// options, known or granted spells, species, background and subclasses.
pub fn env(ch: &Character, sheet: &Sheet) -> Env {
    let mut e = Env::new();
    for (k, v) in &sheet.env {
        e.set(k.clone(), *v);
    }
    for f in ch.all_feats() {
        e.set(format!("feat.{f}"), 1.0);
    }
    for picks in ch.choices.values() {
        for f in picks {
            e.set(format!("feat.{f}"), 1.0);
        }
    }
    for s in ch.spells.known.iter().chain(sheet.spells_granted.iter()) {
        e.set(format!("spell.{s}"), 1.0);
    }
    if !ch.species.is_empty() {
        e.set(format!("species.{}", ch.species), 1.0);
    }
    if !ch.background.is_empty() {
        e.set(format!("background.{}", ch.background), 1.0);
    }
    for cl in &ch.classes {
        if let Some(s) = &cl.subclass {
            e.set(format!("subclass.{s}"), 1.0);
        }
    }
    e
}

/// Evaluate a requirement formula. Flags the character doesn't have
/// (`feat.x`, `spell.y`, `class.z.level`...) count as 0.
pub fn meets(env: &Env, formula: &str) -> bool {
    if formula.trim().is_empty() {
        return true;
    }
    let mut e = env.clone();
    for word in formula.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '.')) {
        let flag = ["feat.", "spell.", "species.", "background.", "subclass.", "class."].iter().any(|p| word.starts_with(p));
        if flag && e.get(word).is_none() {
            e.set(word.to_string(), 0.0);
        }
    }
    expr::eval_cond(formula, &e).unwrap_or(false)
}

/// Why a character doesn't qualify for a feat or class option, or None.
pub fn feat_failure(env: &Env, feat: &Feat) -> Option<String> {
    if meets(env, &feat.requires) {
        return None;
    }
    Some(if feat.prerequisite.is_empty() { format!("{} needs {}", feat.name, feat.requires) } else { format!("{} needs {}", feat.name, feat.prerequisite) })
}

/// "Strength 13 or Dexterity 13" -> "str >= 13 || dex >= 13".
pub fn multiclass_formula(text: &str) -> String {
    let names = [("strength", "str"), ("dexterity", "dex"), ("constitution", "con"), ("intelligence", "int"), ("wisdom", "wis"), ("charisma", "cha")];
    let mut out = String::new();
    for word in text.to_lowercase().replace(',', " ").split_whitespace() {
        match word {
            "and" => out.push_str(" && "),
            "or" => out.push_str(" || "),
            w => {
                if let Some((_, k)) = names.iter().find(|(n, k)| *n == w || *k == w) {
                    out.push_str(k);
                    out.push_str(" >= ");
                } else if w.chars().all(|c| c.is_ascii_digit()) {
                    out.push_str(w);
                }
            }
        }
    }
    out
}

/// Can a spellcasting block learn or cast this spell?
pub fn spell_failure(lib: &Library, sheet: &Sheet, spell: &Spell) -> Option<String> {
    if sheet.spellcasting.is_empty() {
        return Some(format!("{} — this character can't cast spells", spell.name));
    }
    let on: Vec<_> = sheet.spellcasting.iter().filter(|b| crate::play::on_list(lib, b, spell)).collect();
    if on.is_empty() {
        return Some(format!("{} isn't on this character's spell list", spell.name));
    }
    let top = on.iter().map(|b| b.max_spell_level).max().unwrap_or(0);
    if spell.level > top {
        return Some(format!("{} is a level {} spell; this character can cast up to level {top}", spell.name, spell.level));
    }
    None
}

/// Every rule this character breaks.
pub fn check(lib: &Library, ch: &Character) -> Vec<Problem> {
    let sheet = crate::sheet::derive(ch, lib);
    check_with(lib, ch, &sheet)
}

/// [`check`] with a sheet you already have.
pub fn check_with(lib: &Library, ch: &Character, sheet: &Sheet) -> Vec<Problem> {
    let env = env(ch, sheet);
    let mut out = Vec::new();
    let mut add = |area: Area, key: &str, message: String| out.push(Problem { area, key: key.to_string(), message });

    // ---- classes and subclasses
    let multi = ch.classes.len() > 1;
    for cl in &ch.classes {
        let Some(class) = lib.classes.get(&cl.class) else { continue };
        if multi && !class.multiclass_prerequisite.is_empty() && !meets(&env, &multiclass_formula(&class.multiclass_prerequisite)) {
            add(Area::Class, &class.id, format!("Multiclassing as a {} needs {}", class.name, class.multiclass_prerequisite));
        }
        if let Some(sub_id) = &cl.subclass {
            match lib.subclasses.get(sub_id) {
                Some(sub) if sub.class != class.id => add(Area::Subclass, sub_id, format!("{} is not a {} subclass", sub.name, class.name)),
                Some(sub) if cl.level < class.subclass_level => {
                    let label = if class.subclass_label.is_empty() { "subclass" } else { &class.subclass_label };
                    add(Area::Subclass, sub_id, format!("{} ({label}) comes at {} level {}; this character is level {}", sub.name, class.name, class.subclass_level, cl.level))
                }
                _ => {}
            }
        }
    }

    // ---- choices
    for f in sheet.features.iter().filter(|f| !f.options.is_empty()) {
        if f.chosen.len() as u32 > f.picks {
            add(Area::Choice, &f.name, format!("{}: {} picked, {} allowed", f.name, f.chosen.len(), f.picks));
        }
        for id in &f.chosen {
            match lib.feats.get(id) {
                Some(feat) if feat.tags.iter().any(|t| t == &f.options) => {
                    if let Some(why) = feat_failure(&env, feat) {
                        add(Area::Choice, id, why);
                    }
                }
                _ => add(Area::Choice, id, format!("{}: '{id}' is not one of its options", f.name)),
            }
        }
    }

    // ---- feats
    for id in ch.all_feats() {
        if let Some(feat) = lib.feats.get(&id) {
            if crate::builder::is_option_feat(feat) {
                add(Area::Feat, &id, format!("{} is a class option, picked through its feature rather than as a feat", feat.name));
            } else if let Some(why) = feat_failure(&env, feat) {
                add(Area::Feat, &id, why);
            }
        }
    }

    // ---- ability score improvements (single class: we know which levels count)
    if let [cl] = ch.classes.as_slice() {
        if let Some(class) = lib.classes.get(&cl.class) {
            let due = crate::builder::improvement_levels(class, cl.level);
            for a in &ch.advancements {
                if !due.contains(&a.level) {
                    add(Area::Advancement, &a.level.to_string(), format!("No ability score improvement is due at level {} (a {} {} gets them at {})", a.level, class.name, cl.level, class.asi_levels.iter().map(|l| l.to_string()).collect::<Vec<_>>().join(", ")));
                }
                if a.feat.is_none() && a.ability_increases.values().sum::<i32>() != 2 {
                    add(Area::Advancement, &a.level.to_string(), format!("Level {} improvement should add 2 points in total", a.level));
                }
            }
        }
    }

    // ---- skills (single class: the class says which and how many)
    if let [cl] = ch.classes.as_slice() {
        if let Some(class) = lib.classes.get(&cl.class) {
            let allowed = crate::builder::skill_options(class, &lib.rules);
            let from_bg: Vec<String> = lib.backgrounds.get(&ch.background).map(|b| b.skill_proficiencies.clone()).unwrap_or_default();
            let class_picks: Vec<&String> = ch.skill_proficiencies.iter().filter(|s| !from_bg.contains(s)).collect();
            for s in &class_picks {
                if !allowed.contains(s) {
                    let name = lib.rules.skill(s).map(|x| x.name.clone()).unwrap_or_else(|| s.to_string());
                    add(Area::Skill, s, format!("{name} isn't on the {} skill list", class.name));
                }
            }
            if class_picks.len() as u32 > class.skill_choices.count {
                add(Area::Skill, "", format!("{} skills picked; a {} chooses {}", class_picks.len(), class.name, class.skill_choices.count));
            }
        }
    }

    // ---- spells
    let mut cantrips = 0u32;
    let mut leveled = 0u32;
    for id in &ch.spells.known {
        let Some(spell) = lib.spells.get(id) else { continue };
        if spell.level == 0 {
            cantrips += 1;
        } else {
            leveled += 1;
        }
        if let Some(why) = spell_failure(lib, sheet, spell) {
            add(Area::Spell, id, why);
        }
    }
    let cantrip_max: u32 = sheet.spellcasting.iter().map(|b| b.cantrips_known).sum();
    if cantrips > cantrip_max && !sheet.spellcasting.is_empty() {
        add(Area::Spell, "", format!("{cantrips} cantrips known; this character can know {cantrip_max}"));
    }
    if sheet.spellcasting.iter().any(|b| b.spell_limit.is_some()) {
        let spell_max: u32 = sheet.spellcasting.iter().filter_map(|b| b.spell_limit).sum();
        if leveled > spell_max {
            add(Area::Spell, "", format!("{leveled} spells known; this character can have {spell_max}"));
        }
    }
    if let Some(prep_max) = sheet.spellcasting.iter().filter_map(|b| b.prepared_max).reduce(|a, b| a + b) {
        let prepared = ch.spells.prepared.iter().filter(|p| !sheet.spells_granted.contains(p) && lib.spells.get(*p).map(|s| s.level > 0).unwrap_or(false)).count() as i32;
        if prepared > prep_max {
            add(Area::Spell, "", format!("{prepared} spells prepared; this character can prepare {prep_max}"));
        }
    }

    // ---- equipped armor
    for inv in ch.inventory.iter().filter(|i| i.equipped && !i.custom) {
        let Some(item) = lib.items.get(&inv.item) else { continue };
        let Some(a) = &item.armor else { continue };
        let cat = if a.category == "shield" { "shields" } else { a.category.as_str() };
        if !cat.is_empty() && !sheet.proficiencies.armor.iter().any(|p| p == cat || p == &item.id) {
            add(Area::Item, &item.id, format!("Not proficient with {}: disadvantage on Strength and Dexterity rolls and attacks, and no spellcasting while wearing it", item.name));
        }
        if a.strength_requirement > 0 && sheet.abilities.str < a.strength_requirement {
            add(Area::Item, &item.id, format!("{} needs Strength {}: speed drops by 10 feet", item.name, a.strength_requirement));
        }
    }
    out
}

/// The problems for one thing, joined for a tooltip.
pub fn reasons(problems: &[Problem], area: Area, key: &str) -> Option<String> {
    let v: Vec<&str> = problems.iter().filter(|p| p.area == area && p.key == key).map(|p| p.message.as_str()).collect();
    if v.is_empty() {
        None
    } else {
        Some(v.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiclass_text_becomes_a_formula() {
        assert_eq!(multiclass_formula("Strength 13 or Dexterity 13"), "str >= 13 || dex >= 13");
        assert_eq!(multiclass_formula("Dexterity 13 and Wisdom 13"), "dex >= 13 && wis >= 13");
    }

    #[test]
    fn missing_flags_count_as_zero() {
        let mut e = Env::new();
        e.set("class.warlock.level", 5.0);
        assert!(meets(&e, "class.warlock.level >= 5"));
        assert!(!meets(&e, "class.warlock.level >= 5 && feat.pact_blade"));
        e.set("feat.pact_blade", 1.0);
        assert!(meets(&e, "class.warlock.level >= 5 && feat.pact_blade"));
        assert!(!meets(&e, "class.paladin.level >= 1"));
    }
}
