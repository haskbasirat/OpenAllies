//! Native5817A0/581630 after staging, on the same moved Property owner.
//!
//! The Bank, instance directory, Properties box, component table and Ability
//! arena remain in their canonical owners. A failed child retains every prior
//! allocation, attachment and numeric store. This direct insertion entry is
//! for native callers that have already performed any4CEF10 admission.
use crate::{actor_world::{ActorStore,ObjectKey,RuntimeComponent},catalog::Catalog,
    definitions::Bank,load_post_create_world::InitialProperty,
    load_property_instances::{self as property,State},property_fields::Fields,
    restoration::Staged};
use aa_actors::{ActorId,components::ComponentSlot as Slot};
use aa_body::{credit::Object,property_effects::Instances};
use aa_numeric::Precision;
use aa_rules::native::layout_data::PropertyRef;
type Result<T> = std::result::Result<T,String>;
fn text(e:impl std::fmt::Display)->String{e.to_string()}

///Disjoint loans of the retained native owners. This does not authorize a
///battle or manufacture World state; Staged supplies these same fields below.
pub struct Context<'a>{
    pub actors:&'a mut ActorStore,pub bank:&'a mut Bank,
    pub variables:&'a mut crate::load_view_variables::State,
    pub clock:&'a aa_simulation::NativeClock,
}

/// Validate the actual stored target and moved source Bank. This is ownership
/// validation, not a new gameplay eligibility predicate.
fn validate(catalog:&Catalog<'_>,bank:&Bank,actors:&ActorStore,key:ObjectKey)->Result<ActorId>{
    catalog.require_source_owner()?;
    let source=bank.combat_source()?;
    source.completed_means()?;
    if source.source_identities()!=catalog.actors.sources.as_slice(){return Err("staged Property Bank source universe differs".into())}
    let id=actors.id(key).ok_or("5817A0 current stored GTID is absent")?;
    let object=actors.object(key).map_err(text)?;
    let binding=catalog.actor_bindings(&object.definition.proof.id,source.source_precision())?;
    if !matches!(object.definition.native_class,3|4)
        || !crate::load_spatial::same_proof(&object.thing.definition,&object.definition.proof)
        || !crate::load_spatial::same_proof(&binding.definition.proof,&object.definition.proof)
        || object.definition.component_mask!=binding.definition.component_mask
        || object.definition.flags!=binding.definition.flags{
        return Err("5817A0 stored Actor factory/source proof differs".into())
    }
    let actor=object.actor.as_ref().ok_or("5817A0 stored Actor allocation is absent")?;
    match actor.components.get(Slot::Properties){
        Some(RuntimeComponent::Properties(p))if p.owner.0==id.raw()=>Ok(id),
        _=>Err("5817A0 stored Properties payload/owner differs".into()),
    }
}

struct Numeric<'a>{variables:&'a mut crate::load_view_variables::State}
impl property::Effects for Numeric<'_>{
    fn minimum_multiplier(&mut self)->aa_body::Result<aa_body::F32>{Ok(aa_body::F32(self.variables.modifier_clamp_multiply_bits()))}
    fn numeric_changed(&mut self,catalog:&Catalog<'_>,fields:&mut Fields<'_>,modifier:&aa_body::property_effects::NumericModifier,
        properties:&mut aa_body::properties::PropertiesState,instances:&mut Instances)->aa_body::Result<()>{
        crate::load_property_numeric::changed(catalog,fields,modifier,properties,instances,self.variables)
    }
    fn has_character(&self,fields:&Fields<'_>)->aa_body::Result<bool>{Ok(fields.components.present(Slot::Character))}
}

///5787F0's lazy actual member guards. A skipped member never demands an
///instance definition. Positive4CEF10 remains required until its concrete
///stored admission/application callback is bound.
fn organization_added(catalog:&Catalog<'_>,bank:&Bank,actors:&ActorStore,state:&State,
    key:ObjectKey,instance:Object)->Result<()>{
    let owner=actors.id(key).ok_or("5787F0 stored owner GTID is absent")?;
    let mut index=0usize;
    loop{
        let actor=actors.object(key).map_err(text)?.actor.as_ref().ok_or("5787F0 stored Actor absent")?;
        let org=match actor.components.get(Slot::Organization){
            Some(RuntimeComponent::Organization(org))if org.header.owner==owner=>org,
            _=>return Err("5787F0 actual Organization payload/owner differs".into()),
        };
        let Some(&member)=org.members.get(index)else{return Ok(())};
        if !member.is_null(){
            let member_key=actors.key(member).ok_or("5787F0 current member generation is absent")?;
            let actor=actors.object(member_key).map_err(text)?.actor.as_ref().ok_or("5787F0 member Actor storage absent")?;
            if let Some(component)=actor.components.get(Slot::Properties){
                if !matches!(component,RuntimeComponent::Properties(p)if p.owner.0==member.raw()){
                    return Err("5787F0 member Properties payload/owner differs".into())
                }
                state.require_in_bank(catalog,bank)?;
                let value=state.instances().get(&instance).ok_or("5787F0 current instance+C is absent")?;
                if state.facts(value.definition)?.pass_down_members()
                    && !(actor.components.present(Slot::Experience)&&value.kind==7)&&!value.disabled{
                    return Err(format!("5787F0 reached positive stored4CEF10 member {:#010x}, Property {}, kind {}, source {:#010x}",
                        member.raw(),value.definition.0,value.kind,owner.raw()))
                }
            }
        }
        index=index.checked_add(1).ok_or("5787F0 stored member index overflow")?;
    }
}
fn present(actors:&ActorStore,key:ObjectKey,slot:Slot)->Result<bool>{
    Ok(actors.object(key).map_err(text)?.actor.as_ref().ok_or("4CEAC0 stored Actor absent")?.components.present(slot))
}
///The final4CEAC0 dispatch rereads each current slot after its preceding child.
///This runs after the insertion's disjoint Properties/component loan ends.
fn notify(catalog:&Catalog<'_>,bank:&Bank,actors:&mut ActorStore,key:ObjectKey,instance:Object)->Result<()>{
    if present(actors,key,Slot::Organization)?{
        organization_added(catalog,bank,actors,&actors.property_allocations,key,instance)?;
    }
    if present(actors,key,Slot::Settlement)?{
        return Err("584F80 stored Settlement+14/+18/+1C/+24/+28 allocation is required before its lazy property gate".into());
    }
    if present(actors,key,Slot::Performer)?{
        return Err("57F4C0/57F1E0 same stored Performer/Abilities/Properties refresh is required".into());
    }
    Ok(())
}

///Direct native5817A0 on the live stored owner. Callers of generic4CEF10 must
///first run its real5035E0 admission; this function does not imply admission.
#[allow(clippy::too_many_arguments)]
pub fn add(staged:&mut Staged,catalog:&Catalog<'_>,key:ObjectKey,reference:PropertyRef,
    kind:u32,source:Option<ActorId>,pc:Precision)->Result<()>{
    let world=staged.load_world.as_mut().ok_or("5817A0 requires actual World SVar owner")?;
    add_on_owners(&mut Context{actors:&mut staged.actors,bank:&mut staged.definitions,
        variables:&mut world.variables,clock:&staged.clock},catalog,key,reference,kind,source,pc)
}
#[allow(clippy::too_many_arguments)]
pub fn add_on_owners(context:&mut Context<'_>,catalog:&Catalog<'_>,key:ObjectKey,reference:PropertyRef,
    kind:u32,source:Option<ActorId>,pc:Precision)->Result<()>{
    let owner=validate(catalog,context.bank,context.actors,key)?;
    let tail;
    {
        let (state,mut actors)=context.actors.split_property_state();
        state.bind_in_bank(catalog,context.bank)?;
        let definition=state.definition_for_reference(reference)?;
        //5030F0 precedes allegiance, duplicate selection and all effect stores.
        //The clock closure reads the SAME World+E0 after handle reservation.
        let instance=property::construct_instance(state,definition,kind,source,||Ok(context.clock.begin.0))?;
        let focus=actors.focus(key).map_err(text)?;
        if focus.id!=owner{return Err("5817A0 current GTID changed before insertion".into())}
        let object=focus.entity.runtime.as_mut().ok_or("5817A0 current runtime allocation absent")?;
        let actor=object.actor.as_mut().ok_or("5817A0 current Actor allocation absent")?;
        let (properties,components)=actor.components.split_slot(Slot::Properties);
        let Some(RuntimeComponent::Properties(properties))=properties else{return Err("5817A0 current Properties allocation changed".into())};
        tail=property::insert_preconstructed(catalog,Fields::stored(&object.definition,pc,owner,&mut actor.words,
            components,focus.identities,focus.others,context.bank,focus.abilities),
            properties,state,instance,pc,&mut Numeric{variables:context.variables})?;
    }
    if let Some(instance)=tail{notify(catalog,context.bank,context.actors,key,instance)?;}
    Ok(())
}
///Native initial5817A0(property,0,NULL), with the exact source identity carried
///by the actor's completed initial property list.
pub fn initial(staged:&mut Staged,catalog:&Catalog<'_>,key:ObjectKey,property:&InitialProperty,pc:Precision)->Result<()>{
    {
        let state=&mut staged.actors.property_allocations;
        state.bind_in_bank(catalog,&mut staged.definitions)?;
        let definition=state.definition_for_reference(property.reference)?;
        if state.definition_sources(definition)!=Some(property.sources.as_slice()){
            return Err("stored initial Property source identity differs".into())
        }
    }
    add(staged,catalog,key,property.reference,0,None,pc)
}
