//! What every lit pipeline reads once a frame: the camera, the light and the fog.

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

/// The light and fog a frame is drawn with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lighting {
    /// The direction light travels.
    pub light_direction: Vec3,
    pub light_colour: [f32; 3],
    /// Added to every material's emissive (sky slot 20).
    pub scene_colour: [f32; 3],
    pub fog_colour: [f32; 3],
    /// Linear range fog from the eye: none at `fog_start`, whole at `fog_end`.
    pub fog_start: f32,
    pub fog_end: f32,
    pub eye: Vec3,
}

impl Default for Lighting {
    /// A fixed sun and no fog, for a world with no sky.
    ///
    /// STAND-IN: docs/10-sky.md#not-resolved -- what the sun does with its values is
    /// not read; this is used only where there is no atmosphere.
    fn default() -> Self {
        Self {
            light_direction: Vec3::new(-0.35, -0.45, -0.82),
            light_colour: [0.85, 0.85, 0.8],
            scene_colour: [0.16, 0.16, 0.16],
            fog_colour: [0.05, 0.06, 0.08],
            fog_start: 0.0,
            fog_end: f32::MAX,
            eye: Vec3::ZERO,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct FrameUniform {
    pub view_proj: [f32; 16],
    pub light_direction: [f32; 4],
    pub light_colour: [f32; 4],
    pub scene_colour: [f32; 4],
    pub fog_colour: [f32; 4],
    /// Start, end.
    pub fog: [f32; 4],
    pub eye: [f32; 4],
}

impl FrameUniform {
    pub fn new(view_proj: Mat4, l: &Lighting) -> Self {
        let d = l.light_direction.normalize_or(Vec3::NEG_Z);
        let rgb = |c: [f32; 3]| [c[0], c[1], c[2], 1.0];
        Self {
            view_proj: view_proj.to_cols_array(),
            light_direction: [d.x, d.y, d.z, 0.0],
            light_colour: rgb(l.light_colour),
            scene_colour: rgb(l.scene_colour),
            fog_colour: rgb(l.fog_colour),
            fog: [l.fog_start, l.fog_end, 0.0, 0.0],
            eye: [l.eye.x, l.eye.y, l.eye.z, 1.0],
        }
    }
}

/// A colour the files give in display space, as the linear value a shader writing to
/// an sRGB target needs (the game blends in display space; our textures decode to linear).
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
