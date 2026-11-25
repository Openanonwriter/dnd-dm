//! Table-driven rules that a human may edit in `content/rules.toml`:
//! skill list, proficiency bonus by level, XP thresholds, point buy costs,
//! grid rules. Defaults match the 5e SRD.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillDef {
    pub key: String,
    pub name: String,
    pub ability: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PointBuy {
    pub budget: i32,
    pub min: i32,
    pub max: i32,
    /// Score -> cost, keyed by string because TOML keys are strings.
    pub costs: BTreeMap<String, i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DistanceRule {
    /// Every square (including diagonals) is 5 ft. The 5e default.
    #[default]
    Chebyshev,
    /// Diagonals alternate 5 ft / 10 ft (DMG variant).
    Alternating,
    /// True Euclidean distance.
    Euclidean,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Rules {
    #[serde(default = "default_skills")]
    pub skills: Vec<SkillDef>,
    /// Index 0 = level 1.
    #[serde(default = "default_pb")]
    pub proficiency_bonus: Vec<i32>,
    /// XP needed to *reach* each level; index 0 = level 1 (0 XP).
    #[serde(default = "default_xp")]
    pub xp_thresholds: Vec<u32>,
    #[serde(default = "default_point_buy")]
    pub point_buy: PointBuy,
    #[serde(default = "default_standard_array")]
    pub standard_array: Vec<i32>,
    #[serde(default)]
    pub distance_rule: DistanceRule,
    #[serde(default = "five")]
    pub feet_per_square: u32,
    /// Hit points per level after 1st: "fixed" (average rounded up) or "roll".
    #[serde(default = "fixed")]
    pub hp_mode: String,
    /// Full-caster spell slots by caster level (index 0 = level 1), 9 entries each.
    #[serde(default = "default_slots")]
    pub spell_slots: Vec<Vec<u32>>,
    /// Pact magic slots: (slot count, slot level) by warlock level.
    #[serde(default = "default_pact")]
    pub pact_slots: Vec<(u32, u32)>,
    #[serde(default = "default_sizes")]
    pub sizes: BTreeMap<String, u32>,
    #[serde(default = "default_conditions")]
    pub conditions: Vec<String>,
    #[serde(default = "default_damage_types")]
    pub damage_types: Vec<String>,
}

fn five() -> u32 {
    5
}
fn fixed() -> String {
    "fixed".into()
}

impl Default for Rules {
    fn default() -> Self {
        Self {
            skills: default_skills(),
            proficiency_bonus: default_pb(),
            xp_thresholds: default_xp(),
            point_buy: default_point_buy(),
            standard_array: default_standard_array(),
            distance_rule: DistanceRule::Chebyshev,
            feet_per_square: 5,
            hp_mode: "fixed".into(),
            spell_slots: default_slots(),
            pact_slots: default_pact(),
            sizes: default_sizes(),
            conditions: default_conditions(),
            damage_types: default_damage_types(),
        }
    }
}

impl Rules {
    pub fn skill(&self, key: &str) -> Option<&SkillDef> {
        self.skills.iter().find(|s| s.key == key)
    }
    pub fn proficiency_bonus(&self, level: u32) -> i32 {
        let idx = (level.max(1) as usize - 1).min(self.proficiency_bonus.len().saturating_sub(1));
        self.proficiency_bonus.get(idx).copied().unwrap_or(2)
    }
    pub fn level_for_xp(&self, xp: u32) -> u32 {
        let mut lvl = 1;
        for (i, t) in self.xp_thresholds.iter().enumerate() {
            if xp >= *t {
                lvl = i as u32 + 1;
            }
        }
        lvl
    }
    pub fn point_buy_cost(&self, scores: &[i32]) -> Option<i32> {
        let mut total = 0;
        for s in scores {
            total += *self.point_buy.costs.get(&s.to_string())?;
        }
        Some(total)
    }
    /// Size in squares per side (Tiny/Small/Medium = 1, Large = 2, ...).
    pub fn size_squares(&self, size: &str) -> u32 {
        self.sizes.get(&size.to_lowercase()).copied().unwrap_or(1)
    }
    pub fn full_caster_slots(&self, caster_level: u32) -> Vec<u32> {
        if caster_level == 0 {
            return vec![0; 9];
        }
        let idx = (caster_level as usize - 1).min(self.spell_slots.len().saturating_sub(1));
        self.spell_slots.get(idx).cloned().unwrap_or_else(|| vec![0; 9])
    }
}

fn default_skills() -> Vec<SkillDef> {
    let list = [
        ("acrobatics", "Acrobatics", "dex"),
        ("animal_handling", "Animal Handling", "wis"),
        ("arcana", "Arcana", "int"),
        ("athletics", "Athletics", "str"),
        ("deception", "Deception", "cha"),
        ("history", "History", "int"),
        ("insight", "Insight", "wis"),
        ("intimidation", "Intimidation", "cha"),
        ("investigation", "Investigation", "int"),
        ("medicine", "Medicine", "wis"),
        ("nature", "Nature", "int"),
        ("perception", "Perception", "wis"),
        ("performance", "Performance", "cha"),
        ("persuasion", "Persuasion", "cha"),
        ("religion", "Religion", "int"),
        ("sleight_of_hand", "Sleight of Hand", "dex"),
        ("stealth", "Stealth", "dex"),
        ("survival", "Survival", "wis"),
    ];
    list.iter().map(|(k, n, a)| SkillDef { key: k.to_string(), name: n.to_string(), ability: a.to_string() }).collect()
}

fn default_pb() -> Vec<i32> {
    vec![2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 6, 6, 6, 6]
}

fn default_xp() -> Vec<u32> {
    vec![
        0, 300, 900, 2700, 6500, 14000, 23000, 34000, 48000, 64000, 85000, 100000, 120000, 140000, 165000, 195000,
        225000, 265000, 305000, 355000,
    ]
}

fn default_point_buy() -> PointBuy {
    let costs = [(8, 0), (9, 1), (10, 2), (11, 3), (12, 4), (13, 5), (14, 7), (15, 9)]
        .iter()
        .map(|(s, c)| (s.to_string(), *c))
        .collect();
    PointBuy { budget: 27, min: 8, max: 15, costs }
}

fn default_standard_array() -> Vec<i32> {
    vec![15, 14, 13, 12, 10, 8]
}

fn default_slots() -> Vec<Vec<u32>> {
    vec![
        vec![2, 0, 0, 0, 0, 0, 0, 0, 0],
        vec![3, 0, 0, 0, 0, 0, 0, 0, 0],
        vec![4, 2, 0, 0, 0, 0, 0, 0, 0],
        vec![4, 3, 0, 0, 0, 0, 0, 0, 0],
        vec![4, 3, 2, 0, 0, 0, 0, 0, 0],
        vec![4, 3, 3, 0, 0, 0, 0, 0, 0],
        vec![4, 3, 3, 1, 0, 0, 0, 0, 0],
        vec![4, 3, 3, 2, 0, 0, 0, 0, 0],
        vec![4, 3, 3, 3, 1, 0, 0, 0, 0],
        vec![4, 3, 3, 3, 2, 0, 0, 0, 0],
        vec![4, 3, 3, 3, 2, 1, 0, 0, 0],
        vec![4, 3, 3, 3, 2, 1, 0, 0, 0],
        vec![4, 3, 3, 3, 2, 1, 1, 0, 0],
        vec![4, 3, 3, 3, 2, 1, 1, 0, 0],
        vec![4, 3, 3, 3, 2, 1, 1, 1, 0],
        vec![4, 3, 3, 3, 2, 1, 1, 1, 0],
        vec![4, 3, 3, 3, 2, 1, 1, 1, 1],
        vec![4, 3, 3, 3, 3, 1, 1, 1, 1],
        vec![4, 3, 3, 3, 3, 2, 1, 1, 1],
        vec![4, 3, 3, 3, 3, 2, 2, 1, 1],
    ]
}

fn default_pact() -> Vec<(u32, u32)> {
    vec![
        (1, 1), (2, 1), (2, 2), (2, 2), (2, 3), (2, 3), (2, 4), (2, 4), (2, 5), (2, 5),
        (3, 5), (3, 5), (3, 5), (3, 5), (3, 5), (3, 5), (4, 5), (4, 5), (4, 5), (4, 5),
    ]
}

fn default_sizes() -> BTreeMap<String, u32> {
    [("tiny", 1), ("small", 1), ("medium", 1), ("large", 2), ("huge", 3), ("gargantuan", 4)]
        .iter()
        .map(|(k, v)| (k.to_string(), *v))
        .collect()
}

fn default_conditions() -> Vec<String> {
    [
        "blinded", "charmed", "deafened", "frightened", "grappled", "incapacitated", "invisible", "paralyzed",
        "petrified", "poisoned", "prone", "restrained", "stunned", "unconscious", "exhaustion", "concentrating",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

fn default_damage_types() -> Vec<String> {
    [
        "acid", "bludgeoning", "cold", "fire", "force", "lightning", "necrotic", "piercing", "poison", "psychic",
        "radiant", "slashing", "thunder",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}
