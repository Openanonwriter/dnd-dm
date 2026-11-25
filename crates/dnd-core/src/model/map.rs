//! A map: a grid of terrain tiles with walls, doors, lights, props, notes
//! and fog of war. Everything is plain text in the save, so a map can be
//! written by hand: `cells` is a list of strings, one character per square.
//!
//! ```toml
//! cells = [
//!   "##########",
//!   "#....#...#",
//!   "#....+...#",   # '+' is not special; doors are in [[walls]]
//!   "##########",
//! ]
//! ```

use super::Combatant;
use crate::grid::Pos;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Terrain characters used in `cells`.
pub mod tile {
    pub const ROCK: char = '#';
    pub const FLOOR: char = '.';
    pub const GRASS: char = ',';
    pub const DIRT: char = ':';
    pub const WOOD: char = '=';
    pub const WATER: char = '~';
    pub const RUBBLE: char = '^';
    pub const LAVA: char = '%';
    pub const SAND: char = '_';
    pub const SNOW: char = '*';
    /// Sent to players for squares they have never seen.
    pub const UNKNOWN: char = '?';

    /// (character, name) for every paintable tile.
    pub const ALL: [(char, &str); 10] = [
        (FLOOR, "Stone floor"),
        (ROCK, "Solid rock / wall"),
        (WOOD, "Wooden floor"),
        (DIRT, "Dirt"),
        (GRASS, "Grass"),
        (SAND, "Sand"),
        (SNOW, "Snow"),
        (WATER, "Water"),
        (RUBBLE, "Rubble"),
        (LAVA, "Lava"),
    ];

    /// Blocks both sight and movement.
    pub fn solid(c: char) -> bool {
        c == ROCK
    }
    /// Costs double movement.
    pub fn difficult(c: char) -> bool {
        matches!(c, WATER | RUBBLE | SNOW)
    }
}

/// Things worth placing on a map. The name is all that is saved; how one
/// looks is up to the app drawing it.
pub const PROP_KINDS: [&str; 20] = [
    "chest", "barrel", "crate", "table", "chair", "bed", "bookshelf", "pillar", "statue", "altar", "stairs", "campfire", "tree", "bush", "boulder", "well",
    "trap", "rug", "bones", "portal",
];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WallKind {
    /// Blocks sight and movement.
    #[default]
    Wall,
    /// Blocks movement, not sight.
    Window,
    /// Blocks both while closed.
    Door,
    /// A door that looks like a wall to players until it is opened.
    SecretDoor,
}

/// A straight wall between two points, measured in squares from the map's
/// top-left corner (so `x1 = 3.0` is the line between columns 2 and 3).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Wall {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    #[serde(default)]
    pub kind: WallKind,
    #[serde(default, skip_serializing_if = "is_false")]
    pub open: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub locked: bool,
}

impl Wall {
    pub fn is_door(&self) -> bool {
        matches!(self.kind, WallKind::Door | WallKind::SecretDoor)
    }
    pub fn blocks_sight(&self) -> bool {
        match self.kind {
            WallKind::Wall => true,
            WallKind::Window => false,
            WallKind::Door | WallKind::SecretDoor => !self.open,
        }
    }
    pub fn blocks_move(&self) -> bool {
        match self.kind {
            WallKind::Wall | WallKind::Window => true,
            WallKind::Door | WallKind::SecretDoor => !self.open,
        }
    }
    pub fn mid(&self) -> (f32, f32) {
        ((self.x1 + self.x2) / 2.0, (self.y1 + self.y2) / 2.0)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Light {
    pub x: f32,
    pub y: f32,
    /// Bright light radius in feet.
    #[serde(default = "twenty")]
    pub bright: u32,
    /// Dim light reaches this far (feet from the source).
    #[serde(default = "forty")]
    pub dim: u32,
    /// Hex colour, e.g. `ff9a2e`.
    #[serde(default = "torch")]
    pub color: String,
    #[serde(default = "yes")]
    pub on: bool,
}

impl Default for Light {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0, bright: 20, dim: 40, color: torch(), on: true }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Prop {
    pub kind: String,
    pub x: f32,
    pub y: f32,
    /// Quarter turns clockwise.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub rot: u8,
    /// Only the DM sees it (a trap, hidden loot) until revealed.
    #[serde(default, skip_serializing_if = "is_false")]
    pub hidden: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
}

/// A note pinned to a square.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Marker {
    pub x: i32,
    pub y: i32,
    pub text: String,
    /// DM-only unless switched off.
    #[serde(default = "yes")]
    pub hidden: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FogMode {
    /// Players see the whole map.
    #[default]
    Off,
    /// Players see what the DM has revealed with the fog brush.
    Manual,
    /// Players see what their characters can see; walls and doors block it.
    Vision,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MapState {
    /// File name of the map in `maps/`; made from the name when empty.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub id: String,
    #[serde(default)]
    pub name: String,
    /// Which part of the campaign this map belongs to ("Chapter 2", "Phandalin"…).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub chapter: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub image: String,
    #[serde(default = "thirty")]
    pub width: u32,
    #[serde(default = "twenty")]
    pub height: u32,
    #[serde(default = "five")]
    pub feet_per_square: u32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
    /// Old-style marked squares; folded into `cells` and `markers` on load.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub marks: BTreeMap<String, String>,
    /// Terrain, one string per row, one character per square (see `tile`).
    /// Missing rows and short rows are stone floor.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cells: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub walls: Vec<Wall>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lights: Vec<Light>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub props: Vec<Prop>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub markers: Vec<Marker>,
    #[serde(default)]
    pub fog: FogMode,
    /// What the party has seen (or the DM revealed): rows of '0' / '1'.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub explored: Vec<String>,
    /// 0 = daylight, 0.5 = dim, 1 = pitch black (only lights and darkvision help).
    #[serde(default, skip_serializing_if = "is_zero_f")]
    pub darkness: f32,
    /// Where the party appears when the DM moves the game to this map.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<Pos>,
    /// Creatures prepared on this map, waiting while the party is elsewhere.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parked: Vec<Combatant>,
}

fn thirty() -> u32 {
    30
}
fn twenty() -> u32 {
    20
}
fn forty() -> u32 {
    40
}
fn five() -> u32 {
    5
}
fn yes() -> bool {
    true
}
fn torch() -> String {
    "ff9a2e".into()
}
fn is_false(b: &bool) -> bool {
    !*b
}
fn is_zero(v: &u8) -> bool {
    *v == 0
}
fn is_zero_f(v: &f32) -> bool {
    *v == 0.0
}

impl Default for MapState {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: "Battle map".into(),
            chapter: String::new(),
            image: String::new(),
            width: 30,
            height: 20,
            feet_per_square: 5,
            notes: String::new(),
            marks: BTreeMap::new(),
            cells: Vec::new(),
            walls: Vec::new(),
            lights: Vec::new(),
            props: Vec::new(),
            markers: Vec::new(),
            fog: FogMode::Off,
            explored: Vec::new(),
            darkness: 0.0,
            start: None,
            parked: Vec::new(),
        }
    }
}

/// A straight line that blocks something, in squares.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Seg {
    pub a: (f32, f32),
    pub b: (f32, f32),
}

impl MapState {
    pub fn new(name: impl Into<String>, width: u32, height: u32) -> Self {
        Self { name: name.into(), width, height, ..Default::default() }
    }

    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as u32) < self.width && (y as u32) < self.height
    }

    pub fn tile(&self, x: i32, y: i32) -> char {
        if !self.in_bounds(x, y) {
            return tile::ROCK;
        }
        self.cells.get(y as usize).and_then(|r| r.chars().nth(x as usize)).unwrap_or(tile::FLOOR)
    }

    pub fn solid(&self, x: i32, y: i32) -> bool {
        tile::solid(self.tile(x, y))
    }

    pub fn set_tile(&mut self, x: i32, y: i32, c: char) {
        if !self.in_bounds(x, y) {
            return;
        }
        set_in_rows(&mut self.cells, self.width, x as usize, y as usize, c, tile::FLOOR);
    }

    /// Fill every square with one tile.
    pub fn fill(&mut self, c: char) {
        self.cells = vec![std::iter::repeat(c).take(self.width as usize).collect(); self.height as usize];
    }

    pub fn is_explored(&self, x: i32, y: i32) -> bool {
        self.in_bounds(x, y) && self.explored.get(y as usize).and_then(|r| r.as_bytes().get(x as usize)).map(|b| *b == b'1').unwrap_or(false)
    }

    pub fn set_explored(&mut self, x: i32, y: i32, on: bool) {
        if self.in_bounds(x, y) {
            set_in_rows(&mut self.explored, self.width, x as usize, y as usize, if on { '1' } else { '0' }, '0');
        }
    }

    pub fn reset_fog(&mut self) {
        self.explored.clear();
    }

    pub fn reveal_all(&mut self) {
        self.explored = vec!["1".repeat(self.width as usize); self.height as usize];
    }

    /// Turn old-style `marks` into tiles and notes, and tidy row lengths.
    pub fn normalize(&mut self) {
        let marks = std::mem::take(&mut self.marks);
        for (k, v) in marks {
            let Some((x, y)) = k.split_once(',') else { continue };
            let (Ok(x), Ok(y)) = (x.trim().parse::<i32>(), y.trim().parse::<i32>()) else { continue };
            match v.as_str() {
                "wall" => self.set_tile(x, y, tile::ROCK),
                "water" => self.set_tile(x, y, tile::WATER),
                "difficult" => self.set_tile(x, y, tile::RUBBLE),
                _ => self.markers.push(Marker { x, y, text: v, hidden: false }),
            }
        }
        self.cells.truncate(self.height as usize);
        self.explored.truncate(self.height as usize);
    }

    /// Change the size, keeping what is there (new squares are floor).
    pub fn resize(&mut self, width: u32, height: u32) {
        let (width, height) = (width.clamp(4, 200), height.clamp(4, 200));
        for rows in [&mut self.cells, &mut self.explored] {
            rows.truncate(height as usize);
            for r in rows.iter_mut() {
                if r.chars().count() > width as usize {
                    *r = r.chars().take(width as usize).collect();
                }
            }
        }
        self.width = width;
        self.height = height;
    }

    /// Edges between rock and open squares, joined into long runs.
    fn rock_edges(&self) -> Vec<Seg> {
        let (w, h) = (self.width as i32, self.height as i32);
        let mut out = Vec::new();
        // horizontal edges on the line y, between rows y-1 and y
        for y in 0..=h {
            let mut run: Option<i32> = None;
            for x in 0..=w {
                let edge = x < w && (self.solid(x, y - 1) != self.solid(x, y)) && (y > 0 && y < h);
                match (edge, run) {
                    (true, None) => run = Some(x),
                    (false, Some(s)) => {
                        out.push(Seg { a: (s as f32, y as f32), b: (x as f32, y as f32) });
                        run = None;
                    }
                    _ => {}
                }
            }
        }
        for x in 0..=w {
            let mut run: Option<i32> = None;
            for y in 0..=h {
                let edge = y < h && (self.solid(x - 1, y) != self.solid(x, y)) && (x > 0 && x < w);
                match (edge, run) {
                    (true, None) => run = Some(y),
                    (false, Some(s)) => {
                        out.push(Seg { a: (x as f32, s as f32), b: (x as f32, y as f32) });
                        run = None;
                    }
                    _ => {}
                }
            }
        }
        out
    }

    /// Everything that stops a creature seeing through.
    pub fn sight_blockers(&self) -> Vec<Seg> {
        let mut v = self.rock_edges();
        v.extend(self.walls.iter().filter(|w| w.blocks_sight()).map(|w| Seg { a: (w.x1, w.y1), b: (w.x2, w.y2) }));
        v
    }

    /// Everything that stops a creature walking through.
    pub fn move_blockers(&self) -> Vec<Seg> {
        let mut v = self.rock_edges();
        v.extend(self.walls.iter().filter(|w| w.blocks_move()).map(|w| Seg { a: (w.x1, w.y1), b: (w.x2, w.y2) }));
        v
    }

    /// Index of the door nearest to a point (in squares), within `reach`.
    pub fn door_near(&self, x: f32, y: f32, reach: f32) -> Option<usize> {
        self.walls
            .iter()
            .enumerate()
            .filter(|(_, w)| w.is_door())
            .map(|(i, w)| (i, point_seg_distance((x, y), (w.x1, w.y1), (w.x2, w.y2))))
            .filter(|(_, d)| *d <= reach)
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
    }

    /// Index of any wall or door nearest to a point, within `reach`.
    pub fn wall_near(&self, x: f32, y: f32, reach: f32) -> Option<usize> {
        self.walls
            .iter()
            .enumerate()
            .map(|(i, w)| (i, point_seg_distance((x, y), (w.x1, w.y1), (w.x2, w.y2))))
            .filter(|(_, d)| *d <= reach)
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
    }
}

fn set_in_rows(rows: &mut Vec<String>, width: u32, x: usize, y: usize, c: char, pad: char) {
    while rows.len() <= y {
        rows.push(String::new());
    }
    let mut chars: Vec<char> = rows[y].chars().collect();
    while chars.len() < (width as usize).max(x + 1) {
        chars.push(pad);
    }
    chars[x] = c;
    rows[y] = chars.into_iter().collect();
}

pub fn point_seg_distance(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let t = if len2 <= f32::EPSILON { 0.0 } else { (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0) };
    let (cx, cy) = (a.0 + t * dx, a.1 + t * dy);
    ((p.0 - cx).powi(2) + (p.1 - cy).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiles_rows_and_old_marks() {
        let mut m = MapState::new("t", 6, 4);
        assert_eq!(m.tile(2, 2), tile::FLOOR);
        assert!(m.solid(-1, 0), "outside the map is rock");
        m.set_tile(4, 3, tile::WATER);
        assert_eq!(m.cells.len(), 4);
        assert_eq!(m.tile(4, 3), tile::WATER);
        m.marks.insert("1,1".into(), "wall".into());
        m.marks.insert("2,1".into(), "altar of doom".into());
        m.normalize();
        assert!(m.solid(1, 1) && m.marks.is_empty());
        assert_eq!(m.markers[0].text, "altar of doom");
        m.set_explored(5, 0, true);
        assert!(m.is_explored(5, 0) && !m.is_explored(4, 0));
        m.resize(4, 4);
        assert!(!m.is_explored(5, 0));
    }

    #[test]
    fn rock_makes_walls_and_doors_open() {
        let mut m = MapState::new("t", 5, 3);
        m.set_tile(2, 0, tile::ROCK);
        m.set_tile(2, 1, tile::ROCK);
        m.set_tile(2, 2, tile::ROCK);
        // one run down each side of the rock column
        let segs = m.sight_blockers();
        assert_eq!(segs.len(), 2, "{segs:?}");
        assert!(segs.contains(&Seg { a: (2.0, 0.0), b: (2.0, 3.0) }));
        m.walls.push(Wall { x1: 4.0, y1: 0.0, x2: 4.0, y2: 1.0, kind: WallKind::Door, ..Default::default() });
        assert_eq!(m.sight_blockers().len(), 3);
        m.walls[0].open = true;
        assert_eq!(m.sight_blockers().len(), 2);
        assert_eq!(m.door_near(4.1, 0.5, 0.3), Some(0));
        // a map written by hand reads back
        let text = "name = \"Cellar\"\nwidth = 4\nheight = 2\ncells = [\"#..#\", \"#~~#\"]\nfog = \"vision\"\n[[walls]]\nx1 = 1.0\ny1 = 0.0\nx2 = 1.0\ny2 = 1.0\nkind = \"secret_door\"\n";
        let m: MapState = toml::from_str(text).unwrap();
        assert_eq!(m.tile(1, 1), tile::WATER);
        assert_eq!(m.fog, FogMode::Vision);
        assert_eq!(m.walls[0].kind, WallKind::SecretDoor);
        let again: MapState = toml::from_str(&toml::to_string_pretty(&m).unwrap()).unwrap();
        assert_eq!(again, m);
    }
}
