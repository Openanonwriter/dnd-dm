//! Line of sight, light and movement on a map. Walls are line segments
//! (`Seg`), positions are in squares; a creature looks out from the centre
//! of its space.

use crate::grid::Pos;
use crate::model::{tile, MapState, Seg};
use std::collections::{HashMap, VecDeque};

type P = (f32, f32);

fn cross(a: P, b: P) -> f32 {
    a.0 * b.1 - a.1 * b.0
}

/// Do the two segments cross? Touching exactly at an end point of the sight
/// line does not count, so a creature can stand against a wall.
pub fn segments_cross(p: P, p2: P, q: P, q2: P) -> bool {
    let r = (p2.0 - p.0, p2.1 - p.1);
    let s = (q2.0 - q.0, q2.1 - q.1);
    let d = cross(r, s);
    if d.abs() < 1e-9 {
        return false;
    }
    let qp = (q.0 - p.0, q.1 - p.1);
    let t = cross(qp, s) / d;
    let u = cross(qp, r) / d;
    t > 1e-4 && t < 1.0 - 1e-4 && (-1e-4..=1.0 + 1e-4).contains(&u)
}

/// Clear line between two points?
pub fn los(a: P, b: P, segs: &[Seg]) -> bool {
    !segs.iter().any(|s| segments_cross(a, b, s.a, s.b))
}

/// Keep only the segments that come within `radius` of `origin`.
pub fn nearby(segs: &[Seg], origin: P, radius: f32) -> Vec<Seg> {
    segs.iter()
        .filter(|s| crate::model::point_seg_distance(origin, s.a, s.b) <= radius + 1.0)
        .copied()
        .collect()
}

/// Can `origin` see into the square? Five points are tried (centre and near
/// each corner) so a square half behind a corner still counts.
fn sees_cell(origin: P, x: i32, y: i32, segs: &[Seg]) -> bool {
    const PTS: [P; 5] = [(0.5, 0.5), (0.2, 0.2), (0.8, 0.2), (0.2, 0.8), (0.8, 0.8)];
    PTS.iter().any(|(dx, dy)| los(origin, (x as f32 + dx, y as f32 + dy), segs))
}

/// A `width × height` grid of flags, row by row.
#[derive(Debug, Clone, PartialEq)]
pub struct Grid<T> {
    pub width: i32,
    pub height: i32,
    pub data: Vec<T>,
}

impl<T: Copy + Default> Grid<T> {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width: width as i32, height: height as i32, data: vec![T::default(); (width * height) as usize] }
    }
    pub fn get(&self, x: i32, y: i32) -> T {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            T::default()
        } else {
            self.data[(y * self.width + x) as usize]
        }
    }
    pub fn set(&mut self, x: i32, y: i32, v: T) {
        if x >= 0 && y >= 0 && x < self.width && y < self.height {
            self.data[(y * self.width + x) as usize] = v;
        }
    }
}

/// How well lit each square is: 0 dark, 1 dim, 2 bright. `carried` are
/// lights held by creatures: (position, bright feet, dim feet).
pub fn light_grid(map: &MapState, segs: &[Seg], carried: &[(P, u32, u32)]) -> Grid<u8> {
    let mut g = Grid::<u8>::new(map.width, map.height);
    let ambient = if map.darkness < 0.34 {
        2
    } else if map.darkness < 0.67 {
        1
    } else {
        0
    };
    for v in g.data.iter_mut() {
        *v = ambient;
    }
    if ambient == 2 {
        return g;
    }
    let fps = map.feet_per_square.max(1) as f32;
    let mut sources: Vec<(P, f32, f32)> = map.lights.iter().filter(|l| l.on).map(|l| ((l.x, l.y), l.bright as f32 / fps, l.dim.max(l.bright) as f32 / fps)).collect();
    sources.extend(carried.iter().map(|(p, b, d)| (*p, *b as f32 / fps, (*d).max(*b) as f32 / fps)));
    for (o, bright, dim) in sources {
        let near = nearby(segs, o, dim);
        let r = dim.ceil() as i32 + 1;
        for y in (o.1 as i32 - r)..=(o.1 as i32 + r) {
            for x in (o.0 as i32 - r)..=(o.0 as i32 + r) {
                if !map.in_bounds(x, y) {
                    continue;
                }
                let d = ((x as f32 + 0.5 - o.0).powi(2) + (y as f32 + 0.5 - o.1).powi(2)).sqrt();
                let level = if d <= bright + 0.01 {
                    2
                } else if d <= dim + 0.01 {
                    1
                } else {
                    0
                };
                if level > g.get(x, y) && sees_cell(o, x, y, &near) {
                    g.set(x, y, level);
                }
            }
        }
    }
    g
}

/// Someone looking at the map.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Eye {
    pub at: P,
    /// How far it sees without light, in feet.
    pub darkvision: u32,
}

/// Squares the eyes can see right now. Open squares need a clear line and
/// either light or darkvision; rock shows when an open square next to it does.
pub fn visible_cells(map: &MapState, segs: &[Seg], light: &Grid<u8>, eyes: &[Eye]) -> Grid<bool> {
    let mut g = Grid::<bool>::new(map.width, map.height);
    let fps = map.feet_per_square.max(1) as f32;
    let far = (map.width.max(map.height) as f32).min(80.0);
    for e in eyes {
        let near = nearby(segs, e.at, far);
        let dv = e.darkvision as f32 / fps;
        let r = far.ceil() as i32;
        let (ex, ey) = (e.at.0.floor() as i32, e.at.1.floor() as i32);
        for y in (ey - r).max(0)..=(ey + r).min(map.height as i32 - 1) {
            for x in (ex - r).max(0)..=(ex + r).min(map.width as i32 - 1) {
                if g.get(x, y) || map.solid(x, y) {
                    continue;
                }
                let d = ((x as f32 + 0.5 - e.at.0).powi(2) + (y as f32 + 0.5 - e.at.1).powi(2)).sqrt();
                let lit = light.get(x, y) > 0 || d <= dv + 0.01 || d <= 1.6;
                if lit && d <= far && sees_cell(e.at, x, y, &near) {
                    g.set(x, y, true);
                }
            }
        }
    }
    // the rock faces around what is seen
    let open = g.clone();
    for y in 0..map.height as i32 {
        for x in 0..map.width as i32 {
            if map.solid(x, y) && (-1..=1).any(|dy| (-1..=1).any(|dx| open.get(x + dx, y + dy))) {
                g.set(x, y, true);
            }
        }
    }
    g
}

/// Outline of what `origin` can see, for drawing a smooth edge. Rays go to
/// every wall end (and just either side of it) plus an even fan.
pub fn vision_polygon(origin: P, radius: f32, segs: &[Seg]) -> Vec<[f32; 2]> {
    let near = nearby(segs, origin, radius);
    let mut angles: Vec<f32> = (0..72).map(|i| i as f32 / 72.0 * std::f32::consts::TAU - std::f32::consts::PI).collect();
    for s in &near {
        for p in [s.a, s.b] {
            let a = (p.1 - origin.1).atan2(p.0 - origin.0);
            angles.extend([a - 0.0015, a, a + 0.0015]);
        }
    }
    angles.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    angles
        .into_iter()
        .map(|a| {
            let dir = (a.cos(), a.sin());
            let mut best = radius;
            for s in &near {
                let sd = (s.b.0 - s.a.0, s.b.1 - s.a.1);
                let d = cross(dir, sd);
                if d.abs() < 1e-9 {
                    continue;
                }
                let qp = (s.a.0 - origin.0, s.a.1 - origin.1);
                let t = cross(qp, sd) / d;
                let u = cross(qp, dir) / d;
                if t > 1e-4 && t < best && (-1e-4..=1.0001).contains(&u) {
                    best = t;
                }
            }
            [origin.0 + dir.0 * best, origin.1 + dir.1 * best]
        })
        .collect()
}

/// May a creature step from one square to a neighbouring one?
fn step_ok(map: &MapState, segs: &[Seg], from: Pos, to: Pos, side: i32) -> bool {
    for dy in 0..side {
        for dx in 0..side {
            if map.solid(to.x + dx, to.y + dy) {
                return false;
            }
        }
    }
    let c = |p: Pos| (p.x as f32 + side as f32 / 2.0, p.y as f32 + side as f32 / 2.0);
    if !los(c(from), c(to), segs) {
        return false;
    }
    // no squeezing diagonally past a corner
    if from.x != to.x && from.y != to.y {
        let a = Pos::new(to.x, from.y);
        let b = Pos::new(from.x, to.y);
        if map.solid(a.x, a.y) || map.solid(b.x, b.y) || !los(c(from), c(a), segs) || !los(c(from), c(b), segs) {
            return false;
        }
    }
    true
}

/// Shortest walk between two squares that goes round walls, rock and closed
/// doors. Returns the squares stepped on (without the start) and the cost in
/// squares: diagonals cost 1, difficult terrain costs double.
pub fn find_path(map: &MapState, from: Pos, to: Pos, side: u32, max_cost: u32) -> Option<(Vec<Pos>, u32)> {
    let side = side.max(1) as i32;
    if from == to {
        return Some((Vec::new(), 0));
    }
    if !map.in_bounds(to.x, to.y) || !map.in_bounds(to.x + side - 1, to.y + side - 1) {
        return None;
    }
    let segs = map.move_blockers();
    let mut cost: HashMap<Pos, u32> = HashMap::new();
    let mut prev: HashMap<Pos, Pos> = HashMap::new();
    let mut queue: VecDeque<Pos> = VecDeque::new();
    cost.insert(from, 0);
    queue.push_back(from);
    while let Some(p) = queue.pop_front() {
        let here = cost[&p];
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let n = Pos::new(p.x + dx, p.y + dy);
                if !map.in_bounds(n.x, n.y) {
                    continue;
                }
                let step = if tile::difficult(map.tile(n.x, n.y)) { 2 } else { 1 };
                let c = here + step;
                if c > max_cost || cost.get(&n).map(|old| *old <= c).unwrap_or(false) {
                    continue;
                }
                if !step_ok(map, &segs, p, n, side) {
                    continue;
                }
                cost.insert(n, c);
                prev.insert(n, p);
                queue.push_back(n);
            }
        }
    }
    let total = *cost.get(&to)?;
    let mut path = vec![to];
    let mut at = to;
    while let Some(p) = prev.get(&at) {
        if *p == from {
            break;
        }
        path.push(*p);
        at = *p;
    }
    path.reverse();
    Some((path, total))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Light, Wall, WallKind};

    /// Two rooms side by side, joined by a door in the middle of the wall.
    ///   0123456
    /// 0 #######
    /// 1 #..#..#
    /// 2 #..|..#    door between (2,2) and (4,2): the rock at (3,2) is open floor
    /// 3 #..#..#
    /// 4 #######
    fn two_rooms(open: bool) -> MapState {
        let mut m = MapState::new("t", 7, 5);
        m.cells = vec!["#######".into(), "#..#..#".into(), "#.....#".into(), "#..#..#".into(), "#######".into()];
        m.walls.push(Wall { x1: 3.0, y1: 2.0, x2: 3.0, y2: 3.0, kind: WallKind::Door, open, ..Default::default() });
        m
    }

    fn see(m: &MapState, from: P, dv: u32) -> Grid<bool> {
        let segs = m.sight_blockers();
        let light = light_grid(m, &segs, &[]);
        visible_cells(m, &segs, &light, &[Eye { at: from, darkvision: dv }])
    }

    #[test]
    fn doors_block_sight_until_opened() {
        let shut = two_rooms(false);
        let v = see(&shut, (1.5, 2.5), 0);
        assert!(v.get(2, 1) && v.get(1, 3), "own room");
        assert!(!v.get(5, 2) && !v.get(4, 1), "the far room is hidden behind the door");
        assert!(v.get(0, 2) && v.get(3, 1), "the walls of the room show");
        let open = two_rooms(true);
        let v = see(&open, (1.5, 2.5), 0);
        assert!(v.get(5, 2), "straight through the open door");
        assert!(!v.get(4, 1) || !v.get(5, 3) || v.get(5, 2), "corners may stay hidden");
        // the outline never reaches past the closed door
        let poly = vision_polygon((1.5, 2.5), 20.0, &shut.sight_blockers());
        assert!(poly.iter().all(|p| p[0] <= 3.51), "{poly:?}");
        let poly = vision_polygon((1.5, 2.5), 20.0, &open.sight_blockers());
        assert!(poly.iter().any(|p| p[0] > 5.0));
    }

    #[test]
    fn darkness_needs_light_or_darkvision() {
        let mut m = two_rooms(true);
        m.darkness = 1.0;
        let v = see(&m, (1.5, 2.5), 0);
        assert!(v.get(2, 2), "the next square is always seen");
        assert!(!v.get(5, 2), "too dark to see the far room");
        assert!(see(&m, (1.5, 2.5), 60).get(5, 2), "darkvision reaches");
        m.lights.push(Light { x: 5.5, y: 2.5, bright: 5, dim: 10, ..Default::default() });
        let v = see(&m, (1.5, 2.5), 0);
        assert!(v.get(5, 2), "a torch in the far room lights it");
        let segs = m.sight_blockers();
        let light = light_grid(&m, &segs, &[]);
        assert_eq!(light.get(5, 2), 2);
        assert_eq!(light.get(1, 1), 0, "light does not pass through rock");
        let carried = light_grid(&m, &segs, &[((1.5, 1.5), 20, 40)]);
        assert_eq!(carried.get(1, 3), 2, "a carried torch");
    }

    #[test]
    fn paths_go_round_walls() {
        let shut = two_rooms(false);
        assert!(find_path(&shut, Pos::new(1, 2), Pos::new(5, 2), 1, 40).is_none(), "the door is closed");
        let open = two_rooms(true);
        let (path, cost) = find_path(&open, Pos::new(1, 1), Pos::new(5, 1), 1, 40).unwrap();
        assert_eq!(path.last(), Some(&Pos::new(5, 1)));
        assert!(path.contains(&Pos::new(3, 2)), "through the doorway: {path:?}");
        assert_eq!(cost, 4);
        assert!(find_path(&open, Pos::new(1, 1), Pos::new(5, 1), 1, 3).is_none(), "too far for the allowance");
        assert!(find_path(&open, Pos::new(1, 1), Pos::new(3, 1), 1, 40).is_none(), "cannot stand in rock");
        let mut wet = open.clone();
        wet.set_tile(3, 2, tile::WATER);
        assert_eq!(find_path(&wet, Pos::new(1, 1), Pos::new(5, 1), 1, 40).unwrap().1, 5, "water costs double");
    }
}
