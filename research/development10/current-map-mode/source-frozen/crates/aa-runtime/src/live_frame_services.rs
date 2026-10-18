//!Actual frame loans, releasing legacy Bank/FirstTick/fog borrows before
//!scheduled Mind callbacks borrow those SAME owners mutably. Legacy kernels
//!remain the one implementation for every other clocked callback.
use super::*;
use crate::clocked::{Effect,Predicate,Services as _};
use aa_actors::components::ComponentSlot;
use aa_world::{World as ClockedWorld,ObjectId,ListKind,queues::Delivery};
pub(super) struct Services<'a,'source,'catalog_source,S:SessionRead+?Sized,O:AudioOutput+?Sized>{
    pub catalog:&'a Catalog<'catalog_source>,
    pub retained:&'a mut Retained,
    pub source:&'a live_first_tick::Source<'source>,
    pub program:&'a mut CoreProgram,
    pub music:&'a mut MusicRequests,
    pub session:&'a S,
    pub output:&'a mut O,
    pub sync_a:&'a RefCell<Synchronizer>,
    pub gate:&'a mut dyn FnMut()->Result<bool,String>,
}
fn fail(e:impl std::fmt::Display)->aa_world::Error{aa_world::Error::Callback(format!("actual frame callback: {e}"))}
fn queue_fail(e:impl std::fmt::Display)->aa_world::queues::Error{aa_world::queues::Error::Callback(format!("actual frame scheduled callback: {e}"))}
impl<S:SessionRead+?Sized,O:AudioOutput+?Sized> Services<'_,'_,'_,S,O>{
    ///No reference to Bank, variables or Owners from this construction escapes
    ///the single callback. In particular first-tick Bank is not held by step.
    fn legacy<T>(&mut self,call:impl FnOnce(&mut live_clocked::Services<'_>)->T)->Result<T,String>{
        let world=&mut self.retained.world;
        let spatial=match &mut world.spatial{
            SpatialOwner::Adopted(v)if v.error.is_none()&&v.pending.is_empty()=>&mut v.world,
            _=>return Err("actual live spatial owner is absent during callback".into()),
        };
        let fields=world.runtime_fields.frame_fields(world.variables.cheat_fog_word())?;
        let mut first_tick=live_first_tick::Context{session:self.session,source:self.source,
            bank:&self.retained.definitions,settings:&self.retained.settings,requests:self.music,output:self.output};
        let mut legacy=live_clocked::Services{actors:&mut self.retained.actors,owners:&self.retained.owners,spatial,
            registration:&mut world.registration,fields,
            sync:live_clocked::SyncBinding::Shared{owner:self.sync_a,gate:&mut *self.gate},
            triggers:world.trigger_manager.as_mut().ok_or("retained trigger manager is absent")?,
            trigger_program:self.program,first_tick:&mut first_tick};
        Ok(call(&mut legacy))
    }
}
impl<S:SessionRead+?Sized,O:AudioOutput+?Sized> clocked::Services for Services<'_,'_,'_,S,O>{
    fn fog_cheat_enabled(&mut self,w:&mut ClockedWorld)->aa_world::Result<bool>{self.legacy(|s|s.fog_cheat_enabled(w)).map_err(fail)?}
    fn increment_auxiliary_counter(&mut self,w:&mut ClockedWorld)->aa_world::Result<Option<u32>>{self.legacy(|s|s.increment_auxiliary_counter(w)).map_err(fail)?}
    fn pending_component_target(&mut self,w:&mut ClockedWorld,o:ObjectId)->aa_world::Result<ObjectId>{self.legacy(|s|s.pending_component_target(w,o)).map_err(fail)?}
    fn or_object_flags(&mut self,w:&mut ClockedWorld,o:ObjectId,b:u32)->aa_world::Result<()>{self.legacy(|s|s.or_object_flags(w,o,b)).map_err(fail)?}
    fn remove_classification(&mut self,w:&mut ClockedWorld,k:ListKind,o:ObjectId)->aa_world::Result<()>{self.legacy(|s|s.remove_classification(w,k,o)).map_err(fail)?}
    fn effect(&mut self,e:Effect,w:&mut ClockedWorld,q:&mut Queues,r:&mut NativeRng)->aa_world::Result<()>{self.legacy(|s|s.effect(e,w,q,r)).map_err(fail)?}
    fn predicate(&mut self,p:Predicate,o:ObjectId,w:&mut ClockedWorld,q:&mut Queues,r:&mut NativeRng)->aa_world::Result<bool>{self.legacy(|s|s.predicate(p,o,w,q,r)).map_err(fail)?}
    fn resolve_handle(&mut self,o:ObjectId,w:&mut ClockedWorld,q:&mut Queues,r:&mut NativeRng)->aa_world::Result<Option<ObjectId>>{self.legacy(|s|s.resolve_handle(o,w,q,r)).map_err(fail)?}
    fn update_component(&mut self,event:Delivery,world:&mut ClockedWorld,queues:&mut Queues,rng:&mut NativeRng)->aa_world::queues::Result<()>{
        crate::live_scheduler_dispatch::validate_delivery(event)?;
        let target=self.retained.owners.target(event.owner).map_err(queue_fail)?;
        if matches!(target,crate::scheduler_owners::Target::Component{slot,..}if slot==ComponentSlot::Mind as u8){
            let actual=&mut self.retained.world;
            let spatial=match &mut actual.spatial{
                SpatialOwner::Adopted(v)if v.error.is_none()&&v.pending.is_empty()=>&mut v.world,
                _=>return Err(queue_fail("actual Mind spatial owner is absent during update")),
            };
            return crate::stored_mind_update::run(self.catalog,&mut crate::stored_mind_update::Context{
                actors:&mut self.retained.actors,bank:&mut self.retained.definitions,spatial,
                variables:&mut actual.variables,presentation:&mut actual.presentation,zones:&actual.zones,
                runtime:&mut actual.mind_runtime,queues,owners:&mut self.retained.owners,random:rng,
                clock:&world.clock,inventory:&self.retained.evidence.inventory,terrain:&actual.terrain,
                mission:&actual.mission_fields,phase:&world.phase,precision:world.clock_precision,
                current_settings:Some((actual.global_settings_mode,actual.map_mode.as_ref())),
            },event);
        }
        self.legacy(|s|s.update_component(event,world,queues,rng)).map_err(queue_fail)?
    }
}
