//! Saved game state: a campaign with its encounter, map and log.
//! Characters are saved as separate files next to it (see `save`).

use super::{DistanceRule, MapState};
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
    /// Carries a light: bright radius in feet (dim reaches twice as far).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub light: u32,
    /// Sees this far in the dark, in feet.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub darkvision: u32,
    /// Feet walked so far this turn (in a fight).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub moved: u32,
    /// Took the Dash action this turn: twice the movement.
    #[serde(default, skip_serializing_if = "is_no")]
    pub dashed: bool,
    /// Remaining legendary actions / uses, keyed by feature name.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub uses: BTreeMap<String, u32>,
}

fn is_zero(v: &u32) -> bool {
    *v == 0
}
fn is_no(v: &bool) -> bool {
    !*v
}
fn medium() -> String {
    "Medium".into()
}
fn yes() -> bool {
    true
}

/// The condition that marks a combatant as dead rather than only down.
pub const DEAD: &str = "dead";

impl Combatant {
    pub fn is_down(&self) -> bool {
        self.hp <= 0
    }
    /// Marked dead by the DM (kept as the `dead` condition, so it saves and
    /// reaches the players with the other conditions).
    pub fn is_dead(&self) -> bool {
        self.conditions.iter().any(|c| c == DEAD)
    }
    pub fn set_dead(&mut self, dead: bool) {
        self.conditions.retain(|c| c != DEAD);
        if dead {
            self.conditions.push(DEAD.into());
        }
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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct LogEntry {
    pub time: String,
    pub text: String,
}

/// One line of what everyone at the table sees happen: rolls, attacks,
/// spells, whose turn it is. Never holds anything only the DM may know.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TableEntry {
    pub seq: u64,
    pub time: String,
    /// Who did it (a character or creature name; empty = the game itself).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub who: String,
    pub text: String,
    /// roll, attack, spell, turn, rest, info
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub kind: String,
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
    /// Players may pick options their character doesn't qualify for
    /// (a feat above their level, a spell they can't cast yet...). The DM
    /// can always; this opens it to every player. See also the
    /// per-character `allow_rule_breaks`.
    #[serde(default)]
    pub allow_rule_breaks: bool,
    /// Level that player-made characters start at.
    #[serde(default = "default_start_level")]
    pub player_start_level: u32,
    /// Players may walk their own token on the map (on their turn, in a fight).
    #[serde(default = "yes")]
    pub players_move_tokens: bool,
    /// How far a player may walk a token in one go, in feet (0 = their speed).
    #[serde(default)]
    pub player_move_limit: u32,
    /// Attacks and spells players make from their app change the target's
    /// hit points straight away (off = they only roll; the DM applies it).
    #[serde(default = "yes")]
    pub players_apply_damage: bool,
    /// Attacks and spells must be in reach or range with a clear line, and
    /// a turn's movement is limited by speed (off = the DM referees it).
    #[serde(default = "yes")]
    pub enforce_range: bool,
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
            allow_rule_breaks: false,
            player_start_level: 1,
            players_move_tokens: true,
            player_move_limit: 0,
            players_apply_damage: true,
            enforce_range: true,
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
    /// What happened at the table, newest last (the last few hundred lines).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub table: Vec<TableEntry>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
    /// In-game calendar / clock, free text.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub game_time: String,
    /// Player devices that have joined, and which character each one plays.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub devices: Vec<Device>,
    /// The other maps of the campaign (the one in play is `map`). Each is a
    /// file in `maps/`.
    #[serde(skip)]
    pub maps: Vec<MapState>,
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
    /// Tell the whole table something (also kept in the DM's log).
    pub fn say(&mut self, who: &str, text: impl Into<String>, kind: &str) {
        let text = text.into();
        let seq = self.table.last().map(|e| e.seq + 1).unwrap_or(1);
        self.log(if who.is_empty() { text.clone() } else { format!("{who}: {text}") });
        self.table.push(TableEntry { seq, time: chrono::Local::now().format("%H:%M").to_string(), who: who.to_string(), text, kind: kind.to_string() });
        if self.table.len() > 300 {
            let cut = self.table.len() - 300;
            self.table.drain(..cut);
        }
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

    /// An id no other map uses, made from the name.
    pub fn unique_map_id(&self, name: &str) -> String {
        let base = super::Character::slug(name);
        let base = if base.is_empty() { "map".to_string() } else { base };
        let taken = |id: &str| self.map.id == id || self.maps.iter().any(|m| m.id == id);
        if !taken(&base) {
            return base;
        }
        (2..).map(|n| format!("{base}_{n}")).find(|id| !taken(id)).unwrap()
    }

    /// Put a new map on the shelf. Returns its id.
    pub fn add_map(&mut self, mut map: MapState) -> String {
        map.normalize();
        if self.map.id.is_empty() {
            self.map.id = self.unique_map_id(&self.map.name.clone());
        }
        map.id = self.unique_map_id(&map.name);
        let id = map.id.clone();
        self.maps.push(map);
        id
    }

    /// Move the game to another map. The creatures on the current map stay
    /// behind with it, the ones prepared on the new map come out, and the
    /// player characters arrive at its starting point.
    pub fn switch_map(&mut self, id: &str, size_squares: impl Fn(&str) -> u32) -> bool {
        let Some(i) = self.maps.iter().position(|m| m.id == id) else { return false };
        if self.map.id.is_empty() {
            self.map.id = self.unique_map_id(&self.map.name.clone());
        }
        self.encounter.end();
        let (pcs, others): (Vec<Combatant>, Vec<Combatant>) = std::mem::take(&mut self.encounter.combatants).into_iter().partition(|c| matches!(c.kind, CombatantKind::Pc { .. }));
        self.map.parked = others;
        std::mem::swap(&mut self.map, &mut self.maps[i]);
        self.encounter.combatants = pcs;
        self.encounter.combatants.append(&mut self.map.parked);
        for c in self.encounter.combatants.iter_mut().filter(|c| matches!(c.kind, CombatantKind::Pc { .. })) {
            c.pos = None;
        }
        self.place_party(size_squares);
        let name = self.map.name.clone();
        self.log(format!("the party moves to '{name}'"));
        true
    }

    /// Stand every player character without a position near the map's
    /// starting point (or its middle), on open squares.
    pub fn place_party(&mut self, size_squares: impl Fn(&str) -> u32) {
        let start = self.map.start.unwrap_or(Pos::new(self.map.width as i32 / 2, self.map.height as i32 / 2));
        let mut taken: Vec<Pos> = self.encounter.combatants.iter().filter_map(|c| c.pos).collect();
        let mut spots = Vec::new();
        for r in 0..(self.map.width.max(self.map.height) as i32) {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs().max(dy.abs()) == r {
                        spots.push(Pos::new(start.x + dx, start.y + dy));
                    }
                }
            }
        }
        for c in self.encounter.combatants.iter_mut().filter(|c| c.pos.is_none() && matches!(c.kind, CombatantKind::Pc { .. })) {
            let side = size_squares(&c.size).max(1) as i32;
            let free = spots.iter().find(|p| {
                !taken.contains(p) && (0..side).all(|dy| (0..side).all(|dx| !self.map.solid(p.x + dx, p.y + dy)))
                    // reachable from the start without crossing rock: same open area
                    && crate::vision::find_path(&self.map, start, **p, 1, 12).is_some()
            });
            if let Some(p) = free.or(spots.first()) {
                c.pos = Some(*p);
                taken.push(*p);
            }
        }
    }

    pub fn forget_device(&mut self, device_id: &str) -> bool {
        let before = self.devices.len();
        self.devices.retain(|d| d.id != device_id);
        self.devices.len() != before
    }
}
