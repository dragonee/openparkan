// Effect sprites: textured quads, faded by alpha, in the material's blend mode.

struct Camera {
    view_proj: mat4x4<f32>,
    eye: vec4<f32>,
    fog_colour: vec4<f32>,
    // x start, y end.
    fog: vec4<f32>,
    // Added to every material's emissive (sky slot 20), in display space.
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

// The files' colours are display space; the target blends linear.
fn linear(c: vec3<f32>) -> vec3<f32> {
    let low = c / 12.92;
    let high = pow((max(c, vec3<f32>(0.0)) + 0.055) / 1.055, vec3<f32>(2.4));
    return select(high, low, c <= vec3<f32>(0.04045));
}

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) alpha: f32,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) alpha: f32,
    @location(2) world: vec3<f32>,
};

@vertex
fn vs_main(v: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.clip = camera.view_proj * vec4<f32>(v.position, 1.0);
    out.uv = v.uv;
    out.alpha = v.alpha;
    out.world = v.position;
    return out;
}

@fragment
fn fs_main(v: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(skin, skin_sampler, v.uv);
    let d = distance(v.world, camera.eye.xyz);
    let keep = clamp((camera.fog.y - d) / max(camera.fog.y - camera.fog.x, 0.001), 0.0, 1.0);
    let fog = mix(camera.fog_colour.rgb, look.toward.rgb, look.toward.w);
    // STAND-IN: docs/11-effects.md#not-resolved -- how an effect sprite's pre-lit vertices are
    // coloured is not traced: as a batch's emissive, the scene colour plus the material's
    // ambient, held to 1 and decoded, times the texture; its alpha the texture's times the
    // fade, which stands in for the ambient alpha.
    let lit = linear(min(vec3<f32>(1.0), camera.scene_colour.rgb + look.ambient.rgb));
    return vec4<f32>(mix(fog, texel.rgb * lit, keep), texel.a * v.alpha);
}
