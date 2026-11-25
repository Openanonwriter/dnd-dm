//! Data model. Everything here is plain `serde` data so it round-trips to and
//! from human-editable TOML files (content packs, characters, saved games).

pub mod character;
pub mod content;
pub mod modifier;
pub mod rules;
pub mod state;

pub use character::*;
pub use content::*;
pub use modifier::*;
pub use rules::*;
pub use state::*;

use serde::{Deserialize, Serialize};

/// The six ability scores. Keys are always lower-case 3-letter codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Abilities {
    #[serde(default = "ten")]
    pub str: i32,
    #[serde(default = "ten")]
    pub dex: i32,
    #[serde(default = "ten")]
    pub con: i32,
    #[serde(default = "ten")]
    pub int: i32,
    #[serde(default = "ten")]
    pub wis: i32,
    #[serde(default = "ten")]
    pub cha: i32,
}

fn ten() -> i32 {
    10
}

pub const ABILITY_KEYS: [&str; 6] = ["str", "dex", "con", "int", "wis", "cha"];

impl Default for Abilities {
    fn default() -> Self {
        Self { str: 10, dex: 10, con: 10, int: 10, wis: 10, cha: 10 }
    }
}

impl Abilities {
    pub fn get(&self, key: &str) -> Option<i32> {
        Some(match key {
            "str" => self.str,
            "dex" => self.dex,
            "con" => self.con,
            "int" => self.int,
            "wis" => self.wis,
            "cha" => self.cha,
            _ => return None,
        })
    }
    pub fn get_mut(&mut self, key: &str) -> Option<&mut i32> {
        Some(match key {
            "str" => &mut self.str,
            "dex" => &mut self.dex,
            "con" => &mut self.con,
            "int" => &mut self.int,
            "wis" => &mut self.wis,
            "cha" => &mut self.cha,
            _ => return None,
        })
    }
    pub fn set(&mut self, key: &str, v: i32) {
        if let Some(slot) = self.get_mut(key) {
            *slot = v;
        }
    }
    /// Standard 5e modifier: floor((score - 10) / 2).
    pub fn modifier(&self, key: &str) -> i32 {
        self.get(key).map(ability_mod).unwrap_or(0)
    }
    pub fn iter(&self) -> impl Iterator<Item = (&'static str, i32)> + '_ {
        ABILITY_KEYS.iter().map(move |k| (*k, self.get(k).unwrap()))
    }
}

pub fn ability_mod(score: i32) -> i32 {
    ((score - 10) as f64 / 2.0).floor() as i32
}

/// A named, levelled chunk of rules text with optional attached modifiers.
/// Used for species traits, class features, background features, feats and
/// creature traits/actions alike.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Feature {
    pub name: String,
    #[serde(default = "one")]
    pub level: u32,
    #[serde(default)]
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modifiers: Vec<Modifier>,
    /// Limited-use features, e.g. `uses = "1 + floor(level / 4)"`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub uses: String,
    /// `short_rest`, `long_rest`, `dawn`, ... free text.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub recharge: String,
    /// Optional attack block for creature actions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attack: Option<Attack>,
}

fn one() -> u32 {
    1
}

/// Attack line for creature actions / weapon templates.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Attack {
    #[serde(default)]
    pub to_hit: i32,
    /// e.g. "reach 5 ft." or "range 80/320 ft."
    #[serde(default)]
    pub reach: String,
    /// Dice formula, e.g. "1d6 + 2".
    #[serde(default)]
    pub damage: String,
    #[serde(default)]
    pub damage_type: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub extra: String,
}
