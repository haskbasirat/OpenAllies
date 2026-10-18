//! Portable source/owner binding shared by stored Mover/Element/Economy Init leaves.
use crate::{actor_world::{ActorStore,ObjectKey},catalog::Catalog,definitions::Bank};
use aa_actors::{ActorId,actor_creation::{Error,Result},components::ComponentSlot as Slot,loading::LoadKind};
use aa_numeric::Precision;
fn error(e:impl std::fmt::Display)->Error{Error::World(e.to_string())}
pub(crate) fn checked(catalog:&Catalog<'_>,actors:&ActorStore,bank:&Bank,key:ObjectKey,slot:Slot)->Result<(ActorId,Precision)>{
 if !matches!(slot,Slot::Mover|Slot::Element|Slot::Economy){return Err(error("stored component Init unsupported concrete class"))}
 let owner=actors.id(key).ok_or_else(||error("stored component Init current GTID absent"))?;
 if actors.key(owner)!=Some(key){return Err(error("stored component Init registry generation differs"))}
 let object=actors.object(key).map_err(error)?;
 if object.kind!=LoadKind::Actor||object.object.is_none()||object.actor.is_none()
  ||object.definition.native_class!=3||!crate::load_spatial::same_proof(&object.thing.definition,&object.definition.proof){return Err(error("stored component Init actual Actor constructor/proof differs"))}
 let source=crate::live_object_init::Source::bind_in_bank(catalog,bank,&object.definition)?;
 let pc=source.source_precision();
 let mask=catalog.canonical_component_mask(&object.definition.proof.id,pc).map_err(error)?;
 if object.definition.component_mask!=Some(mask)||mask&(1<<slot.index())==0{return Err(error("stored component Init source factory mask differs"))}
 if let Some(binding)=&actors.entity(key).map_err(error)?.bindings{
  let expected=catalog.actor_bindings(&object.definition.proof.id,pc).map_err(error)?;
  if !std::sync::Arc::ptr_eq(binding,&expected){return Err(error("stored component Init cached source allocation differs"))}
 }
 Ok((owner,pc))
}
