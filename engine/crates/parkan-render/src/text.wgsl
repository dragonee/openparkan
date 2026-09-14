// Text: glyph quads from the game font's atlas, placed in screen pixels from an
// anchor in normalised device coordinates, tinted, over everything.

struct Screen {
    size: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(0) var<uniform> screen: Screen;
@group(0) @binding(1) var atlas: texture_2d<f32>;
@group(0) @binding(2) var atlas_sampler: sampler;

struct VertexIn {
    @location(0) anchor: vec2<f32>,
    @location(1) offset: vec2<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) colour: vec4<f32>,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) colour: vec4<f32>,
};

@vertex
fn vs_main(v: VertexIn) -> VertexOut {
    var out: VertexOut;
    // Snap the anchor to a whole pixel so glyphs land on the pixel grid.
    let half = screen.size * 0.5;
    let anchor = floor((v.anchor * vec2<f32>(1.0, -1.0) + 1.0) * half + 0.5);
    let pixel = anchor + v.offset;
    out.clip = vec4<f32>(pixel.x / half.x - 1.0, 1.0 - pixel.y / half.y, 0.0, 1.0);
    out.uv = v.uv;
    out.colour = v.colour;
    return out;
}

@fragment
fn fs_main(v: VertexOut) -> @location(0) vec4<f32> {
    // The atlas is white glyphs on black: its brightness is the glyph's coverage.
    let coverage = textureSample(atlas, atlas_sampler, v.uv).r;
    return vec4<f32>(v.colour.rgb, v.colour.a * coverage);
}
