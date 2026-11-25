//! A player character as *input*: the choices a player made. Everything
//! derived (modifiers, AC, saves...) is computed by `sheet::derive` and never
//! stored, so editing this file by hand can't leave stale numbers behind.

use super::{Abilities, Modifier};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ClassLevel {
    pub class: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subclass: Option<String>,
    #[serde(default = "one")]
    pub level: u32,
    /// Hit-point rolls per level (used when `hp_mode = "roll"`). Index 0 = this class's 1st level.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hp_rolls: Vec<u32>,
}

fn one() -> u32 {
    1
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct InventoryItem {
    /// Item id from the content library, or free text if `custom = true`.
    pub item: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(default = "one")]
    pub qty: u32,
    #[serde(default)]
    pub equipped: bool,
    #[serde(default)]
    pub attuned: bool,
    #[serde(default)]
    pub custom: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
    /// Fog of war for loot: the player app does not see this item at all.
    #[serde(default, skip_serializing_if = "is_false")]
    pub hidden: bool,
    /// Never sent to players (true nature of an unidentified item, curses...).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub dm_notes: String,
}

fn is_false(b: &bool) -> bool {
    !*b
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct HitPoints {
    #[serde(default)]
    pub current: i32,
    #[serde(default)]
    pub temp: i32,
    /// If set, replaces the computed maximum (DM override).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_override: Option<i32>,
    /// Hit dice spent since the last long rest, by die size ("d8" -> 2).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub hit_dice_used: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct DeathSaves {
    #[serde(default)]
    pub successes: u32,
    #[serde(default)]
    pub failures: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Currency {
    #[serde(default)]
    pub cp: u32,
    #[serde(default)]
    pub sp: u32,
    #[serde(default)]
    pub ep: u32,
    #[serde(default)]
    pub gp: u32,
    #[serde(default)]
    pub pp: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SpellState {
    /// Known / in spellbook.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub known: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prepared: Vec<String>,
    /// Slots expended, index 0 = 1st level.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub slots_used: Vec<u32>,
    #[serde(default)]
    pub pact_slots_used: u32,
}

/// Ability score increase or feat taken at an ASI level.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Advancement {
    pub level: u32,
    /// e.g. { str = 2 } or { str = 1, con = 1 }
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub ability_increases: BTreeMap<String, i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feat: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Character {
    /// Stable id (file name). Lower-case, no spaces. Defaults to a slug of the name.
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub player: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub species: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub background: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub classes: Vec<ClassLevel>,
    #[serde(default)]
    pub base_abilities: Abilities,
    /// point_buy, standard_array, rolled, manual
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub ability_method: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub advancements: Vec<Advancement>,
    /// Skill keys chosen from class / background options.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skill_proficiencies: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expertise: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_proficiencies: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub languages: Vec<String>,
    /// Extra feats not tied to an advancement (e.g. variant human).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub feats: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inventory: Vec<InventoryItem>,
    #[serde(default)]
    pub hp: HitPoints,
    #[serde(default)]
    pub death_saves: DeathSaves,
    #[serde(default)]
    pub spells: SpellState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<String>,
    #[serde(default)]
    pub exhaustion: u32,
    #[serde(default)]
    pub inspiration: bool,
    /// Human-assigned modifiers: "+1 to AC from the DM", house rules, blessings.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modifiers: Vec<Modifier>,
    #[serde(default)]
    pub currency: Currency,
    #[serde(default)]
    pub xp: u32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub alignment: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub appearance: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub personality: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub ideals: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub bonds: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub flaws: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub backstory: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
    /// DM-only notes, stripped from player views.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub dm_notes: String,
    /// Anything else a table wants to track.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub custom: BTreeMap<String, String>,
}

impl Character {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self { id: id.into(), name: name.into(), ..Default::default() }
    }
    pub fn total_level(&self) -> u32 {
        self.classes.iter().map(|c| c.level).sum::<u32>().max(1)
    }
    pub fn class_level(&self, class_id: &str) -> u32 {
        self.classes.iter().filter(|c| c.class == class_id).map(|c| c.level).sum()
    }
    pub fn all_feats(&self) -> Vec<String> {
        let mut v = self.feats.clone();
        for a in &self.advancements {
            if let Some(f) = &a.feat {
                v.push(f.clone());
            }
        }
        v
    }
    /// Copy with everything the player must not see removed.
    pub fn player_view(&self) -> Character {
        let mut c = self.clone();
        c.dm_notes.clear();
        c.inventory.retain(|i| !i.hidden);
        for i in &mut c.inventory {
            i.dm_notes.clear();
        }
        c
    }
    /// Make a filesystem-safe id from a display name.
    pub fn slug(name: &str) -> String {
        let mut s: String = name
            .trim()
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect();
        while s.contains("__") {
            s = s.replace("__", "_");
        }
        s.trim_matches('_').to_string()
    }
}
