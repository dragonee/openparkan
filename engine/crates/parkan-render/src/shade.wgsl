// The game's own shade, one vertex at a time: `CShade::ShadeIndexedStrided`
// (`Terrain.dll:0x1004df70`), which stands in for the device's lighting and fog while
// `UseDXLighting` is off, as it ships (docs/10-sky.md, "The lit colour is the game's own").
// Prepended to the pipelines that light; `shade.rs` holds the same arithmetic, tested.
//
// Every colour here is the value the files and the frame hold: the game's device does its
// multiplies, adds and blends on stored values, and so does this renderer.

// What a directional light gives a normal: the cosine to the way back along its travel, and
// nothing from behind (`Ngi32.dll:0x10016139`). A zero normal, a basement corner's, takes none.
fn shade_cosine(normal: vec3<f32>, travel: vec3<f32>) -> f32 {
    return max(dot(normal, -travel), 0.0);
}

// A light's highlight: its travel mirrored in the normal against the way to the eye, squared
// `power - 1` times (`0x1001616f`-`0x100161ca`); none at power 0 (`Terrain.dll:0x1004e9b7`).
fn shade_highlight(normal: vec3<f32>, travel: vec3<f32>, to_eye: vec3<f32>, cosine: f32, power: f32) -> f32 {
    let s = dot(travel + 2.0 * cosine * normal, to_eye);
    if cosine <= 0.0 || s <= 0.0 || power < 0.5 {
        return 0.0;
    }
    return pow(s, exp2(max(power - 1.0, 0.0)));
}

// A lit component past 1 is kneed: c/6 + 5/6 up to 7, and 2 beyond (`0x1004f26d`-`0x1004f2b2`,
// the constants at `0x100a2354` and `0x100a2360`).
fn shade_knee(c: vec3<f32>) -> vec3<f32> {
    let over = c / 6.0 + vec3<f32>(5.0 / 6.0);
    return select(select(vec3<f32>(2.0), over, c <= vec3<f32>(7.0)), c, c <= vec3<f32>(1.0));
}

// The specular colour's own knee: 0.8 s to 1, 0.1 s + 0.7 to 3, and 1 beyond
// (`0x1004f4c3`-`0x1004f521`, `0x100a2374`, `0x100a2378`, `0x100a2384`).
fn shade_spill(s: vec3<f32>) -> vec3<f32> {
    let over = s * 0.1 + vec3<f32>(0.7);
    return select(select(vec3<f32>(1.0), over, s <= vec3<f32>(3.0)), s * 0.8, s <= vec3<f32>(1.0));
}

struct Shaded {
    // The vertex's diffuse colour, which the texture stages modulate.
    diffuse: vec3<f32>,
    // Its specular colour, added after them.
    specular: vec3<f32>,
};

// The two colours of a vertex. `lights` is what the lights give the material's diffuse,
// `highlight` what they give its specular. The material's ambient is added to the first and
// the scene colour is a **floor** under the sum, not a term of it (`g_FastProc` slot 0x50,
// `Ngi32.dll:0x100248a0`); what the knee leaves above 1 goes over to the specular colour
// (`Terrain.dll:0x1004f3e5`-`0x1004f4a5`).
fn shade_lit(lights: vec3<f32>, highlight: vec3<f32>, ambient: vec3<f32>, scene: vec3<f32>) -> Shaded {
    let kneed = shade_knee(max(lights + ambient, scene));
    var out: Shaded;
    out.diffuse = min(kneed, vec3<f32>(1.0));
    out.specular = shade_spill(highlight + max(kneed - vec3<f32>(1.0), vec3<f32>(0.0)));
    return out;
}

// How much of a vertex's colour the fog keeps: all of it up to `start`, none past `end`, and
// between them linear in the **squared** distance (`0x1004f187`-`0x1004f213`, the squares and
// their reciprocal kept by `0x1004bfb0`). It goes out in the specular colour's alpha, which
// is where the device takes its fog factor from.
fn shade_fog(distance: f32, start: f32, end: f32) -> f32 {
    let d = distance * distance;
    let near = start * start;
    let far = end * end;
    if d < near {
        return 1.0;
    }
    if d > far {
        return 0.0;
    }
    return 1.0 - (d - near) / max(far - near, 1e-6);
}
