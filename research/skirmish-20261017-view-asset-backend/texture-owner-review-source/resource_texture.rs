//!Actual selected-depot Texture vf4C input and modern decoded pixel payload.
//!These immutable owners cannot set Resource flags, Task completion, source3C
//!or GPU readiness. The actual same Resource loader performs those later stores.
use crate::resource_objects::{Handle, Kind, SourceFile};
use std::sync::{Arc, Weak};
type Result<T> = std::result::Result<T, String>;
///Identity-only reference to one actual manager-owned NiSourceTexture. The
///source arena is its sole mutable field/pixel owner; this type has no raw
///constructor and cloning it never certifies a loaded resource or GPU state.
#[derive(Clone)]
pub struct SourceTexture {
    pub(crate) application: Weak<()>,
    pub(crate) id: crate::resource_texture_owner::SourceId,
}
///Implemented by the actual modern renderer's sealed device/pipeline owner.
///This is a deliberate backend mapping of the reached710433 store, not a
///whole7101A0 parity or VWorld completion claim.
pub trait RendererInitialization {
    fn require_current_backend(&self) -> Result<()>;
}
///Actual retained modern texture allocation at Source+2C. Implementations own
///the real device texture/view and validate their private input correlation.
///These methods cannot write Resource flags or Task completion.
pub trait RendererTexture: Send + Sync {
    fn require_source(&self, source: &SourceTexture) -> Result<()>;
    fn require_pixels(&self, pixels: &SourcePixels) -> Result<()>;
    fn dimensions(&self) -> [u32; 2];
}
///Identity-only snapshot of the SAME immutable pixel allocation for a real
///GPU upload. Private source/pixel keys prevent attaching a result to another
///or subsequently replaced source. Cloning shares the payload allocation.
#[derive(Clone)]
pub struct SourcePixels {
    pub(crate) source: SourceTexture,
    pub(crate) pixel: crate::resource_texture_owner::PixelId,
    pub(crate) data: Arc<PixelData>,
}
impl SourcePixels {
    pub fn source(&self) -> &SourceTexture {
        &self.source
    }
    pub fn pixels(&self) -> &PixelData {
        &self.data
    }
    pub fn same_identity(&self, other: &Self) -> bool {
        self.source.same_identity(&other.source)
            && self.pixel == other.pixel
            && Arc::ptr_eq(&self.data, &other.data)
    }
    ///Keeps only allocation identity after upload. Retaining this witness does
    ///not keep CPU image storage alive after native87BBF0 discards it.
    pub fn witness(&self) -> PixelWitness {
        PixelWitness {
            source: self.source.clone(),
            pixel: self.pixel,
            data: Arc::downgrade(&self.data),
        }
    }
}
///Weak correlation of one actual source/pixel allocation. A GPU payload may
///retain this after upload without preventing the native CPU discard path.
#[derive(Clone)]
pub struct PixelWitness {
    source: SourceTexture,
    pixel: crate::resource_texture_owner::PixelId,
    data: Weak<PixelData>,
}
impl PixelWitness {
    pub fn is_for(&self, pixels: &SourcePixels) -> bool {
        self.source.same_identity(pixels.source())
            && self.pixel == pixels.pixel
            && Weak::ptr_eq(&self.data, &Arc::downgrade(&pixels.data))
    }
    pub fn source(&self) -> &SourceTexture {
        &self.source
    }
}
impl SourceTexture {
    pub fn same_identity(&self, other: &Self) -> bool {
        self.id == other.id && Weak::ptr_eq(&self.application, &other.application)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    Tga,
    Dds,
    Png,
}
impl Encoding {
    fn from_source(source: &SourceFile) -> Result<Self> {
        let name = source.logical_path().to_ascii_lowercase();
        match name.rsplit('.').next() {
            Some("tga") => Ok(Self::Tga),
            Some("dds") => Ok(Self::Dds),
            Some("png") => Ok(Self::Png),
            _ => Err("actual selected Texture source requires an unbound image reader".into()),
        }
    }
}
///One actual decoded image level. Construction checks storage shape, not
///native load completion. No resize, substitute image or mip generation occurs.
pub struct PixelLevel {
    width: u32,
    height: u32,
    rgba8: Vec<u8>,
}
impl PixelLevel {
    pub fn rgba8(width: u32, height: u32, rgba8: Vec<u8>) -> Result<Self> {
        if width == 0 || height == 0 {
            return Err("decoded Texture has zero dimensions".into());
        }
        let size = u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|n| n.checked_mul(4))
            .ok_or("decoded Texture size overflow")?;
        if usize::try_from(size).ok() != Some(rgba8.len()) {
            return Err("decoded Texture RGBA storage differs from actual dimensions".into());
        }
        Ok(Self {
            width,
            height,
            rgba8,
        })
    }
    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }
    pub fn pixels(&self) -> &[u8] {
        &self.rgba8
    }
}
pub struct PixelData {
    levels: Vec<PixelLevel>,
}
impl PixelData {
    ///The decoder supplies only levels actually decoded from the source.
    ///Generating later native/modern mip levels is a separate policy operation.
    pub fn from_decoded_levels(levels: Vec<PixelLevel>) -> Result<Self> {
        if levels.is_empty() {
            return Err("decoded Texture has no image level".into());
        }
        Ok(Self { levels })
    }
    pub fn levels(&self) -> &[PixelLevel] {
        &self.levels
    }
}
struct Input {
    handle: Handle,
    source: SourceFile,
    bytes: Arc<[u8]>,
    sha256: String,
    encoding: Encoding,
}
///Move-only actual V14Sources read. Only Resources can issue one after checking
///the same manager allocation and retained selected depot identity.
pub struct TextureInput {
    input: Arc<Input>,
}
impl TextureInput {
    pub(crate) fn from_selected(
        handle: Handle,
        file: aa_rules::sources::ResolvedFile,
    ) -> Result<Self> {
        if handle.kind() != Kind::Texture {
            return Err("Texture input requires actual Texture Resource class".into());
        }
        let source = handle
            .source_file()
            .ok_or("generated default Texture has no physical source input")?
            .clone();
        if source.logical_path() != file.logical_path
            || source.origin() != &file.origin
            || source.size() != file.bytes.len() as u64
        {
            return Err("Texture input actual selected source differs".into());
        }
        let encoding = Encoding::from_source(&source)?;
        let sha256 = aa_rules::sources::sha256(&file.bytes);
        Ok(Self {
            input: Arc::new(Input {
                handle,
                source,
                bytes: file.bytes.into(),
                sha256,
                encoding,
            }),
        })
    }
    pub fn handle(&self) -> &Handle {
        &self.input.handle
    }
    pub fn source(&self) -> &SourceFile {
        &self.input.source
    }
    pub fn bytes(&self) -> &[u8] {
        &self.input.bytes
    }
    pub fn sha256(&self) -> &str {
        &self.input.sha256
    }
    pub fn encoding(&self) -> Encoding {
        self.input.encoding
    }
    ///Decodes this same retained stream input without consuming its native
    ///call owner. The ordered8761A0 boundary still closes that owner later.
    pub fn decode_retained(
        &self,
        decode: impl FnOnce(&[u8], Encoding) -> Result<PixelData>,
    ) -> Result<DecodedTexture> {
        let pixels = decode(self.bytes(), self.encoding())?;
        Ok(DecodedTexture {
            input: Arc::clone(&self.input),
            pixels: Arc::new(pixels),
            native_selection: None,
        })
    }
    ///Runs the concrete decoder over exactly these bytes. An error returns the
    ///same input owner; no partially completed native Resource is discarded.
    pub fn decode_with(
        self,
        decode: impl FnOnce(&[u8], Encoding) -> Result<PixelData>,
    ) -> std::result::Result<DecodedTexture, DecodeFailure> {
        match decode(self.bytes(), self.encoding()) {
            Ok(pixels) => Ok(DecodedTexture {
                input: self.input,
                pixels: Arc::new(pixels),
                native_selection: None,
            }),
            Err(error) => Err(DecodeFailure { input: self, error }),
        }
    }
}
pub struct DecodeFailure {
    pub input: TextureInput,
    pub error: String,
}
///Real decoded payload correlated to the same input, with no raw constructor
///or load/readiness booleans. A later loader may publish this same object.
pub struct DecodedTexture {
    input: Arc<Input>,
    pixels: Arc<PixelData>,
    native_selection: Option<(u32, std::ops::Range<usize>)>,
}
pub struct NativeLayoutFailure {
    pub decoded: DecodedTexture,
    pub error: String,
}
pub struct DecodedPublicationFailure {
    pub decoded: DecodedTexture,
    pub error: String,
}
impl DecodedTexture {
    pub fn handle(&self) -> &Handle {
        &self.input.handle
    }
    pub fn source(&self) -> &SourceFile {
        &self.input.source
    }
    pub fn source_sha256(&self) -> &str {
        &self.input.sha256
    }
    pub fn source_bytes(&self) -> &[u8] {
        &self.input.bytes
    }
    pub fn pixels(&self) -> &PixelData {
        &self.pixels
    }
    ///Selects exactly the physical mip span produced by the native encoded
    ///reader at its current source quality mode. No resize or mip creation.
    ///An error returns this SAME decoded owner without discarding its pixels.
    pub fn apply_native_layout(
        mut self,
        layout: &crate::texture_pixel_layout::NativePixelLayout,
    ) -> std::result::Result<Self, NativeLayoutFailure> {
        let check = (|| -> Result<()> {
            layout.require_decoded(&self, layout.source_mode48())?;
            if self.native_selection.is_some() {
                return Err("decoded Texture already has a native mip selection".into());
            }
            let range = layout.source_levels();
            let (_, widths, heights, _, _) = layout.native_fields();
            let levels = self
                .pixels
                .levels
                .get(range.clone())
                .ok_or("native selected mip span is absent from actual decoder payload")?;
            if levels.len() != widths.len()
                || levels.len() != heights.len()
                || levels
                    .iter()
                    .zip(widths.iter().zip(heights))
                    .any(|(level, (&w, &h))| level.width() != w || level.height() != h)
            {
                return Err(
                    "native selected mip dimensions differ from actual decoder payload".into(),
                );
            }
            if Arc::get_mut(&mut self.pixels).is_none() {
                return Err(
                    "native mip selection requires the same unpublished decoded allocation".into(),
                );
            }
            Ok(())
        })();
        if let Err(error) = check {
            return Err(NativeLayoutFailure {
                decoded: self,
                error,
            });
        }
        let range = layout.source_levels();
        let levels = &mut Arc::get_mut(&mut self.pixels)
            .expect("checked same unpublished allocation")
            .levels;
        levels.drain(..range.start);
        levels.truncate(range.len());
        self.native_selection = Some((layout.source_mode48(), range));
        Ok(self)
    }
    pub(crate) fn require_native_selection(
        &self,
        layout: &crate::texture_pixel_layout::NativePixelLayout,
        current_mode: u32,
    ) -> Result<()> {
        layout.require_decoded(self, current_mode)?;
        if self.native_selection.as_ref() != Some(&(current_mode, layout.source_levels())) {
            return Err("decoded Texture has no matching native physical mip selection".into());
        }
        Ok(())
    }
    pub(crate) fn pixels_owner(&self) -> Arc<PixelData> {
        Arc::clone(&self.pixels)
    }
}
