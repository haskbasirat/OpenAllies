//! Deliberate modern renderer vf100 counterpart: upload the actual source's
//! decoded mip levels to the SAME initialized device. This is not a native
//! D3D9 allocation or a Resource load-completion flag.
use super::{camera_device::Reference, GpuScope, SceneRenderer, FORMAT};
use aa_runtime::resource_texture::{PixelData, SourceTexture, SourcePixels, PixelWitness, RendererTexture};
use anyhow::{Context, Result, ensure};

/// Actual resident texture/view and their original source identity. Scene
/// replacement does not destroy this allocation; renderer destruction does
/// invalidate its ownership witness. No public raw/ready constructor exists.
pub struct Uploaded {
    source: SourceTexture,
    pixels: PixelWitness,
    device: Reference,
    allocation: Allocation,
}
struct Allocation {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    dimensions: [u32; 2],
    mip_count: u32,
}
impl Uploaded {
    pub fn source(&self) -> &SourceTexture { &self.source }
    pub fn dimensions(&self) -> [u32; 2] { self.allocation.dimensions }
    pub fn mip_count(&self) -> u32 { self.allocation.mip_count }
    pub fn require_alive(&self) -> Result<()> { self.device.require_alive() }
    pub fn require_for(&self, renderer: &SceneRenderer, source: &SourceTexture) -> Result<()> {
        self.require_alive()?;
        ensure!(self.device.is_for(renderer), "source texture belongs to another renderer allocation");
        ensure!(self.source().same_identity(source), "source texture GPU allocation belongs to another source");
        Ok(())
    }
    pub fn view_for(&self, renderer: &SceneRenderer, source: &SourceTexture) -> Result<&wgpu::TextureView> {
        self.require_for(renderer, source)?; Ok(&self.allocation.view)
    }
}

/// Borrows the pixels; any validation/allocation/upload error leaves the
/// caller's real source and CPU pixels intact for its native failure path.
pub fn upload(renderer: &SceneRenderer, pixels: &SourcePixels) -> Result<Uploaded> {
    let allocation = upload_allocation(renderer, pixels.pixels())?;
    let result = Uploaded { source: pixels.source().clone(), pixels: pixels.witness(), device: renderer.camera_device_reference(), allocation };
    result.require_for(renderer, pixels.source())?;
    Ok(result)
}
impl RendererTexture for Uploaded {
    fn require_source(&self, source: &SourceTexture) -> std::result::Result<(), String> {
        self.require_alive().map_err(|e|e.to_string())?;
        if !self.source().same_identity(source) { return Err("GPU allocation belongs to another SourceTexture".into()) }
        Ok(())
    }
    fn require_pixels(&self, pixels: &SourcePixels) -> std::result::Result<(), String> {
        self.require_source(pixels.source())?;
        if !self.pixels.is_for(pixels) { return Err("GPU allocation belongs to another source pixel allocation".into()) }
        Ok(())
    }
    fn dimensions(&self) -> [u32;2] { self.dimensions() }
}
fn upload_allocation(renderer: &SceneRenderer, pixels: &PixelData) -> Result<Allocation> {
    let (device, queue) = renderer.device_and_queue();
    let levels = pixels.levels();
    let base = levels.first().context("source texture has no decoded image level")?;
    let dimensions = [base.width(), base.height()];
    let mip_count = u32::try_from(levels.len()).context("texture mip count exceeds GPU representation")?;
    ensure!(dimensions.iter().all(|d| *d <= device.limits().max_texture_dimension_2d),
        "source texture dimensions exceed the actual GPU device");
    ensure!(mip_count <= dimensions[0].max(dimensions[1]).ilog2() + 1,
        "source texture mip count exceeds its dimension chain");
    let mut expected = dimensions;
    for level in levels {
        ensure!([level.width(), level.height()] == expected, "source texture mip dimensions are not its actual chain");
        expected = expected.map(|d| (d / 2).max(1));
    }
    let scope = GpuScope::begin(device);
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("native Resource SourceTexture"),
        size: wgpu::Extent3d { width: dimensions[0], height: dimensions[1], depth_or_array_layers: 1 },
        mip_level_count: mip_count, sample_count: 1, dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    for (mip, level) in levels.iter().enumerate() {
        queue.write_texture(wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: mip as u32,
            origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All }, level.pixels(),
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(level.width().checked_mul(4).context("source texture row overflow")?), rows_per_image: Some(level.height()) },
            wgpu::Extent3d { width: level.width(), height: level.height(), depth_or_array_layers: 1 });
    }
    // Submit the queued copies before acknowledging this concrete backend
    // operation. Native Resource flags still belong to its separate loader.
    queue.submit(std::iter::empty());
    let view = texture.create_view(&Default::default());
    scope.finish("source texture creation/upload")?;
    Ok(Allocation { texture, view, dimensions, mip_count })
}

#[cfg(test)]#[path="native_texture_gpu_tests.rs"]mod tests;
