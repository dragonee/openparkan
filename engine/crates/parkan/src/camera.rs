//! The cameras: the hero's first-person eye, and a debug camera that flies freely.

use glam::{Mat4, Vec3};
use parkan_world::hero::Eye;

/// Degrees of the vertical field of view for the debug camera. The game's
/// own first-person view is set by the turret's camera in M3.
pub const DEBUG_FOV_Y_DEGREES: f32 = 60.0;
pub const NEAR: f32 = 0.1;

pub struct FlyCamera {
    pub position: Vec3,
    /// Radians about z: 0 looks along +y, and it turns as an object's heading does.
    pub yaw: f32,
    /// Radians up from level.
    pub pitch: f32,
}

impl FlyCamera {
    /// Behind and above `target`, looking where `heading` faces.
    pub fn behind(target: Vec3, heading: f32) -> Self {
        let mut camera = Self { position: target, yaw: heading, pitch: -0.3 };
        camera.position = target - camera.forward() * 40.0 + Vec3::Z * 4.0;
        camera
    }

    pub fn forward(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        Vec3::new(-sy * cp, cy * cp, sp)
    }

    pub fn right(&self) -> Vec3 {
        self.forward().cross(Vec3::Z).normalize_or_zero()
    }

    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        let view = Mat4::look_to_rh(self.position, self.forward(), Vec3::Z);
        let proj = Mat4::perspective_infinite_reverse_rh(DEBUG_FOV_Y_DEGREES.to_radians(), aspect, NEAR);
        proj * view
    }

    pub fn turn(&mut self, dx: f32, dy: f32) {
        self.yaw -= dx;
        self.pitch = (self.pitch - dy).clamp(-1.55, 1.55);
    }
}

/// The debug camera's field of view across, radians, at `aspect`.
pub fn debug_field(aspect: f32) -> f32 {
    2.0 * ((DEBUG_FOV_Y_DEGREES.to_radians() / 2.0).tan() * aspect).atan()
}

/// The view through the hero's eye. Its field of view is horizontal: the camera
/// keeps tan of half its angle and scales y by it × height ÷ width (docs/30).
pub fn first_person(eye: &Eye, aspect: f32) -> Mat4 {
    let fov_y = 2.0 * ((eye.fov_x / 2.0).tan() / aspect).atan();
    let view = Mat4::look_to_rh(eye.position, eye.forward, eye.up);
    Mat4::perspective_infinite_reverse_rh(fov_y, aspect, eye.near) * view
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_eyes_field_of_view_is_across_the_screen() {
        let eye = Eye { position: Vec3::ZERO, forward: Vec3::Y, up: Vec3::Z, fov_x: 1.3, near: 0.1 };
        // A point at the edge of a 1.3-rad horizontal view lands on the screen's edge.
        let edge = Vec3::new(0.65_f32.tan(), 1.0, 0.0);
        for aspect in [4.0 / 3.0, 16.0 / 9.0] {
            let p = first_person(&eye, aspect).project_point3(edge);
            assert!((p.x - 1.0).abs() < 1e-4, "{p}");
        }
    }

    #[test]
    fn heading_zero_looks_north_and_right_is_east() {
        let c = FlyCamera { position: Vec3::ZERO, yaw: 0.0, pitch: 0.0 };
        assert!((c.forward() - Vec3::Y).length() < 1e-6);
        assert!((c.right() - Vec3::X).length() < 1e-6);
    }

    #[test]
    fn mission_01s_hero_heading_faces_the_dummies() {
        // Rotation -1.639 turns the hero's +y to (0.998, -0.068), towards the targets.
        let c = FlyCamera { position: Vec3::ZERO, yaw: -1.638_556_4, pitch: 0.0 };
        let f = c.forward();
        assert!((f.x - 0.998).abs() < 1e-3 && (f.y + 0.068).abs() < 1e-3, "{f}");
    }

    #[test]
    fn a_point_ahead_projects_to_the_centre_with_reverse_depth() {
        let c = FlyCamera { position: Vec3::ZERO, yaw: 0.0, pitch: 0.0 };
        let near = c.view_proj(1.0).project_point3(Vec3::new(0.0, 1.0, 0.0));
        let far = c.view_proj(1.0).project_point3(Vec3::new(0.0, 100.0, 0.0));
        assert!(near.x.abs() < 1e-5 && near.y.abs() < 1e-5);
        assert!(near.z > far.z && far.z > 0.0);
    }
}
