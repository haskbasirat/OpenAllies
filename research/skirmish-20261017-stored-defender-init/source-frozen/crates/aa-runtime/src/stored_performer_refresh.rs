//! Native57F4C0 ->57F1E0 ->57E430 after Property insertion in ActorStore.
//! The original component boxes, Ability map, instance directory, mutable
//! Bank manager and current SVar State remain in their canonical owners.
use crate::{actor_world::{ActorStore,ActorAccess,ObjectKey,RuntimeComponent},catalog::Catalog,
 definitions::Bank,load_performer_property_refresh as gate,
 load_performer_value_refresh::{self as value,EnvironmentSampler}};
use aa_actors::{ActorId,components::ComponentSlot as Slot};
use aa_body::{F32,properties::PropertiesState,property_api};
use aa_numeric::Precision;
use aa_rules::abilities::DefinitionIdentity;
type Result<T>=std::result::Result<T,String>;
#[cfg(test)]
#[path="stored_performer_refresh_tests.rs"]
mod tests;
fn text(e:impl std::fmt::Display)->String{e.to_string()}
fn actor_error(e:impl std::fmt::Display)->aa_actors::actor_creation::Error{aa_actors::actor_creation::Error::World(e.to_string())}
fn combat(e:impl std::fmt::Display)->aa_combat::Error{aa_combat::Error::World(e.to_string())}
struct PlacementRead<'a>(Option<&'a crate::spatial_world::SpatialWorld>);
impl crate::actor_world::Spatial for PlacementRead<'_>{
 fn placement(&self,id:ActorId)->crate::actor_world::Result<crate::actor_world::PlacementState>{
  crate::actor_world::Spatial::placement(self.0.ok_or(crate::actor_world::Error::Missing("stored Performer actual published placement owner"))?,id)
 }
 fn publish(&mut self,_:ActorId,_:crate::actor_world::PlacementState)->crate::actor_world::Result<()>{Err(crate::actor_world::Error::Missing("read-only stored Performer placement view cannot publish"))}
}

struct Bound<'a,'store,'source>{catalog:&'a Catalog<'source>,actors:&'a mut ActorAccess<'store>,arena:&'a mut aa_mind::Arena,bank:&'a mut Bank,
 key:ObjectKey,owner:ActorId,variables:&'a mut crate::load_view_variables::State,
 environment:EnvironmentSampler,source_pc:Precision,live_pc:Precision,
 spatial:Option<&'a crate::spatial_world::SpatialWorld>,inventory:&'a aa_mapworld::WorldInventory}
impl Bound<'_,'_,'_>{
 fn component(&self,slot:Slot)->Result<Option<&RuntimeComponent>>{
  Ok(self.actors.object(self.key).map_err(text)?.actor.as_ref().ok_or("stored Performer Actor allocation absent")?.components.get(slot))
 }
 fn performer(&self)->Result<&aa_combat::constructors::OwnedPerformer>{match self.component(Slot::Performer)?{
  Some(RuntimeComponent::Performer(p))if p.state.owner==self.owner.raw()=>Ok(p),
  _=>Err("stored Performer payload/owner differs".into()),
 }}
 fn properties(&self)->Result<Option<&PropertiesState>>{match self.component(Slot::Properties)?{
  None=>Ok(None),Some(RuntimeComponent::Properties(p))if p.owner.0==self.owner.raw()=>Ok(Some(p)),
  _=>Err("stored Performer Properties payload/owner differs".into()),
 }}
 fn ability(&self,handle:u32)->Result<&aa_combat::ability::Ability>{self.actors.abilities.get(&handle).ok_or_else(||"stored Performer actual Ability handle absent".into())}
}
impl value::Fields for Bound<'_,'_,'_>{
 fn property_count(&self)->aa_actors::actor_creation::Result<usize>{Ok(self.properties().map_err(actor_error)?.map_or(0,|p|p.effects.len()))}
 fn property_bias(&self,index:usize)->aa_actors::actor_creation::Result<F32>{
  let handle=*self.properties().map_err(actor_error)?.and_then(|p|p.effects.get(index)).ok_or_else(||actor_error("57E430 actual stored Property ordinal vanished"))?;
  let instance=self.actors.property_allocations.instances().get(&handle).ok_or_else(||actor_error("57E430 actual stored Property instance absent"))?;
  if instance.handle!=handle{return Err(actor_error("57E430 actual stored Property handle differs"))}
  let entry=self.bank.entry(instance.definition.0).map_err(actor_error)?;
  if entry.key.domain!="property"{return Err(actor_error("57E430 instance definition belongs to another native manager"))}
  value::property_bias_for_id(self.catalog,&entry.key.name,self.source_pc)
 }
 fn set_value(&mut self,bits:u32)->aa_actors::actor_creation::Result<()>{
  match self.actors.object_mut(self.key).map_err(actor_error)?.actor.as_mut().ok_or_else(||actor_error("stored Performer Actor allocation absent"))?.components.get_mut(Slot::Performer){
   Some(RuntimeComponent::Performer(p))if p.state.owner==self.owner.raw()=>{p.word60=bits;Ok(())},
   _=>Err(actor_error("stored Performer payload/owner differs")),
  }
 }
 fn value(&self)->aa_actors::actor_creation::Result<u32>{Ok(self.performer().map_err(actor_error)?.word60)}
 fn body(&self)->aa_actors::actor_creation::Result<(F32,F32)>{match self.component(Slot::Body).map_err(actor_error)?{
  None=>Ok((F32::ZERO,F32::ZERO)),Some(RuntimeComponent::Body(b))if b.header.owner==self.owner=>Ok((b.body.defense,b.body.maximum)),
  _=>Err(actor_error("57E430 actual stored Body payload/owner differs")),
 }}
 fn speed(&self)->aa_actors::actor_creation::Result<F32>{match self.component(Slot::Mover).map_err(actor_error)?{
  None=>Ok(F32::ZERO),Some(RuntimeComponent::Mover(m))if m.header.owner==self.owner=>Ok(m.speed),
  _=>Err(actor_error("57E430 actual stored Mover payload/owner differs")),
 }}
 fn ability_count(&self,defender:bool)->aa_actors::actor_creation::Result<usize>{
  if !defender{return Ok(self.performer().map_err(actor_error)?.state.abilities.len())}
  match self.component(Slot::Defender).map_err(actor_error)?{None=>Ok(0),Some(RuntimeComponent::Defender(d))if d.header.owner==self.owner=>Ok(d.abilities.len()),_=>Err(actor_error("57E430 actual stored Defender payload/owner differs"))}
 }
 fn ability_at(&self,defender:bool,index:usize)->aa_actors::actor_creation::Result<u32>{
  if !defender{return self.performer().map_err(actor_error)?.state.abilities.get(index).copied().ok_or_else(||actor_error("57E430 actual stored primary ordinal vanished"))}
  match self.component(Slot::Defender).map_err(actor_error)?{Some(RuntimeComponent::Defender(d))if d.header.owner==self.owner=>d.abilities.get(index).copied().ok_or_else(||actor_error("57E430 actual stored Defender ordinal vanished")),_=>Err(actor_error("57E430 actual stored Defender payload/owner differs"))}
 }
 fn enabled(&self,handle:u32)->aa_actors::actor_creation::Result<bool>{Ok(self.ability(handle).map_err(actor_error)?.enabled)}
 fn evaluate(&mut self,handle:u32,unit:[F32;4])->aa_actors::actor_creation::Result<F32>{
  let local=value::local_ability_in_bank(self.catalog,self.bank,self.ability(handle).map_err(actor_error)?,self.source_pc)?;
  if local.definition.flags_54&0x80!=0{return Ok(F32::ZERO)}
  let object=self.actors.object(self.key).map_err(actor_error)?;
  let bonus=value::performer_bias(self.catalog,&object.definition.proof.id,self.source_pc)?;
  let modifier=match object.actor.as_ref().unwrap().components.get(Slot::Properties){
   None=>None,Some(RuntimeComponent::Properties(p))if p.owner.0==self.owner.raw()=>Some((self.owner.raw(),&p.modifier)),
   _=>return Err(actor_error("4CF110 actual stored Properties payload/owner differs")),
  };
  value::evaluate_in_bank(self.catalog,self.bank,&local,bonus,modifier,self.variables,self.environment,unit,self.live_pc)
 }
}
impl aa_combat::performer_property_gate::World for Bound<'_,'_,'_>{
 type Requirement=DefinitionIdentity;
 fn ability_count(&self)->aa_combat::Result<usize>{Ok(self.performer().map_err(combat)?.state.abilities.len())}
 fn ability_at(&self,index:usize)->aa_combat::Result<u32>{self.performer().map_err(combat)?.state.abilities.get(index).copied().ok_or_else(||combat("57F1E0 actual stored primary ordinal vanished"))}
 fn required_properties(&self,ability:u32)->aa_combat::Result<Option<Vec<DefinitionIdentity>>>{gate::requirements_in_bank(self.catalog,self.bank,self.ability(ability).map_err(combat)?,self.source_pc).map_err(combat)}
 fn has_property(&self,required:&DefinitionIdentity)->aa_combat::Result<bool>{
  let properties=self.properties().map_err(combat)?.ok_or_else(||combat("57F1E0->581280 NULL required Properties+10 at5032D6"))?;
  if properties.effects.is_empty(){return Ok(false)}
  let definition=gate::required_definition(self.catalog,&self.actors.property_allocations,required,self.source_pc).map_err(combat)?;
  let found=property_api::find_instance(&properties.effects,definition,None,None,|handle|{
   let instance=self.actors.property_allocations.instances().get(&handle).ok_or_else(||aa_body::Error::Callback("57F1E0 actual stored Property instance absent".into()))?;
   if instance.handle!=handle{return Err(aa_body::Error::Callback("57F1E0 actual stored Property handle differs".into()))}
   Ok((instance.definition,None))
  }).map_err(combat)?;
  Ok(found.is_some())
 }
 fn enabled(&self,handle:u32)->aa_combat::Result<bool>{Ok(self.ability(handle).map_err(combat)?.enabled)}
 fn set_enabled(&mut self,handle:u32,enabled:bool)->aa_combat::Result<()>{self.actors.abilities.get_mut(&handle).ok_or_else(||combat("57F1E0 actual stored Ability handle absent"))?.enabled=enabled;Ok(())}
 fn refresh_actions(&mut self)->aa_combat::Result<()>{
  crate::stored_actor_actions::rebuild_on_owners(self.catalog,self.actors,self.arena,self.bank,self.key,self.inventory,&PlacementRead(self.spatial),Some(&*self.variables),self.live_pc).map_err(combat)
 }
 fn refresh_value(&mut self)->aa_combat::Result<()>{
  self.variables.validate_in_bank(self.catalog,self.bank).map_err(combat)?;
  self.variables.require_published().map_err(combat)?;
  value::refresh_fields(self,self.live_pc).map_err(combat)
 }
}
///Called only after the Property insertion's disjoint loans end. The instance
///argument at57F4C0 is ignored by the actual native wrapper, not reconstructed.
#[allow(clippy::too_many_arguments)]
pub fn refresh_on_owners(catalog:&Catalog<'_>,actors:&mut ActorStore,bank:&mut Bank,
 spatial:Option<&crate::spatial_world::SpatialWorld>,key:ObjectKey,
 variables:&mut crate::load_view_variables::State,inventory:&aa_mapworld::WorldInventory,
 environment:EnvironmentSampler,live_pc:Precision)->Result<()>{
 run_on_owners(catalog,actors,bank,spatial,key,variables,inventory,environment,live_pc,true)
}
///Direct57E430 when the caller already performed its own ordered property
///and action callbacks, as actual4D7230 does. No extra property gate is added.
#[allow(clippy::too_many_arguments)]
pub fn refresh_value_on_owners(catalog:&Catalog<'_>,actors:&mut ActorStore,bank:&mut Bank,
 spatial:Option<&crate::spatial_world::SpatialWorld>,key:ObjectKey,
 variables:&mut crate::load_view_variables::State,inventory:&aa_mapworld::WorldInventory,
 environment:EnvironmentSampler,live_pc:Precision)->Result<()>{
 run_on_owners(catalog,actors,bank,spatial,key,variables,inventory,environment,live_pc,false)
}
#[allow(clippy::too_many_arguments)]
fn run_on_owners(catalog:&Catalog<'_>,actors:&mut ActorStore,bank:&mut Bank,
 spatial:Option<&crate::spatial_world::SpatialWorld>,key:ObjectKey,
 variables:&mut crate::load_view_variables::State,inventory:&aa_mapworld::WorldInventory,
 environment:EnvironmentSampler,live_pc:Precision,property_gate:bool)->Result<()>{
 let(arena,mut access)=actors.split_minds();
 run_on_split_owners(catalog,&mut access,arena,bank,spatial,key,variables,inventory,environment,live_pc,property_gate)
}
///Synchronous Mind/property callbacks lend these paired loans from ONE actual
///ActorStore. The current Mind must remain installed in that same Arena.
///No Arena/component is extracted, copied or temporarily replaced here.
#[allow(clippy::too_many_arguments)]
pub fn refresh_on_split_owners(catalog:&Catalog<'_>,actors:&mut ActorAccess<'_>,arena:&mut aa_mind::Arena,bank:&mut Bank,
 spatial:Option<&crate::spatial_world::SpatialWorld>,key:ObjectKey,
 variables:&mut crate::load_view_variables::State,inventory:&aa_mapworld::WorldInventory,
 environment:EnvironmentSampler,live_pc:Precision)->Result<()>{
 run_on_split_owners(catalog,actors,arena,bank,spatial,key,variables,inventory,environment,live_pc,true)
}
#[allow(clippy::too_many_arguments)]
fn run_on_split_owners(catalog:&Catalog<'_>,actors:&mut ActorAccess<'_>,arena:&mut aa_mind::Arena,bank:&mut Bank,
 spatial:Option<&crate::spatial_world::SpatialWorld>,key:ObjectKey,
 variables:&mut crate::load_view_variables::State,inventory:&aa_mapworld::WorldInventory,
 environment:EnvironmentSampler,live_pc:Precision,property_gate:bool)->Result<()>{
 catalog.require_source_owner()?;
 let source=bank.combat_source()?;
 if source.source_identities()!=catalog.actors.sources.as_slice(){return Err("stored Performer source Bank universe differs".into())}
 source.completed_means()?;
 let source_pc=source.source_precision();
 let owner=actors.id(key).ok_or("stored Performer current GTID absent")?;
 let object=actors.object(key).map_err(text)?;
 let binding=catalog.actor_bindings(&object.definition.proof.id,source_pc)?;
 if !matches!(object.definition.native_class,3|4)
  ||object.definition.native_class!=object.definition.proof.category.runtime_class
  ||!crate::load_spatial::same_proof(&object.thing.definition,&object.definition.proof)
  ||!crate::load_spatial::same_proof(&binding.definition.proof,&object.definition.proof)
  ||object.definition.flags!=binding.definition.flags||object.definition.component_mask!=binding.definition.component_mask
  ||object.definition.component_mask.is_none_or(|m|m&(1<<5)==0){return Err("stored Performer current factory/source proof differs".into())}
 if property_gate{actors.property_allocations.require_in_bank(catalog,bank)?;}
 let mut bound=Bound{catalog,actors,arena,bank,key,owner,variables,environment,source_pc,live_pc,spatial,inventory};
 bound.performer()?;
 if property_gate{aa_combat::performer_property_gate::refresh(&mut bound).map_err(text)}
 else{aa_combat::performer_property_gate::World::refresh_value(&mut bound).map_err(text)}
}
