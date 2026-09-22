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
    // x start, y end: linear range fog from the eye (docs/10-sky.md, "Fog"); z the height
    // nothing below draws at where w is 1, a reflection's clip plane.
    fog: vec4<f32>,
    // The eye; w its field of view across, radians.
    eye: vec4<f32>,
    // x 1: every instance draws flat in its paint (a HUD panel's view of a unit).
    paint: vec4<f32>,
};

// A display-space colour as the linear value an sRGB target needs (frame.rs `linear`).
fn linear(c: vec3<f32>) -> vec3<f32> {
    let low = c / 12.92;
    let high = pow((max(c, vec3<f32>(0.0)) + 0.055) / 1.055, vec3<f32>(2.4));
    return select(high, low, c <= vec3<f32>(0.04045));
}

// The scene's linear range fog: the game asks Direct3D for it on the distance to the eye and
// lets the device compute it per vertex (docs/10, "Nobody reads ForceSWFog" -- the one setting
// that might have said otherwise is read by nothing in the install). Taken per fragment here,
// which is the same fog without the seams a coarse mesh gives it.
fn fogged(colour: vec3<f32>, world: vec3<f32>, toward: vec4<f32>) -> vec3<f32> {
    let d = distance(world, frame.eye.xyz);
    let span = max(frame.fog.y - frame.fog.x, 0.001);
    let keep = clamp((frame.fog.y - d) / span, 0.0, 1.0);
    let fog = mix(frame.fog_colour.rgb, toward.rgb, toward.w);
    return mix(fog, colour, keep);
}

struct Instance {
    model: mat4x4<f32>,
    // The colour a view paints the instance in.
    paint: vec4<f32>,
};

struct Look {
    // The material's own colours, display space.
    diffuse: vec4<f32>,
    emissive: vec4<f32>,
    // The fog colour this material's blend mode draws toward; w 1 where it overrides.
    fog: vec4<f32>,
    // The cell's rectangle: u0, v0, du, dv.
    cell: vec4<f32>,
    // x 1: a lit batch, its lightmap in place of the scene's lights; y 1: the alpha test.
    lit: vec4<f32>,
    // A portal quad's first corner in the model's frame; w 1 on a portal quad.
    portal: vec4<f32>,
    // Its fade under a field of one radian: where it starts, where it is whole; z 1 where it
    // runs on the square root of the distance.
    portal_range: vec4<f32>,
};

@group(0) @binding(0) var<uniform> frame: Frame;
@group(1) @binding(0) var<uniform> instance: Instance;
@group(2) @binding(0) var<uniform> look: Look;
@group(2) @binding(1) var skin: texture_2d<f32>;
@group(2) @binding(2) var skin_sampler: sampler;
// A lit batch's lightmap page; white otherwise.
@group(2) @binding(3) var lightmap: texture_2d<f32>;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) lightmap: vec2<f32>,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) world: vec3<f32>,
    @location(3) paint: vec4<f32>,
    @location(4) lightmap: vec2<f32>,
    // The alpha a portal quad draws with in place of its material's; 1 on anything else.
    @location(5) fade: f32,
};

// A portal quad's alpha by the eye's distance from its first corner (Terrain.dll:0x1002c4d0,
// docs/24, "A building is drawn cell by cell through its portals"): 0 up to where its fade
// starts, 1 from where it is whole, both divided by the field of view.
fn portal_fade() -> f32 {
    if look.portal.w < 0.5 {
        return 1.0;
    }
    let corner = (instance.model * vec4<f32>(look.portal.xyz, 1.0)).xyz;
    let d = distance(corner, frame.eye.xyz);
    let at = select(d, sqrt(d), look.portal_range.z > 0.5);
    let field = max(frame.eye.w, 0.001);
    let near = look.portal_range.x / field;
    let far = look.portal_range.y / field;
    return clamp((at - near) / max(far - near, 0.001), 0.0, 1.0);
}

@vertex
fn vs_main(v: VertexIn) -> VertexOut {
    var out: VertexOut;
    let world = instance.model * vec4<f32>(v.position, 1.0);
    out.clip = frame.view_proj * world;
    out.world = world.xyz;
    // The model matrix is a rotation and a uniform scale.
    out.normal = (instance.model * vec4<f32>(v.normal, 0.0)).xyz;
    out.uv = v.uv;
    out.lightmap = v.lightmap;
    out.paint = instance.paint;
    out.fade = portal_fade();
    return out;
}

@fragment
fn fs_main(v: VertexOut) -> @location(0) vec4<f32> {
    if frame.fog.w > 0.5 && v.world.z < frame.fog.z {
        discard;
    }
    if frame.paint.x > 0.5 {
        // STAND-IN: docs/35-hud.md#the-unit-in-the-middle--read-and-seen -- how the camera
        // applies the colour it is handed in mode 2 is not read. Measured on the recording of
        // Mission 01: an intact dummy flat at about (39, 162, 41) and a mostly destroyed part
        // at (140, 59, 38), the node colour lifted by 0.15; bots shade a little with the light.
        let lift = 0.1 + 0.1 * max(dot(normalize(v.normal), -frame.light_direction.xyz), 0.0);
        return vec4<f32>(linear(min(v.paint.rgb + vec3<f32>(lift), vec3<f32>(1.0))), 1.0);
    }
    // The cell rewrites the coordinates in place: u0 + u × du, v0 + v × dv.
    let texel = textureSample(skin, skin_sampler, look.cell.xy + v.uv * look.cell.zw);
    let baked = textureSample(lightmap, skin_sampler, v.lightmap);
    if look.lit.x > 0.5 {
        // A lit batch (docs/07, "How a lightmapped batch is drawn"): the material's diffuse
        // becomes its self-light and its diffuse 0, so no light reaches it; the emissive is
        // the scene colour plus that diffuse, held to 1, and the texture and the lightmap
        // modulate it. Its alpha is the material's ambient alpha alone.
        let emissive = min(vec3<f32>(1.0), frame.scene_colour.rgb + look.diffuse.rgb);
        let colour = texel.rgb * baked.rgb * linear(emissive);
        return vec4<f32>(fogged(colour, v.world, look.fog), look.diffuse.a);
    }
    let n = normalize(v.normal);
    let a = max(dot(n, -frame.light_direction.xyz), 0.0);
    let b = max(dot(n, -frame.second_direction.xyz), 0.0);
    // D3D's lit vertex colour, modulated by the texture: emissive (the material's ambient
    // colour plus the scene colour the sky adds to every material) and the diffuse lights,
    // held to 1 as fixed-function lighting holds it, then decoded. The alpha is the
    // texture's times the material's ambient alpha.
    let lights = frame.light_colour.rgb * a + frame.second_colour.rgb * b;
    let lit = min(vec3<f32>(1.0), look.emissive.rgb + frame.scene_colour.rgb + look.diffuse.rgb * lights);
    // A portal quad's fade stands in for the material's ambient alpha (Terrain.dll:0x1002c63a).
    let alpha = texel.a * select(look.diffuse.a, v.fade, look.portal.w > 0.5);
    // Every blend mode but 0 alpha-tests GREATEREQUAL against ALPHAREF 1 (docs/07, "What a
    // blended batch writes"): an 8-bit alpha of 0 is dropped, and with it its depth.
    if look.lit.y > 0.5 && round(alpha * 255.0) < 1.0 {
        discard;
    }
    return vec4<f32>(fogged(texel.rgb * linear(lit), v.world, look.fog), alpha);
}
