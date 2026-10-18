//! Native 5788B0/569E40 over the same stored Organization/Denizen and members.
//! A disjoint component-slot loan keeps the allocation and table presence intact;
//! the existing arithmetic kernels perform every reset and accumulation store.
use crate::{
    actor_world::{ActorAccess, ActorStore, Entity, ObjectKey, RuntimeComponent},
    catalog::Catalog,
    definitions::Bank,
    load_denizen_value_refresh::{self as denizen, Occupant},
};
use aa_actors::{
    ActorId, IdentityRegistry,
    actor_creation::{Error, Result},
    components::{ComponentSlot as Slot, OtherComponents},
    denizen_state::DenizenState,
    ground_states::OrganizationState,
};
use aa_body::F32;
use aa_numeric::Precision;
use std::collections::BTreeMap;

fn error(e: impl std::fmt::Display) -> Error { Error::World(e.to_string()) }
fn combat(e: impl std::fmt::Display) -> aa_combat::Error { aa_combat::Error::World(e.to_string()) }

fn checked(catalog:&Catalog<'_>,actors:&ActorAccess<'_>,bank:&Bank,key:ObjectKey,slot:Slot)->Result<(ActorId,Precision)>{
    catalog.require_source_owner().map_err(error)?;
    let owner=actors.id(key).ok_or_else(||error("stored value current GTID absent"))?;
    if actors.key(owner)!=Some(key){return Err(error("stored value current generation differs"))}
    let object=actors.object(key).map_err(error)?;
    if !matches!(object.definition.native_class,3|4)
        ||object.definition.native_class!=object.definition.proof.category.runtime_class
        ||object.actor.is_none()
        ||!crate::load_spatial::same_proof(&object.thing.definition,&object.definition.proof){
        return Err(error("stored value actual Actor constructor/proof differs"))
    }
    let source=crate::live_object_init::Source::bind_in_bank(catalog,bank,&object.definition)?;
    let source_pc=source.source_precision();
    let mask=catalog.canonical_component_mask(&object.definition.proof.id,source_pc).map_err(error)?;
    if object.definition.component_mask!=Some(mask)||mask&(1<<slot.index())==0{
        return Err(error("stored value actual source factory differs"))
    }
    Ok((owner,source_pc))
}

/// Read-only current actor/member view while a different slot is mutably lent.
/// `others` is only a borrowed index; no canonical Entity leaves its arena.
struct Actors<'a>{
    owner:ActorId,
    identities:&'a IdentityRegistry<ObjectKey>,
    others:BTreeMap<ObjectKey,&'a Entity>,
    current:OtherComponents<'a,RuntimeComponent>,
}
impl Actors<'_>{
    fn component(&self,id:ActorId,slot:Slot)->Result<Option<&RuntimeComponent>>{
        if id==self.owner{return Ok(self.current.get(slot))}
        let key=self.identities.get(id).ok_or_else(||error("stored value member GTID absent"))?.value;
        let entity=self.others.get(&key).ok_or_else(||error("stored value member allocation absent"))?;
        let actor=entity.runtime.as_ref().ok_or_else(||error("stored value member runtime absent"))?
            .actor.as_ref().ok_or_else(||error("stored value member Actor storage absent"))?;
        Ok(actor.components.get(slot))
    }
    fn resolves(&self,id:ActorId)->bool{!id.is_null()&&(id==self.owner||self.identities.get(id).is_some())}
    fn performer(&self,id:ActorId)->Result<Option<&aa_combat::constructors::OwnedPerformer>>{
        match self.component(id,Slot::Performer)?{
            None=>Ok(None),Some(RuntimeComponent::Performer(v))if v.state.owner==id.raw()=>Ok(Some(v)),
            _=>Err(error("stored value actual member Performer payload/owner differs")),
        }
    }
    fn health(&self,id:ActorId)->Result<Option<[F32;2]>>{
        match self.component(id,Slot::Body)?{
            None=>Ok(None),Some(RuntimeComponent::Body(v))if v.header.owner==id=>Ok(Some([v.body.health,v.body.maximum])),
            _=>Err(error("stored value actual member Body payload/owner differs")),
        }
    }
}
fn source_value(catalog:&Catalog<'_>,bank:&Bank,id:&str)->Result<F32>{
    catalog.require_untranslated_actor_identity(id).map_err(error)?;
    let canonical=catalog.actors.actors.iter().find(|v|v.id.eq_ignore_ascii_case(id))
        .ok_or_else(||error("stored value canonical ActorData absent"))?;
    let token=bank.token("ActorDefinition",id).ok_or_else(||error("stored value completed target token absent"))?;
    let source=bank.combat_source().map_err(error)?;
    if source.source_identities()!=catalog.actors.sources.as_slice()
        ||!source.definition_id(token).map_err(error)?.eq_ignore_ascii_case(&canonical.id)
        ||source.definition_source(token).map_err(error)?!=&canonical.source{
        return Err(error("stored value completed target source differs"))
    }
    source.completed_values(token).map(|v|v.value_1e4).map_err(error)
}

struct Organization<'a,'source>{catalog:&'a Catalog<'source>,bank:&'a Bank,actors:Actors<'a>,value:&'a mut OrganizationState<String>,source_pc:Precision}
impl aa_combat::organization_combat_value::World for Organization<'_,'_>{
    type Members=();type Member=ActorId;type Performer=ActorId;type Slot=usize;
    fn member_count(&mut self)->aa_combat::Result<i32>{i32::try_from(self.value.members.len()).map_err(|_|combat("stored5788B0 signed member count overflow"))}
    fn member_array(&mut self)->aa_combat::Result<()>{Ok(())}
    fn member(&mut self,_:(),index:i32)->aa_combat::Result<Option<ActorId>>{
        let id=*self.value.members.get(usize::try_from(index).map_err(combat)?)
            .ok_or_else(||combat("stored5788B0 member outside actual vector"))?;
        Ok((!id.is_null()).then_some(id))
    }
    fn performer(&mut self,id:ActorId)->aa_combat::Result<Option<ActorId>>{Ok(self.actors.performer(id).map_err(combat)?.map(|_|id))}
    fn performer_health(&mut self,id:ActorId)->aa_combat::Result<Option<[F32;2]>>{
        let owner=self.actors.performer(id).map_err(combat)?.ok_or_else(||combat("stored57DC40 Performer vanished"))?.state.owner;
        self.actors.health(ActorId::from_raw(owner)).map_err(combat)
    }
    fn performer_value(&mut self,id:ActorId)->aa_combat::Result<F32>{
        Ok(F32(self.actors.performer(id).map_err(combat)?.ok_or_else(||combat("stored5788B0 Performer vanished"))?.word60))
    }
    fn slot(&mut self,index:i32)->aa_combat::Result<Option<usize>>{
        let info=aa_mapworld::restore::RestoreDefinitions::organization_layout_info(self.catalog,&self.value.layout.definition,self.source_pc).map_err(combat)?;
        let group=*info.slot_groups.get(usize::try_from(index).map_err(combat)?).ok_or_else(||combat("stored5788B0 layout slot outside current groups"))? as usize;
        let value=self.value.layout.parameters.get(group).ok_or_else(||combat("stored5788B0 parameter outside actual layout"))?;
        Ok(value.as_ref().map(|_|group))
    }
    fn slot_flags(&mut self,slot:usize)->aa_combat::Result<u32>{
        let id=self.value.layout.parameters.get(slot).and_then(Option::as_deref).ok_or_else(||combat("stored5788B0 selected parameter absent"))?;
        self.catalog.canonical_component_mask(id,self.source_pc).map_err(combat)
    }
    fn slot_value(&mut self,slot:usize)->aa_combat::Result<F32>{
        let id=self.value.layout.parameters.get(slot).and_then(Option::as_deref).ok_or_else(||combat("stored5788B0 selected parameter absent"))?;
        source_value(self.catalog,self.bank,id).map_err(combat)
    }
    fn current(&mut self)->aa_combat::Result<F32>{self.value.words.get(&0x68).copied().map(F32).ok_or_else(||combat("stored Organization+68 uninitialized"))}
    fn maximum(&mut self)->aa_combat::Result<F32>{self.value.words.get(&0x6c).copied().map(F32).ok_or_else(||combat("stored Organization+6C uninitialized"))}
    fn store_current(&mut self,v:F32)->aa_combat::Result<()>{self.value.words.insert(0x68,v.0);Ok(())}
    fn store_maximum(&mut self,v:F32)->aa_combat::Result<()>{self.value.words.insert(0x6c,v.0);Ok(())}
}
struct Denizen<'a,'source>{catalog:&'a Catalog<'source>,bank:&'a Bank,actors:Actors<'a>,value:&'a mut DenizenState,source_pc:Precision}
impl denizen::World for Denizen<'_,'_>{
    fn count(&self)->usize{self.value.inventory.len()}
    fn occupant(&self,index:usize)->Result<Occupant>{
        let id=ActorId::from_raw(self.value.inventory[index].word_00);
        if !self.actors.resolves(id){return Ok(Occupant::Absent)}
        let Some(value)=self.actors.performer(id)?else{return Ok(Occupant::Present{value:None,health:None})};
        Ok(Occupant::Present{value:Some(value.word60),health:self.actors.health(id)?.map(|v|v.map(|v|v.0))})
    }
    fn target_flags(&self,index:usize)->Result<Option<u32>>{self.catalog.canonical_component_mask(&self.value.inventory[index].target,self.source_pc).map(Some).map_err(error)}
    fn target_health(&self,index:usize)->Result<u32>{
        let id=&self.value.inventory[index].target;self.catalog.require_untranslated_actor_identity(id).map_err(error)?;
        self.catalog.actors.native_ground_definition(id,self.source_pc).map_err(error)?.attributes.body
            .map(|v|v.health.bits).ok_or_else(||error("stored45D730 target ActorData+284 unavailable"))
    }
    fn slot_value(&self,index:usize)->Result<u32>{Ok(self.value.inventory[index].value_08.0)}
    fn target_value(&self,index:usize)->Result<u32>{source_value(self.catalog,self.bank,&self.value.inventory[index].target).map(|v|v.0)}
    fn current(&self)->Result<u32>{self.value.words.get(&0x10).copied().ok_or_else(||error("stored Denizen+10 uninitialized"))}
    fn maximum(&self)->Result<u32>{self.value.words.get(&0x14).copied().ok_or_else(||error("stored Denizen+14 uninitialized"))}
    fn set_current(&mut self,v:u32){self.value.words.insert(0x10,v);}
    fn set_maximum(&mut self,v:u32){self.value.words.insert(0x14,v);}
}

pub fn refresh_organization_on_owners(catalog:&Catalog<'_>,actors:&mut ActorAccess<'_>,bank:&Bank,key:ObjectKey,live_pc:Precision)->Result<()>{
    let(owner,source_pc)=checked(catalog,actors,bank,key,Slot::Organization)?;
    let(_,focus)=actors.property_focus(key).map_err(error)?;
    let actor=focus.entity.runtime.as_mut().ok_or_else(||error("stored Organization actual runtime absent"))?
        .actor.as_mut().ok_or_else(||error("stored Organization Actor storage absent"))?;
    let(component,current)=actor.components.split_slot(Slot::Organization);
    let Some(RuntimeComponent::Organization(value))=component else{return Err(error("stored Organization actual typed slot absent"))};
    if value.header.owner!=owner{return Err(error("stored Organization current component owner differs"))}
    aa_combat::organization_combat_value::recompute(&mut Organization{catalog,bank,value,source_pc,
        actors:Actors{owner,identities:focus.identities,others:focus.others,current}},live_pc).map_err(error)
}
pub fn refresh_denizen_on_owners(catalog:&Catalog<'_>,actors:&mut ActorAccess<'_>,bank:&Bank,key:ObjectKey,live_pc:Precision)->Result<()>{
    let(owner,source_pc)=checked(catalog,actors,bank,key,Slot::Denizen)?;
    let(_,focus)=actors.property_focus(key).map_err(error)?;
    let actor=focus.entity.runtime.as_mut().ok_or_else(||error("stored Denizen actual runtime absent"))?
        .actor.as_mut().ok_or_else(||error("stored Denizen Actor storage absent"))?;
    let(component,current)=actor.components.split_slot(Slot::Denizen);
    let Some(RuntimeComponent::Denizen(value))=component else{return Err(error("stored Denizen actual typed slot absent"))};
    if value.header.owner!=owner{return Err(error("stored Denizen current component owner differs"))}
    denizen::recompute(&mut Denizen{catalog,bank,value,source_pc,
        actors:Actors{owner,identities:focus.identities,others:focus.others,current}},live_pc)
}
pub fn refresh_organization(catalog:&Catalog<'_>,actors:&mut ActorStore,bank:&Bank,key:ObjectKey,live_pc:Precision)->Result<()>{
    let(_,mut access)=actors.split_minds();refresh_organization_on_owners(catalog,&mut access,bank,key,live_pc)
}
pub fn refresh_denizen(catalog:&Catalog<'_>,actors:&mut ActorStore,bank:&Bank,key:ObjectKey,live_pc:Precision)->Result<()>{
    let(_,mut access)=actors.split_minds();refresh_denizen_on_owners(catalog,&mut access,bank,key,live_pc)
}
