//! Same environment114 numeric owner for the fresh86B940 constructor and
//!86E8D0 update. Source texture154 belongs to the outer real Resource owner.
//! Rust identity replaces Ni ABI pointers/counters; no view completion is minted.
use crate::{view_environment_projection::{self,Fields},view_grid_state::Globals};
use aa_numeric::Precision;
type Result<T> = std::result::Result<T,String>;

pub(crate) struct Core {
    words: [Option<u32>; 100],
    links: Option<NullLinks>,
    enabled_c4: Option<u8>,
    constructed: bool,
}
///Only the actual fresh constructor can establish these null allocations.
///No caller-supplied bool or replacement empty list is admitted.
struct NullLinks;
pub(crate) struct Failure { pub core: Core, pub error: String }
impl Core {
    pub(crate) fn from_native_constructor(g:&mut impl Globals)->std::result::Result<Self,Failure>{
        let mut core=Self{words:[None;100],links:None,enabled_c4:None,constructed:false};
        let result=(||->Result<()>{
            //86B240/86F710 reached constructor links, before source reads.
            core.links=Some(NullLinks);
            for start in [0x38,0x6c]{
                for(i,v)in [0x3f800000,0,0,0,0x3f800000,0,0,0,0x3f800000].into_iter().enumerate(){core.put(start+4*i as u32,v)}
                let x=g.word(0xb43cf8)?;core.put(start+0x24,x);
                let y=g.word(0xb43cfc)?;core.put(start+0x28,y);
                let z=g.word(0xb43d00)?;core.put(start+0x30,0x3f800000);core.put(start+0x2c,z);
            }
            //86F76D..78E's real bounding-center loads still happen although
            //that unrelated numeric bound is not exposed by this projection.
            for address in [0xb43cf8,0xb43cfc,0xb43d00]{g.word(address)?;}
            //89CA00 establishes actual empty effect-link containers and D0=1.
            //C8/B32450 native identity accounting is replaced by Rust ownership.
            core.enabled_c4=Some(1);core.put(0xd0,1);
            for start in [0xf4]{
                for(i,v)in [0x3f800000,0,0,0,0x3f800000,0,0,0,0x3f800000].into_iter().enumerate(){core.put(start+4*i as u32,v)}
            }
            for offset in [0x118,0x11c,0x120]{core.put(offset,0)}
            for(i,v)in [0x3f800000,0,0,0,0x3f800000,0,0,0,0x3f800000].into_iter().enumerate(){core.put(0x124+4*i as u32,v)}
            for offset in [0x148,0x14c,0x150]{core.put(offset,0)}
            //869C60 copies the actual read-only B3206C unit-X constant.
            for start in [0x16c,0x17c]{for(i,v)in [0x3f800000,0,0,0].into_iter().enumerate(){core.put(start+4*i as u32,v)}}
            for(offset,value)in [(0x158,0),(0x160,0),(0x164,0),(0x15c,3)]{core.put(offset,value)}
            core.constructed=true;Ok(())
        })();
        match result{Ok(())=>Ok(core),Err(error)=>Err(Failure{core,error})}
    }
    fn require_constructor(&self)->Result<()>{
        if self.constructed&&self.links.is_some(){Ok(())}else{Err("TextureEffect constructor source prefix is incomplete".into())}
    }
    ///Actual43EA50 writes, in order, before its explicit86BA10 call.
    pub(crate) fn configure_environment_43ea50(&mut self)->Result<()>{
        self.require_constructor()?;
        for(offset,value)in [(0x160,2),(0x158,1),(0x15c,3),(0x164,2)]{self.put(offset,value)}
        Ok(())
    }
    pub(crate) fn project_mode2_86ba10(&mut self,pc:Precision)->Result<()>{
        self.require_constructor()?;view_environment_projection::update_mode2_86ba10(self,pc)
    }
    pub(crate) fn enable_after_projection_43ea50(&mut self)->Result<()>{self.require_constructor()?;self.enabled_c4=Some(1);Ok(())}
    pub(crate) fn enabled_c4(&self)->Result<u8>{self.enabled_c4.ok_or_else(||"TextureEffect+C4 has no reached constructor writer".into())}
    ///Only the fresh constructor-null links are supported. A future positive
    ///owner must replace NullLinks and implement its actual reached traversal;
    ///it cannot reuse this route as an empty-list fallback.
    pub(crate) fn update_fresh_86e8d0(&mut self,time:u32,controllers:u8,pc:Precision)->Result<()>{
        self.require_constructor()?;update_fresh_86e8d0(self,time,controllers,pc)
    }
    pub(crate) fn word(&self,offset:u32)->Result<u32>{
        if matches!(offset,0x10|0x24|0xa4|0xb0){
            return if self.links.is_some(){Ok(0)}else{Err("TextureEffect fresh link producer is absent".into())}
        }
        self.words.get(offset as usize/4).filter(|_|offset%4==0).and_then(|v|*v).ok_or_else(||format!("TextureEffect+{offset:X} has no reached numeric writer"))
    }
}
impl Fields for Core{
    fn word(&mut self,offset:u32)->Result<u32>{Core::word(self,offset)}
    fn put(&mut self,offset:u32,word:u32){self.words[offset as usize/4]=Some(word)}
}
trait UpdateFields:Fields{
    ///The constructor's actual AB70A0 class proves each virtual target. This
    ///is a typed dispatch read, never a fabricated numeric vtable pointer.
    fn dispatch(&mut self,slot:u32)->Result<()>;
}
impl UpdateFields for Core{
    fn dispatch(&mut self,slot:u32)->Result<()>{
        match slot{0x48|0x68|0x64=>Ok(()),_=>Err("TextureEffect virtual target is outside the proved constructor class".into())}
    }
}

///Shared ordered update kernel. The concrete Core above is the only product
///caller; laboratory Fields test every required-read interruption directly.
fn update_fresh_86e8d0(fields:&mut impl UpdateFields,_time:u32,controllers:u8,pc:Precision)->Result<()>{
    fields.dispatch(0x48)?;
    //86EED0 reaches866BE0 only when the actual second argument's byte!=0.
    if controllers!=0{
        if fields.word(0xa4)?!=0{return Err("TextureEffect positive controller list requires real866BE0 owner".into())}
        if fields.word(0x10)?!=0{return Err("TextureEffect positive property/controller chain requires real866BE0 owner".into())}
    }
    //vf68=86BC30→8A2C70: a fresh parent-null effect copies its actual local
    //transform with thirteen reached REP MOVSD read/write pairs.
    fields.dispatch(0x68)?;
    if fields.word(0x24)?!=0{return Err("TextureEffect positive parent requires actual45CAB0 transform owner".into())}
    for i in 0..13{let word=fields.word(0x38+4*i)?;fields.put(0x6c+4*i,word)}
    if fields.word(0xb0)?!=0{return Err("TextureEffect positive bound requires actual virtual38 update".into())}
    view_environment_projection::update_mode2_86ba10(fields,pc)?;
    //vf64=4B2830 is a real pure return;86E8D0 then rereads parent24.
    fields.dispatch(0x64)?;
    if fields.word(0x24)?!=0{return Err("TextureEffect current parent requires actual virtual7C notification".into())}
    Ok(())
}

#[cfg(test)]#[path="view_texture_effect_tests.rs"]mod tests;
