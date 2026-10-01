//! Rain, snow and lightning: `docs/10-sky.md`, "The weather".
//!
//! Rain and snow are one particle system (`Terrain.dll:0x10072b40`) run two ways. Its
//! particles live in the world and fall there; the box they are kept in is the camera's,
//! from 2 to 50 ahead of it and as wide and high as its view is at 50, and a particle that
//! leaves the box comes back in at the far side. They are drawn as screen-space quads with
//! the depth test off, after the world: a snowflake a square or a diamond that shrinks with
//! its depth, a raindrop a quad stretched from where it was a frame ago to where it is.
//!
//! Lightning draws nothing of its own. It waits, picks a point anywhere on the map, and
//! starts its effect there.

use glam::{Vec2, Vec3};

use crate::effects::Rng;

/// A particle for each thousandth of the intensity (`0x10075b74`, `0x100764f4`): the pool
/// holds 1000 screen positions (`0x10072cad`), and no shipped keyframe asks for more than 1.
pub const POOL: usize = 1000;
/// The most quads one draw makes, `0x258` (`0x100748d2`, `0x10075401`).
pub const DRAWN_MOST: usize = 600;
/// The box's near and far faces, ahead of the camera (`0x10075a38`, `0x10075a3f`).
pub const NEAR: f32 = 2.0;
pub const FAR: f32 = 50.0;
/// The field of view, across and up, of the box the count is full in (`0x1009be14`,
/// `0x1009be18`): a narrower view's box holds its share of the particles by volume.
pub const REFERENCE_FIELD: [f32; 2] = [1.3, 0.975];
/// A particle's size against camera slot 27, the viewport's width over the field of view
/// (`0x1009be70`, `0x1009beb8`): at a field of 1.3 a snowflake is 1.5% of the screen across
/// at the near face, a raindrop 0.5%.
pub const SNOW_SIZE: f32 = 0.0195;
pub const RAIN_SIZE: f32 = 0.0065;
/// What a particle falls at, world units a second (`0x10075b92`, `0x1007654b`).
pub const SNOW_FALL: Vec3 = Vec3::new(0.5, 0.0, -4.0);
pub const RAIN_FALL: Vec3 = Vec3::new(0.5, 0.0, -60.0);
/// The least a snowflake shrinks to with depth (`0x1009be28`); a raindrop has no floor.
pub const SMALLEST: f32 = 0.1;
/// The colour both take: slot 19 with each channel held at `0x50` or above and an alpha of
/// `0x96` (`0x1006ce63`, `0x1006d152`).
pub const COLOUR_FLOOR: f32 = 80.0 / 255.0;
pub const ALPHA: f32 = 150.0 / 255.0;
/// A quad's four texture corners (`0x10073b80`).
pub const CORNERS: [[f32; 2]; 4] = [[0.0, 0.0], [0.0, 0.99], [0.99, 0.99], [0.99, 0.0]];

/// How long a bolt's object rests after a strike before it draws the next one's time
/// (`0x10071ce8`), and the longest that wait is, at no intensity at all (`0x1009bc68`).
pub const BOLT_REST_MS: f64 = 6000.0;
pub const BOLT_WAIT_MS: f64 = 60_000.0;
/// The intensity the wait is drawn at is held to this (`0x1009bc6c`).
pub const BOLT_INTENSITY_MOST: f32 = 0.95;
/// A bolt's effect is put half this far above the ground and sized 40 × 40 × this
/// (`0x1009bc64`, `0x10072058`).
pub const BOLT_HEIGHT: f32 = 600.0;
pub const BOLT_WIDTH: f32 = 40.0;

/// Which of the two the system runs as: the constructor's two flags (`0x1007563d`,
/// `0x10075ddd`), a flutter for snow and a streak for rain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fall {
    Snow,
    Rain,
}

impl Fall {
    pub fn velocity(self) -> Vec3 {
        match self {
            Fall::Snow => SNOW_FALL,
            Fall::Rain => RAIN_FALL,
        }
    }

    pub fn size(self) -> f32 {
        match self {
            Fall::Snow => SNOW_SIZE,
            Fall::Rain => RAIN_SIZE,
        }
    }
}

/// The colour a fall is drawn in, from slot 19 as the keyframes lerp it (0..1).
pub fn colour(slot_19: [f32; 3]) -> [f32; 4] {
    let [r, g, b] = slot_19.map(|c| c.max(COLOUR_FLOOR));
    [r, g, b, ALPHA]
}

/// The rain sound's volume at an intensity, decibels (`0x100765e5`): the intensity's
/// sixteenth root, four square roots, from DirectSound's −10000 hundredths at 0 to 0 at 1.
pub fn rain_volume(intensity: f32) -> f32 {
    100.0 * (intensity.max(0.0).sqrt().sqrt().sqrt().sqrt() - 1.0)
}

/// The C runtime's `rand` as `Terrain.dll` links it (`0x1008e98c`): fifteen bits a draw.
///
/// STAND-IN: docs/10-sky.md#the-weather--read-and-seen -- the game's draws come from the
/// module's one state, seeded 1 and shared with every other caller in `Terrain.dll`, so
/// where a particle spawns depends on every draw made before it; here the weather keeps a
/// state of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rand(pub u32);

impl Rand {
    pub fn next15(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(214_013).wrapping_add(2_531_011);
        (self.0 >> 16) & 0x7fff
    }

    /// 0 to 1, both ends reached: the draw over 32767 (`0x1009bde0`, `0x1009bc60`).
    pub fn unit(&mut self) -> f32 {
        self.next15() as f32 / 32_767.0
    }
}

/// The camera a fall is kept about and drawn through.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub eye: Vec3,
    pub forward: Vec3,
    pub right: Vec3,
    pub up: Vec3,
    /// The field of view across, radians.
    pub field: f32,
    /// The viewport, pixels.
    pub width: f32,
    pub height: f32,
}

impl View {
    pub fn new(eye: Vec3, forward: Vec3, up: Vec3, field: f32, (width, height): (f32, f32)) -> Self {
        let forward = forward.normalize_or(Vec3::Y);
        let right = forward.cross(up).normalize_or(Vec3::X);
        Self { eye, forward, right, up: right.cross(forward), field, width, height }
    }

    /// A world point in the camera's frame: depth, across to the right, up.
    pub fn local(&self, p: Vec3) -> Vec3 {
        let d = p - self.eye;
        Vec3::new(d.dot(self.forward), d.dot(self.right), d.dot(self.up))
    }

    pub fn world(&self, l: Vec3) -> Vec3 {
        self.eye + self.forward * l.x + self.right * l.y + self.up * l.z
    }

    /// Pixels a unit across at a depth of 1.
    pub fn focal(&self) -> f32 {
        self.width * 0.5 / (self.field * 0.5).tan()
    }

    /// Where a world point lands on the viewport, y down, and its depth.
    pub fn pixel(&self, p: Vec3) -> (Vec2, f32) {
        let l = self.local(p);
        let k = self.focal() / l.x;
        (Vec2::new(self.width * 0.5 + l.y * k, self.height * 0.5 - l.z * k), l.x)
    }

    /// The world point drawn `offset` pixels from where `anchor` is drawn, at its depth.
    pub fn nudged(&self, anchor: Vec3, offset: Vec2) -> Vec3 {
        let depth = self.local(anchor).x;
        anchor + (self.right * offset.x - self.up * offset.y) * (depth / self.focal())
    }

    /// The box the particles are kept in (`0x100759fc`–`0x10075ab4`): the view's half-width
    /// and half-height at [`FAR`]. The angle up is the field times the viewport's height over
    /// its width, taken as an angle and not through its tangent.
    pub fn reach(&self) -> Reach {
        let up = self.field * self.height / self.width.max(1.0);
        Reach { across: FAR * (self.field * 0.5).tan(), up: FAR * (up * 0.5).tan() }
    }
}

/// The particles' box in the camera's frame: [`NEAR`] to [`FAR`] deep, and this far to
/// either side and up and down at every depth.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reach {
    pub across: f32,
    pub up: f32,
}

impl Reach {
    pub fn low(&self) -> Vec3 {
        Vec3::new(NEAR, -self.across, -self.up)
    }

    pub fn extent(&self) -> Vec3 {
        Vec3::new(FAR - NEAR, 2.0 * self.across, 2.0 * self.up)
    }

    pub fn volume(&self) -> f32 {
        let e = self.extent();
        e.x * e.y * e.z
    }

    pub fn holds(&self, l: Vec3) -> bool {
        let (lo, hi) = (self.low(), self.low() + self.extent());
        (lo.x..=hi.x).contains(&l.x) && (lo.y..=hi.y).contains(&l.y) && (lo.z..=hi.z).contains(&l.z)
    }

    /// A point outside brought back in, a whole number of boxes along each axis
    /// (`0x10073410`).
    pub fn wrapped(&self, l: Vec3) -> Vec3 {
        let (lo, e) = (self.low(), self.extent());
        let p = l - lo;
        lo + p - (p / e).floor() * e
    }

    /// The box of the reference view (`0x100757c0`).
    pub fn reference() -> Reach {
        let [across, up] = REFERENCE_FIELD.map(|f| FAR * (f * 0.5).tan());
        Reach { across, up }
    }
}

/// How many particles a fall keeps (`0x10075b3a`–`0x10075b8f`): 1000 times the intensity,
/// times the box's volume over the reference box's where that is under 1.
pub fn count(intensity: f32, reach: &Reach) -> usize {
    let share = (reach.volume() / Reach::reference().volume()).min(1.0);
    ((POOL as f32 * intensity.max(0.0) * share).round() as usize).min(POOL)
}

/// One drawn particle: each corner a world point and how many pixels from it the corner is
/// drawn, y down, with the corner's place on the texture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Speck {
    pub anchors: [Vec3; 4],
    pub offsets: [Vec2; 4],
    pub uv: [[f32; 2]; 4],
}

/// A fall's particles.
#[derive(Clone, Debug)]
pub struct Fallen {
    /// Where each is, in the world.
    pub world: Vec<Vec3>,
    /// Where each was before the last advance, which a raindrop is stretched from.
    pub before: Vec<Vec3>,
    reach: Option<Reach>,
    last_ms: Option<f64>,
    rand: Rand,
    flutter: Rng,
}

impl Fallen {
    pub fn new(seed: u32) -> Self {
        Self {
            world: Vec::new(),
            before: Vec::new(),
            reach: None,
            last_ms: None,
            rand: Rand(seed),
            flutter: Rng::new(seed),
        }
    }

    fn spawn(&mut self, view: &View, reach: &Reach) -> Vec3 {
        // Three draws, in the box's own order: depth, across, up (`0x10073200`).
        let (lo, e) = (reach.low(), reach.extent());
        let depth = lo.x + self.rand.unit() * e.x;
        let across = lo.y + self.rand.unit() * e.y;
        let up = lo.z + self.rand.unit() * e.z;
        view.world(Vec3::new(depth, across, up))
    }

    /// One draw's update (`0x10073c10`, `0x100739b0`): a changed box respawns what there is;
    /// every particle moves by the fall's velocity over the time since the last draw, a
    /// snowflake by a flutter besides; one out of the box wraps back into it; and the count
    /// is brought to what the intensity asks, by spawning anywhere in the box or dropping the
    /// last.
    pub fn update(&mut self, fall: Fall, now_ms: f64, view: &View, intensity: f32) {
        let reach = view.reach();
        if self.reach != Some(reach) {
            for i in 0..self.world.len() {
                self.world[i] = self.spawn(view, &reach);
                self.before[i] = self.world[i];
            }
            self.reach = Some(reach);
        }
        let dt = ((now_ms - self.last_ms.unwrap_or(now_ms)) / 1000.0).max(0.0) as f32;
        self.last_ms = Some(now_ms);
        let step = fall.velocity() * dt;
        for i in 0..self.world.len() {
            self.before[i] = self.world[i];
            self.world[i] += step;
            if fall == Fall::Snow {
                // Three draws of the generator, and the last of them on all three axes
                // (`0x1007371a`–`0x100738bd`): each reads the state after the third step.
                self.flutter.next16();
                self.flutter.next16();
                let r = f32::from(self.flutter.next16()) / 65_536.0;
                self.world[i] += Vec3::splat(r * dt);
            }
        }
        for i in 0..self.world.len() {
            let l = view.local(self.world[i]);
            if !reach.holds(l) {
                self.world[i] = view.world(reach.wrapped(l));
                self.before[i] = self.world[i];
            }
        }
        let wanted = count(intensity, &reach);
        while self.world.len() < wanted {
            let p = self.spawn(view, &reach);
            self.world.push(p);
            self.before.push(p);
        }
        self.world.truncate(wanted);
        self.before.truncate(wanted);
    }

    /// What one draw makes of the particles (`0x1007414c`–`0x10075414`), at most
    /// [`DRAWN_MOST`]: those drawn inside the viewport less half a full-sized particle on
    /// every side.
    pub fn specks(&self, fall: Fall, view: &View) -> Vec<Speck> {
        let half = 0.5 * fall.size() * view.width / view.field;
        let inset = half.ceil();
        let inside = |p: Vec2| {
            (inset..=view.width - inset).contains(&p.x) && (inset..=view.height - inset).contains(&p.y)
        };
        let fade = |depth: f32| 1.0 - (depth - NEAR) / (FAR - NEAR);
        let mut out = Vec::new();
        for (i, (&at, &was)) in self.world.iter().zip(&self.before).enumerate() {
            if out.len() >= DRAWN_MOST {
                break;
            }
            let (pixel, depth) = view.pixel(at);
            if !inside(pixel) {
                continue;
            }
            match fall {
                Fall::Rain => {
                    if !inside(view.pixel(was).0) {
                        continue;
                    }
                    let h = half * fade(depth);
                    out.push(Speck {
                        anchors: [was, at, at, was],
                        offsets: [Vec2::new(-h, -h), Vec2::new(-h, h), Vec2::new(h, h), Vec2::new(h, -h)],
                        uv: CORNERS,
                    });
                }
                Fall::Snow => {
                    let s = half * fade(depth).max(SMALLEST);
                    // Its index turns it: the low two bits mirror it and turn its texture a
                    // quarter each, and bit 2 picks the square over the diamond.
                    let k = i & 3;
                    let (sx, sy) = (s * [1.0, 1.0, -1.0, -1.0][k], s * [-1.0, 1.0, -1.0, 1.0][k]);
                    let offsets = if i & 4 != 0 {
                        [Vec2::new(-sx, -sy), Vec2::new(-sx, sy), Vec2::new(sx, sy), Vec2::new(sx, -sy)]
                    } else {
                        let (sx, sy) = (sx * std::f32::consts::SQRT_2, sy * std::f32::consts::SQRT_2);
                        [Vec2::new(-sx, 0.0), Vec2::new(0.0, sy), Vec2::new(sx, 0.0), Vec2::new(0.0, -sy)]
                    };
                    out.push(Speck {
                        anchors: [at; 4],
                        offsets,
                        uv: std::array::from_fn(|j| CORNERS[(j + 4 - k) % 4]),
                    });
                }
            }
        }
        out
    }
}

/// The lightning object's three states (`0x10071cb0`).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Bolt {
    /// Resting since a strike at this time.
    Rest(f64),
    /// Ready to draw the next strike's time.
    Ready,
    /// Striking at this time.
    Due(f64),
}

/// When and where the lightning strikes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Storm {
    state: Bolt,
    rand: Rand,
}

/// One strike: where on the map, before the ground's height is known, and whether its
/// effect's frame is mirrored.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Strike {
    pub x: f32,
    pub y: f32,
    pub mirrored: bool,
}

impl Storm {
    pub fn new(seed: u32) -> Self {
        Self { state: Bolt::Ready, rand: Rand(seed) }
    }

    /// One update at `now_ms` (`0x10071cb0`): six seconds after a strike the object is ready;
    /// ready, and the intensity above 0, it draws the next strike's time, up to
    /// (1 − intensity) × 60 s on, the intensity held at 0.95 (`0x10071920`); at that time it
    /// strikes a point drawn anywhere over the map's box (`0x100720b0`), mirrored one time in
    /// two (`0x10071f8a`).
    pub fn update(&mut self, now_ms: f64, intensity: f32, lo: [f32; 2], hi: [f32; 2]) -> Option<Strike> {
        match self.state {
            Bolt::Rest(since) => {
                if now_ms - since >= BOLT_REST_MS {
                    self.state = Bolt::Ready;
                }
                None
            }
            Bolt::Ready => {
                if intensity > 0.0 {
                    let wait = f64::from(1.0 - intensity.min(BOLT_INTENSITY_MOST)) * BOLT_WAIT_MS;
                    self.state = Bolt::Due(now_ms + (f64::from(self.rand.unit()) * wait).round());
                }
                None
            }
            Bolt::Due(at) => {
                if now_ms < at {
                    return None;
                }
                let x = lo[0] + self.rand.unit() * (hi[0] - lo[0]);
                let y = lo[1] + self.rand.unit() * (hi[1] - lo[1]);
                let mirrored = self.rand.unit() < 0.5;
                self.state = Bolt::Rest(now_ms);
                Some(Strike { x, y, mirrored })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(field: f32, size: (f32, f32)) -> View {
        View::new(Vec3::new(100.0, 200.0, 30.0), Vec3::Y, Vec3::Z, field, size)
    }

    #[test]
    fn the_box_is_the_view_at_50_from_2_ahead() {
        let v = view(1.3, (800.0, 600.0));
        let r = v.reach();
        // tan(0.65) × 50 across, and tan(0.4875) × 50 up: the angle up is 1.3 × 600 ÷ 800.
        assert!((r.across - 38.010).abs() < 1e-2, "{r:?}");
        assert!((r.up - 26.512).abs() < 1e-2, "{r:?}");
        assert_eq!((r.low().x, r.extent().x), (2.0, 48.0));
        assert_eq!(r, Reach::reference(), "the reference box is a 4:3 view 1.3 across");
    }

    #[test]
    fn a_full_intensity_keeps_a_thousand_and_a_narrow_view_its_share() {
        let full = view(1.3, (800.0, 600.0)).reach();
        assert_eq!(count(1.0, &full), 1000);
        assert_eq!(count(0.7, &full), 700);
        assert_eq!(count(0.0, &full), 0);
        // A wider screen's box is lower: 1.3 × 9 ÷ 16 up.
        let wide = view(1.3, (1600.0, 900.0)).reach();
        let share = wide.volume() / full.volume();
        assert!((share - 0.722).abs() < 2e-3, "{share}");
        assert_eq!(count(1.0, &wide), (1000.0 * share).round() as usize);
        // A wider field's box is larger, and the count is held at the pool's 1000.
        assert_eq!(count(1.0, &view(1.6, (800.0, 600.0)).reach()), 1000);
        // Zoomed to 0.2 the box is a sliver.
        assert_eq!(count(1.0, &view(0.2, (800.0, 600.0)).reach()), 19);
    }

    #[test]
    fn particles_spawn_through_the_box_and_stay_in_it() {
        let v = view(1.3, (800.0, 600.0));
        let mut f = Fallen::new(1);
        f.update(Fall::Snow, 0.0, &v, 1.0);
        assert_eq!(f.world.len(), 1000);
        let r = v.reach();
        let depths: Vec<f32> = f.world.iter().map(|&p| v.local(p).x).collect();
        assert!(f.world.iter().all(|&p| r.holds(v.local(p))));
        let mean = depths.iter().sum::<f32>() / 1000.0;
        assert!((mean - 26.0).abs() < 1.5, "spread evenly from 2 to 50: {mean}");
        // Ten seconds of snow at 60 frames a second: every one is still in the box.
        for frame in 1..=600 {
            f.update(Fall::Snow, f64::from(frame) * 1000.0 / 60.0, &v, 1.0);
        }
        assert_eq!(f.world.len(), 1000);
        assert!(f.world.iter().all(|&p| r.holds(v.local(p))));
    }

    #[test]
    fn snow_falls_at_4_and_drifts_and_rain_at_60() {
        let v = view(1.3, (800.0, 600.0));
        for (fall, low, high) in [
            // Snow's flutter adds up to 1 a second on every axis, never less than 0.
            (Fall::Snow, Vec3::new(0.5, 0.0, -4.0), Vec3::new(1.5, 1.0, -3.0)),
            (Fall::Rain, RAIN_FALL, RAIN_FALL),
        ] {
            let mut f = Fallen::new(7);
            f.update(fall, 0.0, &v, 1.0);
            let before = f.world.clone();
            f.update(fall, 100.0, &v, 1.0);
            let mut moved = 0;
            for (a, b) in before.iter().zip(&f.world) {
                // One that wrapped has jumped a box; the rest moved by the velocity.
                let step = (*b - *a) / 0.1;
                if step.length() > 70.0 {
                    continue;
                }
                moved += 1;
                assert!(step.cmpge(low - 1e-2).all() && step.cmple(high + 1e-2).all(), "{fall:?} {step}");
            }
            assert!(moved > 800, "{fall:?}: {moved} did not wrap");
        }
    }

    #[test]
    fn the_particles_stay_in_the_world_as_the_camera_turns() {
        let v = view(1.3, (800.0, 600.0));
        let mut f = Fallen::new(3);
        f.update(Fall::Snow, 0.0, &v, 1.0);
        let before = f.world.clone();
        // The same instant through a camera turned a little: nothing has fallen, and what is
        // still in the turned box has not moved.
        let turned = View::new(v.eye, Vec3::new(0.1, 1.0, 0.0), Vec3::Z, 1.3, (800.0, 600.0));
        f.update(Fall::Snow, 0.0, &turned, 1.0);
        let kept = before.iter().zip(&f.world).filter(|(a, b)| a == b).count();
        assert!(kept > 850, "{kept}");
        assert!(f.world.iter().all(|&p| turned.reach().holds(turned.local(p))));
    }

    #[test]
    fn a_wrap_carries_a_particle_a_whole_box() {
        let r = view(1.3, (800.0, 600.0)).reach();
        let below = Vec3::new(10.0, 5.0, -r.up - 1.0);
        let w = r.wrapped(below);
        assert!((w - Vec3::new(10.0, 5.0, r.up - 1.0)).length() < 1e-4, "{w}");
        let past = Vec3::new(51.0, 0.0, 0.0);
        assert!((r.wrapped(past).x - 3.0).abs() < 1e-4);
    }

    #[test]
    fn a_snowflake_shrinks_with_depth_to_a_tenth() {
        let v = view(1.3, (800.0, 600.0));
        let mut f = Fallen::new(1);
        f.world = vec![v.world(Vec3::new(2.0, 0.0, 0.0)); 5];
        f.world.extend([v.world(Vec3::new(26.0, 0.0, 0.0)), v.world(Vec3::new(50.0, 0.0, 0.0))]);
        f.before = f.world.clone();
        let specks = f.specks(Fall::Snow, &v);
        assert_eq!(specks.len(), 7);
        // Half of 0.0195 × 800 ÷ 1.3 = 6 pixels at the near face.
        let reach = |s: &Speck| s.offsets.iter().map(|o| o.x.abs().max(o.y.abs())).fold(0.0, f32::max);
        // Index 4 is a square, 6 pixels to each side; index 0 a diamond, √2 times that to
        // each point.
        assert!((reach(&specks[4]) - 6.0).abs() < 1e-3);
        assert!((reach(&specks[0]) - 6.0 * std::f32::consts::SQRT_2).abs() < 1e-3);
        assert_eq!(specks[0].offsets.iter().filter(|o| o.x == 0.0 || o.y == 0.0).count(), 4);
        // Index 5 at 26 deep is half the size; index 6 at 50 is held at a tenth.
        assert!((reach(&specks[5]) - 3.0).abs() < 1e-3);
        assert!((reach(&specks[6]) - 0.6).abs() < 1e-3);
        // Its texture turns a quarter with each of the index's low two bits.
        assert_eq!(specks[4].uv, CORNERS);
        assert_eq!(specks[5].uv[1], CORNERS[0]);
        assert_eq!(specks[6].uv[2], CORNERS[0]);
    }

    #[test]
    fn a_raindrop_is_stretched_from_where_it_was() {
        let v = view(1.3, (800.0, 600.0));
        let mut f = Fallen::new(1);
        let at = v.world(Vec3::new(26.0, 1.0, 2.0));
        f.world = vec![at];
        f.before = vec![at - RAIN_FALL / 60.0];
        let s = f.specks(Fall::Rain, &v)[0];
        assert_eq!(s.anchors, [f.before[0], at, at, f.before[0]]);
        // Half of 0.0065 × 800 ÷ 1.3 = 2 pixels, halved again at 26 deep.
        assert!((s.offsets[2] - Vec2::new(1.0, 1.0)).length() < 1e-4);
        // A sixtieth of a second's fall at 26 deep is 20 pixels down the screen.
        let (from, to) = (v.pixel(s.anchors[0]).0, v.pixel(s.anchors[1]).0);
        assert!((to.y - from.y - 20.2).abs() < 0.3, "{from} {to}");
        // One that has just wrapped has no length.
        f.before = vec![at];
        let s = f.specks(Fall::Rain, &v)[0];
        assert_eq!(s.anchors[0], s.anchors[1]);
    }

    #[test]
    fn no_more_than_600_are_drawn_and_none_at_the_edge() {
        let v = view(1.3, (800.0, 600.0));
        let mut f = Fallen::new(1);
        f.world = vec![v.world(Vec3::new(20.0, 0.0, 0.0)); 1000];
        f.before = f.world.clone();
        assert_eq!(f.specks(Fall::Snow, &v).len(), 600);
        // One drawn within half a full flake, 6 pixels and a whole pixel up, of the viewport's
        // edge is left out.
        let k = v.focal() / 20.0;
        f.world = vec![v.world(Vec3::new(20.0, 397.0 / k, 0.0)), v.world(Vec3::new(20.0, 390.0 / k, 0.0))];
        f.before = f.world.clone();
        assert_eq!(f.specks(Fall::Snow, &v).len(), 1);
    }

    #[test]
    fn a_nudged_corner_is_drawn_that_many_pixels_away() {
        let v = view(1.1, (1400.0, 1050.0));
        let at = v.world(Vec3::new(30.0, -4.0, 7.0));
        let (p, _) = v.pixel(at);
        let (q, depth) = v.pixel(v.nudged(at, Vec2::new(5.0, -3.0)));
        assert!((q - p - Vec2::new(5.0, -3.0)).length() < 1e-2, "{p} {q}");
        assert!((depth - 30.0).abs() < 1e-3);
    }

    #[test]
    fn the_colour_is_slot_19_held_at_80_with_an_alpha_of_150() {
        assert_eq!(colour([0.0, 0.0, 0.0]), [80.0 / 255.0, 80.0 / 255.0, 80.0 / 255.0, 150.0 / 255.0]);
        let [r, g, b, a] = colour([1.0, 0.0, 32.0 / 255.0]);
        assert_eq!((r, g, b, a), (1.0, COLOUR_FLOOR, COLOUR_FLOOR, ALPHA));
    }

    #[test]
    fn the_rain_is_loudest_at_full_intensity() {
        assert_eq!(rain_volume(1.0), 0.0);
        assert_eq!(rain_volume(0.0), -100.0);
        assert!((rain_volume(0.5) - -4.24).abs() < 0.01);
    }

    #[test]
    fn the_crt_generator_draws_its_published_sequence() {
        // Seeded 1, the C runtime's first draws.
        let mut r = Rand(1);
        assert_eq!([r.next15(), r.next15(), r.next15()], [41, 18_467, 6_334]);
    }

    #[test]
    fn a_bolt_strikes_six_to_nine_seconds_apart_at_full_intensity() {
        let mut storm = Storm::new(1);
        let (lo, hi) = ([0.0, 0.0], [2000.0, 3000.0]);
        let mut strikes = Vec::new();
        let mut t = 0.0;
        while t < 600_000.0 {
            if let Some(s) = storm.update(t, 1.0, lo, hi) {
                assert!((0.0..=2000.0).contains(&s.x) && (0.0..=3000.0).contains(&s.y));
                strikes.push((t, s));
            }
            t += 1000.0 / 60.0;
        }
        let gaps: Vec<f64> = strikes.windows(2).map(|w| w[1].0 - w[0].0).collect();
        // Six seconds' rest, then up to (1 − 0.95) × 60 s, and a frame or two between states.
        assert!(gaps.iter().all(|&g| (6000.0..9100.0).contains(&g)), "{gaps:?}");
        assert!(gaps.iter().any(|&g| g > 8000.0) && gaps.iter().any(|&g| g < 7000.0), "{gaps:?}");
        // Over the whole map, mirrored about one time in two.
        assert!(strikes.iter().any(|(_, s)| s.x < 500.0) && strikes.iter().any(|(_, s)| s.x > 1500.0));
        let mirrored = strikes.iter().filter(|(_, s)| s.mirrored).count();
        assert!(mirrored * 4 > strikes.len() && mirrored * 4 < strikes.len() * 3, "{mirrored}");
    }

    #[test]
    fn a_weaker_storm_waits_longer_and_none_at_no_intensity() {
        let gaps = |intensity: f32| {
            let mut storm = Storm::new(5);
            let mut last = None;
            let mut gaps = Vec::new();
            let mut t = 0.0;
            while t < 3_600_000.0 {
                if storm.update(t, intensity, [0.0; 2], [1.0; 2]).is_some() {
                    if let Some(l) = last {
                        gaps.push(t - l);
                    }
                    last = Some(t);
                }
                t += 50.0;
            }
            gaps
        };
        let half = gaps(0.5);
        assert!(half.iter().all(|&g| (6000.0..36_200.0).contains(&g)), "{half:?}");
        assert!(half.iter().any(|&g| g > 25_000.0));
        assert!(gaps(0.0).is_empty());
        let mut calm = Storm::new(5);
        assert!((0..1000).all(|i| calm.update(f64::from(i) * 100.0, 0.0, [0.0; 2], [1.0; 2]).is_none()));
    }
}
