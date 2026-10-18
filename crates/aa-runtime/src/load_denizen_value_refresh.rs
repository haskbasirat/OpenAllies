//! Native569E40 over the actual held Denizen inventory and current occupants.
//! Both sums are stored in their real component immediately. Definition fallback
//! reads completed +1E4 from the same source Bank; it never treats an unavailable
//! value as zero. Stale/null GTIDs use the native definition fallback.
use crate::{catalog::Catalog, live_object_init::Record};
use aa_actors::{ActorId, actor_creation::{Error, Result}, components::ComponentSlot,
    denizen_state::DenizenState, loading::ActorShell};
use aa_mapworld::restore::{RestoredComponent, RestoredRecord};
use aa_numeric::{Binary, Precision};

#[cfg(test)]
#[path="load_denizen_value_refresh_tests.rs"]
mod tests;
#[cfg(test)]
#[path="load_actor_value_refresh_actual_tests.rs"]
mod actual_tests;

fn error(value:impl std::fmt::Display)->Error {Error::World(value.to_string())}
fn number(bits:u32)->Result<Binary> {Binary::from_f32_bits(bits).map_err(error)}
fn store(value:Binary)->Result<u32> {value.store_f32().map_err(error)}

/// Resolution distinguishes an existing actor with no Performer (skip) from
/// a stale/null ID (fallback). This is not an Option-flattening shortcut.
pub(crate) enum Occupant { Absent, Present { value:Option<u32>, health:Option<[u32;2]> } }
pub(crate) trait World {
    fn count(&self)->usize;
    fn occupant(&self,index:usize)->Result<Occupant>;
    fn target_flags(&self,index:usize)->Result<Option<u32>>;
    fn target_health(&self,index:usize)->Result<u32>;
    fn slot_value(&self,index:usize)->Result<u32>;
    fn target_value(&self,index:usize)->Result<u32>;
    fn current(&self)->Result<u32>;
    fn maximum(&self)->Result<u32>;
    fn set_current(&mut self,value:u32);
    fn set_maximum(&mut self,value:u32);
}
pub(crate) fn recompute(world:&mut impl World,pc:Precision)->Result<()> {
    //569E44/4B precede the head read and every source/occupant dereference.
    world.set_current(0);world.set_maximum(0);
    for index in 0..world.count() {
        match world.occupant(index)? {
            Occupant::Present{value:None,..}=>continue,
            Occupant::Present{value:Some(value),health}=> {
                let ratio=match health {Some([health,maximum])=>number(health)?.div(&number(maximum)?,pc).map_err(error)?,None=>Binary::from_u32(1)};
                let weighted=ratio.mul(&number(value)?,pc).map_err(error)?;
                world.set_current(store(weighted.add(&number(world.current()?)?,pc).map_err(error)?)?);
                //Native re-resolves the occupant and reads Performer+60 after
                //the current store. The closed immutable adapter preserves ID.
                let Occupant::Present{value:Some(value),..}=world.occupant(index)? else {
                    return Err(Error::NullDereference("569F0B reread occupant Performer"));
                };
                world.set_maximum(store(number(value)?.add(&number(world.maximum()?)?,pc).map_err(error)?)?);
            }
            Occupant::Absent=> {
                let Some(flags)=world.target_flags(index)? else{continue};
                if flags&0x20==0 {continue}
                let ratio=if flags&1!=0 {number(world.slot_value(index)?)?.div(&number(world.target_health(index)?)?,pc).map_err(error)?}else{Binary::from_u32(1)};
                let weighted=ratio.mul(&number(world.target_value(index)?)?,pc).map_err(error)?;
                world.set_current(store(weighted.add(&number(world.current()?)?,pc).map_err(error)?)?);
                world.set_maximum(store(number(world.target_value(index)?)?.add(&number(world.maximum()?)?,pc).map_err(error)?)?);
            }
        }
    }
    Ok(())
}

struct Bound<'a,'r,'source> {
    catalog:&'a Catalog<'source>,record:&'a Record<'r>,denizen:&'a mut DenizenState,source_pc:Precision,
}
impl Bound<'_, '_, '_> {
    fn actor(&self,id:ActorId)->Result<Option<&ActorShell<RestoredComponent>>> {
        if id.is_null(){return Ok(None)}
        if id==self.record.owner{return Ok(Some(self.record.shell.actor.as_ref().ok_or(Error::NullDereference("569EB4 held Actor"))?))}
        let Some(entry)=self.record.registry.get(id)else{return Ok(None)};
        let shell=match &entry.value {
            RestoredRecord::Actor(v)=>&v.shell,
            RestoredRecord::Kingdom(v)=>&v.runtime.as_ref().ok_or_else(||error("569EB4 current Kingdom runtime unavailable"))?.base,
            RestoredRecord::Loading=>return Err(error("569EB4 another held occupant requires its actual shell")),
            _=>return Err(error("569EB4 resolved occupant has no Actor storage")),
        };
        shell.actor.as_ref().map(Some).ok_or(Error::NullDereference("569EB4 resolved Actor"))
    }
}
impl World for Bound<'_, '_, '_> {
    fn count(&self)->usize {self.denizen.inventory.len()}
    fn occupant(&self,index:usize)->Result<Occupant> {
        let id=ActorId::from_raw(self.denizen.inventory[index].word_00);
        let Some(actor)=self.actor(id)?else{return Ok(Occupant::Absent)};
        let performer=match actor.components.get(ComponentSlot::Performer) {
            None=>return Ok(Occupant::Present{value:None,health:None}),
            Some(RestoredComponent::Performer(p)) if p.state.owner==id.raw()=>p,
            _=>return Err(error("569EB4 actual occupant Performer payload/owner differs")),
        };
        let health=match actor.components.get(ComponentSlot::Body) {
            None=>None,
            Some(RestoredComponent::Body(b)) if b.header.owner==id=>Some([b.body.health.0,b.body.maximum.0]),
            _=>return Err(error("57DC40 actual occupant Body payload/owner differs")),
        };
        Ok(Occupant::Present{value:Some(performer.word60),health})
    }
    fn target_flags(&self,index:usize)->Result<Option<u32>> {
        let target=&self.denizen.inventory[index].target;
        //An empty portable name is not a certified null native definition.
        self.catalog.canonical_component_mask(target,self.source_pc).map(Some).map_err(error)
    }
    fn target_health(&self,index:usize)->Result<u32> {
        let target=&self.denizen.inventory[index].target;
        self.catalog.require_untranslated_actor_identity(target).map_err(error)?;
        self.catalog.actors.native_ground_definition(target,self.source_pc).map_err(error)?.attributes.body
            .map(|b|b.health.bits).ok_or_else(||error("45D730 actual target ActorData+284 unavailable"))
    }
    fn slot_value(&self,index:usize)->Result<u32> {Ok(self.denizen.inventory[index].value_08.0)}
    fn target_value(&self,index:usize)->Result<u32> {
        self.catalog.completed_actor_combat_values(&self.denizen.inventory[index].target).map(|v|v.value_1e4.0).map_err(error)
    }
    fn current(&self)->Result<u32> {self.denizen.words.get(&0x10).copied().ok_or_else(||error("569E40 initialized Denizen+10 absent"))}
    fn maximum(&self)->Result<u32> {self.denizen.words.get(&0x14).copied().ok_or_else(||error("569E40 initialized Denizen+14 absent"))}
    fn set_current(&mut self,value:u32) {self.denizen.words.insert(0x10,value);}
    fn set_maximum(&mut self,value:u32) {self.denizen.words.insert(0x14,value);}
}

/// Recompute the actual held component. The same Denizen Box is restored even
/// on Result failure, retaining all already completed native scalar stores.
pub fn refresh(catalog:&Catalog<'_>,record:&mut Record<'_>,pc:Precision)->Result<()> {
    let source_pc=crate::load_actor_init_world::validate(catalog,record)?;
    if record.definition.component_mask.is_none_or(|m|m&(1<<10)==0){return Err(error("569E40 Denizen outside actual source factory"))}
    match record.shell.actor.as_ref().and_then(|a|a.components.get(ComponentSlot::Denizen)) {
        Some(RestoredComponent::Denizen(v)) if v.header.owner==record.owner=>(),
        _=>return Err(error("569E40 actual held Denizen payload/owner differs")),
    }
    let component=record.shell.actor.as_mut().unwrap().components.remove(ComponentSlot::Denizen).unwrap();
    let RestoredComponent::Denizen(mut denizen)=component else{unreachable!()};
    let outcome=recompute(&mut Bound{catalog,record,denizen:&mut denizen,source_pc},pc);
    assert!(record.shell.actor.as_mut().unwrap().components.insert(ComponentSlot::Denizen,RestoredComponent::Denizen(denizen)).is_none());
    outcome
}
