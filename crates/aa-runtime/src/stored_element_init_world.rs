//! Actual5704D0 resupply-rate store and queue suffix on the stored Element.
use crate::{actor_world::{ActorStore,ObjectKey,RuntimeComponent},catalog::Catalog,definitions::Bank,restoration::Staged};
use aa_actors::{ActorId,actor_creation::{Error,Result},components::ComponentSlot as Slot,ground_states::ElementState};
use aa_numeric::Precision;
fn error(e:impl std::fmt::Display)->Error{Error::World(e.to_string())}
fn element(actors:&ActorStore,owner:ActorId)->Result<&ElementState<String>>{match actors.component(owner,Slot::Element).map_err(error)?{
 Some(RuntimeComponent::Element(v))if v.header.owner==owner=>Ok(v),_=>Err(error("stored5704D0 actual Element allocation/owner differs")),
}}
fn checked(catalog:&Catalog<'_>,actors:&ActorStore,bank:&Bank,key:ObjectKey)->Result<(ActorId,Precision)>{
 let result=crate::stored_component_init_source::checked(catalog,actors,bank,key,Slot::Element)?;element(actors,result.0)?;
 let leaves=aa_actors::component_virtuals::leaves(aa_actors::init_create::FactoryKind::Standard(Slot::Element));
 if leaves.init_existing!=0x5704d0||leaves.vf14!=0x4b2830{return Err(error("stored Element actual virtual8/14 differs"))}
 Ok(result)
}
pub fn init_existing(catalog:&Catalog<'_>,staged:&mut Staged,key:ObjectKey,pc:Precision)->Result<bool>{
 let (owner,source_pc)=checked(catalog,&staged.actors,&staged.definitions,key)?;
 let object=staged.actors.object(key).map_err(error)?;
 let source=aa_rules::native::actor_morale::actor_morale_source(catalog.actors,&object.definition.proof.id,source_pc).map_err(error)?;
 if source.actor_source!=object.definition.proof.source{return Err(error("stored5704D0 Element source provenance differs"))}
 let base=source.resupply_rate.ok_or_else(||error("stored5704D0 actual ActorData+2CC source absent"))?.bits;
 let modifier=match object.components().map_err(error)?.get(Slot::Properties){None=>None,
  Some(RuntimeComponent::Properties(p))if p.owner.0==owner.raw()=>Some(&p.modifier),
  _=>return Err(error("stored4CF110 actual Properties payload/owner differs")),
 };
 let value=crate::load_element_init_world::rate(base,modifier,pc)?;
 match staged.actors.component_mut(owner,Slot::Element).map_err(error)?{
  Some(RuntimeComponent::Element(v))if v.header.owner==owner=>v.word_20=value,
  _=>return Err(error("stored5704D0 actual Element vanished before+20 store")),
 }
 //Native5704EA precedes the actual version/head/scheduling reads.
 crate::load_detection_init_world::initialize(&mut crate::stored_component_schedule::World::new(staged,key,Slot::Element,pc)?)
}
pub fn after_init(catalog:&Catalog<'_>,actors:&ActorStore,bank:&Bank,key:ObjectKey)->Result<()>{checked(catalog,actors,bank,key)?;Ok(())}
