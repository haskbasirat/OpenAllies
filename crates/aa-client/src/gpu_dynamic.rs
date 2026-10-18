//! Dynamic gameplay layer: original unit/building models registered once,
//! then drawn every frame with instancing. Skinned infantry use baked
//! animation frames in a storage buffer, selected per instance.

use super::{
    CpuScene, DEPTH, FORMAT, MaterialUniform, PipelineKey, Vertex, alpha_test, compare_function,
    upload_buffer,
};
use anyhow::{Result, bail};
use bytemuck::{Pod, Zeroable};
use std::collections::{BTreeMap, HashMap};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct DynInstance {
    pub model: [[f32; 4]; 4],
    /// RGB team colour; W > 0.5 enables it.
    pub team_color: [f32; 4],
    /// x: frame A, y: frame B, z: blend t, w: tint (+brighten / -darken).
    pub anim: [f32; 4],
}

pub type ModelHandle = u32;

/// Baked frames for one mesh: `frames * vertex_count` entries of pos+normal.
pub struct MeshFrames {
    pub vertex_count: u32,
    pub frames: u32,
    pub data: Vec<[f32; 6]>,
}

struct DynMesh {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    index_count: u32,
    material: wgpu::BindGroup,
    key: PipelineKey,
    anim: Option<wgpu::BindGroup>,
}

pub struct DynModel {
    meshes: Vec<DynMesh>,
    pub radius: f32,
    pub height: f32,
    pub animated: bool,
}

pub struct DynamicLayer {
    shader: wgpu::ShaderModule,
    static_layout: wgpu::PipelineLayout,
    anim_layout: wgpu::PipelineLayout,
    anim_group_layout: wgpu::BindGroupLayout,
    material_layout: wgpu::BindGroupLayout,
    pipelines: BTreeMap<(PipelineKey, bool), wgpu::RenderPipeline>,
    models: Vec<DynModel>,
    pub by_key: HashMap<String, ModelHandle>,
    textures: HashMap<String, wgpu::TextureView>,
    sampler: wgpu::Sampler,
    mask_sampler: wgpu::Sampler,
    fallback: wgpu::TextureView,
    instance_buffer: Option<wgpu::Buffer>,
    capacity: usize,
    batches: Vec<(ModelHandle, u32, u32)>,
    pub draw_calls: usize,
    pub uploaded_bytes: u64,
}

impl DynamicLayer {
    pub fn new(
        device: &wgpu::Device,
        camera_layout: &wgpu::BindGroupLayout,
        material_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gameplay actor shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("dynamic.wgsl").into()),
        });
        let anim_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("baked animation frames"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let static_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("actor static layout"),
            bind_group_layouts: &[camera_layout, material_layout],
            push_constant_ranges: &[],
        });
        let anim_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("actor animated layout"),
            bind_group_layouts: &[camera_layout, material_layout, &anim_group_layout],
            push_constant_ranges: &[],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("actor textures"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            anisotropy_clamp: 4,
            ..Default::default()
        });
        let mask_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("actor unused mask"),
            ..Default::default()
        });
        let white = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("actor fallback"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        Self {
            shader,
            static_layout,
            anim_layout,
            anim_group_layout,
            material_layout: material_layout.clone(),
            pipelines: BTreeMap::new(),
            models: Vec::new(),
            by_key: HashMap::new(),
            textures: HashMap::new(),
            sampler,
            mask_sampler,
            fallback: white.create_view(&Default::default()),
            instance_buffer: None,
            capacity: 0,
            batches: Vec::new(),
            draw_calls: 0,
            uploaded_bytes: 0,
        }
    }

    pub fn model(&self, h: ModelHandle) -> Option<&DynModel> {
        self.models.get(h as usize)
    }

    fn pipeline(&mut self, device: &wgpu::Device, key: PipelineKey, animated: bool) -> Result<()> {
        if self.pipelines.contains_key(&(key, animated)) {
            return Ok(());
        }
        let blend = key.blend_state()?;
        let depth_compare = compare_function(key.depth_compare)?;
        let layout = if animated {
            &self.anim_layout
        } else {
            &self.static_layout
        };
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("gameplay actor pipeline"),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: &self.shader,
                entry_point: Some(if animated { "vs_anim" } else { "vs_static" }),
                compilation_options: Default::default(),
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 3 => Float32x4, 8 => Float32x2],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<DynInstance>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32x4, 9 => Float32x4, 10 => Float32x4],
                    },
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &self.shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: FORMAT,
                    blend,
                    write_mask: wgpu::ColorWrites::RED | wgpu::ColorWrites::GREEN | wgpu::ColorWrites::BLUE,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: (!key.double_sided).then_some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH,
                depth_write_enabled: key.depth_write,
                depth_compare,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        self.pipelines.insert((key, animated), pipeline);
        Ok(())
    }

    fn texture(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, key: &str, scene: &CpuScene) -> Option<wgpu::TextureView> {
        if let Some(v) = self.textures.get(key) {
            return Some(v.clone());
        }
        let tex = scene.textures.get(key)?;
        let base = tex.mips.first()?;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(key),
            size: wgpu::Extent3d {
                width: base.width(),
                height: base.height(),
                depth_or_array_layers: 1,
            },
            mip_level_count: tex.mips.len() as u32,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        for (mip, rgba) in tex.mips.iter().enumerate() {
            self.uploaded_bytes += rgba.len() as u64;
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: mip as u32,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                rgba.as_raw(),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(rgba.width() * 4),
                    rows_per_image: Some(rgba.height()),
                },
                wgpu::Extent3d {
                    width: rgba.width(),
                    height: rgba.height(),
                    depth_or_array_layers: 1,
                },
            );
        }
        let view = texture.create_view(&Default::default());
        self.textures.insert(key.to_owned(), view.clone());
        Some(view)
    }

    /// Register a model loaded with `loading::load_model`. `frames` (one per
    /// mesh, same order) turns the model into a baked-animation actor.
    pub fn add_model(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        key: &str,
        scene: &CpuScene,
        bounds: &aa_assets::Bounds,
        frames: Option<Vec<MeshFrames>>,
    ) -> Result<ModelHandle> {
        if let Some(&h) = self.by_key.get(key) {
            return Ok(h);
        }
        if let Some(f) = &frames
            && f.len() != scene.draws.len()
        {
            bail!("Baked animation mesh count does not match {key}");
        }
        let mut frames = frames.map(|f| f.into_iter().map(Some).collect::<Vec<_>>());
        let mut meshes = Vec::new();
        for (mi, d) in scene.draws.iter().enumerate() {
            if d.indices.is_empty() || d.vertices.is_empty() {
                continue;
            }
            let key = PipelineKey::material(&d.material);
            let mesh_frames = frames.as_mut().and_then(|f| f[mi].take());
            if let Some(mf) = &mesh_frames
                && (mf.vertex_count as usize != d.vertices.len()
                    || mf.data.len() != (mf.vertex_count * mf.frames) as usize)
            {
                bail!("Baked animation vertex layout does not match {}", scene.map_id);
            }
            let animated = mesh_frames.is_some();
            self.pipeline(device, key, animated)?;
            let (threshold, alpha_mode) = alpha_test(&d.material);
            let uniform = MaterialUniform {
                color: d.material.color,
                emissive_cutoff: [
                    d.material.emissive[0],
                    d.material.emissive[1],
                    d.material.emissive[2],
                    threshold,
                ],
                render_flags: [
                    f32::from(d.material.blend),
                    f32::from(d.material.team_color_mask),
                    alpha_mode,
                    0.0,
                ],
                terrain_u: [0.0; 4],
                terrain_v: [0.0; 4],
            };
            let mat_buffer = upload_buffer(
                device,
                queue,
                "actor material",
                bytemuck::bytes_of(&uniform),
                wgpu::BufferUsages::UNIFORM,
            );
            let view = d
                .material
                .texture
                .as_ref()
                .and_then(|k| self.texture(device, queue, k, scene))
                .unwrap_or_else(|| self.fallback.clone());
            let material = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("actor material binding"),
                layout: &self.material_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: mat_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&self.fallback),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::Sampler(&self.mask_sampler),
                    },
                ],
            });
            let vertices = upload_buffer(
                device,
                queue,
                "actor vertices",
                bytemuck::cast_slice(&d.vertices),
                wgpu::BufferUsages::VERTEX,
            );
            let indices = upload_buffer(
                device,
                queue,
                "actor indices",
                bytemuck::cast_slice(&d.indices),
                wgpu::BufferUsages::INDEX,
            );
            self.uploaded_bytes += vertices.size() + indices.size();
            let anim = mesh_frames.map(|mf| {
                let storage = upload_buffer(
                    device,
                    queue,
                    "baked animation frames",
                    bytemuck::cast_slice(&mf.data),
                    wgpu::BufferUsages::STORAGE,
                );
                self.uploaded_bytes += storage.size();
                let info: [u32; 4] = [mf.vertex_count, mf.frames, 0, 0];
                let info = upload_buffer(
                    device,
                    queue,
                    "baked animation info",
                    bytemuck::cast_slice(&info),
                    wgpu::BufferUsages::UNIFORM,
                );
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("baked animation binding"),
                    layout: &self.anim_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: storage.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: info.as_entire_binding(),
                        },
                    ],
                })
            });
            meshes.push(DynMesh {
                vertices,
                indices,
                index_count: d.indices.len() as u32,
                material,
                key,
                anim,
            });
        }
        let animated = meshes.iter().any(|m| m.anim.is_some());
        let size = bounds.size;
        let h = self.models.len() as ModelHandle;
        self.models.push(DynModel {
            meshes,
            radius: (size[0].max(size[2]) * 0.5).max(0.5),
            height: size[1].max(0.5),
            animated,
        });
        self.by_key.insert(key.to_owned(), h);
        Ok(h)
    }

    /// Replace this frame's instances. Sorted by model so each mesh is one draw.
    pub fn set_instances(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        instances: &mut [(ModelHandle, DynInstance)],
    ) {
        instances.sort_by_key(|i| i.0);
        self.batches.clear();
        let mut data: Vec<DynInstance> = Vec::with_capacity(instances.len());
        for (h, inst) in instances.iter() {
            if *h as usize >= self.models.len() {
                continue;
            }
            match self.batches.last_mut() {
                Some((m, _, count)) if *m == *h => *count += 1,
                _ => self.batches.push((*h, data.len() as u32, 1)),
            }
            data.push(*inst);
        }
        if data.is_empty() {
            return;
        }
        if data.len() > self.capacity {
            let capacity = data.len().next_power_of_two().max(256);
            self.instance_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("actor instances"),
                size: (capacity * std::mem::size_of::<DynInstance>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.capacity = capacity;
        }
        if let Some(buffer) = &self.instance_buffer {
            queue.write_buffer(buffer, 0, bytemuck::cast_slice(&data));
        }
    }

    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>, camera: &wgpu::BindGroup) {
        let Some(instances) = &self.instance_buffer else { return };
        if self.batches.is_empty() {
            return;
        }
        pass.set_bind_group(0, camera, &[]);
        pass.set_vertex_buffer(1, instances.slice(..));
        // Opaque/cutout first, then blended meshes.
        for blended in [false, true] {
            for &(model, start, count) in &self.batches {
                let Some(m) = self.models.get(model as usize) else { continue };
                for mesh in &m.meshes {
                    if mesh.key.blend != blended {
                        continue;
                    }
                    let Some(pipeline) = self.pipelines.get(&(mesh.key, mesh.anim.is_some())) else {
                        continue;
                    };
                    pass.set_pipeline(pipeline);
                    pass.set_bind_group(1, &mesh.material, &[]);
                    if let Some(anim) = &mesh.anim {
                        pass.set_bind_group(2, anim, &[]);
                    }
                    pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                    pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..mesh.index_count, 0, start..start + count);
                }
            }
        }
    }
}
