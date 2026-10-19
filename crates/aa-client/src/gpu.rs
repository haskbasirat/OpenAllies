use crate::camera::Camera;
use anyhow::{Context, Result, bail};
use bytemuck::{Pod, Zeroable};
use eframe::egui;
use glam::{Mat4, Vec3};
use std::collections::BTreeMap;
use std::sync::{Arc,Weak,atomic::{AtomicU64,Ordering}};
#[path="view_renderer_device.rs"]
pub mod camera_device;
#[path="native_texture_gpu.rs"]
pub mod native_texture;

///Nonowning certificate of the exact GPU renderer allocation and uploaded
///scene. Reload changes the sole epoch; dropping the renderer expires it.
#[derive(Clone)]
pub(crate) struct RendererWitness {owner:Weak<AtomicU64>,epoch:u64}
impl RendererWitness {
    pub(crate) fn is_current(&self)->bool {
        self.epoch!=0&&self.owner.upgrade().is_some_and(|v|v.load(Ordering::Acquire)==self.epoch)
    }
    pub(crate) fn is_for(&self,renderer:&SceneRenderer)->bool {
        self.is_current()&&Weak::ptr_eq(&self.owner,&Arc::downgrade(&renderer.scene_epoch))
    }
}
#[cfg(test)]
mod renderer_witness_tests {
    use super::*;
    #[test]
    fn exact_renderer_epoch_expires_on_reload_and_drop() {
        let owner=Arc::new(AtomicU64::new(0));
        let zero=RendererWitness {owner:Arc::downgrade(&owner),epoch:0};
        assert!(!zero.is_current());
        owner.store(7,Ordering::Release);
        let current=RendererWitness {owner:Arc::downgrade(&owner),epoch:7};
        assert!(current.is_current());
        let foreign=Arc::new(AtomicU64::new(7));
        assert!(!Weak::ptr_eq(&current.owner,&Arc::downgrade(&foreign)));
        owner.store(8,Ordering::Release);assert!(!current.is_current());
        let next=RendererWitness {owner:Arc::downgrade(&owner),epoch:8};
        assert!(next.is_current());drop(owner);assert!(!next.is_current());
        fn send<T:Send>(){}send::<RendererWitness>();
    }
}

#[path = "transparency.rs"]
mod transparency;
#[path = "gpu_dynamic.rs"]
pub mod dynamic;
#[path = "gpu_particles.rs"]
pub mod particles;

/// Handles wgpu-reported errors on the native backend. This does not promise
/// recovery from driver crashes, device removal or process-level allocation failure.
/// Keep the scopes local to the render thread; finish all three before committing.
struct GpuScope {
    device: wgpu::Device,
    open: bool,
}
impl GpuScope {
    fn begin(device: &wgpu::Device) -> Self {
        device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        device.push_error_scope(wgpu::ErrorFilter::Internal);
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        Self {
            device: device.clone(),
            open: true,
        }
    }
    fn finish(mut self, operation: &str) -> Result<()> {
        let errors = self.drain();
        if !errors.is_empty() {
            bail!("GPU {operation}: {}", errors.join("; "));
        }
        Ok(())
    }
    fn drain(&mut self) -> Vec<String> {
        if !self.open {
            return Vec::new();
        }
        self.open = false;
        (0..3)
            .filter_map(|_| {
                pollster::block_on(self.device.pop_error_scope()).map(|e| e.to_string())
            })
            .collect()
    }
}
impl Drop for GpuScope {
    fn drop(&mut self) {
        for error in self.drain() {
            log::warn!("GPU scope cleanup: {error}");
        }
    }
}

/// Avoid mapped-at-creation helpers: mapping an invalid buffer after a scoped
/// allocation error can itself panic before the caller can inspect its scope.
fn upload_buffer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    contents: &[u8],
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: contents.len() as u64,
        usage: usage | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    if !contents.is_empty() {
        queue.write_buffer(&buffer, 0, contents);
    }
    buffer
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
    /// Cell-local native alpha UV. Zero for models; independent of diffuse UV.
    pub terrain_uv: [f32; 2],
}
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Instance {
    pub model: [[f32; 4]; 4],
    /// RGB replacement color; W=0 retains the inspector's global override.
    pub team_color: [f32; 4],
}
impl Instance {
    pub fn identity() -> Self {
        Self {
            model: Mat4::IDENTITY.to_cols_array_2d(),
            team_color: [0.0; 4],
        }
    }
    pub fn at(position: [f32; 3], yaw: f32, scale: f32) -> Self {
        Self {
            model: Mat4::from_scale_rotation_translation(
                Vec3::splat(scale),
                glam::Quat::from_rotation_y(yaw),
                Vec3::from(position),
            )
            .to_cols_array_2d(),
            team_color: [0.0; 4],
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct InstanceSlot {
    pub draw: usize,
    pub instance: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct InstanceUpdate {
    pub slot: InstanceSlot,
    pub value: Instance,
    pub visible: bool,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InstanceUpdateStats {
    pub changed: usize,
    pub uploaded_bytes: u64,
    pub order_invalidated: bool,
}
#[derive(Clone)]
pub struct CpuMaterial {
    pub texture: Option<String>,
    pub color: [f32; 4],
    pub emissive: [f32; 3],
    pub cutoff: f32,
    pub blend: bool,
    pub double_sided: bool,
    pub team_color_mask: bool,
    pub native_render_state: Option<aa_assets::NativeRenderState>,
    pub terrain_layer: bool,
    pub terrain_mask: Option<crate::terrain_render::TerrainMaskMaterial>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct PipelineKey {
    blend: bool,
    source: u8,
    destination: u8,
    double_sided: bool,
    depth_write: bool,
    depth_compare: u8,
}
impl PipelineKey {
    fn material(material: &CpuMaterial) -> Self {
        let state = material.native_render_state;
        Self {
            blend: material.blend,
            source: state.map_or(6, |s| ((s.alpha_flags >> 1) & 15) as u8),
            destination: state.map_or(7, |s| ((s.alpha_flags >> 5) & 15) as u8),
            double_sided: material.double_sided,
            depth_write: state.map_or(!material.blend, |s| s.zbuffer_flags & 3 == 3),
            depth_compare: state.map_or(3, |s| {
                if s.zbuffer_flags & 1 == 0 {
                    0
                } else {
                    s.zbuffer_function
                }
            }),
        }
    }
    fn blend_state(self) -> Result<Option<wgpu::BlendState>> {
        if !self.blend {
            return Ok(None);
        }
        // No retained original mesh uses destination-alpha-dependent factors.
        // The development viewport keeps alpha opaque for egui, so these must
        // be implemented with a separate internal-alpha target before use.
        if self.source >= 8 || self.destination >= 8 {
            bail!("Destination-alpha-dependent blending needs an internal alpha target");
        }
        let component = wgpu::BlendComponent {
            src_factor: blend_factor(self.source)?,
            dst_factor: blend_factor(self.destination)?,
            operation: wgpu::BlendOperation::Add,
        };
        Ok(Some(wgpu::BlendState {
            color: component,
            alpha: wgpu::BlendComponent::OVER,
        }))
    }
}

fn blend_factor(value: u8) -> Result<wgpu::BlendFactor> {
    use wgpu::BlendFactor as B;
    Ok(match value {
        0 => B::One,
        1 => B::Zero,
        2 => B::Src,
        3 => B::OneMinusSrc,
        4 => B::Dst,
        5 => B::OneMinusDst,
        6 => B::SrcAlpha,
        7 => B::OneMinusSrcAlpha,
        8 => B::DstAlpha,
        9 => B::OneMinusDstAlpha,
        10 => B::SrcAlphaSaturated,
        _ => bail!("Unsupported native blend factor {value}"),
    })
}
fn compare_function(value: u8) -> Result<wgpu::CompareFunction> {
    use wgpu::CompareFunction as C;
    Ok(match value {
        0 => C::Always,
        1 => C::Less,
        2 => C::Equal,
        3 => C::LessEqual,
        4 => C::Greater,
        5 => C::NotEqual,
        6 => C::GreaterEqual,
        7 => C::Never,
        _ => bail!("Unsupported native depth function {value}"),
    })
}

/// Zero disables alpha testing; 1..8 are the authored NIF compare code + 1.
fn alpha_test(material: &CpuMaterial) -> (f32, f32) {
    if material.team_color_mask {
        return (0.0, 0.0);
    }
    if let Some(state) = material.native_render_state {
        return (
            f32::from(state.alpha_threshold) / 255.0,
            if state.alpha_flags & 512 == 0 {
                0.0
            } else {
                f32::from(((state.alpha_flags >> 10) & 7) + 1)
            },
        );
    }
    (
        material.cutoff,
        if material.cutoff > 0.0 { 7.0 } else { 0.0 },
    )
}
pub struct CpuDraw {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub instances: Vec<Instance>,
    pub material: CpuMaterial,
    pub source_order: Option<Vec<[usize; 2]>>,
    pub sort_center: Option<[f32; 3]>,
}
#[derive(Clone)]
pub struct CpuTexture {
    pub mips: Vec<image::RgbaImage>,
}
pub struct CpuScene {
    pub objects: Option<crate::inspection::SceneObjects>,
    pub live: Option<crate::live_scene::LiveScene>,
    pub name: String,
    pub map_id: String,
    pub size: [f32; 2],
    pub draws: Vec<CpuDraw>,
    pub textures: BTreeMap<String, CpuTexture>,
    pub placements: usize,
    pub rendered_instances: usize,
    pub warnings: Vec<String>,
    pub load_seconds: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CameraUniform {
    view_projection: [[f32; 4]; 4],
    eye: [f32; 4],
    light: [f32; 4],
    team_color: [f32; 4],
    /// Native terrain fog (crate::view_fog_presentation): x TerrainMap+A0
    /// world units per block, y +12C pixels per block; z stays 0 (the
    /// skinned/particle shaders test it: the original applies the fog
    /// texture effect only under the TerrainBlock nodes); w: enabled.
    fog: [f32; 4],
    model_root:[[f32;4];4],
    root_inverse:[[f32;4];4],
    root_normal:[[f32;4];4],
    light_ambient:[f32;4],
    light_diffuse:[f32;4],
    light_specular:[f32;4],
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct MaterialUniform {
    color: [f32; 4],
    emissive_cutoff: [f32; 4],
    render_flags: [f32; 4],
    terrain_u: [f32; 4],
    terrain_v: [f32; 4],
}
struct Draw {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    instances: wgpu::Buffer,
    instance_data: Vec<Instance>,
    visible: Vec<bool>,
    index_count: u32,
    instance_count: u32,
    material: wgpu::BindGroup,
    terrain_layer: bool,
    pipeline: PipelineKey,
}
#[derive(Clone)]
struct TransparentInstance {
    draw: usize,
    instance: u32,
    center: [f32; 3],
    local_center: [f32; 3],
    model: Mat4,
    dynamic_bound: bool,
}
type DrawCommand = (usize, std::ops::Range<u32>);

fn vertex_center(vertices: &[Vertex]) -> [f32; 3] {
    let mut minimum = Vec3::splat(f32::INFINITY);
    let mut maximum = Vec3::splat(f32::NEG_INFINITY);
    for vertex in vertices {
        minimum = minimum.min(Vec3::from(vertex.position));
        maximum = maximum.max(Vec3::from(vertex.position));
    }
    (minimum * 0.5 + maximum * 0.5).to_array()
}

fn append_command(commands: &mut Vec<DrawCommand>, draw: usize, instance: u32) {
    if let Some((previous_draw, range)) = commands.last_mut()
        && *previous_draw == draw
        && range.end == instance
    {
        range.end += 1;
    } else {
        commands.push((draw, instance..instance + 1));
    }
}
struct PreparedUpload {
    draws: Vec<Draw>,
    pipelines: BTreeMap<PipelineKey, wgpu::RenderPipeline>,
    immediate_commands: Vec<DrawCommand>,
    transparent_instances: Vec<TransparentInstance>,
    bytes: u64,
    triangles: u64,
}

pub struct SceneRenderer {
    state: egui_wgpu::RenderState,
    camera_buffer: wgpu::Buffer,
    camera_binding: wgpu::BindGroup,
    camera_layout: wgpu::BindGroupLayout,
    fog_sampler: wgpu::Sampler,
    fog_texture: wgpu::Texture,
    fog_size: [u32; 2],
    fog_params: [f32; 4],
    fog_revision: Option<u64>,
    pub dynamic: Option<dynamic::DynamicLayer>,
    pub particles: Option<particles::ParticleLayer>,
    pub clear_color: [f64; 3],
    material_layout: wgpu::BindGroupLayout,
    shader: wgpu::ShaderModule,
    pipeline_layout: wgpu::PipelineLayout,
    pipelines: BTreeMap<PipelineKey, wgpu::RenderPipeline>,
    draws: Vec<Draw>,
    immediate_commands: Vec<DrawCommand>,
    transparent_instances: Vec<TransparentInstance>,
    transparent_slots: BTreeMap<InstanceSlot, usize>,
    scene_epoch: Arc<AtomicU64>,
    native_scene:Option<crate::live_scene::scene_backend::Installed>,
    model_policy:crate::live_scene::model_policy::BackendState,
    screen_manager:crate::view_screen_state::State,
    cached_commands: Vec<DrawCommand>,
    cached_bundle: Option<wgpu::RenderBundle>,
    cached_direction: Option<[u32; 3]>,
    pub ordering_rebuilds: u64,
    target: Option<Target>,
    pub uploaded_bytes: u64,
    pub triangles: u64,
    pub draw_calls: usize,
    pub adapter_name: String,
    pub team_color: [f32; 3],
}
struct Target {
    identity:Arc<()>,
    color: wgpu::Texture,
    view: wgpu::TextureView,
    _depth: wgpu::Texture,
    depth_view: wgpu::TextureView,
    egui_id: egui::TextureId,
    size: [u32; 2],
}
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

impl SceneRenderer {
    pub fn model_variant_capabilities(&self)->crate::live_scene::model_policy::Capabilities{
        self.model_policy.capability(&self.scene_epoch)
    }
    pub(crate) fn screen_manager_mut(&mut self)->&mut crate::view_screen_state::State{&mut self.screen_manager}
    pub fn new(state: egui_wgpu::RenderState) -> Result<Self> {
        let device = &state.device;
        let scope = GpuScope::begin(device);
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("scene camera layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let fog_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("fog of war"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let fog_texture = Self::make_fog_texture(device, &state.queue, [1, 1], &[255]);
        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scene camera"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_binding = Self::make_camera_binding(
            device,
            &camera_layout,
            &camera_buffer,
            &fog_texture,
            &fog_sampler,
        );
        let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("original asset material"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
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
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("original asset preview shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("scene.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scene pipeline layout"),
            bind_group_layouts: &[&camera_layout, &material_layout],
            push_constant_ranges: &[],
        });
        let adapter_name = format!(
            "{} / {:?}",
            state.adapter.get_info().name,
            state.adapter.get_info().backend
        );
        let model_policy=crate::live_scene::model_policy::BackendState::from_actual_adapter(state.adapter.get_info()).map_err(anyhow::Error::msg)?;
        scope.finish("initialization")?;
        Ok(Self {
            state,
            camera_buffer,
            camera_binding,
            camera_layout,
            fog_sampler,
            fog_texture,
            fog_size: [1, 1],
            fog_params: [0.0; 4],
            fog_revision: None,
            dynamic: None,
            particles: None,
            clear_color: [0.065, 0.085, 0.08],
            material_layout,
            shader,
            pipeline_layout,
            pipelines: BTreeMap::new(),
            draws: Vec::new(),
            immediate_commands: Vec::new(),
            transparent_instances: Vec::new(),
            transparent_slots: BTreeMap::new(),
            scene_epoch: Arc::new(AtomicU64::new(0)),
            native_scene:None,
            model_policy,
            screen_manager:crate::view_screen_state::State::from_constructor_fields(),
            cached_commands: Vec::new(),
            cached_bundle: None,
            cached_direction: None,
            ordering_rebuilds: 0,
            target: None,
            uploaded_bytes: 0,
            triangles: 0,
            draw_calls: 0,
            adapter_name,
            team_color: [1.0; 3],
        })
    }

    fn pipeline(&self, key: PipelineKey) -> Result<wgpu::RenderPipeline> {
        let blend = key.blend_state()?;
        let depth_compare = compare_function(key.depth_compare)?;
        Ok(self.state.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("source-authored scene pipeline"), layout: Some(&self.pipeline_layout),
            vertex: wgpu::VertexState { module: &self.shader, entry_point: Some("vs_main"),
                compilation_options: Default::default(), buffers: &[
                    wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<Vertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 3 => Float32x4, 8 => Float32x2] },
                    wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<Instance>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32x4, 9 => Float32x4] },
                ],
            },
            fragment: Some(wgpu::FragmentState { module: &self.shader, entry_point: Some("fs_main"),
                compilation_options: Default::default(), targets: &[Some(wgpu::ColorTargetState {
                    // The viewport itself is opaque; texture alpha still supplies
                    // the authored color blend factors and the alpha test.
                    format: FORMAT, blend, write_mask: wgpu::ColorWrites::RED | wgpu::ColorWrites::GREEN | wgpu::ColorWrites::BLUE,
                })],
            }),
            primitive: wgpu::PrimitiveState { cull_mode: (!key.double_sided).then_some(wgpu::Face::Back), ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState { format: DEPTH, depth_write_enabled: key.depth_write,
                depth_compare, stencil: Default::default(), bias: Default::default() }),
            multisample: Default::default(), multiview: None, cache: None,
        }))
    }

    fn make_fog_texture(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: [u32; 2],
        data: &[u8],
    ) -> wgpu::Texture {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("fog of war"),
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        Self::write_fog(queue, &texture, size, data);
        texture
    }
    fn write_fog(queue: &wgpu::Queue, texture: &wgpu::Texture, size: [u32; 2], data: &[u8]) {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(size[0]),
                rows_per_image: Some(size[1]),
            },
            wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
        );
    }
    fn make_camera_binding(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        buffer: &wgpu::Buffer,
        fog: &wgpu::Texture,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        let view = fog.create_view(&Default::default());
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scene camera binding"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    }

    /// Native terrain fog of war: the map-wide image of the per-TerrainBlock
    /// 0x71C290 pixel grids ([`aa_runtime::view_terrain_fog::TerrainFog::compose`]).
    /// scene.wgsl applies it to TerrainBlock geometry with the 0x720AD0
    /// texture-effect projection; `None` disables it. The texture is written
    /// only when the image revision changes.
    pub fn set_fog(&mut self, fog: Option<&aa_runtime::view_terrain_fog::FogImage>) -> Result<()> {
        // No block has a +124 grid yet: the original draws no TerrainBlock.
        let Some(fog) = fog.filter(|f| f.pixels_per_block != 0) else {
            self.fog_params = [0.0; 4];
            self.fog_revision = None;
            return Ok(());
        };
        anyhow::ensure!(fog.pixels_per_block >= 2 && fog.block_size_a0 > 0, "terrain fog image has no block grid");
        anyhow::ensure!(
            fog.luminance.len() as u64 == u64::from(fog.size[0]) * u64::from(fog.size[1]),
            "terrain fog image size differs from its pixels"
        );
        let limit = self.state.device.limits().max_texture_dimension_2d;
        anyhow::ensure!(fog.size[0] <= limit && fog.size[1] <= limit, "terrain fog image exceeds the device texture size");
        if fog.size != self.fog_size {
            let device = &self.state.device;
            self.fog_texture = Self::make_fog_texture(device, &self.state.queue, fog.size, &fog.luminance);
            self.fog_size = fog.size;
            self.camera_binding = Self::make_camera_binding(
                device,
                &self.camera_layout,
                &self.camera_buffer,
                &self.fog_texture,
                &self.fog_sampler,
            );
            // The cached bundle captured the old camera binding.
            self.cached_bundle = None;
            self.cached_direction = None;
        } else if self.fog_revision != Some(fog.revision) {
            Self::write_fog(&self.state.queue, &self.fog_texture, fog.size, &fog.luminance);
        }
        self.fog_revision = Some(fog.revision);
        self.fog_params = [fog.block_size_a0 as f32, fog.pixels_per_block as f32, 0.0, 1.0];
        Ok(())
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.state.device
    }
    pub fn queue(&self) -> &wgpu::Queue {
        &self.state.queue
    }

    /// Create the billboard particle layer (original effect textures).
    pub fn enable_particles(&mut self, textures_root: &std::path::Path) {
        if self.particles.is_none() {
            self.particles = Some(particles::ParticleLayer::new(
                &self.state.device,
                &self.state.queue,
                &self.camera_layout,
                textures_root,
            ));
        }
    }

    /// Create the dynamic (gameplay actor) layer bound to this renderer's layouts.
    pub fn enable_dynamic(&mut self) {
        if self.dynamic.is_none() {
            self.dynamic = Some(dynamic::DynamicLayer::new(
                &self.state.device,
                &self.camera_layout,
                &self.material_layout,
            ));
        }
    }

    pub fn upload(&mut self, scene: &CpuScene) -> Result<()> {
        let next_epoch = self
            .scene_epoch()
            .checked_add(1)
            .context("Renderer scene epoch overflow")?;
        let prepared = self.prepare_upload(scene)?;
        self.draws = prepared.draws;
        self.pipelines = prepared.pipelines;
        self.immediate_commands = prepared.immediate_commands;
        self.transparent_instances = prepared.transparent_instances;
        self.refresh_transparent_slots();
        self.scene_epoch.store(next_epoch,Ordering::Release);
        self.native_scene=None;
        self.cached_direction = None;
        self.cached_bundle = None;
        self.uploaded_bytes = prepared.bytes;
        self.triangles = prepared.triangles;
        self.draw_calls = self.draws.len();
        Ok(())
    }

    /// Stage only the new resources against this renderer's layouts. Existing
    /// scene buffers are never rewritten by preparation or append publication.
    fn prepare_upload(&self, scene: &CpuScene) -> Result<PreparedUpload> {
        let device = &self.state.device;
        let queue = &self.state.queue;
        let limits = device.limits();
        // Validate every GPU allocation before replacing the currently viewable scene.
        // These are queried device capabilities, not fixed game RAM/VRAM allowances.
        for d in &scene.draws {
            if d.indices.len() % 3 != 0 || d.indices.iter().any(|&i| i as usize >= d.vertices.len())
            {
                bail!("Mesh has invalid triangle indices");
            }
            if d.vertices.iter().any(|v| {
                v.position
                    .iter()
                    .chain(&v.normal)
                    .chain(&v.uv)
                    .chain(&v.color)
                    .chain(&v.terrain_uv)
                    .any(|x| !x.is_finite())
            }) || d.instances.iter().any(|v| {
                v.model
                    .iter()
                    .flatten()
                    .chain(&v.team_color)
                    .any(|x| !x.is_finite())
            }) {
                bail!("Mesh has nonfinite vertex or instance data");
            }
            if d.source_order
                .as_ref()
                .is_some_and(|order| order.len() != d.instances.len())
            {
                bail!("Source traversal order does not match draw instance count");
            }
            for size in [
                d.vertices.len().checked_mul(std::mem::size_of::<Vertex>()),
                d.indices.len().checked_mul(4),
                d.instances
                    .len()
                    .checked_mul(std::mem::size_of::<Instance>()),
            ] {
                if size.context("GPU buffer size overflow")? as u64 > limits.max_buffer_size {
                    bail!("This asset exceeds the selected GPU's maximum single-buffer size");
                }
            }
            u32::try_from(d.indices.len())
                .context("Index count exceeds one GPU draw; asset must be partitioned")?;
            u32::try_from(d.instances.len())
                .context("Instance count exceeds one GPU draw; asset must be partitioned")?;
        }
        for t in scene.textures.values() {
            let first = t.mips.first().context("Texture has no image")?;
            if first.width() == 0
                || first.height() == 0
                || first.width().max(first.height()) > limits.max_texture_dimension_2d
            {
                bail!("Texture dimensions exceed the selected GPU capability");
            }
            let expected_levels = first.width().max(first.height()).ilog2() + 1;
            if t.mips.len() as u64 > u64::from(expected_levels) {
                bail!("Too many texture mip levels");
            }
            for (i, mip) in t.mips.iter().enumerate() {
                if mip.width() != (first.width() >> i).max(1)
                    || mip.height() != (first.height() >> i).max(1)
                {
                    bail!("Texture mip dimensions are inconsistent");
                }
            }
        }
        let scope = GpuScope::begin(device);
        let mut pipelines = BTreeMap::new();
        for draw in &scene.draws {
            let key = PipelineKey::material(&draw.material);
            if let std::collections::btree_map::Entry::Vacant(entry) = pipelines.entry(key) {
                entry.insert(self.pipeline(key)?);
            }
        }
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("mipmapped original textures"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            anisotropy_clamp: 4,
            ..Default::default()
        });
        // Native compositor 72A650 -> 8A07E0(3): wrap U/V; 8A0830(0):
        // point min/mag, no mip filtering. Ground diffuse keeps its sampler.
        let mask_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("native terrain transition sampling"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: 0.0,
            ..Default::default()
        });
        let fallback = CpuTexture {
            mips: vec![image::RgbaImage::from_pixel(1, 1, image::Rgba([255; 4]))],
        };
        let mut texture_views = BTreeMap::new();
        let mut bytes = 0u64;
        for (key, tex) in std::iter::once(("", &fallback))
            .chain(scene.textures.iter().map(|(k, v)| (k.as_str(), v)))
        {
            let base = &tex.mips[0];
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
                bytes += rgba.len() as u64;
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
            texture_views.insert(key.to_owned(), texture.create_view(&Default::default()));
        }
        let mut triangles = 0;
        let draws: Vec<_> = scene
            .draws
            .iter()
            .filter(|d| !d.indices.is_empty() && !d.instances.is_empty())
            .map(|d| {
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
                        d.material
                            .terrain_mask
                            .as_ref()
                            .map_or(0.0, |m| m.cell_size),
                    ],
                    terrain_u: d
                        .material
                        .terrain_mask
                        .as_ref()
                        .map_or([0.0; 4], |m| m.rows[0]),
                    terrain_v: {
                        // .w (unused by the mask rows): this draw is TerrainBlock
                        // geometry, which the native fog texture effect covers.
                        let mut v = d.material.terrain_mask.as_ref().map_or([0.0; 4], |m| m.rows[1]);
                        v[3] = f32::from(d.material.terrain_layer);
                        v
                    },
                };
                let mat_buffer = upload_buffer(
                    device,
                    queue,
                    "original material values",
                    bytemuck::bytes_of(&uniform),
                    wgpu::BufferUsages::UNIFORM,
                );
                let texture = d
                    .material
                    .texture
                    .as_ref()
                    .and_then(|k| texture_views.get(k))
                    .unwrap_or(&texture_views[""]);
                let material = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("original material binding"),
                    layout: &self.material_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(texture),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: mat_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::TextureView(
                                d.material
                                    .terrain_mask
                                    .as_ref()
                                    .and_then(|mask| texture_views.get(&mask.texture))
                                    .unwrap_or(&texture_views[""]),
                            ),
                        },
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: wgpu::BindingResource::Sampler(&mask_sampler),
                        },
                    ],
                });
                let vertices = upload_buffer(
                    device,
                    queue,
                    "original mesh vertices",
                    bytemuck::cast_slice(&d.vertices),
                    wgpu::BufferUsages::VERTEX,
                );
                let indices = upload_buffer(
                    device,
                    queue,
                    "original mesh indices",
                    bytemuck::cast_slice(&d.indices),
                    wgpu::BufferUsages::INDEX,
                );
                let instances = upload_buffer(
                    device,
                    queue,
                    "original map placements",
                    bytemuck::cast_slice(&d.instances),
                    wgpu::BufferUsages::VERTEX,
                );
                bytes += vertices.size() + indices.size() + instances.size() + mat_buffer.size();
                triangles += (d.indices.len() / 3) as u64 * d.instances.len() as u64;
                Draw {
                    vertices,
                    indices,
                    instances,
                    instance_data: d.instances.clone(),
                    visible: vec![true; d.instances.len()],
                    index_count: d.indices.len() as u32,
                    instance_count: d.instances.len() as u32,
                    material,
                    terrain_layer: d.material.terrain_layer,
                    pipeline: PipelineKey::material(&d.material),
                }
            })
            .collect();
        let mut immediate = Vec::new();
        let mut transparent = Vec::new();
        let mut approximate_bound = false;
        for (draw_index, draw) in scene
            .draws
            .iter()
            .filter(|draw| !draw.indices.is_empty() && !draw.instances.is_empty())
            .enumerate()
        {
            if draw.material.terrain_layer {
                continue;
            }
            let flags = draw
                .material
                .native_render_state
                .map_or(u16::from(draw.material.blend), |state| state.alpha_flags);
            let queued = draw.material.blend && transparency::queues_geometry(flags, true);
            let center = if queued {
                draw.sort_center.unwrap_or_else(|| {
                    approximate_bound = true;
                    vertex_center(&draw.vertices)
                })
            } else {
                [0.0; 3]
            };
            for (index, instance) in draw.instances.iter().enumerate() {
                let order = draw
                    .source_order
                    .as_ref()
                    .map_or([draw_index, index], |order| order[index]);
                if queued {
                    let model = Mat4::from_cols_array_2d(&instance.model);
                    let local_center = center;
                    let center = model.transform_point3(Vec3::from(center)).to_array();
                    if !center.iter().all(|value| value.is_finite()) {
                        bail!("Invalid transparent world bound");
                    }
                    transparent.push((
                        order,
                        TransparentInstance {
                            draw: draw_index,
                            instance: index as u32,
                            center,
                            local_center,
                            model,
                            dynamic_bound: draw.sort_center.is_none(),
                        },
                    ));
                } else {
                    immediate.push((order, draw_index, index as u32));
                }
            }
        }
        immediate.sort_by_key(|&(order, _, _)| order);
        transparent.sort_by_key(|(order, _)| *order);
        let mut immediate_commands = Vec::new();
        for (_, draw, instance) in immediate {
            append_command(&mut immediate_commands, draw, instance);
        }
        if approximate_bound {
            log::warn!(
                "Transparent animated/legacy meshes use posed vertex-box centers; native skinned sphere propagation is not reconstructed."
            );
        }
        // Submit pending uploads inside the scope, then retain the existing
        // viewable scene unless every reported operation succeeded.
        queue.submit([]);
        scope.finish("scene upload")?;
        Ok(PreparedUpload {
            draws,
            pipelines,
            immediate_commands,
            transparent_instances: transparent
                .into_iter()
                .map(|(_, instance)| instance)
                .collect(),
            bytes,
            triangles,
        })
    }

    fn refresh_transparent_slots(&mut self) {
        self.transparent_slots = self
            .transparent_instances
            .iter()
            .enumerate()
            .map(|(index, instance)| {
                (
                    InstanceSlot {
                        draw: instance.draw,
                        instance: instance.instance,
                    },
                    index,
                )
            })
            .collect();
    }

    /// Append a source-resolved actor without changing the epoch or any existing
    /// slot. Caller publication order is preserved after the original scene;
    /// this is presentation ordering, not a claim of native scene insertion.
    pub(crate) fn append_actor(
        &mut self,
        expected_epoch: u64,
        scene: &CpuScene,
        visible: bool,
    ) -> Result<Vec<InstanceSlot>> {
        if expected_epoch == 0 || expected_epoch != self.scene_epoch() {
            bail!("Spawn belongs to a different uploaded scene");
        }
        if scene
            .draws
            .iter()
            .any(|d| d.material.terrain_layer || d.instances.len() != 1)
        {
            bail!("Actor append requires one instance per nonterrain mesh");
        }
        let mut prepared = self.prepare_upload(scene)?;
        if prepared.draws.is_empty() {
            bail!("Spawn has no renderable meshes");
        }
        let base = self.draws.len();
        base.checked_add(prepared.draws.len())
            .context("Draw slot overflow")?;
        let bytes = self
            .uploaded_bytes
            .checked_add(prepared.bytes)
            .context("GPU accounting overflow")?;
        let triangles = self
            .triangles
            .checked_add(prepared.triangles)
            .context("Triangle accounting overflow")?;
        let slots = (0..prepared.draws.len())
            .map(|i| InstanceSlot {
                draw: base + i,
                instance: 0,
            })
            .collect();
        // All fallible capacity checks precede publication. Allocation failure
        // inside the system allocator/device loss is not a rollback guarantee.
        self.draws.try_reserve(prepared.draws.len())?;
        self.immediate_commands
            .try_reserve(prepared.immediate_commands.len())?;
        self.transparent_instances
            .try_reserve(prepared.transparent_instances.len())?;
        for draw in &mut prepared.draws {
            draw.visible.fill(visible);
        }
        for (draw, _) in &mut prepared.immediate_commands {
            *draw += base;
        }
        for instance in &mut prepared.transparent_instances {
            instance.draw += base;
        }
        self.draws.extend(prepared.draws);
        for (key, pipeline) in prepared.pipelines {
            self.pipelines.entry(key).or_insert(pipeline);
        }
        self.immediate_commands.extend(prepared.immediate_commands);
        self.transparent_instances
            .extend(prepared.transparent_instances);
        self.refresh_transparent_slots();
        self.cached_direction = None;
        self.cached_bundle = None;
        self.uploaded_bytes = bytes;
        self.triangles = triangles;
        self.draw_calls = self.draws.len();
        Ok(slots)
    }

    pub fn scene_epoch(&self) -> u64 {
        self.scene_epoch.load(Ordering::Acquire)
    }
    pub(crate) fn identity_witness(&self)->RendererWitness {
        RendererWitness {owner:Arc::downgrade(&self.scene_epoch),epoch:self.scene_epoch()}
    }
    pub(crate) fn install_native_scene(&mut self,scene:crate::live_scene::scene_backend::Installed)->Result<()> {
        if self.scene_epoch()==0 {bail!("native scene backend requires an actual uploaded static scene");}
        if self.native_scene.is_some(){bail!("native scene backend is already retained; actual teardown/update is required");}
        //Retain the same semantic installation even if a GPU write fails.
        //No receipt is emitted on that path and render refuses it explicitly.
        self.native_scene=Some(scene);
        let scope=GpuScope::begin(&self.state.device);
        let u=self.native_scene.as_ref().unwrap().uniforms();
        macro_rules! write {($field:ident,$value:expr)=>{self.state.queue.write_buffer(&self.camera_buffer,std::mem::offset_of!(CameraUniform,$field)as u64,bytemuck::bytes_of($value));}}
        write!(light,&u.direction);write!(model_root,&u.root);write!(root_inverse,&u.inverse);write!(root_normal,&u.normal);
        write!(light_ambient,&u.ambient);write!(light_diffuse,&u.diffuse);write!(light_specular,&u.specular);
        scope.finish("native root/light uniform installation")?;
        self.native_scene.as_mut().unwrap().accept_gpu_writes();Ok(())
    }

    /// Changes only existing placement buffers and CPU draw visibility. Models,
    /// poses, materials and textures remain resident. All caller validation is
    /// completed before queue writes; device loss is not a transactional rollback.
    pub fn update_instances(
        &mut self,
        expected_epoch: u64,
        updates: &[InstanceUpdate],
    ) -> Result<InstanceUpdateStats> {
        if expected_epoch == 0 || expected_epoch != self.scene_epoch() {
            bail!("Live instances belong to a different uploaded scene");
        }
        let mut staged = BTreeMap::new();
        let mut order_invalidated = false;
        for update in updates {
            let draw = self
                .draws
                .get(update.slot.draw)
                .context("Live draw slot is out of range")?;
            let index = update.slot.instance as usize;
            let old = draw
                .instance_data
                .get(index)
                .context("Live instance slot is out of range")?;
            if draw.terrain_layer {
                bail!("Actor updates cannot modify terrain layer instances");
            }
            if update
                .value
                .model
                .iter()
                .flatten()
                .chain(&update.value.team_color)
                .any(|n| !n.is_finite())
            {
                bail!("Live instance has nonfinite transform or color");
            }
            if update
                .value
                .team_color
                .iter()
                .any(|n| !(0.0..=1.0).contains(n))
            {
                bail!("Live instance replacement color is outside 0..1");
            }
            let model = Mat4::from_cols_array_2d(&update.value.model);
            let new_center = if let Some(&index) = self.transparent_slots.get(&update.slot) {
                let center = model
                    .transform_point3(Vec3::from(self.transparent_instances[index].local_center))
                    .to_array();
                if center.iter().any(|n| !n.is_finite()) {
                    bail!("Live transform produces a nonfinite transparency bound");
                }
                order_invalidated |= old.model != update.value.model && update.visible;
                Some(center)
            } else {
                None
            };
            order_invalidated |= draw.visible[index] != update.visible;
            if staged.insert(update.slot, (*update, new_center)).is_some() {
                bail!("Duplicate live instance slot in one snapshot");
            }
        }
        staged.retain(|slot, (update, _)| {
            let draw = &self.draws[slot.draw];
            draw.instance_data[slot.instance as usize] != update.value
                || draw.visible[slot.instance as usize] != update.visible
        });
        if staged.is_empty() {
            return Ok(InstanceUpdateStats::default());
        }
        let scope = GpuScope::begin(&self.state.device);
        let mut uploaded_bytes = 0;
        //Adjacent slots within one draw become one bounded queue write.
        let mut writes: Vec<(InstanceSlot, Vec<Instance>)> = Vec::new();
        for (slot, (update, _)) in &staged {
            if self.draws[slot.draw].instance_data[slot.instance as usize] == update.value {
                continue;
            }
            if let Some((first, values)) = writes.last_mut()
                && first.draw == slot.draw
                && u64::from(first.instance) + values.len() as u64 == u64::from(slot.instance)
            {
                values.push(update.value);
            } else {
                writes.push((*slot, vec![update.value]));
            }
        }
        for (first, values) in &writes {
            let bytes = bytemuck::cast_slice(values);
            self.state.queue.write_buffer(
                &self.draws[first.draw].instances,
                u64::from(first.instance) * std::mem::size_of::<Instance>() as u64,
                bytes,
            );
            uploaded_bytes += bytes.len() as u64;
        }
        if uploaded_bytes != 0 {
            self.state.queue.submit([]);
        }
        scope.finish("live actor instance update")?;
        let changed = staged.len();
        for (slot, (update, new_center)) in staged {
            let draw = &mut self.draws[slot.draw];
            draw.instance_data[slot.instance as usize] = update.value;
            draw.visible[slot.instance as usize] = update.visible;
            if let Some(&index) = self.transparent_slots.get(&slot) {
                let instance = &mut self.transparent_instances[index];
                instance.model = Mat4::from_cols_array_2d(&update.value.model);
                instance.center = new_center.context("Missing staged transparent center")?;
            }
        }
        if order_invalidated {
            self.cached_direction = None;
            self.cached_bundle = None;
        }
        Ok(InstanceUpdateStats {
            changed,
            uploaded_bytes,
            order_invalidated,
        })
    }

    ///Inspector-only pose update. Authored material opacity has already been
    ///replaced by the sampled property value in each vertex; the material
    ///uniform remains unchanged. Hidden/collapsed meshes keep their buffers.
    pub fn update_preview_pose(
        &mut self,
        meshes: &[Vec<Vertex>],
        visible: &[bool],
    ) -> Result<()> {
        if meshes.len() != self.draws.len() || visible.len() != self.draws.len() {
            bail!("Preview pose/visibility count differs from uploaded model");
        }
        //Validate the complete batch before geometry or visibility changes.
        //This surface is for isolated inspector models, not world instances.
        for (vertices, draw) in meshes.iter().zip(&self.draws) {
            if draw.instance_count != 1 || draw.terrain_layer {
                bail!("Preview pose requires one isolated instance per mesh");
            }
            if vertices.iter().any(|vertex| {
                vertex.position.iter().chain(&vertex.normal).chain(&vertex.uv)
                    .chain(&vertex.color).chain(&vertex.terrain_uv).any(|v| !v.is_finite())
            }) {
                bail!("Preview pose has non-finite geometry or material alpha");
            }
        }
        self.update_vertices(meshes)?;
        for (draw, visible) in self.draws.iter_mut().zip(visible) {
            draw.visible.fill(*visible);
        }
        //The existing vertex update already invalidated ordering/bundles;
        //both immediate and translucent passes use these same visible bits.
        Ok(())
    }

    /// Replace geometry for an already-uploaded isolated animated model.
    /// Topology, UVs and materials stay fixed; no game-state mutation occurs.
    pub fn update_vertices(&mut self, meshes: &[Vec<Vertex>]) -> Result<()> {
        if meshes.len() != self.draws.len() {
            bail!("Animated mesh count differs from uploaded model");
        }
        for (vertices, draw) in meshes.iter().zip(&self.draws) {
            let bytes = vertices
                .len()
                .checked_mul(std::mem::size_of::<Vertex>())
                .context("Animated vertex count overflow")?;
            if bytes as u64 != draw.vertices.size() {
                bail!("Animated vertex layout differs from uploaded model");
            }
            if vertices
                .iter()
                .any(|v| v.position.iter().chain(&v.normal).any(|x| !x.is_finite()))
            {
                bail!("Animated pose has non-finite geometry");
            }
        }
        let scope = GpuScope::begin(&self.state.device);
        for (vertices, draw) in meshes.iter().zip(&self.draws) {
            self.state
                .queue
                .write_buffer(&draw.vertices, 0, bytemuck::cast_slice(vertices));
        }
        self.state.queue.submit([]);
        scope.finish("animated model pose update")?;
        for instance in &mut self.transparent_instances {
            if instance.dynamic_bound {
                instance.local_center = vertex_center(&meshes[instance.draw]);
                instance.center = instance
                    .model
                    .transform_point3(Vec3::from(instance.local_center))
                    .to_array();
            }
        }
        self.cached_direction = None;
        self.cached_bundle = None;
        Ok(())
    }

    /// A spawned actor owns these meshes exclusively. Validate the complete
    /// subset before any writes; existing actors and instance matrices remain.
    pub(crate) fn update_actor_vertices(
        &mut self,
        epoch: u64,
        updates: &[(InstanceSlot, Vec<Vertex>)],
    ) -> Result<()> {
        if epoch == 0 || epoch != self.scene_epoch() {
            bail!("Animated actor belongs to a previous scene");
        }
        let mut seen = std::collections::BTreeSet::new();
        let mut centers = Vec::new();
        for (slot, vertices) in updates {
            let draw = self
                .draws
                .get(slot.draw)
                .context("Animated actor draw is absent")?;
            if slot.instance != 0
                || draw.instance_count != 1
                || draw.terrain_layer
                || !seen.insert(slot.draw)
            {
                bail!("Animated actor requires exclusively owned mesh slots");
            }
            let size = vertices
                .len()
                .checked_mul(std::mem::size_of::<Vertex>())
                .context("Animated vertex size overflow")?;
            if size as u64 != draw.vertices.size()
                || vertices.iter().any(|v| {
                    v.position
                        .iter()
                        .chain(&v.normal)
                        .chain(&v.uv)
                        .chain(&v.color)
                        .any(|x| !x.is_finite())
                })
            {
                bail!("Invalid spawned actor pose layout/data");
            }
            if let Some(&index) = self.transparent_slots.get(slot) {
                let local = vertex_center(vertices);
                let world = self.transparent_instances[index]
                    .model
                    .transform_point3(Vec3::from(local))
                    .to_array();
                if world.iter().any(|v| !v.is_finite()) {
                    bail!("Invalid spawned pose transparency bound");
                }
                centers.push((index, local, world));
            }
        }
        let scope = GpuScope::begin(&self.state.device);
        for (slot, vertices) in updates {
            self.state.queue.write_buffer(
                &self.draws[slot.draw].vertices,
                0,
                bytemuck::cast_slice(vertices),
            );
        }
        self.state.queue.submit([]);
        scope.finish("live actor pose update")?;
        for (index, local, world) in centers {
            let i = &mut self.transparent_instances[index];
            i.local_center = local;
            i.center = world;
            i.dynamic_bound = true;
        }
        if updates
            .iter()
            .any(|(slot, _)| self.transparent_slots.contains_key(slot))
        {
            self.cached_direction = None;
            self.cached_bundle = None;
        }
        Ok(())
    }

    fn resize(&mut self, desired: [u32; 2]) -> Result<()> {
        let max = self.state.device.limits().max_texture_dimension_2d;
        let size = [desired[0].clamp(1, max), desired[1].clamp(1, max)];
        if self.target.as_ref().map(|t| t.size) == Some(size) {
            return Ok(());
        }
        let scope = GpuScope::begin(&self.state.device);
        let descriptor = |format, usage| wgpu::TextureDescriptor {
            label: Some("scene viewport"),
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: if format == FORMAT {
                &[wgpu::TextureFormat::Rgba8Unorm]
            } else {
                &[]
            },
        };
        let color = self.state.device.create_texture(&descriptor(
            FORMAT,
            wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
        ));
        let view = color.create_view(&Default::default());
        let depth = self
            .state
            .device
            .create_texture(&descriptor(DEPTH, wgpu::TextureUsages::RENDER_ATTACHMENT));
        let depth_view = depth.create_view(&Default::default());
        scope.finish("viewport allocation")?;
        let scope = GpuScope::begin(&self.state.device);
        // The render pass writes linear shader output through an sRGB target,
        // producing gamma-encoded bytes (also used by PNG readback). Egui's
        // native-texture shader expects those gamma values. A non-sRGB sampling
        // view avoids decoding them a second time and darkening the viewport.
        let egui_view = color.create_view(&wgpu::TextureViewDescriptor {
            label: Some("scene gamma bytes for egui"),
            format: Some(wgpu::TextureFormat::Rgba8Unorm),
            ..Default::default()
        });
        let egui_id = self.state.renderer.write().register_native_texture(
            &self.state.device,
            &egui_view,
            wgpu::FilterMode::Linear,
        );
        if let Err(error) = scope.finish("viewport registration") {
            self.state.renderer.write().free_texture(&egui_id);
            return Err(error);
        }
        let replacement = Target {
            identity:Arc::new(()),
            color,
            view,
            _depth: depth,
            depth_view,
            egui_id,
            size,
        };
        if let Some(old) = self.target.replace(replacement) {
            self.state.renderer.write().free_texture(&old.egui_id);
        }
        Ok(())
    }
    pub fn render(&mut self, camera: &Camera, size: [u32; 2]) -> Result<egui::TextureId> {
        if self.native_scene.is_some(){bail!("actual native scene requires its current native camera snapshot");}
        self.resize(size)?;
        let target=self.target.as_ref().unwrap();
        self.render_camera(camera.view_projection(target.size[0]as f32/target.size[1]as f32),camera.eye(),
            (camera.target-camera.eye()).normalize(),target.size)
    }
    ///Battle projection comes only from the sealed actual current NiCamera.
    ///Successful drawing grants a correlated geometric-picking packet, not
    ///native VWorld completion, loading completion or application activation.
    pub fn render_native(&mut self,camera:&Arc<crate::live_scene::camera_backend::Frame>,size:[u32;2])
        ->Result<(egui::TextureId,crate::live_scene::camera_backend::Rendered)>{
        camera.require_current(size)?;
        let max=self.state.device.limits().max_texture_dimension_2d;
        if size.contains(&0)||size.iter().any(|v|*v>max){bail!("native camera viewport exceeds actual GPU target dimensions");}
        let scene=self.native_scene.as_ref().context("actual native scene renderer is not installed")?;
        scene.require_alive()?;
        let inverse=Mat4::from_cols_array_2d(&scene.uniforms().inverse);
        let texture=self.render_camera(camera.matrix(),camera.eye(),camera.direction(),size)?;
        camera.require_current(size)?;
        let picking=crate::live_scene::camera_backend::Rendered::after_draw(camera,self.identity_witness(),inverse);
        Ok((texture,picking))
    }
    fn render_camera(&mut self,view_projection:Mat4,eye:Vec3,direction:Vec3,size:[u32;2])->Result<egui::TextureId>{
        if let Some(scene)=&self.native_scene{scene.require_alive()?;}
        self.resize(size)?;
        let direction=if let Some(scene)=&self.native_scene {
            Mat4::from_cols_array_2d(&scene.uniforms().root).transpose().transform_vector3(direction)
                .try_normalize().context("native scene root has no usable depth-sort direction")?
        }else{direction}.to_array();
        self.prepare_commands(direction)?;
        let scope = GpuScope::begin(&self.state.device);
        let target = self.target.as_ref().unwrap();
        let uniform = CameraUniform {
            view_projection:view_projection.to_cols_array_2d(),
            eye: [eye.x, eye.y, eye.z, 1.0],
            light: self.native_scene.as_ref().map_or([-0.5,1.0,0.35,0.0],|v|v.uniforms().direction),
            team_color: [
                self.team_color[0],
                self.team_color[1],
                self.team_color[2],
                1.0,
            ],
            fog: self.fog_params,
            model_root:self.native_scene.as_ref().map_or(Mat4::IDENTITY.to_cols_array_2d(),|v|v.uniforms().root),
            root_inverse:self.native_scene.as_ref().map_or(Mat4::IDENTITY.to_cols_array_2d(),|v|v.uniforms().inverse),
            root_normal:self.native_scene.as_ref().map_or(Mat4::IDENTITY.to_cols_array_2d(),|v|v.uniforms().normal),
            light_ambient:self.native_scene.as_ref().map_or([0.0;4],|v|v.uniforms().ambient),
            light_diffuse:self.native_scene.as_ref().map_or([0.0;4],|v|v.uniforms().diffuse),
            light_specular:self.native_scene.as_ref().map_or([0.0;4],|v|v.uniforms().specular),
        };
        self.state
            .queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&uniform));
        let mut encoder =
            self.state
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("original scene draw"),
                });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene viewport"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target.view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: self.clear_color[0],
                            g: self.clear_color[1],
                            b: self.clear_color[2],
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &target.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.execute_bundles(std::iter::once(
                self.cached_bundle
                    .as_ref()
                    .context("Missing scene draw bundle")?,
            ));
            if let Some(dynamic) = &self.dynamic {
                dynamic.draw(&mut pass, &self.camera_binding);
            }
            if let Some(particles) = &self.particles {
                particles.draw(&mut pass, &self.camera_binding);
            }
        }
        self.state.queue.submit([encoder.finish()]);
        scope.finish("render")?;
        Ok(target.egui_id)
    }

    /// Immutable geometry produces the same exact keys for the same direction.
    /// Successful scene/pose changes invalidate this cache. View translation,
    /// target size, and team color do not participate in the native depth key.
    fn prepare_commands(&mut self, direction: [f32; 3]) -> Result<()> {
        let bits = direction.map(f32::to_bits);
        if self.cached_direction == Some(bits) {
            return Ok(());
        }
        let mut commands: Vec<DrawCommand> = self
            .draws
            .iter()
            .enumerate()
            .filter(|(_, draw)| draw.terrain_layer)
            .map(|(index, draw)| (index, 0..draw.instance_count))
            .collect();
        for (draw, range) in &self.immediate_commands {
            for instance in range.clone() {
                if self.draws[*draw].visible[instance as usize] {
                    append_command(&mut commands, *draw, instance);
                }
            }
        }
        // The live original rendering FPU lifecycle is not yet proven. The
        // preview explicitly uses binary32 intermediates, matching its world
        // transforms; the native helper itself supports all three x87 modes.
        let mut transparent = self
            .transparent_instances
            .iter()
            .enumerate()
            .filter(|(_, instance)| self.draws[instance.draw].visible[instance.instance as usize])
            .map(|(index, instance)| {
                Ok(transparency::Entry {
                    key: transparency::depth_key(
                        instance.center,
                        0.0,
                        direction,
                        false,
                        aa_numeric::Precision::Single24,
                    )?,
                    value: index,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        transparency::submission_order(&mut transparent)?;
        for entry in transparent {
            let instance = &self.transparent_instances[entry.value];
            append_command(&mut commands, instance.draw, instance.instance);
        }
        //Recording the same validated command order in a render bundle avoids
        //re-encoding thousands of static map draws on every frame. Uniform and
        //vertex-buffer contents remain live; ordering/geometry invalidations
        //above rebuild both caches together.
        let scope = GpuScope::begin(&self.state.device);
        let mut encoder =
            self.state
                .device
                .create_render_bundle_encoder(&wgpu::RenderBundleEncoderDescriptor {
                    label: Some("cached original scene draw order"),
                    color_formats: &[Some(FORMAT)],
                    depth_stencil: Some(wgpu::RenderBundleDepthStencil {
                        format: DEPTH,
                        depth_read_only: false,
                        stencil_read_only: true,
                    }),
                    sample_count: 1,
                    multiview: None,
                });
        encoder.set_bind_group(0, &self.camera_binding, &[]);
        for (index, instances) in &commands {
            let draw = &self.draws[*index];
            encoder.set_pipeline(&self.pipelines[&draw.pipeline]);
            encoder.set_bind_group(1, &draw.material, &[]);
            encoder.set_vertex_buffer(0, draw.vertices.slice(..));
            encoder.set_vertex_buffer(1, draw.instances.slice(..));
            encoder.set_index_buffer(draw.indices.slice(..), wgpu::IndexFormat::Uint32);
            encoder.draw_indexed(0..draw.index_count, 0, instances.clone());
        }
        let bundle = encoder.finish(&wgpu::RenderBundleDescriptor {
            label: Some("cached original scene draws"),
        });
        scope.finish("record scene draw bundle")?;
        self.draw_calls = commands.len();
        self.cached_commands = commands;
        self.cached_bundle = Some(bundle);
        self.cached_direction = Some(bits);
        self.ordering_rebuilds = self.ordering_rebuilds.saturating_add(1);
        Ok(())
    }
    pub fn save_view(&self, path: &std::path::Path) -> Result<()> {
        let target = self.target.as_ref().context("No rendered viewport")?;
        let row = (target.size[0] * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let scope = GpuScope::begin(&self.state.device);
        let buffer = self.state.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("render validation readback"),
            size: row as u64 * target.size[1] as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .state
            .device
            .create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            target.color.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(target.size[1]),
                },
            },
            wgpu::Extent3d {
                width: target.size[0],
                height: target.size[1],
                depth_or_array_layers: 1,
            },
        );
        self.state.queue.submit([encoder.finish()]);
        scope.finish("viewport readback allocation/copy")?;
        let (send, receive) = std::sync::mpsc::sync_channel(1);
        buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = send.send(r);
        });
        self.state.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })?;
        receive.recv_timeout(std::time::Duration::from_secs(1))??;
        let mapped = buffer.slice(..).get_mapped_range();
        let mut rgba = image::RgbaImage::new(target.size[0], target.size[1]);
        for (y, bytes) in mapped.chunks(row as usize).enumerate() {
            let begin = y * target.size[0] as usize * 4;
            rgba.as_mut()[begin..begin + target.size[0] as usize * 4]
                .copy_from_slice(&bytes[..target.size[0] as usize * 4]);
        }
        drop(mapped);
        buffer.unmap();
        // The command promises a new output; create_new also closes the gap
        // between the initial command-line check and this later GPU readback.
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        image::DynamicImage::ImageRgba8(rgba).write_to(&mut output, image::ImageFormat::Png)?;
        output.sync_all()?;
        Ok(())
    }
}

impl Drop for SceneRenderer {
    fn drop(&mut self) {
        if let Some(target) = &self.target {
            self.state.renderer.write().free_texture(&target.egui_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    #[ignore = "requires a native graphics adapter; run explicitly"]
    fn preview_pose_alpha_visibility_and_failed_batch_keep_resident_scene() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN | wgpu::Backends::DX12,
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            required_limits: adapter.limits(),
            ..Default::default()
        })).unwrap();
        let egui_renderer = egui_wgpu::Renderer::new(&device, FORMAT, Default::default());
        let mut renderer = SceneRenderer::new(egui_wgpu::RenderState {
            adapter, available_adapters: Vec::new(), device, queue, target_format: FORMAT,
            renderer: Arc::new(egui::mutex::RwLock::new(egui_renderer)),
        }).unwrap();
        let camera = Camera::for_model(&aa_assets::Bounds {
            min: [-1.0; 3], max: [1.0; 3], size: [2.0; 3],
        });
        let toward_eye = (camera.eye() - camera.target).normalize();
        let right = toward_eye.cross(Vec3::Y).normalize();
        let up = right.cross(toward_eye).normalize();
        let vertices = [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]]
            .map(|[x, y]| Vertex {
                position: (right * x + up * y).to_array(),
                normal: [-0.5, 1.0, 0.35], uv: [0.5; 2],
                color: [1.0; 4], terrain_uv: [0.0; 2],
            }).to_vec();
        let scene = CpuScene {
            objects: None, live: None, name: "animation preview alpha".into(), map_id: "test".into(),
            size: [2.0; 2], draws: vec![CpuDraw {
                vertices: vertices.clone(), indices: vec![0, 1, 2, 0, 2, 3],
                instances: vec![Instance::identity()], material: authored(237, 0, 1, 3),
                source_order: None, sort_center: None,
            }], textures: BTreeMap::new(), placements: 1, rendered_instances: 1,
            warnings: Vec::new(), load_seconds: 0.0,
        };
        renderer.upload(&scene).unwrap();
        let epoch = renderer.scene_epoch();
        let resident_bytes = renderer.uploaded_bytes;
        let original_instance = renderer.draws[0].instance_data[0];
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let folder = std::env::temp_dir().join(format!("aa-preview-alpha-{}-{stamp}", std::process::id()));
        std::fs::create_dir(&folder).unwrap();
        let capture = |renderer: &mut SceneRenderer, name: &str| {
            renderer.render(&camera, [96, 96]).unwrap();
            let path = folder.join(format!("{name}.png"));
            renderer.save_view(&path).unwrap();
            image::open(path).unwrap().to_rgba8()
        };
        let full = capture(&mut renderer, "full");
        let mut invalid = vertices.clone();
        invalid[0].color[3] = f32::NAN;
        assert!(renderer.update_preview_pose(&[invalid], &[false]).is_err());
        assert!(renderer.update_preview_pose(&[vertices.clone()], &[]).is_err());
        assert!(renderer.draws[0].visible[0]);
        assert_eq!(full, capture(&mut renderer, "invalid"));
        let mut half = vertices.clone();
        for vertex in &mut half { vertex.color[3] = 0.5; }
        renderer.update_preview_pose(&[half.clone()], &[true]).unwrap();
        let faded = capture(&mut renderer, "half");
        assert_ne!(full, faded, "sampled opacity never reached the GPU blend");
        renderer.update_preview_pose(&[half], &[false]).unwrap();
        let hidden = capture(&mut renderer, "hidden");
        assert_ne!(faded, hidden, "collapsed mesh remained visible");
        let mut zero = vertices.clone();
        for vertex in &mut zero { vertex.color[3] = 0.0; }
        renderer.update_preview_pose(&[zero], &[true]).unwrap();
        assert_eq!(hidden, capture(&mut renderer, "zero"));
        renderer.update_preview_pose(&[vertices], &[true]).unwrap();
        assert_eq!(full, capture(&mut renderer, "restored"));
        assert_eq!(renderer.scene_epoch(), epoch);
        assert_eq!(renderer.uploaded_bytes, resident_bytes);
        assert_eq!(renderer.draws[0].instance_data[0], original_instance);
    }

    #[test]
    #[ignore = "requires a native graphics adapter; run explicitly"]
    fn live_instance_updates_keep_assets_picking_contract_and_native_order() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN | wgpu::Backends::DX12,
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            required_limits: adapter.limits(),
            ..Default::default()
        }))
        .unwrap();
        let egui_renderer = egui_wgpu::Renderer::new(&device, FORMAT, Default::default());
        let mut renderer = SceneRenderer::new(egui_wgpu::RenderState {
            adapter,
            available_adapters: Vec::new(),
            device,
            queue,
            target_format: FORMAT,
            renderer: Arc::new(egui::mutex::RwLock::new(egui_renderer)),
        })
        .unwrap();
        let camera = Camera::for_model(&aa_assets::Bounds {
            min: [-2.0; 3],
            max: [2.0; 3],
            size: [4.0; 3],
        });
        let toward_eye = (camera.eye() - camera.target).normalize();
        let right = toward_eye.cross(Vec3::Y).normalize();
        let up = right.cross(toward_eye).normalize();
        let make_instance = |position: Vec3, color: [f32; 3]| {
            let mut value = Instance::at(position.to_array(), 0.0, 1.0);
            value.team_color = [color[0], color[1], color[2], 1.0];
            value
        };
        let original = [
            make_instance(right * -0.9, [1.0, 0.0, 0.0]),
            make_instance(right * 0.9, [0.0, 0.0, 1.0]),
        ];
        let mut material = authored(0xec, 0, 3, 3);
        material.team_color_mask = true;
        material.texture = Some("mask".into());
        let mut scene = CpuScene {
            objects: None,
            live: None,
            name: "live instance GPU test".into(),
            map_id: "test".into(),
            size: [4.0; 2],
            draws: vec![CpuDraw {
                vertices: [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]]
                    .map(|[x, y]| Vertex {
                        position: (right * x * 0.6 + up * y * 0.6).to_array(),
                        normal: [-0.5, 1.0, 0.35],
                        uv: [0.5; 2],
                        color: [1.0; 4],
                        terrain_uv: [0.0; 2],
                    })
                    .to_vec(),
                indices: vec![0, 1, 2, 0, 2, 3],
                instances: original.to_vec(),
                material,
                source_order: None,
                sort_center: None,
            }],
            textures: BTreeMap::from([(
                "mask".into(),
                CpuTexture {
                    mips: vec![image::RgbaImage::from_pixel(
                        1,
                        1,
                        image::Rgba([255, 255, 255, 0]),
                    )],
                },
            )]),
            placements: 2,
            rendered_instances: 2,
            warnings: Vec::new(),
            load_seconds: 0.0,
        };
        renderer.upload(&scene).unwrap();
        let epoch = renderer.scene_epoch();
        let resident = renderer.uploaded_bytes;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let folder =
            std::env::temp_dir().join(format!("aa-live-instances-{}-{stamp}", std::process::id()));
        std::fs::create_dir(&folder).unwrap();
        let capture = |renderer: &mut SceneRenderer, name: &str| {
            renderer.render(&camera, [128, 128]).unwrap();
            let path = folder.join(format!("{name}.png"));
            renderer.save_view(&path).unwrap();
            image::open(path).unwrap().to_rgba8()
        };
        let counts = |image: &image::RgbaImage, axis: usize| {
            image
                .pixels()
                .filter(|p| {
                    p[axis] > p[(axis + 1) % 3].saturating_add(80)
                        && p[axis] > p[(axis + 2) % 3].saturating_add(80)
                })
                .count()
        };
        let baseline = capture(&mut renderer, "original");
        assert!(
            counts(&baseline, 0) > 20 && counts(&baseline, 2) > 20,
            "per-instance team colors did not remain distinct"
        );
        let ordered = renderer.ordering_rebuilds;
        let left = InstanceSlot {
            draw: 0,
            instance: 0,
        };
        let right_slot = InstanceSlot {
            draw: 0,
            instance: 1,
        };
        let mut green = original[0];
        green.team_color = [0.0, 1.0, 0.0, 1.0];
        let changed = renderer
            .update_instances(
                epoch,
                &[InstanceUpdate {
                    slot: left,
                    value: green,
                    visible: true,
                }],
            )
            .unwrap();
        assert_eq!(
            changed.uploaded_bytes,
            std::mem::size_of::<Instance>() as u64
        );
        assert!(!changed.order_invalidated);
        let recolored = capture(&mut renderer, "recolored");
        assert_eq!(renderer.ordering_rebuilds, ordered);
        assert_eq!(counts(&recolored, 0), 0);
        assert!(counts(&recolored, 1) > 20 && counts(&recolored, 2) > 20);
        let moved = make_instance(Vec3::X * 1000.0, [0.0, 1.0, 0.0]);
        let changed = renderer
            .update_instances(
                epoch,
                &[InstanceUpdate {
                    slot: left,
                    value: moved,
                    visible: true,
                }],
            )
            .unwrap();
        assert!(!changed.order_invalidated);
        let relocated = capture(&mut renderer, "relocated");
        assert_eq!(counts(&relocated, 1), 0);
        assert!(counts(&relocated, 2) > 20);
        assert_eq!(renderer.uploaded_bytes, resident);
        assert_eq!(renderer.ordering_rebuilds, ordered);
        //A valid earlier row must not leak through when a later row is invalid.
        let before_data = renderer.draws[0].instance_data.clone();
        assert!(
            renderer
                .update_instances(
                    epoch,
                    &[
                        InstanceUpdate {
                            slot: right_slot,
                            value: moved,
                            visible: true
                        },
                        InstanceUpdate {
                            slot: InstanceSlot {
                                draw: 99,
                                instance: 0
                            },
                            value: moved,
                            visible: true
                        },
                    ]
                )
                .is_err()
        );
        assert!(
            renderer
                .update_instances(
                    epoch,
                    &[
                        InstanceUpdate {
                            slot: left,
                            value: green,
                            visible: true
                        },
                        InstanceUpdate {
                            slot: left,
                            value: green,
                            visible: false
                        }
                    ]
                )
                .is_err()
        );
        assert_eq!(renderer.draws[0].instance_data, before_data);
        assert_eq!(capture(&mut renderer, "invalid"), relocated);
        let hidden = renderer
            .update_instances(
                epoch,
                &[InstanceUpdate {
                    slot: right_slot,
                    value: original[1],
                    visible: false,
                }],
            )
            .unwrap();
        assert!(hidden.order_invalidated);
        assert_eq!(hidden.uploaded_bytes, 0);
        let empty = capture(&mut renderer, "hidden");
        assert_eq!(counts(&empty, 2), 0);
        assert_eq!(renderer.cached_commands, vec![(0, 0..1)]);
        let no_op = renderer
            .update_instances(
                epoch,
                &[InstanceUpdate {
                    slot: right_slot,
                    value: original[1],
                    visible: false,
                }],
            )
            .unwrap();
        assert_eq!(no_op, InstanceUpdateStats::default());
        //The native queued bound follows a changed instance transform and later
        //posed local geometry, rather than retaining the original world center.
        scene.draws[0].material.blend = true;
        scene.draws[0].material.native_render_state = Some(aa_assets::NativeRenderState {
            alpha_flags: 0xed,
            alpha_threshold: 0,
            zbuffer_flags: 1,
            zbuffer_function: 3,
        });
        renderer.upload(&scene).unwrap();
        let new_epoch = renderer.scene_epoch();
        assert!(
            renderer
                .update_instances(
                    epoch,
                    &[InstanceUpdate {
                        slot: left,
                        value: green,
                        visible: true
                    }]
                )
                .is_err()
        );
        renderer.render(&camera, [128, 128]).unwrap();
        let before = renderer.ordering_rebuilds;
        let shifted = make_instance(toward_eye * 1.25, [0.0, 0.0, 1.0]);
        assert!(
            renderer
                .update_instances(
                    new_epoch,
                    &[InstanceUpdate {
                        slot: right_slot,
                        value: shifted,
                        visible: true
                    }]
                )
                .unwrap()
                .order_invalidated
        );
        renderer.render(&camera, [128, 128]).unwrap();
        assert_eq!(renderer.ordering_rebuilds, before + 1);
        let mut pose = scene.draws[0].vertices.clone();
        for vertex in &mut pose {
            vertex.position[0] += 0.25;
        }
        renderer.update_vertices(&[pose.clone()]).unwrap();
        renderer
            .update_instances(
                new_epoch,
                &[InstanceUpdate {
                    slot: right_slot,
                    value: original[1],
                    visible: true,
                }],
            )
            .unwrap();
        let idx = renderer.transparent_slots[&right_slot];
        let expected = Mat4::from_cols_array_2d(&original[1].model)
            .transform_point3(Vec3::from(vertex_center(&pose)));
        assert_eq!(
            renderer.transparent_instances[idx].center,
            expected.to_array()
        );
        assert_eq!(
            renderer.draws[0].vertices.size(),
            std::mem::size_of_val(pose.as_slice()) as u64
        );
    }

    fn authored(flags: u16, threshold: u8, zflags: u16, zfunction: u8) -> CpuMaterial {
        CpuMaterial {
            texture: None,
            color: [1.0; 4],
            emissive: [0.0; 3],
            cutoff: 0.0,
            blend: flags & 1 != 0,
            double_sided: true,
            team_color_mask: false,
            native_render_state: Some(aa_assets::NativeRenderState {
                alpha_flags: flags,
                alpha_threshold: threshold,
                zbuffer_flags: zflags,
                zbuffer_function: zfunction,
            }),
            terrain_layer: false,
            terrain_mask: None,
        }
    }

    #[test]
    fn native_material_state_vectors_preserve_authored_independent_switches() {
        let vectors: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/phase5-material-render/native-render-state-vectors.json"
        ))
        .unwrap();
        let blend_table = vectors["tables"]["blend"].as_array().unwrap();
        let compare_table = vectors["tables"]["alpha_compare"].as_array().unwrap();
        for case in vectors["alpha"].as_array().unwrap() {
            let flags = case["flags"].as_u64().unwrap() as u16;
            let threshold = case["threshold"].as_u64().unwrap() as u8;
            let material = authored(flags, threshold, 3, 3);
            let key = PipelineKey::material(&material);
            let calls = case["calls"].as_array().unwrap();
            let value = |state: u64| {
                calls.iter().find(|c| c[0] == state).unwrap()[1]
                    .as_u64()
                    .unwrap()
            };
            assert_eq!(u64::from(key.blend), value(27));
            assert_eq!(
                blend_table[usize::from(key.source)].as_u64().unwrap(),
                value(19)
            );
            assert_eq!(
                blend_table[usize::from(key.destination)].as_u64().unwrap(),
                value(20)
            );
            let (reference, mode) = alpha_test(&material);
            assert_eq!(reference, f32::from(threshold) / 255.0);
            if value(15) == 0 {
                assert_eq!(mode, 0.0);
            } else {
                assert_eq!(
                    compare_table[mode as usize - 1].as_u64().unwrap(),
                    value(25)
                );
            }
            assert_eq!(u64::from(threshold), value(24));
        }
        for case in vectors["depth"].as_array().unwrap() {
            let flags = case["flags"].as_u64().unwrap() as u16;
            let function = case["function"].as_u64().unwrap() as u8;
            let key = PipelineKey::material(&authored(237, 0, flags, function));
            let calls = case["calls"].as_array().unwrap();
            let value = |state: u64| {
                calls.iter().find(|c| c[0] == state).unwrap()[1]
                    .as_u64()
                    .unwrap()
            };
            assert_eq!(key.depth_write, value(7) != 0 && value(14) != 0);
            if value(7) != 0 {
                assert_eq!(
                    compare_table[usize::from(key.depth_compare)]
                        .as_u64()
                        .unwrap(),
                    value(23)
                );
            } else {
                assert_eq!(key.depth_compare, 0);
            }
        }
        let mut team = authored(4845, 150, 3, 3);
        team.team_color_mask = true;
        assert_eq!(alpha_test(&team), (0.0, 0.0));
        let unsupported = authored(1 | (8 << 1), 0, 3, 3);
        assert!(PipelineKey::material(&unsupported).blend_state().is_err());
    }

    #[test]
    #[ignore = "requires a native graphics adapter; run explicitly"]
    fn authored_material_layers_keep_solid_parts_and_cutout_holes() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::DX12 | wgpu::Backends::VULKAN,
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            required_limits: adapter.limits(),
            ..Default::default()
        }))
        .unwrap();
        let egui_renderer = egui_wgpu::Renderer::new(&device, FORMAT, Default::default());
        let mut renderer = SceneRenderer::new(egui_wgpu::RenderState {
            adapter,
            available_adapters: Vec::new(),
            device,
            queue,
            target_format: FORMAT,
            renderer: Arc::new(egui::mutex::RwLock::new(egui_renderer)),
        })
        .unwrap();
        let camera = Camera::for_model(&aa_assets::Bounds {
            min: [-1.0; 3],
            max: [1.0; 3],
            size: [2.0; 3],
        });
        let toward_eye = (camera.eye() - camera.target).normalize();
        let right = toward_eye.cross(Vec3::Y).normalize();
        let up = right.cross(toward_eye).normalize();
        let plane = |material: CpuMaterial, color: [f32; 4], depth: f32| CpuDraw {
            vertices: [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
                .map(|(x, y)| Vertex {
                    terrain_uv: [0.0; 2],
                    position: (toward_eye * depth + right * x * 3.0 + up * y * 3.0).to_array(),
                    normal: [-0.5, 1.0, 0.35],
                    uv: [0.5; 2],
                    color,
                })
                .to_vec(),
            indices: vec![0, 1, 2, 0, 2, 3],
            instances: vec![Instance::identity()],
            material,
            source_order: None,
            sort_center: None,
        };
        let mut sequence = 0;
        let mut pixel = |draws| {
            let scene = CpuScene {
                live: None,
                objects: None,
                name: "material-layer-test".into(),
                map_id: "synthetic".into(),
                size: [2.0; 2],
                draws,
                textures: BTreeMap::new(),
                placements: 0,
                rendered_instances: 0,
                warnings: Vec::new(),
                load_seconds: 0.0,
            };
            renderer.upload(&scene).unwrap();
            renderer.render(&camera, [32, 32]).unwrap();
            let built = renderer.ordering_rebuilds;
            let original_order = renderer.cached_commands.clone();
            renderer.render(&camera, [32, 32]).unwrap();
            assert_eq!(
                renderer.ordering_rebuilds, built,
                "unchanged view re-sorted geometry"
            );
            let mut opposite = Camera::for_model(&aa_assets::Bounds {
                min: [-1.0; 3],
                max: [1.0; 3],
                size: [2.0; 3],
            });
            opposite.yaw += std::f32::consts::PI;
            renderer.render(&opposite, [32, 32]).unwrap();
            assert_eq!(
                renderer.ordering_rebuilds,
                built + 1,
                "rotation did not rebuild order"
            );
            renderer.render(&camera, [32, 32]).unwrap();
            assert_eq!(
                renderer.cached_commands, original_order,
                "cached return view changed native submission order"
            );
            sequence += 1;
            let path = std::env::temp_dir().join(format!(
                "aa-material-pixel-{}-{}-{sequence}.png",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            renderer.save_view(&path).unwrap();
            let image = image::open(&path).unwrap().into_rgba8();
            std::fs::remove_file(path).unwrap();
            let result = image.get_pixel(16, 16).0;
            assert_eq!(result[3], 255, "the presented game viewport stays opaque");
            result
        };
        let background = pixel(Vec::new());
        // The original City's ONE/SRC_ALPHA keeps source RGB at alpha zero;
        // source-alpha blending incorrectly makes this surface disappear.
        let city = pixel(vec![plane(
            authored(193, 0, 3, 3),
            [1.0, 0.0, 0.0, 0.0],
            0.0,
        )]);
        assert!(
            city[0] > 240 && city[1] < 5,
            "city blend lost solid RGB: {city:?}"
        );
        let transparent = pixel(vec![plane(
            authored(237, 0, 1, 3),
            [1.0, 0.0, 0.0, 0.0],
            0.0,
        )]);
        assert_eq!(transparent, background);
        // A GREATER threshold of zero really removes a zero-alpha texel.
        let cutout = pixel(vec![plane(
            authored(4844, 0, 3, 3),
            [1.0, 0.0, 0.0, 0.0],
            0.0,
        )]);
        assert_eq!(cutout, background);
        let solid = pixel(vec![plane(
            authored(4844, 150, 3, 3),
            [1.0, 0.0, 0.0, 1.0],
            0.0,
        )]);
        assert!(solid[0] > 240 && solid[1] < 5);
        // Native blended geometry independently chooses depth writes. A near
        // farm surface must occlude a farther surface rendered afterwards.
        let write = pixel(vec![
            plane(authored(4845 | 8192, 150, 3, 3), [1.0, 0.0, 0.0, 1.0], 0.5),
            plane(authored(237 | 8192, 0, 1, 3), [0.0, 1.0, 0.0, 1.0], 0.0),
        ]);
        assert!(
            write[0] > 240 && write[1] < 5,
            "depth-writing blend vanished: {write:?}"
        );
        let no_write = pixel(vec![
            plane(authored(4845 | 8192, 150, 1, 3), [1.0, 0.0, 0.0, 1.0], 0.5),
            plane(authored(237 | 8192, 0, 1, 3), [0.0, 1.0, 0.0, 1.0], 0.0),
        ]);
        assert!(
            no_write[1] > 240 && no_write[0] < 5,
            "read-only depth was ignored: {no_write:?}"
        );
        // Opposite input orders must render the same spatially separated
        // translucent layers. The farther green card is submitted first.
        let sorted = pixel(vec![
            plane(authored(237, 0, 1, 3), [1.0, 0.0, 0.0, 0.5], 0.5),
            plane(authored(237, 0, 1, 3), [0.0, 1.0, 0.0, 0.5], 0.0),
        ]);
        let reversed = pixel(vec![
            plane(authored(237, 0, 1, 3), [0.0, 1.0, 0.0, 0.5], 0.0),
            plane(authored(237, 0, 1, 3), [1.0, 0.0, 0.0, 0.5], 0.5),
        ]);
        assert_eq!(
            sorted, reversed,
            "transparent spatial order depended on input grouping"
        );
        assert!(
            sorted[0] > sorted[1],
            "near red card must overlay farther green: {sorted:?}"
        );
    }

    // Explicit hardware integration test; creates no window and induces no OOM.
    #[test]
    #[ignore = "requires a native graphics adapter; run explicitly"]
    fn gpu_validation_is_scoped_and_failed_upload_preserves_scene() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::DX12 | wgpu::Backends::VULKAN,
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            required_limits: adapter.limits(),
            ..Default::default()
        }))
        .unwrap();
        // An invalid mip count is a deterministic validation failure, not an
        // allocation stress attempt. It must not escape to the panic handler.
        let scope = GpuScope::begin(&device);
        let _invalid = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("deliberately invalid validation fixture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 2,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        assert!(scope.finish("deliberate invalid mip fixture").is_err());
        let egui_renderer = egui_wgpu::Renderer::new(&device, FORMAT, Default::default());
        let state = egui_wgpu::RenderState {
            adapter,
            available_adapters: Vec::new(),
            device,
            queue,
            target_format: FORMAT,
            renderer: Arc::new(egui::mutex::RwLock::new(egui_renderer)),
        };
        let mut renderer = SceneRenderer::new(state).unwrap();
        let mut scene = CpuScene {
            live: None,
            objects: None,
            name: "fixture".into(),
            map_id: "fixture".into(),
            size: [2.0, 2.0],
            draws: vec![CpuDraw {
                vertices: [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, -1.0]]
                    .map(|position| Vertex {
                        terrain_uv: [0.0; 2],
                        position,
                        normal: [0.0, 1.0, 0.0],
                        uv: [0.0; 2],
                        color: [1.0; 4],
                    })
                    .to_vec(),
                indices: vec![0, 1, 2],
                instances: vec![Instance::identity()],
                source_order: None,
                sort_center: None,
                material: CpuMaterial {
                    texture: None,
                    color: [1.0; 4],
                    emissive: [0.0; 3],
                    cutoff: 0.0,
                    blend: false,
                    double_sided: false,
                    team_color_mask: false,
                    native_render_state: None,
                    terrain_layer: false,
                    terrain_mask: None,
                },
            }],
            textures: BTreeMap::new(),
            placements: 1,
            rendered_instances: 1,
            warnings: Vec::new(),
            load_seconds: 0.0,
        };
        renderer.upload(&scene).unwrap();
        let old = (
            renderer.draw_calls,
            renderer.triangles,
            renderer.uploaded_bytes,
        );
        let wide = renderer
            .state
            .device
            .limits()
            .max_texture_dimension_2d
            .checked_add(1)
            .unwrap();
        // One narrow row costs tens of KiB on current devices, yet exceeds the
        // queried dimension capability. No oversized GPU allocation is issued.
        scene.textures.insert(
            "too-wide".into(),
            CpuTexture {
                mips: vec![image::RgbaImage::new(wide, 1)],
            },
        );
        assert!(renderer.upload(&scene).is_err());
        assert_eq!(
            (
                renderer.draw_calls,
                renderer.triangles,
                renderer.uploaded_bytes
            ),
            old
        );
        assert_eq!(renderer.draws.len(), 1);
        renderer.render(&Camera::new([2.0, 2.0]), [32, 32]).unwrap();
        let original_target = renderer.target.as_ref().unwrap().egui_id;
        renderer.render(&Camera::new([2.0, 2.0]), [32, 32]).unwrap();
        assert_eq!(renderer.target.as_ref().unwrap().egui_id, original_target);
        // Exercise the actual GPU vertex-buffer update path, not just CPU pose
        // sampling: moving the triangle out of view must change rendered pixels.
        let camera = Camera::for_model(&aa_assets::Bounds {
            min: [0.0, 0.0, -1.0],
            max: [1.0, 0.0, 0.0],
            size: [1.0, 0.0, 1.0],
        });
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temporary =
            std::env::temp_dir().join(format!("aa-rust-gpu-pose-{}-{stamp}", std::process::id()));
        std::fs::create_dir(&temporary).unwrap();
        let before = temporary.join("before.png");
        let invalid = temporary.join("invalid.png");
        let moved = temporary.join("moved.png");
        renderer.render(&camera, [64, 64]).unwrap();
        renderer.save_view(&before).unwrap();
        let before_rebuilds = renderer.ordering_rebuilds;
        let mut vertices = scene.draws[0].vertices.clone();
        vertices[0].position[0] = f32::NAN;
        assert!(renderer.update_vertices(&[vertices]).is_err());
        assert!(renderer.update_vertices(&[]).is_err());
        renderer.render(&camera, [64, 64]).unwrap();
        assert_eq!(renderer.ordering_rebuilds, before_rebuilds);
        renderer.save_view(&invalid).unwrap();
        assert_eq!(
            std::fs::read(&before).unwrap(),
            std::fs::read(&invalid).unwrap()
        );
        let mut vertices = scene.draws[0].vertices.clone();
        for vertex in &mut vertices {
            vertex.position[0] += 1000.0;
        }
        renderer.update_vertices(&[vertices]).unwrap();
        renderer.render(&camera, [64, 64]).unwrap();
        assert_eq!(
            renderer.ordering_rebuilds,
            before_rebuilds + 1,
            "pose change used stale order"
        );
        renderer.save_view(&moved).unwrap();
        assert_ne!(
            std::fs::read(&before).unwrap(),
            std::fs::read(&moved).unwrap()
        );
        // Alpha zero in an authored RepColor DDS is a full team-color mask,
        // not a transparent texel. Verify the actual shader output is opaque blue.
        scene.textures.clear();
        scene.textures.insert(
            "mask".into(),
            CpuTexture {
                mips: vec![image::RgbaImage::from_pixel(
                    1,
                    1,
                    image::Rgba([255, 0, 0, 0]),
                )],
            },
        );
        scene.draws[0].material.texture = Some("mask".into());
        scene.draws[0].material.team_color_mask = true;
        renderer.team_color = [0.0, 0.0, 1.0];
        renderer.upload(&scene).unwrap();
        renderer.render(&camera, [64, 64]).unwrap();
        let masked = temporary.join("masked.png");
        renderer.save_view(&masked).unwrap();
        let rgba = image::open(&masked).unwrap().to_rgba8();
        assert!(rgba.pixels().all(|p| p[3] == 255));
        assert!(
            rgba.pixels()
                .any(|p| p[2] > p[0].saturating_add(80) && p[2] > p[1].saturating_add(80))
        );
        let cached_rebuilds = renderer.ordering_rebuilds;
        renderer.team_color = [1.0, 0.0, 0.0];
        renderer.render(&camera, [64, 64]).unwrap();
        assert_eq!(
            renderer.ordering_rebuilds, cached_rebuilds,
            "live color uniform invalidated order"
        );
        let recolored = temporary.join("recolored.png");
        renderer.save_view(&recolored).unwrap();
        let red = image::open(&recolored).unwrap().to_rgba8();
        assert_ne!(
            red, rgba,
            "cached bundle did not observe the changed uniform"
        );
        assert!(
            red.pixels()
                .any(|p| p[0] > p[2].saturating_add(80) && p[0] > p[1].saturating_add(80))
        );
        for path in [before, invalid, moved, masked, recolored] {
            std::fs::remove_file(path).unwrap();
        }
        std::fs::remove_dir(temporary).unwrap();
    }
}
