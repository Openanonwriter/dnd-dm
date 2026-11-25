//! Modifiers are the one mechanism by which content changes numbers on a sheet.
//!
//! A modifier names a **target** (a dotted path such as `ability.str`,
//! `skill.perception`, `save.wis`, `ac`, `hp.max`, `speed`, `initiative`,
//! `attack.melee`, `damage.ranged`, `spell.dc`, `proficiency.skill.stealth`),
//! an **op**, and a **value** formula evaluated against the character
//! (see `expr`). Any target string is accepted; unknown targets are collected
//! on the sheet under "other" so homebrew can invent new ones.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModOp {
    /// Add `value` to the target.
    #[default]
    Add,
    /// Force the target to `value` (last one wins; later sources override).
    Set,
    /// Target becomes the larger of itself and `value` (e.g. Unarmored Defense).
    Max,
    /// Target becomes the smaller of itself and `value`.
    Min,
    /// Multiply the target by `value`.
    Mul,
    /// Grant a proficiency; target is `proficiency.<kind>.<name>`.
    Grant,
    /// Double proficiency for a skill/tool; target is `skill.<name>` or `tool.<name>`.
    Expertise,
    /// Advantage on the target (recorded as a note on the sheet).
    Advantage,
    /// Disadvantage on the target.
    Disadvantage,
    /// Free-form reminder shown next to the target.
    Note,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Modifier {
    pub target: String,
    #[serde(default)]
    pub op: ModOp,
    /// Formula string. Empty is treated as 0 (or ignored for grant/note ops).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub value: String,
    /// Boolean formula; the modifier only applies when it is true.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub condition: String,
    /// Shortcut: only applies at this total level or above.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub min_level: u32,
    /// Where it came from. Content authors may leave it blank; the engine
    /// fills in the feature / item / species name.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
}

fn is_zero(v: &u32) -> bool {
    *v == 0
}

impl Modifier {
    pub fn new(target: impl Into<String>, op: ModOp, value: impl Into<String>) -> Self {
        Self { target: target.into(), op, value: value.into(), ..Default::default() }
    }
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = source.into();
        self
    }
    pub fn grant(target: impl Into<String>) -> Self {
        Self::new(target, ModOp::Grant, "")
    }
}
