//! Source-identity-bound native image storage/mip plan. This carries no load,
//! Source+3C, Resource flag or GPU completion authority.
use crate::texture_pixel_layout_core::{self as core, Layout, Reader};
use crate::{
    resource_objects::Handle,
    resource_texture::{DecodedTexture, Encoding, TextureInput},
};
type Result<T> = std::result::Result<T, String>;

///Validated from one actual selected Resource input and the reached current
///Source+48. The private native format/byte offsets remain independent of the
///decoder's RGBA payload. This object cannot be synthesized from decoded size.
pub struct NativePixelLayout {
    handle: Handle,
    sha256: String,
    source_mode48: u32,
    layout: Layout,
}
impl NativePixelLayout {
    pub fn from_input(input: &TextureInput, source_mode48: u32) -> Result<Self> {
        let reader = match input.encoding() {
            Encoding::Dds => Reader::Dds,
            Encoding::Tga => Reader::Tga,
            Encoding::Png => Reader::Png,
        };
        let layout = core::read(input.bytes(), reader, source_mode48)?;
        Ok(Self {
            handle: input.handle().clone(),
            sha256: input.sha256().into(),
            source_mode48,
            layout,
        })
    }
    pub fn require_input(&self, input: &TextureInput, current_source_mode48: u32) -> Result<()> {
        self.require_identity(input.handle(), input.sha256(), current_source_mode48)
    }
    pub fn require_decoded(
        &self,
        decoded: &DecodedTexture,
        current_source_mode48: u32,
    ) -> Result<()> {
        self.require_identity(
            decoded.handle(),
            decoded.source_sha256(),
            current_source_mode48,
        )
    }
    fn require_identity(&self, handle: &Handle, sha256: &str, mode48: u32) -> Result<()> {
        if !self.handle.same_identity(handle)
            || self.sha256 != sha256
            || self.source_mode48 != mode48
        {
            return Err(
                "native PixelData plan belongs to another Resource/input/current Source+48".into(),
            );
        }
        Ok(())
    }
    pub fn source_mode48(&self) -> u32 {
        self.source_mode48
    }
    ///Original physical levels to select from the decoder's retained chain.
    ///No new mips, resize or quality defaults are supplied here.
    pub fn source_levels(&self) -> std::ops::Range<usize> {
        self.layout.source_levels.clone()
    }
    pub(crate) fn native_fields(&self) -> ([u32; 9], &[u32], &[u32], &[u32], u32) {
        (
            self.layout.format,
            &self.layout.widths,
            &self.layout.heights,
            &self.layout.offsets,
            self.layout.stride,
        )
    }
}
