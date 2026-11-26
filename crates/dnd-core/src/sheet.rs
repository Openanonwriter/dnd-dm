//! Character sheet derivation: turns a [`Character`] plus the [`Library`]
//! into every number on a 5e sheet, with a trace of where each bonus came
//! from so the DM can see *why* a value is what it is.

use crate::expr::{self, Env, Mode};
use crate::library::Library;
use crate::model::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SaveLine {
    pub key: String,
    pub name: String,
    pub bonus: i32,
    pub proficient: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SkillLine {
    pub key: String,
    pub name: String,
    pub ability: String,
    pub bonus: i32,
    pub proficient: bool,
    pub expertise: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct AttackLine {
    pub name: String,
    pub bonus: i32,
    pub damage: String,
    pub damage_type: String,
    pub range: String,
    pub properties: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SpellcastingBlock {
    pub class: String,
    pub class_name: String,
    pub ability: String,
    pub attack_bonus: i32,
    pub save_dc: i32,
    pub prepared_max: Option<i32>,
    pub cantrips_known: u32,
    pub spells_known: Option<u32>,
    pub progression: String,
    /// How many spells of 1st level or higher the character may have chosen
    /// (known casters and spellbooks). None = prepares from the whole class list.
    #[serde(default)]
    pub spell_limit: Option<u32>,
    /// Highest spell level this class can cast now.
    #[serde(default)]
    pub max_spell_level: u32,
    /// Extra spell lists this block may draw from besides its class: the
    /// subclass id (warlock patron spells) and the class's `spell_lists`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lists: Vec<String>,
    #[serde(default)]
    pub ability_mod: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct FeatureLine {
    pub name: String,
    pub source: String,
    pub level: u32,
    pub description: String,
    pub uses: Option<i64>,
    pub recharge: String,
    /// For a feature that offers a choice: the feat tag of its options,
    /// how many to pick, and the feat ids picked so far.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub options: String,
    #[serde(default)]
    pub picks: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub chosen: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct InventoryLine {
    pub item: String,
    pub name: String,
    pub qty: u32,
    pub weight: f64,
    pub equipped: bool,
    pub attuned: bool,
    pub kind: String,
    pub notes: String,
    pub hidden: bool,
    pub dm_notes: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Proficiencies {
    pub armor: Vec<String>,
    pub weapons: Vec<String>,
    pub tools: Vec<String>,
    pub languages: Vec<String>,
}

/// One modifier after evaluation, kept for the "why" panel.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ModifierTrace {
    pub target: String,
    pub op: String,
    pub value: String,
    pub resolved: Option<f64>,
    pub source: String,
    pub applied: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Sheet {
    pub character_id: String,
    pub name: String,
    pub player: String,
    pub species: String,
    pub background: String,
    pub class_line: String,
    pub level: u32,
    pub xp: u32,
    pub next_level_xp: Option<u32>,
    pub alignment: String,
    pub abilities: Abilities,
    pub ability_mods: BTreeMap<String, i32>,
    pub proficiency_bonus: i32,
    pub saves: Vec<SaveLine>,
    pub skills: Vec<SkillLine>,
    pub passive_perception: i32,
    pub passive_investigation: i32,
    pub passive_insight: i32,
    pub ac: i32,
    pub ac_breakdown: String,
    pub hp_max: i32,
    pub hp_current: i32,
    pub hp_temp: i32,
    pub hit_dice: BTreeMap<String, u32>,
    pub hit_dice_used: BTreeMap<String, u32>,
    pub initiative: i32,
    pub speed: i32,
    pub speeds: BTreeMap<String, i32>,
    pub darkvision: i32,
    pub size: String,
    pub attacks: Vec<AttackLine>,
    pub spellcasting: Vec<SpellcastingBlock>,
    pub spell_slots: Vec<u32>,
    pub spell_slots_used: Vec<u32>,
    pub pact_slots: Option<(u32, u32)>,
    pub features: Vec<FeatureLine>,
    pub proficiencies: Proficiencies,
    pub inventory: Vec<InventoryLine>,
    pub total_weight: f64,
    pub carrying_capacity: i32,
    pub conditions: Vec<String>,
    pub exhaustion: u32,
    pub inspiration: bool,
    pub death_saves: DeathSaves,
    pub currency: Currency,
    pub spells_known: Vec<String>,
    pub spells_prepared: Vec<String>,
    /// Spells granted by features and feats (domain spells, racial magic):
    /// always prepared and never counted against the limits.
    #[serde(default)]
    pub spells_granted: Vec<String>,
    /// Targets the engine does not know; homebrew can read them.
    pub other: BTreeMap<String, f64>,
    /// Advantage / disadvantage / notes keyed by target.
    pub notes: BTreeMap<String, Vec<String>>,
    pub warnings: Vec<String>,
    pub trace: Vec<ModifierTrace>,
    pub env: BTreeMap<String, f64>,
}

// ------------------------------------------------------------------ internals

#[derive(Debug, Clone)]
struct Pending {
    m: Modifier,
    source: String,
}

#[derive(Debug, Clone)]
struct Resolved {
    op: ModOp,
    value: f64,
    source: String,
    text: String,
}

struct Engine<'a> {
    lib: &'a Library,
    ch: &'a Character,
    env: Env,
    mods: Vec<Pending>,
    resolved: BTreeMap<String, Vec<Resolved>>,
    features: Vec<(String, Feature)>,
    sheet: Sheet,
}

fn apply_ops(base: f64, list: &[Resolved]) -> f64 {
    let mut v = base;
    for r in list.iter().filter(|r| r.op == ModOp::Set) {
        v = r.value;
    }
    for r in list.iter().filter(|r| r.op == ModOp::Add) {
        v += r.value;
    }
    for r in list.iter().filter(|r| r.op == ModOp::Mul) {
        v *= r.value;
    }
    for r in list.iter().filter(|r| r.op == ModOp::Max) {
        v = v.max(r.value);
    }
    for r in list.iter().filter(|r| r.op == ModOp::Min) {
        v = v.min(r.value);
    }
    v
}

fn norm(s: &str) -> String {
    s.trim().to_lowercase().replace([' ', '-'], "_")
}

impl<'a> Engine<'a> {
    fn num(&self, target: &str, base: i32) -> i32 {
        match self.resolved.get(target) {
            Some(list) => apply_ops(base as f64, list).round() as i32,
            None => base,
        }
    }
    fn has_grant(&self, target: &str) -> bool {
        self.resolved.get(target).map(|l| l.iter().any(|r| r.op == ModOp::Grant)).unwrap_or(false)
    }
    fn has_expertise(&self, target: &str) -> bool {
        self.resolved.get(target).map(|l| l.iter().any(|r| r.op == ModOp::Expertise)).unwrap_or(false)
    }
    fn notes_for(&self, target: &str) -> Vec<String> {
        self.sheet.notes.get(target).cloned().unwrap_or_default()
    }
    fn granted(&self, prefix: &str) -> Vec<String> {
        self.resolved
            .iter()
            .filter(|(k, l)| k.starts_with(prefix) && l.iter().any(|r| r.op == ModOp::Grant))
            .map(|(k, _)| k[prefix.len()..].to_string())
            .collect()
    }

    fn push_mods(&mut self, mods: &[Modifier], source: &str) {
        for m in mods {
            let src = if m.source.is_empty() { source.to_string() } else { m.source.clone() };
            self.mods.push(Pending { m: m.clone(), source: src });
        }
    }

    fn grant_spells(&mut self, spells: &[String]) {
        for s in spells {
            if !self.sheet.spells_granted.contains(s) {
                self.sheet.spells_granted.push(s.clone());
            }
        }
    }

    fn push_feature(&mut self, f: &Feature, source: &str) {
        let ch = self.ch;
        let lib = self.lib;
        self.push_mods(&f.modifiers, &format!("{source}: {}", f.name));
        self.grant_spells(&f.spells);
        self.features.push((source.to_string(), f.clone()));
        if f.options.is_empty() {
            return;
        }
        // the picks for a choice: feats carrying the option tag
        for id in ch.choices.get(&f.name).map(|v| v.as_slice()).unwrap_or(&[]) {
            match lib.feats.get(id) {
                Some(feat) if feat.tags.iter().any(|t| t == &f.options) => {
                    let name = format!("{}: {}", f.name, feat.name);
                    self.push_mods(&feat.modifiers, &name);
                    self.grant_spells(&feat.spells);
                    self.features.push((source.to_string(), Feature { name, level: f.level, description: feat.description.clone(), ..Default::default() }));
                }
                _ => self.sheet.warnings.push(format!("{}: '{id}' is not one of its options", f.name)),
            }
        }
    }

    /// Gather every modifier & feature from the character's choices.
    fn gather(&mut self) {
        let ch = self.ch;
        let lib = self.lib;
        if let Some(sp) = lib.species.get(&ch.species) {
            self.push_mods(&sp.modifiers, &sp.name);
            // species traits can unlock at a character level (tiefling magic)
            for t in sp.traits.iter().filter(|t| t.level <= ch.total_level()) {
                self.push_feature(t, &sp.name);
            }
        } else if !ch.species.is_empty() {
            self.sheet.warnings.push(format!("unknown species '{}'", ch.species));
        }
        if let Some(bg) = lib.backgrounds.get(&ch.background) {
            self.push_mods(&bg.modifiers, &bg.name);
            if let Some(f) = &bg.feature {
                self.push_feature(f, &bg.name);
            }
        } else if !ch.background.is_empty() {
            self.sheet.warnings.push(format!("unknown background '{}'", ch.background));
        }
        for cl in &ch.classes {
            match lib.classes.get(&cl.class) {
                Some(class) => {
                    self.push_mods(&class.modifiers, &class.name);
                    for f in class.features.iter().filter(|f| f.level <= cl.level) {
                        self.push_feature(f, &class.name);
                    }
                    if let Some(sub_id) = &cl.subclass {
                        match lib.subclasses.get(sub_id) {
                            Some(sub) => {
                                self.push_mods(&sub.modifiers, &sub.name);
                                for f in sub.features.iter().filter(|f| f.level <= cl.level) {
                                    self.push_feature(f, &sub.name);
                                }
                            }
                            None => self.sheet.warnings.push(format!("unknown subclass '{sub_id}'")),
                        }
                    }
                }
                None => self.sheet.warnings.push(format!("unknown class '{}'", cl.class)),
            }
        }
        for feat_id in ch.all_feats() {
            match lib.feats.get(&feat_id) {
                Some(feat) => {
                    self.push_mods(&feat.modifiers, &feat.name);
                    self.grant_spells(&feat.spells);
                    self.features.push((
                        "Feat".into(),
                        Feature { name: feat.name.clone(), level: 1, description: feat.description.clone(), ..Default::default() },
                    ));
                }
                None => self.sheet.warnings.push(format!("unknown feat '{feat_id}'")),
            }
        }
        for adv in &ch.advancements {
            for (k, v) in &adv.ability_increases {
                self.mods.push(Pending {
                    m: Modifier::new(format!("ability.{}", norm(k)), ModOp::Add, v.to_string()),
                    source: format!("Level {} ASI", adv.level),
                });
            }
        }
        for inv in &ch.inventory {
            if !inv.equipped || inv.custom {
                continue;
            }
            if let Some(item) = lib.items.get(&inv.item) {
                if item.requires_attunement && !inv.attuned {
                    continue;
                }
                self.push_mods(&item.modifiers, &item.name);
            }
        }
        self.push_mods(&ch.modifiers, "Custom");
    }

    /// Evaluate every pending modifier whose target starts with `prefix`
    /// (or all, if empty) against the current env.
    fn resolve(&mut self, prefix: &str) {
        let level = self.ch.total_level();
        let pending = std::mem::take(&mut self.mods);
        let mut keep = Vec::new();
        for p in pending {
            let target = norm(&p.m.target);
            if !prefix.is_empty() && !target.starts_with(prefix) {
                keep.push(p);
                continue;
            }
            let mut trace = ModifierTrace {
                target: target.clone(),
                op: format!("{:?}", p.m.op).to_lowercase(),
                value: p.m.value.clone(),
                resolved: None,
                source: p.source.clone(),
                applied: false,
                reason: String::new(),
            };
            if p.m.min_level > level {
                trace.reason = format!("needs level {}", p.m.min_level);
                self.sheet.trace.push(trace);
                continue;
            }
            match expr::eval_cond(&p.m.condition, &self.env) {
                Ok(true) => {}
                Ok(false) => {
                    trace.reason = format!("condition false: {}", p.m.condition);
                    self.sheet.trace.push(trace);
                    continue;
                }
                Err(e) => {
                    trace.reason = format!("condition error: {e}");
                    self.sheet.warnings.push(format!("{}: {}", p.source, trace.reason));
                    self.sheet.trace.push(trace);
                    continue;
                }
            }
            match p.m.op {
                ModOp::Grant | ModOp::Expertise => {
                    self.resolved.entry(target.clone()).or_default().push(Resolved {
                        op: p.m.op,
                        value: 0.0,
                        source: p.source.clone(),
                        text: String::new(),
                    });
                    trace.applied = true;
                }
                ModOp::Advantage | ModOp::Disadvantage | ModOp::Note => {
                    let label = match p.m.op {
                        ModOp::Advantage => "advantage",
                        ModOp::Disadvantage => "disadvantage",
                        _ => "note",
                    };
                    let text = if p.m.note.is_empty() && p.m.value.is_empty() {
                        format!("{label} ({})", p.source)
                    } else if p.m.op == ModOp::Note {
                        format!("{} ({})", if p.m.note.is_empty() { &p.m.value } else { &p.m.note }, p.source)
                    } else {
                        format!("{label}: {} ({})", if p.m.note.is_empty() { &p.m.value } else { &p.m.note }, p.source)
                    };
                    self.sheet.notes.entry(target.clone()).or_default().push(text);
                    trace.applied = true;
                }
                _ => {
                    let value_src = if p.m.value.trim().is_empty() { "0" } else { p.m.value.as_str() };
                    match expr::eval(value_src, &self.env, Mode::Average, None) {
                        Ok(r) => {
                            trace.resolved = Some(r.value);
                            trace.applied = true;
                            let text = if r.rolls.is_empty() { String::new() } else { value_src.to_string() };
                            self.resolved.entry(target.clone()).or_default().push(Resolved {
                                op: p.m.op,
                                value: r.value,
                                source: p.source.clone(),
                                text,
                            });
                        }
                        Err(e) => {
                            trace.reason = format!("value error: {e}");
                            self.sheet.warnings.push(format!("{} -> {}: {}", p.source, target, trace.reason));
                        }
                    }
                }
            }
            if !p.m.note.is_empty() && matches!(p.m.op, ModOp::Add | ModOp::Set | ModOp::Max | ModOp::Min | ModOp::Mul) {
                self.sheet.notes.entry(target.clone()).or_default().push(format!("{} ({})", p.m.note, p.source));
            }
            self.sheet.trace.push(trace);
        }
        self.mods = keep;
    }

    fn run(mut self) -> Sheet {
        let ch = self.ch;
        let lib = self.lib;
        let rules = &lib.rules;
        let level = ch.total_level();

        // ---- identity
        self.sheet.character_id = ch.id.clone();
        self.sheet.name = ch.name.clone();
        self.sheet.player = ch.player.clone();
        self.sheet.alignment = ch.alignment.clone();
        self.sheet.level = level;
        self.sheet.xp = ch.xp;
        self.sheet.next_level_xp = rules.xp_thresholds.get(level as usize).copied();
        self.sheet.species = lib.species.get(&ch.species).map(|s| s.name.clone()).unwrap_or_else(|| ch.species.clone());
        self.sheet.background =
            lib.backgrounds.get(&ch.background).map(|s| s.name.clone()).unwrap_or_else(|| ch.background.clone());
        self.sheet.class_line = ch
            .classes
            .iter()
            .map(|c| {
                let cname = lib.classes.get(&c.class).map(|k| k.name.clone()).unwrap_or_else(|| c.class.clone());
                match c.subclass.as_ref().and_then(|s| lib.subclasses.get(s)) {
                    Some(sub) => format!("{} ({}) {}", cname, sub.name, c.level),
                    None => format!("{} {}", cname, c.level),
                }
            })
            .collect::<Vec<_>>()
            .join(" / ");
        self.sheet.size = lib.species.get(&ch.species).map(|s| s.size.clone()).unwrap_or_else(|| "Medium".into());

        // ---- base env
        self.env.set("level", level as f64);
        self.env.set("xp", ch.xp as f64);
        for cl in &ch.classes {
            self.env.set(format!("class.{}.level", cl.class), ch.class_level(&cl.class) as f64);
        }
        for k in ABILITY_KEYS {
            let s = ch.base_abilities.get(k).unwrap_or(10);
            self.env.set(k, s as f64);
            self.env.set(format!("{k}_mod"), ability_mod(s) as f64);
        }
        let pb_base = rules.proficiency_bonus(level);
        self.env.set("pb", pb_base as f64);
        self.env.set("proficiency_bonus", pb_base as f64);

        self.gather();

        // ---- abilities (first pass: only ability.* modifiers, conditions may use base scores)
        self.resolve("ability.");
        let mut abilities = ch.base_abilities;
        for k in ABILITY_KEYS {
            let v = self.num(&format!("ability.{k}"), ch.base_abilities.get(k).unwrap_or(10));
            abilities.set(k, v);
            self.env.set(k, v as f64);
            self.env.set(format!("{k}_mod"), ability_mod(v) as f64);
            self.sheet.ability_mods.insert(k.to_string(), ability_mod(v));
        }
        self.sheet.abilities = abilities;

        // ---- proficiency bonus
        self.resolve("proficiency_bonus");
        let pb = self.num("proficiency_bonus", pb_base);
        self.env.set("pb", pb as f64);
        self.env.set("proficiency_bonus", pb as f64);
        self.sheet.proficiency_bonus = pb;

        // ---- equipment facts, so conditions can say `armored`, `shield`, `armor_heavy`
        let mut armored = 0.0;
        let mut has_shield = 0.0;
        let mut armor_cat = String::new();
        for inv in ch.inventory.iter().filter(|i| i.equipped && !i.custom) {
            if let Some(item) = lib.items.get(&inv.item) {
                if let Some(a) = &item.armor {
                    if a.category == "shield" || item.kind == "shield" {
                        has_shield = 1.0;
                    } else {
                        armored = 1.0;
                        armor_cat = norm(&a.category);
                    }
                }
            }
        }
        self.env.set("armored", armored);
        self.env.set("shield", has_shield);
        for cat in ["light", "medium", "heavy"] {
            self.env.set(format!("armor_{cat}"), if armor_cat == cat { 1.0 } else { 0.0 });
        }

        // ---- everything else
        self.resolve("");

        // ---- proficiencies
        let first_class = ch.classes.first().and_then(|c| lib.classes.get(&c.class));
        let mut save_prof: BTreeSet<String> = first_class.map(|c| c.saving_throws.iter().map(|s| norm(s)).collect()).unwrap_or_default();
        save_prof.extend(self.granted("proficiency.save."));
        let mut skill_prof: BTreeSet<String> = ch.skill_proficiencies.iter().map(|s| norm(s)).collect();
        if let Some(bg) = lib.backgrounds.get(&ch.background) {
            skill_prof.extend(bg.skill_proficiencies.iter().map(|s| norm(s)));
        }
        skill_prof.extend(self.granted("proficiency.skill."));
        let mut expertise: BTreeSet<String> = ch.expertise.iter().map(|s| norm(s)).collect();
        for s in rules.skills.iter() {
            if self.has_expertise(&format!("skill.{}", s.key)) {
                expertise.insert(s.key.clone());
            }
        }
        let mut armor: BTreeSet<String> = BTreeSet::new();
        let mut weapons: BTreeSet<String> = BTreeSet::new();
        let mut tools: BTreeSet<String> = ch.tool_proficiencies.iter().map(|s| norm(s)).collect();
        let mut languages: BTreeSet<String> = ch.languages.iter().cloned().collect();
        for cl in &ch.classes {
            if let Some(c) = lib.classes.get(&cl.class) {
                armor.extend(c.armor_proficiencies.iter().map(|s| norm(s)));
                weapons.extend(c.weapon_proficiencies.iter().map(|s| norm(s)));
                tools.extend(c.tool_proficiencies.iter().map(|s| norm(s)));
            }
        }
        if let Some(bg) = lib.backgrounds.get(&ch.background) {
            tools.extend(bg.tool_proficiencies.iter().map(|s| norm(s)));
            languages.extend(bg.languages.iter().cloned());
        }
        if let Some(sp) = lib.species.get(&ch.species) {
            languages.extend(sp.languages.iter().cloned());
        }
        armor.extend(self.granted("proficiency.armor."));
        weapons.extend(self.granted("proficiency.weapon."));
        tools.extend(self.granted("proficiency.tool."));
        languages.extend(self.granted("proficiency.language.").into_iter().map(|l| titlecase(&l)));
        self.sheet.proficiencies = Proficiencies {
            armor: armor.iter().cloned().collect(),
            weapons: weapons.iter().cloned().collect(),
            tools: tools.iter().cloned().collect(),
            languages: languages.iter().cloned().collect(),
        };

        // ---- saves
        let save_all = self.num("save.all", 0);
        for k in ABILITY_KEYS {
            let proficient = save_prof.contains(k);
            let base = abilities.modifier(k) + if proficient { pb } else { 0 } + save_all;
            let bonus = self.num(&format!("save.{k}"), base);
            let mut notes = self.notes_for(&format!("save.{k}"));
            notes.extend(self.notes_for("save.all"));
            self.sheet.saves.push(SaveLine { key: k.to_string(), name: ability_name(k).to_string(), bonus, proficient, notes });
            self.env.set(format!("save.{k}"), bonus as f64);
        }

        // ---- skills
        let skill_all = self.num("skill.all", 0);
        for s in &rules.skills {
            let proficient = skill_prof.contains(&s.key);
            let exp = expertise.contains(&s.key);
            let prof_bonus = if exp { pb * 2 } else if proficient { pb } else { 0 };
            let base = abilities.modifier(&s.ability) + prof_bonus + skill_all;
            let bonus = self.num(&format!("skill.{}", s.key), base);
            let mut notes = self.notes_for(&format!("skill.{}", s.key));
            notes.extend(self.notes_for("skill.all"));
            self.sheet.skills.push(SkillLine {
                key: s.key.clone(),
                name: s.name.clone(),
                ability: s.ability.clone(),
                bonus,
                proficient: proficient || exp,
                expertise: exp,
                notes,
            });
            self.env.set(format!("skill.{}", s.key), bonus as f64);
        }
        let skill_bonus = |sheet: &Sheet, key: &str| sheet.skills.iter().find(|s| s.key == key).map(|s| s.bonus).unwrap_or(0);
        self.sheet.passive_perception = self.num("passive.perception", 10 + skill_bonus(&self.sheet, "perception"));
        self.sheet.passive_investigation = self.num("passive.investigation", 10 + skill_bonus(&self.sheet, "investigation"));
        self.sheet.passive_insight = self.num("passive.insight", 10 + skill_bonus(&self.sheet, "insight"));

        // ---- armor class
        let dex_mod = abilities.modifier("dex");
        let mut candidates: Vec<(String, i32)> = vec![("Unarmored (10 + Dex)".into(), 10 + dex_mod)];
        let mut shield = 0;
        let mut shield_name = String::new();
        let mut heavy_penalty = false;
        for inv in ch.inventory.iter().filter(|i| i.equipped && !i.custom) {
            if let Some(item) = lib.items.get(&inv.item) {
                if let Some(a) = &item.armor {
                    if a.category == "shield" || item.kind == "shield" {
                        shield += a.base_ac + a.bonus;
                        shield_name = item.name.clone();
                    } else {
                        let dex_part = match a.dex_cap {
                            Some(cap) => dex_mod.min(cap),
                            None => dex_mod,
                        };
                        candidates.push((item.name.clone(), a.base_ac + dex_part + a.bonus));
                        if a.strength_requirement > 0 && abilities.str < a.strength_requirement {
                            heavy_penalty = true;
                        }
                        if a.stealth_disadvantage {
                            self.sheet
                                .notes
                                .entry("skill.stealth".into())
                                .or_default()
                                .push(format!("disadvantage ({})", item.name));
                            if let Some(s) = self.sheet.skills.iter_mut().find(|s| s.key == "stealth") {
                                s.notes.push(format!("disadvantage ({})", item.name));
                            }
                        }
                    }
                }
            }
        }
        if let Some(list) = self.resolved.get("ac.base") {
            for r in list {
                candidates.push((r.source.clone(), r.value.round() as i32));
            }
        }
        let (best_name, best) = candidates.iter().max_by_key(|(_, v)| *v).cloned().unwrap();
        let ac = self.num("ac", best + shield);
        let mut breakdown = format!("{best_name} {best}");
        if shield != 0 {
            breakdown.push_str(&format!(" + {shield_name} {shield}"));
        }
        if let Some(list) = self.resolved.get("ac") {
            for r in list {
                breakdown.push_str(&format!(" {} {} ({})", op_sym(r.op), r.value, r.source));
            }
        }
        self.sheet.ac = ac;
        self.sheet.ac_breakdown = breakdown;
        self.env.set("ac", ac as f64);

        // ---- hit points & hit dice
        let con_mod = abilities.modifier("con");
        let mut hp = 0i32;
        let mut first = true;
        for cl in &ch.classes {
            let die = lib.classes.get(&cl.class).map(|c| c.hit_die).unwrap_or(8) as i32;
            *self.sheet.hit_dice.entry(format!("d{die}")).or_default() += cl.level;
            for lvl in 0..cl.level as usize {
                let roll = if first {
                    first = false;
                    die
                } else if let Some(r) = cl.hp_rolls.get(lvl) {
                    *r as i32
                } else if rules.hp_mode == "roll" && !cl.hp_rolls.is_empty() {
                    die / 2 + 1
                } else {
                    die / 2 + 1
                };
                hp += (roll + con_mod).max(1);
            }
        }
        let per_level = self.num("hp.per_level", 0);
        hp += per_level * level as i32;
        hp = self.num("hp.max", hp);
        if let Some(o) = ch.hp.max_override {
            hp = o;
        }
        self.sheet.hp_max = hp.max(1);
        self.sheet.hp_current = ch.hp.current.min(self.sheet.hp_max);
        self.sheet.hp_temp = ch.hp.temp;
        self.sheet.hit_dice_used = ch.hp.hit_dice_used.clone();
        self.env.set("hp.max", hp as f64);

        // ---- initiative, speed, senses
        self.sheet.initiative = self.num("initiative", dex_mod);
        let species = lib.species.get(&ch.species);
        let base_speed = species.map(|s| s.speed as i32).unwrap_or(30);
        let mut speed = self.num("speed", base_speed);
        if heavy_penalty {
            speed -= 10;
            self.sheet.notes.entry("speed".into()).or_default().push("-10 ft: armor Strength requirement not met".into());
        }
        self.sheet.speed = speed.max(0);
        self.sheet.speeds.insert("walk".into(), self.sheet.speed);
        if let Some(sp) = species {
            for (mode, v) in &sp.speeds {
                let val = self.num(&format!("speed.{}", norm(mode)), *v as i32);
                self.sheet.speeds.insert(mode.clone(), val);
            }
        }
        for (k, list) in &self.resolved {
            if let Some(mode) = k.strip_prefix("speed.") {
                if !self.sheet.speeds.contains_key(mode) {
                    self.sheet.speeds.insert(mode.to_string(), apply_ops(0.0, list).round() as i32);
                }
            }
        }
        self.sheet.darkvision = self.num("darkvision", species.map(|s| s.darkvision as i32).unwrap_or(0));

        // ---- spellcasting
        self.spellcasting(pb);

        // ---- attacks
        self.attacks(pb, &weapons);

        // ---- features
        let feats = std::mem::take(&mut self.features);
        for (source, f) in feats {
            let uses = if f.uses.trim().is_empty() {
                None
            } else {
                match expr::eval_int(&f.uses, &self.env) {
                    Ok(v) => Some(v),
                    Err(e) => {
                        self.sheet.warnings.push(format!("{}: uses formula: {e}", f.name));
                        None
                    }
                }
            };
            let (picks, chosen) = if f.options.is_empty() {
                (0, Vec::new())
            } else {
                let n = if f.choose.trim().is_empty() {
                    1
                } else {
                    match expr::eval_int(&f.choose, &self.env) {
                        Ok(v) => v.max(0) as u32,
                        Err(e) => {
                            self.sheet.warnings.push(format!("{}: choose formula: {e}", f.name));
                            1
                        }
                    }
                };
                let chosen = ch.choices.get(&f.name).cloned().unwrap_or_default();
                if chosen.len() as u32 > n {
                    self.sheet.warnings.push(format!("{}: {} picked, {n} allowed", f.name, chosen.len()));
                }
                (n, chosen)
            };
            self.sheet.features.push(FeatureLine {
                name: f.name.clone(),
                source,
                level: f.level,
                description: f.description.clone(),
                uses,
                recharge: f.recharge.clone(),
                options: f.options.clone(),
                picks,
                chosen,
            });
        }

        // ---- inventory
        let mut total = 0.0;
        for inv in &ch.inventory {
            let (name, weight, kind) = match lib.items.get(&inv.item) {
                Some(item) if !inv.custom => (
                    if inv.name.is_empty() { item.name.clone() } else { inv.name.clone() },
                    item.weight,
                    item.kind.clone(),
                ),
                _ => (if inv.name.is_empty() { inv.item.clone() } else { inv.name.clone() }, 0.0, "custom".into()),
            };
            total += weight * inv.qty as f64;
            self.sheet.inventory.push(InventoryLine {
                item: inv.item.clone(),
                name,
                qty: inv.qty,
                weight,
                equipped: inv.equipped,
                attuned: inv.attuned,
                kind,
                notes: inv.notes.clone(),
                hidden: inv.hidden,
                dm_notes: inv.dm_notes.clone(),
            });
        }
        let coins = ch.currency.cp + ch.currency.sp + ch.currency.ep + ch.currency.gp + ch.currency.pp;
        total += coins as f64 / 50.0;
        self.sheet.total_weight = (total * 10.0).round() / 10.0;
        self.sheet.carrying_capacity = self.num("carrying_capacity", abilities.str * 15);

        // ---- state passthrough
        self.sheet.conditions = ch.conditions.clone();
        self.sheet.exhaustion = ch.exhaustion;
        self.sheet.inspiration = ch.inspiration;
        self.sheet.death_saves = ch.death_saves.clone();
        self.sheet.currency = ch.currency.clone();
        self.sheet.spells_known = ch.spells.known.clone();
        self.sheet.spells_prepared = ch.spells.prepared.clone();

        // ---- unknown targets
        let known_prefixes = [
            "ability.", "save.", "skill.", "passive.", "ac", "hp.", "initiative", "speed", "darkvision", "attack.",
            "damage.", "spell.", "proficiency", "carrying_capacity",
        ];
        for (k, list) in &self.resolved {
            if !known_prefixes.iter().any(|p| k.starts_with(p)) {
                self.sheet.other.insert(k.clone(), apply_ops(0.0, list));
            }
        }

        self.sheet.env = self.env.vars.iter().map(|(k, v)| (k.clone(), *v)).collect();
        self.sheet
    }

    fn spellcasting(&mut self, pb: i32) {
        let ch = self.ch;
        let lib = self.lib;
        let rules = &lib.rules;
        let single_class = ch.classes.len() == 1;
        let mut caster_level = 0u32;
        let mut pact_level = 0u32;
        for cl in &ch.classes {
            let class = match lib.classes.get(&cl.class) {
                Some(c) => c,
                None => continue,
            };
            let sub = cl.subclass.as_ref().and_then(|s| lib.subclasses.get(s));
            let sc = class.spellcasting.as_ref().or(sub.and_then(|s| s.spellcasting.as_ref()));
            let sc = match sc {
                Some(s) => s,
                None => continue,
            };
            let lvl = cl.level;
            // caster level as if this were the only class: what it can learn
            let own_level = match sc.progression.as_str() {
                "full" => lvl,
                "half" if lvl >= 2 => lvl.div_ceil(2),
                "third" if lvl >= 3 => lvl.div_ceil(3),
                _ => 0,
            };
            let mut max_spell_level = rules.full_caster_slots(own_level).iter().rposition(|n| *n > 0).map(|i| i as u32 + 1).unwrap_or(0);
            match sc.progression.as_str() {
                "full" => caster_level += lvl,
                "half" => {
                    caster_level += if single_class {
                        if lvl < 2 { 0 } else { lvl.div_ceil(2) }
                    } else {
                        lvl / 2
                    }
                }
                "third" => {
                    caster_level += if single_class {
                        if lvl < 3 { 0 } else { lvl.div_ceil(3) }
                    } else {
                        lvl / 3
                    }
                }
                "pact" => {
                    pact_level += lvl;
                    let idx = (lvl.max(1) as usize - 1).min(rules.pact_slots.len().saturating_sub(1));
                    max_spell_level = rules.pact_slots.get(idx).map(|p| p.1).unwrap_or(0);
                }
                _ => {}
            }
            let abil = norm(&sc.ability);
            let amod = self.sheet.abilities.modifier(&abil);
            let attack = self.num("spell.attack", pb + amod);
            let dc = self.num("spell.dc", 8 + pb + amod);
            let prepared_max = if sc.prepared.trim().is_empty() {
                None
            } else {
                match expr::eval_int(&sc.prepared, &self.env) {
                    Ok(v) => Some(v as i32),
                    Err(e) => {
                        self.sheet.warnings.push(format!("{}: prepared formula: {e}", class.name));
                        None
                    }
                }
            };
            let idx = (lvl.max(1) - 1) as usize;
            let cantrips = sc.cantrips_known.get(idx).copied().unwrap_or(0);
            let known = sc.spells_known.get(idx).copied();
            let spell_limit = if !sc.spells_known.is_empty() {
                Some(known.unwrap_or(0))
            } else if sc.spellbook_start > 0 {
                Some(sc.spellbook_start + sc.spellbook_per_level * (lvl.max(1) - 1))
            } else {
                None
            };
            self.sheet.spellcasting.push(SpellcastingBlock {
                class: class.id.clone(),
                class_name: class.name.clone(),
                ability: abil,
                attack_bonus: attack,
                save_dc: dc,
                prepared_max,
                cantrips_known: cantrips,
                spells_known: known,
                progression: sc.progression.clone(),
                spell_limit,
                max_spell_level,
                ability_mod: amod,
                lists: {
                    let mut l = sc.spell_lists.clone();
                    if let Some(s) = sub {
                        l.push(s.id.clone());
                        if let Some(ssc) = &s.spellcasting {
                            l.extend(ssc.spell_lists.iter().cloned());
                        }
                    }
                    l
                },
            });
        }
        self.sheet.spell_slots = rules.full_caster_slots(caster_level);
        let mut used = ch.spells.slots_used.clone();
        used.resize(9, 0);
        self.sheet.spell_slots_used = used;
        if pact_level > 0 {
            let idx = (pact_level as usize - 1).min(rules.pact_slots.len().saturating_sub(1));
            self.sheet.pact_slots = rules.pact_slots.get(idx).copied();
        }
        self.env.set("caster_level", caster_level as f64);
    }

    fn attacks(&mut self, pb: i32, weapon_profs: &BTreeSet<String>) {
        let ch = self.ch;
        let lib = self.lib;
        let ab = self.sheet.abilities;
        let str_mod = ab.modifier("str");
        let dex_mod = ab.modifier("dex");
        let atk_all = self.num("attack.all", 0);
        let dmg_all = self.num("damage.all", 0);
        let atk_melee = self.num("attack.melee", 0) + atk_all;
        let atk_ranged = self.num("attack.ranged", 0) + atk_all;
        let dmg_melee = self.num("damage.melee", 0) + dmg_all;
        let dmg_ranged = self.num("damage.ranged", 0) + dmg_all;
        let dmg_text = |extra: &BTreeMap<String, Vec<Resolved>>, key: &str| -> Vec<String> {
            extra.get(key).map(|l| l.iter().filter(|r| !r.text.is_empty()).map(|r| format!("+{} ({})", r.text, r.source)).collect()).unwrap_or_default()
        };

        for inv in ch.inventory.iter().filter(|i| i.equipped && !i.custom) {
            let item = match lib.items.get(&inv.item) {
                Some(i) => i,
                None => continue,
            };
            let w = match &item.weapon {
                Some(w) => w,
                None => continue,
            };
            let props: Vec<String> = w.properties.iter().map(|p| norm(p)).collect();
            let ranged = w.kind == "ranged";
            let finesse = props.iter().any(|p| p == "finesse");
            let thrown = props.iter().any(|p| p == "thrown");
            let abil_mod = if ranged && !thrown {
                dex_mod
            } else if finesse {
                str_mod.max(dex_mod)
            } else {
                str_mod
            };
            let proficient = weapon_profs.contains(&norm(&w.category))
                || weapon_profs.contains(&format!("{}_weapons", norm(&w.category)))
                || weapon_profs.contains(&norm(&item.id))
                || weapon_profs.contains(&norm(&item.name))
                || self.has_grant(&format!("proficiency.weapon.{}", norm(&item.id)))
                || self.has_grant(&format!("proficiency.weapon.{}", norm(&w.category)));
            let (atk_extra, dmg_extra) = if ranged { (atk_ranged, dmg_ranged) } else { (atk_melee, dmg_melee) };
            let bonus = abil_mod + if proficient { pb } else { 0 } + w.bonus + atk_extra;
            let flat = abil_mod + w.bonus + dmg_extra;
            let mut damage = w.damage.clone();
            if flat != 0 {
                damage = format!("{damage} {} {}", if flat > 0 { "+" } else { "-" }, flat.abs());
            }
            let mut notes = Vec::new();
            if !proficient {
                notes.push("not proficient".into());
            }
            if !w.versatile.is_empty() {
                notes.push(format!("versatile {}", w.versatile));
            }
            notes.extend(dmg_text(&self.resolved, if ranged { "damage.ranged" } else { "damage.melee" }));
            notes.extend(dmg_text(&self.resolved, "damage.all"));
            notes.extend(self.notes_for(if ranged { "attack.ranged" } else { "attack.melee" }));
            notes.extend(self.notes_for("attack.all"));
            let name = if inv.name.is_empty() { item.name.clone() } else { inv.name.clone() };
            self.sheet.attacks.push(AttackLine {
                name,
                bonus,
                damage,
                damage_type: w.damage_type.clone(),
                range: if w.range.is_empty() {
                    if props.iter().any(|p| p == "reach") { "10 ft.".into() } else { "5 ft.".into() }
                } else {
                    format!("{} ft.", w.range)
                },
                properties: w.properties.clone(),
                notes,
            });
        }
        // Unarmed strike is always available.
        let unarmed_dmg = 1 + str_mod + dmg_melee;
        self.sheet.attacks.push(AttackLine {
            name: "Unarmed Strike".into(),
            bonus: str_mod + pb + atk_melee,
            damage: unarmed_dmg.max(1).to_string(),
            damage_type: "bludgeoning".into(),
            range: "5 ft.".into(),
            properties: vec![],
            notes: vec![],
        });
    }
}

fn op_sym(op: ModOp) -> &'static str {
    match op {
        ModOp::Add => "+",
        ModOp::Set => "=",
        ModOp::Max => "max",
        ModOp::Min => "min",
        ModOp::Mul => "×",
        _ => "",
    }
}

pub fn ability_name(k: &str) -> &'static str {
    match k {
        "str" => "Strength",
        "dex" => "Dexterity",
        "con" => "Constitution",
        "int" => "Intelligence",
        "wis" => "Wisdom",
        "cha" => "Charisma",
        _ => "?",
    }
}

fn titlecase(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

impl Sheet {
    /// The sheet as the player app may see it: hidden items, DM notes and
    /// the internal trace are removed. Numbers are unchanged (a hidden magic
    /// ring still protects its wearer; the player just doesn't know why).
    pub fn player_view(&self) -> Sheet {
        let mut s = self.clone();
        s.inventory.retain(|i| !i.hidden);
        for i in &mut s.inventory {
            i.dm_notes.clear();
        }
        s.trace.clear();
        s.env.clear();
        s.warnings.clear();
        s
    }
}

/// Derive the full sheet for a character.
pub fn derive(ch: &Character, lib: &Library) -> Sheet {
    let engine = Engine {
        lib,
        ch,
        env: Env::new(),
        mods: Vec::new(),
        resolved: BTreeMap::new(),
        features: Vec::new(),
        sheet: Sheet::default(),
    };
    engine.run()
}

/// Suggested starting HP (full first hit die + Con) for a fresh character.
pub fn starting_hp(ch: &Character, lib: &Library) -> i32 {
    derive(ch, lib).hp_max
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lib() -> Library {
        let mut lib = Library::new();
        lib.insert(
            Entry::Species(Species {
                id: "dwarf".into(),
                name: "Dwarf".into(),
                speed: 25,
                darkvision: 60,
                modifiers: vec![Modifier::new("ability.con", ModOp::Add, "2")],
                traits: vec![Feature {
                    name: "Dwarven Resilience".into(),
                    description: "Advantage on saves vs poison".into(),
                    modifiers: vec![Modifier { target: "save.con".into(), op: ModOp::Advantage, note: "vs. poison".into(), ..Default::default() }],
                    ..Default::default()
                }],
                ..Default::default()
            }),
            None,
        );
        lib.insert(
            Entry::Class(Class {
                id: "fighter".into(),
                name: "Fighter".into(),
                hit_die: 10,
                saving_throws: vec!["str".into(), "con".into()],
                armor_proficiencies: vec!["heavy".into()],
                weapon_proficiencies: vec!["simple".into(), "martial".into()],
                features: vec![Feature {
                    name: "Fighting Style: Defense".into(),
                    level: 1,
                    modifiers: vec![Modifier { target: "ac".into(), op: ModOp::Add, value: "1".into(), condition: "armored".into(), ..Default::default() }],
                    ..Default::default()
                }],
                ..Default::default()
            }),
            None,
        );
        lib.insert(
            Entry::Item(Item {
                id: "chain_mail".into(),
                name: "Chain Mail".into(),
                kind: "armor".into(),
                weight: 55.0,
                armor: Some(Armor { base_ac: 16, category: "heavy".into(), dex_cap: Some(0), strength_requirement: 13, stealth_disadvantage: true, bonus: 0 }),
                ..Default::default()
            }),
            None,
        );
        lib.insert(
            Entry::Item(Item {
                id: "longsword".into(),
                name: "Longsword".into(),
                kind: "weapon".into(),
                weight: 3.0,
                weapon: Some(Weapon { damage: "1d8".into(), damage_type: "slashing".into(), category: "martial".into(), kind: "melee".into(), versatile: "1d10".into(), ..Default::default() }),
                ..Default::default()
            }),
            None,
        );
        lib.insert(
            Entry::Feat(Feat {
                id: "tough".into(),
                name: "Tough".into(),
                modifiers: vec![Modifier::new("hp.max", ModOp::Add, "2 * level")],
                ..Default::default()
            }),
            None,
        );
        lib
    }

    fn fighter() -> Character {
        let mut ch = Character::new("bruenor", "Bruenor");
        ch.species = "dwarf".into();
        ch.classes = vec![ClassLevel { class: "fighter".into(), level: 3, ..Default::default() }];
        ch.base_abilities = Abilities { str: 16, dex: 12, con: 14, int: 8, wis: 10, cha: 10 };
        ch.skill_proficiencies = vec!["athletics".into()];
        ch.inventory = vec![
            InventoryItem { item: "chain_mail".into(), equipped: true, qty: 1, ..Default::default() },
            InventoryItem { item: "longsword".into(), equipped: true, qty: 1, ..Default::default() },
        ];
        ch.feats = vec!["tough".into()];
        ch
    }

    #[test]
    fn derives_core_numbers() {
        let lib = lib();
        let s = derive(&fighter(), &lib);
        assert!(s.warnings.is_empty(), "{:?}", s.warnings);
        assert_eq!(s.abilities.con, 16); // 14 + 2 dwarf
        assert_eq!(s.proficiency_bonus, 2);
        assert_eq!(s.ac, 17); // chain mail 16 (no dex) + Defense +1 (condition true because armored)
        // HP: L1 10+3, L2 6+3, L3 6+3 = 31, + Tough 6 = 37
        assert_eq!(s.hp_max, 37);
        assert_eq!(s.speed, 25);
        let athletics = s.skills.iter().find(|k| k.key == "athletics").unwrap();
        assert_eq!(athletics.bonus, 5); // +3 str +2 pb
        assert!(athletics.proficient);
        let stealth = s.skills.iter().find(|k| k.key == "stealth").unwrap();
        assert!(stealth.notes.iter().any(|n| n.contains("disadvantage")));
        let con_save = s.saves.iter().find(|k| k.key == "con").unwrap();
        assert_eq!(con_save.bonus, 5);
        assert!(con_save.notes.iter().any(|n| n.contains("poison")));
        let sword = s.attacks.iter().find(|a| a.name == "Longsword").unwrap();
        assert_eq!(sword.bonus, 5);
        assert_eq!(sword.damage, "1d8 + 3");
        assert_eq!(s.hit_dice.get("d10"), Some(&3));
        assert_eq!(s.passive_perception, 10);
    }

    #[test]
    fn custom_modifiers_and_overrides() {
        let lib = lib();
        let mut ch = fighter();
        ch.modifiers.push(Modifier::new("ac", ModOp::Add, "2").with_source("Ring of Protection (DM)"));
        ch.modifiers.push(Modifier::new("skill.all", ModOp::Add, "1"));
        ch.modifiers.push(Modifier::new("homebrew.luck", ModOp::Add, "3"));
        ch.hp.max_override = Some(50);
        let s = derive(&ch, &lib);
        assert_eq!(s.ac, 19);
        assert_eq!(s.hp_max, 50);
        assert_eq!(s.skills.iter().find(|k| k.key == "athletics").unwrap().bonus, 6);
        assert_eq!(s.other.get("homebrew.luck"), Some(&3.0));
        assert!(s.ac_breakdown.contains("Ring of Protection"));
    }

    #[test]
    fn unknown_refs_warn_not_panic() {
        let lib = lib();
        let mut ch = fighter();
        ch.species = "dragonborn".into();
        ch.feats.push("nope".into());
        let s = derive(&ch, &lib);
        assert_eq!(s.warnings.len(), 2);
        assert_eq!(s.abilities.con, 14);
    }
}
