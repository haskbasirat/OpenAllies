//! Images embedded in original NIF effects (NiSourceTexture with internal
//! NiPixelData), e.g. the frames of the HealthBar / MoraleBar status-bar
//! VFX (Data/Effects/HealthBar/HealthBar.NIF: an NiFlipController cycling
//! health01..health11), read with aa-formats' port of NiStream.
use aa_formats::nif::{self, Value};

/// One decoded image: (width, height, RGBA8).
pub type Rgba = (u32, u32, Vec<u8>);

fn u32s<'a>(f: &'a nif::Fields, name: &str) -> Option<&'a [u32]> {
    match f.get(name)? {
        Value::U32s(v) => Some(v),
        _ => None,
    }
}

fn bytes<'a>(f: &'a nif::Fields, name: &str) -> Option<&'a [u8]> {
    match f.get(name)? {
        Value::Bytes(v) => Some(v),
        _ => None,
    }
}

/// Decode an NiPixelData's first mipmap level (RGB24 / RGBA32 with the
/// format's colour masks).
fn decode_pixels(f: &nif::Fields) -> Option<Rgba> {
    let format = f.u32("format")?;
    let bpp = f.u32("bits_per_pixel")?;
    let masks = u32s(f, "color_masks")?;
    let mips = u32s(f, "mipmaps")?;
    let (w, h, off) = (*mips.first()?, *mips.get(1)?, *mips.get(2)? as usize);
    let px = bytes(f, "pixels")?;
    let step = (bpp / 8) as usize;
    if !(format == 0 || format == 1) || !(step == 3 || step == 4) {
        return None;
    }
    let channel = |v: u32, mask: u32| -> u8 {
        if mask == 0 {
            return 255;
        }
        let shift = mask.trailing_zeros();
        let max = mask >> shift;
        (((v & mask) >> shift) * 255 / max.max(1)) as u8
    };
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for i in 0..(w * h) as usize {
        let at = off + i * step;
        let raw = px.get(at..at + step)?;
        let v = if step == 4 { u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]) } else { u32::from_le_bytes([raw[0], raw[1], raw[2], 0]) };
        out.extend([channel(v, masks[0]), channel(v, masks[1]), channel(v, masks[2]), if step == 4 { channel(v, masks[3]) } else { 255 }]);
    }
    Some((w, h, out))
}

/// The frames an NIF's NiFlipController cycles, in source order; falls
/// back to every embedded texture in stream order.
pub fn flip_frames(data: &[u8]) -> Vec<Rgba> {
    let Ok(file) = nif::load(data) else { return Vec::new() };
    let texture = |i: usize| -> Option<Rgba> {
        let src = file.objects.get(i)?;
        let link = src.fields.link("pixel_data")?;
        decode_pixels(&file.object(link)?.fields)
    };
    for o in &file.objects {
        if o.is("NiFlipController") {
            if let Some(links) = o.fields.links("sources") {
                let frames: Vec<Rgba> = links.iter().filter_map(|&l| file.target(l)).filter_map(texture).collect();
                if !frames.is_empty() {
                    return frames;
                }
            }
        }
    }
    (0..file.objects.len()).filter(|&i| file.objects[i].is("NiSourceTexture")).filter_map(texture).collect()
}

/// The span of an NIF's geometry (every NiTriShapeData / NiTriStripsData
/// vertex, scaled by the object scales): (width along x, height along the
/// larger of y / z), metres.
pub fn extent(data: &[u8]) -> Option<(f32, f32)> {
    let file = nif::load(data).ok()?;
    let scale = file.objects.iter().filter_map(|o| o.fields.f32("scale")).map(nif::f32_value).filter(|s| *s > 0.0).fold(1.0f32, |a, s| a * s);
    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    for o in &file.objects {
        let Some(v) = o.fields.f32s("vertices") else { continue };
        for p in v.chunks_exact(3) {
            for i in 0..3 {
                let x = nif::f32_value(p[i]);
                lo[i] = lo[i].min(x);
                hi[i] = hi[i].max(x);
            }
        }
    }
    if lo[0] > hi[0] {
        return None;
    }
    let w = (hi[0] - lo[0]) * scale;
    let h = (hi[1] - lo[1]).max(hi[2] - lo[2]) * scale;
    Some((w, h))
}
