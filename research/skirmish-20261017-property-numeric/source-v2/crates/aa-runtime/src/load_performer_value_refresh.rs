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

fn property_bias(catalog:&Catalog<'_>,record:&Record<'_>,instance:Object,context:&Context<'_>,source_pc:Precision)->Result<F32> {
    let arena=context.instances.ok_or(Error::Unsupported("57E430 positive Properties effect requires actual pre-staging instance owner"))?;
    let instance=arena.get(&instance).ok_or(Error::Unsupported("57E430 actual Property instance identity missing"))?;
    let id={let bank=catalog.definition_bank.try_borrow().map_err(error)?;let entry=bank.entry(instance.definition.0).map_err(error)?;
        if entry.key.domain!="property" {return Err(Error::Unsupported("57E430 instance definition belongs to another native manager"))}
        entry.key.name.clone()};
    let data=catalog.layout_data(source_pc).map_err(error)?;
    let entries=data.properties.iter().filter(|p|p.ids.eq_ignore_ascii_case(&id)).collect::<Vec<_>>();
    let [entry]=entries.as_slice()else{return Err(Error::Unsupported("57E430 actual registered Property identity is absent/ambiguous"))};
    if entry.reloads!=0||entry.is_technology.is_none(){return Err(Error::Unsupported("57E430 completed/reloaded Property field owner required"))}
    let origin=entry.sources.first().ok_or(Error::Unsupported("57E430 Property source origin missing"))?;
    let roots=catalog.actors.resolved_definitions("Property").map_err(error)?;
    let matches=roots.iter().filter(|n|n.sources().first()==Some(origin)).collect::<Vec<_>>();
    let [node]=matches.as_slice()else{return Err(Error::Unsupported("57E430 Property registered source identity differs"))};
    //A definition's +28 is read regardless of instance-disabled state.
    let bits=node.field("performer_cv_bias",source_pc).map_err(error)?.ok_or(Error::Unsupported("57E430 Property+28 producer unavailable"))?.fixed_bits().map_err(error)?;
    let _=record;Ok(F32(bits))
}

pub(crate) fn local_ability(catalog:&Catalog<'_>,handle:u32,source_pc:Precision)->Result<AbilityLocal> {
    let (local_name,definition,local_token)={
        let arena=catalog.ability_arena.try_borrow().map_err(error)?;let ability=arena.get(handle).map_err(error)?;
        let token=ability.local_definition.ok_or(Error::NullDereference("57E430 Ability+8 local definition"))?;
        let bank=catalog.definition_bank.try_borrow().map_err(error)?;let entry=bank.entry(token).map_err(error)?;
        if entry.key.domain!="actorlocalability" {return Err(Error::Unsupported("57E430 local Ability token belongs to another manager"))}
        (entry.key.name.clone(),ability.definition,token)
    };
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
    let bank=catalog.definition_bank.try_borrow().map_err(error)?;
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
    let binding=catalog.actor_bindings(&record.definition.proof.id,source_pc).map_err(error)?;
    let bonus=binding.performer.as_ref().ok_or(Error::Unsupported("57E430 actual ActorData+210 source missing"))?
        .authored.fields.get("cv_bias").ok_or(Error::Unsupported("57E430 actual Performer cv_bias source absent"))?.fixed_bits().map_err(error)?;
    let modifier=match record.shell.actor.as_ref().unwrap().components.get(ComponentSlot::Properties) {
        None=>None,
        Some(RestoredComponent::Properties(p))if p.owner.0==record.owner.raw()=>Some((record.owner.raw(),&p.modifier)),
        _=>return Err(Error::Unsupported("4CF110 actual current Properties payload/owner differs")),
    };
    let sampler=context.current_environment;
    let mut bank=catalog.definition_bank.try_borrow_mut().map_err(error)?;
    context.variables.validate_in_bank(catalog,&bank).map_err(error)?;
    let manager=bank.combat_source.as_deref_mut().ok_or(Error::Unsupported("runtime45E090 actual manager owner absent"))?;
    let means=manager.completed_means().map_err(error)?;
    let settings=context.variables.combat_values().formula_settings();
    let unit=UnitInputs{defense:unit_base[0],health:unit_base[1],speed:unit_base[2],additive_a:unit_base[3],additive_b:F32(bonus)};
    let mut runtime=Runtime{manager,variables:&*context.variables,modifier,pc};
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

/// Evaluate both actual ability tables in native order and store each maximum
/// immediately. Same Performer allocation returns on every normal Result path.
pub fn refresh(catalog:&Catalog<'_>,record:&mut Record<'_>,context:&mut Context<'_>,pc:Precision)->Result<()> {
    let source_pc=crate::load_actor_init_world::validate(catalog,record)?;
    context.variables.validate_catalog(catalog).map_err(error)?;context.variables.require_published().map_err(error)?;
    if record.definition.component_mask.is_none_or(|m|m&(1<<5)==0){return Err(Error::Unsupported("57E430 Performer outside actual source factory"))}
    match record.shell.actor.as_ref().unwrap().components.get(ComponentSlot::Performer) {
        Some(RestoredComponent::Performer(p))if p.state.owner==record.owner.raw()=>(),
        _=>return Err(Error::Unsupported("57E430 actual held Performer payload/owner differs")),
    }
    let component=record.shell.actor.as_mut().unwrap().components.remove(ComponentSlot::Performer).unwrap();
    let RestoredComponent::Performer(mut performer)=component else{unreachable!()};
    let outcome=(||{
        let mut property_sum=Binary::from_u32(0);
        match record.shell.actor.as_ref().unwrap().components.get(ComponentSlot::Properties) {
            None=>(),
            Some(RestoredComponent::Properties(p))if p.owner.0==record.owner.raw()=>{
                if p.effects.len()>i32::MAX as usize{return Err(Error::Unsupported("57E430 signed Properties count representation"))}
                for instance in &p.effects {property_sum=property_sum.add(&number(property_bias(catalog,record,*instance,context,source_pc)?)?,pc).map_err(error)?;}
            },_=>return Err(Error::Unsupported("57E430 actual Properties payload/owner differs")),
        }
        let additive=F32(property_sum.store_f32().map_err(error)?);
        performer.word60=0; //57E4C9, after the complete property-sum prefix.
        let actor=record.shell.actor.as_ref().unwrap();
        let (defense,health)=match actor.components.get(ComponentSlot::Body){None=>(F32::ZERO,F32::ZERO),
            Some(RestoredComponent::Body(b))if b.header.owner==record.owner=>(b.body.defense,b.body.maximum),
            _=>return Err(Error::Unsupported("57E430 actual Body payload/owner differs"))};
        let speed=match actor.components.get(ComponentSlot::Mover){None=>F32::ZERO,
            Some(RestoredComponent::Mover(m))if m.header.owner==record.owner=>m.speed,
            _=>return Err(Error::Unsupported("57E430 actual Mover payload/owner differs"))};
        if performer.state.abilities.len()>i32::MAX as usize{return Err(Error::Unsupported("57E430 signed Performer ability count representation"))}
        let mut evaluate_table=|table:&[u32]|->Result<()> {
            for handle in table {
                let enabled=catalog.ability_arena.try_borrow().map_err(error)?.get(*handle).map_err(error)?.enabled;
                if !enabled{continue}
                let value=evaluate(catalog,record,context,*handle,[defense,health,speed,additive],source_pc,pc)?;
                let less=number(value)?.compare(&number(F32(performer.word60))?)==std::cmp::Ordering::Less;
                performer.word60=if less{performer.word60}else{value.0};
            }Ok(())
        };
        evaluate_table(&performer.state.abilities)?;
        match actor.components.get(ComponentSlot::Defender){None=>(),
            Some(RestoredComponent::Defender(d))if d.header.owner==record.owner=>{
                if d.abilities.len()>i32::MAX as usize{return Err(Error::Unsupported("57E430 signed Defender ability count representation"))}
                evaluate_table(&d.abilities)?;
            },_=>return Err(Error::Unsupported("57E430 actual Defender payload/owner differs")),
        }
        Ok(())
    })();
    assert!(record.shell.actor.as_mut().unwrap().components.insert(ComponentSlot::Performer,RestoredComponent::Performer(performer)).is_none());
    outcome
}
