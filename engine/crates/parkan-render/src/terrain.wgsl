// The ground: a face's material and its microtexture, and its second material over the first,
// lit and fogged a vertex at a time as the game's shade does it (shade.wgsl is prepended).

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
    eye: vec4<f32>,
    paint: vec4<f32>,
    // x: how many of the point lights light every surface in their range, which come first;
    // y: how many there are.
    point_counts: vec4<f32>,
    points: array<PointLight, 64>,
};

// What the frame's point lights give a landscape vertex: the landscape's draw item carries the
// cell's light list as a mesh's does (Terrain.dll:0x1004482a), less the owner-only lights
// (0x10047c96), and the shade's lighter lights its vertices with it -- the colour times the
// cosine at the vertex times a0 + a1 x + a2 x^2, x the share of the range left, nothing past
// the range (Ngi32.dll:0x100164f0).
fn point_lights(world: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    var sum = vec3<f32>(0.0);
    if dot(normal, normal) <= 0.0 {
        return sum;
    }
    let n = normalize(normal);
    let everyones = i32(frame.point_counts.x);
    for (var i = 0; i < everyones; i++) {
        let light = frame.points[i];
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

struct Layers {
    // The first material's diffuse; w 1 where its microtexture is drawn.
    diffuse1: vec4<f32>,
    // Its ambient, the self-light.
    ambient1: vec4<f32>,
    // The second material's diffuse; w 1 where the faces wear one.
    diffuse2: vec4<f32>,
    // Its ambient; w 1 where its microtexture is drawn.
    ambient2: vec4<f32>,
    // x the second material's ambient alpha, which scales the vertices' own.
    alpha: vec4<f32>,
    // The buildings' cut mask: its origin x and y, texels a unit, and w 1 when there is one;
    // then its width and height.
    cut: vec4<f32>,
    cut_size: vec4<f32>,
};

@group(0) @binding(0) var<uniform> frame: Frame;
@group(1) @binding(0) var<uniform> layers: Layers;
@group(1) @binding(1) var layer1: texture_2d<f32>;
@group(1) @binding(2) var layer2: texture_2d<f32>;
@group(1) @binding(3) var ground: sampler;
@group(1) @binding(4) var cuts: texture_2d<f32>;
@group(1) @binding(5) var micro1: texture_2d<f32>;
@group(1) @binding(6) var micro2: texture_2d<f32>;

// Whether a building has cut the landscape away here (docs/03, "Placing a building cuts the
// landscape").
fn cut_away(world: vec3<f32>) -> bool {
    if layers.cut.w < 0.5 {
        return false;
    }
    let t = (world.xy - layers.cut.xy) * layers.cut.z;
    if t.x < 0.0 || t.y < 0.0 || t.x >= layers.cut_size.x || t.y >= layers.cut_size.y {
        return false;
    }
    return textureLoad(cuts, vec2<i32>(t), 0).r > 0.5;
}

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv1: vec2<f32>,
    @location(3) uv2: vec2<f32>,
    @location(4) blend: f32,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) uv1: vec2<f32>,
    @location(2) uv2: vec2<f32>,
    @location(3) blend: f32,
    // Each material's two vertex colours, and what the fog keeps.
    @location(4) lit1: vec3<f32>,
    @location(5) spill1: vec3<f32>,
    @location(6) lit2: vec3<f32>,
    @location(7) spill2: vec3<f32>,
    @location(8) keep: f32,
};

@vertex
fn vs_main(v: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.clip = frame.view_proj * vec4<f32>(v.position, 1.0);
    out.world = v.position;
    out.uv1 = v.uv1;
    out.uv2 = v.uv2;
    out.blend = v.blend;
    // The landscape's item is lit (flags 0x414, `Terrain.dll:0x10044627`) with the stream's
    // normals as they are: unit, or zero on a basement corner. No ground material carries a
    // specular colour, so no highlight is taken.
    // The effects' point lights are in the item's list beside the sun's two, summed with them.
    let light = frame.light_colour.rgb * shade_cosine(v.normal, frame.light_direction.xyz)
        + frame.second_colour.rgb * shade_cosine(v.normal, frame.second_direction.xyz)
        + point_lights(v.position, v.normal);
    let scene = frame.scene_colour.rgb;
    let first = shade_lit(light * layers.diffuse1.rgb, vec3<f32>(0.0), layers.ambient1.rgb, scene);
    let second = shade_lit(light * layers.diffuse2.rgb, vec3<f32>(0.0), layers.ambient2.rgb, scene);
    out.lit1 = first.diffuse;
    out.spill1 = first.specular;
    out.lit2 = second.diffuse;
    out.spill2 = second.specular;
    out.keep = shade_fog(distance(v.position, frame.eye.xyz), frame.fog.x, frame.fog.y);
    return out;
}

// One material's surface: its texture by the vertex's diffuse colour, by its microtexture
// doubled where it has one -- render phase 9, `MODULATE2X` over the second texture stage
// (`Terrain.dll:0x1002b4c2`-`0x1002b505`) -- and the specular colour added. The device holds
// each step to 1.
fn surface(base: vec3<f32>, micro: vec3<f32>, detailed: bool, lit: vec3<f32>, spill: vec3<f32>) -> vec3<f32> {
    var colour = base * lit;
    if detailed {
        colour = min(colour * micro * 2.0, vec3<f32>(1.0));
    }
    return min(colour + spill, vec3<f32>(1.0));
}

@fragment
fn fs_main(v: VertexOut) -> @location(0) vec4<f32> {
    if (frame.fog.w > 0.5 && v.world.z < frame.fog.z) || cut_away(v.world) {
        discard;
    }
    // Both materials take stream 5's coordinates and both microtextures stream 18's: the
    // second material's item is a copy of the first's (`0x1002c02d`-`0x1002c044`).
    let base1 = textureSample(layer1, ground, v.uv1).rgb;
    let detail1 = textureSample(micro1, ground, v.uv2).rgb;
    let base2 = textureSample(layer2, ground, v.uv1).rgb;
    let detail2 = textureSample(micro2, ground, v.uv2).rgb;
    var colour = surface(base1, detail1, layers.diffuse1.w > 0.5, v.lit1, v.spill1);
    if layers.diffuse2.w > 0.5 {
        // The second material goes over the first, `SRCALPHA`/`INVSRCALPHA` on the vertex
        // alpha of stream 14 (`0x1002c0d7`-`0x1002c0f2`). Both are fogged alike, so mixing
        // them before the fog is the same as blending one fogged surface over the other.
        let over = surface(base2, detail2, layers.ambient2.w > 0.5, v.lit2, v.spill2);
        colour = mix(colour, over, clamp(v.blend * layers.alpha.x, 0.0, 1.0));
    }
    return vec4<f32>(mix(frame.fog_colour.rgb, colour, v.keep), 1.0);
}

// The water under `REFLECTION_SHIFTED` (`Terrain.dll:0x1002ca80`, render phase 10): the
// reflection texture over the water box, its lookup displaced by the environment bump map,
// times the face's lit colour.
struct Water {
    // The water box's corners, x0 y0 x1 y1.
    box_: vec4<f32>,
    // x the bump's drift, (clock ms mod EMBMBumpMove) / EMBMBumpMove; y EMBMBumpTile; z the
    // bump matrix's scale, EMBMCoeff00 and 11; w EMBMMaxVal.
    bump: vec4<f32>,
};

@group(2) @binding(0) var<uniform> water: Water;
@group(2) @binding(1) var reflection: texture_2d<f32>;
@group(2) @binding(2) var reflection_sampler: sampler;

// The 32 × 32 bump map `CShade` makes (`0x100491e0`), texel (i, j): du and dv are
// round(EMBMMaxVal × cos and sin of 4π(i/32 + j/32 − 1)), signed bytes, sampled nearest and
// tiled.
//
// STAND-IN: docs/03-terrain.md#not-established -- how far the bump displaces is not read
// from the binary: a signed byte stands for −1 to 1 at 127, as the device's convention has it.
fn bump_at(uv: vec2<f32>) -> vec2<f32> {
    let cell = floor(fract(uv) * 32.0);
    let theta = 4.0 * 3.14159 * (cell.x / 32.0 + cell.y / 32.0 - 1.0);
    return vec2<f32>(round(water.bump.w * cos(theta)), round(water.bump.w * sin(theta))) / 127.0;
}

@fragment
fn fs_water(v: VertexOut) -> @location(0) vec4<f32> {
    if cut_away(v.world) {
        discard;
    }
    let size = water.box_.zw - water.box_.xy;
    let over = vec2<f32>(1.0 - (v.world.y - water.box_.y) / size.y, 1.0 - (v.world.x - water.box_.x) / size.x);
    let bumped = over + water.bump.z * bump_at(water.bump.y * over + vec2<f32>(water.bump.x));
    // The reflection holds the frame's own stored values, and the water's vertices carry the
    // material's shade like any ground's.
    let seen = textureSample(reflection, reflection_sampler, bumped).rgb;
    let colour = min(seen * v.lit1 + v.spill1, vec3<f32>(1.0));
    return vec4<f32>(mix(frame.fog_colour.rgb, colour, v.keep), 1.0);
}
