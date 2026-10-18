//!527E30 ground move over SAME Installed scalar fields, Arena status and
//!regiment_owned path. Shared navigation coordinators also serve the old lab
//!APIs; this adapter never constructs a CompanyMoveState/status/path copy.
use super::*;
use aa_navigation::company_move::{self as native,CompanyActivationFields,CompanySpeedFields,CompanyExitFields,CompanyLifecycleFields,Extents};
fn nav(e:impl std::fmt::Display)->aa_navigation::Error{aa_navigation::Error::World(e.to_string())}
type NResult<T>=aa_navigation::Result<T>;
struct Move<'a,'b,'context,'store,'source>{bound:&'a mut Bound<'b,'source>,context:&'a mut Context<'context,'store>,binding:StateBinding,state:StateId}
impl Move<'_,'_,'_,'_,'_>{
    fn read(&self,offset:u32)->NResult<u32>{installed(self.context.actors,self.context.arena,self.binding.mind).map_err(nav)?.words.get(&offset).copied().ok_or_else(||nav(format!("actual Regiment+{offset:X} scalar was not initialized")))}
    fn write(&mut self,offset:u32,value:u32)->NResult<()>{
        let Some(RuntimeComponent::Mind(m))=self.context.actors.component_mut(self.binding.actor,Slot::Mind).map_err(nav)?else{return Err(nav("actual Regiment component absent"))};
        if m.kind!=Kind::Regiment||m.mind_id!=self.binding.mind{return Err(nav("actual Regiment installed/core identity differs"))}
        m.words.insert(offset,value);Ok(())
    }
    fn organization(&self)->NResult<&aa_actors::ground_states::OrganizationState<String>>{
        match self.context.actors.component(self.binding.actor,Slot::Organization).map_err(nav)?{
            Some(RuntimeComponent::Organization(o))if o.header.owner==self.binding.actor=>Ok(o),_=>Err(nav("actual Regiment Organization owner absent")),
        }
    }
    fn owned(&mut self)->NResult<&mut aa_mind::regiment_load::Owned>{
        let id=self.binding.mind;
        if !self.context.actors.regiment_owned.contains_key(&id){
            let Some(RuntimeComponent::Mind(m))=self.context.actors.component_mut(self.binding.actor,Slot::Mind).map_err(nav)?else{return Err(nav("actual Regiment component absent"))};
            if m.mind_id!=id{return Err(nav("Regiment constructor collections belong to another Mind"))}
            let owner=aa_mind::regiment_load::take_constructed_owned(m).map_err(nav)?;
            self.context.actors.regiment_owned.insert(id,owner);
        }
        self.context.actors.regiment_owned.get_mut(&id).ok_or_else(||nav("same Regiment collection transfer absent"))
    }
    fn point(&mut self,offset:u32,point:[u32;2],extent:Extents,p:Precision)->NResult<()>{
        self.write(offset,point[0])?;self.write(offset+4,point[1])?;
        for axis in 0..2{let value=native::clamp_coordinate(extent.extent_bits[axis],self.read(offset+axis as u32*4)?,p)?;self.write(offset+axis as u32*4,value)?;}Ok(())
    }
}
impl CompanyActivationFields for Move<'_,'_,'_,'_,'_>{
    fn can_activate(&mut self,mode:u32)->NResult<bool>{
        if matches!(mode,0|1){return Ok(true)}let state=self.state;
        self.bound.with_world(self.context,|w,arena|aa_mind::World::can_activate(w,state,arena)).map_err(nav)
    }
    fn store_target(&mut self,id:u32)->NResult<()>{self.write(0x78,id)}
    fn target_position(&mut self,id:u32)->NResult<[u32;2]>{
        let actor=ActorId::from_raw(id);let key=key(self.context.actors,actor).map_err(nav)?;
        let p=self.context.actors.placement(key,self.bound.spatial).map_err(nav)?;Ok([p.position_bits[0],p.position_bits[1]])
    }
    fn requested_destination(&mut self,point:[u32;2],e:Extents,p:Precision)->NResult<()>{self.point(0x84,point,e,p)}
    fn group_offset(&mut self)->NResult<Option<[u32;2]>>{
        let group=ActorId::from_raw(word(self.context.actors,self.binding.actor,0x164).map_err(nav)?);
        if self.context.actors.key(group).is_none(){return Ok(None)}
        Err(nav("51D840 positive same Group4D1900/4DDEE0/4DE6C0 formation offset required"))
    }
    fn path_destination(&mut self,mut point:[u32;2],e:Extents,p:Precision)->NResult<()>{
        //51D840 fallback fld/fstp quiets X; the second coordinate is MOV.
        point[0]=aa_mind::cai_thinks::x87_copy(point[0]);self.point(0x7c,point,e,p)
    }
    fn speed_auxiliary(&mut self,speed:u32,aux:[u32;2])->NResult<()>{self.write(0x94,speed)?;self.write(0x8c,aux[0])?;self.write(0x90,aux[1])}
    fn location_diagnostic(&mut self,_:[u32;2])->NResult<()>{
        if self.context.runtime.path_log()==0{Ok(())}else{Err(nav("520466 actual PathLog eager position/identity formatter and retained diagnostic transport required"))}
    }
    fn target_diagnostic(&mut self,_:u32)->NResult<()>{
        if self.context.runtime.path_log()==0{Ok(())}else{Err(nav("5203C4 actual PathLog target formatter and retained diagnostic transport required"))}
    }
    fn activate(&mut self,mode:u32)->NResult<bool>{let state=self.state;self.bound.with_world(self.context,|w,arena|aa_mind::activate(state,mode,arena,w)).map_err(nav)}
}
impl CompanySpeedFields for Move<'_,'_,'_,'_,'_>{
    fn organization_speed_cap(&mut self)->NResult<u32>{self.organization()?.words.get(&0xd8).copied().ok_or_else(||nav("actual Organization+D8 current speed cap uninitialized"))}
    fn store_speed(&mut self,value:u32)->NResult<()>{self.write(0x94,value)}
    fn broadcast_speed(&mut self)->NResult<()>{
        //4364C0 includeInactive=0: skip NULL slots; reread actual vector on
        //every increment after the member callback. No copied member list.
        let mut slot=0usize;
        loop{
            let Some(member)=self.organization()?.members.get(slot).copied()else{return Ok(())};slot+=1;
            if member.is_null(){continue}
            let m=match self.context.actors.component(member,Slot::Mind).map_err(nav)?{
                Some(RuntimeComponent::Mind(m))=>m,_=>return Err(nav("52065A actual member Mind absent")),
            };
            let offset=match m.kind{Kind::Element=>0x7c,Kind::Regiment=>0x68,_=>return Err(nav("520661 member Mind virtual64 concrete kind required"))};
            let state=*m.state_bindings.get(&offset).ok_or_else(||nav("520661 same member Move state absent"))?;
            if self.context.arena.state(state).map_err(nav)?.status!=2{continue}
            let mind=m.mind_id;let kind=m.kind;let speed=self.read(0x94)?;
            if kind==Kind::Regiment{
                let binding=StateBinding{actor:member,mind,kind,offset};
                native::set_speed_on_fields(&mut Move{bound:self.bound,context:self.context,binding,state},speed)?;
            }else{
                let handle=self.context.runtime.movement.handle(state).map_err(nav)?;
                handle.write().map_err(nav)?.plan.movement.requested_speed_bits=speed;
                return Err(nav("535F00/52E610 after actual member speed store requires current GroupSpeedFloor/Stop or576D20 effects"))
            }
        }
    }
}
impl CompanyExitFields for Move<'_,'_,'_,'_,'_>{
    fn store_status(&mut self,value:u32)->NResult<()>{self.context.arena.state_mut(self.state).map_err(nav)?.status=value;Ok(())}
    fn reset_owned_path(&mut self)->NResult<()>{native::reset_path(&mut self.owned()?.path);Ok(())}
    fn store_tries(&mut self,value:i32)->NResult<()>{self.write(0x98,value as u32)}
}
impl CompanyLifecycleFields for Move<'_,'_,'_,'_,'_>{
    fn copy_auxiliary_to_mind(&mut self)->NResult<()>{let x=aa_mind::cai_thinks::x87_copy(self.read(0x8c)?);let y=aa_mind::cai_thinks::x87_copy(self.read(0x90)?);self.write(0x2b4,x)?;self.write(0x2b8,y)}
    fn current_speed(&mut self)->NResult<u32>{self.read(0x94)}
    fn current_target_position(&mut self)->NResult<Option<[u32;2]>>{
        let id=self.read(0x78)?;if self.context.actors.key(ActorId::from_raw(id)).is_none(){return Ok(None)}self.target_position(id).map(Some)
    }
    fn clear_formation(&mut self)->NResult<()>{self.owned()?.desired_formation=None;Ok(())}
    fn current_world_time(&mut self)->NResult<u32>{Ok(self.bound.clock.begin.0)}
    fn store_start_time(&mut self,value:u32)->NResult<()>{self.write(0xbc,value)}
}
fn state(c:&Context<'_,'_>,mind:MindId)->aa_mind::Result<(StateBinding,StateId)>{
    let m=installed(c.actors,c.arena,mind)?;
    if m.kind!=Kind::Regiment{return required("527E30 requires same installed Regiment")}
    let state=*m.state_bindings.get(&0x68).ok_or_else(||error("527E30 actual Move state absent"))?;
    Ok((mind_world::state_binding(c.actors,c.arena,state)?,state))
}
pub(super) fn command(bound:&mut Bound<'_,'_>,context:&mut Context<'_,'_>,mind:MindId,command:u32)->aa_mind::Result<bool>{
    let current=context.runtime.commands.value(command)?;
    let flags=current.flags;
    let(aux,speed)=crate::regiment_command::move_arguments(flags,current.auxiliary_bits,current.scalar_bits);
    if flags&1!=0{return required("527E8B targeted Move actual Organization578AC0/57A5E0 then target16C/Bombard/Kill children required")}
    let point=current.position_bits;let(binding,state)=state(context,mind)?;
    let extents=Extents{extent_bits:bound.zones.geometry.extent_bits};let pc=bound.pc;
    native::activate_to_location_on_fields(&mut Move{bound,context,binding,state},2,point,speed,aux,extents,pc).map(|v|v.result).map_err(error)
}
pub(super) fn enter(bound:&mut Bound<'_,'_>,context:&mut Context<'_,'_>,binding:StateBinding,state:StateId)->aa_mind::Result<()>{
    let extents=Extents{extent_bits:bound.zones.geometry.extent_bits};let pc=bound.pc;
    native::enter_on_fields(&mut Move{bound,context,binding,state},extents,pc).map_err(error)
}
pub(super) fn leave(bound:&mut Bound<'_,'_>,context:&mut Context<'_,'_>,binding:StateBinding,state:StateId)->aa_mind::Result<()>{native::leave_on_fields(&mut Move{bound,context,binding,state}).map_err(error)}
///520250's real current zone-vector guard precedes the current command read.
///Positive slot4 POINTER reaches the target/arrival/combat predicates.
pub(super) fn actor_state(bound:&Bound<'_,'_>,context:&Context<'_,'_>,binding:StateBinding)->aa_mind::Result<u8>{
    let object=context.actors.object(key(context.actors,binding.actor)?).map_err(error)?;
    let zones=&object.actor.as_ref().ok_or_else(||error("520250 same Actor shell absent"))?.zones_148;
    if crate::regiment_command::move_zoc_requires_predicates(zones.len(),||zones.zones().slots[4].is_some()){
        let target=ActorId::from_raw(*installed(context.actors,context.arena,binding.mind)?.words.get(&0x78)
            .ok_or_else(||error("52FD00 actual Move target+10 uninitialized"))?);
        let flags=context.actors.key(target).map(|key|context.actors.placement(key,bound.spatial).map(|p|p.flags).map_err(error)).transpose()?;
        if crate::regiment_command::move_has_live_target(flags,||target==binding.actor){
            return required("5202A0 actual positive live-target4CEDE0/51D7F0/51FBC0 predicates required")
        }
    }
    let Some(id)=context.arena.mind(binding.mind)?.current_command else{return Ok(crate::regiment_command::move_actor_state_after_zoc(None))};
    let kind=context.runtime.commands.value(id)?.definition.ok_or_else(||error("5202F1 actual current command definition absent"))?.mind_type;
    Ok(crate::regiment_command::move_actor_state_after_zoc(Some(kind)))
}
