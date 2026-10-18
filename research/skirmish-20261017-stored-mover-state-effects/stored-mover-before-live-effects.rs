//! Actual576700 on the same stored Mover, current queue/state/flag owners.
//! The native falling byte gates every World/version/queue/state read. Reached
//! live4D4A90 remains required; its preceding real state24 bit survives.
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
struct Effects<'a>{staged:&'a mut Staged,key:ObjectKey,original_flags:u32}
impl Effects<'_>{
 fn commit_state(&mut self,fields:&state::Fields)->Result<()>{
  let current=&mut self.staged.actors.object_mut(self.key).map_err(error)?.components_mut().map_err(error)?.state;
  current.low=fields.bits as u32;current.high=(fields.bits>>32)as u32;
  Ok(())
 }
 fn commit_falling_suffix(&mut self,fields:&state::Fields)->Result<()>{
  if fields.thing_flags!=(self.original_flags|0x1000){return Err(error("stored falling state attempted an unsupported flag store"))}
  if flags(self.staged,self.key)?!=self.original_flags{return Err(error("stored falling state actual flag owner changed"))}
  if let Ok(p)=self.staged.actors.staged_placement_mut(self.key){p.flags|=0x1000;return Ok(())}
  let world=self.staged.load_world.as_mut().ok_or_else(||error("stored falling flag store actual World absent"))?;
  let crate::load_world::SpatialOwner::Adopted(adoption)=&mut world.spatial else{return Err(error("stored falling flag store actual published owner absent"))};
  if adoption.error.is_some()||!adoption.pending.is_empty(){return Err(error("stored falling flag store requires completed same index adoption"))}
  adoption.world.mark_falling(self.key).map_err(error)
 }
 fn unbound(&mut self,fields:&state::Fields,address:&str)->state::Result<()>{
  self.commit_state(fields).map_err(|e|state::Error::Callback(e.to_string()))?;
  Err(state::Error::Callback(format!("{address} stored falling state current effects are required")))
 }
}
impl state::Effects for Effects<'_>{
 fn leave(&mut self,f:&mut state::Fields,_:&'static state::Descriptor)->state::Result<()>{self.unbound(f,"4D16B0")}
 fn enter(&mut self,f:&mut state::Fields,_:&'static state::Descriptor,_:Option<&'static state::Descriptor>)->state::Result<()>{self.unbound(f,"4D4A90")}
 fn rebuild_abilities(&mut self,f:&mut state::Fields)->state::Result<()>{self.unbound(f,"4D1350")}
}
struct Stored<'a>{world:crate::stored_component_schedule::World<'a>,owner:ActorId}
impl crate::mover_init_fields::Fields for Stored<'_>{
 fn falling(&self)->Result<u8>{mover(&self.world.staged.actors,self.owner)?.bytes.get(&0x56).copied().ok_or_else(||error("stored576700 actual Mover+56 uninitialized"))}
 fn version(&mut self)->Result<u32>{Ok(self.world.staged.report.world_version)}
 fn pending(&mut self)->Result<bool>{self.world.pending()}
 fn add_periodic(&mut self)->Result<()>{self.world.add_periodic(0)}
 fn activate_falling(&mut self)->Result<()>{
  let staged=&mut *self.world.staged;let key=self.world.key;
  let original_flags=flags(staged,key)?;let current=staged.actors.object(key).map_err(error)?.components().map_err(error)?.state;
  let mut fields=state::Fields{bits:u64::from(current.low)|(u64::from(current.high)<<32),thing_flags:original_flags};
  let already_falling=fields.bits&(1<<24)!=0;
  let mut effects=Effects{staged,key,original_flags};
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
 crate::mover_init_fields::initialize(&mut Stored{world:crate::stored_component_schedule::World::new(staged,key,Slot::Mover,pc)?,owner})
}
pub fn after_init(catalog:&Catalog<'_>,actors:&ActorStore,bank:&Bank,key:ObjectKey)->Result<()>{checked(catalog,actors,bank,key)?;Ok(())}
