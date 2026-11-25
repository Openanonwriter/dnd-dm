//! Saving and loading campaigns. A save is a folder:
//! ```text
//! saves/<campaign>/campaign.toml
//! saves/<campaign>/characters/<id>.toml
//! ```
//! Everything is TOML so it can be edited by hand or kept in git.

use crate::model::{Campaign, Character};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub fn save_character(ch: &Character, path: &Path) -> Result<()> {
    let text = toml::to_string_pretty(ch)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, text).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

pub fn load_character(path: &Path) -> Result<Character> {
    let text = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let mut ch: Character = toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
    if ch.id.is_empty() {
        ch.id = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    }
    Ok(ch)
}

/// Save the whole campaign into `dir`, replacing the character files.
pub fn save_campaign(camp: &mut Campaign, dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    camp.saved = chrono::Local::now().to_rfc3339();
    let text = toml::to_string_pretty(&*camp)?;
    std::fs::write(dir.join("campaign.toml"), text)?;
    let chars_dir = dir.join("characters");
    std::fs::create_dir_all(&chars_dir)?;
    // remove characters that no longer exist
    if let Ok(rd) = std::fs::read_dir(&chars_dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "toml").unwrap_or(false) {
                let stem = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                if !camp.characters.iter().any(|c| c.id == stem) {
                    let _ = std::fs::remove_file(&p);
                }
            }
        }
    }
    for ch in &camp.characters {
        save_character(ch, &chars_dir.join(format!("{}.toml", ch.id)))?;
    }
    Ok(())
}

pub fn load_campaign(dir: &Path) -> Result<Campaign> {
    let text = std::fs::read_to_string(dir.join("campaign.toml")).with_context(|| format!("reading {}", dir.display()))?;
    let mut camp: Campaign = toml::from_str(&text).context("parsing campaign.toml")?;
    let chars_dir = dir.join("characters");
    if chars_dir.is_dir() {
        let mut paths: Vec<PathBuf> = std::fs::read_dir(&chars_dir)?
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().map(|x| x == "toml").unwrap_or(false))
            .collect();
        paths.sort();
        for p in paths {
            match load_character(&p) {
                Ok(ch) => camp.characters.push(ch),
                Err(e) => camp.log(format!("failed to load {}: {e:#}", p.display())),
            }
        }
    }
    Ok(camp)
}

/// Campaign folders under `saves_dir`, newest first.
pub fn list_campaigns(saves_dir: &Path) -> Vec<(String, PathBuf, String)> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(saves_dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.join("campaign.toml").is_file() {
                let name = p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                let saved = std::fs::read_to_string(p.join("campaign.toml"))
                    .ok()
                    .and_then(|t| toml::from_str::<Campaign>(&t).ok())
                    .map(|c| c.saved)
                    .unwrap_or_default();
                out.push((name, p, saved));
            }
        }
    }
    out.sort_by(|a, b| b.2.cmp(&a.2));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    #[test]
    fn roundtrip() {
        let dir = std::env::temp_dir().join(format!("dnd-dm-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut camp = Campaign::new("Test");
        let mut ch = Character::new("ann", "Ann");
        ch.classes.push(ClassLevel { class: "wizard".into(), level: 2, ..Default::default() });
        ch.modifiers.push(Modifier::new("ac", ModOp::Add, "1"));
        camp.characters.push(ch);
        camp.encounter.combatants.push(Combatant { id: "gob".into(), name: "Goblin".into(), hp: 7, hp_max: 7, pos: Some(crate::grid::Pos::new(1, 2)), ..Default::default() });
        camp.log("hello");
        save_campaign(&mut camp, &dir).unwrap();
        let loaded = load_campaign(&dir).unwrap();
        assert_eq!(loaded.name, "Test");
        assert_eq!(loaded.characters.len(), 1);
        assert_eq!(loaded.characters[0].classes[0].level, 2);
        assert_eq!(loaded.encounter.combatants[0].pos, Some(crate::grid::Pos::new(1, 2)));
        assert_eq!(loaded.log.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
