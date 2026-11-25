//! # dnd-core
//!
//! Rules engine and data model for the dnd-dm suite. Everything a human can
//! author lives in TOML (see `docs/CONTENT_FORMAT.md`); this crate loads it,
//! derives character sheets, rolls dice, measures distances on a grid and
//! saves whole game states.
//!
//! The crate is UI-free so the same code backs the Windows DM app, the LAN
//! server and (later) the Android player app.

pub mod api;
pub mod builder;
pub mod export;
pub mod expr;
pub mod grid;
pub mod library;
pub mod mapgen;
pub mod model;
pub mod rng;
pub mod save;
pub mod sheet;
pub mod view;
pub mod vision;

pub use library::Library;
pub use model::*;
pub use rng::Rng;
pub use sheet::{derive, Sheet};

/// Everything the DM app and the server share: loaded content plus the
/// current campaign.
#[derive(Debug, Default)]
pub struct World {
    pub library: Library,
    pub campaign: Campaign,
    pub content_dirs: Vec<std::path::PathBuf>,
    pub rng: Rng,
    /// Incremented on every change so clients can poll cheaply.
    pub revision: u64,
    /// When each player device last talked to the server (not saved).
    pub presence: std::collections::HashMap<String, std::time::Instant>,
}

/// A device counts as connected if it checked in this recently.
pub const PRESENCE_WINDOW: std::time::Duration = std::time::Duration::from_secs(45);

impl World {
    pub fn new(content_dirs: Vec<std::path::PathBuf>) -> Self {
        let library = Library::load_packs(&content_dirs);
        Self { library, campaign: Campaign::new("New Campaign"), content_dirs, rng: Rng::from_time(), revision: 1, presence: Default::default() }
    }

    /// Note that a player device is alive right now.
    pub fn seen(&mut self, device_id: &str) {
        self.presence.insert(device_id.to_string(), std::time::Instant::now());
    }

    pub fn device_online(&self, device_id: &str) -> bool {
        self.presence.get(device_id).map(|t| t.elapsed() < PRESENCE_WINDOW).unwrap_or(false)
    }

    pub fn reload_content(&mut self) {
        self.library = Library::load_packs(&self.content_dirs);
        self.touch();
    }

    pub fn touch(&mut self) {
        self.revision += 1;
    }

    pub fn sheet(&self, character_id: &str) -> Option<Sheet> {
        self.campaign.character(character_id).map(|c| derive(c, &self.library))
    }

    /// Build a combatant for a library creature.
    pub fn combatant_from_creature(&mut self, template: &str, label: Option<&str>) -> Option<Combatant> {
        let c = self.library.creatures.get(template)?.clone();
        let name = label.map(|s| s.to_string()).unwrap_or_else(|| c.name.clone());
        let id = self.campaign.encounter.unique_id(&name);
        let hp = if c.hp > 0 { c.hp } else if !c.hp_formula.is_empty() {
            expr::eval(&c.hp_formula, &expr::Env::new(), expr::Mode::Average, None).map(|r| r.int() as i32).unwrap_or(1)
        } else {
            1
        };
        Some(Combatant {
            id,
            name,
            kind: CombatantKind::Creature { template: template.to_string() },
            hp,
            hp_max: hp,
            ac: c.ac,
            size: c.size.clone(),
            initiative: 0,
            ..Default::default()
        })
    }

    /// Build a combatant for a player character using the derived sheet.
    pub fn combatant_from_character(&self, character_id: &str) -> Option<Combatant> {
        let ch = self.campaign.character(character_id)?;
        let s = derive(ch, &self.library);
        Some(Combatant {
            id: self.campaign.encounter.unique_id(&ch.id),
            name: ch.name.clone(),
            kind: CombatantKind::Pc { id: ch.id.clone() },
            hp: s.hp_current,
            hp_max: s.hp_max,
            temp_hp: s.hp_temp,
            ac: s.ac,
            size: s.size.clone(),
            initiative: 0,
            darkvision: s.darkvision.max(0) as u32,
            ..Default::default()
        })
    }

    /// Roll a formula with the world RNG.
    pub fn roll(&mut self, formula: &str) -> Result<expr::EvalResult, expr::ExprError> {
        expr::roll(formula, &expr::Env::new(), &mut self.rng)
    }

    /// Roll initiative for every combatant that is at 0.
    pub fn roll_initiative(&mut self, only_zero: bool) {
        let mut rolls = Vec::new();
        for c in &self.campaign.encounter.combatants {
            if only_zero && c.initiative != 0 {
                continue;
            }
            let bonus = match &c.kind {
                CombatantKind::Pc { id } => self.sheet(id).map(|s| s.initiative).unwrap_or(0),
                CombatantKind::Creature { template } => {
                    self.library.creatures.get(template).map(|cr| cr.abilities.modifier("dex")).unwrap_or(0)
                }
                CombatantKind::Custom => 0,
            };
            rolls.push((c.id.clone(), bonus));
        }
        for (id, bonus) in rolls {
            let d = self.rng.roll_die(20) as i32;
            if let Some(c) = self.campaign.encounter.find_mut(&id) {
                c.initiative = d + bonus;
            }
            self.campaign.log(format!("{id} rolls initiative: {d} {} = {}", expr::fmt_bonus(bonus as i64), d + bonus));
        }
        self.campaign.encounter.sort_by_initiative();
        self.touch();
    }
}
