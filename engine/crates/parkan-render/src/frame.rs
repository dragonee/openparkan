//! What every lit pipeline reads once a frame: the camera and the light.

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

/// The sun the world is lit by until M5 reads the sky.
///
/// STAND-IN: docs/10-sky.md#not-resolved -- what the sun does with its values
/// is not read, and no sky keyframe is interpolated yet.
pub const LIGHT_DIRECTION: Vec3 = Vec3::new(-0.35, -0.45, -0.82);
pub const LIGHT_COLOUR: [f32; 3] = [0.85, 0.85, 0.8];
/// STAND-IN: docs/10-sky.md -- Mission 01's noon scene colour, 40/255 grey,
/// held fixed until M5 interpolates the sky.
pub const SCENE_COLOUR: [f32; 3] = [0.16, 0.16, 0.16];

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct FrameUniform {
    pub view_proj: [f32; 16],
    pub light_direction: [f32; 4],
    pub light_colour: [f32; 4],
    pub scene_colour: [f32; 4],
}

impl FrameUniform {
    pub fn new(view_proj: Mat4) -> Self {
        let d = LIGHT_DIRECTION.normalize();
        let [r, g, b] = LIGHT_COLOUR;
        let [sr, sg, sb] = SCENE_COLOUR;
        Self {
            view_proj: view_proj.to_cols_array(),
            light_direction: [d.x, d.y, d.z, 0.0],
            light_colour: [r, g, b, 1.0],
            scene_colour: [sr, sg, sb, 1.0],
        }
    }
}
