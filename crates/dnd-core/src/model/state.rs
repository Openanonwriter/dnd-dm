//! Saved game state: a campaign with its encounter, map and log.
//! Characters are saved as separate files next to it (see `save`).

use super::DistanceRule;
use crate::grid::Pos;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CombatantKind {
    /// A player character; `id` is the character id.
    Pc { id: String },
    /// A creature from the library; `template` is the creature id.
    Creature { template: String },
    /// Something ad hoc the DM typed in.
    Custom,
}

impl Default for CombatantKind {
    fn default() -> Self {
        CombatantKind::Custom
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Combatant {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub kind: CombatantKind,
    #[serde(default)]
    pub initiative: i32,
    #[serde(default)]
    pub hp: i32,
    #[serde(default)]
    pub hp_max: i32,
    #[serde(default)]
    pub temp_hp: i32,
    #[serde(default)]
    pub ac: i32,
    #[serde(default = "medium")]
    pub size: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pos: Option<Pos>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
    #[serde(default = "yes")]
    pub visible: bool,
    /// Concentration spell name, if any.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub concentrating_on: String,
    /// Remaining legendary actions / uses, keyed by feature name.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub uses: BTreeMap<String, u32>,
}

fn medium() -> String {
    "Medium".into()
}
fn yes() -> bool {
    true
}

impl Combatant {
    pub fn is_down(&self) -> bool {
        self.hp <= 0
    }
    /// Apply damage, soaking temp HP first. Returns actual HP lost.
    pub fn damage(&mut self, amount: i32) -> i32 {
        let mut remaining = amount.max(0);
        let soaked = remaining.min(self.temp_hp);
        self.temp_hp -= soaked;
        remaining -= soaked;
        let before = self.hp;
        self.hp = (self.hp - remaining).max(0);
        before - self.hp
    }
    pub fn heal(&mut self, amount: i32) {
        self.hp = (self.hp + amount.max(0)).min(self.hp_max.max(self.hp));
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Encounter {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub round: u32,
    /// Index into `combatants` (sorted by initiative, descending).
    #[serde(default)]
    pub turn: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub combatants: Vec<Combatant>,
}

impl Encounter {
    pub fn sort_by_initiative(&mut self) {
        self.combatants.sort_by(|a, b| b.initiative.cmp(&a.initiative).then_with(|| a.name.cmp(&b.name)));
    }
    pub fn start(&mut self) {
        self.sort_by_initiative();
        self.active = true;
        self.round = 1;
        self.turn = 0;
    }
    pub fn next_turn(&mut self) {
        if self.combatants.is_empty() {
            return;
        }
        self.turn += 1;
        if self.turn >= self.combatants.len() {
            self.turn = 0;
            self.round += 1;
        }
    }
    pub fn prev_turn(&mut self) {
        if self.combatants.is_empty() {
            return;
        }
        if self.turn == 0 {
            self.turn = self.combatants.len() - 1;
            self.round = self.round.saturating_sub(1).max(1);
        } else {
            self.turn -= 1;
        }
    }
    pub fn current(&self) -> Option<&Combatant> {
        self.combatants.get(self.turn)
    }
    pub fn end(&mut self) {
        self.active = false;
        self.round = 0;
        self.turn = 0;
    }
    pub fn find(&self, id: &str) -> Option<&Combatant> {
        self.combatants.iter().find(|c| c.id == id)
    }
    pub fn find_mut(&mut self, id: &str) -> Option<&mut Combatant> {
        self.combatants.iter_mut().find(|c| c.id == id)
    }
    pub fn remove(&mut self, id: &str) {
        if let Some(i) = self.combatants.iter().position(|c| c.id == id) {
            self.combatants.remove(i);
            if self.turn >= self.combatants.len() {
                self.turn = 0;
            }
        }
    }
    /// Unique id for a new combatant with this base name.
    pub fn unique_id(&self, base: &str) -> String {
        let base = super::Character::slug(base);
        if self.find(&base).is_none() {
            return base;
        }
        let mut n = 2;
        loop {
            let id = format!("{base}_{n}");
            if self.find(&id).is_none() {
                return id;
            }
            n += 1;
        }
    }
}

/// Battle map description. Image is optional; the grid alone is enough for
/// distance and area math.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MapState {
    #[serde(default)]
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub image: String,
    #[serde(default = "thirty")]
    pub width: u32,
    #[serde(default = "twenty")]
    pub height: u32,
    #[serde(default = "five")]
    pub feet_per_square: u32,
    /// Terrain / walls / notes the DM wants to remember, free text.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
    /// Marked squares (difficult terrain, walls, hazards) keyed by "x,y".
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub marks: BTreeMap<String, String>,
}

fn thirty() -> u32 {
    30
}
fn twenty() -> u32 {
    20
}
fn five() -> u32 {
    5
}

impl Default for MapState {
    fn default() -> Self {
        Self { name: "Battle map".into(), image: String::new(), width: 30, height: 20, feet_per_square: 5, notes: String::new(), marks: BTreeMap::new() }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct LogEntry {
    pub time: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CampaignSettings {
    #[serde(default)]
    pub distance_rule: DistanceRule,
    #[serde(default)]
    pub show_monster_hp_to_players: bool,
    /// Join code players must enter in their app.
    #[serde(default)]
    pub join_code: String,
    /// Code required for `/api/dm/*` calls (remote DM tools). Empty = refuse all.
    #[serde(default)]
    pub dm_code: String,
    #[serde(default = "default_port")]
    pub port: u16,
    /// When off (the default) a player's app shows only the character the DM
    /// assigned to that device; when on, unassigned players may pick one.
    #[serde(default)]
    pub players_choose_character: bool,
    /// A session is being played right now. While off, players may build and
    /// upload their own character; while on, new characters come from the DM.
    #[serde(default)]
    pub game_live: bool,
    /// Level that player-made characters start at.
    #[serde(default = "default_start_level")]
    pub player_start_level: u32,
}

fn default_start_level() -> u32 {
    1
}

fn default_port() -> u16 {
    8420
}

impl Default for CampaignSettings {
    fn default() -> Self {
        Self {
            distance_rule: DistanceRule::Chebyshev,
            show_monster_hp_to_players: false,
            join_code: String::new(),
            dm_code: String::new(),
            port: 8420,
            players_choose_character: false,
            game_live: false,
            player_start_level: 1,
        }
    }
}

/// A player's phone or PC that registered with the server. The id is made up
/// by the app on first run and never changes; the name is what the player
/// typed, so the DM knows whose device it is.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Device {
    pub id: String,
    pub name: String,
    /// Character id the DM assigned (or the player claimed); empty = none yet.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub character: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub last_seen: String,
    /// Id of the character this device's player built themselves (they may
    /// rebuild that one between sessions).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub made: String,
    /// Ability scores the server rolled for this player (4d6 drop lowest, six times).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rolled: Vec<i32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Campaign {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub created: String,
    #[serde(default)]
    pub saved: String,
    #[serde(default)]
    pub settings: CampaignSettings,
    #[serde(default)]
    pub encounter: Encounter,
    #[serde(default)]
    pub map: MapState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub log: Vec<LogEntry>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
    /// In-game calendar / clock, free text.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub game_time: String,
    /// Player devices that have joined, and which character each one plays.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub devices: Vec<Device>,
    /// Characters live in their own files; kept here in memory.
    #[serde(skip)]
    pub characters: Vec<super::Character>,
}

impl Campaign {
    pub fn new(name: impl Into<String>) -> Self {
        let now = chrono::Local::now().to_rfc3339();
        Self { name: name.into(), created: now.clone(), saved: now, ..Default::default() }
    }
    pub fn log(&mut self, text: impl Into<String>) {
        self.log.push(LogEntry { time: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(), text: text.into() });
    }
    pub fn character(&self, id: &str) -> Option<&super::Character> {
        self.characters.iter().find(|c| c.id == id)
    }
    pub fn character_mut(&mut self, id: &str) -> Option<&mut super::Character> {
        self.characters.iter_mut().find(|c| c.id == id)
    }

    pub fn device(&self, id: &str) -> Option<&Device> {
        self.devices.iter().find(|d| d.id == id)
    }

    /// Add the device or update its name. Returns true when something the DM
    /// would notice changed (new device or new name).
    pub fn register_device(&mut self, id: &str, name: &str) -> bool {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        if let Some(d) = self.devices.iter_mut().find(|d| d.id == id) {
            d.last_seen = now;
            if d.name != name {
                let old = std::mem::replace(&mut d.name, name.to_string());
                self.log(format!("player device '{old}' is now called '{name}'"));
                return true;
            }
            return false;
        }
        self.devices.push(Device { id: id.to_string(), name: name.to_string(), character: String::new(), last_seen: now, ..Default::default() });
        self.log(format!("player device '{name}' joined"));
        true
    }

    /// The character a device plays, if it is assigned one that still exists.
    pub fn device_character(&self, device_id: &str) -> Option<&super::Character> {
        let d = self.device(device_id)?;
        if d.character.is_empty() {
            return None;
        }
        self.character(&d.character)
    }

    /// Give a device a character (`None` takes it away). Fills in the
    /// character's player name when it is blank. Returns false when the
    /// device or the character does not exist.
    pub fn assign_device(&mut self, device_id: &str, character: Option<&str>) -> bool {
        let Some(i) = self.devices.iter().position(|d| d.id == device_id) else { return false };
        let dev_name = self.devices[i].name.clone();
        match character {
            Some(cid) => {
                let Some(ch) = self.character_mut(cid) else { return false };
                if ch.player.trim().is_empty() {
                    ch.player = dev_name.clone();
                }
                let ch_name = ch.name.clone();
                self.devices[i].character = cid.to_string();
                self.log(format!("'{dev_name}' now plays {ch_name}"));
            }
            None => {
                self.devices[i].character.clear();
                self.log(format!("'{dev_name}' has no character assigned"));
            }
        }
        true
    }

    /// Assign every unassigned device whose name matches a character's
    /// `player` field (case-insensitive) or the character's name. Returns
    /// how many were assigned.
    pub fn auto_assign_devices(&mut self) -> usize {
        let mut pairs = Vec::new();
        for d in self.devices.iter().filter(|d| d.character.is_empty() || self.character(&d.character).is_none()) {
            let n = d.name.trim().to_lowercase();
            let taken = |cid: &str| self.devices.iter().any(|o| o.character == cid) || pairs.iter().any(|(_, c): &(String, String)| c == cid);
            if let Some(ch) = self.characters.iter().find(|c| (c.player.trim().to_lowercase() == n || c.name.trim().to_lowercase() == n) && !taken(&c.id)) {
                pairs.push((d.id.clone(), ch.id.clone()));
            }
        }
        for (d, c) in &pairs {
            self.assign_device(d, Some(c));
        }
        pairs.len()
    }

    pub fn forget_device(&mut self, device_id: &str) -> bool {
        let before = self.devices.len();
        self.devices.retain(|d| d.id != device_id);
        self.devices.len() != before
    }
}
