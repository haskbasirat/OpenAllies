//!Exact scalar helpers for405030 and5A5410/5A99C0 memory accounting.
type Result<T>=std::result::Result<T,String>;
pub(crate) fn class_weights(current:Option<i32>)->Result<(i32,i32)>{let value=current.unwrap_or(10);if value<=0{return Err("Model cache weight outside proved positive constructor domain".into())}Ok((value,10))}
///405030 stops atNUL, uppercases onlyASCIIa-z and wraps a33-fold UTF16 sum.
pub(crate) fn filename_hash(units:&[u16])->u32{units.iter().copied().take_while(|v|*v!=0).fold(0u32,|h,c|h.wrapping_mul(33).wrapping_add(u32::from(if(0x61..=0x7a).contains(&c){c-0x20}else{c})))}
///5A546D FILD32; negative values add binary32 2^32 at current PC before
///the FMUL0.2f and96CC4C truncate. No binary32 spill is inserted.
pub(crate) fn weighted_size(bytes:u32,pc:aa_numeric::Precision)->Result<u32>{
 weighted_size_constant(bytes,0x3e4ccccd,pc)
}
pub(crate) fn weighted_size_constant(bytes:u32,factor:u32,pc:aa_numeric::Precision)->Result<u32>{
 use aa_numeric::Binary;
 let mut size=Binary::from_i32(bytes as i32);
 if (bytes as i32)<0{size=size.add(&Binary::from_f32_bits(0x4f800000).map_err(|e|e.to_string())?,pc).map_err(|e|e.to_string())?;}
 let value=size.mul(&Binary::from_f32_bits(factor).map_err(|e|e.to_string())?,pc).map_err(|e|e.to_string())?;
 let raw=value.to_x87_extended_bits().ok_or("native memory size cannot be represented as x87 finite value")?;
 let exponent=((raw>>64)&0x7fff)as i32-16383;
 if exponent<0{return Ok(0)}
 if exponent>63||raw>>79!=0{return Err("native memory-size integer conversion outside proved nonnegative64bit domain".into())}
 Ok(((raw as u64)>>(63-exponent))as u32)
}
