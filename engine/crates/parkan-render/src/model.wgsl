// Placed objects: a texture lit fixed-function style, per instance transform.

struct Frame {
    view_proj: mat4x4<f32>,
    light_direction: vec4<f32>,
    light_colour: vec4<f32>,
    scene_colour: vec4<f32>,
};

struct Instance {
    model: mat4x4<f32>,
};

struct Look {
    diffuse: vec4<f32>,
    emissive: vec4<f32>,
};

@group(0) @binding(0) var<uniform> frame: Frame;
@group(1) @binding(0) var<uniform> instance: Instance;
@group(2) @binding(0) var<uniform> look: Look;
@group(2) @binding(1) var skin: texture_2d<f32>;
@group(2) @binding(2) var skin_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
};

@vertex
fn vs_main(v: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.clip = frame.view_proj * instance.model * vec4<f32>(v.position, 1.0);
    // The model matrix is a rotation about z and a uniform scale.
    out.normal = (instance.model * vec4<f32>(v.normal, 0.0)).xyz;
    out.uv = v.uv;
    return out;
}

@fragment
fn fs_main(v: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(skin, skin_sampler, v.uv);
    let n = normalize(v.normal);
    let diffuse = max(dot(n, -frame.light_direction.xyz), 0.0);
    // D3D's lit vertex colour, modulated by the texture: emissive (plus the
    // scene colour the sky adds to every material) and the diffuse light.
    let light = look.emissive.rgb + frame.scene_colour.rgb + look.diffuse.rgb * frame.light_colour.rgb * diffuse;
    return vec4<f32>(texel.rgb * light, texel.a);
}
