//! What one viewer may know about the map right now. The DM gets
//! everything; a player gets only the squares their character has seen, and
//! tokens only where it can see them this moment. The apps draw a `MapView`
//! and never see the map itself, so there is nothing to peek at.

use crate::grid::Pos;
use crate::model::*;
use crate::vision::{self, Eye, Grid};
use crate::World;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq)]
pub enum Viewer {
    Dm,
    /// Everything any player character can see.
    Party,
    /// One character (falls back to the party while it has no token).
    Character(String),
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TokenView {
    pub id: String,
    pub name: String,
    /// `pc`, `creature` or `custom`.
    pub kind: String,
    /// Class id for a PC, creature id for a creature: picks how it is drawn.
    #[serde(default)]
    pub look: String,
    pub x: i32,
    pub y: i32,
    /// Side length in squares.
    pub size: u32,
    /// Health from 0 to 1, when the viewer may know it.
    #[serde(default)]
    pub hp: Option<f32>,
    #[serde(default)]
    pub down: bool,
    #[serde(default)]
    pub conditions: Vec<String>,
    #[serde(default)]
    pub current: bool,
    /// The viewer may move this token.
    #[serde(default)]
    pub mine: bool,
    /// DM only: players cannot see it.
    #[serde(default)]
    pub hidden: bool,
    /// Carries a light (bright radius in feet).
    #[serde(default)]
    pub light: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MapView {
    pub id: String,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub feet_per_square: u32,
    pub darkness: f32,
    /// Fog is in use (otherwise everything is in plain view).
    pub fog: bool,
    /// The viewer is the DM: fog is drawn see-through and hidden things are marked.
    pub dm: bool,
    /// Terrain rows; `?` where the viewer has never seen.
    pub cells: Vec<String>,
    /// Per square: `0` never seen, `1` remembered, `2` in view now.
    pub seen: Vec<String>,
    /// Per square: `0` dark, `1` dim, `2` bright.
    pub light: Vec<String>,
    pub walls: Vec<Wall>,
    pub lights: Vec<Light>,
    pub props: Vec<Prop>,
    pub markers: Vec<Marker>,
    pub tokens: Vec<TokenView>,
    pub start: Option<Pos>,
    pub encounter_active: bool,
    pub round: u32,
    /// Feet the viewer's own character can still walk this turn (only in a
    /// fight, on their turn).
    #[serde(default)]
    pub move_left: Option<u32>,
}

impl MapView {
    /// Feet between two tokens, edge to edge, counting diagonals as one
    /// square (two tokens side by side are 5 ft apart).
    pub fn distance_ft(&self, a: &str, b: &str) -> Option<u32> {
        let find = |id: &str| self.tokens.iter().find(|t| t.id == id);
        let (a, b) = (find(a)?, find(b)?);
        let gap = |a0: i32, asz: u32, b0: i32, bsz: u32| (a0 - (b0 + bsz as i32 - 1)).max(b0 - (a0 + asz as i32 - 1)).max(0);
        let squares = gap(a.x, a.size, b.x, b.size).max(gap(a.y, a.size, b.y, b.size)).max(if a.id == b.id { 0 } else { 1 });
        Some(squares as u32 * self.feet_per_square.max(1))
    }
    pub fn tile(&self, x: i32, y: i32) -> char {
        if x < 0 || y < 0 {
            return tile::ROCK;
        }
        self.cells.get(y as usize).and_then(|r| r.chars().nth(x as usize)).unwrap_or(tile::ROCK)
    }
    fn digit(rows: &[String], x: i32, y: i32, default: u8) -> u8 {
        if x < 0 || y < 0 {
            return default;
        }
        rows.get(y as usize).and_then(|r| r.as_bytes().get(x as usize)).map(|b| b.saturating_sub(b'0')).unwrap_or(default)
    }
    pub fn seen_at(&self, x: i32, y: i32) -> u8 {
        if self.fog {
            Self::digit(&self.seen, x, y, 0)
        } else {
            2
        }
    }
    pub fn light_at(&self, x: i32, y: i32) -> u8 {
        Self::digit(&self.light, x, y, 0)
    }
}

fn center(c: &Combatant, rules: &Rules) -> Option<(f32, f32)> {
    let p = c.pos?;
    let half = rules.size_squares(&c.size).max(1) as f32 / 2.0;
    Some((p.x as f32 + half, p.y as f32 + half))
}

fn rows<T: Copy + Default>(g: &Grid<T>, f: impl Fn(T) -> char) -> Vec<String> {
    (0..g.height).map(|y| (0..g.width).map(|x| f(g.get(x, y))).collect()).collect()
}

impl World {
    fn eyes(&self, viewer: &Viewer) -> Vec<Eye> {
        let rules = &self.library.rules;
        let pcs = || self.campaign.encounter.combatants.iter().filter(|c| matches!(c.kind, CombatantKind::Pc { .. }));
        let eye = |c: &Combatant| center(c, rules).map(|at| Eye { at, darkvision: c.darkvision });
        if let Viewer::Character(id) = viewer {
            let own: Vec<Eye> = pcs().filter(|c| matches!(&c.kind, CombatantKind::Pc { id: pc } if pc == id)).filter_map(eye).collect();
            if !own.is_empty() {
                return own;
            }
        }
        pcs().filter_map(eye).collect()
    }

    fn lights_and_sight(&self) -> (Vec<Seg>, Grid<u8>) {
        let map = &self.campaign.map;
        let rules = &self.library.rules;
        let segs = map.sight_blockers();
        let carried: Vec<((f32, f32), u32, u32)> =
            self.campaign.encounter.combatants.iter().filter(|c| c.light > 0).filter_map(|c| center(c, rules).map(|p| (p, c.light, c.light * 2))).collect();
        let light = vision::light_grid(map, &segs, &carried);
        (segs, light)
    }

    /// 0 never seen, 1 remembered, 2 in view, for every square.
    fn seen_grid(&self, viewer: &Viewer, segs: &[Seg], light: &Grid<u8>) -> Grid<u8> {
        let map = &self.campaign.map;
        let mut g = Grid::<u8>::new(map.width, map.height);
        match map.fog {
            FogMode::Off => g.data.iter_mut().for_each(|v| *v = 2),
            FogMode::Manual => {
                for y in 0..g.height {
                    for x in 0..g.width {
                        g.set(x, y, if map.is_explored(x, y) { 2 } else { 0 });
                    }
                }
            }
            FogMode::Vision => {
                let now = vision::visible_cells(map, segs, light, &self.eyes(viewer));
                for y in 0..g.height {
                    for x in 0..g.width {
                        g.set(x, y, if now.get(x, y) { 2 } else if map.is_explored(x, y) { 1 } else { 0 });
                    }
                }
            }
        }
        g
    }

    /// Remember what the party can see now. Returns true if the party saw
    /// something new. Call after anything moves or a door opens.
    pub fn update_explored(&mut self) -> bool {
        if self.campaign.map.fog != FogMode::Vision {
            return false;
        }
        let (segs, light) = self.lights_and_sight();
        let now = vision::visible_cells(&self.campaign.map, &segs, &light, &self.eyes(&Viewer::Party));
        let mut changed = false;
        for y in 0..now.height {
            for x in 0..now.width {
                if now.get(x, y) && !self.campaign.map.is_explored(x, y) {
                    self.campaign.map.set_explored(x, y, true);
                    changed = true;
                }
            }
        }
        changed
    }

    pub fn map_view(&self, viewer: &Viewer) -> MapView {
        let map = &self.campaign.map;
        let enc = &self.campaign.encounter;
        let rules = &self.library.rules;
        let dm = *viewer == Viewer::Dm;
        let (segs, light) = self.lights_and_sight();
        // the DM is shown the party's fog, drawn see-through
        let seen = self.seen_grid(if dm { &Viewer::Party } else { viewer }, &segs, &light);
        let known = |x: i32, y: i32| dm || seen.get(x, y) > 0;
        let near_known = |x: f32, y: f32| {
            dm || [(-0.02, -0.02), (0.02, -0.02), (-0.02, 0.02), (0.02, 0.02)].iter().any(|(dx, dy)| seen.get((x + dx).floor() as i32, (y + dy).floor() as i32) > 0)
        };
        let show_hp = self.campaign.settings.show_monster_hp_to_players;
        let own_pc = match viewer {
            Viewer::Character(id) => Some(id.as_str()),
            _ => None,
        };
        let turn_id = if enc.active { enc.current().map(|c| c.id.as_str()) } else { None };

        let cells = (0..map.height as i32).map(|y| (0..map.width as i32).map(|x| if known(x, y) { map.tile(x, y) } else { tile::UNKNOWN }).collect()).collect();
        let walls = map
            .walls
            .iter()
            .filter(|w| {
                let (mx, my) = w.mid();
                near_known(mx, my) || near_known(w.x1, w.y1) && near_known(w.x2, w.y2)
            })
            .map(|w| {
                let mut w = w.clone();
                if !dm {
                    // a closed secret door is just a wall, and nobody is told what is locked
                    if w.kind == WallKind::SecretDoor {
                        w.kind = if w.open { WallKind::Door } else { WallKind::Wall };
                    }
                    w.locked = false;
                }
                w
            })
            .collect();
        let tokens = enc
            .combatants
            .iter()
            .filter_map(|c| {
                let p = c.pos?;
                let size = rules.size_squares(&c.size).max(1);
                let pc_id = match &c.kind {
                    CombatantKind::Pc { id } => Some(id.as_str()),
                    _ => None,
                };
                let in_view = || crate::grid::footprint(p, size).iter().any(|q| seen.get(q.x, q.y) == 2);
                if !dm && (!c.visible || (pc_id.is_none() && !in_view())) {
                    return None;
                }
                let mine = match (own_pc, pc_id) {
                    (Some(a), Some(b)) => a == b && self.campaign.settings.players_move_tokens && (turn_id.is_none() || turn_id == Some(c.id.as_str())),
                    _ => dm,
                };
                Some(TokenView {
                    id: c.id.clone(),
                    name: c.name.clone(),
                    kind: match c.kind {
                        CombatantKind::Pc { .. } => "pc",
                        CombatantKind::Creature { .. } => "creature",
                        CombatantKind::Custom => "custom",
                    }
                    .into(),
                    look: match &c.kind {
                        CombatantKind::Pc { id } => self.campaign.character(id).and_then(|ch| ch.classes.first()).map(|cl| cl.class.clone()).unwrap_or_default(),
                        CombatantKind::Creature { template } => template.clone(),
                        CombatantKind::Custom => String::new(),
                    },
                    x: p.x,
                    y: p.y,
                    size,
                    hp: if dm || pc_id.is_some() || show_hp { Some(if c.hp_max > 0 { (c.hp as f32 / c.hp_max as f32).clamp(0.0, 1.0) } else { 1.0 }) } else { None },
                    down: c.is_down(),
                    conditions: c.conditions.clone(),
                    current: turn_id == Some(c.id.as_str()),
                    mine,
                    hidden: !c.visible,
                    light: c.light,
                })
            })
            .collect();
        MapView {
            id: map.id.clone(),
            name: map.name.clone(),
            width: map.width,
            height: map.height,
            feet_per_square: map.feet_per_square,
            darkness: map.darkness,
            fog: map.fog != FogMode::Off,
            dm,
            cells,
            light: rows(&light, |v| (b'0' + v) as char).into_iter().enumerate().map(|(y, r)| r.chars().enumerate().map(|(x, c)| if known(x as i32, y as i32) { c } else { '0' }).collect()).collect(),
            seen: rows(&seen, |v| (b'0' + v) as char),
            walls,
            lights: map.lights.iter().filter(|l| near_known(l.x, l.y)).cloned().collect(),
            props: map.props.iter().filter(|p| dm || (!p.hidden && known(p.x.floor() as i32, p.y.floor() as i32))).cloned().collect(),
            markers: map.markers.iter().filter(|m| dm || (!m.hidden && known(m.x, m.y))).cloned().collect(),
            tokens,
            start: if dm { map.start } else { None },
            encounter_active: enc.active,
            round: enc.round,
            move_left: own_pc.and_then(|id| self.pc_combatant(id)).and_then(|cid| self.move_budget(&cid)),
        }
    }

    /// Move a token. With `walk` the way must be clear (walls, rock, closed
    /// doors) and no longer than `max_squares`; the DM passes `walk = false`
    /// to put a token anywhere. Returns the distance walked in feet.
    pub fn move_token(&mut self, id: &str, to: Pos, walk: bool, max_squares: u32) -> Result<u32, String> {
        let rules = &self.library.rules;
        let map = &self.campaign.map;
        let c = self.campaign.encounter.find(id).ok_or("no such token")?;
        let side = rules.size_squares(&c.size).max(1);
        if !map.in_bounds(to.x, to.y) || !map.in_bounds(to.x + side as i32 - 1, to.y + side as i32 - 1) {
            return Err("that is off the map".into());
        }
        let mut feet = 0;
        if walk {
            let from = c.pos.ok_or("the DM has not put you on the map yet")?;
            if self.campaign.encounter.combatants.iter().any(|o| o.id != id && o.pos == Some(to)) {
                return Err("someone is standing there".into());
            }
            let (_, cost) = vision::find_path(map, from, to, side, max_squares).ok_or("you cannot get there from here")?;
            feet = cost * map.feet_per_square;
        }
        if let Some(c) = self.campaign.encounter.find_mut(id) {
            c.pos = Some(to);
        }
        self.update_explored();
        self.touch();
        Ok(feet)
    }

    /// Open or close a door. Returns its new state, or an error if locked.
    pub fn toggle_door(&mut self, index: usize, force: bool) -> Result<bool, String> {
        let w = self.campaign.map.walls.get_mut(index).filter(|w| w.is_door()).ok_or("no door there")?;
        if w.locked && !w.open && !force {
            return Err("it is locked".into());
        }
        w.open = !w.open;
        let open = w.open;
        self.update_explored();
        self.touch();
        Ok(open)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> World {
        let mut w = World::default();
        let mut m = MapState::new("Cellar", 7, 5);
        m.cells = vec!["#######".into(), "#..#..#".into(), "#.....#".into(), "#..#..#".into(), "#######".into()];
        m.walls.push(Wall { x1: 3.0, y1: 2.0, x2: 3.0, y2: 3.0, kind: WallKind::SecretDoor, locked: true, ..Default::default() });
        m.props.push(Prop { kind: "chest".into(), x: 5.5, y: 1.5, ..Default::default() });
        m.props.push(Prop { kind: "trap".into(), x: 1.5, y: 1.5, hidden: true, ..Default::default() });
        m.markers.push(Marker { x: 1, y: 3, text: "ambush here".into(), hidden: true });
        m.fog = FogMode::Vision;
        w.campaign.map = m;
        w.campaign.characters.push(Character::new("ann", "Ann"));
        w.campaign.encounter.combatants.push(Combatant { id: "ann".into(), name: "Ann".into(), kind: CombatantKind::Pc { id: "ann".into() }, hp: 5, hp_max: 10, pos: Some(Pos::new(1, 2)), visible: true, ..Default::default() });
        w.campaign.encounter.combatants.push(Combatant { id: "rat".into(), name: "Rat".into(), kind: CombatantKind::Creature { template: "rat".into() }, hp: 1, hp_max: 1, pos: Some(Pos::new(5, 2)), visible: true, ..Default::default() });
        w.update_explored();
        w
    }

    #[test]
    fn players_get_only_what_they_have_seen() {
        let mut w = world();
        let ann = Viewer::Character("ann".into());
        let v = w.map_view(&ann);
        assert!(v.fog && !v.dm);
        assert_eq!(v.tile(1, 1), tile::FLOOR);
        assert_eq!(v.tile(5, 2), tile::UNKNOWN, "the far room is not sent at all");
        assert_eq!(v.seen_at(1, 1), 2);
        assert_eq!(v.seen_at(5, 2), 0);
        assert_eq!(v.tokens.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(), vec!["ann"], "the rat is out of sight");
        assert!(v.tokens[0].mine && v.tokens[0].hp == Some(0.5));
        assert!(v.props.is_empty() && v.markers.is_empty(), "no hidden trap, no DM note, no unseen chest");
        assert_eq!(v.walls[0].kind, WallKind::Wall, "a secret door looks like a wall");
        assert!(!v.walls[0].locked && v.start.is_none());

        let dm = w.map_view(&Viewer::Dm);
        assert!(dm.dm && dm.tokens.len() == 2 && dm.props.len() == 2 && dm.markers.len() == 1);
        assert_eq!(dm.tile(5, 2), tile::FLOOR);
        assert_eq!(dm.walls[0].kind, WallKind::SecretDoor);
        assert_eq!(dm.seen_at(5, 2), 0, "the DM is shown where the party's fog is");

        // locked: a player cannot open it, the DM can; then the far room comes into view
        assert!(w.toggle_door(0, false).is_err());
        assert_eq!(w.toggle_door(0, true), Ok(true));
        let v = w.map_view(&ann);
        assert_eq!(v.walls[0].kind, WallKind::Door);
        assert_eq!(v.tile(5, 2), tile::FLOOR);
        assert!(v.tokens.iter().any(|t| t.id == "rat" && t.hp.is_none() && !t.mine));
        assert!(v.props.iter().any(|p| p.kind == "chest"));

        // walking: through the doorway is fine, into rock or too far is not
        assert_eq!(w.move_token("ann", Pos::new(4, 2), true, 6), Ok(15));
        assert!(w.move_token("ann", Pos::new(3, 1), true, 6).is_err());
        assert!(w.move_token("ann", Pos::new(5, 2), true, 6).is_err(), "the rat is there");
        assert!(w.move_token("ann", Pos::new(1, 1), true, 2).is_err(), "further than allowed");
        // the door shuts behind her: the first room is remembered, not in view
        w.toggle_door(0, true).unwrap();
        let v = w.map_view(&ann);
        assert_eq!(v.seen_at(1, 1), 1);
        assert_eq!(v.tile(1, 1), tile::FLOOR);
        assert_eq!(v.seen_at(5, 1), 2);
        // in a fight she may only move on her turn
        w.campaign.encounter.active = true;
        w.campaign.encounter.turn = 1;
        assert!(!w.map_view(&ann).tokens.iter().find(|t| t.id == "ann").unwrap().mine);
        w.campaign.encounter.turn = 0;
        assert!(w.map_view(&ann).tokens.iter().find(|t| t.id == "ann").unwrap().mine);
    }

    #[test]
    fn manual_fog_and_no_fog() {
        let mut w = world();
        w.campaign.map.fog = FogMode::Manual;
        w.campaign.map.reset_fog();
        let v = w.map_view(&Viewer::Party);
        assert_eq!(v.tile(1, 1), tile::UNKNOWN);
        assert_eq!(v.tokens.len(), 1, "player characters always show");
        w.campaign.map.set_explored(5, 2, true);
        let v = w.map_view(&Viewer::Party);
        assert_eq!(v.tile(5, 2), tile::FLOOR);
        assert_eq!(v.tokens.len(), 2);
        w.campaign.map.fog = FogMode::Off;
        let v = w.map_view(&Viewer::Party);
        assert!(!v.fog && v.tile(1, 1) == tile::FLOOR && v.seen_at(0, 0) == 2);
        assert!(v.props.iter().all(|p| !p.hidden));
    }
}
