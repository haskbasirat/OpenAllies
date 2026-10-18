//!752AE0 two59ABA0 calls around76F790. The retained raw DWORD history238
//!and status ratio60 are borrowed from the SAME Node used by task315.
use super::*;
use aa_numeric::{Binary,Precision};
use std::cmp::Ordering;
fn numeric(e:impl std::fmt::Display)->Error{missing(e.to_string())}
fn number(bits:u32)->Result<Binary>{Binary::from_f32_bits(bits).map_err(numeric)}
fn seconds(ms:u32,p:Precision)->Result<Binary>{
    let mut v=Binary::from_i32(ms as i32);
    if (ms as i32)<0{v=v.add(&number(0x4f800000)?,p).map_err(numeric)?;}
    v.mul(&number(0x3a83126f)?,p).map_err(numeric)
}
pub(crate) fn begin(ms:u32,p:Precision)->Result<u32>{seconds(ms,p)?.store_f32().map_err(numeric)}
pub(crate) fn finish(local:&mut LocalClientBorrow<'_>,start:u32,end_ms:u32,p:Precision)->Result<()>{
    let duration=seconds(end_ms,p)?.sub(&number(start)?,p).and_then(|v|v.store_f32()).map_err(numeric)?;
    let history=&mut *local.task_history_238;
    history.append_duration_74ecd0(duration)?;
    let floor=history.end_word().wrapping_sub(16);
    if history.begin_word()<floor{history.retire_duration_750990(floor)?;}
    let mut sum=0u32;let mut count=0i32;
    for i in history.begin_word()..history.end_word(){
        sum=number(sum)?.add(&number(history.duration_word(i)?)?,p).and_then(|v|v.store_f32()).map_err(numeric)?;
        count=count.wrapping_add(1);
    }
    if count>=10{
        let mut ratio=number(sum)?.div(&Binary::from_i32(count),p).and_then(|v|v.store_f32()).map_err(numeric)?;
        //Native stores60 before either comparison; preserve that prefix if
        //a later nonfinite value/unsupported comparison is encountered.
        *local.heartbeat.frame_ratio=Some(ratio);
        if number(ratio)?.compare(&number(0x38d1b717)?)==Ordering::Less{ratio=0x38d1b717;}
        else if number(ratio)?.compare(&Binary::from_i32(1))==Ordering::Greater{ratio=0x3f800000;}
        *local.heartbeat.frame_ratio=Some(ratio);
    }
    Ok(())
}
#[cfg(test)]
#[path="local_frame_duration_tests.rs"]
mod tests;
