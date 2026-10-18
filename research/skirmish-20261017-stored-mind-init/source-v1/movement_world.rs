//! Live Element Move payload ownership and ground-motion world bindings.
//! Mind Arena owns stack/status; component storage owns Mover; SpatialWorld
//! alone owns published placement. No copied state persists between callbacks.
use aa_mind::{Error, Result, StateId, construct::{Installed,Kind,Reference},element_move::Payload};
use aa_navigation::{MoveState,OwnedPath,result_plan};
use std::{collections::BTreeMap,sync::{Arc,RwLock,RwLockReadGuard,RwLockWriteGuard,TryLockError}};
use crate::{actor_world::{ActorAccess,ActiveComponent,RuntimeComponent,ObjectKey},spatial_world::SpatialWorld,scheduler_owners::{Owners,Target}};
use aa_actors::{ActorId,components::ComponentSlot};
use aa_world::queues::Queues;
use aa_simulation::NativeRng;
use aa_spatial::{index::{SpatialIndex,TransformCallbacks},index_world::GroundSurface,query::{QueryGeometry,Hit}};
use aa_movement::{ActorMotion,MoverState,GroundWorld,Event};

#[cfg(test)]
#[path="movement_payload_tests.rs"]
mod payload_tests;

fn fail(value:impl std::fmt::Display)->Error {Error::World(format!("live movement: {value}"))}

/// Stable allocation shared by runtime adapters. Cloning this handle does not
/// clone path/state. A callback already holding the current payload uses its
/// explicit &mut Payload argument; an unsupported independent reentry errors.
#[derive(Clone)]
pub struct PayloadHandle(Arc<RwLock<Payload>>);
impl PayloadHandle {
    pub fn read(&self)->Result<RwLockReadGuard<'_,Payload>> {
        match self.0.try_read() {
            Ok(value)=>Ok(value),
            Err(TryLockError::Poisoned(value))=>Ok(value.into_inner()),
            Err(TryLockError::WouldBlock)=>Err(fail("active Move payload must use its callback borrow")),
        }
    }
    pub fn write(&self)->Result<RwLockWriteGuard<'_,Payload>> {
        match self.0.try_write() {
            Ok(value)=>Ok(value),
            Err(TryLockError::Poisoned(value))=>Ok(value.into_inner()),
            Err(TryLockError::WouldBlock)=>Err(fail("active Move payload must use its callback borrow")),
        }
    }
}
#[derive(Default)]
pub struct Runtime {
    states:BTreeMap<StateId,PayloadHandle>,
    pub status_diagnostic_emitted:bool,
}
impl Runtime {
    pub fn handle(&self,state:StateId)->Result<PayloadHandle> {
        self.states.get(&state).cloned().ok_or_else(||fail("typed Move payload is not installed"))
    }
    /// Transfer initialized Element+7C scalar/path fields from the installed
    /// constructor exactly once. Saved nonempty paths need their actual decoder
    /// and cannot be replaced with a fresh empty path through this entry.
    pub fn bind_element(&mut self,mind:&mut Installed)->Result<StateId> {
        self.bind_element_with(mind,Vec::new())
    }
    /// bind_element for a restored Element whose saved +D4 path held nodes
    /// (aa_mind::saved_load::Applied::element_path, head first): the
    /// collection words (+E0/+E4/+E8/+EC) come from the loaded Mind and the
    /// node payloads from the save. Their kind counts must agree.
    pub fn bind_element_restored(&mut self,mind:&mut Installed,nodes:Vec<aa_navigation::PathNode>)->Result<StateId> {
        let count=|kind:u32|nodes.iter().filter(|n|n.words[4]==kind).count() as u32;
        if mind.words.get(&0xe0)!=Some(&count(1)) || mind.words.get(&0xe4)!=Some(&count(2)) {
            return Err(fail("restored path nodes disagree with the loaded kind counts"));
        }
        self.bind_element_with(mind,nodes)
    }
    fn bind_element_with(&mut self,mind:&mut Installed,nodes:Vec<aa_navigation::PathNode>)->Result<StateId> {
        if mind.kind!=Kind::Element {return Err(fail("Move payload requires original Element constructor"));}
        let state=*mind.state_bindings.get(&0x7c).ok_or_else(||fail("Element Move state binding missing"))?;
        if self.states.contains_key(&state) {return Err(fail("Move payload already installed"));}
        let word=|offset|mind.words.get(&offset).copied().ok_or_else(||fail(format!("constructor field {offset:#x} was not initialized")));
        if word(0xd8)?!=0 || word(0xdc)?!=0 {return Err(fail("nonempty native path needs source restoration"));}
        if mind.references.get(&0xd4)!=Some(&Reference::NativeSubobject{vtable:0xa34738}) {
            return Err(fail("Move owned-path constructor identity mismatch"));
        }
        let dirty=*mind.bytes.get(&0xe8).ok_or_else(||fail("Move path dirty byte was not initialized"))?;
        let payload=Payload {
            plan:result_plan::State {
                movement:MoveState {target_id:word(0x8c)?,destination_bits:[word(0x90)?,word(0x94)?],
                    auxiliary_bits:[word(0x98)?,word(0x9c)?],requested_speed_bits:word(0xa0)?,phase:word(0xa8)?,mode:word(0xac)?,waypoint_offset_bits:word(0xd0)?,
                    path:OwnedPath {nodes,kind1_count:word(0xe0)? as i32,kind2_count:word(0xe4)? as i32,dirty:dirty!=0,word18:word(0xec)?}},
                attempts:word(0xa4)? as i32,previous_destination_bits:[word(0xb0)?,word(0xb4)?]},
            collision:[word(0xb8)?,word(0xbc)?,word(0xc0)?,word(0xc4)?],collision_vector:[word(0xc8)?,word(0xcc)?],detached_time_bits:word(0xf0)?,word78:word(0xf4)?,
        };
        // No fallible callback occurs between publication and source removal.
        self.states.insert(state,PayloadHandle(Arc::new(RwLock::new(payload))));
        for offset in [0x8c,0x90,0x94,0x98,0x9c,0xa0,0xa4,0xa8,0xac,0xb0,0xb4,0xb8,0xbc,0xc0,0xc4,0xc8,0xcc,0xd0,0xd8,0xdc,0xe0,0xe4,0xec,0xf0,0xf4] {
            mind.words.remove(&offset);
        }
        mind.bytes.remove(&0xe8);mind.references.remove(&0xd4);
        Ok(state)
    }
}

/// Disjoint canonical stores during active-component switching. Native effects
/// may update the active slot again; the caller rereads it after each callback.
pub struct ActivationContext<'a,'store> {
    pub actors:&'a mut ActorAccess<'store>,
    pub spatial:&'a mut SpatialWorld,
    pub queues:&'a mut Queues,
    pub owners:&'a Owners,
}
pub trait ActivationEffects {
    fn permits_inactive(&mut self,actor:ActorId,slot:ComponentSlot,context:&mut ActivationContext<'_,'_>)->Result<bool>;
    fn enter_other(&mut self,actor:ActorId,slot:ComponentSlot,same:bool,context:&mut ActivationContext<'_,'_>)->Result<()>;
    fn leave_other(&mut self,actor:ActorId,slot:ComponentSlot,same:bool,context:&mut ActivationContext<'_,'_>)->Result<()>;
    ///42A590 on source ActorDefinition+24C (start) or+254 (stop). Source FX
    /// definitions must be consulted; an unimplemented nonempty event is error.
    fn mover_effect(&mut self,actor:ActorId,started:bool,context:&mut ActivationContext<'_,'_>)->Result<()>;
}
fn key(actors:&ActorAccess<'_>,actor:ActorId)->Result<ObjectKey>{actors.key(actor).ok_or_else(||fail("actor GTID is not live"))}
fn active(actors:&ActorAccess<'_>,actor:ActorId)->Result<Option<ComponentSlot>>{
    match actors.object(key(actors,actor)?).map_err(fail)?.active_component {
        ActiveComponent::None=>Ok(None),ActiveComponent::Slot(slot)=>Ok(Some(slot)),
        ActiveComponent::UnresolvedNativeWord(_)=>Err(fail("saved active component reference is unresolved")),
    }
}
fn cancel_mover(actor:ActorId,context:&mut ActivationContext<'_,'_>)->Result<()> {
    let owner=context.owners.get(Target::Component{object:key(context.actors,actor)?,slot:ComponentSlot::Mover as u8}).map_err(fail)?;
    context.queues.cancel_owner(owner).map_err(fail)?;Ok(())
}
fn mover_mut<'a>(actors:&'a mut ActorAccess<'_>,actor:ActorId)->Result<&'a mut aa_actors::ground_states::MoverState>{
    match actors.component_mut(actor,ComponentSlot::Mover).map_err(fail)? {
        Some(RuntimeComponent::Mover(value))=>Ok(value),_=>Err(fail("required Mover component is absent")),
    }
}
///Actual4CFC10, including its unusual same-component leave/enter sequence.
///Mover enter576790 and leave576B60 bind real canonical timers and field writes.
pub fn activate_component(actor:ActorId,slot:Option<ComponentSlot>,context:&mut ActivationContext<'_,'_>,effects:&mut impl ActivationEffects)->Result<bool>{
    if let Some(slot)=slot {
        if context.actors.component(actor,slot).map_err(fail)?.is_none(){return Err(fail("requested active component is absent"));}
        let flags=context.actors.placement(key(context.actors,actor)?,context.spatial).map_err(fail)?.flags;
        if flags&1!=0 {
            //Concrete Mover vtable+44 is6A71F0 (false).
            let allowed=slot!=ComponentSlot::Mover && effects.permits_inactive(actor,slot,context)?;
            if !allowed {return Ok(false);}
        }
    }
    let previous=active(context.actors,actor)?;
    let same=slot.is_some() && previous==slot;
    context.actors.object_mut(key(context.actors,actor)?).map_err(fail)?.active_component=slot.map_or(ActiveComponent::None,ActiveComponent::Slot);
    if let Some(old)=previous {
        if old==ComponentSlot::Mover {
            cancel_mover(actor,context)?;
            let mover=mover_mut(context.actors,actor)?;
            mover.bytes.insert(0x55,0);
            if !same {mover.words.insert(0x20,0);}
            mover.words.insert(0x1c,0);
            if !same {effects.mover_effect(actor,false,context)?;}
        } else {effects.leave_other(actor,old,same,context)?;}
    }
    let Some(next)=active(context.actors,actor)? else{return Ok(false);};
    if next==ComponentSlot::Mover {
        cancel_mover(actor,context)?;
        if !same {effects.mover_effect(actor,true,context)?;}
    } else {effects.enter_other(actor,next,same,context)?;}
    Ok(true)
}

pub struct MotionContext<'a,'store> {
    pub arena:&'a mut aa_mind::Arena,
    pub actors:&'a mut ActorAccess<'store>,
    pub spatial:&'a mut SpatialWorld,
    pub queues:&'a mut Queues,
    pub owners:&'a Owners,
    pub random:&'a mut NativeRng,
}
/// Remaining actual actor/Mind/FX effects. Pose, shapes, collision, timers,
/// Character pause/resume and Mover storage are bound by MotionWorld itself.
pub trait MotionEffects:ActivationEffects {
    /// Actor+F8 TerrainData+38, derived from source properties. No default speed.
    fn terrain_speed_limit(&self,actor:ActorId,actors:&ActorAccess<'_>,spatial:&SpatialWorld)->Result<u32>;
    fn event(&mut self,actor:ActorId,event:Event,context:&mut MotionContext<'_,'_>)->Result<bool>;
    fn blocked(&mut self,actor:ActorId,hit:&Hit,facing:[u32;2],context:&mut MotionContext<'_,'_>)->Result<()>;
    fn position_changed(&mut self,actors:&mut ActorAccess<'_>,index:&mut SpatialIndex,key:u32)->aa_spatial::Result<()>;
    fn world_phase(&self)->u32;
    fn presentation_available(&self)->bool;
    fn presentation_position_changed(&mut self,actors:&mut ActorAccess<'_>,index:&mut SpatialIndex,key:u32)->aa_spatial::Result<()>;
    fn presentation_facing_changed(&mut self,actors:&mut ActorAccess<'_>,index:&mut SpatialIndex,key:u32)->aa_spatial::Result<()>;
}
struct Transform<'a,'store,E> {actors:&'a mut ActorAccess<'store>,effects:&'a mut E}
impl<E:MotionEffects> TransformCallbacks for Transform<'_,'_,E> {
    fn position_changed(&mut self,index:&mut SpatialIndex,key:u32)->aa_spatial::Result<()>{self.effects.position_changed(self.actors,index,key)}
    fn world_phase(&self)->u32{self.effects.world_phase()}
    fn presentation_available(&self)->bool{self.effects.presentation_available()}
    fn presentation_position_changed(&mut self,index:&mut SpatialIndex,key:u32)->aa_spatial::Result<()>{self.effects.presentation_position_changed(self.actors,index,key)}
    fn presentation_facing_changed(&mut self,index:&mut SpatialIndex,key:u32)->aa_spatial::Result<()>{self.effects.presentation_facing_changed(self.actors,index,key)}
}
fn movement_error(e:impl std::fmt::Display)->aa_movement::Error{aa_movement::Error::World(e.to_string())}

/// Scoped5773C0/5774A0 receiver over actual stores. Snapshots are published
/// before callbacks and refreshed afterwards, including partial-error paths.
pub struct MotionWorld<'a,'store,'surface,E> {
    pub actor:ActorId,
    pub context:MotionContext<'a,'store>,
    pub surface:&'a GroundSurface<'surface>,
    pub geometry:&'a QueryGeometry,
    pub policy:crate::spatial_policy::PolicyInputs<'a>,
    pub effects:&'a mut E,
}
impl<E:MotionEffects> MotionWorld<'_,'_,'_,E> {
    fn snapshot(&self)->aa_movement::Result<(MoverState,ActorMotion)> {
        let actor=self.actor;
        let component=self.context.actors.component(actor,ComponentSlot::Mover).map_err(movement_error)?;
        let Some(RuntimeComponent::Mover(mover))=component else {return Err(movement_error("required Mover missing"));};
        if mover.header.owner!=actor {return Err(movement_error("Mover owner identity mismatch"));}
        let mover=mover.movement_state().map_err(movement_error)?;
        let k=key(self.context.actors,actor).map_err(movement_error)?;
        let p=self.context.actors.placement(k,self.context.spatial).map_err(movement_error)?;
        let character=self.context.actors.component(actor,ComponentSlot::Character).map_err(movement_error)?;
        let (has_character,character_stopped)=match character {Some(RuntimeComponent::Character(c))=>(true,c.active),None=>(false,false),_=>return Err(movement_error("Character slot payload mismatch"))};
        let limit=self.effects.terrain_speed_limit(actor,self.context.actors,self.context.spatial).map_err(movement_error)?;
        Ok((mover,ActorMotion {position:p.position_bits.map(f32::from_bits),facing:p.facing_bits.map(f32::from_bits),flags:p.flags,
            has_mind:self.context.actors.component(actor,ComponentSlot::Mind).map_err(movement_error)?.is_some(),has_character,character_stopped,terrain_speed_limit:f32::from_bits(limit)}))
    }
    fn publish(&mut self,mover:&MoverState)->aa_movement::Result<()> {
        mover_mut(self.context.actors,self.actor).map_err(movement_error)?.apply_movement_state(mover);Ok(())
    }
    fn refresh<T>(&mut self,result:aa_movement::Result<T>,m:&mut MoverState,a:&mut ActorMotion)->aa_movement::Result<T> {
        match self.snapshot() {
            Ok((fresh_m,fresh_a))=>{*m=fresh_m;*a=fresh_a;result},
            Err(refresh)=>match result {Ok(_)=>Err(refresh),Err(first)=>Err(movement_error(format!("{first}; refresh: {refresh}")))},
        }
    }
    fn mover_owner(&self)->aa_movement::Result<aa_world::queues::Owner> {
        self.context.owners.get(Target::Component {object:key(self.context.actors,self.actor).map_err(movement_error)?,slot:ComponentSlot::Mover as u8}).map_err(movement_error)
    }
    pub fn begin(&mut self,order:aa_movement::GroundOrder,precision_radius_bits:u32)->aa_movement::Result<bool> {
        let (mut mover,mut actor)=self.snapshot()?;
        let result=aa_movement::begin_ground_motion(&mut mover,&mut actor,order,f32::from_bits(precision_radius_bits),self.context.queues.precision,self);
        let published=self.publish(&mover);
        match (result,published) {(Ok(v),Ok(()))=>Ok(v),(Err(e),_)=>Err(e),(Ok(_),Err(e))=>Err(e)}
    }
    pub fn advance(&mut self,acceleration_delta_bits:u32,terrain_minimum_bits:u32)->aa_movement::Result<aa_movement::StepOutcome> {
        let (mut mover,mut actor)=self.snapshot()?;
        let context=aa_movement::StepContext {acceleration_delta:f32::from_bits(acceleration_delta_bits),world_delta:f32::from_bits(self.context.queues.clock.step_bits),terrain_minimum_move_speed:f32::from_bits(terrain_minimum_bits),precision:self.context.queues.precision};
        let result=aa_movement::ground_step_original_pc53(&mut mover,&mut actor,context,self);
        let published=self.publish(&mover);
        match (result,published) {(Ok(v),Ok(()))=>Ok(v),(Err(e),_)=>Err(e),(Ok(_),Err(e))=>Err(e)}
    }
    /// Same5774A0 step at the queue's explicit precision. The caller supplies
    /// the sticky x87 precision-exception state pending at the445400 call
    /// (the simulation thread runs PC53 with PE set every tick).
    pub fn advance_original(&mut self,acceleration_delta_bits:u32,terrain_minimum_bits:u32,precision_exception_pending:bool)->aa_movement::Result<aa_movement::StepOutcome> {
        let (mut mover,mut actor)=self.snapshot()?;
        let context=aa_movement::StepContext {acceleration_delta:f32::from_bits(acceleration_delta_bits),world_delta:f32::from_bits(self.context.queues.clock.step_bits),terrain_minimum_move_speed:f32::from_bits(terrain_minimum_bits),precision:self.context.queues.precision};
        let result=aa_movement::ground_step_original(&mut mover,&mut actor,context,precision_exception_pending,self);
        let published=self.publish(&mover);
        match (result,published) {(Ok(v),Ok(()))=>Ok(v),(Err(e),_)=>Err(e),(Ok(_),Err(e))=>Err(e)}
    }
}
impl<E:MotionEffects> GroundWorld for MotionWorld<'_,'_,'_,E> {
    type Collision=Hit;
    fn activate(&mut self,m:&mut MoverState,a:&mut ActorMotion)->aa_movement::Result<bool>{
        self.publish(m)?;
        let result=activate_component(self.actor,Some(ComponentSlot::Mover),&mut ActivationContext {actors:self.context.actors,spatial:self.context.spatial,queues:self.context.queues,owners:self.context.owners},self.effects).map_err(movement_error);
        self.refresh(result,m,a)
    }
    fn target_position(&mut self,id:u32)->aa_movement::Result<Option<[f32;2]>>{
        let Some(k)=self.context.actors.key(ActorId::from_raw(id)) else{return Ok(None);};
        let p=self.context.actors.placement(k,self.context.spatial).map_err(movement_error)?;
        Ok(Some([f32::from_bits(p.position_bits[0]),f32::from_bits(p.position_bits[1])]))
    }
    fn shapes_in_range(&mut self,_:&ActorMotion,id:u32,range:f32)->aa_movement::Result<bool>{
        let source=key(self.context.actors,self.actor).map_err(movement_error)?;
        let target=key(self.context.actors,ActorId::from_raw(id)).map_err(movement_error)?;
        aa_spatial::soft_range::source_receiver_in_range(source,target,range.to_bits(),self.context.queues.precision,&mut self.context.spatial.soft_view(&*self.context.actors)).map_err(movement_error)
    }
    fn collision_at(&mut self,_:&ActorMotion,xy:[f32;2])->aa_movement::Result<Option<Hit>>{
        let object=key(self.context.actors,self.actor).map_err(movement_error)?;
        let actor=self.context.spatial.key(object).map_err(movement_error)?;
        let (index,bindings)=self.context.spatial.split_index();
        let mut policy=self.policy.bind(self.context.actors,self.context.arena,bindings);
        let mut query=aa_spatial::actor_context::prepare(index,actor,false,&policy).map_err(movement_error)?;
        query.set_position(index,xy.map(f32::to_bits),&policy).map_err(movement_error)?;
        let hit=aa_spatial::query::first_hit(index,self.geometry,&query,&mut policy).map_err(movement_error)?;
        Ok(hit.blocked().then_some(hit))
    }
    fn target_position_with_state(&mut self,id:u32,m:&mut MoverState,a:&mut ActorMotion)->aa_movement::Result<Option<[f32;2]>>{
        self.publish(m)?;let result=self.target_position(id);self.refresh(result,m,a)
    }
    fn shapes_in_range_with_state(&mut self,id:u32,range:f32,m:&mut MoverState,a:&mut ActorMotion)->aa_movement::Result<bool>{
        self.publish(m)?;let result=self.shapes_in_range(a,id,range);self.refresh(result,m,a)
    }
    fn collision_at_with_state(&mut self,xy:[f32;2],m:&mut MoverState,a:&mut ActorMotion)->aa_movement::Result<Option<Hit>>{
        self.publish(m)?;let result=self.collision_at(a,xy);self.refresh(result,m,a)
    }
    fn set_facing(&mut self,m:&mut MoverState,a:&mut ActorMotion,xy:[f32;2])->aa_movement::Result<()> {
        self.publish(m)?;let object=key(self.context.actors,self.actor).map_err(movement_error)?;
        let result=self.context.spatial.set_facing(object,xy.map(f32::to_bits),&mut Transform {actors:self.context.actors,effects:self.effects}).map_err(movement_error);
        self.refresh(result,m,a)
    }
    fn move_on_ground(&mut self,m:&mut MoverState,a:&mut ActorMotion,xy:[f32;2])->aa_movement::Result<()> {
        self.publish(m)?;let object=key(self.context.actors,self.actor).map_err(movement_error)?;
        let result=self.context.spatial.move_ground(object,xy.map(f32::to_bits),self.surface,&mut Transform {actors:self.context.actors,effects:self.effects}).map_err(movement_error);
        self.refresh(result,m,a)
    }
    fn notify(&mut self,event:Event,m:&mut MoverState,a:&mut ActorMotion)->aa_movement::Result<bool> {
        self.publish(m)?;
        let result=(||->aa_movement::Result<bool>{match event {
            Event::ScheduleImmediate=>{let owner=self.mover_owner()?;self.context.queues.add_periodic(0,owner,0,self.context.random).map_err(movement_error)?;Ok(true)},
            Event::CancelScheduled=>{let owner=self.mover_owner()?;self.context.queues.cancel_owner(owner).map_err(movement_error)?;Ok(true)},
            Event::CharacterStop|Event::CharacterResume=>{
                let Some(RuntimeComponent::Character(c))=self.context.actors.component_mut(self.actor,ComponentSlot::Character).map_err(movement_error)? else{return Err(movement_error("required Character missing"));};
                let now=aa_body::F32(self.context.queues.clock.begin_bits);
                if event==Event::CharacterStop {aa_animation::runtime::pause(c,now)} else {aa_animation::runtime::resume(c,now,self.context.queues.precision)}.map_err(movement_error)?;
                Ok(true)
            },
            _=>self.effects.event(self.actor,event,&mut self.context).map_err(movement_error),
        }})();self.refresh(result,m,a)
    }
    fn blocked(&mut self,hit:&Hit,facing:[f32;2],m:&mut MoverState,a:&mut ActorMotion)->aa_movement::Result<()> {
        self.publish(m)?;let result=self.effects.blocked(self.actor,hit,facing.map(f32::to_bits),&mut self.context).map_err(movement_error);self.refresh(result,m,a)
    }
}
