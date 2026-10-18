//!Sole growable same-Application NiSourceTexture/PixelData field owner.
//!Resource source3C and environment154 hold identity references into it.
use crate::{
    resource_objects::Handle,
    resource_texture::{
        DecodedTexture, PixelData, PixelLevel, RendererTexture, SourcePixels, SourceTexture,
    },
};
use std::sync::{Arc, Weak};
type Result<T> = std::result::Result<T, String>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SourceId(usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PixelId(usize);
///Actual immutable native reader metadata. RGBA-decoded byte count is never
///substituted for the native encoded allocation/format words.
pub(crate) struct PixelLayout {
    pub(crate) format: [u32; 9],
    pub(crate) widths: Vec<u32>,
    pub(crate) heights: Vec<u32>,
    pub(crate) offsets: Vec<u32>,
    pub(crate) stride: u32,
}
#[cfg(test)]
mod tests {
    use super::*;
    fn laboratory_resource(origin: &Arc<()>) -> Handle {
        let source =
            crate::resource_objects::SourceFile::from_catalog(&aa_rules::sources::CatalogEntry {
                logical_path: "native-default-field-fixture.tga".into(),
                size: 0,
                origin: aa_rules::sources::Origin::V14Loose,
            });
        //This is an explicit native field fixture, not an admitted file/read owner.
        Handle::new(
            origin,
            crate::resource_objects::Kind::Texture,
            "native-default-field-fixture.tga".encode_utf16().collect(),
            source,
        )
    }
    #[test]
    fn exact_native_default_pixels_and_fresh_source_allocations_all_pc() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261017-texture-load-kernel/native-default-texture.json"
        ))
        .unwrap();
        for row in fixture["rows"].as_array().unwrap() {
            if row["input"]["renderer_mode"] != 0 {
                continue;
            } //separate concrete87BA60 proof
            let origin = Arc::new(());
            let resource = laboratory_resource(&origin);
            let mut owner = State::from_fresh_application(&origin);
            let mask = row["input"]["color_bits"].as_u64().unwrap() as u32;
            owner.colors_mask = mask;
            for i in 0..4 {
                if mask & (1 << i) != 0 {
                    owner.colors[i] = Some(0x11223340 + i as u32)
                }
            }
            let mut keys = Vec::new();
            for _ in 0..row["input"]["calls"].as_u64().unwrap() {
                keys.push(owner.source_from_default_5a4d80(&resource).unwrap());
            }
            assert_eq!(
                serde_json::json!(owner.colors_mask),
                row["output"]["color_bits"]
            );
            assert_eq!(
                serde_json::json!(owner.colors.map(Option::unwrap)),
                row["output"]["colors"]
            );
            for (i, key) in keys.iter().copied().enumerate() {
                let pixel = owner.pixels(key).unwrap().unwrap();
                let p = owner.pixel(pixel).unwrap();
                let expected = &row["output"]["pixels"][i];
                assert_eq!(serde_json::json!(p.refs4), expected["refs"]);
                assert_eq!(serde_json::json!(p.layout.format), expected["format"]);
                assert_eq!(serde_json::json!(p.layout.widths), expected["widths"]);
                assert_eq!(serde_json::json!(p.layout.heights), expected["heights"]);
                assert_eq!(serde_json::json!(p.layout.offsets), expected["offsets"]);
                assert_eq!(serde_json::json!(p.layout.stride), expected["stride"]);
                assert_eq!(
                    serde_json::json!(p.payload.data().levels()[0].pixels()),
                    expected["rgba"]
                );
                let source = owner.source(key).unwrap();
                let expected = &row["output"]["sources"][i];
                assert_eq!(serde_json::json!(source.refs4), expected["refs"]);
                assert_eq!(serde_json::json!(source.prefs), expected["preferences"]);
                assert_eq!(serde_json::json!(source.discard44), expected["discard"]);
                assert_eq!(serde_json::json!(source.mode48), expected["quality"]);
                assert_eq!(source.previous, i.checked_sub(1).map(|n| keys[n]));
                assert_eq!(source.next, keys.get(i + 1).copied());
                assert!(source.filename38.is_none());
                assert!(source.renderer2c.is_none());
            }
            assert!(
                !owner
                    .reference(keys[0])
                    .unwrap()
                    .same_identity(&owner.reference(keys[1]).unwrap())
            );
        }
    }
    #[test]
    fn weak_pixel_witness_allows_actual_cpu_discard_and_rejects_other_source() {
        let origin = Arc::new(());
        let resource = laboratory_resource(&origin);
        let mut owner = State::from_fresh_application(&origin);
        let first = owner.source_from_default_5a4d80(&resource).unwrap();
        let second = owner.source_from_default_5a4d80(&resource).unwrap();
        let pixels = owner.pixels_for_renderer(first).unwrap().unwrap();
        let witness = pixels.witness();
        let weak = Arc::downgrade(&pixels.data);
        assert!(witness.is_for(&pixels));
        assert!(!witness.is_for(&owner.pixels_for_renderer(second).unwrap().unwrap()));
        let foreign = State::from_fresh_application(&Arc::new(()));
        assert!(foreign.key(pixels.source()).is_err());
        let id = owner.pixels(first).unwrap().unwrap();
        drop(pixels);
        owner.release_pixel(id).unwrap();
        owner.clear_pixels(first).unwrap();
        assert!(
            weak.upgrade().is_none(),
            "GPU identity witness must not retain CPU pixels"
        );
        assert!(owner.pixels_for_renderer(first).unwrap().is_none());
        assert!(owner.pixel_refs(id).is_err());
        //Actual constructor-native Source refcounts drive retirement; weak public
        //identity references are not a second native ownership count.
        let reference = owner.reference(first).unwrap();
        owner.retain_source(first).unwrap();
        owner.release_source(first).unwrap();
        assert!(owner.key(&reference).is_err());
        assert_eq!(owner.first, Some(second));
        assert_eq!(owner.source(second).unwrap().previous, None);
    }
}
enum Payload {
    Default(Arc<PixelData>),
    Decoded(Arc<DecodedTexture>),
}
impl Payload {
    fn data(&self) -> &PixelData {
        match self {
            Self::Default(v) => v,
            Self::Decoded(v) => v.pixels(),
        }
    }
    fn owner(&self) -> Arc<PixelData> {
        match self {
            Self::Default(v) => Arc::clone(v),
            Self::Decoded(v) => v.pixels_owner(),
        }
    }
}
struct Pixel {
    refs4: u32,
    payload: Payload,
    layout: PixelLayout,
}
struct Source {
    refs4: u32,
    pixel40: Option<PixelId>,
    discard44: u8,
    mode48: u32,
    prefs: [u32; 3],
    filename38: Option<Vec<u16>>,
    resource: Handle,
    previous: Option<SourceId>,
    next: Option<SourceId>,
    renderer2c: Option<Arc<dyn RendererTexture>>,
    failed_reader40: Vec<DecodedTexture>,
}
pub(crate) struct State {
    application: Weak<()>,
    sources: Vec<Option<Source>>,
    pixels: Vec<Option<Pixel>>,
    first: Option<SourceId>,
    last: Option<SourceId>,
    colors_mask: u32,
    colors: [Option<u32>; 4],
    load_on_create_b323b0: u8,
    discard_app_pixels_b43f20: u8,
    pub(crate) resize_memory_b3d8e4: u32,
    pub(crate) missing_direct_b3d8f0: bool,
}
impl State {
    pub(crate) fn from_fresh_application(application: &Arc<()>) -> Self {
        //B323B0's PE byte is1. B3D8E0/B3D8E4/B3D8F0/B43F20 are fresh BSS0.
        //Only the real renderer-init mapping may later execute710433's store0.
        Self {
            application: Arc::downgrade(application),
            sources: Vec::new(),
            pixels: Vec::new(),
            first: None,
            last: None,
            colors_mask: 0,
            colors: [None; 4],
            load_on_create_b323b0: 1,
            discard_app_pixels_b43f20: 0,
            resize_memory_b3d8e4: 0,
            missing_direct_b3d8f0: false,
        }
    }
    pub(crate) fn renderer_init_710433(&mut self) {
        self.load_on_create_b323b0 = 0
    }
    pub(crate) fn load_on_create(&self) -> u8 {
        self.load_on_create_b323b0
    }
    pub(crate) fn discard_app_pixels(&self) -> u8 {
        self.discard_app_pixels_b43f20
    }
    pub(crate) fn renderer_present(&self, key: SourceId) -> Result<bool> {
        Ok(self.source(key)?.renderer2c.is_some())
    }
    pub(crate) fn discard_flag(&self, key: SourceId) -> Result<u8> {
        Ok(self.source(key)?.discard44)
    }
    pub(crate) fn source_mode(&self, key: SourceId) -> Result<u32> {
        Ok(self.source(key)?.mode48)
    }
    pub(crate) fn clear_pixels(&mut self, key: SourceId) -> Result<()> {
        self.source_mut(key)?.pixel40 = None;
        Ok(())
    }
    fn source(&self, key: SourceId) -> Result<&Source> {
        self.sources
            .get(key.0)
            .and_then(Option::as_ref)
            .ok_or_else(|| "NiSourceTexture identity was retired/foreign".into())
    }
    fn source_mut(&mut self, key: SourceId) -> Result<&mut Source> {
        self.sources
            .get_mut(key.0)
            .and_then(Option::as_mut)
            .ok_or_else(|| "NiSourceTexture identity was retired/foreign".into())
    }
    fn pixel(&self, key: PixelId) -> Result<&Pixel> {
        self.pixels
            .get(key.0)
            .and_then(Option::as_ref)
            .ok_or_else(|| "NiPixelData identity was retired/foreign".into())
    }
    fn pixel_mut(&mut self, key: PixelId) -> Result<&mut Pixel> {
        self.pixels
            .get_mut(key.0)
            .and_then(Option::as_mut)
            .ok_or_else(|| "NiPixelData identity was retired/foreign".into())
    }
    pub(crate) fn reference(&self, key: SourceId) -> Result<SourceTexture> {
        self.source(key)?;
        Ok(SourceTexture {
            application: self.application.clone(),
            id: key,
        })
    }
    pub(crate) fn key(&self, reference: &SourceTexture) -> Result<SourceId> {
        if !Weak::ptr_eq(&self.application, &reference.application)
            || self.application.upgrade().is_none()
        {
            return Err("NiSourceTexture belongs to retired/different Application".into());
        }
        self.source(reference.id)?;
        Ok(reference.id)
    }
    ///87BDD0→87B930 actual constructor-null fields and global list append.
    ///The source payload is never copied when links or Resource fields change.
    fn allocate_source(
        &mut self,
        resource: &Handle,
        filename38: Option<Vec<u16>>,
        pixel40: Option<PixelId>,
    ) -> Result<SourceId> {
        if let Some(pixel) = pixel40 {
            self.retain_pixel(pixel)?
        }
        let key = SourceId(self.sources.len());
        if let Some(last) = self.last {
            self.source_mut(last)?.next = Some(key)
        } else {
            self.first = Some(key)
        }
        self.sources.push(Some(Source {
            refs4: 0,
            pixel40,
            discard44: 1,
            mode48: 0,
            prefs: [6, 2, 3],
            filename38,
            resource: resource.clone(),
            previous: self.last,
            next: None,
            renderer2c: None,
            failed_reader40: Vec::new(),
        }));
        self.last = Some(key);
        Ok(key)
    }
    ///Fresh87BE50 object prefix, before its current process/renderer child.
    pub(crate) fn source_from_name(&mut self, resource: &Handle) -> Result<SourceId> {
        self.allocate_source(resource, Some(resource.logical_ids().to_vec()), None)
    }
    ///Full proven5A4D80 PixelData allocation/writes plus87BF00 ctor prefix.
    ///The caller still MUST perform current process/renderer4C before returning
    ///this source from the factory. SourceRefs0/PixelRefs1 are native states.
    pub(crate) fn source_from_default_5a4d80(&mut self, resource: &Handle) -> Result<SourceId> {
        for (i, value) in [0xff0000ff, 0xff00ff00, 0xffff0000, 0xffffffff]
            .into_iter()
            .enumerate()
        {
            if self.colors_mask & (1 << i) == 0 {
                self.colors_mask |= 1 << i;
                self.colors[i] = Some(value)
            }
        }
        let bytes = self
            .colors
            .iter()
            .flat_map(|v| v.expect("reached color writer").to_le_bytes())
            .collect();
        let data = PixelData::from_decoded_levels(vec![PixelLevel::rgba8(2, 2, bytes)?])?;
        let key = PixelId(self.pixels.len());
        self.pixels.push(Some(Pixel {
            refs4: 0,
            payload: Payload::Default(Arc::new(data)),
            layout: PixelLayout {
                format: [
                    1, 0, 255, 65280, 16711680, 4278190080, 32, 0x20820881, 0xc4100,
                ],
                widths: vec![2],
                heights: vec![2],
                offsets: vec![0, 16],
                stride: 4,
            },
        }));
        self.allocate_source(resource, None, Some(key))
    }
    pub(crate) fn retain_source(&mut self, key: SourceId) -> Result<()> {
        let s = self.source_mut(key)?;
        s.refs4 = s.refs4.wrapping_add(1);
        Ok(())
    }
    pub(crate) fn release_source(&mut self, key: SourceId) -> Result<()> {
        let s = self.source_mut(key)?;
        s.refs4 = s.refs4.wrapping_sub(1);
        if s.refs4 == 0 {
            self.destroy_source(key)?
        }
        Ok(())
    }
    pub(crate) fn destroy_source(&mut self, key: SourceId) -> Result<()> {
        let row = self
            .sources
            .get_mut(key.0)
            .and_then(Option::take)
            .ok_or("NiSourceTexture destruction identity absent")?;
        if let Some(previous) = row.previous {
            self.source_mut(previous)?.next = row.next
        } else {
            self.first = row.next
        }
        if let Some(next) = row.next {
            self.source_mut(next)?.previous = row.previous
        } else {
            self.last = row.previous
        }
        if let Some(pixel) = row.pixel40 {
            self.release_pixel(pixel)?
        }
        Ok(())
    }
    pub(crate) fn retain_pixel(&mut self, key: PixelId) -> Result<()> {
        let p = self.pixel_mut(key)?;
        p.refs4 = p.refs4.wrapping_add(1);
        Ok(())
    }
    pub(crate) fn release_pixel(&mut self, key: PixelId) -> Result<()> {
        let p = self.pixel_mut(key)?;
        p.refs4 = p.refs4.wrapping_sub(1);
        if p.refs4 == 0 {
            self.pixels[key.0] = None
        }
        Ok(())
    }
    pub(crate) fn pixels(&self, key: SourceId) -> Result<Option<PixelId>> {
        Ok(self.source(key)?.pixel40)
    }
    pub(crate) fn source_width(&self, key: SourceId) -> Result<u32> {
        if let Some(renderer) = &self.source(key)?.renderer2c {
            renderer.require_source(&self.reference(key)?)?;
            return Ok(renderer.dimensions()[0]);
        }
        let Some(pixel) = self.pixels(key)? else {
            return Ok(0);
        };
        self.pixel(pixel)?
            .layout
            .widths
            .first()
            .copied()
            .ok_or_else(|| "NiPixelData width array missing".into())
    }
    pub(crate) fn source_height(&self, key: SourceId) -> Result<u32> {
        if let Some(renderer) = &self.source(key)?.renderer2c {
            renderer.require_source(&self.reference(key)?)?;
            return Ok(renderer.dimensions()[1]);
        }
        let Some(pixel) = self.pixels(key)? else {
            return Ok(0);
        };
        self.pixel(pixel)?
            .layout
            .heights
            .first()
            .copied()
            .ok_or_else(|| "NiPixelData height array missing".into())
    }
    pub(crate) fn set_source_discard(&mut self, key: SourceId, value: u8) -> Result<()> {
        self.source_mut(key)?.discard44 = value;
        Ok(())
    }
    pub(crate) fn set_source_mode(&mut self, key: SourceId, value: u32) -> Result<()> {
        self.source_mut(key)?.mode48 = value;
        Ok(())
    }
    pub(crate) fn resource(&self, key: SourceId) -> Result<&Handle> {
        Ok(&self.source(key)?.resource)
    }
    pub(crate) fn pixel_format(&self, key: PixelId, index: usize) -> Result<u32> {
        self.pixel(key)?
            .layout
            .format
            .get(index)
            .copied()
            .ok_or_else(|| "NiPixelData format word absent".into())
    }
    pub(crate) fn pixel_storage(&self, key: PixelId) -> Result<u32> {
        self.pixel(key)?
            .layout
            .offsets
            .last()
            .copied()
            .ok_or_else(|| "NiPixelData total offset absent".into())
    }
    pub(crate) fn pixel_data(&self, key: PixelId) -> Result<&PixelData> {
        Ok(self.pixel(key)?.payload.data())
    }
    pub(crate) fn pixels_for_renderer(&self, source: SourceId) -> Result<Option<SourcePixels>> {
        let Some(pixel) = self.pixels(source)? else {
            return Ok(None);
        };
        Ok(Some(SourcePixels {
            source: self.reference(source)?,
            pixel,
            data: self.pixel(pixel)?.payload.owner(),
        }))
    }
    pub(crate) fn attach_renderer(
        &mut self,
        pixels: &SourcePixels,
        renderer: Arc<dyn RendererTexture>,
    ) -> Result<()> {
        let source = self.key(&pixels.source)?;
        if self.pixels(source)? != Some(pixels.pixel)
            || !Arc::ptr_eq(&self.pixel(pixels.pixel)?.payload.owner(), &pixels.data)
        {
            return Err("GPU Texture upload belongs to replaced native PixelData".into());
        }
        renderer.require_source(&pixels.source)?;
        renderer.require_pixels(pixels)?;
        self.source_mut(source)?.renderer2c = Some(renderer);
        Ok(())
    }
    pub(crate) fn source_refs(&self, key: SourceId) -> Result<u32> {
        Ok(self.source(key)?.refs4)
    }
    pub(crate) fn pixel_refs(&self, key: PixelId) -> Result<u32> {
        Ok(self.pixel(key)?.refs4)
    }
    ///The real parser-derived native layout is required independently of the
    ///modern RGBA buffer. It may not be guessed from decompressed byte length.
    pub(crate) fn store_decoded(
        &mut self,
        key: SourceId,
        decoded: DecodedTexture,
        layout: PixelLayout,
    ) -> std::result::Result<(), crate::resource_texture::DecodedPublicationFailure> {
        let check = (|| -> Result<()> {
            if !decoded.handle().same_identity(self.resource(key)?) {
                return Err("decoded Texture came from a different Resource allocation".into());
            }
            if layout.widths.len() != layout.heights.len()
                || layout.offsets.len() != layout.widths.len() + 1
                || layout.widths.is_empty()
            {
                return Err("native PixelData layout arrays disagree".into());
            }
            if let Some(old) = self.pixels(key)? {
                self.pixel(old)?;
            }
            Ok(())
        })();
        if let Err(error) = check {
            return Err(crate::resource_texture::DecodedPublicationFailure { decoded, error });
        }
        let pixel = PixelId(self.pixels.len());
        self.pixels.push(Some(Pixel {
            refs4: 0,
            payload: Payload::Decoded(Arc::new(decoded)),
            layout,
        }));
        let old = self
            .source(key)
            .expect("checked same source allocation")
            .pixel40;
        if let Some(old) = old {
            self.release_pixel(old)
                .expect("checked same old pixel allocation")
        }
        self.source_mut(key)
            .expect("checked same source allocation")
            .pixel40 = Some(pixel);
        self.retain_pixel(pixel)
            .expect("same newly allocated pixel");
        Ok(())
    }
    pub(crate) fn retain_failed_decoded(
        &mut self,
        key: SourceId,
        decoded: DecodedTexture,
    ) -> std::result::Result<(), crate::resource_texture::DecodedPublicationFailure> {
        let check = self.resource(key).and_then(|resource| {
            if resource.same_identity(decoded.handle()) {
                Ok(())
            } else {
                Err("failed decoded Texture belongs to another native source Resource".into())
            }
        });
        if let Err(error) = check {
            return Err(crate::resource_texture::DecodedPublicationFailure { decoded, error });
        }
        self.source_mut(key)
            .expect("checked same source allocation")
            .failed_reader40
            .push(decoded);
        Ok(())
    }
}
