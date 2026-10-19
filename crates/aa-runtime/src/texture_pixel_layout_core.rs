//! Original DDS/TGA/PNG reader metadata for the explicitly supported physical
//! source forms. This is encoded/native NiPixelData storage, never RGBA length.
//! Image decompression, stream failures and Source publication remain callers.
type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Reader {
    Dds,
    Tga,
    Png,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Layout {
    pub format: [u32; 9],
    pub widths: Vec<u32>,
    pub heights: Vec<u32>,
    pub offsets: Vec<u32>,
    pub stride: u32,
    pub source_levels: std::ops::Range<usize>,
}

// Actual 9E75C0/9E75F0 ->8696E0 static constructor outputs, not GPU formats.
const RGB: [u32; 9] = [0, 0, 255, 65280, 16711680, 0, 24, 8521824, 16640];
const RGBA: [u32; 9] = [
    1, 0, 255, 65280, 16711680, 4278190080, 32, 545392769, 803072,
];

fn bytes(data: &[u8], offset: usize, size: usize) -> Result<&[u8]> {
    data.get(
        offset
            ..offset
                .checked_add(size)
                .ok_or("native image header offset overflow")?,
    )
    .ok_or_else(|| "actual native image header is truncated".into())
}
fn le(data: &[u8], offset: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        bytes(data, offset, 4)?.try_into().unwrap(),
    ))
}
fn be(data: &[u8], offset: usize) -> Result<u32> {
    Ok(u32::from_be_bytes(
        bytes(data, offset, 4)?.try_into().unwrap(),
    ))
}

///Finite valid image domain of 875AC0. This binding keeps levels below its
///local offset/width array boundary. Invalid huge/zero images get a portable binding error, not claimed
///native allocator/stack-overflow equivalence. All in-domain sizes are exact.
fn allocate(format: [u32; 9], mut w: u32, mut h: u32, levels: u32, first: usize) -> Result<Layout> {
    if w == 0 || h == 0 || levels == 0 || levels > 15 {
        return Err(
            "native PixelData requires positive dimensions and 1..15 levels in this binding".into(),
        );
    }
    let stride = if (4..=6).contains(&format[0]) {
        0
    } else {
        (format[6] & 255) >> 3
    };
    let mut result = Layout {
        format,
        widths: Vec::new(),
        heights: Vec::new(),
        offsets: vec![0],
        stride,
        source_levels: first..first + levels as usize,
    };
    for _ in 0..levels {
        let x = w.max(1);
        let y = h.max(1);
        let size = if (4..=6).contains(&format[0]) {
            //875C31..4A rounds each dimension to4 before multiplying.
            u128::from(x).div_ceil(4)
                * u128::from(y).div_ceil(4)
                * if format[0] == 4 { 8 } else { 16 }
        } else if stride != 0 {
            u128::from(x) * u128::from(y) * u128::from(stride)
        } else {
            u128::from(x) * u128::from(y) / 2
        };
        let next = u128::from(*result.offsets.last().unwrap()) + size;
        let next = u32::try_from(next)
            .map_err(|_| "native PixelData size exceeds finite u32 storage binding")?;
        result.widths.push(x);
        result.heights.push(y);
        result.offsets.push(next);
        w >>= 1;
        h >>= 1;
    }
    Ok(result)
}

///875AC0 for a PixelData created by 87C280 (resize): the source format words
///and level count at the new top-level size.
pub(crate) fn allocate_875ac0(format: [u32; 9], w: u32, h: u32, levels: u32) -> Result<Layout> {
    allocate(format, w, h, levels, 0)
}

pub(crate) fn read(data: &[u8], reader: Reader, mode48: u32) -> Result<Layout> {
    match reader {
        Reader::Dds => dds(data, mode48),
        Reader::Tga => tga(data),
        Reader::Png => png(data),
    }
}

fn dds(data: &[u8], mode48: u32) -> Result<Layout> {
    bytes(data, 0, 128)?;
    if &data[..4] != b"DDS " || le(data, 4)? != 124 || le(data, 8)? & 0x1007 != 0x1007 {
        return Err("native DDS8DDA50 rejects magic/size/required header flags".into());
    }
    let flags = le(data, 8)?;
    if flags & 0x800000 != 0 && le(data, 24)? != 0 {
        return Err("native DDS8DDA50 rejects volume depth".into());
    }
    let pf = le(data, 80)?;
    let format = if pf & 4 != 0 {
        let kind = match bytes(data, 84, 4)? {
            b"DXT1" => 4,
            b"DXT3" => 5,
            b"DXT5" => 6,
            _ => return Err("native DDS8DDA50 rejects this FourCC".into()),
        };
        [kind, 0, 0, 0, 0, 0, 0, kind, 0]
    } else if pf & 0x40 != 0 {
        match le(data, 88)? {
            //8DD990 canonicalizes every accepted32-bit source format toRGBA.
            32 => RGBA,
            16 | 24 => return Err("native DDS16/24-bit conversion metadata requires its separate verified reader domain".into()),
            _ => return Err("native DDS8DD960 rejects this RGB bit count".into()),
        }
    } else {
        return Err("native DDS8DDA50 requires RGB or supported compressed format".into());
    };
    let caps = le(data, 108)?;
    if caps & 8 == 0 && caps & 0x401000 == 0 {
        return Err("native DDS8DDA50 rejects missing texture caps".into());
    }
    if le(data, 112)? & 0x200200 != 0 {
        return Err("native DDS8DDA50 rejects cube/volume caps".into());
    }
    let original = if caps & 8 != 0 && caps & 0x400000 != 0 && le(data, 28)? != 1 {
        le(data, 28)?
    } else {
        1
    };
    let (mut w, mut h, mut levels) = (le(data, 16)?, le(data, 12)?, original);
    //Validate the full allocation that native creates BEFORE quality skipping.
    let _full = allocate(format, w, h, original, 0)?;
    let mut skipped = 0;
    while skipped < mode48 && levels > 1 && w > 4 && h > 4 {
        w >>= 1;
        h >>= 1;
        levels -= 1;
        skipped += 1;
    }
    allocate(format, w, h, levels, skipped as usize)
}

fn tga(data: &[u8]) -> Result<Layout> {
    let h = bytes(data, 0, 18)?;
    if h[1] != 0 || !matches!(h[2], 2 | 10) || !matches!(h[16], 24 | 32) {
        return Err("native TGA binding requires truecolor2/10 with24/32 bits; palette/other depths remain required".into());
    }
    //Actual header reader ignores descriptor alpha width for format selection.
    let w = u16::from_le_bytes(h[12..14].try_into().unwrap()) as u32;
    let height = u16::from_le_bytes(h[14..16].try_into().unwrap()) as u32;
    allocate(if h[16] == 32 { RGBA } else { RGB }, w, height, 1, 0)
}

fn png(data: &[u8]) -> Result<Layout> {
    let h = bytes(data, 0, 33)?;
    //The complete native libpng path validates signature/stream. This binding
    //also validates before exposing a plan; it is not a failed-longjmp model.
    if &h[..8] != b"\x89PNG\r\n\x1a\n" || &h[12..16] != b"IHDR" || be(data, 8)? != 13 {
        return Err("actual PNG signature/IHDR is invalid".into());
    }
    if h[24] != 8 || !matches!(h[25], 0 | 2 | 4 | 6) || h[26] != 0 || h[27] != 0 || h[28] != 0 {
        return Err(
            "native PNG binding requires8-bit nonpalette grayscale/RGB/alpha source".into(),
        );
    }
    let (w, height) = (be(data, 16)?, be(data, 20)?);
    //The native reader retains these as WORDs at4C/4E before Pixel creation.
    //Refuse oversized source dimensions rather than silently truncating a
    //modern decoded image into a differently sized native payload.
    if w > u16::MAX as u32 || height > u16::MAX as u32 {
        return Err("native PNG WORD dimensions exceed this finite source binding".into());
    }
    allocate(RGBA, w, height, 1, 0)
}

#[cfg(test)]
#[path = "texture_pixel_layout_core_tests.rs"]
mod tests;
