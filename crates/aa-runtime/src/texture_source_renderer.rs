//!Native SourceTexture87BA60 and pixel-discard87BBF0 over SAME arena owners.
//!An actual Renderer vf100 child must publish its real RendererData2C before
//!the discard gates reread it. No GPU acceptance or loaded flag is synthesized.
pub type Result<T> = std::result::Result<T, String>;
pub trait Fields {
    type Source: Copy;
    type Renderer: Copy;
    type Pixel: Copy;
    ///Actual retained process GRenderer owner. Missing implementation is an
    ///error; it must never be converted to a fictitious null renderer.
    fn renderer_b442c0(&mut self) -> Result<Option<Self::Renderer>>;
    fn create_renderer_data_100(
        &mut self,
        renderer: Self::Renderer,
        source: Self::Source,
    ) -> Result<bool>;
    fn discard_policy_b43f20(&mut self) -> Result<u8>;
    fn renderer_data_present_2c(&mut self, source: Self::Source) -> Result<bool>;
    fn discard_flag_44(&mut self, source: Self::Source) -> Result<u8>;
    fn pixels_40(&mut self, source: Self::Source) -> Result<Option<Self::Pixel>>;
    ///Native decrement and any reached virtual0 destruction. On failure the
    ///real refcount prefix remains committed, before Source40 is cleared.
    fn release_pixel(&mut self, pixel: Self::Pixel) -> Result<()>;
    fn clear_pixels_40(&mut self, source: Self::Source) -> Result<()>;
}
pub fn create<F: Fields>(fields: &mut F, source: F::Source) -> Result<bool> {
    if let Some(renderer) = fields.renderer_b442c0()? {
        if !fields.create_renderer_data_100(renderer, source)? {
            return Ok(false);
        }
    }
    if fields.discard_policy_b43f20()? != 0
        && fields.renderer_data_present_2c(source)?
        && fields.discard_flag_44(source)? != 0
    {
        discard(fields, source)?;
    }
    Ok(true)
}
pub fn discard<F: Fields>(fields: &mut F, source: F::Source) -> Result<()> {
    if let Some(pixel) = fields.pixels_40(source)? {
        fields.release_pixel(pixel)?;
        fields.clear_pixels_40(source)?;
    }
    Ok(())
}
#[cfg(test)]
#[path = "texture_source_renderer_tests.rs"]
mod tests;
