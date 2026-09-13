// The ground: two material layers mixed by the layer-1 weight, lit.

struct Frame {
    view_proj: mat4x4<f32>,
    // STAND-IN: docs/10-sky.md#not-resolved -- a fixed sun until M5 reads the sky.
    light_direction: vec4<f32>,
    light_colour: vec4<f32>,
    // The colour added to every material's emissive (sky slot 20), fixed until M5.
    scene_colour: vec4<f32>,
};

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
    @location(0) normal: vec3<f32>,
    @location(1) uv1: vec2<f32>,
    @location(2) uv2: vec2<f32>,
    @location(3) blend: f32,
};

@vertex
fn vs_main(v: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.clip = frame.view_proj * vec4<f32>(v.position, 1.0);
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
    let light = frame.scene_colour.rgb + frame.light_colour.rgb * diffuse;
    return vec4<f32>(colour * light, 1.0);
}
