//! Concrete7A0C20 notifications on the same published Actor allocation.
//! Non-MARKER classification3/4 branches bind7A0390 and the category3
//!79F6E0 return. Ordinary unit/controller effects remain required when reached.
use crate::{actor_world::{ActorStore,RuntimeObject},catalog::Catalog,
    definitions::Bank,spatial_world::SpatialWorld};
use aa_actors::{ActorId,components::ComponentSlot,loading::LoadKind};
use aa_mind::{sai::actor_notify,Error,Result};
use aa_navigation::{sai_state::{ActorClassFields,classification_three},startup::Fields};

fn error(e:impl std::fmt::Display)->Error {Error::World(e.to_string())}

trait State {
    fn owner_word(&self)->Result<u32>;
    fn category(&self)->Result<u32>;
    fn class_three(&self)->Result<bool>;
    fn flare(&self)->Result<bool>;
    fn require_non_marker(&self)->Result<()>;
    fn set_category(&mut self,value:u32)->Result<()>;
    fn ordinary(&mut self,_category:u32,_owner:u32)->Result<()> {Err(error("7A0390 ordinary category requires actual79F500 controller/goal ownership"))}
}
fn register(w:&mut impl State,requested:u32)->Result<()> {
    let owner=w.owner_word()?; //actual virtual108 before classification
    let mut category=requested;
    if w.class_three()? {category=3;}
    if w.flare()? {category=4;}
    w.set_category(category)?; //4CEEE0 precedes the MARKER branch
    w.require_non_marker()?;
    match category {3|4=>Ok(()),_=>w.ordinary(category,owner)}
}
fn unregister(w:&mut impl State)->Result<()> {
    let _owner=w.owner_word()?;
    let previous=w.category()?;
    w.require_non_marker()?;
    if previous!=3 {
        return Err(error("79F6E0 requires actual ordinary/Flare reference/list retirement"));
    }
    w.set_category(5)
}

enum Controllers<'a> {
    Read {slots:&'a [Option<u32>],arena:&'a crate::load_sai_players::Arena},
    World {slots:&'a [Option<u32>],arena:&'a mut crate::load_sai_players::Arena,
        map:&'a mut crate::load_sai_world::Map,manager90:&'a mut Vec<u32>,kingdoms:&'a crate::load_kingdom_init_world::KingdomRegistryState},
}
impl Controllers<'_> {
    fn parts(&self)->(&[Option<u32>],&crate::load_sai_players::Arena) {match self {Self::Read {slots,arena}=>(slots,arena),Self::World {slots,arena,..}=>(slots,arena)}}
}
struct World<'a,'source,F> {
    catalog:&'a Catalog<'source>,bank:&'a Bank,actors:&'a mut ActorStore,
    spatial:&'a SpatialWorld,ready:&'a mut F,actor:ActorId,
    players:Option<Controllers<'a>>,
}
impl<F:Fields> World<'_,'_,F> {
    fn object(&self)->Result<&RuntimeObject> {
        self.object_for(self.actor)
    }
    fn object_for(&self,actor:ActorId)->Result<&RuntimeObject> {
        self.catalog.require_source_owner().map_err(error)?;
        let source=self.bank.combat_source().map_err(error)?;
        source.completed_means().map_err(error)?;
        if source.source_identities()!=self.catalog.actors.sources {
            return Err(error("SAI actor source Bank/catalog ownership differs"));
        }
        let key=self.actors.key(actor).ok_or_else(||error("SAI actor current generation absent"))?;
        let object=self.actors.object(key).map_err(error)?;
        let proof=&object.definition.proof;
        self.catalog.require_untranslated_actor_identity(&proof.id).map_err(error)?;
        let canonical=self.catalog.actors.find(&proof.id).ok_or_else(||error("SAI actor canonical source absent"))?;
        let token=self.bank.token("ActorDefinition",&proof.id).ok_or_else(||error("SAI actor source token absent"))?;
        source.completed_values(token).map_err(error)?;
        if !matches!(object.definition.native_class,3|4) || object.kind!=LoadKind::Actor
            || object.actor.is_none() || object.object.is_none()
            || canonical.source!=proof.source || canonical.template_chain!=proof.template_chain
            || self.catalog.actors.native_category(&proof.id).map_err(error)?!=proof.category
            || !crate::load_spatial::same_proof(&object.thing.definition,proof)
            || !source.definition_id(token).map_err(error)?.eq_ignore_ascii_case(&proof.id)
            || source.definition_source(token).map_err(error)?!=&proof.source
            || object.definition.component_mask!=Some(self.catalog.canonical_component_mask(&proof.id,source.source_precision()).map_err(error)?) {
            return Err(error("SAI actor actual constructor/source certificate differs"));
        }
        let spatial_key=self.spatial.key(key).map_err(error)?;
        if self.spatial.object(spatial_key).map_err(error)?!=key
            || self.spatial.index().reference_for_key(spatial_key).map_err(error)?!=actor.raw() {
            return Err(error("SAI actor published spatial identity differs"));
        }
        for index in 0..31 {
            let slot=ComponentSlot::from_index(index).unwrap();
            if let Some(component)=object.actor.as_ref().unwrap().components.get(slot)
                && component.slot()!=slot {
                return Err(error("SAI actor component allocation/slot differs"));
            }
        }
        Ok(object)
    }
    fn flags(&self)->Result<u32> {
        self.flags_for(self.actor)
    }
    fn flags_for(&self,actor:ActorId)->Result<u32> {
        let _=self.object_for(actor)?;
        let key=self.actors.key(actor).unwrap();
        Ok(self.actors.placement(key,self.spatial).map_err(error)?.flags)
    }
    fn element_owner(&self,actor:ActorId)->Result<Option<ActorId>> {
        let c=&self.object_for(actor)?.actor.as_ref().unwrap().components;
        let Some(value)=c.get(ComponentSlot::Element) else {return Ok(None)};
        let crate::actor_world::RuntimeComponent::Element(value)=value else {return Err(error("SAI Element payload differs"))};
        if value.header.owner!=actor {return Err(error("SAI actual Element owner differs"))}
        if value.organization.is_some_and(ActorId::is_null) {return Err(error("SAI Element has a noncanonical Some(NULL) organization"))}
        Ok(value.organization)
    }
    fn mind_word28(&self,actor:ActorId)->Result<Option<u32>> {
        let c=&self.object_for(actor)?.actor.as_ref().unwrap().components;
        let Some(value)=c.get(ComponentSlot::Mind) else {return Ok(None)};
        let crate::actor_world::RuntimeComponent::Mind(value)=value else {return Err(error("SAI Mind payload differs"))};
        let mind=self.actors.minds.mind(value.mind_id)?;
        if mind.actor!=actor.raw() {return Err(error("SAI actual installed Mind owner differs"))}
        Ok(Some(mind.words_24_30[1]))
    }
    fn active(&self,actor:ActorId,seen:&mut std::collections::BTreeSet<ActorId>)->Result<bool> {
        let flags=self.flags_for(actor)?;
        if flags&4!=0 {return Ok(false)}
        if !seen.insert(actor) {return Err(error("SAI actual Element organization recursion is cyclic"))}
        let contained=match self.element_owner(actor)? {Some(parent)=>Some(self.active(parent,seen)?),None=>None};
        let c=&self.object_for(actor)?.actor.as_ref().unwrap().components;
        Ok(aa_navigation::sai_state::actor_active_for_sai(aa_navigation::sai_state::ActorActivity {
            flags8:flags,contained_object_result:contained,component6c:c.present(ComponentSlot::Mover),
            componentac:c.present(ComponentSlot::MobileDeployment),componenta0:c.present(ComponentSlot::Recruit),component88:c.present(ComponentSlot::Denizen)}))
    }
    fn water_single(&self,actor:ActorId)->Result<bool> {
        let flags=self.flags_for(actor)?;let state=self.object_for(actor)?.actor.as_ref().unwrap().components.state;
        Ok(crate::load_sai_lists::water(flags,state.low,state.high))
    }
    fn water(&self)->Result<bool> {
        let c=&self.object()?.actor.as_ref().unwrap().components;
        if let Some(value)=c.get(ComponentSlot::Organization) {
            let crate::actor_world::RuntimeComponent::Organization(value)=value else {return Err(error("SAI Organization payload differs"))};
            if value.header.owner!=self.actor {return Err(error("SAI Organization owner differs"))}
            //4D4E70 appends nonnull slots from the native member array backwards.
            for &member in value.members.iter().rev() {
                if !member.is_null()&&self.water_single(member)? {return Ok(true)}
            }
        }
        self.water_single(self.actor)
    }
    fn require_actor(&self,raw:u32)->Result<()> {
        if raw!=self.actor.raw() {return Err(error("SAI notification changed the held actor identity"));}
        Ok(())
    }
}
impl<F:Fields> State for World<'_,'_,F> {
    fn owner_word(&self)->Result<u32> {
        self.object()?.actor.as_ref().unwrap().words.get(&0xe8).copied()
            .ok_or_else(||error("SAI actor virtual108 actual E8 owner field is absent"))
    }
    fn category(&self)->Result<u32> {
        self.object()?.actor.as_ref().unwrap().words.get(&0x140).copied()
            .ok_or_else(||error("SAI actor actual140 category field is absent"))
    }
    fn class_three(&self)->Result<bool> {
        let flags8=self.flags()?;
        let c=&self.object()?.actor.as_ref().unwrap().components;
        Ok(classification_three(ActorClassFields {flags8,component90:c.present(ComponentSlot::Wall),
            components:[ComponentSlot::Organization,ComponentSlot::Element,ComponentSlot::Economy,
                ComponentSlot::Denizen,ComponentSlot::Structure,ComponentSlot::Settlement,
                ComponentSlot::Recruit,ComponentSlot::Builder,ComponentSlot::MobileDeployment]
                .map(|s|c.present(s))}))
    }
    fn flare(&self)->Result<bool> {Ok(self.object()?.actor.as_ref().unwrap().components.present(ComponentSlot::Flare))}
    fn require_non_marker(&self)->Result<()> {
        //The native AND cannot be true for a completed category other than32,
        //regardless of the source model pointer+88. No null model is invented.
        if self.object()?.definition.proof.category.registry_index==32 {
            return Err(error("SAI MARKER branch requires actual ObjectData88 and source virtual24 publication"));
        }
        Ok(())
    }
    fn set_category(&mut self,value:u32)->Result<()> {
        let _=self.object()?;
        let key=self.actors.key(self.actor).unwrap();
        *self.actors.object_mut(key).map_err(error)?.actor.as_mut().unwrap().words.get_mut(&0x140)
            .ok_or_else(||error("SAI actor140 allocation is absent"))?=value;
        Ok(())
    }
    fn ordinary(&mut self,category:u32,owner:u32)->Result<()> {
        let (slots,arena)=self.players.as_ref().ok_or_else(||error("7A0390 actual79F500 manager/player owner is unbound"))?.parts();
        let mut listeners=0;let mut current=0;
        for (slot,handle) in slots.iter().enumerate() {
            let Some(handle)=handle else {continue};let p=arena.player(*handle).map_err(error)?;
            if p.slot() as usize!=slot {return Err(error("SAI actual slot/allocation differs"))}
            current+=1;if p.kingdom().raw()==owner {listeners+=1;}
        }
        if category!=0||owner==0||listeners!=0 {return ordinary_effects(category,owner,listeners,current)}
        let key=self.actors.key(self.actor).ok_or_else(||error("SAI actor generation absent"))?;
        let position=self.actors.placement(key,self.spatial).map_err(error)?.position_bits;
        let region=match self.players.as_ref().unwrap() {
            Controllers::World {map,..}=>map.topology().region_at([position[0],position[1]]).map_err(error)?.ok_or_else(||error("SAI actual category0 position has no native region"))?,
            Controllers::Read {..}=>return Err(error("7A98E0 actual current map/list owner is unbound")),
        };
        let water=self.water()?;
        let pointer=self.object()?.actor.as_ref().unwrap().words.get(&0x154).copied().ok_or_else(||error("SAI actual Actor154 constructor field is unbound"))?;
        if pointer!=0 {return Err(error("7BC330/470 positive Actor154 needs its actual SAIActor owner before time14 write"))}
        let pc=self.bank.combat_source().map_err(error)?.source_precision();
        let kingdom=ActorId::from_raw(owner);
        let derived=crate::load_sai_players::canonical_kingdom(self.catalog,self.bank,self.actors,kingdom,pc).map_err(error)?;
        let index=derived.words.get(&0x188).copied().ok_or_else(||error("SAI actual Kingdom all-index is unbound"))?as usize;
        let Controllers::World {arena,map,manager90,kingdoms,..}=self.players.as_mut().unwrap() else {unreachable!()};
        if kingdoms.all().get(index)!=Some(&Some(kingdom)) {return Err(error("SAI current all-Kingdom slot differs"))}
        map.prepend_actor(region,water,index,self.actor.raw()).map_err(error)?;
        if !water&&arena.has_player_allocations() {return Err(error("7BB120 actual per-region initialized AI callbacks are required after land-list prepend"))}
        if current!=0 {return Err(error("7A83A0 actual controller seen-actor notification is required after region-list publication"))}
        arena.prepend_manager_actor(manager90,index,self.actor.raw()).map_err(error)
    }
}
impl<F:Fields> actor_notify::ActorNotifyWorld for World<'_,'_,F> {
    fn sai_ready(&mut self)->Result<bool> {Ok(self.ready.ready())}
    fn actor_c8(&mut self,actor:u32)->Result<bool> {self.require_actor(actor)?;self.flare()}
    fn actor_category(&mut self,actor:u32)->Result<u32> {self.require_actor(actor)?;self.category()}
    fn actor_removed(&mut self,actor:u32)->Result<bool> {self.require_actor(actor)?;Ok(self.flags()?&1!=0)}
    fn predicate_79e300(&mut self,actor:u32)->Result<bool> {self.require_actor(actor)?;self.class_three()}
    fn actor_4ceff0(&mut self,actor:u32)->Result<bool> {self.require_actor(actor)?;Ok(self.mind_word28(self.actor)?.is_some_and(|v|v!=0))}
    fn actor_virtual_20(&mut self,actor:u32)->Result<bool> {self.require_actor(actor)?;self.active(self.actor,&mut Default::default())}
    fn predicate_79e080(&mut self,actor:u32)->Result<bool> {
        self.require_actor(actor)?;
        // All three getters execute, even when the first is nonnull.
        let c=&self.object()?.actor.as_ref().unwrap().components;
        let structure=crate::load_detection_memory::actor_container(self.actor,c).map_err(error)?;
        let element=self.element_owner(self.actor)?;
        let mind=self.mind_word28(self.actor)?.is_some_and(|raw|self.actors.key(ActorId::from_raw(raw)).is_some());
        Ok(structure.is_some()||element.is_some()||mind)
    }
    fn unregister(&mut self,actor:u32)->Result<()> {self.require_actor(actor)?;unregister(self)}
    fn register(&mut self,actor:u32,category:u32)->Result<()> {self.require_actor(actor)?;register(self,category)}
    fn register_unit(&mut self,actor:u32,flag:bool)->Result<()> {
        self.require_actor(actor)?;let category=u32::from(flag);
        if self.ready.ready()&&self.category()?!=category {
            if self.category()?!=5 {unregister(self)?;}
            if self.flags()?&1==0 {register(self,category)?;}
        }
        Ok(())
    }
}

/// Requires this World's actual ActorStore/spatial pair and the same manager
/// field lease. An inactive manager returns before inspecting any actor/source.
pub fn notify<F:Fields>(catalog:&Catalog<'_>,bank:&Bank,actors:&mut ActorStore,
    spatial:&SpatialWorld,actor:ActorId,ready:&mut F)->Result<()> {
    actor_notify::notify(&mut World {catalog,bank,actors,spatial,ready,actor,players:None},actor.raw())
}
/// Same notification with the actual manager+60 slots and allocation arena.
pub fn notify_with_players<F:Fields>(catalog:&Catalog<'_>,bank:&Bank,actors:&mut ActorStore,
    spatial:&SpatialWorld,actor:ActorId,ready:&mut F,slots:&[Option<u32>],arena:&crate::load_sai_players::Arena)->Result<()> {
    actor_notify::notify(&mut World {catalog,bank,actors,spatial,ready,actor,players:Some(Controllers::Read {slots,arena})},actor.raw())
}
pub(crate) fn notify_with_world<F:Fields>(catalog:&Catalog<'_>,bank:&Bank,actors:&mut ActorStore,
    spatial:&SpatialWorld,actor:ActorId,ready:&mut F,slots:&[Option<u32>],arena:&mut crate::load_sai_players::Arena,
    map:&mut crate::load_sai_world::Map,manager90:&mut Vec<u32>,kingdoms:&crate::load_kingdom_init_world::KingdomRegistryState)->Result<()> {
    actor_notify::notify(&mut World {catalog,bank,actors,spatial,ready,actor,players:Some(Controllers::World {slots,arena,map,manager90,kingdoms})},actor.raw())
}
fn ordinary_effects(category:u32,owner:u32,listeners:usize,current_players:usize)->Result<()> {
    if category==2 {
        return if listeners==0 {Ok(())}else {Err(error("7AFE90 actual matching controller definition notification is required"))};
    }
    if owner==0 {return Ok(())}
    if listeners!=0 {return Err(error("7B3F70 actual matching controller actor ownership is required"))}
    if category==0 {return Err(error("7A98E0 actual per-Kingdom map lists and manager90 actor list are required"))}
    if current_players!=0 {return Err(error("7A83A0 actual controller seen-actor notification is required"))}
    Ok(())
}

#[cfg(test)]
#[path="load_sai_actor_tests.rs"]
mod tests;
