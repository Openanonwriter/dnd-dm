//! Wire types shared by the server and the player apps. Everything is plain
//! serde so the JSON is stable and documented in `docs/API.md`.

use crate::grid::Pos;
use crate::model::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Info {
    pub app: String,
    pub version: String,
    pub campaign: String,
    pub revision: u64,
    pub join_code_required: bool,
    pub characters: usize,
    pub encounter_active: bool,
    #[serde(default)]
    pub packs: Vec<String>,
    /// A session is running: players can no longer upload new characters.
    #[serde(default)]
    pub game_live: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Listed {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CharListed {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub player: String,
    #[serde(default)]
    pub level: u32,
    #[serde(default)]
    pub class_line: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PlayerCombatant {
    pub id: String,
    pub name: String,
    pub is_pc: bool,
    pub initiative: i32,
    pub hp: Option<i32>,
    pub hp_max: Option<i32>,
    pub ac: Option<i32>,
    #[serde(default)]
    pub size: String,
    pub pos: Option<Pos>,
    #[serde(default)]
    pub conditions: Vec<String>,
    pub is_current: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PlayerEncounter {
    pub name: String,
    pub active: bool,
    pub round: u32,
    #[serde(default)]
    pub map: MapState,
    #[serde(default)]
    pub combatants: Vec<PlayerCombatant>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct RollReq {
    pub formula: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct RollResp {
    pub formula: String,
    pub total: i64,
    pub rolls: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ErrorBody {
    pub error: String,
}

/// A player walking their own token to a square.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct MapMoveReq {
    pub device_id: String,
    pub x: i32,
    pub y: i32,
}

/// A player opening or closing the door nearest to a point (in squares).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct MapDoorReq {
    pub device_id: String,
    pub x: f32,
    pub y: f32,
}

/// The map after a player's move, and how far they walked.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct MapMoved {
    pub feet: u32,
    pub map: crate::view::MapView,
}

/// A player app introducing itself (also its heartbeat).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct RegisterReq {
    pub device_id: String,
    pub name: String,
}

/// A player app asking to play a character (only when the DM allows picking).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ClaimReq {
    pub device_id: String,
    pub character: String,
}

/// The server's answer to a registration: who you are and what you play.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Seat {
    pub device_id: String,
    pub name: String,
    /// Assigned character id, if any.
    #[serde(default)]
    pub character: Option<String>,
    #[serde(default)]
    pub character_name: String,
    /// True when the DM lets players pick their own character.
    #[serde(default)]
    pub can_choose: bool,
    /// True when this device may build and upload its own character now
    /// (no session running and nothing assigned yet).
    #[serde(default)]
    pub can_create: bool,
    /// True while the DM is running a session.
    #[serde(default)]
    pub game_live: bool,
    /// The assigned character is one this device's player built, so they
    /// may rebuild it between sessions.
    #[serde(default)]
    pub made_character: bool,
    /// Ability scores the server rolled for this device (empty = not yet).
    #[serde(default)]
    pub rolled: Vec<i32>,
    /// Level that player-made characters start at.
    #[serde(default)]
    pub start_level: u32,
    /// The DM lets this device pick options its character doesn't qualify
    /// for (campaign-wide, or for the assigned character).
    #[serde(default)]
    pub allow_rule_breaks: bool,
}

/// A player asking the server to roll their ability scores.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct RollAbilitiesReq {
    pub device_id: String,
}

/// A player uploading a character they built. Only the choices are used
/// (see `builder::prepare_player_character`); the rest is ignored.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CreateReq {
    pub device_id: String,
    pub character: Character,
    /// Which option of each starting-equipment line to take (0 = first).
    #[serde(default)]
    pub gear: Vec<usize>,
}

/// What a player may build a character from.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct BuilderOptions {
    #[serde(default)]
    pub species: Vec<Species>,
    #[serde(default)]
    pub classes: Vec<Class>,
    #[serde(default)]
    pub subclasses: Vec<Subclass>,
    #[serde(default)]
    pub backgrounds: Vec<Background>,
    #[serde(default)]
    pub feats: Vec<Feat>,
    /// Visible items, id -> name (to recognise starting equipment).
    #[serde(default)]
    pub items: BTreeMap<String, String>,
    #[serde(default)]
    pub rules: Rules,
    /// Level that player-made characters start at.
    #[serde(default)]
    pub start_level: u32,
    /// Spells players may see, for choosing a caster's spells.
    #[serde(default)]
    pub spells: Vec<Spell>,
    /// Visible items in full, so the player app can explain them.
    #[serde(default)]
    pub item_entries: Vec<Item>,
}

/// A player asking for something to be rolled where the whole table sees it.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TableRollReq {
    pub device_id: String,
    pub formula: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub mode: crate::play::RollMode,
}

/// A player attacking a combatant with one of the attacks on their sheet.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct AttackReq {
    pub device_id: String,
    /// Index into the sheet's attacks.
    pub attack: usize,
    pub target: String,
    #[serde(default)]
    pub mode: crate::play::RollMode,
}

/// A player casting a spell. `slot` 0 = cantrip, or a ritual without a slot.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CastReq {
    pub device_id: String,
    pub spell: String,
    #[serde(default)]
    pub slot: u32,
    #[serde(default)]
    pub targets: Vec<String>,
    #[serde(default)]
    pub mode: crate::play::RollMode,
}

/// A player choosing which spells their character knows.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SpellsReq {
    pub device_id: String,
    pub known: Vec<String>,
}

/// Small things a player does for their own character: `end_turn`,
/// `short_rest`, `long_rest`, `hit_die` (with `arg` = "d8"), `death_save`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct DoReq {
    pub device_id: String,
    pub action: String,
    #[serde(default)]
    pub arg: String,
}

/// One row of the DM's device list.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct DeviceListed {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub character: Option<String>,
    pub online: bool,
    #[serde(default)]
    pub last_seen: String,
}

/// What a player is allowed to change on their own sheet. Everything else
/// (build, modifiers, hidden items, DM notes) stays with the DM.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PlayerPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hp: Option<HitPoints>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub death_saves: Option<DeathSaves>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spells: Option<SpellState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conditions: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exhaustion: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inspiration: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<Currency>,
    /// Visible inventory only; hidden items are preserved server-side.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inventory: Option<Vec<InventoryItem>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub appearance: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub personality: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ideals: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bonds: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flaws: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backstory: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom: Option<BTreeMap<String, String>>,
    /// Uses spent of limited features.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uses: Option<BTreeMap<String, u32>>,
}

impl PlayerPatch {
    /// Everything a player may edit, taken from a (player-view) character.
    pub fn from_character(c: &Character) -> Self {
        Self {
            hp: Some(c.hp.clone()),
            death_saves: Some(c.death_saves.clone()),
            spells: Some(c.spells.clone()),
            conditions: Some(c.conditions.clone()),
            exhaustion: Some(c.exhaustion),
            inspiration: Some(c.inspiration),
            uses: Some(c.uses.clone()),
            currency: Some(c.currency.clone()),
            inventory: Some(c.inventory.iter().filter(|i| !i.hidden).cloned().collect()),
            notes: Some(c.notes.clone()),
            appearance: Some(c.appearance.clone()),
            personality: Some(c.personality.clone()),
            ideals: Some(c.ideals.clone()),
            bonds: Some(c.bonds.clone()),
            flaws: Some(c.flaws.clone()),
            backstory: Some(c.backstory.clone()),
            player: Some(c.player.clone()),
            custom: Some(c.custom.clone()),
        }
    }
}

impl PlayerPatch {
    /// Only what the player changed between two copies of their character,
    /// so a slow phone never writes back something the DM changed meanwhile.
    pub fn diff(old: &Character, new: &Character) -> Self {
        let was = Self::from_character(old);
        let all = Self::from_character(new);
        macro_rules! changed {
            ($($f:ident),*) => { Self { $($f: if all.$f != was.$f { all.$f } else { None }),* } };
        }
        changed!(hp, death_saves, spells, conditions, exhaustion, inspiration, currency, inventory, notes, appearance, personality, ideals, bonds, flaws, backstory, player, custom, uses)
    }
}

impl Character {
    /// Apply a player's changes, keeping everything DM-owned intact.
    pub fn apply_player_patch(&mut self, p: PlayerPatch) {
        // a dead character stays dead until magic or the DM brings it back
        let dead = self.is_dead();
        if let Some(mut v) = p.hp {
            if dead {
                v.current = self.hp.current;
            }
            self.hp = v;
        }
        if let Some(v) = p.death_saves {
            if !dead {
                self.death_saves = v;
            }
        }
        if let Some(mut v) = p.spells {
            // which spells a character knows is decided by `play::choose_spells`
            v.known = std::mem::take(&mut self.spells.known);
            self.spells = v;
        }
        if let Some(v) = p.uses {
            self.uses = v;
        }
        if let Some(v) = p.conditions {
            self.conditions = v;
        }
        if let Some(v) = p.exhaustion {
            self.exhaustion = v;
        }
        if let Some(v) = p.inspiration {
            self.inspiration = v;
        }
        if let Some(v) = p.currency {
            self.currency = v;
        }
        if let Some(mut visible) = p.inventory {
            // keep DM-only fields of the visible items the player sent back
            let old_visible: Vec<&InventoryItem> = self.inventory.iter().filter(|i| !i.hidden).collect();
            for (i, item) in visible.iter_mut().enumerate() {
                item.hidden = false;
                if let Some(old) = old_visible.get(i) {
                    if old.item == item.item {
                        item.dm_notes = old.dm_notes.clone();
                    }
                }
            }
            let hidden: Vec<InventoryItem> = self.inventory.iter().filter(|i| i.hidden).cloned().collect();
            visible.extend(hidden);
            self.inventory = visible;
        }
        if let Some(v) = p.notes {
            self.notes = v;
        }
        if let Some(v) = p.appearance {
            self.appearance = v;
        }
        if let Some(v) = p.personality {
            self.personality = v;
        }
        if let Some(v) = p.ideals {
            self.ideals = v;
        }
        if let Some(v) = p.bonds {
            self.bonds = v;
        }
        if let Some(v) = p.flaws {
            self.flaws = v;
        }
        if let Some(v) = p.backstory {
            self.backstory = v;
        }
        if let Some(v) = p.player {
            self.player = v;
        }
        if let Some(v) = p.custom {
            self.custom = v;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patch_keeps_hidden_items_and_dm_notes() {
        let mut c = Character::new("a", "A");
        c.inventory = vec![
            InventoryItem { item: "dagger".into(), qty: 1, dm_notes: "nicked".into(), ..Default::default() },
            InventoryItem { item: "ring".into(), qty: 1, hidden: true, ..Default::default() },
        ];
        c.dm_notes = "secret".into();
        let mut view = c.player_view();
        assert_eq!(view.inventory.len(), 1);
        view.inventory[0].qty = 3;
        view.inventory.push(InventoryItem { item: "torch".into(), qty: 2, hidden: true, ..Default::default() }); // player tries to add a hidden item
        view.hp.current = 5;
        let patch = PlayerPatch::from_character(&view);
        c.apply_player_patch(patch);
        assert_eq!(c.hp.current, 5);
        assert_eq!(c.dm_notes, "secret");
        assert_eq!(c.inventory.len(), 2, "{:?}", c.inventory); // dagger + ring; the torch was hidden so filtered out of the patch
        assert_eq!(c.inventory[0].qty, 3);
        assert_eq!(c.inventory[0].dm_notes, "nicked");
        assert!(c.inventory[1].hidden);
    }
}
