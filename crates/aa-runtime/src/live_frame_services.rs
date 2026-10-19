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
    ///Census sink and the tick being updated; None for every native update.
    pub census:Option<(&'a RefCell<Vec<super::CensusEntry>>,u32)>,
}
fn fail(e:impl std::fmt::Display)->aa_world::Error{aa_world::Error::Callback(format!("actual frame callback: {e}"))}
fn queue_fail(e:impl std::fmt::Display)->aa_world::queues::Error{aa_world::queues::Error::Callback(format!("actual frame scheduled callback: {e}"))}
impl<S:SessionRead+?Sized,O:AudioOutput+?Sized> Services<'_,'_,'_,S,O>{
    ///Census only: record the failed leaf and report whether it was swallowed.
    fn census_skip(&self,site:impl FnOnce()->String,error:&dyn std::fmt::Display)->bool{
        let Some((sink,tick))=self.census else {return false};
        match sink.try_borrow_mut() {
            Ok(mut entries)=>{entries.push(super::CensusEntry{tick,site:site(),error:error.to_string()});true}
            Err(_)=>false,
        }
    }
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
            trigger_program:self.program,first_tick:&mut first_tick,
            sun:Some(crate::live_updates::trigger_program::SunServices{actors:self.catalog.actors,sun:&mut world.sun,
                source_precision:self.retained.definitions.combat_source()?.source_precision()})};
        Ok(call(&mut legacy))
    }
}
impl<S:SessionRead+?Sized,O:AudioOutput+?Sized> clocked::Services for Services<'_,'_,'_,S,O>{
    fn fog_cheat_enabled(&mut self,w:&mut ClockedWorld)->aa_world::Result<bool>{self.legacy(|s|s.fog_cheat_enabled(w)).map_err(fail)?}
    fn increment_auxiliary_counter(&mut self,w:&mut ClockedWorld)->aa_world::Result<Option<u32>>{self.legacy(|s|s.increment_auxiliary_counter(w)).map_err(fail)?}
    fn pending_component_target(&mut self,w:&mut ClockedWorld,o:ObjectId)->aa_world::Result<ObjectId>{self.legacy(|s|s.pending_component_target(w,o)).map_err(fail)?}
    fn or_object_flags(&mut self,w:&mut ClockedWorld,o:ObjectId,b:u32)->aa_world::Result<()>{self.legacy(|s|s.or_object_flags(w,o,b)).map_err(fail)?}
    fn remove_classification(&mut self,w:&mut ClockedWorld,k:ListKind,o:ObjectId)->aa_world::Result<()>{self.legacy(|s|s.remove_classification(w,k,o)).map_err(fail)?}
    fn effect(&mut self,e:Effect,w:&mut ClockedWorld,q:&mut Queues,r:&mut NativeRng)->aa_world::Result<()>{
        let result=self.legacy(|s|s.effect(e,w,q,r)).map_err(fail).and_then(|v|v);
        match result {
            Err(error) if self.census_skip(||format!("world effect {e:?}"),&error)=>Ok(()),
            other=>other,
        }
    }
    fn predicate(&mut self,p:Predicate,o:ObjectId,w:&mut ClockedWorld,q:&mut Queues,r:&mut NativeRng)->aa_world::Result<bool>{self.legacy(|s|s.predicate(p,o,w,q,r)).map_err(fail)?}
    fn resolve_handle(&mut self,o:ObjectId,w:&mut ClockedWorld,q:&mut Queues,r:&mut NativeRng)->aa_world::Result<Option<ObjectId>>{self.legacy(|s|s.resolve_handle(o,w,q,r)).map_err(fail)?}
    fn update_component(&mut self,event:Delivery,world:&mut ClockedWorld,queues:&mut Queues,rng:&mut NativeRng)->aa_world::queues::Result<()>{
        let result=self.update_component_native(event,world,queues,rng);
        match result {
            Err(error) if self.census_skip(||{
                let target=self.retained.owners.target(event.owner).map(|t|format!("{t:?}")).unwrap_or_else(|e|e.to_string());
                format!("queue{} {} {target}",event.queue,if event.elapsed_bits.is_some(){"periodic+14"}else if event.queue==3{"deferred+18"}else{"deferred+1C"})
            },&error)=>Ok(()),
            other=>other,
        }
    }
}
impl<S:SessionRead+?Sized,O:AudioOutput+?Sized> Services<'_,'_,'_,S,O>{
    fn update_component_native(&mut self,event:Delivery,world:&mut ClockedWorld,queues:&mut Queues,rng:&mut NativeRng)->aa_world::queues::Result<()>{
        crate::live_scheduler_dispatch::validate_delivery(event)?;
        let target=self.retained.owners.target(event.owner).map_err(queue_fail)?;
        if matches!(target,crate::scheduler_owners::Target::Component{slot,..}if slot==ComponentSlot::Mind as u8){
            let actual=&mut self.retained.world;
            let spatial=match &mut actual.spatial{
                SpatialOwner::Adopted(v)if v.error.is_none()&&v.pending.is_empty()=>&mut v.world,
                _=>return Err(queue_fail("actual Mind spatial owner is absent during update")),
            };
            let mut context=crate::stored_mind_update::Context{
                actors:&mut self.retained.actors,bank:&mut self.retained.definitions,spatial,
                variables:&mut actual.variables,presentation:&mut actual.presentation,zones:&actual.zones,
                runtime:&mut actual.mind_runtime,queues,owners:&mut self.retained.owners,random:rng,
                clock:&world.clock,inventory:&self.retained.evidence.inventory,terrain:&actual.terrain,
                mission:&actual.mission_fields,phase:&world.phase,precision:world.clock_precision,
                current_settings:Some((actual.global_settings_mode,actual.map_mode.as_ref())),
            };
            if let crate::scheduler_owners::Target::Component{object,..}=target
                && crate::live_updates::element_mind::is_element(&context,object){
                return crate::live_updates::element_mind::run(self.catalog,&mut context,event);
            }
            return crate::stored_mind_update::run(self.catalog,&mut context,event);
        }
        if let crate::scheduler_owners::Target::Component{object,slot}=target
            && slot==ComponentSlot::Properties as u8 && event.elapsed_bits.is_some(){
            return crate::live_updates::properties::run(object,event,&mut crate::live_updates::properties::Context{
                actors:&mut self.retained.actors,
            });
        }
        if let crate::scheduler_owners::Target::Component{object,slot}=target
            && slot==ComponentSlot::Supply as u8 && event.elapsed_bits.is_some(){
            let source_precision=self.retained.definitions.combat_source().map_err(queue_fail)?.source_precision();
            return crate::live_updates::supply::run(object,event,&mut crate::live_updates::supply::Context{
                catalog:self.catalog,bank:&self.retained.definitions,actors:&mut self.retained.actors,
                world:&mut self.retained.world,clock:&world.clock,phase:world.phase,
                source_precision,precision:world.clock_precision,
            });
        }
        if let crate::scheduler_owners::Target::Component{object,slot}=target
            && slot==ComponentSlot::Construction as u8 && event.elapsed_bits.is_some(){
            let actual=&mut self.retained.world;
            let spatial=match &actual.spatial{
                SpatialOwner::Adopted(v)if v.error.is_none()&&v.pending.is_empty()=>&v.world,
                _=>return Err(queue_fail("actual Construction spatial owner is absent during update")),
            };
            return crate::live_updates::construction::run(object,event,&mut crate::live_updates::construction::Context{
                actors:&mut self.retained.actors,spatial,variables:&actual.variables,
                clock:&world.clock,precision:world.clock_precision,
            });
        }
        if let crate::scheduler_owners::Target::Component{object,slot}=target
            && slot==ComponentSlot::City as u8 && event.elapsed_bits.is_some(){
            let actual=&mut self.retained.world;
            let spatial=match &actual.spatial{
                SpatialOwner::Adopted(v)if v.error.is_none()&&v.pending.is_empty()=>&v.world,
                _=>return Err(queue_fail("actual City spatial owner is absent during update")),
            };
            return crate::live_updates::city::run(object,event,&mut crate::live_updates::city::Context{
                actors:&mut self.retained.actors,spatial,variables:&actual.variables,queues,
                clock:&world.clock,precision:world.clock_precision,
            });
        }
        if let crate::scheduler_owners::Target::Component{object,slot}=target
            && slot==ComponentSlot::Detection as u8 && event.queue!=4{
            return crate::live_updates::detection::run(object,event,&mut crate::live_updates::detection::Context{
                catalog:self.catalog,bank:&self.retained.definitions,actors:&mut self.retained.actors,
                world:&mut self.retained.world,clock:&world.clock,phase:world.phase,precision:world.clock_precision,
            });
        }
        if let crate::scheduler_owners::Target::Component{object,slot}=target
            && slot==ComponentSlot::Element as u8 && event.elapsed_bits.is_some(){
            let actual=&mut self.retained.world;
            let spatial=match &actual.spatial{
                SpatialOwner::Adopted(v)if v.error.is_none()&&v.pending.is_empty()=>&v.world,
                _=>return Err(queue_fail("actual Element spatial owner is absent during update")),
            };
            return crate::live_updates::element::run(object,event,&mut crate::live_updates::element::Context{
                actors:&mut self.retained.actors,bank:&self.retained.definitions,
                economy:&mut self.retained.evidence.kingdom_economy,spatial,variables:&actual.variables,
                clock:&world.clock,precision:world.clock_precision,
            });
        }
        if let crate::scheduler_owners::Target::Component{object,slot}=target
            && slot==ComponentSlot::Economy as u8 && event.elapsed_bits.is_some(){
            //Economy vtable A3C668 periodic +14 = 56F2B0.
            let actual=&self.retained.world;
            let spatial=match &actual.spatial{
                SpatialOwner::Adopted(v)if v.error.is_none()&&v.pending.is_empty()=>&v.world,
                _=>return Err(queue_fail("actual Economy spatial owner is absent during update")),
            };
            let source_precision=self.retained.definitions.combat_source().map_err(queue_fail)?.source_precision();
            return crate::live_updates::economy::run(object,event,&crate::live_updates::economy::Context{
                catalog:self.catalog,bank:&self.retained.definitions,actors:&self.retained.actors,spatial,
                zones:&actual.zones,source_precision,
            },queues,&mut self.retained.log);
        }
        if let crate::scheduler_owners::Target::DynamicDoodad{object,slot}=target{
            //Disturbable A40DDC +18 = 5902C0; Breakable A40D94 +18 = 58FA30.
            let source_precision=self.retained.definitions.combat_source().map_err(queue_fail)?.source_precision();
            return crate::live_updates::doodad::run(object,slot,event,&mut crate::live_updates::doodad::Context{
                catalog:self.catalog,bank:&self.retained.definitions,actors:&self.retained.actors,
                world:&mut self.retained.world,source_precision,precision:world.clock_precision,
            },queues);
        }
        if matches!(target,crate::scheduler_owners::Target::Sun) && event.queue==3{
            //Sun vtable A2A578 deferred +18 = 4EDDC0 (data unused).
            return crate::live_updates::sun::update(&mut self.retained.world.sun,queues,event.owner,
                world.clock.begin.0,world.clock_precision).map_err(queue_fail);
        }
        if let crate::scheduler_owners::Target::Kingdom(key)=target{
            let actual=&mut self.retained.world;
            let source_precision=self.retained.definitions.combat_source().map_err(queue_fail)?.source_precision();
            return crate::live_updates::kingdom::run(key,event,&mut crate::live_updates::kingdom::Context{
                catalog:self.catalog,actors:&mut self.retained.actors,economy:&mut self.retained.evidence.kingdom_economy,
                variables:&actual.variables,registration:&actual.registration,majors:actual.kingdoms.majors(),
                mission:&actual.mission_fields,current_settings:Some((actual.global_settings_mode,actual.map_mode.as_ref())),
                source_precision,clock:&world.clock,phase:world.phase,precision:world.clock_precision,
            });
        }
        self.legacy(|s|s.update_component(event,world,queues,rng)).map_err(queue_fail)?
    }
}
