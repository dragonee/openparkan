// The ground: two material layers mixed by the layer-1 weight, lit.

struct Frame {
    view_proj: mat4x4<f32>,
    // The sun, as the sky's keyframes give it.
    light_direction: vec4<f32>,
    light_colour: vec4<f32>,
    // The colour added to every material's emissive (sky slot 20).
    scene_colour: vec4<f32>,
    fog_colour: vec4<f32>,
    // x start, y end: linear range fog from the eye (docs/10-sky.md, "Fog").
    fog: vec4<f32>,
    eye: vec4<f32>,
};

fn fogged(colour: vec3<f32>, world: vec3<f32>, toward: vec4<f32>) -> vec3<f32> {
    let d = distance(world, frame.eye.xyz);
    let span = max(frame.fog.y - frame.fog.x, 0.001);
    let keep = clamp((frame.fog.y - d) / span, 0.0, 1.0);
    let fog = mix(frame.fog_colour.rgb, toward.rgb, toward.w);
    return mix(fog, colour, keep);
}

struct Layers {
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

@fragment
fn fs_main(v: VertexOut) -> @location(0) vec4<f32> {
    var colour = textureSample(layer1, ground, v.uv1).rgb * layers.tint1.rgb;
    let under = textureSample(layer2, ground, v.uv2).rgb * layers.tint2.rgb;
    if layers.tint2.w > 0.5 {
        colour = mix(under, colour, v.blend);
    }
    let n = normalize(v.normal);
    let diffuse = max(dot(n, -frame.light_direction.xyz), 0.0);
    // A lit vertex colour is held to 1, as Direct3D's fixed-function lighting holds it.
    let light = min(vec3<f32>(1.0), frame.scene_colour.rgb + frame.light_colour.rgb * diffuse);
    return vec4<f32>(fogged(colour * light, v.world, vec4<f32>(0.0)), 1.0);
}
