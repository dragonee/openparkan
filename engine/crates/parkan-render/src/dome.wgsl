// The sky's cap: the nebula on it, the gradient over that, and the clouds on a copy of it
// lowered below the camera. One pipeline draws all three; the uniform says which. Every
// colour is the stored value, as the frame holds it.

struct Layer {
    view_proj: mat4x4<f32>,
    // The clouds: the fog colour their own fog draws toward. Unused by the other layers.
    tint: vec4<f32>,
    uv_scale: f32,
    // 0 the gradient, 1 a texture as it is, 2 the gradient held opaque, 3 the clouds.
    mode: u32,
};

@group(0) @binding(0) var<uniform> layer: Layer;
@group(1) @binding(0) var skin: texture_2d<f32>;
@group(1) @binding(1) var skin_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    // The vertex's diffuse colour: the gradient's own, or the clouds' lit colour.
    @location(2) colour: vec4<f32>,
    // The clouds' specular colour, and in w what their fog keeps.
    @location(3) spill: vec4<f32>,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) colour: vec4<f32>,
    @location(2) spill: vec4<f32>,
};

@vertex
fn vs_main(v: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.clip = layer.view_proj * vec4<f32>(v.position, 1.0);
    out.uv = v.uv * layer.uv_scale;
    out.colour = v.colour;
    out.spill = v.spill;
    return out;
}

@fragment
fn fs_main(v: VertexOut) -> @location(0) vec4<f32> {
    if layer.mode == 0u {
        return v.colour;
    }
    if layer.mode == 2u {
        return vec4<f32>(v.colour.rgb, 1.0);
    }
    let texel = textureSample(skin, skin_sampler, v.uv);
    if layer.mode == 3u {
        // The clouds are a lit item (flags 0x14, `Terrain.dll:0x1007aaf1`): the texture by the
        // vertex's diffuse colour, its specular added, and the layer's own fog taken from the
        // specular's alpha, toward the frame's fog colour. The alpha is the texture's by the
        // material's ambient alpha, which the vertex colour carries.
        let lit = min(texel.rgb * v.colour.rgb + v.spill.rgb, vec3<f32>(1.0));
        return vec4<f32>(mix(layer.tint.rgb, lit, v.spill.w), texel.a * v.colour.a);
    }
    // The nebula's vertices carry a constant white (`0x100792f5`) and a specular of
    // 0xff000000 (`0x100792bc`): its texture as it is, and no fog.
    return texel;
}
