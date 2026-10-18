//!57F1E0 primary-ability property gate and ordered downstream services.
//! World owns the actual table/bytes. Missing native callbacks must error;
//! committed enabled-byte changes are never rolled back by this coordinator.
use crate::{Error,Result};
pub trait World{
 type Requirement;
 fn ability_count(&self)->Result<usize>;
 fn ability_at(&self,index:usize)->Result<u32>;
 ///None=actual NULL local+8, Some(empty)=actual NULL local+58.
 fn required_properties(&self,ability:u32)->Result<Option<Vec<Self::Requirement>>>;
 ///581280 with NULL source/exclude. Disabled instances still match.
 fn has_property(&self,required:&Self::Requirement)->Result<bool>;
 fn enabled(&self,ability:u32)->Result<bool>;
 fn set_enabled(&mut self,ability:u32,value:bool)->Result<()>;
 fn refresh_actions(&mut self)->Result<()>;
 fn refresh_value(&mut self)->Result<()>;
}
pub fn refresh(world:&mut impl World)->Result<()>{
 let count=world.ability_count()?;
 if count>i32::MAX as usize{return Err(Error::Invalid("57F1E0 signed ability count representation"))}
 let mut changed=false;
 for index in 0..count{
  let ability=world.ability_at(index)?;
  let Some(required)=world.required_properties(ability)?else{continue};
  if required.is_empty(){continue}
  let mut allowed=true;
  for required in &required{if !world.has_property(required)?{allowed=false;break}}
  if world.enabled(ability)?!=allowed{world.set_enabled(ability,allowed)?;changed=true}
 }
 if changed{world.refresh_actions()?;}
 world.refresh_value()
}
