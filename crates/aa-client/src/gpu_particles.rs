//! Camera-facing billboard particles and effect sprite quads (smoke, fire,
//! sparks, muzzle flashes, tracers) drawn after the actor layer. Contract:
//! `ParticleLayer::new(device, queue, camera_layout, textures_root)`,
//! `set(device, queue, &[ParticleInstance])` each frame (one buffer write),
//! `draw(pass, camera_binding)` inside the scene render pass.
//!
//! Textures: every original effect texture used by the VFX player
//! (`crate::vfx::atlas_sources`) is resampled to one layer of a 128x128
//! texture array (with mips). Blending is premultiplied alpha, so
//! alpha-blended and additive particles share one pipeline and draw call;
//! depth test on, depth write off; fog of war darkening as `dynamic.wgsl`.

#![allow(dead_code)]

use crate::vfx::{AtlasSource, Procedural};
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct ParticleInstance {
    /// Render-space centre / anchor / streak head (metres).
    pub position: [f32; 3],
    /// Billboard half-size or streak half-width (metres).
    pub size: f32,
    /// RGBA tint (straight alpha; the shader premultiplies).
    pub color: [f32; 4],
    /// x: texture layer, y: rotation radians, z: blend (0 alpha, 1 additive,
    /// 2 additive one/one), w: mode (see `MODE_*`).
    pub params: [f32; 4],
    /// Quad modes: corner offsets xyz + u for corners 00, 10, 01, 11
    /// (triangles 00-10-01 and 10-11-01). Streak: corners[0].xyz = tail offset.
    pub corners: [[f32; 4]; 4],
    /// v texture coordinate of the four corners.
    pub corner_v: [f32; 4],
}

/// Camera-facing square particle (position, size, rotation).
pub const MODE_BILLBOARD: f32 = 0.0;
/// World-space quad: corners are render-space offsets from `position`.
pub const MODE_QUAD: f32 = 1.0;
/// Camera-facing streak from `position` back to `position + corners[0]`.
pub const MODE_STREAK: f32 = 2.0;
/// Billboard node quad: corners in (camera right, camera up, toward camera).
pub const MODE_FACE_CAMERA: f32 = 3.0;
/// Billboard node quad rotating about world up: (right, up, toward camera).
pub const MODE_FACE_UP: f32 = 4.0;

pub const LAYER_SIZE: u32 = 128;

pub struct ParticleLayer {
    pipeline: wgpu::RenderPipeline,
    atlas: wgpu::BindGroup,
    buffer: Option<wgpu::Buffer>,
    capacity: usize,
    count: u32,
    pub layers: u32,
}

impl ParticleLayer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        camera_layout: &wgpu::BindGroupLayout,
        textures_root: &std::path::Path,
    ) -> Self {
        let sources = crate::vfx::atlas_sources();
        let max_layers = device.limits().max_texture_array_layers.max(1) as usize;
        let layers = sources.len().clamp(1, max_layers) as u32;
        let mip_count = LAYER_SIZE.ilog2() + 1;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("effect texture array"),
            size: wgpu::Extent3d {
                width: LAYER_SIZE,
                height: LAYER_SIZE,
                depth_or_array_layers: layers,
            },
            mip_level_count: mip_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        for (layer, source) in sources.iter().take(layers as usize).enumerate() {
            let image = load_layer(source, textures_root);
            let mut level = image;
            for mip in 0..mip_count {
                let size = (LAYER_SIZE >> mip).max(1);
                if level.width() != size {
                    level = image::imageops::resize(
                        &level,
                        size,
                        size,
                        image::imageops::FilterType::Triangle,
                    );
                }
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &texture,
                        mip_level: mip,
                        origin: wgpu::Origin3d {
                            x: 0,
                            y: 0,
                            z: layer as u32,
                        },
                        aspect: wgpu::TextureAspect::All,
                    },
                    level.as_raw(),
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(size * 4),
                        rows_per_image: Some(size),
                    },
                    wgpu::Extent3d {
                        width: size,
                        height: size,
                        depth_or_array_layers: 1,
                    },
                );
            }
        }
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("effect texture array view"),
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("effect textures"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let atlas_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("effect texture layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let atlas = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("effect texture binding"),
            layout: &atlas_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("effect particle shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("particles.wgsl").into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("effect particle layout"),
            bind_group_layouts: &[camera_layout, &atlas_layout],
            push_constant_ranges: &[],
        });
        let premultiplied = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent::OVER,
        };
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("effect particles"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<ParticleInstance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x3, 1 => Float32, 2 => Float32x4, 3 => Float32x4,
                        4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32x4,
                        8 => Float32x4
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: Some(premultiplied),
                    write_mask: wgpu::ColorWrites::RED
                        | wgpu::ColorWrites::GREEN
                        | wgpu::ColorWrites::BLUE,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        Self {
            pipeline,
            atlas,
            buffer: None,
            capacity: 0,
            count: 0,
            layers,
        }
    }

    /// Replace this frame's particles (one queue write).
    pub fn set(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, particles: &[ParticleInstance]) {
        self.count = particles.len() as u32;
        if particles.is_empty() {
            return;
        }
        if particles.len() > self.capacity {
            let capacity = particles.len().next_power_of_two().max(1024);
            self.buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("effect particle instances"),
                size: (capacity * std::mem::size_of::<ParticleInstance>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.capacity = capacity;
        }
        if let Some(buffer) = &self.buffer {
            queue.write_buffer(buffer, 0, bytemuck::cast_slice(particles));
        }
    }

    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>, camera: &wgpu::BindGroup) {
        let Some(buffer) = &self.buffer else { return };
        if self.count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, camera, &[]);
        pass.set_bind_group(1, &self.atlas, &[]);
        pass.set_vertex_buffer(0, buffer.slice(..));
        pass.draw(0..6, 0..self.count);
    }
}

fn load_layer(source: &AtlasSource, root: &std::path::Path) -> image::RgbaImage {
    let decoded = match source {
        AtlasSource::Converted(rel) => {
            image::open(root.join("assets/converted/textures").join(rel)).ok()
        }
        AtlasSource::Original(rel) => image::open(root.join("assets/original/Data").join(rel)).ok(),
        AtlasSource::Embedded(bytes) => image::load_from_memory(bytes).ok(),
        AtlasSource::Procedural(kind) => return procedural(*kind),
    };
    match decoded {
        Some(img) => {
            let rgba = img.to_rgba8();
            if rgba.width() == LAYER_SIZE && rgba.height() == LAYER_SIZE {
                rgba
            } else {
                image::imageops::resize(
                    &rgba,
                    LAYER_SIZE,
                    LAYER_SIZE,
                    image::imageops::FilterType::CatmullRom,
                )
            }
        }
        None => {
            log::warn!("Effect texture unavailable: {source:?}");
            image::RgbaImage::from_pixel(LAYER_SIZE, LAYER_SIZE, image::Rgba([255, 255, 255, 0]))
        }
    }
}

fn procedural(kind: Procedural) -> image::RgbaImage {
    let n = LAYER_SIZE;
    image::RgbaImage::from_fn(n, n, |x, y| {
        let u = (x as f32 + 0.5) / n as f32;
        let v = (y as f32 + 0.5) / n as f32;
        let a = match kind {
            Procedural::White => 1.0,
            // tracer: bright core across v, fading toward the tail (u = 0)
            Procedural::Streak => {
                let across = (1.0 - ((v - 0.5).abs() * 2.0)).clamp(0.0, 1.0);
                let core = across.powf(1.6);
                let along = u.powf(0.7) * (1.0 - ((u - 0.97).max(0.0) / 0.03)).clamp(0.0, 1.0);
                core * along
            }
            Procedural::Glow => {
                let d = ((u - 0.5).powi(2) + (v - 0.5).powi(2)).sqrt() * 2.0;
                (1.0 - d).clamp(0.0, 1.0).powi(2)
            }
        };
        image::Rgba([255, 255, 255, (a * 255.0).round() as u8])
    })
}
