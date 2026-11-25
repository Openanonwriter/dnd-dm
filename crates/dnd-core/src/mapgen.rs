//! Quick maps: a dungeon of rooms and corridors, a cave, or open country.
//! They are ordinary maps afterwards; the DM edits them like any other.

use crate::grid::Pos;
use crate::model::*;
use crate::Rng;

fn below(rng: &mut Rng, n: u32) -> i32 {
    if n == 0 {
        0
    } else {
        (rng.next_u64() % n as u64) as i32
    }
}

fn between(rng: &mut Rng, lo: i32, hi: i32) -> i32 {
    lo + below(rng, (hi - lo + 1).max(1) as u32)
}

#[derive(Debug, Clone, Copy)]
struct Room {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

impl Room {
    fn center(&self) -> (i32, i32) {
        (self.x + self.w / 2, self.y + self.h / 2)
    }
    fn touches(&self, o: &Room) -> bool {
        self.x - 1 <= o.x + o.w && o.x - 1 <= self.x + self.w && self.y - 1 <= o.y + o.h && o.y - 1 <= self.y + self.h
    }
    fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

/// Rooms joined by corridors, a door at each room entrance, torches and a
/// little furniture. Dark, with vision fog on.
pub fn dungeon(name: &str, width: u32, height: u32, rng: &mut Rng) -> MapState {
    let mut m = MapState::new(name, width.clamp(16, 200), height.clamp(12, 200));
    m.fill(tile::ROCK);
    let (w, h) = (m.width as i32, m.height as i32);
    let mut rooms: Vec<Room> = Vec::new();
    let want = ((w * h) / 110).clamp(3, 24);
    for _ in 0..want * 12 {
        if rooms.len() as i32 >= want {
            break;
        }
        let r = Room { w: between(rng, 4, 9.min(w - 4)), h: between(rng, 3, 7.min(h - 4)), x: 0, y: 0 };
        let r = Room { x: between(rng, 1, (w - r.w - 1).max(1)), y: between(rng, 1, (h - r.h - 1).max(1)), ..r };
        if r.x + r.w >= w || r.y + r.h >= h || rooms.iter().any(|o| r.touches(o)) {
            continue;
        }
        for y in r.y..r.y + r.h {
            for x in r.x..r.x + r.w {
                m.set_tile(x, y, tile::FLOOR);
            }
        }
        rooms.push(r);
    }
    rooms.sort_by_key(|r| r.x + r.y);
    // corridors: an L between each room and the next
    for i in 1..rooms.len() {
        let (ax, ay) = rooms[i - 1].center();
        let (bx, by) = rooms[i].center();
        let horizontal_first = below(rng, 2) == 0;
        let mut carve = |m: &mut MapState, x: i32, y: i32| m.set_tile(x, y, tile::FLOOR);
        if horizontal_first {
            for x in ax.min(bx)..=ax.max(bx) {
                carve(&mut m, x, ay);
            }
            for y in ay.min(by)..=ay.max(by) {
                carve(&mut m, bx, y);
            }
        } else {
            for y in ay.min(by)..=ay.max(by) {
                carve(&mut m, ax, y);
            }
            for x in ax.min(bx)..=ax.max(bx) {
                carve(&mut m, x, by);
            }
        }
    }
    // doors where a one-square corridor meets a room wall
    for r in &rooms {
        for x in r.x..r.x + r.w {
            for (y, edge) in [(r.y - 1, r.y), (r.y + r.h, r.y + r.h)] {
                if !m.solid(x, y) && m.solid(x - 1, y) && m.solid(x + 1, y) && !rooms.iter().any(|o| o.contains(x, y)) {
                    m.walls.push(Wall { x1: x as f32, y1: edge as f32, x2: x as f32 + 1.0, y2: edge as f32, kind: WallKind::Door, ..Default::default() });
                }
            }
        }
        for y in r.y..r.y + r.h {
            for (x, edge) in [(r.x - 1, r.x), (r.x + r.w, r.x + r.w)] {
                if !m.solid(x, y) && m.solid(x, y - 1) && m.solid(x, y + 1) && !rooms.iter().any(|o| o.contains(x, y)) {
                    m.walls.push(Wall { x1: edge as f32, y1: y as f32, x2: edge as f32, y2: y as f32 + 1.0, kind: WallKind::Door, ..Default::default() });
                }
            }
        }
    }
    // dressing
    let kinds = ["chest", "barrel", "crate", "table", "bookshelf", "statue", "bones", "rug", "bed", "altar"];
    for (i, r) in rooms.iter().enumerate() {
        if below(rng, 3) > 0 {
            m.lights.push(Light { x: r.x as f32 + 0.5, y: r.y as f32 + 0.5, ..Default::default() });
        }
        if i > 0 && below(rng, 4) > 0 {
            let (x, y) = (between(rng, r.x, r.x + r.w - 1), between(rng, r.y, r.y + r.h - 1));
            let kind = kinds[below(rng, kinds.len() as u32) as usize];
            m.props.push(Prop { kind: kind.into(), x: x as f32 + 0.5, y: y as f32 + 0.5, ..Default::default() });
        }
        if r.w >= 7 && r.h >= 5 {
            for (x, y) in [(r.x + 1, r.y + 1), (r.x + r.w - 2, r.y + 1), (r.x + 1, r.y + r.h - 2), (r.x + r.w - 2, r.y + r.h - 2)] {
                m.props.push(Prop { kind: "pillar".into(), x: x as f32 + 0.5, y: y as f32 + 0.5, ..Default::default() });
            }
        }
    }
    if let Some(first) = rooms.first() {
        let (x, y) = first.center();
        m.start = Some(Pos::new(x, y));
        m.props.push(Prop { kind: "stairs".into(), x: first.x as f32 + 0.5, y: first.y as f32 + first.h as f32 - 0.5, ..Default::default() });
    }
    m.darkness = 1.0;
    m.fog = FogMode::Vision;
    m
}

/// A natural cave: random rock smoothed until it forms chambers, keeping
/// only the largest connected one.
pub fn cave(name: &str, width: u32, height: u32, rng: &mut Rng) -> MapState {
    let mut m = MapState::new(name, width.clamp(16, 200), height.clamp(12, 200));
    let (w, h) = (m.width as i32, m.height as i32);
    let mut rock = vec![vec![true; w as usize]; h as usize];
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            rock[y as usize][x as usize] = below(rng, 100) < 44;
        }
    }
    let at = |g: &Vec<Vec<bool>>, x: i32, y: i32| x < 0 || y < 0 || x >= w || y >= h || g[y as usize][x as usize];
    for _ in 0..5 {
        let old = rock.clone();
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let n = (-1..=1).flat_map(|dy| (-1..=1).map(move |dx| (dx, dy))).filter(|(dx, dy)| (*dx, *dy) != (0, 0) && at(&old, x + dx, y + dy)).count();
                rock[y as usize][x as usize] = n >= 5 || (n == 4 && old[y as usize][x as usize]);
            }
        }
    }
    // keep the biggest open area
    let mut label = vec![vec![0u32; w as usize]; h as usize];
    let (mut best, mut best_size, mut next) = (0u32, 0usize, 1u32);
    for y in 0..h {
        for x in 0..w {
            if rock[y as usize][x as usize] || label[y as usize][x as usize] != 0 {
                continue;
            }
            let mut stack = vec![(x, y)];
            let mut size = 0;
            label[y as usize][x as usize] = next;
            while let Some((cx, cy)) = stack.pop() {
                size += 1;
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let (nx, ny) = (cx + dx, cy + dy);
                    if nx >= 0 && ny >= 0 && nx < w && ny < h && !rock[ny as usize][nx as usize] && label[ny as usize][nx as usize] == 0 {
                        label[ny as usize][nx as usize] = next;
                        stack.push((nx, ny));
                    }
                }
            }
            if size > best_size {
                best_size = size;
                best = next;
            }
            next += 1;
        }
    }
    let mut open: Vec<(i32, i32)> = Vec::new();
    m.cells = (0..h)
        .map(|y| {
            (0..w)
                .map(|x| {
                    if label[y as usize][x as usize] == best && best != 0 {
                        open.push((x, y));
                        tile::DIRT
                    } else {
                        tile::ROCK
                    }
                })
                .collect()
        })
        .collect();
    // pools, rubble and stalagmites
    for &(x, y) in &open {
        match below(rng, 60) {
            0 => m.props.push(Prop { kind: "boulder".into(), x: x as f32 + 0.5, y: y as f32 + 0.5, ..Default::default() }),
            1 | 2 => m.set_tile(x, y, tile::RUBBLE),
            3 => {
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1), (-1, 0)] {
                    if !m.solid(x + dx, y + dy) {
                        m.set_tile(x + dx, y + dy, tile::WATER);
                    }
                }
            }
            _ => {}
        }
    }
    if !open.is_empty() {
        let (x, y) = open[open.len() / 2];
        m.set_tile(x, y, tile::DIRT);
        m.start = Some(Pos::new(x, y));
    }
    m.darkness = 1.0;
    m.fog = FogMode::Vision;
    m
}

/// Open country: grass with clumps of trees, a few rocks, a stream and a path.
pub fn wilderness(name: &str, width: u32, height: u32, rng: &mut Rng) -> MapState {
    let mut m = MapState::new(name, width.clamp(16, 200), height.clamp(12, 200));
    m.fill(tile::GRASS);
    let (w, h) = (m.width as i32, m.height as i32);
    // a wandering stream, top to bottom
    let mut x = between(rng, w / 5, w * 4 / 5);
    for y in 0..h {
        m.set_tile(x, y, tile::WATER);
        m.set_tile(x + 1, y, tile::WATER);
        x = (x + between(rng, -1, 1)).clamp(1, w - 3);
    }
    // a dirt path, left to right, with a ford
    let mut y = between(rng, h / 4, h * 3 / 4);
    for x in 0..w {
        if m.tile(x, y) != tile::WATER {
            m.set_tile(x, y, tile::DIRT);
        } else {
            m.set_tile(x, y, tile::RUBBLE);
        }
        if below(rng, 4) == 0 {
            y = (y + between(rng, -1, 1)).clamp(1, h - 2);
        }
    }
    m.start = Some(Pos::new(1, y.clamp(1, h - 2)));
    // clumps of trees
    for _ in 0..(w * h / 60).max(3) {
        let (cx, cy) = (between(rng, 0, w - 1), between(rng, 0, h - 1));
        for _ in 0..between(rng, 2, 7) {
            let (tx, ty) = (cx + between(rng, -2, 2), cy + between(rng, -2, 2));
            if m.in_bounds(tx, ty) && m.tile(tx, ty) == tile::GRASS && !m.props.iter().any(|p| p.x.floor() as i32 == tx && p.y.floor() as i32 == ty) {
                let kind = if below(rng, 5) == 0 { "bush" } else { "tree" };
                m.props.push(Prop { kind: kind.into(), x: tx as f32 + 0.5, y: ty as f32 + 0.5, ..Default::default() });
            }
        }
    }
    for _ in 0..(w * h / 150).max(2) {
        let (bx, by) = (between(rng, 0, w - 1), between(rng, 0, h - 1));
        if m.tile(bx, by) == tile::GRASS {
            m.props.push(Prop { kind: "boulder".into(), x: bx as f32 + 0.5, y: by as f32 + 0.5, ..Default::default() });
        }
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vision::find_path;

    #[test]
    fn generated_maps_are_playable() {
        for seed in 1..6u64 {
            let mut rng = Rng::seeded(seed);
            let d = dungeon("d", 40, 28, &mut rng);
            assert_eq!(d.cells.len(), 28);
            let start = d.start.expect("a dungeon has a way in");
            assert!(!d.solid(start.x, start.y));
            assert!(!d.walls.is_empty(), "rooms have doors (seed {seed})");
            let open = d.cells.iter().flat_map(|r| r.chars()).filter(|c| *c != tile::ROCK).count();
            assert!(open > 100, "seed {seed}: only {open} open squares");
            // every door can be reached once doors are opened
            let mut unlocked = d.clone();
            unlocked.walls.iter_mut().for_each(|w| w.open = true);
            let far = unlocked.props.iter().find(|p| p.kind != "stairs" && p.kind != "pillar");
            if let Some(p) = far {
                assert!(find_path(&unlocked, start, Pos::new(p.x as i32, p.y as i32), 1, 400).is_some(), "seed {seed}: {p:?} unreachable");
            }

            let c = cave("c", 40, 28, &mut rng);
            let s = c.start.expect("a cave has open ground");
            assert!(!c.solid(s.x, s.y));
            assert!(c.solid(0, 0) && c.solid(39, 27), "caves are closed in");

            let f = wilderness("w", 40, 28, &mut rng);
            assert!(f.props.iter().any(|p| p.kind == "tree"));
            assert!(f.cells.iter().any(|r| r.contains(tile::WATER)));
        }
    }
}
