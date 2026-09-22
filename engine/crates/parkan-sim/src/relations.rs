//! A clan brain's relations: the word, the attitude behind it, and the takt that re-reads one
//! from the other. See `docs/25-sensors.md`, "Clan relations".
//!
//! Each clan's SuperAI keeps one 16-byte record per clan (`ai.dll`, the array at the object's
//! `+0x43c`, 64 of them): the attitude, a pending decrease, a pending increase and the word.
//! The takt (`0x10005f30`, run every 7000–7999 ms from `0x100017f0`) applies whatever is
//! pending, holds the attitude to 0..1, reads the word off it and then drifts the attitude
//! toward its band's rest point by 0.0033 — never across the band's edge.
//!
//! The only thing that ever writes a change field is a hit: `Behavior.dll:0x1000658c` calls
//! the SuperAI's slot 13 (`ai.dll:0x10001fe0`) for every hit a non-hero unit takes, and that
//! adds a fixed 0.004 to the **decrease** toward the firer's clan. Nothing anywhere writes the
//! increase, so a relation can only ever fall.

/// The relation words, as `parkan_formats::mission` names them.
pub const HOSTILE: u32 = 0;
pub const NEUTRAL: u32 = 1;
pub const ALLIED: u32 = 2;

/// The attitude a word is given when it is set (`ai.dll:0x10005e80`): hostile, neutral, allied.
pub const SET: [f32; 3] = [0.166_65, 0.499_95, 0.833_3];
/// The band edges the takt reads a word off (`0x10005fa6`, `0x10005fc0`).
pub const LOW_EDGE: f32 = 0.333_3;
pub const HIGH_EDGE: f32 = 0.666_6;
/// Where each band's drift comes to rest (`0x10034488`, `0x10034480`, `0x1003447c`).
pub const REST: [f32; 3] = [0.283_3, 0.499_95, 0.716_6];
/// How far the attitude drifts a takt (`0x10034484`).
pub const DRIFT: f32 = 0.003_305_166_5;
/// What one hit adds to the decrease (`ai.dll:0x10001ff7`, the float at `0x10034284`). The
/// caller passes an amount of its own (1.0, `Behavior.dll:0x1000657f`) and the slot ignores it.
pub const HIT: f32 = 0.004;
/// The takt's period: 7000 ms plus a jitter of 0..999 (`ai.dll:0x100017f0`).
pub const TAKT_MS: f64 = 7000.0;
pub const TAKT_JITTER_MS: f64 = 1000.0;

/// One clan's record about another.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Pair {
    /// The record's `+0`.
    attitude: f32,
    /// Its `+4`, which the takt subtracts and clears.
    down: f32,
    /// Its `+8`, which the takt adds and clears. Nothing writes it.
    up: f32,
    /// Its `+0xc`, the word every other system reads.
    word: u32,
}

impl Pair {
    fn new(word: u32) -> Self {
        Self { attitude: SET[word.min(2) as usize], down: 0.0, up: 0.0, word }
    }
}

/// Every clan's relations, and when each clan's brain takes its next takt.
#[derive(Clone, Debug)]
pub struct Relations {
    pairs: Vec<Vec<Pair>>,
    /// Each clan's next takt, in milliseconds of the play's clock.
    due: Vec<f64>,
    seed: u32,
}

impl Relations {
    /// The relations a mission's word matrix starts the game with: `iron3d.dll:0x100a2773`
    /// hands every word to the clan's SuperAI through slot 7, which also sets the attitude.
    pub fn new(words: &[Vec<u32>]) -> Self {
        let pairs = words.iter().map(|row| row.iter().map(|&w| Pair::new(w)).collect()).collect();
        Self { pairs, due: vec![0.0; words.len()], seed: 0x2545_f491 }
    }

    pub fn clans(&self) -> usize {
        self.pairs.len()
    }

    /// Clan `us`'s word towards clan `them`.
    pub fn word(&self, us: usize, them: usize) -> Option<u32> {
        self.pairs.get(us)?.get(them).map(|p| p.word)
    }

    /// Clan `us`'s attitude towards clan `them`.
    pub fn attitude(&self, us: usize, them: usize) -> Option<f32> {
        self.pairs.get(us)?.get(them).map(|p| p.attitude)
    }

    /// The word matrix as the rest of the engine reads it.
    pub fn words(&self) -> Vec<Vec<u32>> {
        self.pairs.iter().map(|row| row.iter().map(|p| p.word).collect()).collect()
    }

    /// Set clan `us`'s word towards `them` (slot 7, `ai.dll:0x10005e80`): the attitude follows
    /// the word, and `them`'s record about `us` takes both.
    pub fn set(&mut self, us: usize, them: usize, word: u32) {
        if word > ALLIED || us >= self.pairs.len() || them >= self.pairs.len() {
            return;
        }
        let pair = Pair::new(word);
        self.pairs[us][them] = pair;
        self.pairs[them][us].attitude = pair.attitude;
        self.pairs[them][us].word = word;
    }

    /// A hit on a unit of clan `us` fired by a unit of clan `them` (`ai.dll:0x10001fe0`, the
    /// SuperAI's slot 13): 0.004 onto the decrease the next takt will apply. The caller checks
    /// that the victim is not a hero, not a building and not in a network game
    /// (`Behavior.dll:0x100064b0`).
    pub fn hurt(&mut self, us: usize, them: usize) {
        if us == them || them >= 0x40 {
            return;
        }
        if let Some(pair) = self.pairs.get_mut(us).and_then(|row| row.get_mut(them)) {
            pair.down += HIT;
        }
    }

    /// Run every clan brain whose takt is due at `now_ms`, and say whether any word changed.
    pub fn takt(&mut self, now_ms: f64) -> bool {
        let mut moved = false;
        for us in 0..self.pairs.len() {
            if now_ms < self.due[us] {
                continue;
            }
            self.due[us] = now_ms + TAKT_MS + f64::from(self.random()) * TAKT_JITTER_MS;
            moved |= self.takt_one(us);
        }
        moved
    }

    /// One clan brain's pass over its records (`ai.dll:0x10005f30`), skipping its own.
    fn takt_one(&mut self, us: usize) -> bool {
        let mut moved = false;
        for them in 0..self.pairs[us].len() {
            if them == us {
                continue;
            }
            let mut pair = self.pairs[us][them];
            let was = pair.word;
            if pair.down != 0.0 {
                pair.attitude -= pair.down;
                pair.down = 0.0;
            }
            if pair.up != 0.0 {
                pair.attitude += pair.up;
                pair.up = 0.0;
            }
            pair.attitude = pair.attitude.clamp(0.0, 1.0);
            pair.word = band(pair.attitude);
            drift(&mut pair);
            self.pairs[us][them] = pair;
            // Only a changed word reaches the other clan's brain (`0x100060a3`), and it takes
            // this clan's attitude with it.
            if pair.word != was {
                moved = true;
                self.pairs[them][us].word = pair.word;
                self.pairs[them][us].attitude = pair.attitude;
            }
        }
        moved
    }

    /// STAND-IN: docs/25-sensors.md#clan-relations-the-files-words-straight-through--read-and-measured
    /// -- the takt jitter's random source is not read: a 32-bit xorshift. 0..1.
    fn random(&mut self) -> f32 {
        let mut x = self.seed;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.seed = x;
        (x >> 8) as f32 / (1u32 << 24) as f32
    }
}

/// The word an attitude reads as (`ai.dll:0x10005fa4`-`0x10005ff0`).
pub fn band(attitude: f32) -> u32 {
    if attitude < LOW_EDGE {
        HOSTILE
    } else if attitude < HIGH_EDGE {
        NEUTRAL
    } else {
        ALLIED
    }
}

/// The drift the takt applies after reading the word: toward the band's rest point, and held
/// there, so it never crosses an edge (`0x10005ffa`-`0x10006092`).
fn drift(pair: &mut Pair) {
    let rest = REST[pair.word.min(2) as usize];
    match pair.word {
        // Hostile rises to 0.2833 and stops: a hostile clan never becomes neutral by itself.
        HOSTILE => {
            if pair.attitude < rest {
                pair.attitude += DRIFT;
                if pair.attitude > rest {
                    pair.attitude = rest;
                }
            }
        }
        // Neutral moves toward 0.49995 from either side, with no hold.
        NEUTRAL => {
            if pair.attitude < rest {
                pair.attitude += DRIFT;
            } else if pair.attitude > rest {
                pair.attitude -= DRIFT;
            }
        }
        // Allied falls to 0.7166 and stops. Below it nothing moves, so a hit's loss is kept.
        _ => {
            if pair.attitude > rest {
                pair.attitude -= DRIFT;
                if pair.attitude < rest {
                    pair.attitude = rest;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two(word: u32) -> Relations {
        Relations::new(&[vec![ALLIED, word], vec![word, ALLIED]])
    }

    #[test]
    fn a_word_sets_its_attitude() {
        let r = two(ALLIED);
        assert_eq!(r.attitude(0, 1), Some(SET[ALLIED as usize]));
        assert_eq!(r.attitude(1, 0), Some(SET[ALLIED as usize]));
        assert_eq!(two(HOSTILE).attitude(0, 1), Some(SET[HOSTILE as usize]));
    }

    /// With nothing pending the takt only drifts, and the drift stops at the band's rest
    /// point: an allied attitude falls from 0.8333 to 0.7166 and stays.
    #[test]
    fn the_drift_stops_at_the_rest_point() {
        let mut r = two(ALLIED);
        let mut now = 0.0;
        for _ in 0..200 {
            now += 8000.0;
            r.takt(now);
        }
        assert_eq!(r.word(0, 1), Some(ALLIED));
        assert_eq!(r.attitude(0, 1), Some(REST[ALLIED as usize]));
        let mut r = two(HOSTILE);
        let mut now = 0.0;
        for _ in 0..200 {
            now += 8000.0;
            r.takt(now);
        }
        assert_eq!(r.word(0, 1), Some(HOSTILE));
        assert_eq!(r.attitude(0, 1), Some(REST[HOSTILE as usize]));
    }

    /// 13 hits take an ally at rest below 2/3 and make it neutral, both ways; 12 do not.
    /// 0.7166 - 13 x 0.004 = 0.6646, under the 0.6666 edge.
    #[test]
    fn thirteen_hits_lose_an_ally() {
        for (hits, want) in [(12, ALLIED), (13, NEUTRAL)] {
            let mut r = two(ALLIED);
            let mut now = 0.0;
            for _ in 0..100 {
                now += 8000.0;
                r.takt(now);
            }
            assert_eq!(r.attitude(0, 1), Some(REST[ALLIED as usize]));
            for _ in 0..hits {
                r.hurt(0, 1);
            }
            now += 8000.0;
            r.takt(now);
            assert_eq!(r.word(0, 1), Some(want), "{hits} hits");
            assert_eq!(r.word(1, 0), Some(want), "{hits} hits, the other way");
        }
    }

    /// 42 hits in one takt take a neutral clan hostile from its rest point; 41 do not.
    /// 0.49995 - 42 x 0.004 = 0.33195, under the 1/3 edge.
    #[test]
    fn forty_two_hits_make_an_enemy() {
        for (hits, want) in [(41, NEUTRAL), (42, HOSTILE)] {
            let mut r = two(NEUTRAL);
            for _ in 0..hits {
                r.hurt(0, 1);
            }
            r.takt(1.0);
            assert_eq!(r.word(0, 1), Some(want), "{hits} hits");
        }
    }

    /// A hit's loss is kept in the allied band -- the drift does not carry the attitude back
    /// above 0.7166 -- so a trickle of hits still ends an alliance.
    #[test]
    fn an_allys_losses_accumulate() {
        let mut r = two(ALLIED);
        let mut now = 0.0;
        for _ in 0..100 {
            now += 8000.0;
            r.takt(now);
        }
        for _ in 0..13 {
            r.hurt(0, 1);
            now += 8000.0;
            r.takt(now);
        }
        assert_eq!(r.word(0, 1), Some(NEUTRAL));
    }

    /// Nothing raises a relation: no hit count and no waiting turns a hostile clan neutral.
    #[test]
    fn a_relation_never_rises() {
        let mut r = two(HOSTILE);
        let mut now = 0.0;
        for _ in 0..1000 {
            now += 8000.0;
            r.takt(now);
        }
        assert_eq!(r.word(0, 1), Some(HOSTILE));
        assert!(r.attitude(0, 1).unwrap() < LOW_EDGE);
    }

    /// The takt runs on its own 7-8 s period, not once a tick.
    #[test]
    fn the_takt_waits_its_period() {
        let mut r = two(ALLIED);
        r.takt(0.0);
        let after = r.attitude(0, 1);
        for step in 1..70 {
            r.takt(f64::from(step) * 100.0);
        }
        assert_eq!(r.attitude(0, 1), after, "nothing moves before 7 s");
        r.takt(8000.0);
        assert_ne!(r.attitude(0, 1), after);
    }

    #[test]
    fn a_clan_never_looks_at_itself() {
        let mut r = two(ALLIED);
        r.takt(1.0);
        assert_eq!(r.word(0, 0), Some(ALLIED));
        assert_eq!(r.attitude(0, 0), Some(SET[ALLIED as usize]));
    }
}
