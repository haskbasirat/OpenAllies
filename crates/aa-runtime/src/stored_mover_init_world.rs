//! Actual576700 on the same stored Mover, current queue/state/flag owners.
//! The native falling byte gates every World/version/queue/state read. Reached
//! live4D4A90/4D16B0 use the same stored state-effects owners. Every callback
//! result rereads the actual state and flags before the native suffix runs.
use crate::{actor_world::{ActorStore,ObjectKey,RuntimeComponent},catalog::Catalog,definitions::Bank,restoration::Staged};
use aa_actors::{ActorId,actor_creation::{Error,Result},components::ComponentSlot as Slot,ground_states::MoverState,state};
use aa_numeric::Precision;
fn error(e:impl std::fmt::Display)->Error{Error::World(e.to_string())}
fn mover(actors:&ActorStore,owner:ActorId)->Result<&MoverState>{match actors.component(owner,Slot::Mover).map_err(error)?{
 Some(RuntimeComponent::Mover(v))if v.header.owner==owner=>Ok(v),_=>Err(error("stored576700 actual Mover allocation/owner differs")),
}}
fn checked(catalog:&Catalog<'_>,actors:&ActorStore,bank:&Bank,key:ObjectKey)->Result<ActorId>{
 let (owner,_)=crate::stored_component_init_source::checked(catalog,actors,bank,key,Slot::Mover)?;mover(actors,owner)?;
 let leaves=aa_actors::component_virtuals::leaves(aa_actors::init_create::FactoryKind::Standard(Slot::Mover));
 if leaves.init_existing!=0x576700||leaves.vf14!=0x4b2830{return Err(error("stored Mover actual virtual8/14 differs"))}
 Ok(owner)
}
fn flags(staged:&Staged,key:ObjectKey)->Result<u32>{
 if let Ok(p)=staged.actors.staged_placement(key){return Ok(p.flags)}
 let world=staged.load_world.as_ref().ok_or_else(||error("stored falling state requires actual World placement owner"))?;
 Ok(staged.actors.placement(key,world.live_spatial().map_err(error)?).map_err(error)?.flags)
}
struct Effects<'a,'source>{staged:&'a mut Staged,catalog:&'a Catalog<'source>,key:ObjectKey,owner:ActorId,precision:Precision,last_flags:u32}
impl Effects<'_,'_>{
 fn require_owner(&self)->Result<()>{
  if self.staged.actors.id(self.key)!=Some(self.owner)||self.staged.actors.key(self.owner)!=Some(self.key){return Err(error("stored falling state current GTID generation differs"))}
  Ok(())
 }
 fn commit_state(&mut self,fields:&state::Fields)->Result<()>{
  self.require_owner()?;
  let current=&mut self.staged.actors.object_mut(self.key).map_err(error)?.components_mut().map_err(error)?.state;
  current.low=fields.bits as u32;current.high=(fields.bits>>32)as u32;
  Ok(())
 }
 fn commit_falling_suffix(&mut self,fields:&state::Fields)->Result<()>{
  self.require_owner()?;
  if fields.thing_flags!=(self.last_flags|0x1000){return Err(error("stored falling state attempted an unsupported flag store"))}
  if flags(self.staged,self.key)?!=self.last_flags{return Err(error("stored falling state actual flag owner changed"))}
  if let Ok(p)=self.staged.actors.staged_placement_mut(self.key){p.flags|=0x1000;return Ok(())}
  let world=self.staged.load_world.as_mut().ok_or_else(||error("stored falling flag store actual World absent"))?;
  let crate::load_world::SpatialOwner::Adopted(adoption)=&mut world.spatial else{return Err(error("stored falling flag store actual published owner absent"))};
  if adoption.error.is_some()||!adoption.pending.is_empty(){return Err(error("stored falling flag store requires completed same index adoption"))}
  adoption.world.mark_falling(self.key).map_err(error)
 }
 fn rebuild(&mut self)->std::result::Result<(),String>{
  let world=self.staged.load_world.as_ref().ok_or("stored falling action rebuild actual World absent")?;
  let spatial=world.live_spatial()?;
  if spatial.index().precision()!=self.precision{return Err("stored falling action rebuild spatial/live PC differs".into())}
  crate::stored_actor_actions::rebuild_with_variables(self.catalog,&mut self.staged.actors,&mut self.staged.definitions,self.key,&self.staged.inventory,spatial,Some(&world.variables),self.precision)
 }
 fn reread(&mut self,fields:&mut state::Fields)->Result<()>{
  self.require_owner()?;
  let current=self.staged.actors.object(self.key).map_err(error)?.components().map_err(error)?.state;
  fields.bits=u64::from(current.low)|(u64::from(current.high)<<32);
  fields.thing_flags=flags(self.staged,self.key)?;
  self.last_flags=fields.thing_flags;
  Ok(())
 }
}
impl state::Effects for Effects<'_,'_>{
 fn leave(&mut self,f:&mut state::Fields,old:&'static state::Descriptor)->state::Result<()>{
  self.commit_state(f).map_err(|e|state::Error::Callback(e.to_string()))?;
  let result=crate::stored_actor_state_effects::leave_staged(self.staged,self.catalog,self.key,old.index,self.precision);
  self.reread(f).map_err(|e|state::Error::Callback(e.to_string()))?;
  result.map_err(state::Error::Callback)
 }
 fn enter(&mut self,f:&mut state::Fields,new:&'static state::Descriptor,old:Option<&'static state::Descriptor>)->state::Result<()>{
  self.commit_state(f).map_err(|e|state::Error::Callback(e.to_string()))?;
  let result=crate::stored_actor_state_effects::enter_staged(self.staged,self.catalog,self.key,new.index,old.map(|d|d.index),self.precision);
  self.reread(f).map_err(|e|state::Error::Callback(e.to_string()))?;
  result.map_err(state::Error::Callback)
 }
 fn rebuild_abilities(&mut self,f:&mut state::Fields)->state::Result<()>{
  self.commit_state(f).map_err(|e|state::Error::Callback(e.to_string()))?;
  let result=self.rebuild();
  self.reread(f).map_err(|e|state::Error::Callback(e.to_string()))?;
  result.map_err(state::Error::Callback)
 }
}
struct Stored<'a,'source>{world:crate::stored_component_schedule::World<'a>,catalog:&'a Catalog<'source>,owner:ActorId}
impl crate::mover_init_fields::Fields for Stored<'_,'_>{
 fn falling(&self)->Result<u8>{mover(&self.world.staged.actors,self.owner)?.bytes.get(&0x56).copied().ok_or_else(||error("stored576700 actual Mover+56 uninitialized"))}
 fn version(&mut self)->Result<u32>{Ok(self.world.staged.report.world_version)}
 fn pending(&mut self)->Result<bool>{self.world.pending()}
 fn add_periodic(&mut self)->Result<()>{self.world.add_periodic(0)}
 fn activate_falling(&mut self)->Result<()>{
  let staged=&mut *self.world.staged;let key=self.world.key;
  let original_flags=flags(staged,key)?;let current=staged.actors.object(key).map_err(error)?.components().map_err(error)?.state;
  let mut fields=state::Fields{bits:u64::from(current.low)|(u64::from(current.high)<<32),thing_flags:original_flags};
  let already_falling=fields.bits&(1<<24)!=0;
  let mut effects=Effects{staged,catalog:self.catalog,key,owner:self.owner,precision:self.world.precision,last_flags:original_flags};
  let outcome=state::set(&mut fields,24,true,&mut effects).map_err(error);
  effects.commit_state(&fields)?;
  //An unchanged state request returns before the native OR. Otherwise the
  //successful suffix writes even if bit12 was already set on entry.
  if outcome.is_ok()&&!already_falling{effects.commit_falling_suffix(&fields)?;}
  outcome
 }
}
pub fn init_existing(catalog:&Catalog<'_>,staged:&mut Staged,key:ObjectKey,pc:Precision)->Result<bool>{
 let owner=checked(catalog,&staged.actors,&staged.definitions,key)?;
 crate::mover_init_fields::initialize(&mut Stored{world:crate::stored_component_schedule::World::new(staged,key,Slot::Mover,pc)?,catalog,owner})
}
pub fn after_init(catalog:&Catalog<'_>,actors:&ActorStore,bank:&Bank,key:ObjectKey)->Result<()>{checked(catalog,actors,bank,key)?;Ok(())}
