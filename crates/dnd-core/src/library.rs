//! Content library: loads every `*.toml` under one or more content packs.
//!
//! Layout of a pack (any folder):
//! ```text
//! pack.toml            optional: name / author / description
//! rules.toml           optional: overrides for skills, PB table, XP table...
//! species/*.toml
//! classes/*.toml
//! subclasses/*.toml
//! backgrounds/*.toml
//! items/*.toml
//! feats/*.toml
//! spells/*.toml
//! creatures/*.toml
//! ```
//! Files may be nested in sub-folders. A file outside a kind folder needs a
//! `kind = "spell"` key. Packs loaded later override earlier entries with the
//! same id, so a `homebrew/` pack can patch the base pack.

use crate::model::*;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PackInfo {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub license: String,
    #[serde(skip)]
    pub path: PathBuf,
}

#[derive(Debug, Default, Clone)]
pub struct Library {
    pub rules: Rules,
    pub species: BTreeMap<String, Species>,
    pub classes: BTreeMap<String, Class>,
    pub subclasses: BTreeMap<String, Subclass>,
    pub backgrounds: BTreeMap<String, Background>,
    pub items: BTreeMap<String, Item>,
    pub feats: BTreeMap<String, Feat>,
    pub spells: BTreeMap<String, Spell>,
    pub creatures: BTreeMap<String, Creature>,
    /// Where each entry came from, for "open in editor".
    pub sources: BTreeMap<(Kind, String), PathBuf>,
    pub packs: Vec<PackInfo>,
    pub warnings: Vec<String>,
}

#[derive(Deserialize)]
struct KindProbe {
    kind: Option<String>,
}

impl Library {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load (or re-load) every pack in order.
    pub fn load_packs(dirs: &[PathBuf]) -> Self {
        let mut lib = Library::new();
        for d in dirs {
            if let Err(e) = lib.load_pack(d) {
                lib.warnings.push(format!("{}: {e:#}", d.display()));
            }
        }
        lib
    }

    pub fn load_pack(&mut self, dir: &Path) -> Result<()> {
        if !dir.is_dir() {
            anyhow::bail!("content pack folder does not exist");
        }
        let mut info = PackInfo::default();
        let pack_file = dir.join("pack.toml");
        if pack_file.is_file() {
            let text = std::fs::read_to_string(&pack_file)?;
            info = toml::from_str(&text).with_context(|| format!("parsing {}", pack_file.display()))?;
        }
        if info.name.is_empty() {
            info.name = dir.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "pack".into());
        }
        info.path = dir.to_path_buf();
        let rules_file = dir.join("rules.toml");
        if rules_file.is_file() {
            let text = std::fs::read_to_string(&rules_file)?;
            self.rules = toml::from_str(&text).with_context(|| format!("parsing {}", rules_file.display()))?;
        }
        self.walk(dir, None)?;
        self.packs.push(info);
        Ok(())
    }

    fn walk(&mut self, dir: &Path, kind: Option<Kind>) -> Result<()> {
        let mut entries: Vec<_> = std::fs::read_dir(dir)?.filter_map(|e| e.ok()).collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || name.starts_with('_') {
                continue;
            }
            if path.is_dir() {
                let k = Kind::from_folder(&name).or(kind);
                self.walk(&path, k)?;
            } else if path.extension().map(|e| e == "toml").unwrap_or(false) {
                if name == "pack.toml" || name == "rules.toml" {
                    continue;
                }
                if let Err(e) = self.load_file(&path, kind) {
                    self.warnings.push(format!("{}: {e:#}", path.display()));
                }
            }
        }
        Ok(())
    }

    /// Load one file. It holds either a single entry, or several under an
    /// `[[entries]]` array (or `[[items]]`, `[[spells]]`... matching its kind).
    pub fn load_file(&mut self, path: &Path, kind_hint: Option<Kind>) -> Result<()> {
        let text = std::fs::read_to_string(path)?;
        let probe: KindProbe = toml::from_str(&text).context("not valid TOML")?;
        let kind = match probe.kind.as_deref().and_then(Kind::from_folder) {
            Some(k) => k,
            None => kind_hint.context("file is not inside a content folder and has no `kind = \"...\"` key")?,
        };
        let table: toml::Table = toml::from_str(&text).context("not valid TOML")?;
        let multi = table
            .get("entries")
            .or_else(|| table.get(kind.folder()))
            .and_then(|v| v.as_array())
            .cloned();
        match multi {
            Some(list) => {
                for (i, v) in list.iter().enumerate() {
                    let sub = toml::to_string(v).context("re-serializing entry")?;
                    match parse_entry(kind, &sub) {
                        Ok(entry) => self.insert(entry, Some(path.to_path_buf())),
                        Err(e) => self.warnings.push(format!("{} entry #{}: {e:#}", path.display(), i + 1)),
                    }
                }
            }
            None => {
                let entry = parse_entry(kind, &text)?;
                self.insert(entry, Some(path.to_path_buf()));
            }
        }
        Ok(())
    }

    pub fn insert(&mut self, entry: Entry, source: Option<PathBuf>) {
        let kind = entry.kind();
        let id = entry.id().to_string();
        if id.is_empty() {
            self.warnings.push(format!("{} entry without id skipped ({:?})", kind.label(), source));
            return;
        }
        match entry {
            Entry::Species(x) => {
                self.species.insert(id.clone(), x);
            }
            Entry::Class(x) => {
                self.classes.insert(id.clone(), x);
            }
            Entry::Subclass(x) => {
                self.subclasses.insert(id.clone(), x);
            }
            Entry::Background(x) => {
                self.backgrounds.insert(id.clone(), x);
            }
            Entry::Item(x) => {
                self.items.insert(id.clone(), x);
            }
            Entry::Feat(x) => {
                self.feats.insert(id.clone(), x);
            }
            Entry::Spell(x) => {
                self.spells.insert(id.clone(), x);
            }
            Entry::Creature(x) => {
                self.creatures.insert(id.clone(), x);
            }
        }
        if let Some(p) = source {
            self.sources.insert((kind, id), p);
        }
    }

    pub fn get(&self, kind: Kind, id: &str) -> Option<Entry> {
        Some(match kind {
            Kind::Species => Entry::Species(self.species.get(id)?.clone()),
            Kind::Class => Entry::Class(self.classes.get(id)?.clone()),
            Kind::Subclass => Entry::Subclass(self.subclasses.get(id)?.clone()),
            Kind::Background => Entry::Background(self.backgrounds.get(id)?.clone()),
            Kind::Item => Entry::Item(self.items.get(id)?.clone()),
            Kind::Feat => Entry::Feat(self.feats.get(id)?.clone()),
            Kind::Spell => Entry::Spell(self.spells.get(id)?.clone()),
            Kind::Creature => Entry::Creature(self.creatures.get(id)?.clone()),
        })
    }

    /// (id, name) pairs sorted by name.
    pub fn list(&self, kind: Kind) -> Vec<(String, String)> {
        let mut v: Vec<(String, String)> = match kind {
            Kind::Species => self.species.values().map(|x| (x.id.clone(), x.name.clone())).collect(),
            Kind::Class => self.classes.values().map(|x| (x.id.clone(), x.name.clone())).collect(),
            Kind::Subclass => self.subclasses.values().map(|x| (x.id.clone(), x.name.clone())).collect(),
            Kind::Background => self.backgrounds.values().map(|x| (x.id.clone(), x.name.clone())).collect(),
            Kind::Item => self.items.values().map(|x| (x.id.clone(), x.name.clone())).collect(),
            Kind::Feat => self.feats.values().map(|x| (x.id.clone(), x.name.clone())).collect(),
            Kind::Spell => self.spells.values().map(|x| (x.id.clone(), x.name.clone())).collect(),
            Kind::Creature => self.creatures.values().map(|x| (x.id.clone(), x.name.clone())).collect(),
        };
        v.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));
        v
    }

    pub fn count(&self, kind: Kind) -> usize {
        match kind {
            Kind::Species => self.species.len(),
            Kind::Class => self.classes.len(),
            Kind::Subclass => self.subclasses.len(),
            Kind::Background => self.backgrounds.len(),
            Kind::Item => self.items.len(),
            Kind::Feat => self.feats.len(),
            Kind::Spell => self.spells.len(),
            Kind::Creature => self.creatures.len(),
        }
    }

    pub fn subclasses_of(&self, class_id: &str) -> Vec<&Subclass> {
        self.subclasses.values().filter(|s| s.class == class_id).collect()
    }

    pub fn spells_for_class(&self, class_id: &str) -> Vec<&Spell> {
        let mut v: Vec<&Spell> = self.spells.values().filter(|s| s.classes.iter().any(|c| c == class_id)).collect();
        v.sort_by(|a, b| a.level.cmp(&b.level).then_with(|| a.name.cmp(&b.name)));
        v
    }

    pub fn source_of(&self, kind: Kind, id: &str) -> Option<&PathBuf> {
        self.sources.get(&(kind, id.to_string()))
    }

    /// Write an entry as TOML into `pack_dir/<kind folder>/<id>.toml`.
    pub fn save_entry(&mut self, entry: &Entry, pack_dir: &Path) -> Result<PathBuf> {
        let folder = pack_dir.join(entry.kind().folder());
        std::fs::create_dir_all(&folder)?;
        let path = folder.join(format!("{}.toml", entry.id()));
        std::fs::write(&path, entry_to_toml(entry)?)?;
        self.insert(entry.clone(), Some(path.clone()));
        Ok(path)
    }
}

pub fn parse_entry(kind: Kind, text: &str) -> Result<Entry> {
    Ok(match kind {
        Kind::Species => Entry::Species(toml::from_str(text)?),
        Kind::Class => Entry::Class(toml::from_str(text)?),
        Kind::Subclass => Entry::Subclass(toml::from_str(text)?),
        Kind::Background => Entry::Background(toml::from_str(text)?),
        Kind::Item => Entry::Item(toml::from_str(text)?),
        Kind::Feat => Entry::Feat(toml::from_str(text)?),
        Kind::Spell => Entry::Spell(toml::from_str(text)?),
        Kind::Creature => Entry::Creature(toml::from_str(text)?),
    })
}

/// Serialize an entry to TOML without the internal `kind` tag.
pub fn entry_to_toml(entry: &Entry) -> Result<String> {
    Ok(match entry {
        Entry::Species(x) => toml::to_string_pretty(x)?,
        Entry::Class(x) => toml::to_string_pretty(x)?,
        Entry::Subclass(x) => toml::to_string_pretty(x)?,
        Entry::Background(x) => toml::to_string_pretty(x)?,
        Entry::Item(x) => toml::to_string_pretty(x)?,
        Entry::Feat(x) => toml::to_string_pretty(x)?,
        Entry::Spell(x) => toml::to_string_pretty(x)?,
        Entry::Creature(x) => toml::to_string_pretty(x)?,
    })
}

/// A starter file for a new entry of the given kind, with comments.
pub fn template(kind: Kind, id: &str, name: &str) -> String {
    match kind {
        Kind::Species => format!(
            "id = \"{id}\"\nname = \"{name}\"\nsize = \"Medium\"\nspeed = 30\ndarkvision = 0\nlanguages = [\"Common\"]\ndescription = \"\"\n\n# [[modifiers]]\n# target = \"ability.con\"\n# op = \"add\"\n# value = \"2\"\n\n# [[traits]]\n# name = \"Keen Senses\"\n# description = \"You have proficiency in the Perception skill.\"\n# modifiers = [{{ target = \"proficiency.skill.perception\", op = \"grant\" }}]\n"
        ),
        Kind::Class => format!(
            "id = \"{id}\"\nname = \"{name}\"\nhit_die = 8\nprimary_abilities = [\"str\"]\nsaving_throws = [\"str\", \"con\"]\narmor_proficiencies = [\"light\", \"medium\", \"shields\"]\nweapon_proficiencies = [\"simple\", \"martial\"]\nsubclass_level = 3\nsubclass_label = \"Archetype\"\ndescription = \"\"\n\n[skill_choices]\ncount = 2\nfrom = [\"athletics\", \"perception\"]\n\n[[features]]\nname = \"Example Feature\"\nlevel = 1\ndescription = \"What it does.\"\n# modifiers = [{{ target = \"ac\", op = \"add\", value = \"1\" }}]\n\n# [spellcasting]\n# ability = \"wis\"\n# progression = \"full\"\n# prepared = \"max(1, wis_mod + class.{id}.level)\"\n# cantrips_known = [3,3,3,4,4,4,4,4,4,5,5,5,5,5,5,5,5,5,5,5]\n"
        ),
        Kind::Subclass => format!(
            "id = \"{id}\"\nname = \"{name}\"\nclass = \"fighter\"\ndescription = \"\"\n\n[[features]]\nname = \"Example Feature\"\nlevel = 3\ndescription = \"\"\n"
        ),
        Kind::Background => format!(
            "id = \"{id}\"\nname = \"{name}\"\nskill_proficiencies = [\"insight\", \"religion\"]\ntool_proficiencies = []\nlanguages = []\nlanguage_choices = 2\nstarting_equipment = [\"holy symbol\", \"15 gp\"]\ndescription = \"\"\n\n[feature]\nname = \"Feature Name\"\ndescription = \"\"\n"
        ),
        Kind::Item => format!(
            "id = \"{id}\"\nname = \"{name}\"\nkind = \"weapon\"   # weapon | armor | shield | gear | consumable | wondrous | tool\ncost = \"15 gp\"\nweight = 3.0\nrarity = \"common\"\nrequires_attunement = false\ndescription = \"\"\n\n[weapon]\ndamage = \"1d8\"\ndamage_type = \"slashing\"\ncategory = \"martial\"   # simple | martial\nkind = \"melee\"         # melee | ranged\nproperties = [\"versatile\"]\nversatile = \"1d10\"\nbonus = 0\n\n# [armor]\n# base_ac = 14\n# category = \"medium\"\n# dex_cap = 2\n# strength_requirement = 0\n# stealth_disadvantage = false\n\n# [[modifiers]]      # applied while equipped\n# target = \"ac\"\n# op = \"add\"\n# value = \"1\"\n"
        ),
        Kind::Feat => format!(
            "id = \"{id}\"\nname = \"{name}\"\nprerequisite = \"\"\ndescription = \"\"\n\n[[modifiers]]\ntarget = \"hp.max\"\nop = \"add\"\nvalue = \"2 * level\"\n"
        ),
        Kind::Spell => format!(
            "id = \"{id}\"\nname = \"{name}\"\nlevel = 1\nschool = \"evocation\"\ncasting_time = \"1 action\"\nrange = \"120 feet\"\ncomponents = \"V, S\"\nduration = \"Instantaneous\"\nconcentration = false\nritual = false\nclasses = [\"wizard\"]\ndamage = \"3d8\"\ndamage_type = \"fire\"\nsave = \"\"\nattack = true\ndescription = \"\"\nhigher_levels = \"\"\n\n# [area]\n# shape = \"sphere\"   # sphere | cube | cone | line | cylinder\n# size = 20\n"
        ),
        Kind::Creature => format!(
            "id = \"{id}\"\nname = \"{name}\"\nsize = \"Medium\"\nkind = \"humanoid\"\nalignment = \"neutral\"\nac = 13\nac_note = \"leather armor\"\nhp = 11\nhp_formula = \"2d8 + 2\"\ncr = \"1/4\"\nxp = 50\nsenses = [\"darkvision 60 ft.\", \"passive Perception 10\"]\nlanguages = [\"Common\"]\ndescription = \"\"\n\n[speed]\nwalk = 30\n\n[abilities]\nstr = 12\ndex = 14\ncon = 12\nint = 10\nwis = 10\ncha = 8\n\n[skills]\nstealth = 4\n\n[[traits]]\nname = \"Nimble Escape\"\ndescription = \"Can take the Disengage or Hide action as a bonus action.\"\n\n[[actions]]\nname = \"Scimitar\"\ndescription = \"Melee Weapon Attack.\"\nattack = {{ to_hit = 4, reach = \"reach 5 ft.\", damage = \"1d6 + 2\", damage_type = \"slashing\" }}\n"
        ),
    }
}
