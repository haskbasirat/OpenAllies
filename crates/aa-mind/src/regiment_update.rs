//!529D00 base deferred Update and524910 Regiment Update. Native this points
//!at Mind+8 (the Updatable subobject), not at its primary component vtable.
use crate::{MindId,Result};
pub trait BaseWorld {
    type Actor:Copy;
    type Kingdom:Copy;
    fn actor(&mut self,mind:MindId)->Result<Self::Actor>;
    fn actor_has_state10(&mut self,actor:Self::Actor)->Result<bool>;
    fn current_owner108(&mut self,actor:Self::Actor)->Result<Option<Self::Kingdom>>;
    fn kingdom_byte239(&mut self,owner:Self::Kingdom)->Result<u8>;
    fn attached_to_organization(&mut self,actor:Self::Actor)->Result<bool>;
    fn current_home_4d11a0(&mut self,actor:Self::Actor)->Result<bool>;
    fn mind_virtual1a8(&mut self,mind:MindId)->Result<()>;
    fn timed_list_4d37e0(&mut self,actor:Self::Actor)->Result<()>;
}
pub fn base_update<W:BaseWorld>(w:&mut W,mind:MindId)->Result<()> {
    let actor=w.actor(mind)?;
    if w.actor_has_state10(actor)? {
        let actor=w.actor(mind)?;
        if let Some(owner)=w.current_owner108(actor)? {
            if w.kingdom_byte239(owner)?!=0 {
                let actor=w.actor(mind)?;
                if !w.attached_to_organization(actor)? {
                    //529D3E reuses the current Actor in ECX from529D2A.
                    if !w.current_home_4d11a0(actor)? {
                        return w.mind_virtual1a8(mind);
                    }
                }
            }
        }
    }
    let actor=w.actor(mind)?;
    w.timed_list_4d37e0(actor)
}
pub trait World:BaseWorld {
    type State:Copy;
    fn actor_flags(&mut self,actor:Self::Actor)->Result<u32>;
    fn member_count_57a730(&mut self,actor:Self::Actor)->Result<u32>;
    ///523EA0 on the actual embedded Engage state at Mind+1CC.
    fn refresh_engage(&mut self,mind:MindId)->Result<()>;
    ///529F70 returns the current top pointer. Null is a native dereference,
    ///so concrete adapters must report that error rather than invent Sleep.
    fn current_state(&mut self,mind:MindId)->Result<Self::State>;
    fn think(&mut self,state:Self::State)->Result<()>;
    ///CURRENT Updatable+C intrusive scheduling head, after the Think callback.
    fn updatable_head_present(&mut self,mind:MindId)->Result<bool>;
    fn schedule(&mut self,mind:MindId,delay_bits:u32,payload:u32)->Result<()>;
    ///763B90 destroys the actual arrival nodes, preserving the empty tail.
    fn clear_arrivals(&mut self,mind:MindId)->Result<()>;
}
pub fn update<W:World>(w:&mut W,mind:MindId)->Result<()> {
    base_update(w,mind)?;
    let actor=w.actor(mind)?;
    //52491D captures this Actor once for both flag tests and member getter.
    if w.actor_flags(actor)?&1!=0 {return Ok(());}
    if w.actor_flags(actor)?&(1<<6)!=0 {return Ok(());}
    if w.member_count_57a730(actor)?==0 {return w.mind_virtual1a8(mind);}
    w.refresh_engage(mind)?;
    let state=w.current_state(mind)?;
    w.think(state)?;
    if !w.updatable_head_present(mind)? {
        let actor=w.actor(mind)?;
        if w.actor_flags(actor)?&1==0 {w.schedule(mind,0x4000_0000,0)?;}
    }
    w.clear_arrivals(mind)
}
