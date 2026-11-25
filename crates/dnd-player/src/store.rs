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
        }
    }
}

#[derive(Debug, Clone)]
pub struct Store {
    pub dir: PathBuf,
}

impl Store {
    pub fn new(dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(dir.join("cache"));
        Self { dir }
    }

    pub fn load_config(&self) -> Config {
        std::fs::read_to_string(self.dir.join("config.toml")).ok().and_then(|t| toml::from_str(&t).ok()).unwrap_or_default()
    }

    pub fn save_config(&self, c: &Config) {
        if let Ok(t) = toml::to_string_pretty(c) {
            let _ = std::fs::write(self.dir.join("config.toml"), t);
        }
    }

    pub fn cache_character(&self, ch: &Character, sheet: &Sheet) {
        if let Ok(t) = toml::to_string_pretty(ch) {
            let _ = std::fs::write(self.dir.join("cache").join(format!("{}.toml", ch.id)), t);
        }
        if let Ok(t) = serde_json::to_string(sheet) {
            let _ = std::fs::write(self.dir.join("cache").join(format!("{}.sheet.json", ch.id)), t);
        }
    }

    pub fn load_cached(&self, id: &str) -> Option<(Character, Sheet)> {
        let ch: Character = toml::from_str(&std::fs::read_to_string(self.dir.join("cache").join(format!("{id}.toml"))).ok()?).ok()?;
        let sheet: Sheet = serde_json::from_str(&std::fs::read_to_string(self.dir.join("cache").join(format!("{id}.sheet.json"))).ok()?).ok()?;
        Some((ch, sheet))
    }

    /// The DM's species / classes / backgrounds, kept so a character can be
    /// drafted while the DM's laptop is off.
    pub fn save_builder(&self, o: &dnd_core::api::BuilderOptions) {
        if let Ok(t) = serde_json::to_string(o) {
            let _ = std::fs::write(self.dir.join("builder.json"), t);
        }
    }

    pub fn load_builder(&self) -> Option<dnd_core::api::BuilderOptions> {
        serde_json::from_str(&std::fs::read_to_string(self.dir.join("builder.json")).ok()?).ok()
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
