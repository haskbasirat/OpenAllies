//! Concrete original base/overlay samplers for a world's required height leaf.
//! Shape ownership, tile updates and notifications remain required world logic.
use crate::{HeightLayer, HeightMethod, Result};
use aa_heightfield::Heightfield;
use aa_numeric::{Binary, Precision};

pub struct HeightLayers<'a> {
    base: &'a Heightfield,
    overlay: &'a Heightfield,
}
impl<'a> HeightLayers<'a> {
    pub fn new(base: &'a Heightfield, overlay: &'a Heightfield) -> Result<Self> {
        base.validate()?;
        overlay.validate()?;
        if (
            base.width,
            base.height,
            base.coordinate_shift,
            base.cell_size_bits,
            base.index_bias,
        ) != (
            overlay.width,
            overlay.height,
            overlay.coordinate_shift,
            overlay.cell_size_bits,
            overlay.index_bias,
        ) {
            return Err(aa_heightfield::Error::LayoutMismatch.into());
        }
        Ok(Self { base, overlay })
    }
    /// Preserve the sampler's extended return; query_actor_height determines the
    /// native binary32 stores and conditional overlay arithmetic. No guessed
    /// interpolation, combined-height cache or implicit FPU precision is used.
    pub fn sample(
        &self,
        layer: HeightLayer,
        method: HeightMethod,
        position_bits: [u32; 2],
        precision: Precision,
    ) -> Result<Binary> {
        let field = match layer {
            HeightLayer::Base => self.base,
            HeightLayer::Overlay => self.overlay,
        };
        Ok(match method {
            HeightMethod::Nearest => field.sample_nearest(position_bits)?,
            HeightMethod::Interpolated => field.sample_interpolated(position_bits, precision)?,
        })
    }
}
