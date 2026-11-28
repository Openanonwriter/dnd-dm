//! Local persistence: a small config file and a cache of the last sheet so
//! the app still opens the character when the DM's laptop is off.

use dnd_core::{Character, Sheet};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub server_url: String,
    #[serde(default)]
    pub join_code: String,
    #[serde(default)]
    pub character_id: String,
    #[serde(default = "one")]
    pub zoom: f32,
    #[serde(default)]
    pub recent_servers: Vec<String>,
    /// The name this player goes by; the DM sees it next to this device.
    #[serde(default)]
    pub player_name: String,
    /// Made up on first run; identifies this install to the DM's server.
    #[serde(default)]
    pub device_id: String,
    /// Campaign the server at `server_url` was running when last connected
    /// (empty until the first answer from a server).
    #[serde(default)]
    pub campaign: String,
    /// Every game this device has joined: one per server and campaign.
    #[serde(default)]
    pub games: Vec<Game>,
}

/// One game the player has joined: a campaign on a DM's server. The same
/// address running a different campaign is a different game, so each keeps
/// its own character, name, join code and offline copy.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Game {
    #[serde(default)]
    pub server_url: String,
    #[serde(default)]
    pub campaign: String,
    #[serde(default)]
    pub join_code: String,
    #[serde(default)]
    pub player_name: String,
    #[serde(default)]
    pub character_id: String,
    #[serde(default)]
    pub character_name: String,
    /// Seconds since 1970 when this game was last connected to.
    #[serde(default)]
    pub last_used: u64,
}

impl Game {
    pub fn key(&self) -> String {
        game_key(&self.server_url, &self.campaign)
    }
}

/// Folder name for a game's files: the same for the same server and campaign
/// on every run (FNV-1a, so it does not depend on the Rust version).
pub fn game_key(server_url: &str, campaign: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in server_url.bytes().chain([b'\n']).chain(campaign.bytes()) {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

pub fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// A random id for this install (no personal or hardware information in it).
pub fn new_device_id() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut out = String::new();
    for _ in 0..2 {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0));
        out.push_str(&format!("{:016x}", h.finish()));
    }
    out
}

fn one() -> f32 {
    1.0
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server_url: String::new(),
            join_code: String::new(),
            character_id: String::new(),
            zoom: 1.0,
            recent_servers: Vec::new(),
            player_name: String::new(),
            device_id: String::new(),
            campaign: String::new(),
            games: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Store {
    pub dir: PathBuf,
    /// Key of the game whose files are in use (see [`game_key`]); empty
    /// before any game is known. Characters and the DM's content list are
    /// kept per game in `games/<key>/`. Files written by versions that knew
    /// only one game sit directly in `dir` and are still read as a fallback.
    pub scope: String,
}

impl Store {
    pub fn new(dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(dir.join("cache"));
        Self { dir, scope: String::new() }
    }

    pub fn load_config(&self) -> Config {
        std::fs::read_to_string(self.dir.join("config.toml")).ok().and_then(|t| toml::from_str(&t).ok()).unwrap_or_default()
    }

    pub fn save_config(&self, c: &Config) {
        if let Ok(t) = toml::to_string_pretty(c) {
            let _ = std::fs::write(self.dir.join("config.toml"), t);
        }
    }

    /// Use the files of this game from now on.
    pub fn set_scope(&mut self, key: &str) {
        self.scope = key.to_string();
        let _ = std::fs::create_dir_all(self.game_dir(key).join("cache"));
    }

    fn game_dir(&self, key: &str) -> PathBuf {
        if key.is_empty() {
            self.dir.clone()
        } else {
            self.dir.join("games").join(key)
        }
    }

    /// Delete everything kept for a game (its offline characters and content list).
    pub fn forget_game(&self, key: &str) {
        if !key.is_empty() {
            let _ = std::fs::remove_dir_all(self.game_dir(key));
        }
    }

    pub fn cache_character(&self, ch: &Character, sheet: &Sheet) {
        let dir = self.game_dir(&self.scope).join("cache");
        if let Ok(t) = toml::to_string_pretty(ch) {
            let _ = std::fs::write(dir.join(format!("{}.toml", ch.id)), t);
        }
        if let Ok(t) = serde_json::to_string(sheet) {
            let _ = std::fs::write(dir.join(format!("{}.sheet.json", ch.id)), t);
        }
    }

    pub fn load_cached(&self, id: &str) -> Option<(Character, Sheet)> {
        self.load_cached_in(&self.scope, id)
    }

    /// The offline copy of a character in the given game.
    pub fn load_cached_in(&self, key: &str, id: &str) -> Option<(Character, Sheet)> {
        if id.is_empty() {
            return None;
        }
        let read = |dir: PathBuf| -> Option<(Character, Sheet)> {
            let ch: Character = toml::from_str(&std::fs::read_to_string(dir.join(format!("{id}.toml"))).ok()?).ok()?;
            let sheet: Sheet = serde_json::from_str(&std::fs::read_to_string(dir.join(format!("{id}.sheet.json"))).ok()?).ok()?;
            Some((ch, sheet))
        };
        read(self.game_dir(key).join("cache")).or_else(|| read(self.dir.join("cache")))
    }

    /// The DM's species / classes / backgrounds, kept so a character can be
    /// drafted while the DM's laptop is off.
    pub fn save_builder(&self, o: &dnd_core::api::BuilderOptions) {
        if let Ok(t) = serde_json::to_string(o) {
            let _ = std::fs::write(self.game_dir(&self.scope).join("builder.json"), t);
        }
    }

    pub fn load_builder(&self) -> Option<dnd_core::api::BuilderOptions> {
        let read = |dir: PathBuf| -> Option<dnd_core::api::BuilderOptions> { serde_json::from_str(&std::fs::read_to_string(dir.join("builder.json")).ok()?).ok() };
        read(self.game_dir(&self.scope)).or_else(|| read(self.dir.clone()))
    }

    /// The character the player is still building.
    pub fn save_draft(&self, ch: &Character) {
        if let Ok(t) = toml::to_string_pretty(ch) {
            let _ = std::fs::write(self.dir.join("draft.toml"), t);
        }
    }

    pub fn load_draft(&self) -> Option<Character> {
        toml::from_str(&std::fs::read_to_string(self.dir.join("draft.toml")).ok()?).ok()
    }

    pub fn clear_draft(&self) {
        let _ = std::fs::remove_file(self.dir.join("draft.toml"));
    }

    pub fn export_path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}_sheet.html"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store(tag: &str) -> Store {
        let dir = std::env::temp_dir().join(format!("dnd-player-store-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        Store::new(dir)
    }

    fn hero(id: &str, name: &str) -> (Character, Sheet) {
        let c = Character { id: id.into(), name: name.into(), ..Default::default() };
        (c, Sheet::default())
    }

    /// Two campaigns can both have a character called "hero": each game keeps its own copy.
    #[test]
    fn games_keep_their_characters_apart() {
        let mut store = temp_store("apart");
        let a = game_key("http://10.0.0.1:8420", "Sunless Cellar");
        let b = game_key("http://10.0.0.2:8420", "Sunless Cellar");
        let c = game_key("http://10.0.0.1:8420", "Spider Caves");
        assert!(a != b && a != c && b != c);
        assert_eq!(a, game_key("http://10.0.0.1:8420", "Sunless Cellar"));

        store.set_scope(&a);
        let (ch, sh) = hero("hero", "Thia");
        store.cache_character(&ch, &sh);
        store.set_scope(&b);
        assert!(store.load_cached("hero").is_none());
        let (ch, sh) = hero("hero", "Borin");
        store.cache_character(&ch, &sh);

        assert_eq!(store.load_cached_in(&a, "hero").unwrap().0.name, "Thia");
        assert_eq!(store.load_cached_in(&b, "hero").unwrap().0.name, "Borin");
        store.forget_game(&a);
        assert!(store.load_cached_in(&a, "hero").is_none());
        assert_eq!(store.load_cached_in(&b, "hero").unwrap().0.name, "Borin");
        let _ = std::fs::remove_dir_all(&store.dir);
    }

    /// Files written before the app knew several games are still found.
    #[test]
    fn old_single_game_files_still_load() {
        let mut store = temp_store("legacy");
        let (ch, sh) = hero("fresh_fighter", "Fresh Fighter");
        store.cache_character(&ch, &sh); // no scope: the old place
        store.set_scope(&game_key("http://10.0.0.1:8420", "New Campaign"));
        assert_eq!(store.load_cached("fresh_fighter").unwrap().0.name, "Fresh Fighter");
        let _ = std::fs::remove_dir_all(&store.dir);
    }

    #[test]
    fn config_from_an_older_version_loads() {
        let c: Config = toml::from_str("server_url = \"http://10.0.0.1:8420\"\ncharacter_id = \"x\"\nplayer_name = \"Sam\"\n").unwrap();
        assert!(c.games.is_empty() && c.campaign.is_empty());
        assert_eq!(c.character_id, "x");
    }
}
