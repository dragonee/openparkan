// The HUD's art: triangles in screen pixels over a page of the interface's textures, or
// flat colour, tinted, over everything.

struct Screen {
    size: vec2<f32>,
    // The pages' shared size, which page pixels are divided by.
    page: vec2<f32>,
};

@group(0) @binding(0) var<uniform> screen: Screen;
@group(0) @binding(1) var pages: texture_2d_array<f32>;
@group(0) @binding(2) var pages_sampler: sampler;

struct VertexIn {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) layer: u32,
    @location(3) colour: vec4<f32>,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) layer: u32,
    @location(2) colour: vec4<f32>,
};

@vertex
fn vs_main(v: VertexIn) -> VertexOut {
    var out: VertexOut;
    let half = screen.size * 0.5;
    out.clip = vec4<f32>(v.position.x / half.x - 1.0, 1.0 - v.position.y / half.y, 0.0, 1.0);
    out.uv = v.uv / screen.page;
    out.layer = v.layer;
    out.colour = v.colour;
    return out;
}

@fragment
fn fs_main(v: VertexOut) -> @location(0) vec4<f32> {
    // The target is read without sRGB decoding: the art, its tint and the blend all meet in
    // display space, as the game's 16-bit surfaces did.
    let texel = textureSample(pages, pages_sampler, v.uv, v.layer);
    return vec4<f32>(texel.rgb * v.colour.rgb, texel.a * v.colour.a);
}
