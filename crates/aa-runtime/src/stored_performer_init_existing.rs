//! Native57E000 on the actual stored Performer and Ability map.
//! Rank is the native NULL input (zero), not current Experience state. The
//! full changed-byte4D1350 tail runs only after releasing component loans.
use crate::{actor_world::{ActorStore,ObjectKey,RuntimeComponent,Spatial},catalog::Catalog,definitions::Bank};
use aa_actors::{ActorId,components::ComponentSlot as Slot};
use aa_combat::performer::{self,RankWorld};
use aa_numeric::Precision;
type Result<T>=std::result::Result<T,String>;
fn text(e:impl std::fmt::Display)->String{e.to_string()}
fn combat(e:impl std::fmt::Display)->aa_combat::Error{aa_combat::Error::World(e.to_string())}

fn validate(catalog:&Catalog<'_>,actors:&ActorStore,bank:&Bank,key:ObjectKey)->Result<(ActorId,Precision)>{
 if aa_actors::component_virtuals::leaves(aa_actors::init_create::FactoryKind::Standard(Slot::Performer)).init_existing!=0x57e000{return Err("stored Performer InitExisting actual virtual8 differs".into())}
 catalog.require_source_owner()?;let source=bank.combat_source()?;source.completed_means()?;
 if source.source_identities()!=catalog.actors.sources.as_slice(){return Err("stored57E000 source Bank universe differs".into())}
 let source_pc=source.source_precision();let owner=actors.id(key).ok_or("stored57E000 actual GTID absent")?;
 let object=actors.object(key).map_err(text)?;let binding=catalog.actor_bindings(&object.definition.proof.id,source_pc)?;
 if object.kind!=aa_actors::loading::LoadKind::Actor||object.definition.native_class!=3
  ||object.definition.native_class!=object.definition.proof.category.runtime_class
  ||!crate::load_spatial::same_proof(&object.thing.definition,&object.definition.proof)
  ||!crate::load_spatial::same_proof(&binding.definition.proof,&object.definition.proof)
  ||object.definition.flags!=binding.definition.flags||object.definition.component_mask!=binding.definition.component_mask
  ||object.definition.component_mask.is_none_or(|m|m&(1<<5)==0){return Err("stored57E000 actual factory/source proof differs".into())}
 match object.components().map_err(text)?.get(Slot::Performer){
  Some(RuntimeComponent::Performer(p))if p.state.owner==owner.raw()=>(),
  _=>return Err("stored57E000 actual Performer payload/owner differs".into()),
 }
 Ok((owner,source_pc))
}
struct Rank<'a,'source>{catalog:&'a Catalog<'source>,bank:&'a Bank,
 abilities:&'a mut std::collections::BTreeMap<u32,aa_combat::ability::Ability>,source_pc:Precision}
impl RankWorld for Rank<'_,'_>{
 fn ability_local_definition(&self,handle:u32)->aa_combat::Result<Option<u32>>{Ok(self.abilities.get(&handle).ok_or_else(||combat("stored57DCF0 actual Ability handle absent"))?.local_definition)}
 fn rank_limits(&self,local:u32)->aa_combat::Result<[i32;2]>{crate::load_performer_init_world::rank_limits_in_bank(self.catalog,self.bank,local,self.source_pc).map_err(combat)}
 fn ability_enabled(&self,handle:u32)->aa_combat::Result<bool>{Ok(self.abilities.get(&handle).ok_or_else(||combat("stored57DCF0 actual Ability handle absent"))?.enabled)}
 fn set_ability_enabled(&mut self,handle:u32,enabled:bool)->aa_combat::Result<()>{self.abilities.get_mut(&handle).ok_or_else(||combat("stored57DCF0 actual Ability handle absent"))?.enabled=enabled;Ok(())}
 fn notify(&mut self,_:u32)->aa_combat::Result<()>{Err(combat("stored rank prefix cannot notify with active immutable Performer loan"))}
}
#[allow(clippy::too_many_arguments)]
pub fn init_existing<S:Spatial>(catalog:&Catalog<'_>,actors:&mut ActorStore,bank:&mut Bank,key:ObjectKey,
 inventory:&aa_mapworld::WorldInventory,spatial:&S,live_pc:Precision)->Result<bool>{
 init_existing_with_variables(catalog,actors,bank,key,inventory,spatial,None,live_pc)
}
#[allow(clippy::too_many_arguments)]
pub fn init_existing_with_variables<S:Spatial>(catalog:&Catalog<'_>,actors:&mut ActorStore,bank:&mut Bank,key:ObjectKey,
 inventory:&aa_mapworld::WorldInventory,spatial:&S,variables:Option<&crate::load_view_variables::State>,live_pc:Precision)->Result<bool>{
 let (owner,source_pc)=validate(catalog,actors,bank,key)?;
 let changed={
  let (_,mut access)=actors.split_property_state();let focus=access.focus(key).map_err(text)?;
  if focus.id!=owner{return Err("stored57E000 focused GTID changed".into())}
  let object=focus.entity.runtime.as_ref().ok_or("stored57E000 actual runtime allocation absent")?;
  let Some(RuntimeComponent::Performer(current))=object.components().map_err(text)?.get(Slot::Performer)else{return Err("stored57E000 actual Performer vanished".into())};
  if current.state.abilities.len()>i32::MAX as usize{return Err("57DCF0 signed primary Ability count representation".into())}
  performer::gate_by_rank_prefix(&current.state,None,&mut Rank{catalog,bank,abilities:focus.abilities,source_pc}).map_err(text)?
 };
 if changed{crate::stored_actor_actions::rebuild_with_variables(catalog,actors,bank,key,inventory,spatial,variables,live_pc)?;}
 Ok(true)
}
pub fn after_init(catalog:&Catalog<'_>,actors:&ActorStore,bank:&Bank,key:ObjectKey)->Result<()>{
 validate(catalog,actors,bank,key)?;
 if aa_actors::component_virtuals::leaves(aa_actors::init_create::FactoryKind::Standard(Slot::Performer)).vf14!=0x4b2830{return Err("stored Performer AfterInit actual virtual14 differs".into())}
 Ok(())
}
