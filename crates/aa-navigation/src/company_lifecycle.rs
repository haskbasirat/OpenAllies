//!Same523080/51D7B0/5205E0 coordinator on lab or live disjoint fields.
use super::{CompanyMoveState,CompanyWorld,CompanyActivationFields,Extents,
    set_destination_on_fields,reset_path,b,store,EPSILON,activation::Legacy};
use crate::Result;
use aa_numeric::Precision;
use std::cmp::Ordering;
pub trait CompanySpeedFields{
    fn organization_speed_cap(&mut self)->Result<u32>;
    fn store_speed(&mut self,value:u32)->Result<()>;
    ///Real4364C0 iteration; each native member callback sees current speed.
    fn broadcast_speed(&mut self)->Result<()>;
}
pub fn set_speed_on_fields(fields:&mut impl CompanySpeedFields,speed:u32)->Result<()>{
    let cap=fields.organization_speed_cap()?;
    let mut speed=speed;
    if b(speed)?.compare(&b(cap)?)==Ordering::Greater{speed=cap;}
    if b(speed)?.compare(&b(EPSILON)?)==Ordering::Less{speed=cap;}
    fields.store_speed(store(&b(speed)?)?)?;
    fields.broadcast_speed()
}
pub trait CompanyExitFields{
    fn store_status(&mut self,value:u32)->Result<()>;
    fn reset_owned_path(&mut self)->Result<()>;
    fn store_tries(&mut self,value:i32)->Result<()>;
}
pub trait CompanyLifecycleFields:CompanyActivationFields+CompanySpeedFields+CompanyExitFields{
    fn copy_auxiliary_to_mind(&mut self)->Result<()>;
    fn current_speed(&mut self)->Result<u32>;
    ///Full current GTID/generation lookup after reset/tries stores.
    fn current_target_position(&mut self)->Result<Option<[u32;2]>>;
    fn clear_formation(&mut self)->Result<()>;
    fn current_world_time(&mut self)->Result<u32>;
    fn store_start_time(&mut self,value:u32)->Result<()>;
}
pub fn enter_on_fields(fields:&mut impl CompanyLifecycleFields,extents:Extents,p:Precision)->Result<()>{
    fields.store_status(2)?;
    fields.copy_auxiliary_to_mind()?;
    let speed=fields.current_speed()?;
    set_speed_on_fields(fields,speed)?;
    fields.reset_owned_path()?;
    fields.store_tries(0)?;
    if let Some(point)=fields.current_target_position()?{set_destination_on_fields(fields,point,extents,p)?;}
    fields.clear_formation()?;
    let time=fields.current_world_time()?;
    fields.store_start_time(time)
}
pub fn leave_on_fields(fields:&mut impl CompanyExitFields)->Result<()>{
    fields.store_status(0)?;fields.reset_owned_path()?;fields.store_tries(0)
}
impl<W:CompanyWorld> CompanySpeedFields for Legacy<'_,W>{
    fn organization_speed_cap(&mut self)->Result<u32>{self.world.organization_speed_cap()}
    fn store_speed(&mut self,value:u32)->Result<()>{self.state.speed_bits=value;Ok(())}
    fn broadcast_speed(&mut self)->Result<()>{
        for slot in self.world.member_slots()?{if self.world.member_accepts_speed(slot)?{self.world.member_set_speed(slot,self.state.speed_bits)?;}}Ok(())
    }
}
impl<W:CompanyWorld> CompanyExitFields for Legacy<'_,W>{
    fn store_status(&mut self,value:u32)->Result<()>{self.state.status=value;Ok(())}
    fn reset_owned_path(&mut self)->Result<()>{reset_path(&mut self.state.path);Ok(())}
    fn store_tries(&mut self,value:i32)->Result<()>{self.state.tries=value;Ok(())}
}
impl<W:CompanyWorld> CompanyLifecycleFields for Legacy<'_,W>{
    fn copy_auxiliary_to_mind(&mut self)->Result<()>{self.world.set_mind_auxiliary(self.state.auxiliary_bits)}
    fn current_speed(&mut self)->Result<u32>{Ok(self.state.speed_bits)}
    fn current_target_position(&mut self)->Result<Option<[u32;2]>>{self.world.actor_position(self.state.target_id)}
    fn clear_formation(&mut self)->Result<()>{self.state.formation_word=0;Ok(())}
    fn current_world_time(&mut self)->Result<u32>{self.world.world_time()}
    fn store_start_time(&mut self,value:u32)->Result<()>{self.state.start_time_bits=value;Ok(())}
}
pub(super) struct StateOnly<'a>(pub &'a mut CompanyMoveState);
impl CompanyExitFields for StateOnly<'_>{
    fn store_status(&mut self,value:u32)->Result<()>{self.0.status=value;Ok(())}
    fn reset_owned_path(&mut self)->Result<()>{reset_path(&mut self.0.path);Ok(())}
    fn store_tries(&mut self,value:i32)->Result<()>{self.0.tries=value;Ok(())}
}
