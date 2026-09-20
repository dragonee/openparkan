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
    // The font's phase (Ngi32.dll's record 13, docs/12) modulates the texel by the run's
    // colour and takes its alpha from the texture alone, then alpha-tests GREATEREQUAL 1.
    // The atlas is keyed on palette index 0, which comes back with alpha 0, so the test
    // drops exactly the cell's background and every other texel is written opaque -- the
    // glyph's own grey ramp times the colour. The target is read without sRGB decoding,
    // so all of it is in display space.
    let texel = textureSample(atlas, atlas_sampler, v.uv);
    if texel.a < 0.5 {
        discard;
    }
    return vec4<f32>(texel.rgb * v.colour.rgb, v.colour.a);
}
