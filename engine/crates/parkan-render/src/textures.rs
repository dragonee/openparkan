//! Every texture the world names, uploaded once with the file's own mips.

use parkan_world::textures::Texture;

/// The format a texture's bytes are read in by everything the scene draws: as they are stored,
/// which is how the game's device reads them (docs/10-sky.md, "The frame holds what the files
/// hold").
pub const STORED_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

pub struct GpuTextures {
    /// Each texture read with its bytes as stored.
    pub stored: Vec<wgpu::TextureView>,
    /// The same textures read through an sRGB decode: what the effect sprites' shader samples,
    /// encoding its result back.
    pub views: Vec<wgpu::TextureView>,
    /// Drawn where a material names no texture that resolves.
    pub white: wgpu::TextureView,
    /// Repeat, trilinear, anisotropic: what ground and skins tile with.
    pub sampler: wgpu::Sampler,
}

/// A texture's two views: its bytes as stored, and through an sRGB decode.
fn upload(device: &wgpu::Device, queue: &wgpu::Queue, t: &Texture) -> (wgpu::TextureView, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(&t.name),
        size: wgpu::Extent3d { width: t.width, height: t.height, depth_or_array_layers: 1 },
        mip_level_count: t.levels.len() as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: STORED_FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[STORED_FORMAT.add_srgb_suffix()],
    });
    for (level, pixels) in t.levels.iter().enumerate() {
        let (w, h) = ((t.width >> level).max(1), (t.height >> level).max(1));
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: level as u32,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            pixels,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4 * w), rows_per_image: Some(h) },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
    }
    let decoded = texture.create_view(&wgpu::TextureViewDescriptor {
        format: Some(STORED_FORMAT.add_srgb_suffix()),
        ..Default::default()
    });
    (texture.create_view(&Default::default()), decoded)
}

impl GpuTextures {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, textures: &[Texture]) -> Self {
        let white = Texture {
            name: "white".into(),
            width: 1,
            height: 1,
            levels: vec![vec![255; 4]],
            pages: Vec::new(),
        };
        let (stored, views) = textures.iter().map(|t| upload(device, queue, t)).unzip();
        Self {
            stored,
            views,
            // White is white read either way.
            white: upload(device, queue, &white).0,
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("repeat"),
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                address_mode_w: wgpu::AddressMode::Repeat,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::MipmapFilterMode::Linear,
                anisotropy_clamp: 8,
                ..Default::default()
            }),
        }
    }

    /// A texture as stored, or white where there is none.
    pub fn view(&self, index: Option<usize>) -> &wgpu::TextureView {
        index.and_then(|i| self.stored.get(i)).unwrap_or(&self.white)
    }
}
