//! NiSourceTexture and NiPixelData.
use super::super::stream::Stream;
use super::super::{Fields, NifError, Value};
use super::scene::{object, object_net, R};

/// NiSourceTexture::LoadBinary 87BFA0 (NiTexture::LoadBinary 8955A0 is
/// NiObjectNET's).
///
/// Below 10.0.1.4 the external flag selects either the file name (and a
/// pushed link count of 0) or an internal flag with an optional pixel-data
/// link; from 10.0.1.4 the flag, the file name and the pixel-data link are
/// always read. The name conversion (8963B0), search path (8D02E0) and the
/// stream texture palette (NiStream+0x194, 8806D0/8806C0, which can make a
/// later object with the same name resolve to an earlier one through the
/// stream's object array, 880A00) read nothing and are not ported.
pub(crate) fn source_texture(s: &mut Stream, f: &mut Fields) -> R {
    object_net(s, f)?;
    if s.below(0x0a00_0104) {
        let external = s.u8()?;
        f.put("use_external", Value::U8(external));
        if external != 0 {
            f.put("file_name", Value::Str(s.cstring()?));
            s.push_count(0);
        } else {
            let internal = s.u8()?;
            f.put("has_pixel_data", Value::U8(internal));
            if internal == 0 {
                s.push_count(0);
            } else {
                s.push_count(1);
                f.put("pixel_data", Value::Link(s.link_id()?));
            }
        }
    } else {
        f.put("use_external", Value::U8(s.u8()?));
        f.put("file_name", Value::Str(s.cstring()?));
        s.push_count(1);
        f.put("pixel_data", Value::Link(s.link_id()?));
    }
    let read = s.u32()?;
    let mut layout = read;
    if s.below(0x0303_000c) {
        if layout == 4 {
            layout = 6;
        }
    } else if s.below(0x0500_0001) && layout == 5 {
        layout = 6;
    }
    if layout == read {
        f.put("pixel_layout", Value::U32(layout));
    } else {
        f.put_converted("pixel_layout", Value::U32(layout), Value::U32(read));
    }
    f.put("use_mipmaps", Value::U32(s.u32()?));
    f.put("alpha_format", Value::U32(s.u32()?));
    let read = s.u8()?;
    f.put_static("is_static", read);
    Ok(())
}

impl Fields {
    /// A byte the loader stores as `!= 0`.
    pub(crate) fn put_static(&mut self, name: &'static str, read: u8) {
        let value = (read != 0) as u8;
        if value == read {
            self.put(name, Value::U8(value));
        } else {
            self.put_converted(name, Value::U8(value), Value::U8(read));
        }
    }
}

/// NiPixelFormat::LoadBinary 869A80.
fn pixel_format(s: &mut Stream, f: &mut Fields) -> R {
    let read = s.u32()?;
    let mut format = read;
    let mut masks = Vec::with_capacity(4);
    for _ in 0..4 {
        masks.push(s.u32()?);
    }
    let bits = s.u32()?;
    let fast = [s.u32()?, s.u32()?];
    let mut tiling = None;
    if s.at_least(0x0a00_010b) {
        tiling = Some(s.u32()?);
    } else if s.version == 0x0a00_0100 && format & 0x10000 != 0 {
        format &= 0xfffe_ffff;
        tiling = Some(1);
    }
    if format == read {
        f.put("format", Value::U32(format));
    } else {
        f.put_converted("format", Value::U32(format), Value::U32(read));
    }
    f.put("color_masks", Value::U32s(masks));
    f.put("bits_per_pixel", Value::U32(bits));
    f.put("compare", Value::U32s(fast.to_vec()));
    if let Some(t) = tiling {
        f.put("tiling", Value::U32(t));
    }
    Ok(())
}

/// NiPixelData::LoadBinary 875D30. The mipmap width/height/offset words are
/// read into 16-entry stack arrays whose last height slot is the return
/// address: more than 15 levels is native stack corruption.
pub(crate) fn pixel_data(s: &mut Stream, f: &mut Fields) -> R {
    object(s, f)?;
    pixel_format(s, f)?;
    f.put("palette", Value::Link(s.link_id()?));
    let levels = s.u32()?;
    f.put("mipmap_levels", Value::U32(levels));
    f.put("bytes_per_pixel", Value::U32(s.u32()?));
    if levels > 15 {
        return Err(NifError::OutOfRange("NiPixelData mipmap level count"));
    }
    let mut words = Vec::with_capacity(levels as usize * 3);
    for _ in 0..levels {
        words.push(s.u32()?);
        words.push(s.u32()?);
        words.push(s.u32()?);
    }
    f.put("mipmaps", Value::U32s(words));
    let total = s.u32()?;
    f.put("total_size", Value::U32(total));
    f.put("pixels", Value::Bytes(s.bytes(total as usize)?));
    Ok(())
}
