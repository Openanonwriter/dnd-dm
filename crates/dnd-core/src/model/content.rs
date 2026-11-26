//! Content-pack entries: species, classes, subclasses, backgrounds, items,
//! feats, spells and creatures. Each lives in its own TOML file.

use super::{Abilities, Attack, Feature, Modifier};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Which content table a file belongs to. Determined from the folder it is
/// in (`content/species/*.toml`) or from an explicit `kind = "..."` key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Species,
    Class,
    Subclass,
    Background,
    Item,
    Feat,
    Spell,
    Creature,
}

impl Kind {
    pub const ALL: [Kind; 8] =
        [Kind::Species, Kind::Class, Kind::Subclass, Kind::Background, Kind::Item, Kind::Feat, Kind::Spell, Kind::Creature];

    pub fn folder(self) -> &'static str {
        match self {
            Kind::Species => "species",
            Kind::Class => "classes",
            Kind::Subclass => "subclasses",
            Kind::Background => "backgrounds",
            Kind::Item => "items",
            Kind::Feat => "feats",
            Kind::Spell => "spells",
            Kind::Creature => "creatures",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Kind::Species => "Species",
            Kind::Class => "Classes",
            Kind::Subclass => "Subclasses",
            Kind::Background => "Backgrounds",
            Kind::Item => "Items",
            Kind::Feat => "Feats",
            Kind::Spell => "Spells",
            Kind::Creature => "Creatures",
        }
    }
    pub fn from_folder(name: &str) -> Option<Kind> {
        let n = name.to_lowercase();
        Kind::ALL.iter().copied().find(|k| k.folder() == n || n == format!("{:?}", k).to_lowercase())
    }
}

// ------------------------------------------------------------------ species

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Species {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "medium")]
    pub size: String,
    #[serde(default = "thirty")]
    pub speed: u32,
    /// Extra movement modes, e.g. `fly = 30`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub speeds: BTreeMap<String, u32>,
    #[serde(default)]
    pub darkvision: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub languages: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub traits: Vec<Feature>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modifiers: Vec<Modifier>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Hidden from player apps (DM-only content).
    #[serde(default, skip_serializing_if = "is_false")]
    pub hidden: bool,
}

fn medium() -> String {
    "Medium".into()
}
fn thirty() -> u32 {
    30
}

// ------------------------------------------------------------------ classes

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SkillChoice {
    #[serde(default)]
    pub count: u32,
    /// Skill keys to choose from; empty means "any".
    #[serde(default)]
    pub from: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Spellcasting {
    /// Spellcasting ability key: int / wis / cha.
    pub ability: String,
    /// `full`, `half`, `third`, `pact` (warlock), or `none`.
    #[serde(default = "full")]
    pub progression: String,
    /// Formula for prepared spells, e.g. "max(1, wis_mod + class.cleric.level)".
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub prepared: String,
    /// Cantrips known by class level (index 0 = level 1). Missing = 0.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cantrips_known: Vec<u32>,
    /// Spells known by class level for "known" casters (bard, sorcerer...).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub spells_known: Vec<u32>,
    #[serde(default)]
    pub ritual_casting: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub focus: String,
    /// Which spell list ids this class may pick from (defaults to its own id).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub spell_lists: Vec<String>,
    /// Spellbook casters (wizard): spells in the book at 1st level, and how
    /// many more are added at each level after. 0 = no spellbook.
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub spellbook_start: u32,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub spellbook_per_level: u32,
}

fn is_zero_u32(v: &u32) -> bool {
    *v == 0
}

fn full() -> String {
    "full".into()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Class {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "eight")]
    pub hit_die: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub primary_abilities: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub saving_throws: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub armor_proficiencies: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub weapon_proficiencies: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_proficiencies: Vec<String>,
    #[serde(default)]
    pub skill_choices: SkillChoice,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub starting_equipment: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<Feature>,
    #[serde(default = "three")]
    pub subclass_level: u32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub subclass_label: String,
    #[serde(default = "default_asi")]
    pub asi_levels: Vec<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spellcasting: Option<Spellcasting>,
    /// Multiclass prerequisite note, free text.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub multiclass_prerequisite: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modifiers: Vec<Modifier>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Hidden from player apps (DM-only content).
    #[serde(default, skip_serializing_if = "is_false")]
    pub hidden: bool,
}

fn eight() -> u32 {
    8
}
fn three() -> u32 {
    3
}
fn default_asi() -> Vec<u32> {
    vec![4, 8, 12, 16, 19]
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Subclass {
    pub id: String,
    pub name: String,
    /// Parent class id.
    pub class: String,
    #[serde(default)]
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<Feature>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spellcasting: Option<Spellcasting>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modifiers: Vec<Modifier>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Hidden from player apps (DM-only content).
    #[serde(default, skip_serializing_if = "is_false")]
    pub hidden: bool,
}

// ------------------------------------------------------------------ backgrounds

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Background {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skill_proficiencies: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_proficiencies: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub languages: Vec<String>,
    #[serde(default)]
    pub language_choices: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feature: Option<Feature>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub starting_equipment: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modifiers: Vec<Modifier>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Hidden from player apps (DM-only content).
    #[serde(default, skip_serializing_if = "is_false")]
    pub hidden: bool,
}

// ------------------------------------------------------------------ items

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Weapon {
    /// e.g. "1d8"
    pub damage: String,
    #[serde(default)]
    pub damage_type: String,
    /// `simple` or `martial`
    #[serde(default = "simple")]
    pub category: String,
    /// `melee` or `ranged`
    #[serde(default = "melee")]
    pub kind: String,
    /// finesse, light, heavy, two_handed, versatile, thrown, reach, loading, ammunition...
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub properties: Vec<String>,
    /// Two-handed damage for versatile weapons, e.g. "1d10".
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub versatile: String,
    /// "80/320" for ranged/thrown.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub range: String,
    /// Magic bonus to attack and damage.
    #[serde(default)]
    pub bonus: i32,
}

fn simple() -> String {
    "simple".into()
}
fn melee() -> String {
    "melee".into()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Armor {
    pub base_ac: i32,
    /// `light`, `medium`, `heavy`, `shield`
    #[serde(default)]
    pub category: String,
    /// Max Dex bonus added. Omit for unlimited (light). 2 for medium, 0 for heavy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dex_cap: Option<i32>,
    #[serde(default)]
    pub strength_requirement: i32,
    #[serde(default)]
    pub stealth_disadvantage: bool,
    #[serde(default)]
    pub bonus: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Item {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// weapon, armor, shield, gear, consumable, wondrous, tool, ammunition...
    #[serde(default = "gear")]
    pub kind: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub cost: String,
    #[serde(default)]
    pub weight: f64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub rarity: String,
    #[serde(default)]
    pub requires_attunement: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weapon: Option<Weapon>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub armor: Option<Armor>,
    /// Modifiers applied while the item is equipped (and attuned, if required).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modifiers: Vec<Modifier>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Hidden from player apps (DM-only content).
    #[serde(default, skip_serializing_if = "is_false")]
    pub hidden: bool,
}

fn gear() -> String {
    "gear".into()
}

// ------------------------------------------------------------------ feats

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Feat {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub prerequisite: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modifiers: Vec<Modifier>,
    /// Spell ids granted while the character has this feat or option
    /// (always prepared, not counted against limits).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub spells: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Hidden from player apps (DM-only content).
    #[serde(default, skip_serializing_if = "is_false")]
    pub hidden: bool,
}

// ------------------------------------------------------------------ spells

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Area {
    /// sphere, cube, cone, line, cylinder
    pub shape: String,
    /// Radius / side / length in feet.
    pub size: u32,
    /// Width for lines (default 5).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Spell {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub level: u32,
    #[serde(default)]
    pub school: String,
    #[serde(default)]
    pub casting_time: String,
    #[serde(default)]
    pub range: String,
    #[serde(default)]
    pub components: String,
    #[serde(default)]
    pub duration: String,
    #[serde(default)]
    pub concentration: bool,
    #[serde(default)]
    pub ritual: bool,
    #[serde(default)]
    pub description: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub higher_levels: String,
    /// Class ids whose list contains this spell.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub classes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area: Option<Area>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub damage: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub damage_type: String,
    /// Saving throw ability, if any.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub save: String,
    #[serde(default)]
    pub attack: bool,
    /// With `save`: a successful save halves the damage instead of avoiding it.
    #[serde(default, skip_serializing_if = "is_false")]
    pub save_half: bool,
    /// Extra damage or healing per slot level above the spell's own, e.g. "1d6".
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub upcast: String,
    /// Healing formula; `mod` is the caster's spellcasting modifier.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub heal: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Hidden from player apps (DM-only content).
    #[serde(default, skip_serializing_if = "is_false")]
    pub hidden: bool,
}

// ------------------------------------------------------------------ creatures

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Creature {
    pub id: String,
    pub name: String,
    #[serde(default = "medium")]
    pub size: String,
    /// humanoid, beast, undead...
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub alignment: String,
    #[serde(default = "ten_i")]
    pub ac: i32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub ac_note: String,
    #[serde(default)]
    pub hp: i32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub hp_formula: String,
    #[serde(default = "walk30")]
    pub speed: BTreeMap<String, u32>,
    #[serde(default)]
    pub abilities: Abilities,
    /// Explicit save bonuses (only proficient ones need listing).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub saves: BTreeMap<String, i32>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub skills: BTreeMap<String, i32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub damage_vulnerabilities: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub damage_resistances: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub damage_immunities: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub condition_immunities: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub senses: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub languages: Vec<String>,
    /// "1/4", "1/2", "3" ...
    #[serde(default)]
    pub cr: String,
    #[serde(default)]
    pub xp: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub traits: Vec<Feature>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<Feature>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bonus_actions: Vec<Feature>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reactions: Vec<Feature>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub legendary_actions: Vec<Feature>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub spells: Vec<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Hidden from player apps (DM-only content).
    #[serde(default, skip_serializing_if = "is_false")]
    pub hidden: bool,
}

fn ten_i() -> i32 {
    10
}
fn walk30() -> BTreeMap<String, u32> {
    let mut m = BTreeMap::new();
    m.insert("walk".to_string(), 30);
    m
}

impl Creature {
    pub fn proficiency_bonus(&self) -> i32 {
        let cr = parse_cr(&self.cr);
        match cr {
            c if c < 5.0 => 2,
            c if c < 9.0 => 3,
            c if c < 13.0 => 4,
            c if c < 17.0 => 5,
            c if c < 21.0 => 6,
            c if c < 25.0 => 7,
            c if c < 29.0 => 8,
            _ => 9,
        }
    }
    pub fn save_bonus(&self, ability: &str) -> i32 {
        self.saves.get(ability).copied().unwrap_or_else(|| self.abilities.modifier(ability))
    }
    pub fn first_attack(&self) -> Option<(&Feature, &Attack)> {
        self.actions.iter().find_map(|a| a.attack.as_ref().map(|atk| (a, atk)))
    }
}

/// "1/4" -> 0.25, "3" -> 3.0
pub fn parse_cr(s: &str) -> f64 {
    let s = s.trim();
    if let Some((a, b)) = s.split_once('/') {
        let a: f64 = a.trim().parse().unwrap_or(0.0);
        let b: f64 = b.trim().parse().unwrap_or(1.0);
        if b != 0.0 {
            return a / b;
        }
    }
    s.parse().unwrap_or(0.0)
}

/// Any content entry, for generic listing and JSON API output.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Entry {
    Species(Species),
    Class(Class),
    Subclass(Subclass),
    Background(Background),
    Item(Item),
    Feat(Feat),
    Spell(Spell),
    Creature(Creature),
}

impl Entry {
    pub fn kind(&self) -> Kind {
        match self {
            Entry::Species(_) => Kind::Species,
            Entry::Class(_) => Kind::Class,
            Entry::Subclass(_) => Kind::Subclass,
            Entry::Background(_) => Kind::Background,
            Entry::Item(_) => Kind::Item,
            Entry::Feat(_) => Kind::Feat,
            Entry::Spell(_) => Kind::Spell,
            Entry::Creature(_) => Kind::Creature,
        }
    }
    pub fn id(&self) -> &str {
        match self {
            Entry::Species(x) => &x.id,
            Entry::Class(x) => &x.id,
            Entry::Subclass(x) => &x.id,
            Entry::Background(x) => &x.id,
            Entry::Item(x) => &x.id,
            Entry::Feat(x) => &x.id,
            Entry::Spell(x) => &x.id,
            Entry::Creature(x) => &x.id,
        }
    }
    pub fn name(&self) -> &str {
        match self {
            Entry::Species(x) => &x.name,
            Entry::Class(x) => &x.name,
            Entry::Subclass(x) => &x.name,
            Entry::Background(x) => &x.name,
            Entry::Item(x) => &x.name,
            Entry::Feat(x) => &x.name,
            Entry::Spell(x) => &x.name,
            Entry::Creature(x) => &x.name,
        }
    }
    pub fn hidden(&self) -> bool {
        match self {
            Entry::Species(x) => x.hidden,
            Entry::Class(x) => x.hidden,
            Entry::Subclass(x) => x.hidden,
            Entry::Background(x) => x.hidden,
            Entry::Item(x) => x.hidden,
            Entry::Feat(x) => x.hidden,
            Entry::Spell(x) => x.hidden,
            Entry::Creature(x) => x.hidden,
        }
    }
    pub fn description(&self) -> &str {
        match self {
            Entry::Species(x) => &x.description,
            Entry::Class(x) => &x.description,
            Entry::Subclass(x) => &x.description,
            Entry::Background(x) => &x.description,
            Entry::Item(x) => &x.description,
            Entry::Feat(x) => &x.description,
            Entry::Spell(x) => &x.description,
            Entry::Creature(x) => &x.description,
        }
    }
}

fn is_false(b: &bool) -> bool {
    !*b
}
