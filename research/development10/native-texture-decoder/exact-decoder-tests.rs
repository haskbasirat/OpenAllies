use std::io::Cursor;use image::ImageDecoder;type Result<T>=std::result::Result<T,String>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    Tga,
    Dds,
    Png,
}

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



fn dds(width:u32,height:u32,mips:u32,fourcc:&[u8;4],data:&[u8])->Vec<u8>{
 let mut bytes=vec![0;128];bytes[..4].copy_from_slice(b"DDS ");
 for(o,v)in[(4,124u32),(8,0x1007),(12,height),(16,width),(28,mips),(76,32),(80,4),(108,if mips>1{0x401008}else{0x1000})]{bytes[o..o+4].copy_from_slice(&v.to_le_bytes())}
 bytes[84..88].copy_from_slice(fourcc);bytes.extend_from_slice(data);bytes
}
#[test]fn dxt1_alpha_and_actual_sub_block_mip_levels_survive(){
 let mut payload=Vec::new();payload.extend_from_slice(&[0,0,255,255,255,255,255,255]);
 payload.extend_from_slice(&[0,248,31,0,0,0,0,0]);payload.extend_from_slice(&[224,7,0,0,0,0,0,0]);
 let bytes=dds(4,4,3,b"DXT1",&payload);let pixels=decode_bytes(&bytes,Encoding::Dds).unwrap();
 assert_eq!(pixels.levels().iter().map(|m|(m.width(),m.height())).collect::<Vec<_>>(),[(4,4),(2,2),(1,1)]);
 assert!(pixels.levels()[0].pixels().chunks_exact(4).all(|p|p==[0,0,0,0]));
 assert!(pixels.levels()[1].pixels().chunks_exact(4).all(|p|p==[255,0,0,255]));
 assert_eq!(pixels.levels()[2].pixels(),[0,255,0,255]);
}
#[test]fn dxt3_explicit_alpha_and_dxt5_interpolated_alpha_survive(){
 let mut d3=vec![0x10,0x32,0x54,0x76,0x98,0xba,0xdc,0xfe];d3.extend_from_slice(&[0,248,31,0,0,0,0,0]);
 let pixels=decode_bytes(&dds(4,4,1,b"DXT3",&d3),Encoding::Dds).unwrap();
 assert_eq!(pixels.levels()[0].pixels().chunks_exact(4).map(|p|p[3]).collect::<Vec<_>>(),(0u8..16).map(|n|n*17).collect::<Vec<_>>());
 let mut d5=vec![128,64,2,0,0,0,0,0];d5.extend_from_slice(&[0,248,31,0,0,0,0,0]);
 let pixels=decode_bytes(&dds(4,4,1,b"DXT5",&d5),Encoding::Dds).unwrap();assert_eq!(pixels.levels()[0].pixels()[3],118);
}
#[test]fn rgba_dds_masks_and_native_ignored_header_pitch_preserve_channels(){
 let mut bytes=dds(1,2,1,b"NONE",&[3,2,1,4,9,9,9,9,7,6,5,8,9,9,9,9]);
 for(o,v)in[(8,0x100fu32),(20,8),(80,0x41),(88,32),(92,0xff0000),(96,0xff00),(100,0xff),(104,0xff000000)]{bytes[o..o+4].copy_from_slice(&v.to_le_bytes())}
 bytes[84..88].fill(0);let pixels=decode_bytes(&bytes,Encoding::Dds).unwrap();assert_eq!(pixels.levels()[0].pixels(),[1,2,3,4,9,9,9,9]);
 assert!(decode_bytes(&bytes[..135],Encoding::Dds).is_err());
}
#[test]fn tga_origin_and_rle_preserve_original_alpha(){
 let mut bytes=vec![0;18];bytes[2]=2;bytes[12]=2;bytes[14]=2;bytes[16]=32;bytes[17]=8;
 bytes.extend_from_slice(&[255,0,0,32,255,255,255,64,0,0,255,255,0,255,0,128]);
 let pixels=decode_bytes(&bytes,Encoding::Tga).unwrap();assert_eq!(pixels.levels()[0].pixels(),[255,0,0,255,0,255,0,128,0,0,255,32,255,255,255,64]);
 let mut rle=vec![0;18];rle[2]=10;rle[12]=1;rle[14]=2;rle[16]=32;rle[17]=8;rle.extend_from_slice(&[0x81,255,0,0,128]);
 assert_eq!(decode_bytes(&rle,Encoding::Tga).unwrap().levels()[0].pixels(),[0,0,255,128,0,0,255,128]);
}
#[test]fn truncated_and_multidimensional_dds_never_substitute_an_image(){
 let mut bytes=dds(4,4,1,b"DXT1",&[0;8]);assert!(decode_bytes(&bytes[..135],Encoding::Dds).is_err());
 bytes[112..116].copy_from_slice(&0x200u32.to_le_bytes());assert!(decode_bytes(&bytes,Encoding::Dds).is_err());
}


fn hex(s:&str)->Vec<u8>{s.as_bytes().chunks_exact(2).map(|v|u8::from_str_radix(std::str::from_utf8(v).unwrap(),16).unwrap()).collect()}
#[test]
fn defined_native_reader_pixels_match_actual_decoder(){
    let data:serde_json::Value=serde_json::from_str(include_str!("E:\\Games\\Axis and Alies\\rust-rebuild\\research\\skirmish-20261017-texture-reader\\native-reader-pixels.json")).unwrap();
    let mut count=0;
    for row in data["rows"].as_array().unwrap(){
        let bytes=hex(row["input"]["bytes"].as_str().unwrap());
        let encoding=match row["input"]["extension"].as_str().unwrap(){"dds"=>Encoding::Dds,"tga"=>Encoding::Tga,"png"=>Encoding::Png,_=>panic!("fixture reader")};
        //The old grayscale PNG reader has allocator-dependent pixel reads.
        //Its metadata is separately proved; no native gray-pixel parity claim.
        if encoding==Encoding::Png && matches!(bytes[25],0|4){continue}
        let decoded=decode_bytes(&bytes,encoding).unwrap_or_else(|error|panic!("{}: {error}",row["input"]["label"]));
        assert_eq!(decoded.levels()[0].pixels(),hex(row["output"]["normalized_rgba"].as_str().unwrap()),"{} / {}",row["precision"],row["input"]["label"]);
        count+=1;
    }
    assert_eq!(count,90);
}
