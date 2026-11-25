//! Grid geometry: distances between tokens and the squares covered by
//! spheres, cubes, cones and lines. Positions are in squares; `0,0` is the
//! top-left of the map. A creature's `pos` is its top-left square and it
//! covers `size × size` squares.

use crate::model::{Combatant, DistanceRule, Rules};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
}

impl Pos {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// Distance between two squares, in squares.
pub fn distance_squares(a: Pos, b: Pos, rule: DistanceRule) -> f64 {
    let dx = (a.x - b.x).abs();
    let dy = (a.y - b.y).abs();
    match rule {
        DistanceRule::Chebyshev => dx.max(dy) as f64,
        DistanceRule::Alternating => {
            let diag = dx.min(dy);
            let straight = dx.max(dy) - diag;
            // every second diagonal costs 2
            (straight + diag + diag / 2) as f64
        }
        DistanceRule::Euclidean => ((dx * dx + dy * dy) as f64).sqrt(),
    }
}

/// Distance in feet, rounded down to a whole number.
pub fn distance_ft(a: Pos, b: Pos, rule: DistanceRule, feet_per_square: u32) -> i32 {
    (distance_squares(a, b, rule) * feet_per_square as f64).floor() as i32
}

/// Squares occupied by a creature at `origin` with the given side length.
pub fn footprint(origin: Pos, side: u32) -> Vec<Pos> {
    let side = side.max(1) as i32;
    let mut v = Vec::with_capacity((side * side) as usize);
    for dy in 0..side {
        for dx in 0..side {
            v.push(Pos::new(origin.x + dx, origin.y + dy));
        }
    }
    v
}

/// Shortest distance between any square of `a` and any square of `b`.
pub fn footprint_distance(a: &[Pos], b: &[Pos], rule: DistanceRule) -> f64 {
    let mut best = f64::INFINITY;
    for pa in a {
        for pb in b {
            let d = distance_squares(*pa, *pb, rule);
            if d < best {
                best = d;
            }
        }
    }
    best
}

pub fn combatant_footprint(c: &Combatant, rules: &Rules) -> Vec<Pos> {
    match c.pos {
        Some(p) => footprint(p, rules.size_squares(&c.size)),
        None => Vec::new(),
    }
}

/// Distance in feet between two combatants (edge to edge). `None` if either
/// has no position.
pub fn combatant_distance_ft(a: &Combatant, b: &Combatant, rules: &Rules, rule: DistanceRule) -> Option<i32> {
    let fa = combatant_footprint(a, rules);
    let fb = combatant_footprint(b, rules);
    if fa.is_empty() || fb.is_empty() {
        return None;
    }
    Some((footprint_distance(&fa, &fb, rule) * rules.feet_per_square as f64).floor() as i32)
}

/// All squares within `radius_ft` of `center` (a sphere / circle on the grid).
pub fn cells_in_sphere(center: Pos, radius_ft: u32, feet_per_square: u32, rule: DistanceRule) -> Vec<Pos> {
    let r = (radius_ft / feet_per_square.max(1)) as i32;
    let mut v = Vec::new();
    for y in (center.y - r)..=(center.y + r) {
        for x in (center.x - r)..=(center.x + r) {
            let p = Pos::new(x, y);
            if distance_squares(center, p, rule) <= r as f64 + 1e-9 {
                v.push(p);
            }
        }
    }
    v
}

/// Squares of a cube whose top-left corner is `origin`.
pub fn cells_in_cube(origin: Pos, size_ft: u32, feet_per_square: u32) -> Vec<Pos> {
    footprint(origin, (size_ft / feet_per_square.max(1)).max(1))
}

/// Squares along a line from `origin` toward `(dx, dy)` direction, `length_ft`
/// long and `width_ft` wide.
pub fn cells_in_line(origin: Pos, dir: (f64, f64), length_ft: u32, width_ft: u32, feet_per_square: u32) -> Vec<Pos> {
    let fps = feet_per_square.max(1) as f64;
    let len = length_ft as f64 / fps;
    let half_w = (width_ft as f64 / fps) / 2.0;
    let (ux, uy) = unit(dir);
    let r = len.ceil() as i32 + 1;
    let mut v = Vec::new();
    for y in (origin.y - r)..=(origin.y + r) {
        for x in (origin.x - r)..=(origin.x + r) {
            // vector from origin's center to this cell's center
            let vx = (x - origin.x) as f64;
            let vy = (y - origin.y) as f64;
            let along = vx * ux + vy * uy;
            let across = (vx * uy - vy * ux).abs();
            if along >= -1e-9 && along <= len - 0.5 + 1e-9 && across <= half_w.max(0.5) - 1e-9 + 0.5 {
                v.push(Pos::new(x, y));
            }
        }
    }
    v
}

/// Squares in a cone from `origin` toward `(dx, dy)`, `length_ft` long. A 5e
/// cone is as wide at any point as it is long at that point.
pub fn cells_in_cone(origin: Pos, dir: (f64, f64), length_ft: u32, feet_per_square: u32) -> Vec<Pos> {
    let fps = feet_per_square.max(1) as f64;
    let len = length_ft as f64 / fps;
    let (ux, uy) = unit(dir);
    let r = len.ceil() as i32 + 1;
    let mut v = Vec::new();
    for y in (origin.y - r)..=(origin.y + r) {
        for x in (origin.x - r)..=(origin.x + r) {
            let vx = (x - origin.x) as f64;
            let vy = (y - origin.y) as f64;
            let along = vx * ux + vy * uy;
            let across = (vx * uy - vy * ux).abs();
            if along > 0.0 && along <= len + 1e-9 && across <= along / 2.0 + 0.5 {
                v.push(Pos::new(x, y));
            }
        }
    }
    v
}

fn unit(dir: (f64, f64)) -> (f64, f64) {
    let m = (dir.0 * dir.0 + dir.1 * dir.1).sqrt();
    if m == 0.0 {
        (1.0, 0.0)
    } else {
        (dir.0 / m, dir.1 / m)
    }
}

/// Combatants with at least one occupied square inside `cells`.
pub fn combatants_in_cells<'a>(cells: &[Pos], combatants: &'a [Combatant], rules: &Rules) -> Vec<&'a Combatant> {
    combatants
        .iter()
        .filter(|c| combatant_footprint(c, rules).iter().any(|p| cells.contains(p)))
        .collect()
}

/// Combatants within `radius_ft` of `center` (any occupied square counts).
pub fn combatants_within<'a>(
    center: Pos,
    radius_ft: u32,
    combatants: &'a [Combatant],
    rules: &Rules,
    rule: DistanceRule,
) -> Vec<(&'a Combatant, i32)> {
    let mut out = Vec::new();
    for c in combatants {
        let fp = combatant_footprint(c, rules);
        if fp.is_empty() {
            continue;
        }
        let d = footprint_distance(&[center], &fp, rule) * rules.feet_per_square as f64;
        if d <= radius_ft as f64 + 1e-9 {
            out.push((c, d.floor() as i32));
        }
    }
    out.sort_by_key(|(_, d)| *d);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distances() {
        let a = Pos::new(0, 0);
        let b = Pos::new(3, 4);
        assert_eq!(distance_ft(a, b, DistanceRule::Chebyshev, 5), 20);
        assert_eq!(distance_ft(a, b, DistanceRule::Alternating, 5), 25); // 3 diag (5,10,5) + 1 straight
        assert_eq!(distance_ft(a, b, DistanceRule::Euclidean, 5), 25);
    }

    #[test]
    fn sphere_and_cube() {
        let cells = cells_in_sphere(Pos::new(5, 5), 10, 5, DistanceRule::Chebyshev);
        assert_eq!(cells.len(), 25);
        let cells = cells_in_sphere(Pos::new(5, 5), 10, 5, DistanceRule::Euclidean);
        assert_eq!(cells.len(), 13);
        assert_eq!(cells_in_cube(Pos::new(0, 0), 15, 5).len(), 9);
    }

    #[test]
    fn footprints() {
        let big = footprint(Pos::new(2, 2), 2);
        assert_eq!(big.len(), 4);
        let d = footprint_distance(&[Pos::new(0, 0)], &big, DistanceRule::Chebyshev);
        assert_eq!(d, 2.0);
    }

    #[test]
    fn cone_and_line() {
        let cone = cells_in_cone(Pos::new(0, 0), (1.0, 0.0), 15, 5);
        assert!(cone.contains(&Pos::new(1, 0)));
        assert!(cone.contains(&Pos::new(3, 1)));
        assert!(!cone.contains(&Pos::new(1, 2)));
        assert!(!cone.contains(&Pos::new(-1, 0)));
        let line = cells_in_line(Pos::new(0, 0), (0.0, 1.0), 30, 5, 5);
        assert!(line.contains(&Pos::new(0, 5)));
        assert!(!line.contains(&Pos::new(1, 3)));
        assert!(!line.contains(&Pos::new(0, 7)));
    }
}
