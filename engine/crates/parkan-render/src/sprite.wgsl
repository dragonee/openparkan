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

// How much of a sprite the fog keeps (`Terrain.dll:0x1004bf20`, the shader component's slot 4
// at `0x1004f710`): all of it up to `start`, none past `end`, and between them
// 1 - (d^2 - start^2) / (end^2 - start^2). `shade.rs`'s `fog` is the same arithmetic, tested.
fn sprite_fog(distance: f32, start: f32, end: f32) -> f32 {
    let d = distance * distance;
    let near = start * start;
    let far = end * end;
    if d <= near {
        return 1.0;
    }
    if d >= far {
        return 0.0;
    }
    return 1.0 - (d - near) / max(far - near, 1e-6);
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
    // The fog's share is in the specular alpha of the sprite's one colour pair, linear in the
    // *squared* distance between the fog's start and end as the world's is
    // (`Terrain.dll:0x1004bf20`, docs/11, "How an effect sprite is coloured"); effect draw flag
    // 4 holds it at 1, which the tint's w says below.
    //
    // STAND-IN: docs/11-effects.md#how-an-effect-sprite-is-coloured--read-and-measured -- the
    // game takes one distance for the sprite, the one its item is built with
    // (`Terrain.dll:0x1003c340`), and all four corners carry that factor; here each pixel
    // takes its own distance.
    let keep = sprite_fog(distance(v.world, camera.eye.xyz), camera.fog.x, camera.fog.y);
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
    //
    // The rain's and the snow's quads are pre-transformed vertices with a colour of their own
    // in the diffuse and a specular of 0xff000000, whose alpha is the fog factor: the texture
    // times that colour, and no fog (docs/10, "The weather"). A tint's w of 2 says so.
    let device = clamp(camera.scene_colour.rgb + v.tint.rgb, vec3<f32>(0.0), vec3<f32>(1.0));
    // A tint's w of 3 is the sprite of an effect whose header holds the fog factor at 1 (effect
    // draw flag 4): its look's colour, and no fog.
    let own = v.tint.w > 1.5 && v.tint.w < 2.5;
    let unfogged = v.tint.w > 1.5;
    let plain = v.tint.w < 0.5 || v.tint.w > 2.5;
    let lit = select(select(device, look.ambient.rgb * v.tint.rgb, plain), v.tint.rgb, own);
    return vec4<f32>(mix(fog, display(texel.rgb) * lit, select(keep, 1.0, unfogged)), texel.a * v.alpha);
}
