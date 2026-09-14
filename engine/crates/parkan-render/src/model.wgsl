// Placed objects: a texture lit fixed-function style, per instance transform.

struct Frame {
    view_proj: mat4x4<f32>,
    // The sun object's two directional lights: the direction each travels, and its
    // colour in the files' display space (docs/10-sky.md).
    light_direction: vec4<f32>,
    light_colour: vec4<f32>,
    second_direction: vec4<f32>,
    second_colour: vec4<f32>,
    // The colour added to every material's emissive (sky slot 20), display space.
    scene_colour: vec4<f32>,
    // Linear.
    fog_colour: vec4<f32>,
    // x start, y end: linear range fog from the eye (docs/10-sky.md, "Fog").
    fog: vec4<f32>,
    eye: vec4<f32>,
};

// A display-space colour as the linear value an sRGB target needs (frame.rs `linear`).
fn linear(c: vec3<f32>) -> vec3<f32> {
    let low = c / 12.92;
    let high = pow((max(c, vec3<f32>(0.0)) + 0.055) / 1.055, vec3<f32>(2.4));
    return select(high, low, c <= vec3<f32>(0.04045));
}

// STAND-IN: docs/10-sky.md#not-resolved -- whether ForceSWFog is read outside Terrain.dll,
// which asks Direct3D for linear range vertex fog, is not read; the fog is taken per
// fragment, which looks the same.
fn fogged(colour: vec3<f32>, world: vec3<f32>, toward: vec4<f32>) -> vec3<f32> {
    let d = distance(world, frame.eye.xyz);
    let span = max(frame.fog.y - frame.fog.x, 0.001);
    let keep = clamp((frame.fog.y - d) / span, 0.0, 1.0);
    let fog = mix(frame.fog_colour.rgb, toward.rgb, toward.w);
    return mix(fog, colour, keep);
}

struct Instance {
    model: mat4x4<f32>,
};

struct Look {
    // The material's own colours, display space.
    diffuse: vec4<f32>,
    emissive: vec4<f32>,
    // The fog colour this material's blend mode draws toward; w 1 where it overrides.
    fog: vec4<f32>,
    // The cell's rectangle: u0, v0, du, dv.
    cell: vec4<f32>,
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
    @location(2) world: vec3<f32>,
};

@vertex
fn vs_main(v: VertexIn) -> VertexOut {
    var out: VertexOut;
    let world = instance.model * vec4<f32>(v.position, 1.0);
    out.clip = frame.view_proj * world;
    out.world = world.xyz;
    // The model matrix is a rotation and a uniform scale.
    out.normal = (instance.model * vec4<f32>(v.normal, 0.0)).xyz;
    out.uv = v.uv;
    return out;
}

@fragment
fn fs_main(v: VertexOut) -> @location(0) vec4<f32> {
    // The cell rewrites the coordinates in place: u0 + u × du, v0 + v × dv.
    let texel = textureSample(skin, skin_sampler, look.cell.xy + v.uv * look.cell.zw);
    let n = normalize(v.normal);
    let a = max(dot(n, -frame.light_direction.xyz), 0.0);
    let b = max(dot(n, -frame.second_direction.xyz), 0.0);
    // D3D's lit vertex colour, modulated by the texture: emissive (the material's ambient
    // colour plus the scene colour the sky adds to every material) and the diffuse lights,
    // held to 1 as fixed-function lighting holds it, then decoded. The alpha is the
    // texture's times the material's ambient alpha.
    let lights = frame.light_colour.rgb * a + frame.second_colour.rgb * b;
    let lit = min(vec3<f32>(1.0), look.emissive.rgb + frame.scene_colour.rgb + look.diffuse.rgb * lights);
    return vec4<f32>(fogged(texel.rgb * linear(lit), v.world, look.fog), texel.a * look.diffuse.a);
}
