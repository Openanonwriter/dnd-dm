//! How far an attack or spell reaches, read from the words the content
//! uses ("reach 5 ft.", "range 80/320 ft.", "20/60", "120 feet", "Touch",
//! "Self (15-foot cone)"), and what the distance to a target then means.

use serde::{Deserialize, Serialize};

/// Reach of one attack or spell, in feet. 0 = it does not work that way.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reach {
    /// Melee reach (5 for most weapons and for touch spells, 10 with reach).
    #[serde(default)]
    pub melee: u32,
    /// Range without penalty.
    #[serde(default)]
    pub normal: u32,
    /// Furthest range; between `normal` and this the roll has disadvantage.
    #[serde(default)]
    pub long: u32,
    /// Sight, unlimited, special: no distance to check.
    #[serde(default)]
    pub unlimited: bool,
    /// "Self" with no area: only the caster.
    #[serde(default)]
    pub self_only: bool,
}

/// How a target at some distance can be attacked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InRange {
    Melee,
    Ranged,
    /// Beyond normal range: disadvantage.
    Long,
}

fn numbers(s: &str) -> Vec<u32> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for c in s.chars().chain(std::iter::once(' ')) {
        if c.is_ascii_digit() {
            cur.push(c);
        } else if !cur.is_empty() {
            if let Ok(n) = cur.parse() {
                out.push(n);
            }
            cur.clear();
        }
    }
    out
}

impl Reach {
    pub fn melee(feet: u32) -> Self {
        Self { melee: feet, ..Default::default() }
    }

    /// Read a reach from content text. Unknown wording means "no limit",
    /// so homebrew never becomes unusable because of how it is phrased.
    pub fn parse(text: &str) -> Self {
        let t = text.trim().to_lowercase();
        let mut r = Reach::default();
        if t.is_empty() || t.contains("sight") || t.contains("unlimited") || t.contains("special") || t.contains("mile") {
            r.unlimited = true;
            return r;
        }
        if t.starts_with("self") {
            match numbers(&t).first() {
                Some(n) => {
                    r.normal = *n;
                    r.long = *n;
                }
                None => r.self_only = true,
            }
            return r;
        }
        if t.contains("touch") {
            r.melee = 5;
            return r;
        }
        for seg in t.split(" or ").flat_map(|s| s.split(',')) {
            let n = numbers(seg);
            let Some(first) = n.first().copied() else { continue };
            if seg.contains('/') && n.len() >= 2 {
                r.normal = first;
                r.long = n[1].max(first);
            } else if seg.contains("reach") {
                r.melee = first;
            } else if seg.contains("range") || seg.contains("feet") || seg.contains("foot") {
                r.normal = first;
                r.long = first;
            } else if seg.contains("ft") {
                if first <= 15 {
                    r.melee = first;
                } else {
                    r.normal = first;
                    r.long = first;
                }
            }
        }
        if r.melee == 0 && r.normal == 0 {
            r.unlimited = true;
        }
        r
    }

    /// Stretch the ranged part (a feat or feature that lengthens range).
    pub fn scaled(mut self, percent: u32) -> Self {
        if percent != 100 && percent > 0 {
            self.normal = self.normal * percent / 100;
            self.long = self.long * percent / 100;
        }
        self
    }

    /// Let a spell's area reach past the point it is aimed at.
    pub fn widened(mut self, feet: u32) -> Self {
        if self.normal > 0 {
            self.normal += feet;
            self.long += feet;
        }
        self
    }

    /// The furthest this can reach at all.
    pub fn max(&self) -> u32 {
        self.melee.max(self.long).max(self.normal)
    }

    /// Can something `feet` away be reached, and how?
    pub fn judge(&self, feet: u32) -> Option<InRange> {
        if self.unlimited {
            return Some(InRange::Ranged);
        }
        if self.melee > 0 && feet <= self.melee {
            return Some(InRange::Melee);
        }
        if self.normal > 0 {
            if feet <= self.normal {
                return Some(InRange::Ranged);
            }
            if feet <= self.long {
                return Some(InRange::Long);
            }
        }
        None
    }

    /// "5 ft.", "80/320 ft.", "5 ft. or 20/60 ft."
    pub fn describe(&self) -> String {
        let mut v = Vec::new();
        if self.melee > 0 {
            v.push(format!("{} ft.", self.melee));
        }
        if self.normal > 0 {
            v.push(if self.long > self.normal { format!("{}/{} ft.", self.normal, self.long) } else { format!("{} ft.", self.normal) });
        }
        if v.is_empty() {
            return if self.self_only { "self".into() } else { "any distance".into() };
        }
        v.join(" or ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_usual_wordings() {
        assert_eq!(Reach::parse("reach 5 ft., one target"), Reach::melee(5));
        assert_eq!(Reach::parse("5 ft."), Reach::melee(5));
        assert_eq!(Reach::parse("10 ft."), Reach::melee(10));
        let bow = Reach::parse("range 80/320 ft., one target");
        assert_eq!((bow.melee, bow.normal, bow.long), (0, 80, 320));
        let spear = Reach::parse("reach 5 ft. or range 20/60 ft., one target");
        assert_eq!((spear.melee, spear.normal, spear.long), (5, 20, 60));
        assert_eq!(Reach::parse("20/60 ft.").long, 60);
        let bolt = Reach::parse("120 feet");
        assert_eq!((bolt.normal, bolt.long), (120, 120));
        assert_eq!(Reach::parse("Touch"), Reach::melee(5));
        assert!(Reach::parse("Self").self_only);
        assert_eq!(Reach::parse("Self (15-foot cone)").normal, 15);
        assert!(Reach::parse("Sight").unlimited);
        assert!(Reach::parse("1 mile").unlimited);
        assert!(Reach::parse("whatever the DM says").unlimited);
    }

    #[test]
    fn judges_distance() {
        let spear = Reach::parse("reach 5 ft. or range 20/60 ft.");
        assert_eq!(spear.judge(5), Some(InRange::Melee));
        assert_eq!(spear.judge(20), Some(InRange::Ranged));
        assert_eq!(spear.judge(45), Some(InRange::Long));
        assert_eq!(spear.judge(65), None);
        assert_eq!(Reach::melee(5).judge(10), None);
        assert_eq!(Reach::parse("120 feet").scaled(200).judge(240), Some(InRange::Ranged));
        assert_eq!(Reach::parse("150 feet").widened(20).judge(170), Some(InRange::Ranged));
        assert_eq!(spear.describe(), "5 ft. or 20/60 ft.");
    }
}
