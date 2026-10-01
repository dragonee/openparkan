// Placed objects: a texture by the colours the game's shade gives each vertex, per instance
// transform (shade.wgsl is prepended).

// A point light as the shade's lighter takes it (Terrain.dll:0x1004eacc-0x1004ecc9): where it
// stands and its range; its colour, as the files give it, and the owner it lights alone; its
// constant, linear and quadratic attenuation.
struct PointLight {
    position: vec4<f32>,
    colour: vec4<f32>,
    attenuation: vec4<f32>,
};

struct Frame {
    view_proj: mat4x4<f32>,
    // The sun object's two directional lights: the direction each travels, and its
    // colour as the files give it (docs/10-sky.md).
    light_direction: vec4<f32>,
    light_colour: vec4<f32>,
    second_direction: vec4<f32>,
    second_colour: vec4<f32>,
    // The floor under every lit colour (sky slot 20).
    scene_colour: vec4<f32>,
    // As the files give it.
    fog_colour: vec4<f32>,
    // x start, y end of the fog, from the eye (docs/10-sky.md, "Fog"); z the height
    // nothing below draws at where w is 1, a reflection's clip plane.
    fog: vec4<f32>,
    // The eye; w its field of view across, radians.
    eye: vec4<f32>,
    // x 1: every instance draws in its paint, the camera's mode 2 (a HUD panel's view of a
    // unit, a building being placed).
    paint: vec4<f32>,
    // x: how many of the point lights light every surface in their range, which come first;
    // y: how many there are.
    point_counts: vec4<f32>,
    points: array<PointLight, 64>,
};

struct Instance {
    model: mat4x4<f32>,
    // The colour a view paints the instance in.
    paint: vec4<f32>,
    // x: whose it is, which an owner-only light is matched against; 0 on what nobody owns.
    owner: vec4<f32>,
};

// What the frame's point lights give a vertex before its material's diffuse multiplies it
// (CShade::ShadeIndexedStrided, Terrain.dll:0x1004df70, and Ngi32.dll:0x100164f0): each
// light's colour times the cosine at the vertex times a0 + a1 x + a2 x^2, x the share of the
// light's range left, and nothing past the range or on a vertex turned away (docs/11, "What a
// light does to a surface"). The game lights per vertex, in software, and the rasteriser
// carries the result across the face.
fn point_lights(world: vec3<f32>, normal: vec3<f32>, owner: f32) -> vec3<f32> {
    var sum = vec3<f32>(0.0);
    if dot(normal, normal) <= 0.0 {
        return sum;
    }
    let n = normalize(normal);
    let everyones = i32(frame.point_counts.x);
    let count = i32(frame.point_counts.y);
    for (var i = 0; i < count; i++) {
        let light = frame.points[i];
        if i >= everyones && light.colour.w != owner {
            continue;
        }
        let to = light.position.xyz - world;
        let d = length(to);
        if d > light.position.w || d <= 0.0 {
            continue;
        }
        let facing = dot(n, to / d);
        if facing <= 0.0 {
            continue;
        }
        // The share of the range left, and the three terms on it.
        let x = (light.position.w - d) / light.position.w;
        let fall = light.attenuation.x + (light.attenuation.y + light.attenuation.z * x) * x;
        sum += light.colour.rgb * (facing * fall);
    }
    return sum;
}

struct Look {
    // The material's own colours, as the files give them: its diffuse with its ambient alpha,
    // and its ambient, the self-light.
    diffuse: vec4<f32>,
    ambient: vec4<f32>,
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
    // The material's specular colour; w its power word, 0 for no highlight.
    specular: vec4<f32>,
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
    // The vertex's two colours as the shade leaves them: the diffuse, which the texture stages
    // modulate, and the specular, added after; w of the second is what the fog keeps.
    @location(6) lit: vec3<f32>,
    @location(7) spill: vec4<f32>,
    // A painted vertex's specular: what its lit colour passes 1 by.
    @location(8) gloss: vec3<f32>,
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
    // The shade (`CShade::ShadeIndexedStrided`, Terrain.dll:0x1004df70) colours the vertex,
    // and the device carries the colour across the face, so it is formed here. The item's
    // lights: the sun's two, and the effects' point lights in the list beside them.
    var n = vec3<f32>(0.0);
    if dot(out.normal, out.normal) > 0.0 {
        // The normal as the model's frame turns it; the matrix's scale goes with the length.
        n = normalize(out.normal);
    }
    let a = shade_cosine(n, frame.light_direction.xyz);
    let b = shade_cosine(n, frame.second_direction.xyz);
    let lights = frame.light_colour.rgb * a
        + frame.second_colour.rgb * b
        + point_lights(world.xyz, out.normal, instance.owner.x);
    let scene = frame.scene_colour.rgb;
    out.gloss = vec3<f32>(0.0);
    if frame.paint.x > 0.5 {
        // The camera's mode 2 (docs/35, "The unit in the middle"): the batch's self-light is
        // the colour the camera was handed, its texture is none, and its diffuse stays its
        // own: the view's lights on the diffuse plus the self-light, held up to the scene
        // colour, through the knee, what passes 1 going to the specular.
        let painted = shade_lit(look.diffuse.rgb * lights, vec3<f32>(0.0), instance.paint.rgb, scene);
        out.paint = vec4<f32>(painted.diffuse, 1.0);
        out.gloss = painted.specular;
    }
    // The world's own draw: the lights on the material's diffuse, and their highlights on its
    // specular. A batch that takes a lightmap has had its diffuse made its self-light and its
    // diffuse and power made 0 before this (`0x1002c1a0`, docs/07, "How a lightmapped batch
    // is drawn"), so no light reaches it and the scene colour is the floor under that colour
    // alone.
    var diffuse = look.diffuse.rgb * lights;
    var highlight = vec3<f32>(0.0);
    var ambient = look.ambient.rgb;
    if look.lit.x > 0.5 {
        diffuse = vec3<f32>(0.0);
        ambient = look.diffuse.rgb;
    } else {
        let to_eye = normalize(frame.eye.xyz - world.xyz);
        // STAND-IN: docs/10-sky.md#the-lit-colour-is-the-games-own--read-and-measured -- a
        // point light's highlight is taken the same way (Ngi32.dll:0x100166b5); here the
        // sun's two alone.
        highlight = look.specular.rgb
            * (frame.light_colour.rgb * shade_highlight(n, frame.light_direction.xyz, to_eye, a, look.specular.w)
                + frame.second_colour.rgb
                    * shade_highlight(n, frame.second_direction.xyz, to_eye, b, look.specular.w));
    }
    let shaded = shade_lit(diffuse, highlight, ambient, scene);
    out.lit = shaded.diffuse;
    out.spill = vec4<f32>(
        shaded.specular,
        shade_fog(distance(world.xyz, frame.eye.xyz), frame.fog.x, frame.fog.y),
    );
    return out;
}

@fragment
fn fs_main(v: VertexOut) -> @location(0) vec4<f32> {
    if frame.fog.w > 0.5 && v.world.z < frame.fog.z {
        discard;
    }
    if frame.paint.x > 0.5 {
        // Stage 0 has no texture in mode 2, so the vertex's colour is the pixel's, and the
        // device adds the specular after it (docs/37, "The scan bands").
        return vec4<f32>(min(v.paint.rgb + v.gloss, vec3<f32>(1.0)), 1.0);
    }
    // The cell rewrites the coordinates in place: u0 + u × du, v0 + v × dv.
    let texel = textureSample(skin, skin_sampler, look.cell.xy + v.uv * look.cell.zw);
    let baked = textureSample(lightmap, skin_sampler, v.lightmap);
    // What the fog draws toward: the frame's colour, or the one the blend mode swaps in.
    let fog = mix(frame.fog_colour.rgb, look.fog.rgb, look.fog.w);
    if look.lit.x > 0.5 {
        // A lit batch (docs/07, "How a lightmapped batch is drawn"): the texture and the
        // lightmap modulate the vertex's colour -- render phase 3 -- and the specular is added.
        // Its alpha is the material's ambient alpha alone.
        let colour = min(texel.rgb * baked.rgb * v.lit + v.spill.rgb, vec3<f32>(1.0));
        return vec4<f32>(mix(fog, colour, v.spill.w), look.diffuse.a);
    }
    // The texture by the vertex's diffuse colour, the specular added, each held to 1 as the
    // device holds it, and the fog's share taken from the specular's alpha. The alpha is the
    // texture's times the material's ambient alpha.
    let colour = min(texel.rgb * v.lit + v.spill.rgb, vec3<f32>(1.0));
    // A portal quad's fade stands in for the material's ambient alpha (Terrain.dll:0x1002c63a).
    let alpha = texel.a * select(look.diffuse.a, v.fade, look.portal.w > 0.5);
    // Every blend mode but 0 alpha-tests GREATEREQUAL against ALPHAREF 1 (docs/07, "What a
    // blended batch writes"): an 8-bit alpha of 0 is dropped, and with it its depth.
    if look.lit.y > 0.5 && round(alpha * 255.0) < 1.0 {
        discard;
    }
    return vec4<f32>(mix(fog, colour, v.spill.w), alpha);
}
