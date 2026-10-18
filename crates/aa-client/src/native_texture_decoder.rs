//! Modern CPU decoding of the SAME selected native Resource file bytes.
//! Source mip levels and alpha are preserved; no substitute texture, resize,
//! generated mip, resource flag, task completion or GPU acknowledgement here.
use aa_runtime::resource_texture::{DecodeFailure, DecodedTexture, Encoding, PixelData, PixelLevel, TextureInput};
use std::io::Cursor;
use image::ImageDecoder;
type Result<T> = std::result::Result<T, String>;

pub fn decode(input: TextureInput) -> std::result::Result<DecodedTexture, DecodeFailure> {
    input.decode_with(decode_bytes)
}
///The native selected-stream owner stays open until its ordered close. This
///shares that SAME immutable input allocation with the decoded result.
pub fn decode_retained(input: &TextureInput) -> Result<DecodedTexture> {
    input.decode_retained(decode_bytes)
}
fn decode_bytes(bytes: &[u8], encoding: Encoding) -> Result<PixelData> {
    if encoding == Encoding::Dds { return decode_dds(bytes); }
    //The actual native TGA reader uses vertical-origin bit5 and pixel depth;
    //it ignores horizontal-origin/declared-alpha bits. Preserve its order
    //and all actual32-bit alpha instead of image's descriptor interpretation.
    let mut native_tga;
    let bytes=if encoding==Encoding::Tga && bytes.len()>=18 {
        native_tga=bytes.to_vec();native_tga[17]=(bytes[17]&32)|if bytes[16]==32{8}else{0};&native_tga[..]
    }else{bytes};
    let format = match encoding { Encoding::Tga => image::ImageFormat::Tga,
        Encoding::Png => image::ImageFormat::Png, Encoding::Dds => unreachable!() };
    let mut reader = image::ImageReader::with_format(Cursor::new(bytes), format);
    // Hardware/checked allocation errors remain errors; the decoder has no
    // artificial image-memory ceiling inherited from image::Limits defaults.
    reader.no_limits();
    let rgba = reader.decode().map_err(|e| e.to_string())?.into_rgba8();
    PixelData::from_decoded_levels(vec![PixelLevel::rgba8(rgba.width(), rgba.height(), rgba.into_raw())?])
}
fn word(bytes: &[u8], offset: usize) -> Result<u32> {
    let data = bytes.get(offset..offset.checked_add(4).ok_or("DDS offset overflow")?)
        .ok_or("truncated DDS header")?;
    Ok(u32::from_le_bytes(data.try_into().unwrap()))
}
fn storage(width: u32, height: u32, stride: u32) -> Result<usize> {
    let n = u64::from(width).checked_mul(u64::from(height)).and_then(|v|v.checked_mul(u64::from(stride)))
        .ok_or("decoded texture allocation overflow")?;
    usize::try_from(n).map_err(|_|"decoded texture exceeds addressable allocation".into())
}
fn zeros(size: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new(); bytes.try_reserve_exact(size).map_err(|_|"decoded texture allocation failed")?;
    bytes.resize(size, 0); Ok(bytes)
}
fn decode_dds(bytes: &[u8]) -> Result<PixelData> {
    if bytes.get(..4) != Some(b"DDS ") || word(bytes,4)? != 124 || word(bytes,76)? != 32 {
        return Err("invalid original DDS header".into());
    }
    let mut width=word(bytes,16)?; let mut height=word(bytes,12)?;
    if width==0 || height==0 { return Err("DDS dimensions are zero".into()); }
    // Neither a cubemap/volume nor a modern DX10 array can be represented by
    // this two-dimensional native Texture payload. Never silently pick a face.
    if word(bytes,112)? & (0x200|0x20_0000) != 0 || word(bytes,24)? > 1 {
        return Err("DDS cube/volume source requires its actual multidimensional reader".into());
    }
    let caps=word(bytes,108)?;
    let declared=word(bytes,28)?;
    //Native8DDA50 admits a mip chain only with BOTH complex/mipmap caps.
    let count=if caps&8!=0 && caps&0x400000!=0 && declared!=1{declared}else{1};
    if count==0 {return Err("DDS native mip chain is empty".into())}
    if count>width.max(height).ilog2()+1 {return Err("DDS mip count exceeds its dimension chain".into());}
    let pf=word(bytes,80)?; let fourcc=bytes.get(84..88).ok_or("truncated DDS pixel format")?;
    let compressed=pf&4!=0;
    let block_size=if compressed {match fourcc {b"DXT1"=>8,b"DXT3"|b"DXT5"=>16,
        _=>return Err("DDS compression requires an unbound native reader".into())}} else {0};
    if !compressed && (pf&0x40==0 || word(bytes,88)?!=32) {
        return Err("DDS uncompressed source requires its actual non-RGBA32 reader".into());
    }
    let masks=if compressed {[0;4]} else {[word(bytes,92)?,word(bytes,96)?,word(bytes,100)?,if pf&1!=0{word(bytes,104)?}else{0}]};
    if !compressed && masks.iter().enumerate().any(|(i,&m)| (i<3&&m==0)||(m!=0 && m>>m.trailing_zeros()!=255)) {
        return Err("DDS channel mask requires an unbound non-eight-bit reader".into());
    }
    let mut levels=Vec::new(); let mut offset=128usize;
    for _level in 0..count {
        let rgba=if compressed {
            let bx=width.div_ceil(4); let by=height.div_ceil(4);
            let size=storage(bx,by,block_size)?;
            let end=offset.checked_add(size).ok_or("DDS mip extent overflow")?;
            let data=bytes.get(offset..end).ok_or("truncated DDS compressed mip chain")?;
            offset=end; decode_blocks(data,width,height,fourcc)?
        } else {
            let tight=width.checked_mul(4).ok_or("DDS row overflow")?;
            //Native8DE7F0 reads tightly packed levels and ignores DDS header
            //pitch. The same original byte offsets must drive our pixels.
            let pitch=tight;
            let size=storage(pitch,height,1)?; let end=offset.checked_add(size).ok_or("DDS mip extent overflow")?;
            let data=bytes.get(offset..end).ok_or("truncated DDS RGBA mip chain")?; offset=end;
            let mut rgba=zeros(storage(width,height,4)?)?;
            for y in 0..height as usize {for x in 0..width as usize {
                let p=u32::from_le_bytes(data[y*pitch as usize+x*4..y*pitch as usize+x*4+4].try_into().unwrap());
                for c in 0..4 {rgba[(y*width as usize+x)*4+c]=if masks[c]==0 {255}else{((p&masks[c])>>masks[c].trailing_zeros())as u8};}
            }} rgba
        };
        levels.push(PixelLevel::rgba8(width,height,rgba)?);
        width=(width/2).max(1); height=(height/2).max(1);
    }
    PixelData::from_decoded_levels(levels)
}
// The public DDS reader is used for each real compressed source level.
fn decode_blocks(bytes: &[u8], width: u32, height: u32, fourcc: &[u8]) -> Result<Vec<u8>> {
    use image::codecs::dds::DdsDecoder;
    let pw=width.div_ceil(4).checked_mul(4).ok_or("DDS padded width overflow")?;
    let ph=height.div_ceil(4).checked_mul(4).ok_or("DDS padded height overflow")?;
    //A scratch one-level container exposes this exact encoded level to the
    //public codec. Its header is not the source provenance/size metadata.
    let mut framed=zeros(128usize.checked_add(bytes.len()).ok_or("DDS scratch extent overflow")?)?;
    framed[..4].copy_from_slice(b"DDS ");
    for(offset,value)in[(4,124u32),(8,0x1007),(12,ph),(16,pw),(28,1),(76,32),(80,4),(108,0x1000)]{
        framed[offset..offset+4].copy_from_slice(&value.to_le_bytes());
    }
    framed[84..88].copy_from_slice(fourcc);framed[128..].copy_from_slice(bytes);
    let decoder=DdsDecoder::new(Cursor::new(framed)).map_err(|e|e.to_string())?;
    let channels=if fourcc==b"DXT1" {3}else{4};
    let mut padded=zeros(usize::try_from(decoder.total_bytes()).map_err(|_|"DXT decoded size overflow")?)?;
    decoder.read_image(&mut padded).map_err(|e|e.to_string())?;
    let mut rgba=zeros(storage(width,height,4)?)?;
    for y in 0..height as usize {for x in 0..width as usize {
        let from=(y*pw as usize+x)*channels;let to=(y*width as usize+x)*4;
        rgba[to..to+3].copy_from_slice(&padded[from..from+3]);
        rgba[to+3]=if channels==4 {padded[from+3]} else {
            // image's DXT1 codec produces RGB; restore the format's actual
            // one-bit alpha from block selector3 in the three-colour mode.
            let block=((y/4)*(pw as usize/4)+x/4)*8;
            let c0=u16::from_le_bytes(bytes[block..block+2].try_into().unwrap());
            let c1=u16::from_le_bytes(bytes[block+2..block+4].try_into().unwrap());
            let selectors=u32::from_le_bytes(bytes[block+4..block+8].try_into().unwrap());
            if c0<=c1 && (selectors>>(2*((y%4)*4+x%4)))&3==3 {0}else{255}
        };
    }} Ok(rgba)
}

#[cfg(test)]#[path="native_texture_decoder_tests.rs"]mod tests;
#[cfg(test)]#[path="native_texture_pixel_tests.rs"]mod native_pixel_tests;
