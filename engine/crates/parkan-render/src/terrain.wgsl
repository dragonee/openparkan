// The ground: two material layers mixed by the layer-1 weight, lit.

struct Frame {
    view_proj: mat4x4<f32>,
    // The sun object's two directional lights: the direction each travels, and its
    // colour in the files' display space (docs/10-sky.md).
    light_direction: vec4<f32>,
    light_colour: vec4<f32>,
    second_direction: vec4<f32>,
    second_colour: vec4<f32>,
    // The colour added to every material's emissive (sky slot 20), display space.
    scene_colour: vec4<f32>,
    // Linear.
    fog_colour: vec4<f32>,
    // x start, y end: linear range fog from the eye (docs/10-sky.md, "Fog"); z the height
    // nothing below draws at where w is 1, a reflection's clip plane.
    fog: vec4<f32>,
    eye: vec4<f32>,
};

// A display-space colour as the linear value an sRGB target needs (frame.rs `linear`).
fn linear(c: vec3<f32>) -> vec3<f32> {
    let low = c / 12.92;
    let high = pow((max(c, vec3<f32>(0.0)) + 0.055) / 1.055, vec3<f32>(2.4));
    return select(high, low, c <= vec3<f32>(0.04045));
}

// STAND-IN: docs/10-sky.md#not-resolved -- whether ForceSWFog is read outside Terrain.dll,
// which asks Direct3D for linear range vertex fog, is not read; the fog is taken per
// fragment, which looks the same.
fn fogged(colour: vec3<f32>, world: vec3<f32>, toward: vec4<f32>) -> vec3<f32> {
    let d = distance(world, frame.eye.xyz);
    let span = max(frame.fog.y - frame.fog.x, 0.001);
    let keep = clamp((frame.fog.y - d) / span, 0.0, 1.0);
    let fog = mix(frame.fog_colour.rgb, toward.rgb, toward.w);
    return mix(fog, colour, keep);
}

struct Layers {
    // Each layer material's diffuse, decoded to linear.
    tint1: vec4<f32>,
    // w is 1 when the faces wear a second layer.
    tint2: vec4<f32>,
};

@group(0) @binding(0) var<uniform> frame: Frame;
@group(1) @binding(0) var<uniform> layers: Layers;
@group(1) @binding(1) var layer1: texture_2d<f32>;
@group(1) @binding(2) var layer2: texture_2d<f32>;
@group(1) @binding(3) var ground: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv1: vec2<f32>,
    @location(3) uv2: vec2<f32>,
    @location(4) blend: f32,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(4) world: vec3<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv1: vec2<f32>,
    @location(2) uv2: vec2<f32>,
    @location(3) blend: f32,
};

@vertex
fn vs_main(v: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.clip = frame.view_proj * vec4<f32>(v.position, 1.0);
    out.world = v.position;
    out.normal = v.normal;
    out.uv1 = v.uv1;
    out.uv2 = v.uv2;
    out.blend = v.blend;
    return out;
}

// The lit colour a face's lights give it, held to 1 as Direct3D's fixed-function lighting
// holds it.
fn lit_at(normal: vec3<f32>) -> vec3<f32> {
    let n = normalize(normal);
    let a = max(dot(n, -frame.light_direction.xyz), 0.0);
    let b = max(dot(n, -frame.second_direction.xyz), 0.0);
    let lights = frame.light_colour.rgb * a + frame.second_colour.rgb * b;
    return min(vec3<f32>(1.0), frame.scene_colour.rgb + lights);
}

@fragment
fn fs_main(v: VertexOut) -> @location(0) vec4<f32> {
    if frame.fog.w > 0.5 && v.world.z < frame.fog.z {
        discard;
    }
    var colour = textureSample(layer1, ground, v.uv1).rgb * layers.tint1.rgb;
    let under = textureSample(layer2, ground, v.uv2).rgb * layers.tint2.rgb;
    if layers.tint2.w > 0.5 {
        colour = mix(under, colour, v.blend);
    }
    return vec4<f32>(fogged(colour * linear(lit_at(v.normal)), v.world, vec4<f32>(0.0)), 1.0);
}

// The water under `REFLECTION_SHIFTED` (`Terrain.dll:0x1002ca80`, render phase 10): the
// reflection texture over the water box, its lookup displaced by the environment bump map,
// times the face's lit colour.
struct Water {
    // The water box's corners, x0 y0 x1 y1.
    box_: vec4<f32>,
    // x the bump's drift, (clock ms mod EMBMBumpMove) / EMBMBumpMove; y EMBMBumpTile; z the
    // bump matrix's scale, EMBMCoeff00 and 11; w EMBMMaxVal.
    bump: vec4<f32>,
};

@group(2) @binding(0) var<uniform> water: Water;
@group(2) @binding(1) var reflection: texture_2d<f32>;
@group(2) @binding(2) var reflection_sampler: sampler;

// The 32 × 32 bump map `CShade` makes (`0x100491e0`), texel (i, j): du and dv are
// round(EMBMMaxVal × cos and sin of 4π(i/32 + j/32 − 1)), signed bytes, sampled nearest and
// tiled.
//
// STAND-IN: docs/03-terrain.md#not-established -- how far the bump displaces is not read
// from the binary: a signed byte stands for −1 to 1 at 127, as the device's convention has it.
fn bump_at(uv: vec2<f32>) -> vec2<f32> {
    let cell = floor(fract(uv) * 32.0);
    let theta = 4.0 * 3.14159 * (cell.x / 32.0 + cell.y / 32.0 - 1.0);
    return vec2<f32>(round(water.bump.w * cos(theta)), round(water.bump.w * sin(theta))) / 127.0;
}

@fragment
fn fs_water(v: VertexOut) -> @location(0) vec4<f32> {
    let size = water.box_.zw - water.box_.xy;
    let over = vec2<f32>(1.0 - (v.world.y - water.box_.y) / size.y, 1.0 - (v.world.x - water.box_.x) / size.x);
    let bumped = over + water.bump.z * bump_at(water.bump.y * over + vec2<f32>(water.bump.x));
    let seen = textureSample(reflection, reflection_sampler, bumped).rgb;
    let colour = seen * layers.tint1.rgb * linear(lit_at(v.normal));
    return vec4<f32>(fogged(colour, v.world, vec4<f32>(0.0)), 1.0);
}
