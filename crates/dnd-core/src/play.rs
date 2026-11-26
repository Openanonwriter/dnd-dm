//! Things that happen at the table: rolls everyone sees, attacks and spells
//! against a target, turns, rests, death saves, choosing spells. Every one
//! of them is decided here, on the DM's machine, and told to the table
//! through [`Campaign::say`], so no app has to be trusted with the dice.

use crate::expr::{self, Env};
use crate::library::Library;
use crate::model::*;
use crate::range::{InRange, Reach};
use crate::sheet::{Sheet, SpellcastingBlock};
use crate::World;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RollMode {
    #[default]
    Normal,
    Advantage,
    Disadvantage,
}

/// What an action came to, in words for the table and in numbers.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ActionResult {
    pub text: String,
    #[serde(default)]
    pub hit: Option<bool>,
    #[serde(default)]
    pub total: i64,
    #[serde(default)]
    pub damage: i32,
}

/// Multiply the number of dice in every `NdS` of a formula ("1d8 + 3" x2 ->
/// "2d8 + 3"): critical hits, cantrips growing with level, bigger slots.
pub fn mul_dice(formula: &str, k: u32) -> String {
    let b: Vec<char> = formula.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < b.len() {
        let start = i;
        let mut j = i;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        let in_word = start > 0 && (b[start - 1].is_alphanumeric() || b[start - 1] == '_' || b[start - 1] == '.');
        let is_die = !in_word && j + 1 < b.len() && (b[j] == 'd' || b[j] == 'D') && b[j + 1].is_ascii_digit();
        if is_die {
            let n: u32 = if j > start { b[start..j].iter().collect::<String>().parse().unwrap_or(1) } else { 1 };
            out.push_str(&(n * k).to_string());
            out.push('d');
            i = j + 1;
            while i < b.len() && b[i].is_ascii_digit() {
                out.push(b[i]);
                i += 1;
            }
        } else if j > start {
            out.extend(&b[start..j]);
            i = j;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    out
}

/// How far a spell reaches for this caster: its own range, stretched by
/// what the sheet says, and far enough to catch the edge of its area.
pub fn spell_reach(spell: &Spell, percent: u32) -> Reach {
    let mut r = Reach::parse(&spell.range);
    let from_self = spell.range.trim().to_lowercase().starts_with("self");
    if from_self {
        if let (true, Some(a)) = (r.self_only, &spell.area) {
            r = Reach { normal: a.size, long: a.size, ..Default::default() };
        }
        return r;
    }
    r = r.scaled(if percent == 0 { 100 } else { percent });
    match &spell.area {
        Some(a) => r.widened(a.size),
        None => r,
    }
}

/// How many times a cantrip's dice are rolled at this character level.
pub fn cantrip_dice(level: u32) -> u32 {
    1 + (level >= 5) as u32 + (level >= 11) as u32 + (level >= 17) as u32
}

/// True when the spell is on the list of this class (by id or shared list).
pub fn on_class_list(lib: &Library, class: &str, spell: &Spell) -> bool {
    if spell.classes.iter().any(|c| c == class) {
        return true;
    }
    let lists = lib.classes.get(class).and_then(|c| c.spellcasting.as_ref()).map(|s| s.spell_lists.clone()).unwrap_or_default();
    spell.classes.iter().any(|c| lists.contains(c))
}

/// True when the spell is on one of the lists this spellcasting block draws
/// from: its class (and that class's `spell_lists`) or its subclass, so a
/// patron's expanded spells name the subclass id in `classes`.
pub fn on_list(lib: &Library, block: &SpellcastingBlock, spell: &Spell) -> bool {
    on_class_list(lib, &block.class, spell) || spell.classes.iter().any(|c| block.lists.contains(c))
}

/// The spellcasting block a character would cast this spell with.
pub fn casting_block<'a>(lib: &Library, sheet: &'a Sheet, spell: &Spell) -> Option<&'a SpellcastingBlock> {
    sheet.spellcasting.iter().find(|b| on_list(lib, b, spell)).or(sheet.spellcasting.first())
}

/// Spells a character could add to the ones they know, and how many picks
/// are left: (cantrips left, spells left, ids to choose from). A class that
/// prepares from its whole list (cleric) only ever picks cantrips.
pub fn spell_picks(lib: &Library, ch: &Character, sheet: &Sheet) -> (u32, u32, Vec<String>) {
    let known_cantrips = ch.spells.known.iter().filter(|id| lib.spells.get(*id).map(|s| s.level == 0).unwrap_or(false)).count() as u32;
    let known_spells = ch.spells.known.len() as u32 - known_cantrips;
    let cantrip_max: u32 = sheet.spellcasting.iter().map(|b| b.cantrips_known).sum();
    let spell_max: u32 = sheet.spellcasting.iter().filter_map(|b| b.spell_limit).sum();
    let cantrips_left = cantrip_max.saturating_sub(known_cantrips);
    let spells_left = spell_max.saturating_sub(known_spells);
    let mut options = Vec::new();
    for s in lib.spells.values().filter(|s| !s.hidden && !ch.spells.known.contains(&s.id) && !sheet.spells_granted.contains(&s.id)) {
        let ok = sheet.spellcasting.iter().any(|b| on_list(lib, b, s) && if s.level == 0 { cantrips_left > 0 } else { b.spell_limit.is_some() && spells_left > 0 && s.level <= b.max_spell_level });
        if ok {
            options.push(s.id.clone());
        }
    }
    (cantrips_left, spells_left, options)
}

/// Spells a character may prepare or cast: the ones they know, the ones
/// their features grant (up to the highest level they can cast), plus the
/// whole class list for classes that prepare from it.
pub fn spells_available(lib: &Library, ch: &Character, sheet: &Sheet) -> Vec<String> {
    let mut v = ch.spells.known.clone();
    let top = sheet.spellcasting.iter().map(|b| b.max_spell_level).max();
    for id in &sheet.spells_granted {
        let ok = lib.spells.get(id).map(|s| !s.hidden && (s.level == 0 || top.map(|t| s.level <= t).unwrap_or(true))).unwrap_or(false);
        if ok && !v.contains(id) {
            v.push(id.clone());
        }
    }
    for s in lib.spells.values().filter(|s| !s.hidden && s.level > 0) {
        if sheet.spellcasting.iter().any(|b| b.spell_limit.is_none() && s.level <= b.max_spell_level && on_list(lib, b, s)) && !v.contains(&s.id) {
            v.push(s.id.clone());
        }
    }
    v
}

/// Set the spells a character knows to `wanted`, if the rules allow it.
/// Spells the DM already gave stay legal; with `add_only` (a game is
/// running) nothing may be dropped.
pub fn choose_spells(lib: &Library, ch: &mut Character, wanted: &[String], add_only: bool) -> Result<(), String> {
    let sheet = crate::sheet::derive(ch, lib);
    if sheet.spellcasting.is_empty() {
        return if wanted.is_empty() { Ok(()) } else { Err("this character does not cast spells".into()) };
    }
    let mut list: Vec<String> = Vec::new();
    let (mut cantrips, mut leveled) = (0u32, 0u32);
    for id in wanted {
        if list.contains(id) {
            continue;
        }
        let spell = lib.spells.get(id).filter(|s| !s.hidden).ok_or_else(|| format!("unknown spell '{id}'"))?;
        if !ch.spells.known.contains(id) {
            let ok = sheet.spellcasting.iter().any(|b| on_list(lib, b, spell) && (spell.level == 0 || (b.spell_limit.is_some() && spell.level <= b.max_spell_level)));
            if !ok {
                return Err(format!("{} is not a spell this character can learn yet", spell.name));
            }
        }
        if spell.level == 0 {
            cantrips += 1;
        } else {
            leveled += 1;
        }
        list.push(id.clone());
    }
    let had_cantrips = ch.spells.known.iter().filter(|id| lib.spells.get(*id).map(|s| s.level == 0).unwrap_or(false)).count() as u32;
    let had_leveled = ch.spells.known.len() as u32 - had_cantrips;
    let cantrip_max: u32 = sheet.spellcasting.iter().map(|b| b.cantrips_known).sum();
    let spell_max: u32 = sheet.spellcasting.iter().filter_map(|b| b.spell_limit).sum();
    if cantrips > cantrip_max.max(had_cantrips) {
        return Err(format!("too many cantrips: this character knows {cantrip_max}"));
    }
    if leveled > spell_max.max(had_leveled) {
        return Err(format!("too many spells: this character can have {spell_max}"));
    }
    if add_only && ch.spells.known.iter().any(|k| !list.contains(k)) {
        return Err("during a game you can add spells but not swap them".into());
    }
    ch.spells.known = list;
    let available = spells_available(lib, ch, &sheet);
    ch.spells.prepared.retain(|p| available.contains(p));
    Ok(())
}

impl World {
    fn d20(&mut self, mode: RollMode) -> (i32, String) {
        let a = self.rng.roll_die(20) as i32;
        match mode {
            RollMode::Normal => (a, a.to_string()),
            RollMode::Advantage => {
                let b = self.rng.roll_die(20) as i32;
                (a.max(b), format!("{} (advantage: {a}, {b})", a.max(b)))
            }
            RollMode::Disadvantage => {
                let b = self.rng.roll_die(20) as i32;
                (a.min(b), format!("{} (disadvantage: {a}, {b})", a.min(b)))
            }
        }
    }

    /// The combatant that stands for this character in the encounter.
    pub fn pc_combatant(&self, character_id: &str) -> Option<String> {
        self.campaign.encounter.combatants.iter().find(|c| matches!(&c.kind, CombatantKind::Pc { id } if id == character_id)).map(|c| c.id.clone())
    }

    /// Copy a character's hit points and conditions onto its token.
    pub fn sync_combatant_from_pc(&mut self, character_id: &str) {
        let Some(s) = self.sheet(character_id) else { return };
        let Some(cid) = self.pc_combatant(character_id) else { return };
        let conditions = self.campaign.character(character_id).map(|c| c.conditions.clone()).unwrap_or_default();
        if let Some(c) = self.campaign.encounter.find_mut(&cid) {
            c.hp = s.hp_current;
            c.hp_max = s.hp_max;
            c.temp_hp = s.hp_temp;
            c.ac = s.ac;
            c.conditions = conditions;
        }
    }

    /// Copy a token's hit points back to the character sheet it stands for.
    pub fn sync_pc_from_combatant(&mut self, combatant_id: &str) {
        let Some(c) = self.campaign.encounter.find(combatant_id).cloned() else { return };
        if let CombatantKind::Pc { id } = &c.kind {
            if let Some(ch) = self.campaign.character_mut(id) {
                ch.hp.current = c.hp;
                ch.hp.temp = c.temp_hp;
            }
        }
    }

    fn save_bonus(&self, combatant_id: &str, ability: &str) -> i32 {
        let Some(c) = self.campaign.encounter.find(combatant_id) else { return 0 };
        match &c.kind {
            CombatantKind::Pc { id } => self.sheet(id).and_then(|s| s.saves.iter().find(|x| x.key == ability).map(|x| x.bonus)).unwrap_or(0),
            CombatantKind::Creature { template } => self.library.creatures.get(template).map(|cr| cr.save_bonus(ability)).unwrap_or(0),
            CombatantKind::Custom => 0,
        }
    }

    /// Take hit points off a combatant, minding what its kind shrugs off.
    /// Returns the hit points lost and a note for the table.
    pub fn deal_damage(&mut self, target: &str, amount: i32, damage_type: &str) -> (i32, String) {
        let Some(c) = self.campaign.encounter.find(target).cloned() else { return (0, String::new()) };
        let dt = damage_type.trim().to_lowercase();
        let mut amount = amount.max(0);
        let mut note = String::new();
        if let CombatantKind::Creature { template } = &c.kind {
            if let Some(cr) = self.library.creatures.get(template) {
                let has = |list: &Vec<String>| !dt.is_empty() && list.iter().any(|x| x.trim().to_lowercase() == dt);
                if has(&cr.damage_immunities) {
                    amount = 0;
                    note = " (immune)".into();
                } else if has(&cr.damage_resistances) {
                    amount /= 2;
                    note = " (resisted, halved)".into();
                } else if has(&cr.damage_vulnerabilities) {
                    amount *= 2;
                    note = " (vulnerable, doubled)".into();
                }
            }
        }
        let mut lost = 0;
        let mut down = false;
        if let Some(cm) = self.campaign.encounter.find_mut(target) {
            let was_up = cm.hp > 0;
            lost = cm.damage(amount);
            down = was_up && cm.hp <= 0;
            if amount > 0 && !cm.concentrating_on.is_empty() {
                note.push_str(&format!(" · concentration save DC {}", (amount / 2).max(10)));
            }
        }
        self.sync_pc_from_combatant(target);
        if down {
            note.push_str(&format!(" · {} is down!", c.name));
        }
        (lost, note)
    }

    /// Give hit points back to a combatant (never above its maximum).
    pub fn heal(&mut self, target: &str, amount: i32) -> i32 {
        let mut got = 0;
        if let Some(cm) = self.campaign.encounter.find_mut(target) {
            let before = cm.hp;
            cm.hp = (cm.hp + amount.max(0)).min(cm.hp_max.max(cm.hp));
            got = cm.hp - before;
        }
        self.sync_pc_from_combatant(target);
        if let Some(CombatantKind::Pc { id }) = self.campaign.encounter.find(target).map(|c| c.kind.clone()) {
            if got > 0 {
                if let Some(ch) = self.campaign.character_mut(&id) {
                    ch.death_saves = Default::default();
                }
            }
        }
        got
    }

    fn target_name(&self, id: &str) -> Result<(String, i32), String> {
        let c = self.campaign.encounter.find(id).ok_or("that target is not in the encounter")?;
        Ok((c.name.clone(), c.ac))
    }

    /// A combatant that is out cold or held cannot attack or cast.
    fn can_act(&self, cid: &str) -> Result<(), String> {
        let Some(c) = self.campaign.encounter.find(cid) else { return Ok(()) };
        if c.hp <= 0 && c.hp_max > 0 {
            return Err(format!("{} is down", c.name));
        }
        for bad in ["incapacitated", "paralyzed", "petrified", "stunned", "unconscious"] {
            if c.conditions.iter().any(|x| x.eq_ignore_ascii_case(bad)) {
                return Err(format!("{} is {bad} and cannot act", c.name));
            }
        }
        Ok(())
    }

    /// Walking speed of a combatant in feet, as things stand right now.
    pub fn speed_of(&self, cid: &str) -> u32 {
        let Some(c) = self.campaign.encounter.find(cid) else { return 30 };
        for held in ["grappled", "restrained", "paralyzed", "petrified", "stunned", "unconscious"] {
            if c.conditions.iter().any(|x| x.eq_ignore_ascii_case(held)) {
                return 0;
            }
        }
        match &c.kind {
            // a sheet with no species yet has no speed: walk like anyone else
            CombatantKind::Pc { id } => self.sheet(id).map(|s| s.speed.max(0) as u32).filter(|s| *s > 0).unwrap_or(30),
            // a creature that only flies or swims moves at its best speed
            CombatantKind::Creature { template } => {
                self.library.creatures.get(template).and_then(|cr| cr.speed.get("walk").copied().filter(|s| *s > 0).or(cr.speed.values().copied().max())).filter(|s| *s > 0).unwrap_or(30)
            }
            CombatantKind::Custom => 30,
        }
    }

    /// Feet a combatant can still walk this turn; None when it is not its
    /// turn in a fight (no budget applies).
    pub fn move_budget(&self, cid: &str) -> Option<u32> {
        let e = &self.campaign.encounter;
        if !e.active {
            return None;
        }
        let c = e.current().filter(|c| c.id == cid)?;
        Some((self.speed_of(cid) * if c.dashed { 2 } else { 1 }).saturating_sub(c.moved))
    }

    /// A player walks their own token: on their turn in a fight, as far as
    /// the movement they have left; a good stretch at a time otherwise.
    pub fn player_walk(&mut self, cid: &str, to: crate::grid::Pos) -> Result<u32, String> {
        if !self.campaign.settings.players_move_tokens {
            return Err("the DM moves the tokens in this game".into());
        }
        let fps = self.campaign.map.feet_per_square.max(1);
        let in_fight = self.campaign.encounter.active;
        let enforce = self.campaign.settings.enforce_range;
        let budget = if in_fight {
            if self.campaign.encounter.current().map(|c| c.id != cid).unwrap_or(true) {
                return Err("wait for your turn".into());
            }
            if enforce {
                self.move_budget(cid).unwrap_or(0)
            } else {
                self.speed_of(cid).max(fps)
            }
        } else {
            match self.campaign.settings.player_move_limit {
                0 => self.speed_of(cid) * 4,
                n => n,
            }
        };
        if budget < fps {
            return Err(if self.speed_of(cid) == 0 { "you cannot move right now".to_string() } else { "no movement left this turn (Dash gives more)".to_string() });
        }
        let feet = self.move_token(cid, to, true, budget / fps).map_err(|e| if in_fight && e.contains("cannot get there") { format!("too far or blocked: you have {budget} ft of movement left") } else { e })?;
        if in_fight {
            if let Some(c) = self.campaign.encounter.find_mut(cid) {
                c.moved += feet;
            }
        }
        Ok(feet)
    }

    /// The Dash action: twice the movement this turn.
    pub fn dash(&mut self, character_id: &str) -> Result<(), String> {
        let cid = self.pc_combatant(character_id).ok_or("you are not in the fight")?;
        if !self.campaign.encounter.active || self.campaign.encounter.current().map(|c| c.id != cid).unwrap_or(true) {
            return Err("it is not your turn".into());
        }
        self.can_act(&cid)?;
        let c = self.campaign.encounter.find_mut(&cid).unwrap();
        if c.dashed {
            return Err("you already dashed this turn".into());
        }
        c.dashed = true;
        let name = c.name.clone();
        self.campaign.say(&name, "dashes", "turn");
        self.touch();
        Ok(())
    }

    fn fresh_turn(&mut self) {
        let turn = self.campaign.encounter.turn;
        if let Some(c) = self.campaign.encounter.combatants.get_mut(turn) {
            c.moved = 0;
            c.dashed = false;
        }
    }

    /// Can `attacker` reach `target` with this, and how does the d20 go?
    /// Checks distance (reach, normal and long range), walls in the way,
    /// an enemy next to a ranged attacker, and the conditions on both.
    /// Returns the roll mode to use, a note for the table, and whether a
    /// hit is an automatic critical. Without positions on the map only the
    /// conditions count.
    pub fn engage(&self, attacker: &str, target: &str, reach: &Reach, what: &str, is_attack: bool, mode: RollMode) -> Result<(RollMode, String, bool), String> {
        self.can_act(attacker)?;
        let e = &self.campaign.encounter;
        let (Some(a), Some(t)) = (e.find(attacker), e.find(target)) else { return Ok((mode, String::new(), false)) };
        let rules = &self.library.rules;
        let rule = self.campaign.settings.distance_rule;
        let own = attacker == target;
        let dist = if own { Some(0) } else { crate::grid::combatant_distance_ft(a, t, rules, rule) };
        let has = |c: &Combatant, k: &str| c.conditions.iter().any(|x| x.eq_ignore_ascii_case(k));
        let (mut adv, mut dis): (Vec<String>, Vec<String>) = (Vec::new(), Vec::new());
        let mut melee = reach.melee > 0 && reach.normal == 0;
        if let (Some(d), true, false) = (dist, self.campaign.settings.enforce_range, own) {
            if reach.self_only {
                return Err(format!("{what} only affects {}", a.name));
            }
            match reach.judge(d.max(0) as u32) {
                None => return Err(format!("{} is {d} ft away; {what} reaches {}", t.name, reach.describe())),
                Some(InRange::Melee) => melee = true,
                Some(InRange::Ranged) => melee = false,
                Some(InRange::Long) => {
                    melee = false;
                    if is_attack {
                        dis.push("long range".into());
                    }
                }
            }
            let map = &self.campaign.map;
            let centre = |c: &Combatant| c.pos.map(|p| (p.x as f32 + rules.size_squares(&c.size).max(1) as f32 / 2.0, p.y as f32 + rules.size_squares(&c.size).max(1) as f32 / 2.0));
            if map.width > 0 && !map.cells.is_empty() {
                if let (Some(p), Some(q)) = (centre(a), centre(t)) {
                    if !crate::vision::los(p, q, &map.sight_blockers()) {
                        return Err(format!("there is no clear line from {} to {}", a.name, t.name));
                    }
                }
            }
            if is_attack && !melee {
                let pc = |c: &Combatant| matches!(c.kind, CombatantKind::Pc { .. });
                let crowded = e.combatants.iter().any(|o| {
                    o.id != a.id && o.hp > 0 && o.visible && pc(o) != pc(a) && !has(o, "incapacitated") && !has(o, "unconscious") && crate::grid::combatant_distance_ft(a, o, rules, rule).map(|x| x <= 5).unwrap_or(false)
                });
                if crowded {
                    dis.push("an enemy within 5 ft".into());
                }
            }
        }
        let mut auto_crit = false;
        if is_attack && !own {
            let close = dist.map(|d| d <= 5).unwrap_or(melee);
            for k in ["blinded", "poisoned", "frightened", "restrained", "prone"] {
                if has(a, k) {
                    dis.push(k.to_string());
                }
            }
            if has(a, "invisible") {
                adv.push("unseen attacker".into());
            }
            for k in ["blinded", "paralyzed", "petrified", "restrained", "stunned", "unconscious"] {
                if has(t, k) {
                    adv.push(format!("target {k}"));
                }
            }
            if has(t, "invisible") {
                dis.push("target invisible".into());
            }
            if has(t, "prone") {
                if close {
                    adv.push("target prone".into());
                } else {
                    dis.push("target prone, at a distance".into());
                }
            }
            auto_crit = close && (has(t, "paralyzed") || has(t, "unconscious"));
        }
        let mut note = String::new();
        if !adv.is_empty() {
            note.push_str(&format!(" (advantage: {})", adv.join(", ")));
        }
        if !dis.is_empty() {
            note.push_str(&format!(" (disadvantage: {})", dis.join(", ")));
        }
        let up = !adv.is_empty() || mode == RollMode::Advantage;
        let down = !dis.is_empty() || mode == RollMode::Disadvantage;
        let mode = match (up, down) {
            (true, false) => RollMode::Advantage,
            (false, true) => RollMode::Disadvantage,
            _ => RollMode::Normal,
        };
        Ok((mode, note, auto_crit))
    }

    /// One attack roll against a combatant's armor class, damage included.
    #[allow(clippy::too_many_arguments)]
    fn resolve_attack(&mut self, attacker: Option<&str>, reach: &Reach, what: &str, bonus: i32, damage: &str, damage_type: &str, target: &str, mode: RollMode, env: &Env, apply: bool) -> Result<ActionResult, String> {
        let (tname, ac) = self.target_name(target)?;
        let (mode, note, auto_crit) = match attacker {
            Some(a) => self.engage(a, target, reach, what, true, mode)?,
            None => (mode, String::new(), false),
        };
        let (d, dtext) = self.d20(mode);
        let total = d + bonus;
        let hit = d == 20 || (d != 1 && total >= ac);
        let crit = d == 20 || (hit && auto_crit);
        let mut text = format!("{what} at {tname}: {dtext} {} = {total}{note}", expr::fmt_bonus(bonus as i64));
        let mut dealt = 0;
        if hit {
            let formula = if crit { mul_dice(damage, 2) } else { damage.to_string() };
            let amount = if formula.trim().is_empty() { 0 } else { expr::roll(&formula, env, &mut self.rng).map_err(|e| format!("damage '{formula}': {e}"))?.int().max(0) as i32 };
            text.push_str(if crit { " · CRITICAL HIT" } else { " · hit" });
            if amount > 0 {
                if apply {
                    let (_, note) = self.deal_damage(target, amount, damage_type);
                    text.push_str(&format!(" for {amount} {damage_type}{note}"));
                } else {
                    text.push_str(&format!(" for {amount} {damage_type} (the DM applies it)"));
                }
                dealt = amount;
            }
        } else {
            text.push_str(if d == 1 { " · critical miss" } else { " · miss" });
        }
        Ok(ActionResult { text, hit: Some(hit), total: total as i64, damage: dealt })
    }

    /// A formula rolled where everyone sees it.
    pub fn table_roll(&mut self, who: &str, formula: &str, label: &str, mode: RollMode) -> Result<ActionResult, String> {
        let f = match mode {
            RollMode::Normal => formula.to_string(),
            RollMode::Advantage => formula.replacen("1d20", "2d20kh1", 1),
            RollMode::Disadvantage => formula.replacen("1d20", "2d20kl1", 1),
        };
        let r = expr::roll(&f, &Env::new(), &mut self.rng).map_err(|e| e.to_string())?;
        let dice: Vec<String> = r.rolls.iter().map(|d| d.to_string()).collect();
        let mut text = if label.is_empty() { format!("{f} = {}", r.int()) } else { format!("{label}: {f} = {}", r.int()) };
        if !dice.is_empty() {
            text.push_str(&format!("  [{}]", dice.join(", ")));
        }
        self.campaign.say(who, text.clone(), "roll");
        self.touch();
        Ok(ActionResult { text, hit: None, total: r.int(), damage: 0 })
    }

    /// A player character attacks with one of the attacks on its sheet.
    pub fn pc_attack(&mut self, character_id: &str, attack: usize, target: &str, mode: RollMode) -> Result<ActionResult, String> {
        let sheet = self.sheet(character_id).ok_or("no such character")?;
        let a = sheet.attacks.get(attack).ok_or("no such attack")?.clone();
        if !self.campaign.encounter.find(target).map(|c| c.visible).unwrap_or(false) {
            return Err("you cannot see that target".into());
        }
        let apply = self.campaign.settings.players_apply_damage;
        let me = self.pc_combatant(character_id);
        let reach = if a.reach == Reach::default() { Reach::parse(&a.range) } else { a.reach };
        let r = self.resolve_attack(me.as_deref(), &reach, &a.name, a.bonus, &a.damage, &a.damage_type, target, mode, &Env::new(), apply)?;
        self.campaign.say(&sheet.name, r.text.clone(), "attack");
        self.touch();
        Ok(r)
    }

    /// A creature in the encounter uses one of its attack actions.
    pub fn creature_attack(&mut self, attacker: &str, action: usize, target: &str, mode: RollMode) -> Result<ActionResult, String> {
        let c = self.campaign.encounter.find(attacker).ok_or("no such combatant")?.clone();
        let CombatantKind::Creature { template } = &c.kind else { return Err("only creatures have stat-block attacks".into()) };
        let cr = self.library.creatures.get(template).ok_or("creature not in the library")?;
        let act = cr.actions.get(action).ok_or("no such action")?.clone();
        let atk = act.attack.ok_or("that action is not an attack")?;
        let r = self.resolve_attack(Some(attacker), &Reach::parse(&atk.reach), &act.name, atk.to_hit, &atk.damage, &atk.damage_type, target, mode, &Env::new(), true)?;
        let who = if c.visible { c.name.clone() } else { "Something unseen".to_string() };
        self.campaign.say(&who, r.text.clone(), "attack");
        self.touch();
        Ok(r)
    }

    /// A player character casts a spell. `slot` is the slot level spent
    /// (0 for a cantrip, or for a ritual cast without a slot).
    pub fn pc_cast(&mut self, character_id: &str, spell_id: &str, slot: u32, targets: &[String], mode: RollMode) -> Result<ActionResult, String> {
        let sheet = self.sheet(character_id).ok_or("no such character")?;
        let ch = self.campaign.character(character_id).ok_or("no such character")?.clone();
        let spell = self.library.spells.get(spell_id).filter(|s| !s.hidden).ok_or("no such spell")?.clone();
        let block = casting_block(&self.library, &sheet, &spell).ok_or("this character does not cast spells")?.clone();
        let granted = sheet.spells_granted.contains(&spell.id);
        let known = ch.spells.known.contains(&spell.id) || granted;
        if spell.level == 0 {
            if !known {
                return Err(format!("{} does not know {}", ch.name, spell.name));
            }
        } else {
            if !spells_available(&self.library, &ch, &sheet).contains(&spell.id) {
                return Err(format!("{} does not know {}", ch.name, spell.name));
            }
            if block.prepared_max.is_some() && !granted && !ch.spells.prepared.contains(&spell.id) && !(spell.ritual && slot == 0) {
                return Err(format!("{} is not prepared", spell.name));
            }
        }
        let me = self.pc_combatant(character_id);
        let reach = spell_reach(&spell, sheet.spell_range_pct);
        if let Some(a) = &me {
            self.can_act(a)?;
        }
        for t in targets {
            if !self.campaign.encounter.find(t).map(|c| c.visible).unwrap_or(false) {
                return Err("you cannot see one of those targets".into());
            }
            // out of range or behind a wall: say so before a slot is spent
            if let Some(a) = &me {
                self.engage(a, t, &reach, &spell.name, spell.attack, mode)?;
            }
        }
        // ---- pay for it
        let mut how = String::new();
        if spell.level > 0 {
            if slot == 0 {
                if !spell.ritual {
                    return Err("pick a spell slot".into());
                }
                how = " as a ritual".into();
            } else {
                if slot < spell.level || slot > 9 {
                    return Err(format!("{} needs a slot of level {} or higher", spell.name, spell.level));
                }
                let i = slot as usize - 1;
                let have = sheet.spell_slots.get(i).copied().unwrap_or(0);
                let used = ch.spells.slots_used.get(i).copied().unwrap_or(0);
                let pact = sheet.pact_slots.filter(|(n, l)| *l == slot && ch.spells.pact_slots_used < *n);
                let c = self.campaign.character_mut(character_id).unwrap();
                if used < have {
                    c.spells.slots_used.resize(9, 0);
                    c.spells.slots_used[i] += 1;
                } else if pact.is_some() {
                    c.spells.pact_slots_used += 1;
                } else {
                    return Err(format!("no level {slot} spell slot left"));
                }
                if slot > spell.level {
                    how = format!(" with a level {slot} slot");
                }
            }
        }
        let mut env = Env::new();
        env.set("mod", block.ability_mod as f64);
        env.set("slot", slot as f64);
        env.set("level", ch.total_level() as f64);
        let grow = |base: &str| -> String {
            if base.trim().is_empty() {
                return String::new();
            }
            if spell.level == 0 {
                return mul_dice(base, cantrip_dice(ch.total_level()));
            }
            if slot > spell.level && !spell.upcast.trim().is_empty() {
                return format!("{base} + {}", mul_dice(&spell.upcast, slot - spell.level));
            }
            base.to_string()
        };
        let damage = grow(&spell.damage);
        let healing = grow(&spell.heal);
        let apply = self.campaign.settings.players_apply_damage;
        let mut parts: Vec<String> = Vec::new();
        let mut total_damage = 0;
        let mut any_hit = None;

        if !healing.is_empty() {
            let amount = expr::roll(&healing, &env, &mut self.rng).map_err(|e| format!("healing '{healing}': {e}"))?.int().max(0) as i32;
            if targets.is_empty() {
                parts.push(format!("heals {amount}"));
            }
            for t in targets {
                let (tname, _) = self.target_name(t)?;
                if apply {
                    let got = self.heal(t, amount);
                    parts.push(format!("{tname} regains {got} hit points"));
                } else {
                    parts.push(format!("{tname} regains {amount} hit points (the DM applies it)"));
                }
            }
        } else if spell.attack {
            if targets.is_empty() {
                let (d, dtext) = self.d20(mode);
                let mut line = format!("spell attack {dtext} {} = {}", expr::fmt_bonus(block.attack_bonus as i64), d + block.attack_bonus);
                if !damage.is_empty() {
                    let amount = expr::roll(&damage, &env, &mut self.rng).map_err(|e| format!("damage '{damage}': {e}"))?.int();
                    line.push_str(&format!(" · {amount} {} on a hit", spell.damage_type));
                }
                parts.push(line);
            }
            for t in targets {
                let r = self.resolve_attack(me.as_deref(), &reach, "spell attack", block.attack_bonus, &damage, &spell.damage_type, t, mode, &env, apply)?;
                total_damage += r.damage;
                any_hit = Some(any_hit.unwrap_or(false) || r.hit.unwrap_or(false));
                parts.push(r.text);
            }
        } else if !spell.save.is_empty() {
            let ability = spell.save.trim().to_lowercase();
            let amount = if damage.is_empty() { 0 } else { expr::roll(&damage, &env, &mut self.rng).map_err(|e| format!("damage '{damage}': {e}"))?.int().max(0) as i32 };
            let save_name = crate::sheet::ability_name(&ability);
            if targets.is_empty() {
                parts.push(if amount > 0 { format!("{save_name} save DC {} · {amount} {}", block.save_dc, spell.damage_type) } else { format!("{save_name} save DC {}", block.save_dc) });
            }
            for t in targets {
                let (tname, _) = self.target_name(t)?;
                let bonus = self.save_bonus(t, &ability);
                let d = self.rng.roll_die(20) as i32;
                let saved = d + bonus >= block.save_dc;
                let mut line = format!("{tname} {} the {save_name} save ({})", if saved { "makes" } else { "fails" }, d + bonus);
                let take = if saved { if spell.save_half { amount / 2 } else { 0 } } else { amount };
                if amount > 0 {
                    if take > 0 && apply {
                        let (_, note) = self.deal_damage(t, take, &spell.damage_type);
                        line.push_str(&format!(": {take} {}{note}", spell.damage_type));
                    } else if take > 0 {
                        line.push_str(&format!(": {take} {} (the DM applies it)", spell.damage_type));
                    } else {
                        line.push_str(": no damage");
                    }
                    total_damage += take;
                }
                any_hit = Some(any_hit.unwrap_or(false) || !saved);
                parts.push(line);
            }
        } else if !damage.is_empty() {
            let amount = expr::roll(&damage, &env, &mut self.rng).map_err(|e| format!("damage '{damage}': {e}"))?.int().max(0) as i32;
            if targets.is_empty() {
                parts.push(format!("{amount} {}", spell.damage_type));
            }
            for t in targets {
                let (tname, _) = self.target_name(t)?;
                if apply {
                    let (_, note) = self.deal_damage(t, amount, &spell.damage_type);
                    parts.push(format!("{tname} takes {amount} {}{note}", spell.damage_type));
                } else {
                    parts.push(format!("{tname} takes {amount} {} (the DM applies it)", spell.damage_type));
                }
                total_damage += amount;
            }
        } else if !targets.is_empty() {
            let names: Vec<String> = targets.iter().filter_map(|t| self.campaign.encounter.find(t).map(|c| c.name.clone())).collect();
            parts.push(format!("on {}", names.join(", ")));
        }

        if spell.concentration {
            if let Some(c) = self.campaign.character_mut(character_id) {
                if !c.conditions.iter().any(|x| x == "concentrating") {
                    c.conditions.push("concentrating".into());
                }
            }
            if let Some(cid) = self.pc_combatant(character_id) {
                if let Some(cm) = self.campaign.encounter.find_mut(&cid) {
                    cm.concentrating_on = spell.name.clone();
                }
            }
            self.sync_combatant_from_pc(character_id);
        }
        let mut text = format!("casts {}{how}", spell.name);
        if !parts.is_empty() {
            text.push_str(" · ");
            text.push_str(&parts.join(" · "));
        }
        self.campaign.say(&ch.name, text.clone(), "spell");
        self.touch();
        Ok(ActionResult { text, hit: any_hit, total: 0, damage: total_damage })
    }

    /// Say whose turn it is now.
    pub fn announce_turn(&mut self) {
        let e = &self.campaign.encounter;
        if !e.active {
            return;
        }
        let who = match e.current() {
            Some(c) if c.visible => c.name.clone(),
            _ => "the DM".to_string(),
        };
        let round = e.round;
        self.campaign.say("", format!("Round {round} · {who}'s turn"), "turn");
    }

    /// Start the fight: sort by initiative, round 1.
    pub fn start_encounter(&mut self) {
        self.campaign.encounter.start();
        self.fresh_turn();
        self.campaign.say("", "Roll for initiative! The fight begins.", "turn");
        self.announce_turn();
        self.touch();
    }

    /// Move to the next combatant that can still act.
    pub fn next_turn(&mut self) {
        let n = self.campaign.encounter.combatants.len();
        for _ in 0..n.max(1) {
            self.campaign.encounter.next_turn();
            // the dead creatures are skipped; a dying character still rolls death saves
            let skip = self.campaign.encounter.current().map(|c| c.is_down() && !matches!(c.kind, CombatantKind::Pc { .. })).unwrap_or(false);
            if !skip {
                break;
            }
        }
        self.fresh_turn();
        self.announce_turn();
        self.touch();
    }

    /// A player says their turn is over.
    pub fn end_turn(&mut self, character_id: &str) -> Result<(), String> {
        if !self.campaign.encounter.active {
            return Err("no fight is running".into());
        }
        let mine = matches!(self.campaign.encounter.current().map(|c| &c.kind), Some(CombatantKind::Pc { id }) if id == character_id);
        if !mine {
            return Err("it is not your turn".into());
        }
        self.next_turn();
        Ok(())
    }

    /// A short or long rest for one character.
    pub fn rest(&mut self, character_id: &str, long: bool) -> Result<(), String> {
        let sheet = self.sheet(character_id).ok_or("no such character")?;
        let ch = self.campaign.character_mut(character_id).ok_or("no such character")?;
        if long {
            ch.hp.current = sheet.hp_max;
            ch.hp.temp = 0;
            ch.spells.slots_used.clear();
            ch.spells.pact_slots_used = 0;
            ch.death_saves = Default::default();
            ch.uses.clear();
            ch.exhaustion = ch.exhaustion.saturating_sub(1);
            ch.conditions.retain(|c| c != "concentrating" && c != "unconscious");
            // half of all hit dice come back, the biggest pool first
            let mut back = (sheet.level / 2).max(1);
            for (_, used) in ch.hp.hit_dice_used.iter_mut().rev() {
                let n = back.min(*used);
                *used -= n;
                back -= n;
            }
        } else {
            ch.spells.pact_slots_used = 0;
            let short: Vec<String> = sheet.features.iter().filter(|f| f.recharge.replace(' ', "_").to_lowercase().contains("short")).map(|f| f.name.clone()).collect();
            ch.uses.retain(|k, _| !short.contains(k));
        }
        let name = ch.name.clone();
        self.sync_combatant_from_pc(character_id);
        self.campaign.say(&name, if long { "finishes a long rest" } else { "takes a short rest" }, "rest");
        self.touch();
        Ok(())
    }

    /// Spend one hit die during a short rest: roll it, add Constitution.
    pub fn spend_hit_die(&mut self, character_id: &str, die: &str) -> Result<ActionResult, String> {
        let sheet = self.sheet(character_id).ok_or("no such character")?;
        let have = sheet.hit_dice.get(die).copied().ok_or("no such hit die")?;
        let sides: u32 = die.trim_start_matches('d').parse().map_err(|_| "no such hit die")?;
        let roll = self.rng.roll_die(sides) as i32;
        let con = sheet.abilities.modifier("con");
        let ch = self.campaign.character_mut(character_id).ok_or("no such character")?;
        let used = ch.hp.hit_dice_used.entry(die.to_string()).or_insert(0);
        if *used >= have {
            return Err(format!("no {die} hit dice left"));
        }
        *used += 1;
        let gain = (roll + con).max(0);
        let before = ch.hp.current;
        ch.hp.current = (ch.hp.current + gain).min(sheet.hp_max);
        let got = ch.hp.current - before;
        let name = ch.name.clone();
        let text = format!("spends a hit die: {roll} {} = {gain}, regains {got} hit points", expr::fmt_bonus(con as i64));
        self.sync_combatant_from_pc(character_id);
        self.campaign.say(&name, text.clone(), "rest");
        self.touch();
        Ok(ActionResult { text, hit: None, total: gain as i64, damage: 0 })
    }

    /// A dying character's death saving throw.
    pub fn death_save(&mut self, character_id: &str) -> Result<ActionResult, String> {
        let d = self.rng.roll_die(20) as i32;
        let ch = self.campaign.character_mut(character_id).ok_or("no such character")?;
        if ch.hp.current > 0 {
            return Err("only a character at 0 hit points rolls death saves".into());
        }
        let mut text = format!("death save: {d}");
        if d == 20 {
            ch.hp.current = 1;
            ch.death_saves = Default::default();
            text.push_str(" · back on their feet with 1 hit point!");
        } else if d >= 10 {
            ch.death_saves.successes += 1;
            if ch.death_saves.successes >= 3 {
                ch.death_saves = Default::default();
                if !ch.conditions.iter().any(|c| c == "stable") {
                    ch.conditions.push("stable".into());
                }
                text.push_str(" · success, and now stable");
            } else {
                text.push_str(&format!(" · success ({} of 3)", ch.death_saves.successes));
            }
        } else {
            ch.death_saves.failures += if d == 1 { 2 } else { 1 };
            if ch.death_saves.failures >= 3 {
                ch.death_saves.failures = 3;
                text.push_str(" · failure. They die.");
            } else {
                text.push_str(&format!(" · failure ({} of 3)", ch.death_saves.failures));
            }
        }
        let name = ch.name.clone();
        self.sync_combatant_from_pc(character_id);
        self.campaign.say(&name, text.clone(), "roll");
        self.touch();
        Ok(ActionResult { text, hit: Some(d >= 10), total: d as i64, damage: 0 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dice_multiply() {
        assert_eq!(mul_dice("1d8 + 3", 2), "2d8 + 3");
        assert_eq!(mul_dice("d10", 3), "3d10");
        assert_eq!(mul_dice("2d6 + 1d4 + mod", 2), "4d6 + 2d4 + mod");
        assert_eq!(mul_dice("8d6", 1), "8d6");
        assert_eq!(cantrip_dice(1), 1);
        assert_eq!(cantrip_dice(5), 2);
        assert_eq!(cantrip_dice(17), 4);
    }

    fn world() -> World {
        let mut w = World::default();
        w.rng = crate::Rng::seeded(7);
        w.library.insert(
            Entry::Class(Class {
                id: "wizard".into(),
                name: "Wizard".into(),
                hit_die: 6,
                spellcasting: Some(Spellcasting { ability: "int".into(), progression: "full".into(), prepared: "max(1, int_mod + class.wizard.level)".into(), cantrips_known: vec![3; 20], spellbook_start: 6, spellbook_per_level: 2, ..Default::default() }),
                ..Default::default()
            }),
            None,
        );
        w.library.insert(Entry::Class(Class { id: "cleric".into(), name: "Cleric".into(), hit_die: 8, spellcasting: Some(Spellcasting { ability: "wis".into(), progression: "full".into(), prepared: "max(1, wis_mod + class.cleric.level)".into(), cantrips_known: vec![3; 20], ..Default::default() }), ..Default::default() }), None);
        let spell = |id: &str, level: u32, class: &str| Spell { id: id.into(), name: id.into(), level, classes: vec![class.into()], ..Default::default() };
        w.library.insert(Entry::Spell(Spell { attack: true, damage: "1d10".into(), damage_type: "fire".into(), ..spell("fire_bolt", 0, "wizard") }), None);
        w.library.insert(Entry::Spell(Spell { damage: "3d4 + 3".into(), damage_type: "force".into(), upcast: "1d4 + 1".into(), ..spell("magic_missile", 1, "wizard") }), None);
        w.library.insert(Entry::Spell(Spell { save: "dex".into(), save_half: true, damage: "8d6".into(), damage_type: "fire".into(), upcast: "1d6".into(), ..spell("fireball", 3, "wizard") }), None);
        w.library.insert(Entry::Spell(Spell { heal: "1d8 + mod".into(), upcast: "1d8".into(), ..spell("cure_wounds", 1, "cleric") }), None);
        w.library.insert(Entry::Spell(spell("bless", 1, "cleric")), None);
        w.library.insert(Entry::Creature(Creature { id: "troll".into(), name: "Troll".into(), ac: 5, hp: 60, damage_resistances: vec!["force".into()], actions: vec![Feature { name: "Claw".into(), attack: Some(Attack { to_hit: 30, damage: "2d6 + 4".into(), damage_type: "slashing".into(), ..Default::default() }), ..Default::default() }], ..Default::default() }), None);
        let mut wiz = Character::new("wiz", "Wiz");
        wiz.classes = vec![ClassLevel { class: "wizard".into(), level: 5, ..Default::default() }];
        wiz.base_abilities.set("int", 16);
        wiz.hp.current = 20;
        let mut cle = Character::new("cle", "Cle");
        cle.classes = vec![ClassLevel { class: "cleric".into(), level: 1, ..Default::default() }];
        cle.base_abilities.set("wis", 16);
        cle.hp.current = 3;
        w.campaign.characters = vec![wiz, cle];
        for id in ["wiz", "cle"] {
            let c = w.combatant_from_character(id).unwrap();
            w.campaign.encounter.combatants.push(c);
        }
        let mut t = w.combatant_from_creature("troll", None).unwrap();
        t.visible = true;
        w.campaign.encounter.combatants.push(t);
        for c in &mut w.campaign.encounter.combatants {
            c.visible = true;
        }
        w
    }

    #[test]
    fn range_walls_conditions_and_movement() {
        use crate::grid::Pos;
        let mut w = world();
        let mut m = MapState::new("Hall", 30, 3);
        m.cells = vec![".".repeat(30); 3];
        w.campaign.map = m;
        let put = |w: &mut World, id: &str, x: i32| w.campaign.encounter.find_mut(id).unwrap().pos = Some(Pos::new(x, 1));
        put(&mut w, "wiz", 1);
        put(&mut w, "troll", 2);
        put(&mut w, "cle", 28);
        // fists reach the troll next to her, and nothing further
        assert!(w.pc_attack("wiz", 0, "troll", RollMode::Normal).is_ok());
        put(&mut w, "troll", 8);
        let no = w.pc_attack("wiz", 0, "troll", RollMode::Normal).unwrap_err();
        assert!(no.contains("35 ft away") && no.contains("5 ft"), "{no}");
        // a thrown dagger: fine at 20, disadvantage to 60, impossible beyond
        let dagger = Reach::parse("reach 5 ft. or range 20/60 ft.");
        let (mode, note, _) = w.engage("wiz", "troll", &dagger, "Dagger", true, RollMode::Normal).unwrap();
        assert_eq!(mode, RollMode::Disadvantage);
        assert!(note.contains("long range"), "{note}");
        let (mode, _, _) = w.engage("wiz", "troll", &dagger, "Dagger", true, RollMode::Advantage).unwrap();
        assert_eq!(mode, RollMode::Normal, "advantage and disadvantage cancel");
        put(&mut w, "troll", 20);
        assert!(w.engage("wiz", "troll", &dagger, "Dagger", true, RollMode::Normal).is_err());
        // a bow with an enemy in her face
        let bow = Reach::parse("range 80/320 ft.");
        put(&mut w, "troll", 2);
        let (mode, note, _) = w.engage("wiz", "troll", &bow, "Longbow", true, RollMode::Normal).unwrap();
        assert_eq!(mode, RollMode::Disadvantage);
        assert!(note.contains("enemy within 5 ft"), "{note}");
        // conditions: a restrained target is easier, a prone one far away harder
        put(&mut w, "troll", 8);
        w.campaign.encounter.find_mut("troll").unwrap().conditions = vec!["restrained".into()];
        assert_eq!(w.engage("wiz", "troll", &bow, "Longbow", true, RollMode::Normal).unwrap().0, RollMode::Advantage);
        w.campaign.encounter.find_mut("troll").unwrap().conditions = vec!["prone".into()];
        assert_eq!(w.engage("wiz", "troll", &bow, "Longbow", true, RollMode::Normal).unwrap().0, RollMode::Disadvantage);
        put(&mut w, "troll", 2);
        w.campaign.encounter.find_mut("troll").unwrap().conditions = vec!["paralyzed".into()];
        let (mode, _, crit) = w.engage("wiz", "troll", &Reach::melee(5), "Fist", true, RollMode::Normal).unwrap();
        assert!(mode == RollMode::Advantage && crit, "a hit on a paralyzed creature next to you is a critical");
        w.campaign.encounter.find_mut("troll").unwrap().conditions.clear();
        // a wall between them stops arrows and spells alike
        put(&mut w, "troll", 8);
        w.campaign.map.walls.push(Wall { x1: 5.0, y1: 0.0, x2: 5.0, y2: 3.0, ..Default::default() });
        assert!(w.engage("wiz", "troll", &bow, "Longbow", true, RollMode::Normal).unwrap_err().contains("no clear line"));
        w.campaign.map.walls.clear();
        // spells: touch needs to be next to the friend, and no slot is lost finding out
        w.campaign.character_mut("cle").unwrap().spells.prepared = vec!["cure_wounds".into()];
        w.library.spells.get_mut("cure_wounds").unwrap().range = "Touch".into();
        assert!(w.pc_cast("cle", "cure_wounds", 1, &["wiz".into()], RollMode::Normal).unwrap_err().contains("ft away"));
        assert!(w.campaign.character("cle").unwrap().spells.slots_used.iter().all(|u| *u == 0));
        assert_eq!(spell_reach(&Spell { range: "150 feet".into(), area: Some(Area { shape: "sphere".into(), size: 20, width: None }), ..Default::default() }, 100).long, 170);
        assert_eq!(spell_reach(&Spell { range: "120 feet".into(), ..Default::default() }, 200).normal, 240);
        // someone who cannot act cannot attack
        w.campaign.encounter.find_mut("wiz").unwrap().conditions = vec!["stunned".into()];
        assert!(w.pc_attack("wiz", 0, "troll", RollMode::Normal).unwrap_err().contains("stunned"));
        w.campaign.encounter.find_mut("wiz").unwrap().conditions.clear();
        // turning the rule off hands it back to the DM
        w.campaign.settings.enforce_range = false;
        assert!(w.pc_attack("wiz", 0, "troll", RollMode::Normal).is_ok());
        w.campaign.settings.enforce_range = true;

        // ---- movement: speed per turn, twice with a dash, none while held
        for c in &mut w.campaign.encounter.combatants {
            c.initiative = if c.id == "wiz" { 20 } else { 1 };
        }
        w.start_encounter();
        let speed = w.move_budget("wiz").unwrap();
        assert!(speed >= 25, "{speed}");
        assert!(w.move_budget("troll").is_none(), "not the troll's turn");
        assert!(w.player_walk("troll", Pos::new(9, 1)).is_err());
        let far = 1 + (speed / 5) as i32 + 1;
        assert!(w.player_walk("wiz", Pos::new(far, 0)).unwrap_err().contains("movement left"));
        assert_eq!(w.player_walk("wiz", Pos::new(far - 1, 0)).unwrap(), speed);
        assert_eq!(w.move_budget("wiz"), Some(0));
        assert!(w.player_walk("wiz", Pos::new(far, 0)).is_err());
        w.dash("wiz").unwrap();
        assert_eq!(w.move_budget("wiz"), Some(speed));
        assert!(w.dash("wiz").is_err());
        w.campaign.encounter.find_mut("wiz").unwrap().conditions = vec!["grappled".into()];
        assert_eq!(w.move_budget("wiz"), Some(0));
        w.next_turn();
        assert_eq!(w.campaign.encounter.find("wiz").unwrap().dashed, true, "kept until her next turn starts");
    }

    #[test]
    fn choosing_spells_follows_the_class() {
        let mut w = world();
        let lib = std::mem::take(&mut w.library);
        let wiz = w.campaign.character_mut("wiz").unwrap();
        assert!(choose_spells(&lib, wiz, &["cure_wounds".into()], false).is_err(), "not a wizard spell");
        choose_spells(&lib, wiz, &["fire_bolt".into(), "magic_missile".into(), "fireball".into()], false).unwrap();
        assert!(choose_spells(&lib, wiz, &["fire_bolt".into()], true).is_err(), "no dropping spells mid-game");
        let cle = w.campaign.character_mut("cle").unwrap();
        assert!(choose_spells(&lib, cle, &["bless".into()], false).is_err(), "a cleric prepares, it does not learn");
        let sheet = crate::sheet::derive(cle, &lib);
        assert!(spells_available(&lib, cle, &sheet).contains(&"bless".to_string()));
        assert!(!spells_available(&lib, cle, &sheet).contains(&"fireball".to_string()));
    }

    #[test]
    fn casting_spends_slots_and_hurts() {
        let mut w = world();
        let lib = std::mem::take(&mut w.library);
        choose_spells(&lib, w.campaign.character_mut("wiz").unwrap(), &["fire_bolt".into(), "magic_missile".into(), "fireball".into()], false).unwrap();
        w.library = lib;
        assert!(w.pc_cast("wiz", "magic_missile", 1, &["troll".into()], RollMode::Normal).is_err(), "not prepared yet");
        w.campaign.character_mut("wiz").unwrap().spells.prepared = vec!["magic_missile".into(), "fireball".into()];
        let r = w.pc_cast("wiz", "magic_missile", 2, &["troll".into()], RollMode::Normal).unwrap();
        // 4d4 + 4 force, halved by the troll's resistance
        assert!((8..=20).contains(&r.damage), "{r:?}");
        assert!(r.text.contains("resisted"), "{}", r.text);
        let troll_hp = w.campaign.encounter.find("troll").unwrap().hp;
        assert_eq!(troll_hp, 60 - r.damage / 2);
        assert_eq!(w.campaign.character("wiz").unwrap().spells.slots_used[1], 1);
        let r = w.pc_cast("wiz", "fireball", 3, &["troll".into(), "cle".into()], RollMode::Normal).unwrap();
        assert!(r.text.contains("save"), "{}", r.text);
        assert!(w.pc_cast("wiz", "fireball", 4, &[], RollMode::Normal).is_err(), "a level 5 wizard has no level 4 slot");
        // cantrip: two dice at level 5, AC 5 is hard to miss
        let r = w.pc_cast("wiz", "fire_bolt", 0, &["troll".into()], RollMode::Advantage).unwrap();
        assert!(r.text.contains("advantage"));
        assert_eq!(w.campaign.table.last().unwrap().who, "Wiz");
    }

    #[test]
    fn attacks_heal_rest_and_turns() {
        let mut w = world();
        let r = w.creature_attack("troll", 0, "cle", RollMode::Normal).unwrap();
        assert_eq!(r.hit, Some(true));
        assert_eq!(w.campaign.character("cle").unwrap().hp.current, 0, "the sheet follows the token");
        w.campaign.character_mut("cle").unwrap().spells.prepared = vec!["cure_wounds".into()];
        assert!(w.pc_cast("cle", "cure_wounds", 1, &["cle".into()], RollMode::Normal).unwrap_err().contains("down"), "no casting at 0 hit points");
        w.heal("cle", 1);
        w.pc_cast("cle", "cure_wounds", 1, &["cle".into()], RollMode::Normal).unwrap();
        assert!(w.campaign.character("cle").unwrap().hp.current > 0);
        w.rest("cle", true).unwrap();
        let cle = w.campaign.character("cle").unwrap();
        assert!(cle.spells.slots_used.iter().all(|u| *u == 0));
        assert_eq!(cle.hp.current, w.sheet("cle").unwrap().hp_max);

        w.start_encounter();
        let first = w.campaign.encounter.current().unwrap().clone();
        let other = if matches!(&first.kind, CombatantKind::Pc { id } if id == "wiz") { "cle" } else { "wiz" };
        assert!(w.end_turn(other).is_err() || !matches!(first.kind, CombatantKind::Pc { .. }));
        w.next_turn();
        assert!(w.campaign.table.iter().any(|e| e.kind == "turn"));
    }
}
