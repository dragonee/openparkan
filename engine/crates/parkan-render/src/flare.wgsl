// The lens flare's twelve ghosts: quads already in clip space, added to the frame.

@group(0) @binding(0) var skin: texture_2d<f32>;
@group(0) @binding(1) var skin_sampler: sampler;

struct VertexIn {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) tint: vec4<f32>,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) tint: vec4<f32>,
};

@vertex
fn vs_main(v: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.clip = vec4<f32>(v.position, 0.0, 1.0);
    out.uv = v.uv;
    out.tint = v.tint;
    return out;
}

@fragment
fn fs_main(v: VertexOut) -> @location(0) vec4<f32> {
    return textureSample(skin, skin_sampler, v.uv) * v.tint;
}
