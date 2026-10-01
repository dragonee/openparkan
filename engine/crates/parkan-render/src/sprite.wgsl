// Effect sprites: textured quads, faded by alpha, in the material's blend mode.

struct Camera {
    view_proj: mat4x4<f32>,
    eye: vec4<f32>,
    fog_colour: vec4<f32>,
    // x start, y end.
    fog: vec4<f32>,
    // Added to a lit batch's emissive (sky slot 20), in display space. An effect sprite
    // never goes through a device material, so it is not read here.
    scene_colour: vec4<f32>,
};

struct Skin {
    // The fog colour this material's blend mode draws toward; w 1 where it overrides.
    toward: vec4<f32>,
    // The material's ambient colour, its self-light, in display space.
    ambient: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var skin: texture_2d<f32>;
@group(1) @binding(1) var skin_sampler: sampler;
@group(1) @binding(2) var<uniform> look: Skin;

// A linear colour encoded back to display space, where the sprites draw and blend.
fn display(c: vec3<f32>) -> vec3<f32> {
    let low = c * 12.92;
    let high = 1.055 * pow(max(c, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(high, low, c <= vec3<f32>(0.0031308));
}

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) alpha: f32,
    @location(3) tint: vec4<f32>,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) alpha: f32,
    @location(2) world: vec3<f32>,
    @location(3) tint: vec4<f32>,
};

@vertex
fn vs_main(v: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.clip = camera.view_proj * vec4<f32>(v.position, 1.0);
    out.uv = v.uv;
    out.alpha = v.alpha;
    out.world = v.position;
    out.tint = v.tint;
    return out;
}

// Drawn into the frame read without sRGB decoding, so a sprite blends in display space, as
// the game's device blends the values its surface holds (docs/11-effects.md, "Bolts, streams
// and fades"): the texture and the fog colour are encoded back, and the files' colours used as
// they are.
@fragment
fn fs_main(v: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(skin, skin_sampler, v.uv);
    let d = distance(v.world, camera.eye.xyz);
    // STAND-IN: docs/11-effects.md#how-an-effect-sprite-is-coloured--read-and-measured -- the
    // game puts a fog factor of its own in the specular alpha of every vertex, linear in the
    // *squared* distance between its near and far (`Terrain.dll:0x1004bf20`), and effect draw
    // flag 4 forces it to 1. Here the renderer's own fog, linear in the distance itself.
    let keep = clamp((camera.fog.y - d) / max(camera.fog.y - camera.fog.x, 0.001), 0.0, 1.0);
    let fog = display(mix(camera.fog_colour.rgb, look.toward.rgb, look.toward.w));
    // An effect sprite's quad carries one pre-lit colour on every vertex: the material's
    // ambient, with the ambient alpha -- where the fade lands -- as its alpha
    // (`Terrain.dll:0x1004f710`, the stream written at `0x1002845b` with a stride of 0).
    // The scene colour is not added: the draw item's flags are 4, without the 0x10 that would
    // build a device material at all (`0x1002fe3d`). A component above 1 is kneed to
    // c/6 + 5/6 and held at 2, which no shipped effect material reaches -- all 1598 entries
    // of the 243 they draw carry a byte ambient (docs/11, "How an effect sprite is coloured").
    //
    // A light drawn on a surface is an item of its own whose flags 0x14 do build a device
    // material, its ambient the light's colour over its length (`Terrain.dll:0x1002ae12`): its
    // self-light is the scene colour plus that, as a lit batch's emissive is, and the device
    // holds it at 1 (docs/11, "What a light does to a surface"). The tint's w says which.
    let device = clamp(camera.scene_colour.rgb + v.tint.rgb, vec3<f32>(0.0), vec3<f32>(1.0));
    let lit = select(look.ambient.rgb * v.tint.rgb, device, v.tint.w > 0.5);
    return vec4<f32>(mix(fog, display(texel.rgb) * lit, keep), texel.a * v.alpha);
}
