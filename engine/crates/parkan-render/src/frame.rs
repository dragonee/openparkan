//! What every lit pipeline reads once a frame: the camera, the lights and the fog.
//!
//! The lit colour is formed as fixed-function lighting forms it, from the files' own
//! display-space colours, and held to 1; the shaders decode it once, and decode the
//! fog, the dome and the textures, to the linear values an sRGB target blends.

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

/// The scene colour before any sky has sent one: the shader component's state block, one for
/// the whole game (`CID_SHADER` is `terrain.dll CreateShader`, a singleton), is built with
/// (0.2, 0.2, 0.2) (`Terrain.dll:0x1004bbcd`), and only the sky's mask-`0x10` message changes
/// it (docs/10-sky.md, "The scene colour is added to every material").
pub const SHADER_SCENE_COLOUR: [f32; 3] = [0.2; 3];

/// A directional light.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Light {
    /// The direction light travels.
    pub direction: Vec3,
    /// In the files' display space; it may pass 1, as a lifted sun does.
    pub colour: [f32; 3],
}

impl Light {
    pub const OFF: Light = Light { direction: Vec3::NEG_Z, colour: [0.0; 3] };
}

/// The most point lights a frame carries.
///
/// STAND-IN: docs/11-effects.md#what-a-light-does-to-a-surface--read-and-measured -- the game
/// gathers a list for each draw item, every light whose reach meets the object's sphere or the
/// landscape cell's box (`Terrain.dll:0x100479c0`, `0x10047bb0`), and its lighter walks all of
/// it. Here a frame carries one list, the 64 lights whose reach comes nearest the eye, and each
/// lights whatever stands inside its range.
pub const MAX_POINT_LIGHTS: usize = 64;

/// A point light as the shade's lighter takes it (docs/11, "What a light does to a surface"):
/// its colour times the cosine at a vertex times a₀ + a₁ x + a₂ x², x the share of its range
/// left, and nothing past its range.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointLight {
    pub position: Vec3,
    pub range: f32,
    /// In the files' display space; an effect's light starts as high as 13.
    pub colour: [f32; 3],
    /// Constant, linear and quadratic, on the share of the range left.
    pub attenuation: [f32; 3],
    /// The instances it lights, by the owner they carry, or 0 for every surface in its range:
    /// an owner-only light (`0x80000000`) reaches its owner alone (`Terrain.dll:0x10047a52`).
    pub owner: u32,
}

impl PointLight {
    const OFF: PointLight = PointLight {
        position: Vec3::ZERO,
        range: 0.0,
        colour: [0.0; 3],
        attenuation: [1.0, 0.0, 0.0],
        owner: 0,
    };
}

/// A frame's point lights: those that light everything first, then the owners' own.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Points {
    lights: [PointLight; MAX_POINT_LIGHTS],
    shared: usize,
    count: usize,
}

impl Default for Points {
    fn default() -> Self {
        Self { lights: [PointLight::OFF; MAX_POINT_LIGHTS], shared: 0, count: 0 }
    }
}

impl Points {
    /// The lights of `all` a frame seen from `eye` carries, at most [`MAX_POINT_LIGHTS`]: those
    /// that light everything first, then the owners' own, each kind by how near its reach
    /// comes to the eye.
    pub fn nearest(all: &[PointLight], eye: Vec3) -> Self {
        let mut kept: Vec<&PointLight> = all.iter().filter(|l| l.range > 0.0).collect();
        let reach = |l: &PointLight| (l.position.distance(eye) - l.range).max(0.0);
        kept.sort_by(|a, b| (a.owner != 0).cmp(&(b.owner != 0)).then(reach(a).total_cmp(&reach(b))));
        kept.truncate(MAX_POINT_LIGHTS);
        let mut lights = [PointLight::OFF; MAX_POINT_LIGHTS];
        for (slot, light) in lights.iter_mut().zip(&kept) {
            *slot = **light;
        }
        Self { lights, shared: kept.iter().filter(|l| l.owner == 0).count(), count: kept.len() }
    }

    pub fn as_slice(&self) -> &[PointLight] {
        &self.lights[..self.count]
    }
}

/// The lights and fog a frame is drawn with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lighting {
    /// The sun object's two directional lights (`docs/10-sky.md`, "What the sun does
    /// with its seven values").
    pub lights: [Light; 2],
    /// Added to every material's emissive (sky slot 20), in display space.
    pub scene_colour: [f32; 3],
    /// Linear, as the target blends it.
    pub fog_colour: [f32; 3],
    /// Linear range fog from the eye: none at `fog_start`, whole at `fog_end`.
    pub fog_start: f32,
    pub fog_end: f32,
    pub eye: Vec3,
    /// The eye's field of view across, radians, which a portal quad's fade is scaled by
    /// (docs/24, "A building is drawn cell by cell through its portals").
    pub field: f32,
    /// The world clock, ms, that material tracks play on.
    pub clock_ms: f64,
    /// The effects' point lights, which the device lights every lit surface with beside the
    /// sun's two (docs/11, "What a light does to a surface").
    pub points: Points,
}

impl Default for Lighting {
    /// A fixed light and no fog, for a world drawn without an atmosphere. Every mission
    /// directory has a `sky.ske` (`docs/10-sky.md`), so this lights a scene only when
    /// the file fails to load.
    fn default() -> Self {
        Self {
            lights: [
                Light { direction: Vec3::new(-0.35, -0.45, -0.82), colour: [0.85, 0.85, 0.8] },
                Light::OFF,
            ],
            scene_colour: SHADER_SCENE_COLOUR,
            fog_colour: [0.05, 0.06, 0.08],
            fog_start: 0.0,
            fog_end: f32::MAX,
            eye: Vec3::ZERO,
            field: parkan_world::models::PORTAL_FIELD,
            clock_ms: 0.0,
            points: Points::default(),
        }
    }
}

/// A point light in the layout the shaders read: the position and the range, the colour and the
/// owner, the three attenuation terms.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct PointUniform {
    pub position: [f32; 4],
    pub colour: [f32; 4],
    pub attenuation: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct FrameUniform {
    pub view_proj: [f32; 16],
    pub light_direction: [f32; 4],
    pub light_colour: [f32; 4],
    pub second_direction: [f32; 4],
    pub second_colour: [f32; 4],
    pub scene_colour: [f32; 4],
    pub fog_colour: [f32; 4],
    /// Start, end; then the height nothing below draws at, where w is 1 (a reflection's clip
    /// plane).
    pub fog: [f32; 4],
    /// The eye, and in w its field of view across.
    pub eye: [f32; 4],
    /// x 1 where the instances draw in their paint, as a HUD panel's view does.
    pub paint: [f32; 4],
    /// x how many of the point lights light every surface, which come first; y how many
    /// there are.
    pub point_counts: [f32; 4],
    pub points: [PointUniform; MAX_POINT_LIGHTS],
}

impl FrameUniform {
    pub fn new(view_proj: Mat4, l: &Lighting) -> Self {
        let direction = |light: &Light| {
            let d = light.direction.normalize_or(Vec3::NEG_Z);
            [d.x, d.y, d.z, 0.0]
        };
        let rgb = |c: [f32; 3]| [c[0], c[1], c[2], 1.0];
        let [a, b] = &l.lights;
        Self {
            view_proj: view_proj.to_cols_array(),
            light_direction: direction(a),
            light_colour: rgb(a.colour),
            second_direction: direction(b),
            second_colour: rgb(b.colour),
            scene_colour: rgb(l.scene_colour),
            fog_colour: rgb(l.fog_colour),
            fog: [l.fog_start, l.fog_end, 0.0, 0.0],
            eye: [l.eye.x, l.eye.y, l.eye.z, l.field],
            paint: [0.0; 4],
            point_counts: [l.points.shared as f32, l.points.count as f32, 0.0, 0.0],
            points: l.points.lights.map(|p| PointUniform {
                position: [p.position.x, p.position.y, p.position.z, p.range],
                colour: [p.colour[0], p.colour[1], p.colour[2], p.owner as f32],
                attenuation: [p.attenuation[0], p.attenuation[1], p.attenuation[2], 0.0],
            }),
        }
    }

    /// The same frame, drawing nothing below `height`.
    pub fn clipped_below(mut self, height: f32) -> Self {
        self.fog[2] = height;
        self.fog[3] = 1.0;
        self
    }
}

/// A colour the files give in display space, as the linear value a shader writing to
/// an sRGB target needs; the shaders' `linear` is the same curve.
///
/// STAND-IN: docs/10-sky.md#the-dome-the-fog-and-the-scene-colour--read-and-measured --
/// how the files' colours meet textures decoded to linear is not read (the game blends
/// in display space); the sky's, the fog's, the texture tints and the lit colour are
/// decoded from sRGB, so blends come out as the game's display-space ones.
pub fn linear(c: [f32; 3]) -> [f32; 3] {
    c.map(|v| if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) })
}

/// The fog colour a blend mode draws toward (`Terrain.dll:0x1002ffea`): additive to black,
/// mode 3 to white, mode 5 to grey `0x7f7f7f` (as linear); `w` 1 where it overrides the scene's.
pub fn fog_override(mode: u8) -> [f32; 4] {
    match mode {
        2 => [0.0, 0.0, 0.0, 1.0],
        3 => [1.0, 1.0, 1.0, 1.0],
        5 => [0.212, 0.212, 0.212, 1.0],
        _ => [0.0; 4],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_frame_carries_both_lights_in_the_layout_the_shaders_read() {
        let mut l = Lighting::default();
        l.lights[1] = Light { direction: Vec3::new(0.0, 0.0, -2.0), colour: [0.3, 0.3, 0.4] };
        let u = FrameUniform::new(Mat4::IDENTITY, &l);
        // A mat4, ten vec4s and three more a point light, no padding: what `Frame` in
        // model.wgsl and terrain.wgsl is.
        assert_eq!(std::mem::size_of::<FrameUniform>(), 64 + 10 * 16 + MAX_POINT_LIGHTS * 48);
        assert_eq!(u.point_counts, [0.0; 4], "no point light until a scene hands one over");
        assert_eq!(u.paint, [0.0; 4], "the scene does not paint");
        assert_eq!(u.second_direction, [0.0, 0.0, -1.0, 0.0], "normalised");
        assert_eq!(u.second_colour, [0.3, 0.3, 0.4, 1.0]);
        assert_eq!(u.light_colour, [0.85, 0.85, 0.8, 1.0], "display space, as given");
    }

    #[test]
    fn with_no_sky_the_scene_colour_is_the_shaders_own_grey() {
        // `Terrain.dll:0x1004bbcd` stores 0x3e4ccccd three times into the block's colour.
        assert_eq!(f32::from_bits(0x3e4c_cccd), 0.2);
        assert_eq!(Lighting::default().scene_colour, [0.2; 3]);
    }

    #[test]
    fn a_lifted_light_keeps_its_colour_past_1() {
        let l = Lighting {
            lights: [Light { direction: Vec3::NEG_Z, colour: [2.5, 1.0, 0.0] }, Light::OFF],
            ..Lighting::default()
        };
        assert_eq!(FrameUniform::new(Mat4::IDENTITY, &l).light_colour, [2.5, 1.0, 0.0, 1.0]);
    }

    /// The frame carries the lights whose reach comes nearest the eye, those that light
    /// everything ahead of the owners' own, each with its range, owner and attenuation where
    /// the shaders read them.
    #[test]
    fn the_frame_carries_the_nearest_point_lights_the_shared_ones_first() {
        let light = |x: f32, owner: u32| PointLight {
            position: Vec3::new(x, 0.0, 0.0),
            range: 7.0,
            colour: [0.0, 0.0, 0.9],
            attenuation: [1.0, 1.0, 0.0],
            owner,
        };
        let dark = PointLight { range: 0.0, ..light(1.0, 0) };
        let all = [light(50.0, 3), light(30.0, 0), dark, light(10.0, 5), light(900.0, 0)];
        let points = Points::nearest(&all, Vec3::ZERO);
        let order: Vec<(f32, u32)> = points.as_slice().iter().map(|l| (l.position.x, l.owner)).collect();
        assert_eq!(order, [(30.0, 0), (900.0, 0), (10.0, 5), (50.0, 3)], "a light of no range is left out");
        let u = FrameUniform::new(Mat4::IDENTITY, &Lighting { points, ..Lighting::default() });
        assert_eq!(u.point_counts, [2.0, 4.0, 0.0, 0.0]);
        assert_eq!(u.points[2].position, [10.0, 0.0, 0.0, 7.0]);
        assert_eq!(u.points[2].colour, [0.0, 0.0, 0.9, 5.0]);
        assert_eq!(u.points[2].attenuation, [1.0, 1.0, 0.0, 0.0]);

        let many: Vec<PointLight> = (0..100).map(|i| light(i as f32 * 20.0, 0)).collect();
        let kept = Points::nearest(&many, Vec3::ZERO);
        assert_eq!(kept.as_slice().len(), MAX_POINT_LIGHTS);
        assert!(kept.as_slice().iter().all(|l| l.position.x < 64.0 * 20.0), "the nearest are kept");
        // A lamp's own light never crowds out one that lights everything.
        let mut crowded: Vec<PointLight> = (0..100).map(|i| light(i as f32, 7)).collect();
        crowded.push(light(5000.0, 0));
        let kept = Points::nearest(&crowded, Vec3::ZERO);
        assert_eq!((kept.shared, kept.as_slice()[0].position.x), (1, 5000.0));
    }

    #[test]
    fn linear_is_the_srgb_decode() {
        let [black, mid, white] = linear([0.0, 0.5, 1.0]);
        assert_eq!(black, 0.0);
        assert!((mid - 0.214).abs() < 1e-3 && (white - 1.0).abs() < 1e-6);
    }
}
