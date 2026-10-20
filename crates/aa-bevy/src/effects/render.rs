//! Drawing the original effects: every effect quad of a frame (NIF sprites,
//! NiPSys particles, tracers) goes into one camera-sorted mesh drawn with
//! one premultiplied-alpha material over a texture array of the original
//! effect textures, the same scheme as the wgpu client's particles.wgsl
//! (alpha-blended, additive src-alpha/one and one/one particles share the
//! pipeline and the back-to-front order). Projected textures (scorch marks)
//! are terrain-conforming grids in a second mesh drawn before the particles.
//! Colours are the original's gamma-space values; the shader converts them
//! to linear for Bevy's sRGB target.
use bevy::asset::{RenderAssetUsages, uuid_handle};
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError, Extent3d, TextureDataOrder, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension};
use bevy::shader::{Shader, ShaderRef};

pub const SHADER: Handle<Shader> = uuid_handle!("6c2d9a41-58e3-4f0b-a7c6-1e93b04d2f85");
/// Texture array layer size (the client resamples to 128; 256 keeps the
/// larger original explosion sheets sharper).
pub const LAYER_SIZE: u32 = 256;

/// Camera basis for camera-facing quads (render space).
#[derive(Clone, Copy, Debug)]
pub struct View {
    pub eye: Vec3,
    pub right: Vec3,
    pub up: Vec3,
    pub back: Vec3,
    /// World size of one screen pixel at one metre (line widths).
    pub pixel: f32,
}

impl Default for View {
    fn default() -> Self {
        Self { eye: Vec3::new(0.0, 100.0, 0.0), right: Vec3::X, up: Vec3::Y, back: Vec3::Z, pixel: 0.0005 }
    }
}

/// One textured quad: corners in order 00, 10, 01, 11; colour in the
/// original's gamma space with alpha; blend 0 alpha, 1 additive
/// (src alpha, one), 2 additive (one, one).
#[derive(Clone, Copy, Debug)]
pub struct Quad {
    pub corners: [Vec3; 4],
    pub uv: [[f32; 2]; 4],
    pub color: [f32; 4],
    pub layer: u16,
    pub blend: u8,
}

impl Quad {
    /// A camera-facing particle of half-size `size`, rotated in the view plane.
    pub fn billboard(view: &View, pos: Vec3, size: f32, angle: f32, color: Vec4, layer: u16, blend: u8) -> Self {
        let (s, c) = angle.sin_cos();
        let mut corners = [Vec3::ZERO; 4];
        let mut uv = [[0.0; 2]; 4];
        for k in 0..4 {
            let fx = (k & 1) as f32;
            let fy = (k >> 1) as f32;
            let (sx, sy) = (fx * 2.0 - 1.0, fy * 2.0 - 1.0);
            let o = Vec2::new(c * sx - s * sy, s * sx + c * sy) * size;
            corners[k] = pos + view.right * o.x + view.up * o.y;
            uv[k] = [fx, 1.0 - fy];
        }
        Self { corners, uv, color: color.to_array(), layer, blend }
    }
    /// A streak from `tail` to `head`, `width` wide, facing the camera.
    pub fn streak(view: &View, tail: Vec3, head: Vec3, width: f32, color: [f32; 4], layer: u16, blend: u8) -> Self {
        let dir = head - tail;
        let to_cam = (view.eye - head).normalize_or_zero();
        let mut side = dir.cross(to_cam);
        if side.length_squared() < 1e-10 {
            side = view.up;
        }
        let side = side.normalize() * (width * 0.5);
        Self {
            corners: [tail - side, head - side, tail + side, head + side],
            uv: [[0.0, 1.0], [1.0, 1.0], [0.0, 0.0], [1.0, 0.0]],
            color,
            layer,
            blend,
        }
    }
    fn center(&self) -> Vec3 {
        (self.corners[0] + self.corners[3]) * 0.5
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct FxMaterial {
    #[texture(0, dimension = "2d_array")]
    #[sampler(1)]
    pub atlas: Handle<Image>,
    pub depth_bias: f32,
}

impl Material for FxMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(SHADER)
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Premultiplied
    }
    fn depth_bias(&self) -> f32 {
        self.depth_bias
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
    /// Effect quads are two-sided (NIF sprites, streaks, billboards).
    fn specialize(_: &MaterialPipeline, descriptor: &mut RenderPipelineDescriptor, _: &MeshVertexBufferLayoutRef, _: MaterialPipelineKey<Self>) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

pub fn add_render(app: &mut App) {
    app.add_plugins(MaterialPlugin::<FxMaterial>::default());
    let _ = app.world_mut().resource_mut::<Assets<Shader>>().insert(SHADER.id(), Shader::from_wgsl(include_str!("particles.wgsl"), "aa_bevy/effects/particles.wgsl"));
}

/// Build the texture array from decoded RGBA layers (any size; resampled).
pub fn atlas_image(layers: &[image::RgbaImage]) -> Image {
    let n = layers.len().max(2) as u32;
    let mips = LAYER_SIZE.ilog2() + 1;
    let mut data = Vec::new();
    for i in 0..n as usize {
        let base = match layers.get(i) {
            Some(img) if img.width() == LAYER_SIZE && img.height() == LAYER_SIZE => img.clone(),
            Some(img) => image::imageops::resize(img, LAYER_SIZE, LAYER_SIZE, image::imageops::FilterType::Triangle),
            None => image::RgbaImage::from_pixel(LAYER_SIZE, LAYER_SIZE, image::Rgba([255, 255, 255, 255])),
        };
        let mut level = base;
        for m in 0..mips {
            data.extend_from_slice(level.as_raw());
            if m + 1 < mips {
                let s = (LAYER_SIZE >> (m + 1)).max(1);
                level = image::imageops::resize(&level, s, s, image::imageops::FilterType::Triangle);
            }
        }
    }
    let mut image = Image::new_uninit(
        Extent3d { width: LAYER_SIZE, height: LAYER_SIZE, depth_or_array_layers: n },
        TextureDimension::D2,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.data = Some(data);
    image.data_order = TextureDataOrder::LayerMajor;
    image.texture_descriptor.mip_level_count = mips;
    image.texture_view_descriptor = Some(TextureViewDescriptor { dimension: Some(TextureViewDimension::D2Array), ..default() });
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::ClampToEdge,
        address_mode_w: ImageAddressMode::ClampToEdge,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        ..default()
    });
    image
}

/// A dynamic mesh rebuilt every frame from quads.
pub fn empty_mesh() -> Mesh {
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
    write(&mut mesh, &[], Vec3::ZERO, false);
    mesh
}

/// Fill `mesh` with `quads`, sorted back to front from `eye` when `sort`.
pub fn write(mesh: &mut Mesh, quads: &[Quad], eye: Vec3, sort: bool) {
    let mut order: Vec<(f32, u32)> = quads.iter().enumerate().map(|(i, q)| (q.center().distance_squared(eye), i as u32)).collect();
    if sort {
        order.sort_unstable_by(|a, b| b.0.total_cmp(&a.0));
    }
    let n = quads.len().max(1);
    let mut positions = Vec::with_capacity(n * 4);
    let mut uvs = Vec::with_capacity(n * 4);
    let mut params = Vec::with_capacity(n * 4);
    let mut colors = Vec::with_capacity(n * 4);
    let mut indices = Vec::with_capacity(n * 6);
    for &(_, i) in &order {
        let q = &quads[i as usize];
        let base = positions.len() as u32;
        for k in 0..4 {
            positions.push(q.corners[k].to_array());
            uvs.push(q.uv[k]);
            params.push([q.layer as f32, q.blend as f32]);
            colors.push(q.color);
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base + 1, base + 3, base + 2]);
    }
    if positions.is_empty() {
        // A degenerate triangle keeps the vertex layout valid.
        for _ in 0..3 {
            positions.push([0.0, -1.0e4, 0.0]);
            uvs.push([0.0, 0.0]);
            params.push([0.0, 0.0]);
            colors.push([0.0; 4]);
        }
        indices.extend_from_slice(&[0, 1, 2]);
    }
    // A fresh mesh: the extracted one no longer holds its main-world data.
    *mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, params);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
}
