//!5640A0 rejection prefix. True means the real redirect child is still needed,
//! not that eligibility/append/schedule has completed.
pub trait Read {
    type Error;
    fn flags(&self,actor:u32)->Result<u32,Self::Error>;
    fn owner(&self,actor:u32)->Result<u32,Self::Error>;
    fn nonmajor(&self,kingdom:u32)->Result<u32,Self::Error>;
}
pub fn needs_redirect<W:Read+?Sized>(world:&W,city_owner:u32,target:u32)->Result<bool,W::Error> {
    if target==0 || target==city_owner {return Ok(false)}
    if world.flags(target)?&1!=0 {return Ok(false)}
    if world.owner(target)?==0 {return Ok(false)}
    //Two actual owner108 calls, rather than a cached owner value.
    if world.nonmajor(world.owner(target)?)?!=0 {return Ok(false)}
    Ok(true)
}
