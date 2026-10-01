//! The game's own shade, one vertex at a time: `docs/10-sky.md`, "The lit colour is the
//! game's own".
//!
//! `UseDXLighting` ships 0 (`Terrain.dll:0x1005fcce`, read at `0x1002f8fd` and `0x1002fe50`),
//! so no device material is ever built and no device light or fog range set: every draw item
//! that asks to be lit goes through `CShade::ShadeIndexedStrided` (`0x1004df70`, the shader
//! component's slot 8) instead, which writes a diffuse and a specular `D3DCOLOR` on each
//! vertex. The device modulates the texture stages by the first, adds the second
//! (`SPECULARENABLE` is 1 from `Ngi32.dll`'s reset, `0x10006c38`), and takes its fog factor
//! from the second's alpha.
//!
//! [`SHADE_WGSL`] is the same arithmetic for the pipelines' vertex stages; this is its tested
//! twin, and what lights the clouds, whose vertices are few enough to shade here.

use glam::Vec3;

/// The shared functions `terrain.wgsl` and `model.wgsl` are prefixed with.
pub const SHADE_WGSL: &str = include_str!("shade.wgsl");

/// Where a lit component is kneed, the slope and the offset past it, where the knee ends and
/// what it holds there (`Terrain.dll:0x1009a168`, `0x100a2354`, `0x100a2360`, `0x1009b35c`,
/// and the literal 2.0 at `0x1004f28e`).
pub const KNEE_FROM: f32 = 1.0;
pub const KNEE_SLOPE: f32 = 1.0 / 6.0;
pub const KNEE_OFFSET: f32 = 5.0 / 6.0;
pub const KNEE_TO: f32 = 7.0;
pub const KNEE_HELD: f32 = 2.0;

/// The specular colour's knee: its slope up to 1, its slope and offset up to
/// [`SPILL_TO`], and 1 beyond (`0x100a2374`, `0x100a2378`, `0x100a2384`, `0x1009b354`).
pub const SPILL_SLOPE: f32 = 0.8;
pub const SPILL_OVER_SLOPE: f32 = 0.1;
pub const SPILL_OVER_OFFSET: f32 = 0.7;
pub const SPILL_TO: f32 = 3.0;

/// A lit component past 1: `c/6 + 5/6` up to 7, and 2 beyond (`0x1004f26d`–`0x1004f2b2`).
pub fn knee(c: f32) -> f32 {
    if c <= KNEE_FROM {
        c
    } else if c <= KNEE_TO {
        c * KNEE_SLOPE + KNEE_OFFSET
    } else {
        KNEE_HELD
    }
}

/// The specular component's knee (`0x1004f4c3`–`0x1004f521`).
pub fn spill(s: f32) -> f32 {
    if s <= 1.0 {
        s * SPILL_SLOPE
    } else if s <= SPILL_TO {
        s * SPILL_OVER_SLOPE + SPILL_OVER_OFFSET
    } else {
        1.0
    }
}

/// A vertex's two colours.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shaded {
    /// The diffuse colour, which the texture stages modulate.
    pub diffuse: [f32; 3],
    /// The specular colour, added after them.
    pub specular: [f32; 3],
}

/// The two colours of a vertex. `lights` is what the lights give the material's diffuse and
/// `highlight` what they give its specular; the material's `ambient` is added to the first,
/// and the scene colour is a **floor** under the sum, not a term of it: `g_FastProc`'s slot
/// `0x50` is `max(lights + ambient, scene)` in all four of its builds
/// (`Ngi32.dll:0x100248a0`, `0x1001bdd0`, `0x1001d980`, `0x1001ffc0`, called at
/// `Terrain.dll:0x1004f23d`). What the knee leaves above 1 is taken off the diffuse and put
/// on the specular (`0x1004f3e5`–`0x1004f4a5`), so an overbright light whitens.
pub fn lit(lights: [f32; 3], highlight: [f32; 3], ambient: [f32; 3], scene: [f32; 3]) -> Shaded {
    let kneed: [f32; 3] = std::array::from_fn(|c| knee((lights[c] + ambient[c]).max(scene[c])));
    Shaded {
        diffuse: kneed.map(|k| k.min(1.0)),
        specular: std::array::from_fn(|c| spill(highlight[c] + (kneed[c] - 1.0).max(0.0))),
    }
}

/// What a directional light gives a normal: the cosine to the way back along its travel,
/// nothing from behind (`Ngi32.dll:0x10016139`–`0x10016159`).
pub fn cosine(normal: Vec3, travel: Vec3) -> f32 {
    normal.dot(-travel).max(0.0)
}

/// A light's highlight on a material of specular `power`: the light's travel mirrored in the
/// normal, against the unit vector to the eye, squared `power − 1` times
/// (`0x1001616f`–`0x100161ca`); none at power 0, which turns the specular off for the whole
/// item (`Terrain.dll:0x1004e9b7`).
pub fn highlight(normal: Vec3, travel: Vec3, to_eye: Vec3, power: u8) -> f32 {
    let cosine = cosine(normal, travel);
    let s = (travel + 2.0 * cosine * normal).dot(to_eye);
    if cosine <= 0.0 || s <= 0.0 || power == 0 {
        return 0.0;
    }
    (1..power).fold(s, |s, _| s * s)
}

/// How much of a vertex's colour the fog keeps: all of it up to `start`, none past `end`,
/// and between them linear in the **squared** distance to the eye
/// (`0x1004f187`–`0x1004f213`; the squares and the reciprocal of their difference are kept
/// by `0x1004bfb0`).
pub fn fog(distance: f32, start: f32, end: f32) -> f32 {
    let (d, near, far) = (distance * distance, start * start, end * end);
    if d < near {
        1.0
    } else if d > far {
        0.0
    } else {
        1.0 - (d - near) / (far - near).max(1e-6)
    }
}

/// What the frame holds where a fogged, textured vertex is drawn: the texel by the diffuse
/// colour, the specular added, and the fog colour mixed in by what the fog does not keep --
/// all on the stored values, each step held to 1 as the device holds it.
pub fn drawn(texel: [f32; 3], shaded: &Shaded, fog_colour: [f32; 3], keep: f32) -> [f32; 3] {
    std::array::from_fn(|c| {
        let lit = (texel[c] * shaded.diffuse[c] + shaded.specular[c]).min(1.0);
        fog_colour[c] + (lit - fog_colour[c]) * keep
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_knees_constants_are_the_binarys() {
        // The floats `Terrain.dll` keeps at 0x100a2354, 0x100a2360, 0x1009b35c, 0x100a2374,
        // 0x100a2378, 0x100a2384 and 0x1009b354.
        assert_eq!(f32::from_bits(0x3e2a_aaab), KNEE_SLOPE);
        assert_eq!(f32::from_bits(0x3f55_5555), 0.833_333_3);
        assert!((KNEE_OFFSET - f32::from_bits(0x3f55_5555)).abs() < 1e-7);
        assert_eq!(f32::from_bits(0x40e0_0000), KNEE_TO);
        assert_eq!(f32::from_bits(0x3f4c_cccd), SPILL_SLOPE);
        assert_eq!(f32::from_bits(0x3dcc_cccd), SPILL_OVER_SLOPE);
        assert_eq!(f32::from_bits(0x3f33_3333), SPILL_OVER_OFFSET);
        assert_eq!(f32::from_bits(0x4040_0000), SPILL_TO);
    }

    #[test]
    fn a_component_is_kneed_past_1_and_held_at_2_past_7() {
        assert_eq!(knee(0.4), 0.4);
        assert_eq!(knee(1.0), 1.0);
        assert!((knee(4.0) - 1.5).abs() < 1e-6, "4/6 + 5/6");
        assert!((knee(7.0) - 2.0).abs() < 1e-6, "the knee meets what it is held at");
        assert_eq!(knee(30.0), 2.0);
    }

    #[test]
    fn the_specular_is_scaled_to_four_fifths_and_meets_1_at_3() {
        assert_eq!(spill(0.0), 0.0);
        assert!((spill(0.5) - 0.4).abs() < 1e-6);
        assert!((spill(1.0) - 0.8).abs() < 1e-6);
        assert!((spill(2.0) - 0.9).abs() < 1e-6);
        assert!((spill(3.0) - 1.0).abs() < 1e-6);
        assert_eq!(spill(9.0), 1.0);
    }

    #[test]
    fn the_scene_colour_is_a_floor_under_the_light_not_a_term_of_it() {
        // C02 M04's scene colour at 02:29, (86, 111, 67) over 255.
        let scene = [0.337, 0.435, 0.263];
        // A face side-on to both lights takes the scene colour whole.
        assert_eq!(lit([0.0; 3], [0.0; 3], [0.0; 3], scene).diffuse, scene);
        // A light below the floor on a channel leaves the floor there; one above it replaces
        // the floor rather than standing on it.
        let s = lit([0.2, 0.6, 0.1], [0.0; 3], [0.0; 3], scene);
        assert_eq!(s.diffuse, [0.337, 0.6, 0.263]);
        assert_eq!(s.specular, [0.0; 3]);
        // The material's ambient is added before the floor is taken.
        assert_eq!(lit([0.2, 0.2, 0.2], [0.0; 3], [0.3, 0.0, 0.0], scene).diffuse, [0.5, 0.435, 0.263]);
    }

    #[test]
    fn what_the_knee_leaves_above_1_whitens_through_the_specular() {
        // A light of 4 on a white material: kneed to 1.5, so the diffuse is 1 and the half
        // over goes to the specular at four fifths.
        let s = lit([4.0, 0.5, 0.0], [0.0; 3], [0.0; 3], [0.0; 3]);
        assert_eq!(s.diffuse, [1.0, 0.5, 0.0]);
        assert!((s.specular[0] - 0.4).abs() < 1e-6, "{:?}", s.specular);
        assert_eq!(s.specular[1], 0.0);
        // A highlight rides the same colour, and the two are kneed together.
        let s = lit([4.0, 0.0, 0.0], [0.75, 0.25, 0.0], [0.0; 3], [0.0; 3]);
        assert!((s.specular[0] - (1.25 * 0.1 + 0.7)).abs() < 1e-6);
        assert!((s.specular[1] - 0.2).abs() < 1e-6);
    }

    #[test]
    fn a_light_reaches_only_the_side_it_falls_on_and_its_highlight_is_squared_by_the_power() {
        let down = Vec3::NEG_Z;
        assert_eq!(cosine(Vec3::Z, down), 1.0);
        assert_eq!(cosine(Vec3::NEG_Z, down), 0.0, "nothing from behind");
        assert_eq!(cosine(Vec3::ZERO, down), 0.0, "a basement corner's zero normal takes none");
        // Light falling 45 degrees onto level ground, the eye where it mirrors to.
        let travel = Vec3::new(1.0, 0.0, -1.0).normalize();
        let mirrored = Vec3::new(1.0, 0.0, 1.0).normalize();
        assert!((highlight(Vec3::Z, travel, mirrored, 3) - 1.0).abs() < 1e-5);
        // 20 degrees off the mirror: the cosine squared twice at power 3, five times at 6.
        let off = Vec3::new(20f32.to_radians().cos(), 20f32.to_radians().sin(), 0.0);
        let to_eye = (mirrored + (off - Vec3::X) * 0.5).normalize();
        let s = (travel + 2.0 * cosine(Vec3::Z, travel) * Vec3::Z).dot(to_eye);
        assert!((highlight(Vec3::Z, travel, to_eye, 3) - s.powi(4)).abs() < 1e-5);
        assert!((highlight(Vec3::Z, travel, to_eye, 6) - s.powi(32)).abs() < 1e-5);
        assert_eq!(highlight(Vec3::Z, travel, to_eye, 0), 0.0, "power 0 has no highlight");
        assert_eq!(highlight(Vec3::Z, travel, -mirrored, 3), 0.0, "nor the far side of the mirror");
    }

    #[test]
    fn the_fog_is_linear_in_the_squared_distance() {
        assert_eq!(fog(0.0, 0.0, 500.0), 1.0);
        // Half way out the game keeps three quarters, where a fog linear in the distance
        // itself would keep half.
        assert!((fog(250.0, 0.0, 500.0) - 0.75).abs() < 1e-6);
        assert!((fog(400.0, 0.0, 500.0) - 0.36).abs() < 1e-6);
        assert_eq!(fog(500.0, 0.0, 500.0), 0.0);
        assert_eq!(fog(900.0, 0.0, 500.0), 0.0);
        // The clouds' own range, 5000 to 11380.7 (`Terrain.dll:0x1007a5d4`): whole at the
        // apex, gone by the third ring of the cap.
        assert_eq!(fog(5000.0, 5000.0, 11380.7), 1.0);
        assert!((fog(7035.0, 5000.0, 11380.7) - 0.766).abs() < 2e-3);
        assert!(fog(11063.0, 5000.0, 11380.7) < 0.07);
        assert_eq!(fog(15553.0, 5000.0, 11380.7), 0.0);
    }

    /// What an sRGB target would have shown had the same blend been done on decoded values:
    /// the renderer's arithmetic before this module.
    fn decoded_blend(texel: f32, lit: f32, fog: f32, keep: f32) -> f32 {
        let decode = |v: f32| if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) };
        let encode = |v: f32| if v <= 0.003_130_8 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 };
        encode(decode(fog) + (decode(texel) * decode(lit) - decode(fog)) * keep)
    }

    #[test]
    fn a_fog_blend_on_stored_values_is_darker_than_the_same_blend_decoded() {
        // A dark ground texel, 0.3, lit at 0.5, half fogged toward C02 M04's green 0.675:
        // the game writes 0.4125, and the decoded blend wrote 0.5004 -- 22 levels of 255 paler.
        let shaded = Shaded { diffuse: [0.5; 3], specular: [0.0; 3] };
        let game = drawn([0.3; 3], &shaded, [0.675; 3], 0.5)[0];
        assert!((game - 0.4125).abs() < 1e-6);
        let before = decoded_blend(0.3, 0.5, 0.675, 0.5);
        assert!((before - 0.5004).abs() < 1e-3, "{before}");
        assert!(((before - game) * 255.0 - 22.4).abs() < 0.1);
        // Unfogged it ran the other way, and by less: the product of two decoded values is
        // 0.1318 encoded, under the 0.15 the game multiplies to.
        assert!((decoded_blend(0.3, 0.5, 0.675, 1.0) - 0.1318).abs() < 1e-3);
        assert!((drawn([0.3; 3], &shaded, [0.675; 3], 1.0)[0] - 0.15).abs() < 1e-6);
    }

    #[test]
    fn the_shader_twin_parses_and_names_the_same_constants() {
        for needle in [
            "c / 6.0",
            "5.0 / 6.0",
            "vec3<f32>(7.0)",
            "s * 0.8",
            "s * 0.1",
            "vec3<f32>(0.7)",
            "vec3<f32>(3.0)",
        ] {
            assert!(SHADE_WGSL.contains(needle), "shade.wgsl lost {needle}");
        }
        let module = wgpu::naga::front::wgsl::parse_str(SHADE_WGSL).expect("shade.wgsl parses");
        wgpu::naga::valid::Validator::new(
            wgpu::naga::valid::ValidationFlags::all(),
            wgpu::naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .expect("shade.wgsl validates");
    }
}
