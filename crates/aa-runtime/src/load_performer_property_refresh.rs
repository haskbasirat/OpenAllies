//! Native57F1E0 on the same held Performer, Properties and AbilityArena.
//! Required-property lookup uses actual ordered effects; disabled duplicates
//! still satisfy581280. Enabled-byte stores survive later actions/CV errors.
use crate::{catalog::Catalog,live_object_init::Record,load_property_instances::State,
    load_performer_value_refresh::{self as value,EnvironmentSampler}};
use aa_actors::{actor_creation::{Error,Result},components::ComponentSlot};
use aa_body::{credit::Object,property_api};
use aa_mapworld::{restore::RestoredComponent,WorldInventory};
use aa_numeric::Precision;
use aa_rules::{abilities::DefinitionIdentity,native::layout_data::PropertyRef};

pub struct Context<'a>{
 pub variables:&'a mut crate::load_view_variables::State,
 pub properties:&'a State,
 pub inventory:&'a WorldInventory,
 pub current_environment:EnvironmentSampler,
}
fn error(e:impl std::fmt::Display)->Error{Error::World(e.to_string())}

/// Observe immutable completed local requirements only after validating the
/// actual Arena+Bank shared/local tokens. No ability or definition is recreated.
fn requirements(catalog:&Catalog<'_>,handle:u32,pc:Precision)->Result<Option<Vec<DefinitionIdentity>>>{
 let (name,definition,local_token)={
  let arena=catalog.ability_arena.try_borrow().map_err(error)?;let ability=arena.get(handle).map_err(error)?;
  let Some(token)=ability.local_definition else{return Ok(None)};
  let bank=catalog.definition_bank.try_borrow().map_err(error)?;let entry=bank.entry(token).map_err(error)?;
  if entry.key.domain!="actorlocalability"{return Err(Error::Unsupported("57F1E0 Ability local token belongs to another manager"))}
  (entry.key.name.clone(),ability.definition,token)
 };
 requirements_from_names(catalog,name,definition,local_token,pc,|read|{let bank=catalog.definition_bank.try_borrow().map_err(error)?;read(&bank)})
}
///Same completed local requirements with the canonical owners after staging.
pub(crate) fn requirements_in_bank(catalog:&Catalog<'_>,bank:&crate::definitions::Bank,ability:&aa_combat::ability::Ability,pc:Precision)->Result<Option<Vec<DefinitionIdentity>>>{
 let Some(token)=ability.local_definition else{return Ok(None)};
 let entry=bank.entry(token).map_err(error)?;
 if entry.key.domain!="actorlocalability"{return Err(Error::Unsupported("57F1E0 Ability local token belongs to another manager"))}
 requirements_from_names(catalog,entry.key.name.clone(),ability.definition,token,pc,|read|read(bank))
}
fn requirements_from_names(catalog:&Catalog<'_>,name:String,definition:u32,local_token:u32,pc:Precision,
 read_bank:impl FnOnce(&dyn Fn(&crate::definitions::Bank)->Result<Option<Vec<DefinitionIdentity>>>)->Result<Option<Vec<DefinitionIdentity>>>)->Result<Option<Vec<DefinitionIdentity>>>{
 let (id,ordinal,defender)=if let Some((id,index))=name.rsplit_once("/ability/"){(id,index,false)}
  else if let Some((id,index))=name.rsplit_once("/defender/"){(id,index,true)}
  else{return Err(Error::Unsupported("57F1E0 actual local source producer unavailable"))};
 let ordinal=ordinal.parse::<usize>().map_err(error)?;catalog.require_untranslated_actor_identity(id).map_err(error)?;
 let binding=catalog.actor_bindings(id,pc).map_err(error)?;let defender_source;
 let loaded=if defender{
  defender_source=catalog.actors.native_loaded_defender(id,pc,binding.character.as_ref().map(|c|c as &dyn aa_rules::abilities::AnimationResolver)).map_err(error)?;
  defender_source.as_ref().ok_or(Error::Unsupported("57F1E0 actual Defender local source absent"))?.abilities().get(ordinal)
 }else{binding.performer.as_ref().ok_or(Error::Unsupported("57F1E0 actual Performer local source absent"))?.abilities().get(ordinal)}
  .ok_or(Error::Unsupported("57F1E0 actual local source ordinal absent"))?;
 crate::saved_organization_actions::require_shared_ability_identity(catalog,&loaded.authored.shared_definition_id).map_err(error)?;
 read_bank(&|bank|{
 if bank.token("ActorLocalAbility",&name)!=Some(local_token){return Err(Error::Unsupported("57F1E0 actual local token source differs"))}
 let expected=match &loaded.authored.shared_definition_id{
  None=>bank.token("ActorLocalAbilityDefinition",&name),
  Some(aa_rules::native::TypedValue::String{value,..})=>bank.token("AbilityDefinition",value),
  _=>return Err(Error::Unsupported("57F1E0 actual shared source type differs")),
 };
 if expected!=Some(definition){return Err(Error::Unsupported("57F1E0 actual shared definition token differs"))}
 Ok(Some(loaded.required_properties.clone()))
 })
}

pub(crate) fn required_definition(catalog:&Catalog<'_>,state:&State,source:&DefinitionIdentity,pc:Precision)->Result<Object>{
 if !source.kind.eq_ignore_ascii_case("Property"){return Err(Error::Unsupported("57F1E0 required definition belongs to another manager"))}
 let data=catalog.layout_data(pc).map_err(error)?;
 let reference=data.find_property(&source.id,None).ok_or(Error::Unsupported("57F1E0 completed required Property is absent"))?;
 let PropertyRef::Registered(index)=reference else{return Err(Error::Unsupported("57F1E0 required Property resolved to embedded source"))};
 let definition=data.properties.get(index as usize).ok_or(Error::Unsupported("57F1E0 required Property ordinal absent"))?;
 if definition.sources!=source.sources{return Err(Error::Unsupported("57F1E0 required Property source provenance differs"))}
 let token=state.definition_for_reference(reference).map_err(error)?;
 if state.definition_sources(token)!=Some(source.sources.as_slice())||state.facts(token).map_err(error)?.reference()!=reference{
  return Err(Error::Unsupported("57F1E0 actual Property allocation source mapping differs"))
 }
 Ok(token)
}
fn performer<'a>(record:&'a Record<'_>)->Result<&'a aa_combat::constructors::OwnedPerformer>{
 match record.shell.actor.as_ref().unwrap().components.get(ComponentSlot::Performer){
  Some(RestoredComponent::Performer(p))if p.state.owner==record.owner.raw()=>Ok(p),
  _=>Err(Error::Unsupported("57F1E0 actual held Performer owner/payload differs")),
 }
}
struct Bound<'a,'source,'record,'context>{
 catalog:&'a Catalog<'source>,record:&'a mut Record<'record>,context:&'a mut Context<'context>,source_pc:Precision,live_pc:Precision,
}
fn combat(e:impl std::fmt::Display)->aa_combat::Error{aa_combat::Error::World(e.to_string())}
impl aa_combat::performer_property_gate::World for Bound<'_,'_,'_,'_>{
 type Requirement=DefinitionIdentity;
 fn ability_count(&self)->aa_combat::Result<usize>{Ok(performer(self.record).map_err(combat)?.state.abilities.len())}
 fn ability_at(&self,index:usize)->aa_combat::Result<u32>{performer(self.record).map_err(combat)?.state.abilities.get(index).copied().ok_or_else(||combat("57F1E0 actual primary ordinal vanished"))}
 fn required_properties(&self,ability:u32)->aa_combat::Result<Option<Vec<DefinitionIdentity>>>{requirements(self.catalog,ability,self.source_pc).map_err(combat)}
 fn has_property(&self,required:&DefinitionIdentity)->aa_combat::Result<bool>{
  let properties=match self.record.shell.actor.as_ref().unwrap().components.get(ComponentSlot::Properties){
   Some(RestoredComponent::Properties(p))if p.owner.0==self.record.owner.raw()=>p,
   None=>return Err(combat("57F1E0->581280 NULL required Properties+10 at5032D6")),
   _=>return Err(combat("57F1E0 actual Properties owner/payload differs")),
  };
  //581280 empty vector returns NULL without an instance/definition read.
  if properties.effects.is_empty(){return Ok(false)}
  let definition=required_definition(self.catalog,self.context.properties,required,self.source_pc).map_err(combat)?;
  let found=property_api::find_instance(&properties.effects,definition,None,None,|handle|{
   let instance=self.context.properties.instances().get(&handle).ok_or_else(||aa_body::Error::Callback("57F1E0 actual Property instance is absent".into()))?;
   if instance.handle!=handle{return Err(aa_body::Error::Callback("57F1E0 Property instance handle differs".into()))}
   //Native NULL source/exclude: +14 GTID and disabled+10 are not read.
   Ok((instance.definition,None))
  }).map_err(combat)?;
  Ok(found.is_some())
 }
 fn enabled(&self,handle:u32)->aa_combat::Result<bool>{Ok(self.catalog.ability_arena.try_borrow().map_err(combat)?.get(handle).map_err(combat)?.enabled)}
 fn set_enabled(&mut self,handle:u32,value:bool)->aa_combat::Result<()>{self.catalog.ability_arena.try_borrow_mut().map_err(combat)?.get_mut(handle).map_err(combat)?.enabled=value;Ok(())}
 fn refresh_actions(&mut self)->aa_combat::Result<()>{crate::load_actor_action_refresh::refresh(self.catalog,self.context.inventory,self.record,self.live_pc).map_err(combat)}
 fn refresh_value(&mut self)->aa_combat::Result<()>{value::refresh(self.catalog,self.record,&mut value::Context{variables:&mut *self.context.variables,instances:Some(self.context.properties.instances()),current_environment:self.context.current_environment},self.live_pc).map_err(combat)}
}
/// Exact primary-table gate followed by conditional full4D1350 and concrete
///57E430. Properties stays in its actual slot for both downstream callbacks.
pub fn refresh(catalog:&Catalog<'_>,record:&mut Record<'_>,context:&mut Context<'_>,live_pc:Precision)->Result<()>{
 let source_pc=crate::load_actor_init_world::validate(catalog,record)?;
 context.properties.require_catalog(catalog).map_err(error)?;
 if record.definition.component_mask.is_none_or(|m|m&(1<<5)==0){return Err(Error::Unsupported("57F1E0 Performer outside source factory"))}
 aa_combat::performer_property_gate::refresh(&mut Bound{catalog,record,context,source_pc,live_pc}).map_err(error)
}
