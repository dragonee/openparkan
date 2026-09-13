// Effect sprites: textured quads, faded by alpha, in the material's blend mode.

struct Camera {
    view_proj: mat4x4<f32>,
    eye: vec4<f32>,
    fog_colour: vec4<f32>,
    // x start, y end.
    fog: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var skin: texture_2d<f32>;
@group(1) @binding(1) var skin_sampler: sampler;
// The fog colour this material's blend mode draws toward; w 1 where it overrides.
@group(1) @binding(2) var<uniform> toward: vec4<f32>;

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
    let fog = mix(camera.fog_colour.rgb, toward.rgb, toward.w);
    return vec4<f32>(mix(fog, texel.rgb, keep), texel.a * v.alpha);
}
