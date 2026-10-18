//!Concrete Texture5A8CD0 field binding to the same ResourceManager allocation.
//!The native kernel owns operation order; this adapter owns no flag/pose copy.
use crate::{
    resource_depot::Resources,
    resource_objects::Handle,
    resource_texture::{SourceTexture, TextureInput},
};
pub type Result<T> = std::result::Result<T, String>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Variable {
    ResourceNoNiTextures,
    ForceLowQualityTextures,
    UseHighQualityTextures,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Diagnostic {
    Loading,
    MissingSource,
    MissingRetainedPixels,
    MissingResizePixels,
    IncompatibleResizeFormat,
    IncompatibleRenderFormat,
    UnusualPixelFormat,
    Resize {
        original: [u32; 2],
        target: [u32; 2],
    },
    Oversized {
        target: [u32; 2],
    },
}
///These callbacks bind current UI/renderer/stream owners. Missing callbacks
///are reached errors, never invented null globals or successful diagnostics.
///They cannot mark Resource/Task readiness; those stores belong to the native
///callers after this same-entry loader returns successfully.
pub trait Backend {
    fn variable_current(&mut self, _variable: Variable) -> Result<u8> {
        Err("actual current Texture UVar allocation is required".into())
    }
    fn diagnostic(&mut self, _resource: &Handle, _event: Diagnostic) -> Result<()> {
        Err("actual Texture diagnostic sink is required".into())
    }
    fn missing_direct_assert_855d30(&mut self, _resource: &Handle) -> Result<bool> {
        Err("855D30 actual direct-Texture assertion service is required".into())
    }
    ///Reads the actual current nullable GRenderer owner, not GPU readiness.
    fn renderer_b442c0_present(&mut self) -> Result<bool> {
        Err("87BA60 actual current GRenderer owner is required".into())
    }
    ///Performs real vf100 and publishes its correlated allocation at SAME
    ///Source2C through Resources::attach_source_renderer_2c before returning.
    fn create_renderer_data_100(
        &mut self,
        _resources: &mut Resources<'_>,
        _source: &SourceTexture,
    ) -> Result<bool> {
        Err("87BA60 actual Renderer vf100 is required".into())
    }
    ///Owns the real selected-stream reader/decode child. The incoming input is
    ///retained on failure; publication requires the separately proved native
    ///encoded PixelData layout, never decoded RGBA byte-length accounting.
    fn load_selected_stream_40(
        &mut self,
        _resources: &mut Resources<'_>,
        _source: &SourceTexture,
        _input: &mut Option<TextureInput>,
    ) -> Result<()> {
        Err(
            "87BB50 actual selected Texture reader and native PixelData metadata are required"
                .into(),
        )
    }
    fn close_selected_stream_8761a0(&mut self, _resource: &Handle) -> Result<()> {
        Err("8761A0 actual selected Texture stream destruction is required".into())
    }
    fn reload_direct_44(
        &mut self,
        _resources: &mut Resources<'_>,
        _source: &SourceTexture,
    ) -> Result<()> {
        Err("actual direct Texture virtual44 reload is required".into())
    }
    fn resize_87c5c0(
        &mut self,
        _resources: &mut Resources<'_>,
        _source: &SourceTexture,
        _target: [u32; 2],
    ) -> Result<Option<SourceTexture>> {
        Err("87C5C0 actual Texture resize allocation is required".into())
    }
}

pub(crate) struct Call {
    pub(crate) state: crate::texture_load_kernel::State<
        crate::resource_texture_owner::SourceId,
        crate::resource_texture_owner::PixelId,
    >,
    pub(crate) stream: Option<TextureInput>,
}
impl Call {
    pub(crate) fn new() -> Self {
        Self {
            state: crate::texture_load_kernel::State::new_call(),
            stream: None,
        }
    }
}

pub(crate) fn load(
    resources: &mut Resources<'_>,
    resource: &Handle,
    pc: aa_numeric::Precision,
    backend: &mut impl Backend,
) -> Result<bool> {
    let mut call =
        resources.with_entry(resource, |entry| {
            let texture = entry
                .texture
                .as_mut()
                .ok_or("Texture vf4C requires actual Texture allocation")?;
            if texture.loading {
                return Err("same Texture vf4C is already borrowed by its actual loader".into());
            }
            if texture.load_call.as_ref().is_some_and(|call| {
                call.state.stage() != crate::texture_load_kernel::Stage::NotStarted
            }) {
                return Err(
                "same Texture retains a prior native load call; actual unload/teardown is required"
                    .into(),
            );
            }
            texture.loading = true;
            Ok(texture.load_call.take().unwrap_or_else(Call::new))
        })?;
    let result = {
        let mut fields = Fields {
            resources,
            resource,
            backend,
            stream: &mut call.stream,
            normalizing_source: None,
        };
        call.state.load(&mut fields, pc)
    };
    let restore = resources.with_entry(resource, |entry| {
        let texture = entry
            .texture
            .as_mut()
            .ok_or("same Texture allocation changed during load")?;
        if !texture.loading || texture.load_call.is_some() {
            return Err("same Texture loader owner changed during callback".into());
        }
        texture.load_call = Some(call);
        texture.loading = false;
        Ok(())
    });
    match (result, restore) {
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error),
        (Ok(value), Ok(())) => Ok(value),
    }
}
struct Fields<'a, 'r, B> {
    resources: &'a mut Resources<'r>,
    resource: &'a Handle,
    backend: &'a mut B,
    stream: &'a mut Option<TextureInput>,
    normalizing_source: Option<crate::resource_texture_owner::SourceId>,
}
impl<B: Backend> Fields<'_, '_, B> {
    fn source(&self, key: crate::resource_texture_owner::SourceId) -> Result<SourceTexture> {
        self.resources.with_texture_objects(|a| a.reference(key))
    }
    fn diagnostic(&mut self, event: Diagnostic) -> Result<()> {
        self.backend.diagnostic(self.resource, event)
    }
    fn create_source(
        &mut self,
        default: bool,
    ) -> Result<Option<crate::resource_texture_owner::SourceId>> {
        let source = self.resources.with_texture_objects(|a| {
            if default {
                a.source_from_default_5a4d80(self.resource)
            } else {
                a.source_from_name(self.resource)
            }
        })?;
        if self
            .resources
            .with_texture_objects(|a| Ok(a.load_on_create()))?
            != 0
        {
            if !crate::texture_source_renderer::create(
                &mut RendererFields { fields: self },
                source,
            )? {
                self.resources
                    .with_texture_objects(|a| a.destroy_source(source))?;
                return Ok(None);
            }
        }
        Ok(Some(source))
    }
}
impl<B: Backend> crate::texture_load_kernel::Fields for Fields<'_, '_, B> {
    type Source = crate::resource_texture_owner::SourceId;
    type Pixel = crate::resource_texture_owner::PixelId;
    fn location_count(&mut self) -> Result<u32> {
        self.resources.with_entry(self.resource, |e| {
            u32::try_from(e.locations.len())
                .map_err(|_| "Texture location count exceeds native word".into())
        })
    }
    fn last_location_writable(&mut self) -> Result<Option<bool>> {
        self.resources.texture_last_location_writable(self.resource)
    }
    fn writable_child_present(&mut self) -> Result<bool> {
        self.resources.with_entry(self.resource, |e| {
            Ok(e.texture
                .as_ref()
                .ok_or("Texture fields absent")?
                .writable_child40
                .is_some())
        })
    }
    fn filename_length(&mut self) -> Result<u32> {
        u32::try_from(self.resource.logical_ids().len())
            .map_err(|_| "Texture filename length exceeds native word".into())
    }
    fn resource_no_ni_textures(&mut self) -> Result<u8> {
        self.backend
            .variable_current(Variable::ResourceNoNiTextures)
    }
    fn flags8(&mut self) -> Result<u32> {
        self.resources.flags_08(self.resource)
    }
    fn store_flags8(&mut self, flags: u32) -> Result<()> {
        self.resources.with_entry(self.resource, |e| {
            e.flags8 = flags;
            Ok(())
        })
    }
    fn loading_diagnostic(&mut self) -> Result<()> {
        self.diagnostic(Diagnostic::Loading)
    }
    fn default_source_5a4d80(&mut self) -> Result<Option<Self::Source>> {
        self.create_source(true)
    }
    fn current_source3c(&mut self) -> Result<Option<Self::Source>> {
        let source = self.resources.texture_5a63d0(self.resource)?;
        source
            .map(|s| self.resources.with_texture_objects(|a| a.key(&s)))
            .transpose()
    }
    fn same_source(&mut self, a: Option<Self::Source>, b: Option<Self::Source>) -> Result<bool> {
        Ok(a == b)
    }
    fn missing_direct_warning(&mut self) -> Result<bool> {
        self.resources
            .with_texture_objects(|a| Ok(a.missing_direct_b3d8f0))
    }
    fn assert_missing_direct_855d30(&mut self) -> Result<bool> {
        self.backend.missing_direct_assert_855d30(self.resource)
    }
    fn store_missing_direct_warning(&mut self, value: bool) -> Result<()> {
        self.resources.with_texture_objects(|a| {
            a.missing_direct_b3d8f0 = value;
            Ok(())
        })
    }
    fn retain_source(&mut self, source: Self::Source) -> Result<()> {
        self.resources
            .with_texture_objects(|a| a.retain_source(source))
    }
    fn release_source(&mut self, source: Self::Source) -> Result<()> {
        self.resources
            .with_texture_objects(|a| a.release_source(source))
    }
    fn reload_direct_44(&mut self, source: Self::Source) -> Result<()> {
        let source = self.source(source)?;
        self.backend.reload_direct_44(self.resources, &source)
    }
    fn create_source_87be50(&mut self) -> Result<Option<Self::Source>> {
        self.create_source(false)
    }
    fn force_low_quality(&mut self) -> Result<u8> {
        self.backend
            .variable_current(Variable::ForceLowQualityTextures)
    }
    fn use_high_quality(&mut self) -> Result<u8> {
        self.backend
            .variable_current(Variable::UseHighQualityTextures)
    }
    fn set_low_quality_3c(&mut self, source: Self::Source, value: u32) -> Result<()> {
        self.resources
            .with_texture_objects(|a| a.set_source_mode(source, value))
    }
    fn open_selected_stream_59a9b0(&mut self) -> Result<()> {
        if self.stream.is_some() {
            return Err("Texture selected stream is already owned by this native call".into());
        }
        *self.stream = Some(self.resources.texture_input_4c(self.resource)?);
        Ok(())
    }
    fn load_selected_stream_40(&mut self, source: Self::Source) -> Result<()> {
        let source = self.source(source)?;
        self.backend
            .load_selected_stream_40(self.resources, &source, self.stream)
    }
    fn close_selected_stream_8761a0(&mut self) -> Result<()> {
        self.backend.close_selected_stream_8761a0(self.resource)?;
        *self.stream = None;
        Ok(())
    }
    fn missing_source_diagnostic(&mut self) -> Result<()> {
        self.diagnostic(Diagnostic::MissingSource)
    }
    fn missing_retained_pixels_diagnostic(&mut self) -> Result<()> {
        self.diagnostic(Diagnostic::MissingRetainedPixels)
    }
    fn source_pixels(&mut self, source: Self::Source) -> Result<Option<Self::Pixel>> {
        self.resources.with_texture_objects(|a| a.pixels(source))
    }
    fn set_source_discard44(&mut self, source: Self::Source, value: u8) -> Result<()> {
        self.resources
            .with_texture_objects(|a| a.set_source_discard(source, value))
    }
    fn source_width34(&mut self, source: Self::Source) -> Result<u32> {
        let width = self
            .resources
            .with_texture_objects(|a| a.source_width(source))?;
        self.normalizing_source = Some(source);
        Ok(width)
    }
    fn source_height38(&mut self, source: Self::Source) -> Result<u32> {
        self.resources
            .with_texture_objects(|a| a.source_height(source))
    }
    fn word(&mut self, word: crate::texture_load_kernel::Word) -> Result<u32> {
        self.resources.with_entry(self.resource, |e| {
            use crate::texture_load_kernel::Word::*;
            let t = e.texture.as_ref().ok_or("Texture fields absent")?;
            match word {
                Width54 => t.word54.ok_or("Texture54 was not initialized".into()),
                Height58 => t.word58.ok_or("Texture58 was not initialized".into()),
                U4C => Ok(t.rect44[2]),
                V50 => Ok(t.rect44[3]),
            }
        })
    }
    fn store_word(&mut self, word: crate::texture_load_kernel::Word, value: u32) -> Result<()> {
        self.resources.with_entry(self.resource, |e| {
            use crate::texture_load_kernel::Word::*;
            let t = e.texture.as_mut().ok_or("Texture fields absent")?;
            match word {
                Width54 => t.word54 = Some(value),
                Height58 => t.word58 = Some(value),
                U4C => t.rect44[2] = value,
                V50 => t.rect44[3] = value,
            };
            Ok(())
        })
    }
    fn memory_counter(&mut self) -> Result<u32> {
        self.resources
            .with_texture_objects(|a| Ok(a.resize_memory_b3d8e4))
    }
    fn store_memory_counter(&mut self, value: u32) -> Result<()> {
        self.resources.with_texture_objects(|a| {
            a.resize_memory_b3d8e4 = value;
            Ok(())
        })
    }
    fn filename_is_png(&mut self) -> Result<bool> {
        Ok(String::from_utf16(self.resource.logical_ids())
            .map_err(|_| "Texture filename is not valid source UTF16")?
            .rsplit('.')
            .next()
            .is_some_and(|s| s.eq_ignore_ascii_case("png")))
    }
    fn resize_diagnostic(&mut self, original: [u32; 2], target: [u32; 2]) -> Result<()> {
        self.diagnostic(Diagnostic::Resize { original, target })
    }
    fn missing_resize_pixels_diagnostic(&mut self) -> Result<()> {
        self.diagnostic(Diagnostic::MissingResizePixels)
    }
    fn pixel_format_word(&mut self, pixel: Self::Pixel, component: usize) -> Result<u32> {
        self.resources
            .with_texture_objects(|a| a.pixel_format(pixel, component))
    }
    fn required_resize_format_word(&mut self, component: usize) -> Result<u32> {
        [1, 0]
            .get(component)
            .copied()
            .ok_or("native default PixelFormat comparison component absent".into())
    }
    fn incompatible_resize_format_diagnostic(&mut self) -> Result<()> {
        self.diagnostic(Diagnostic::IncompatibleResizeFormat)
    }
    fn resize_87c5c0(
        &mut self,
        pixel: Self::Pixel,
        target: [u32; 2],
    ) -> Result<Option<Self::Source>> {
        let source = self
            .normalizing_source
            .ok_or("Texture resize has no current normalizing source identity")?;
        let reference = self.resources.with_texture_objects(|a| {
            if a.pixels(source)? != Some(pixel) {
                return Err(
                    "Texture resize PixelData changed after native normalization reads".into(),
                );
            }
            a.reference(source)
        })?;
        let result = self
            .backend
            .resize_87c5c0(self.resources, &reference, target)?;
        result
            .map(|source| self.resources.with_texture_objects(|a| a.key(&source)))
            .transpose()
    }
    fn oversized_diagnostic(&mut self, target: [u32; 2]) -> Result<()> {
        self.diagnostic(Diagnostic::Oversized { target })
    }
    fn retain_pixel(&mut self, pixel: Self::Pixel) -> Result<()> {
        self.resources
            .with_texture_objects(|a| a.retain_pixel(pixel))
    }
    fn release_pixel(&mut self, pixel: Self::Pixel) -> Result<()> {
        self.resources
            .with_texture_objects(|a| a.release_pixel(pixel))
    }
    fn pixel_storage_bytes(&mut self, pixel: Self::Pixel) -> Result<u32> {
        self.resources
            .with_texture_objects(|a| a.pixel_storage(pixel))
    }
    fn account_5aa270(&mut self, bytes: u32) -> Result<()> {
        self.resources.with_entry(self.resource, |e| {
            e.memory0c = bytes;
            e.memory10 = bytes;
            Ok(())
        })
    }
    fn render_format_count(&mut self) -> Result<i32> {
        self.resources.with_entry(self.resource, |e| {
            i32::try_from(
                e.texture
                    .as_ref()
                    .ok_or("Texture fields absent")?
                    .render_count60,
            )
            .map_err(|_| "Texture render format count exceeds native word".into())
        })
    }
    fn render_format_word(&mut self, index: i32, component: usize) -> Result<u32> {
        self.resources.with_entry(self.resource, |e| {
            let t = e.texture.as_ref().ok_or("Texture fields absent")?;
            let index =
                usize::try_from(index).map_err(|_| "negative native render format index")?;
            t.render_slots
                .get(index)
                .and_then(|v| v.get(component))
                .copied()
                .ok_or("native Texture render-format slot absent".into())
        })
    }
    fn incompatible_render_format_diagnostic(&mut self) -> Result<()> {
        self.diagnostic(Diagnostic::IncompatibleRenderFormat)
    }
    fn pixel_type(&mut self, pixel: Self::Pixel) -> Result<i32> {
        self.resources
            .with_texture_objects(|a| a.pixel_format(pixel, 0).map(|v| v as i32))
    }
    fn pixel_bits_per_pixel(&mut self, pixel: Self::Pixel) -> Result<u8> {
        self.resources
            .with_texture_objects(|a| a.pixel_format(pixel, 6).map(|v| v as u8))
    }
    fn unusual_pixel_format_diagnostic(&mut self) -> Result<()> {
        self.diagnostic(Diagnostic::UnusualPixelFormat)
    }
    fn store_source3c(&mut self, source: Option<Self::Source>) -> Result<()> {
        let source = source.map(|s| self.source(s)).transpose()?;
        self.resources.with_entry(self.resource, |e| {
            e.texture.as_mut().ok_or("Texture fields absent")?.source3c = source;
            Ok(())
        })
    }
}
struct RendererFields<'a, 'b, 'r, B> {
    fields: &'a mut Fields<'b, 'r, B>,
}
impl<B: Backend> crate::texture_source_renderer::Fields for RendererFields<'_, '_, '_, B> {
    type Source = crate::resource_texture_owner::SourceId;
    type Pixel = crate::resource_texture_owner::PixelId;
    type Renderer = ();
    fn renderer_b442c0(&mut self) -> Result<Option<()>> {
        self.fields
            .backend
            .renderer_b442c0_present()
            .map(|v| v.then_some(()))
    }
    fn create_renderer_data_100(&mut self, _: (), source: Self::Source) -> Result<bool> {
        let source = self.fields.source(source)?;
        self.fields
            .backend
            .create_renderer_data_100(self.fields.resources, &source)
    }
    fn discard_policy_b43f20(&mut self) -> Result<u8> {
        self.fields
            .resources
            .with_texture_objects(|a| Ok(a.discard_app_pixels()))
    }
    fn renderer_data_present_2c(&mut self, source: Self::Source) -> Result<bool> {
        self.fields
            .resources
            .with_texture_objects(|a| a.renderer_present(source))
    }
    fn discard_flag_44(&mut self, source: Self::Source) -> Result<u8> {
        self.fields
            .resources
            .with_texture_objects(|a| a.discard_flag(source))
    }
    fn pixels_40(&mut self, source: Self::Source) -> Result<Option<Self::Pixel>> {
        self.fields
            .resources
            .with_texture_objects(|a| a.pixels(source))
    }
    fn release_pixel(&mut self, pixel: Self::Pixel) -> Result<()> {
        self.fields
            .resources
            .with_texture_objects(|a| a.release_pixel(pixel))
    }
    fn clear_pixels_40(&mut self, source: Self::Source) -> Result<()> {
        self.fields
            .resources
            .with_texture_objects(|a| a.clear_pixels(source))
    }
}
