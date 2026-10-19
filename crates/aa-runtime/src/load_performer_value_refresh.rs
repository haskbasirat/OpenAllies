//! Native57E430 on the real held Performer/ability tables and current globals.
//! Current SVar cells come from the SAME World.variables owner. Source fields
//! and completed means remain distinct immutable Bank data; CRT controls are
//! sampled from the actual runtime thread by the supplied platform leaf.
use crate::{catalog::Catalog,live_object_init::Record,load_combat_variables::Field};
use aa_actors::{actor_creation::{Error,Result},components::ComponentSlot};
use aa_body::{F32,properties::ModifierState,credit::Object,property_effects::Instances};
use aa_combat::combat_value::{self as cv,AbilityLocal,UnitInputs};
use aa_mapworld::restore::RestoredComponent;
use aa_numeric::{Binary,Precision,crt_pow::{CrtEnvironment,PowPath,select_path}};

pub type EnvironmentSampler=fn()->std::result::Result<CrtEnvironment,String>;
/// An absent instance map means unbound, not empty. It is required only when
/// the actual Properties.effects vector contains a positive handle.
pub struct Context<'a> {
    pub variables:&'a mut crate::load_view_variables::State,
    pub instances:Option<&'a Instances>,
    pub current_environment:EnvironmentSampler,
}
fn error(e:impl std::fmt::Display)->Error {Error::World(e.to_string())}
fn combat(e:impl std::fmt::Display)->aa_combat::Error {aa_combat::Error::World(e.to_string())}
fn number(v:F32)->Result<Binary>{Binary::from_f32_bits(v.0).map_err(error)}

fn property_bias(catalog:&Catalog<'_>,_record:&Record<'_>,instance:Object,context:&Context<'_>,source_pc:Precision)->Result<F32> {
    let arena=context.instances.ok_or(Error::Unsupported("57E430 positive Properties effect requires actual pre-staging instance owner"))?;
    let instance=arena.get(&instance).ok_or(Error::Unsupported("57E430 actual Property instance identity missing"))?;
    let id={let bank=catalog.definition_bank.try_borrow().map_err(error)?;let entry=bank.entry(instance.definition.0).map_err(error)?;
        if entry.key.domain!="property" {return Err(Error::Unsupported("57E430 instance definition belongs to another native manager"))}
        entry.key.name.clone()};
    property_bias_for_id(catalog,&id,source_pc)
}
pub(crate) fn property_bias_for_id(catalog:&Catalog<'_>,id:&str,source_pc:Precision)->Result<F32>{
    let data=catalog.layout_data(source_pc).map_err(error)?;
    let entries=data.properties.iter().filter(|p|p.ids.eq_ignore_ascii_case(&id)).collect::<Vec<_>>();
    let [entry]=entries.as_slice()else{return Err(Error::Unsupported("57E430 actual registered Property identity is absent/ambiguous"))};
    if entry.reloads!=0||entry.is_technology.is_none(){return Err(Error::Unsupported("57E430 completed/reloaded Property field owner required"))}
    let origin=entry.sources.first().ok_or(Error::Unsupported("57E430 Property source origin missing"))?;
    //resolved_definitions_at: the tag's same error, else exactly the roots
    //whose first source is `origin`, in order (filtering every root).
    let roots=catalog.actors.resolved_definitions_at("Property",origin).map_err(error)?;
    let matches=roots.iter().collect::<Vec<_>>();
    let [node]=matches.as_slice()else{return Err(Error::Unsupported("57E430 Property registered source identity differs"))};
    //A definition's +28 is read regardless of instance-disabled state.
    let bits=node.field("performer_cv_bias",source_pc).map_err(error)?.ok_or(Error::Unsupported("57E430 Property+28 producer unavailable"))?.fixed_bits().map_err(error)?;
    Ok(F32(bits))
}

pub(crate) fn local_ability(catalog:&Catalog<'_>,handle:u32,source_pc:Precision)->Result<AbilityLocal> {
    let (local_name,definition,local_token)={
        let arena=catalog.ability_arena.try_borrow().map_err(error)?;let ability=arena.get(handle).map_err(error)?;
        let token=ability.local_definition.ok_or(Error::NullDereference("57E430 Ability+8 local definition"))?;
        let bank=catalog.definition_bank.try_borrow().map_err(error)?;let entry=bank.entry(token).map_err(error)?;
        if entry.key.domain!="actorlocalability" {return Err(Error::Unsupported("57E430 local Ability token belongs to another manager"))}
        (entry.key.name.clone(),ability.definition,token)
    };
    //Keep the original first Bank/arena loans ended before source binding.
    local_ability_from_names(catalog,local_name,definition,local_token,source_pc,|read|{
        let bank=catalog.definition_bank.try_borrow().map_err(error)?;read(&bank)
    })
}
///Same immutable local/shared source validation after staging moved both owners.
///The caller resolves the actual Ability handle in its canonical live arena;
///neither the Ability nor Bank is cloned or installed back into Catalog.
pub(crate) fn local_ability_in_bank(catalog:&Catalog<'_>,bank:&crate::definitions::Bank,
    ability:&aa_combat::ability::Ability,source_pc:Precision)->Result<AbilityLocal>{
    let token=ability.local_definition.ok_or(Error::NullDereference("57E430 Ability+8 local definition"))?;
    let entry=bank.entry(token).map_err(error)?;
    if entry.key.domain!="actorlocalability" {return Err(Error::Unsupported("57E430 local Ability token belongs to another manager"))}
    local_ability_from_names(catalog,entry.key.name.clone(),ability.definition,token,source_pc,|read|read(bank))
}
fn local_ability_from_names(catalog:&Catalog<'_>,local_name:String,definition:u32,local_token:u32,source_pc:Precision,
    read_bank:impl FnOnce(&dyn Fn(&crate::definitions::Bank)->Result<AbilityLocal>)->Result<AbilityLocal>)->Result<AbilityLocal>{
    let (id,index,defender)=if let Some((id,index))=local_name.rsplit_once("/ability/"){(id,index,false)}
        else if let Some((id,index))=local_name.rsplit_once("/defender/"){(id,index,true)}
        else{return Err(Error::Unsupported("57E430 actual local Ability source identity producer unavailable"))};
    let index=index.parse::<usize>().map_err(error)?;catalog.require_untranslated_actor_identity(id).map_err(error)?;
    let binding=catalog.actor_bindings(id,source_pc).map_err(error)?;
    let defender_source;
    let loaded=if defender {
        defender_source=catalog.actors.native_loaded_defender(id,source_pc,binding.character.as_ref().map(|c|c as &dyn aa_rules::abilities::AnimationResolver)).map_err(error)?;
        defender_source.as_ref().ok_or(Error::Unsupported("57E430 Defender local source missing"))?.abilities().get(index)
    }else {binding.performer.as_ref().ok_or(Error::Unsupported("57E430 Performer local source missing"))?.abilities().get(index)}
        .ok_or(Error::Unsupported("57E430 local Ability source ordinal missing"))?;
    crate::saved_organization_actions::require_shared_ability_identity(catalog,&loaded.authored.shared_definition_id).map_err(error)?;
    read_bank(&|bank|{
    if bank.token("ActorLocalAbility",&local_name)!=Some(local_token){return Err(Error::Unsupported("57E430 actual local Ability source token differs"))}
    let expected=match &loaded.authored.shared_definition_id {
        None=>bank.token("ActorLocalAbilityDefinition",&local_name),
        Some(aa_rules::native::TypedValue::String{value,..})=>bank.token("AbilityDefinition",value),
        _=>return Err(Error::Unsupported("57E430 shared Ability source type differs")),
    };
    if expected!=Some(definition){return Err(Error::Unsupported("57E430 actual Ability shared-definition token differs"))}
    let mut local=crate::combat_value_source::ability_input(loaded).map_err(error)?;
    if let Some(bits)=bank.entry(definition).map_err(error)?.range_override_bits {
        //The same Bank's explicit constructor/faction range write, not an
        //authored fallback. Other unbound shared field writers still refuse.
        local.definition.range_24=F32(bits);
    }
    Ok(local)
    })
}

struct Runtime<'a> {
    manager:&'a mut crate::combat_source_completion::State,
    variables:&'a crate::load_view_variables::State,
    modifier:Option<(u32,&'a ModifierState)>,pc:Precision,
}
impl Runtime<'_> {
    fn modifier(&self,id:u32)->aa_combat::Result<&ModifierState>{
        match self.modifier{Some((owner,value))if owner==id=>Ok(value),_=>Err(combat("45E090 actual current modifier owner differs"))}
    }
    fn clamp(&self)->F32{F32(self.variables.combat_values().bits(Field::ModifierClampAdd))}
}
impl cv::World for Runtime<'_> {
    fn ranged_minimum(&mut self)->aa_combat::Result<F32>{self.manager.bind_current_ranged_minimum();Ok(F32(self.variables.combat_values().bits(Field::RangedMinimum)))}
    fn bind_defense_weight_ranged(&mut self)->aa_combat::Result<()>{cv::World::bind_defense_weight_ranged(self.manager)}
    fn defense_weight_ranged(&mut self)->aa_combat::Result<F32>{
        if !self.manager.current_defense_weight_is_bound(){return Err(combat("runtime ranged defense read before actual cached-pointer binding"))}
        Ok(F32(self.variables.combat_values().bits(Field::DefenseWeightRanged)))}
    fn modify_projectile_range(&mut self,id:u32,base:F32)->aa_combat::Result<Binary>{
        Binary::from_f32_bits(self.modifier(id)?.projectile_range(base,self.clamp(),self.pc).map_err(combat)?.0).map_err(combat)}
    fn modify_bombard_range(&mut self,id:u32,base:F32)->aa_combat::Result<Binary>{
        Binary::from_f32_bits(self.modifier(id)?.bombard_range(base,self.clamp(),self.pc).map_err(combat)?.0).map_err(combat)}
    fn modify_power(&mut self,id:u32,base:F32,ranged:bool)->aa_combat::Result<Binary>{
        Binary::from_f32_bits(self.modifier(id)?.power(ranged,base,self.clamp(),self.pc).map_err(combat)?.0).map_err(combat)}
    fn modify_recharge(&mut self,id:u32,base:F32)->aa_combat::Result<Binary>{self.modifier(id)?.recharge(base,self.pc).map_err(combat)}
    fn log_enabled(&mut self)->aa_combat::Result<bool>{self.manager.current_log_enabled()}
}

fn evaluate(catalog:&Catalog<'_>,record:&Record<'_>,context:&mut Context<'_>,handle:u32,unit_base:[F32;4],source_pc:Precision,pc:Precision)->Result<F32> {
    let local=local_ability(catalog,handle,source_pc)?;
    //45E090's actual excluded-definition return precedes every global read.
    if local.definition.flags_54&0x80!=0{return Ok(F32::ZERO)}
    let bonus=performer_bias(catalog,&record.definition.proof.id,source_pc)?;
    let modifier=match record.shell.actor.as_ref().unwrap().components.get(ComponentSlot::Properties) {
        None=>None,
        Some(RestoredComponent::Properties(p))if p.owner.0==record.owner.raw()=>Some((record.owner.raw(),&p.modifier)),
        _=>return Err(Error::Unsupported("4CF110 actual current Properties payload/owner differs")),
    };
    let mut bank=catalog.definition_bank.try_borrow_mut().map_err(error)?;
    evaluate_in_bank(catalog,&mut bank,&local,bonus,modifier,context.variables,context.current_environment,unit_base,pc)
}
pub(crate) fn performer_bias(catalog:&Catalog<'_>,id:&str,source_pc:Precision)->Result<F32>{
    let binding=catalog.actor_bindings(id,source_pc).map_err(error)?;
    let bits=binding.performer.as_ref().ok_or(Error::Unsupported("57E430 actual ActorData+210 source missing"))?
        .authored.fields.get("cv_bias").ok_or(Error::Unsupported("57E430 actual Performer cv_bias source absent"))?.fixed_bits().map_err(error)?;
    Ok(F32(bits))
}
///Common value calculation over the actual Bank manager, live modifier and
///current SVar owner. Neither staged nor saved callers reconstruct inputs.
#[allow(clippy::too_many_arguments)]
pub(crate) fn evaluate_in_bank(catalog:&Catalog<'_>,bank:&mut crate::definitions::Bank,local:&AbilityLocal,bonus:F32,
    modifier:Option<(u32,&ModifierState)>,variables:&mut crate::load_view_variables::State,sampler:EnvironmentSampler,
    unit_base:[F32;4],pc:Precision)->Result<F32>{
    if local.definition.flags_54&0x80!=0{return Ok(F32::ZERO)}
    variables.validate_in_bank(catalog,bank).map_err(error)?;
    let manager=bank.combat_source.as_deref_mut().ok_or(Error::Unsupported("runtime45E090 actual manager owner absent"))?;
    let means=manager.completed_means().map_err(error)?;
    let settings=variables.combat_values().formula_settings();
    let unit=UnitInputs{defense:unit_base[0],health:unit_base[1],speed:unit_base[2],additive_a:unit_base[3],additive_b:bonus};
    let mut runtime=Runtime{manager,variables:&*variables,modifier,pc};
    let result=cv::ability_value_with_environment(&local,&unit,modifier.map(|m|m.0),&means,&settings,|| {
        let environment=sampler().map_err(combat)?;
        if select_path(environment)!=PowPath::Sse2||environment.mxcsr&!0x3F!=0x1F80||environment.x87_control&0x0C00!=0 {
            return Err(combat("runtime45E090 current CRT controls outside supported SSE2/nearest domain"));
        }
        Ok(environment)
    },&mut runtime,pc).map_err(error)?;
    if result.log.is_some(){return Err(Error::Unsupported("runtime45E090 CSV row requires actual retained stream writer"))}
    Ok(result.value)
}

///Storage-independent borrowed fields. Implementations access their current
///canonical allocation at each read/store; no component or Ability is copied.
pub(crate) trait Fields{
 fn property_count(&self)->Result<usize>;
 fn property_bias(&self,index:usize)->Result<F32>;
 fn set_value(&mut self,bits:u32)->Result<()>;
 fn value(&self)->Result<u32>;
 fn body(&self)->Result<(F32,F32)>;
 fn speed(&self)->Result<F32>;
 fn ability_count(&self,defender:bool)->Result<usize>;
 fn ability_at(&self,defender:bool,index:usize)->Result<u32>;
 fn enabled(&self,handle:u32)->Result<bool>;
 fn evaluate(&mut self,handle:u32,unit:[F32;4])->Result<F32>;
}
///Native57E430's property-sum, reset, stats, primary and Defender order. All
///intermediate max stores survive a later source/CRT/service failure.
pub(crate) fn refresh_fields(fields:&mut impl Fields,pc:Precision)->Result<()>{
 let count=fields.property_count()?;
 if count>i32::MAX as usize{return Err(Error::Unsupported("57E430 signed Properties count representation"))}
 let mut sum=Binary::from_u32(0);
 for index in 0..count{sum=sum.add(&number(fields.property_bias(index)?)?,pc).map_err(error)?;}
 let additive=F32(sum.store_f32().map_err(error)?);
 fields.set_value(0)?;
 let (defense,health)=fields.body()?;
 let speed=fields.speed()?;
 for defender in [false,true]{
  let count=fields.ability_count(defender)?;
  if count>i32::MAX as usize{return Err(Error::Unsupported(if defender{"57E430 signed Defender ability count representation"}else{"57E430 signed Performer ability count representation"}))}
  for index in 0..count{
   let handle=fields.ability_at(defender,index)?;
   if !fields.enabled(handle)?{continue}
   let value=fields.evaluate(handle,[defense,health,speed,additive])?;
   let prior=fields.value()?;
   let less=number(value)?.compare(&number(F32(prior))?)==std::cmp::Ordering::Less;
   //57E58C/57E640 always store, including the unchanged previous maximum.
   fields.set_value(if less{prior}else{value.0})?;
  }
 }
 Ok(())
}
struct SavedFields<'a,'source,'record,'context>{catalog:&'a Catalog<'source>,record:&'a mut Record<'record>,context:&'a mut Context<'context>,source_pc:Precision,pc:Precision}
impl SavedFields<'_,'_,'_,'_>{
 fn performer(&self)->Result<&aa_combat::constructors::OwnedPerformer>{
  match self.record.shell.actor.as_ref().unwrap().components.get(ComponentSlot::Performer){
   Some(RestoredComponent::Performer(p))if p.state.owner==self.record.owner.raw()=>Ok(p),
   _=>Err(Error::Unsupported("57E430 actual held Performer payload/owner differs")),
  }
 }
 fn properties(&self)->Result<Option<&aa_body::properties::PropertiesState>>{
  match self.record.shell.actor.as_ref().unwrap().components.get(ComponentSlot::Properties){
   None=>Ok(None),Some(RestoredComponent::Properties(p))if p.owner.0==self.record.owner.raw()=>Ok(Some(p)),
   _=>Err(Error::Unsupported("57E430 actual Properties payload/owner differs")),
  }
 }
}
impl Fields for SavedFields<'_,'_,'_,'_>{
 fn property_count(&self)->Result<usize>{Ok(self.properties()?.map_or(0,|p|p.effects.len()))}
 fn property_bias(&self,index:usize)->Result<F32>{
  let handle=*self.properties()?.and_then(|p|p.effects.get(index)).ok_or(Error::Unsupported("57E430 actual Property ordinal vanished"))?;
  property_bias(self.catalog,self.record,handle,self.context,self.source_pc)
 }
 fn set_value(&mut self,bits:u32)->Result<()>{match self.record.shell.actor.as_mut().unwrap().components.get_mut(ComponentSlot::Performer){Some(RestoredComponent::Performer(p))if p.state.owner==self.record.owner.raw()=>{p.word60=bits;Ok(())},_=>Err(Error::Unsupported("57E430 actual held Performer payload/owner differs"))}}
 fn value(&self)->Result<u32>{Ok(self.performer()?.word60)}
 fn body(&self)->Result<(F32,F32)>{match self.record.shell.actor.as_ref().unwrap().components.get(ComponentSlot::Body){None=>Ok((F32::ZERO,F32::ZERO)),Some(RestoredComponent::Body(b))if b.header.owner==self.record.owner=>Ok((b.body.defense,b.body.maximum)),_=>Err(Error::Unsupported("57E430 actual Body payload/owner differs"))}}
 fn speed(&self)->Result<F32>{match self.record.shell.actor.as_ref().unwrap().components.get(ComponentSlot::Mover){None=>Ok(F32::ZERO),Some(RestoredComponent::Mover(m))if m.header.owner==self.record.owner=>Ok(m.speed),_=>Err(Error::Unsupported("57E430 actual Mover payload/owner differs"))}}
 fn ability_count(&self,defender:bool)->Result<usize>{if !defender{return Ok(self.performer()?.state.abilities.len())}match self.record.shell.actor.as_ref().unwrap().components.get(ComponentSlot::Defender){None=>Ok(0),Some(RestoredComponent::Defender(d))if d.header.owner==self.record.owner=>Ok(d.abilities.len()),_=>Err(Error::Unsupported("57E430 actual Defender payload/owner differs"))}}
 fn ability_at(&self,defender:bool,index:usize)->Result<u32>{if !defender{return self.performer()?.state.abilities.get(index).copied().ok_or(Error::Unsupported("57E430 actual primary ordinal vanished"))}match self.record.shell.actor.as_ref().unwrap().components.get(ComponentSlot::Defender){Some(RestoredComponent::Defender(d))if d.header.owner==self.record.owner=>d.abilities.get(index).copied().ok_or(Error::Unsupported("57E430 actual Defender ordinal vanished")),_=>Err(Error::Unsupported("57E430 actual Defender payload/owner differs"))}}
 fn enabled(&self,handle:u32)->Result<bool>{Ok(self.catalog.ability_arena.try_borrow().map_err(error)?.get(handle).map_err(error)?.enabled)}
 fn evaluate(&mut self,handle:u32,unit:[F32;4])->Result<F32>{evaluate(self.catalog,self.record,self.context,handle,unit,self.source_pc,self.pc)}
}
///Both saved and stored APIs delegate to the same field kernel. The saved
///Performer box now stays in its original slot throughout the closed call.
pub fn refresh(catalog:&Catalog<'_>,record:&mut Record<'_>,context:&mut Context<'_>,pc:Precision)->Result<()> {
 let source_pc=crate::load_actor_init_world::validate(catalog,record)?;
 context.variables.validate_catalog(catalog).map_err(error)?;context.variables.require_published().map_err(error)?;
 if record.definition.component_mask.is_none_or(|m|m&(1<<5)==0){return Err(Error::Unsupported("57E430 Performer outside actual source factory"))}
 let mut fields=SavedFields{catalog,record,context,source_pc,pc};
 fields.performer()?;
 refresh_fields(&mut fields,pc)
}
