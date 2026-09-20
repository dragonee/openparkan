// The sky's cap: the nebula on it, the gradient over that, and the clouds on a copy of it
// lowered below the camera. One pipeline draws all three; the uniform says which.

struct Layer {
    view_proj: mat4x4<f32>,
    tint: vec4<f32>,
    uv_scale: f32,
    // 0 the gradient, 1 a texture times the tint, 2 the gradient held opaque.
    mode: u32,
};

@group(0) @binding(0) var<uniform> layer: Layer;
@group(1) @binding(0) var skin: texture_2d<f32>;
@group(1) @binding(1) var skin_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) colour: vec4<f32>,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) colour: vec4<f32>,
};

@vertex
fn vs_main(v: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.clip = layer.view_proj * vec4<f32>(v.position, 1.0);
    out.uv = v.uv * layer.uv_scale;
    out.colour = v.colour;
    return out;
}

@fragment
fn fs_main(v: VertexOut) -> @location(0) vec4<f32> {
    if layer.mode == 0u {
        return v.colour;
    }
    if layer.mode == 2u {
        return vec4<f32>(v.colour.rgb, 1.0);
    }
    return textureSample(skin, skin_sampler, v.uv) * layer.tint;
}
