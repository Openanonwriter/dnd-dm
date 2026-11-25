//! Tiny deterministic PRNG (xoshiro256**). No external dependency, seedable
//! so tests and replays are reproducible.

use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct Rng {
    s: [u64; 4],
}

impl Rng {
    pub fn seeded(seed: u64) -> Self {
        // splitmix64 to expand the seed
        let mut x = seed;
        let mut next = || {
            x = x.wrapping_add(0x9E3779B97F4A7C15);
            let mut z = x;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
            z ^ (z >> 31)
        };
        Self { s: [next(), next(), next(), next()] }
    }

    pub fn from_time() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x1234_5678);
        Self::seeded(nanos)
    }

    pub fn next_u64(&mut self) -> u64 {
        let result = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        result
    }

    /// Uniform integer in `1..=sides`.
    pub fn roll_die(&mut self, sides: u32) -> u32 {
        if sides == 0 {
            return 0;
        }
        (self.next_u64() % sides as u64) as u32 + 1
    }
}

impl Default for Rng {
    fn default() -> Self {
        Self::from_time()
    }
}
