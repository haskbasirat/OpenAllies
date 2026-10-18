//!Native Texture5A8CD0/5A8B40/5A6A10 field order over one borrowed Resource.
//!Source/Pixel tokens identify SAME-manager owned allocations, never cloned
//!decoded payloads. Native stream, renderer, resize and diagnostics are explicit
//!children; this module grants no Resource/Task/GPU readiness capability.
use aa_numeric::{Binary, Precision};
pub type Result<T> = std::result::Result<T, String>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Word {
    Width54,
    Height58,
    U4C,
    V50,
}
///All methods borrow the actual Resource/Source/Pixel owners. No method has a
///successful default, and all native retain/release operations remain explicit.
pub trait Fields {
    type Source: Copy;
    type Pixel: Copy;
    fn location_count(&mut self) -> Result<u32>;
    fn last_location_writable(&mut self) -> Result<Option<bool>>;
    fn writable_child_present(&mut self) -> Result<bool>;
    fn filename_length(&mut self) -> Result<u32>;
    fn resource_no_ni_textures(&mut self) -> Result<u8>;
    fn flags8(&mut self) -> Result<u32>;
    fn store_flags8(&mut self, flags: u32) -> Result<()>;
    fn loading_diagnostic(&mut self) -> Result<()>;
    fn default_source_5a4d80(&mut self) -> Result<Option<Self::Source>>;
    fn current_source3c(&mut self) -> Result<Option<Self::Source>>;
    fn same_source(&mut self, a: Option<Self::Source>, b: Option<Self::Source>) -> Result<bool>;
    fn missing_direct_warning(&mut self) -> Result<bool>;
    fn assert_missing_direct_855d30(&mut self) -> Result<bool>;
    fn store_missing_direct_warning(&mut self, value: bool) -> Result<()>;
    fn retain_source(&mut self, source: Self::Source) -> Result<()>;
    fn release_source(&mut self, source: Self::Source) -> Result<()>;
    fn reload_direct_44(&mut self, source: Self::Source) -> Result<()>;
    fn create_source_87be50(&mut self) -> Result<Option<Self::Source>>;
    fn force_low_quality(&mut self) -> Result<u8>;
    fn use_high_quality(&mut self) -> Result<u8>;
    fn set_low_quality_3c(&mut self, source: Self::Source, value: u32) -> Result<()>;
    ///Retains the actual selected stream and stack-NiStream wrapper in the
    ///concrete in-flight owner. No decoder-ready boolean substitutes for it.
    fn open_selected_stream_59a9b0(&mut self) -> Result<()>;
    fn load_selected_stream_40(&mut self, source: Self::Source) -> Result<()>;
    ///Runs after output retain, before the local-source release. Actual8761A0
    ///and nonnull BaseStream virtual10 destruction remain ordered children.
    fn close_selected_stream_8761a0(&mut self) -> Result<()>;
    fn missing_source_diagnostic(&mut self) -> Result<()>;
    fn missing_retained_pixels_diagnostic(&mut self) -> Result<()>;
    fn source_pixels(&mut self, source: Self::Source) -> Result<Option<Self::Pixel>>;
    fn set_source_discard44(&mut self, source: Self::Source, value: u8) -> Result<()>;
    ///Actual vf34/38: RendererData2C has priority over PixelData40, then zero.
    fn source_width34(&mut self, source: Self::Source) -> Result<u32>;
    fn source_height38(&mut self, source: Self::Source) -> Result<u32>;
    fn word(&mut self, word: Word) -> Result<u32>;
    fn store_word(&mut self, word: Word, value: u32) -> Result<()>;
    fn memory_counter(&mut self) -> Result<u32>;
    fn store_memory_counter(&mut self, value: u32) -> Result<()>;
    fn filename_is_png(&mut self) -> Result<bool>;
    fn resize_diagnostic(&mut self, original: [u32; 2], target: [u32; 2]) -> Result<()>;
    fn missing_resize_pixels_diagnostic(&mut self) -> Result<()>;
    fn pixel_format_word(&mut self, pixel: Self::Pixel, component: usize) -> Result<u32>;
    fn required_resize_format_word(&mut self, component: usize) -> Result<u32>;
    fn incompatible_resize_format_diagnostic(&mut self) -> Result<()>;
    fn resize_87c5c0(
        &mut self,
        pixel: Self::Pixel,
        target: [u32; 2],
    ) -> Result<Option<Self::Source>>;
    fn oversized_diagnostic(&mut self, target: [u32; 2]) -> Result<()>;
    fn retain_pixel(&mut self, pixel: Self::Pixel) -> Result<()>;
    fn release_pixel(&mut self, pixel: Self::Pixel) -> Result<()>;
    fn pixel_storage_bytes(&mut self, pixel: Self::Pixel) -> Result<u32>;
    fn account_5aa270(&mut self, bytes: u32) -> Result<()>;
    fn render_format_count(&mut self) -> Result<i32>;
    fn render_format_word(&mut self, index: i32, component: usize) -> Result<u32>;
    fn incompatible_render_format_diagnostic(&mut self) -> Result<()>;
    fn pixel_type(&mut self, pixel: Self::Pixel) -> Result<i32>;
    fn pixel_bits_per_pixel(&mut self, pixel: Self::Pixel) -> Result<u8>;
    fn unusual_pixel_format_diagnostic(&mut self) -> Result<()>;
    ///Called only after the old Source has been released successfully. This
    ///writes SAME Entry.source3C; native retain of the new source follows it.
    fn store_source3c(&mut self, source: Option<Self::Source>) -> Result<()>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    NotStarted,
    Location,
    Filename,
    Policy,
    Diagnostic,
    Source,
    Quality,
    Stream,
    Missing,
    KeepPixels,
    Discard,
    Normalize,
    Account,
    Format,
    Publish,
    Release,
    Complete,
}
///Retained in-flight identity tokens and stage, without owning or copying any
///native payload. The concrete manager owns all allocations even on errors.
pub struct State<S, P> {
    stage: Stage,
    source: Option<S>,
    pixel: Option<P>,
}
impl<S: Copy, P: Copy> State<S, P> {
    pub fn new_call() -> Self {
        Self {
            stage: Stage::NotStarted,
            source: None,
            pixel: None,
        }
    }
    pub fn stage(&self) -> Stage {
        self.stage
    }
    pub fn local_source(&self) -> Option<S> {
        self.source
    }
    pub fn local_pixel(&self) -> Option<P> {
        self.pixel
    }
    pub fn load<F: Fields<Source = S, Pixel = P>>(
        &mut self,
        f: &mut F,
        pc: Precision,
    ) -> Result<bool> {
        if self.stage != Stage::NotStarted {
            return Err(
                "Texture load call retains a prior attempt; use the real caller lifecycle".into(),
            );
        }
        self.stage = Stage::Location;
        if f.location_count()? != 0
            && f.last_location_writable()? == Some(true)
            && f.writable_child_present()?
        {
            self.stage = Stage::Complete;
            return Ok(true);
        }
        self.stage = Stage::Filename;
        if f.filename_length()? == 0 {
            self.stage = Stage::Complete;
            return Ok(false);
        }
        self.stage = Stage::Policy;
        if f.resource_no_ni_textures()? == 1 {
            let flags = f.flags8()?;
            f.store_flags8(flags | 0x40)?;
        }
        self.stage = Stage::Diagnostic;
        f.loading_diagnostic()?;
        self.stage = Stage::Source;
        let flags = f.flags8()?;
        if flags & 0x40 != 0 {
            self.source = f.default_source_5a4d80()?;
            if let Some(s) = self.source {
                f.retain_source(s)?;
            }
        } else if flags & 0x10 != 0 {
            if !f.missing_direct_warning()? && f.current_source3c()?.is_none() {
                let value = f.assert_missing_direct_855d30()?;
                f.store_missing_direct_warning(value)?;
            }
            self.source = f.current_source3c()?;
            if let Some(s) = self.source {
                f.retain_source(s)?;
            }
            let s = self
                .source
                .ok_or("5A8E53 direct Texture source is NULL before virtual44")?;
            f.reload_direct_44(s)?;
        } else {
            self.create_5a8b40(f)?;
            //Outer5A4F60 assignment retain, then returned temporary release.
            let s = self.source.unwrap();
            f.retain_source(s)?;
            f.release_source(s)?;
        }
        self.stage = Stage::Missing;
        let Some(source) = self.source else {
            f.missing_source_diagnostic()?;
            self.stage = Stage::Complete;
            return Ok(false);
        };
        self.stage = Stage::KeepPixels;
        if f.flags8()? & (1 << 20) != 0 && f.source_pixels(source)?.is_none() {
            f.missing_retained_pixels_diagnostic()?;
            self.stage = Stage::Release;
            f.release_source(source)?;
            self.source = None;
            self.stage = Stage::Complete;
            return Ok(false);
        }
        self.stage = Stage::Discard;
        f.set_source_discard44(source, 1)?;
        self.stage = Stage::Normalize;
        normalize(f, &mut self.source, pc)?;
        let source = self
            .source
            .ok_or("5A8F8D resized Texture source is NULL before Pixel40")?;
        self.pixel = f.source_pixels(source)?;
        if let Some(pixel) = self.pixel {
            f.retain_pixel(pixel)?;
        }
        self.stage = Stage::Account;
        if let Some(pixel) = self.pixel {
            let bytes = f.pixel_storage_bytes(pixel)?.wrapping_add(0x118);
            f.account_5aa270(bytes)?;
            self.stage = Stage::Format;
            let count = f.render_format_count()?;
            if count != 0 {
                let mut matched = false;
                let mut index = 0;
                while index < count {
                    if f.pixel_format_word(pixel, 0)? == f.render_format_word(index, 0)?
                        && f.pixel_format_word(pixel, 1)? == f.render_format_word(index, 1)?
                    {
                        matched = true;
                        break;
                    }
                    index += 1;
                }
                if !matched {
                    f.incompatible_render_format_diagnostic()?;
                }
            }
            let kind = f.pixel_type(pixel)?;
            if !(4..=6).contains(&kind) && f.pixel_bits_per_pixel(pixel)? != 0x20 {
                f.unusual_pixel_format_diagnostic()?;
            }
        } else {
            let h = f.word(Word::Height58)?;
            let w = f.word(Word::Width54)?;
            f.account_5aa270(h.wrapping_mul(w).wrapping_mul(4).wrapping_add(0x118))?;
        }
        self.stage = Stage::Publish;
        let old = f.current_source3c()?;
        //Identity equality is supplied by SAME private-arena tokens, never payload
        //or filename equality. Kept generic by an explicit equality operation below.
        if !f.same_source(old, Some(source))? {
            if let Some(old) = old {
                f.release_source(old)?;
            }
            f.store_source3c(Some(source))?;
            f.retain_source(source)?;
        }
        self.stage = Stage::Release;
        if let Some(pixel) = self.pixel {
            f.release_pixel(pixel)?;
            self.pixel = None;
        }
        f.release_source(source)?;
        self.source = None;
        self.stage = Stage::Complete;
        Ok(true)
    }
    fn create_5a8b40<F: Fields<Source = S, Pixel = P>>(&mut self, f: &mut F) -> Result<()> {
        self.source = f.create_source_87be50()?;
        if let Some(s) = self.source {
            f.retain_source(s)?;
        }
        self.stage = Stage::Quality;
        let low = f.force_low_quality()? != 0;
        if low || f.use_high_quality()? == 0 {
            f.set_low_quality_3c(
                self.source
                    .ok_or("5A8C30 source is NULL before quality virtual3C")?,
                1,
            )?;
        }
        self.stage = Stage::Stream;
        f.open_selected_stream_59a9b0()?;
        f.load_selected_stream_40(
            self.source
                .ok_or("5A8C57 source is NULL before stream virtual40")?,
        )?;
        //Output retain precedes stream destruction, then local-source release.
        let s = self.source.unwrap();
        f.retain_source(s)?;
        f.close_selected_stream_8761a0()?;
        f.release_source(s)?;
        Ok(())
    }
}
fn next_power(value: u32) -> u32 {
    if value == 0 {
        0
    } else {
        value.checked_next_power_of_two().unwrap_or(0)
    }
}
///5A6A10 writes original54/58 before fallback/rounding/resize. The 1024 test
///is a signed diagnostic, never a size limit or a changed image dimension.
pub fn normalize<F: Fields>(
    f: &mut F,
    source: &mut Option<F::Source>,
    pc: Precision,
) -> Result<()> {
    let s = source.ok_or("5A6A1F Texture source is NULL before width virtual34")?;
    let width = f.source_width34(s)?;
    f.store_word(Word::Width54, width)?;
    let s = source.ok_or("5A6A29 Texture source is NULL before height virtual38")?;
    let height = f.source_height38(s)?;
    let width = f.word(Word::Width54)?;
    f.store_word(Word::Height58, height)?;
    if width == 0 || height == 0 {
        f.store_word(Word::Width54, 16)?;
        f.store_word(Word::Height58, 16)?;
    }
    let w = f.word(Word::Width54)?;
    let pw = next_power(w);
    let h = f.word(Word::Height58)?;
    let ph = next_power(h);
    if w != pw || h != ph {
        let s = source.ok_or("5A6A78 Texture source is NULL before Pixel40")?;
        if f.source_pixels(s)?.is_some() {
            let memory = f.memory_counter()?;
            f.store_memory_counter(
                memory.wrapping_add(
                    ph.wrapping_sub(h)
                        .wrapping_mul(pw.wrapping_sub(w))
                        .wrapping_mul(4),
                ),
            )?;
            if !f.filename_is_png()? {
                f.resize_diagnostic([w, h], [pw, ph])?;
            }
            let s = source.ok_or("5A6B17 Texture source is NULL before repeated Pixel40")?;
            let pixel = f.source_pixels(s)?;
            if pixel.is_none() {
                f.missing_resize_pixels_diagnostic()?;
            }
            let pixel =
                pixel.ok_or("5A6B4A Texture pixels are NULL after missing-pixel diagnostic")?;
            if f.pixel_format_word(pixel, 0)? != f.required_resize_format_word(0)?
                || f.pixel_format_word(pixel, 1)? != f.required_resize_format_word(1)?
            {
                f.incompatible_resize_format_diagnostic()?;
            }
            let new = f.resize_87c5c0(pixel, [pw, ph])?;
            if !f.same_source(*source, new)? {
                if let Some(old) = *source {
                    f.release_source(old)?;
                }
                *source = new;
                if let Some(new) = new {
                    f.retain_source(new)?;
                }
            }
            let wf = f.word(Word::Width54)?;
            f.store_word(Word::U4C, ratio(wf, pw, pc)?)?;
            let hf = f.word(Word::Height58)?;
            f.store_word(Word::V50, ratio(hf, ph, pc)?)?;
        }
    }
    if (pw as i32) > 1024 || (ph as i32) > 1024 {
        f.oversized_diagnostic([pw, ph])?;
    }
    Ok(())
}
fn unsigned(value: u32, pc: Precision) -> Result<Binary> {
    let value = Binary::from_i32(value as i32);
    if value.is_negative_nonzero() {
        value
            .add(
                &Binary::from_f32_bits(0x4f800000).map_err(|e| e.to_string())?,
                pc,
            )
            .map_err(|e| e.to_string())
    } else {
        Ok(value)
    }
}
fn ratio(original: u32, target: u32, pc: Precision) -> Result<u32> {
    let target = Binary::from_i32(target as i32)
        .store_f32()
        .map_err(|e| e.to_string())?;
    unsigned(original, pc)?
        .div(
            &Binary::from_f32_bits(target).map_err(|e| e.to_string())?,
            pc,
        )
        .and_then(|x| x.store_f32())
        .map_err(|e| e.to_string())
}
#[cfg(test)]
#[path = "texture_load_kernel_tests.rs"]
mod tests;
