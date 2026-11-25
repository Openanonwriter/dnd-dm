//! Finds the project root (the folder holding `content/`) and reads the
//! optional `dnd-dm.toml` config next to it.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Content packs, loaded in order; later packs override earlier ones.
    #[serde(default = "default_content")]
    pub content_dirs: Vec<String>,
    #[serde(default = "default_saves")]
    pub saves_dir: String,
    #[serde(default = "default_exports")]
    pub exports_dir: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "yes")]
    pub autostart_server: bool,
}

fn yes() -> bool {
    true
}

fn default_content() -> Vec<String> {
    vec!["content".into(), "homebrew".into()]
}
fn default_saves() -> String {
    "saves".into()
}
fn default_exports() -> String {
    "exports".into()
}
fn default_port() -> u16 {
    8420
}

impl Default for Config {
    fn default() -> Self {
        Self { content_dirs: default_content(), saves_dir: default_saves(), exports_dir: default_exports(), port: 8420, autostart_server: true }
    }
}

#[derive(Debug, Clone)]
pub struct Paths {
    pub root: PathBuf,
    pub config_path: PathBuf,
    pub config: Config,
    pub content_dirs: Vec<PathBuf>,
    pub saves_dir: PathBuf,
    pub exports_dir: PathBuf,
}

impl Paths {
    pub fn discover() -> Self {
        let mut candidates: Vec<PathBuf> = Vec::new();
        if let Ok(cwd) = std::env::current_dir() {
            candidates.push(cwd);
        }
        if let Ok(exe) = std::env::current_exe() {
            let mut p = exe.parent().map(|p| p.to_path_buf());
            while let Some(dir) = p {
                candidates.push(dir.clone());
                p = dir.parent().map(|p| p.to_path_buf());
            }
        }
        let root = candidates
            .iter()
            .find(|c| c.join("content").is_dir() || c.join("dnd-dm.toml").is_file())
            .cloned()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let config_path = root.join("dnd-dm.toml");
        let config: Config = std::fs::read_to_string(&config_path).ok().and_then(|t| toml::from_str(&t).ok()).unwrap_or_default();
        let abs = |s: &str| {
            let p = Path::new(s);
            if p.is_absolute() {
                p.to_path_buf()
            } else {
                root.join(p)
            }
        };
        let content_dirs: Vec<PathBuf> = config.content_dirs.iter().map(|s| abs(s)).collect();
        for (i, d) in content_dirs.iter().enumerate() {
            if !d.exists() {
                let _ = std::fs::create_dir_all(d);
                if i > 0 {
                    let _ = std::fs::write(
                        d.join("pack.toml"),
                        "name = \"Homebrew\"\nauthor = \"\"\ndescription = \"Your own species, classes, items, spells and creatures. Files here override the base pack.\"\n",
                    );
                    for k in dnd_core::Kind::ALL {
                        let _ = std::fs::create_dir_all(d.join(k.folder()));
                    }
                }
            }
        }
        let saves_dir = abs(&config.saves_dir);
        let exports_dir = abs(&config.exports_dir);
        let _ = std::fs::create_dir_all(&saves_dir);
        let _ = std::fs::create_dir_all(&exports_dir);
        if !config_path.exists() {
            let _ = std::fs::write(&config_path, toml::to_string_pretty(&config).unwrap_or_default());
        }
        Self { root, config_path, config, content_dirs, saves_dir, exports_dir }
    }
}
