//!524D70 on SAME Installed payload/Arena/list/queues and522890 services.
//!Full shared Think chooses each branch; unavailable positive world children
//!remain explicit reached errors after their actual preceding stores.
use super::*;
use aa_mind::cai_sleep::SleepThinkWorld;
struct Sleep<'a, 'b, 'store, 'source> {
    bound: &'a mut Bound<'b, 'source>,
    actors: &'a mut ActorAccess<'store>,
    runtime: &'a mut mind_world::Runtime,
    queues: &'a mut aa_world::queues::Queues,
    owners: &'a mut crate::scheduler_owners::Owners,
    binding: StateBinding,
}
impl Sleep<'_, '_, '_, '_> {
    fn current(&self) -> aa_mind::Result<&Installed> {
        match self
            .actors
            .component(self.binding.actor, Slot::Mind)
            .map_err(error)?
        {
            Some(RuntimeComponent::Mind(m))
                if m.kind == Kind::Regiment && m.mind_id == self.binding.mind =>
            {
                Ok(m)
            }
            _ => required("CAISleep current Regiment allocation differs"),
        }
    }
    fn read(&self, offset: u32) -> aa_mind::Result<u32> {
        self.current()?
            .words
            .get(&offset)
            .copied()
            .ok_or_else(|| error(format!("CAISleep actual Mind+{offset:X} word is unwritten")))
    }
    fn byte(&self, offset: u32) -> aa_mind::Result<u8> {
        self.current()?
            .bytes
            .get(&offset)
            .copied()
            .ok_or_else(|| error(format!("CAISleep actual Mind+{offset:X} byte is unwritten")))
    }
    fn write(&mut self, offset: u32, value: u32) -> aa_mind::Result<()> {
        self.current()?;
        let Some(RuntimeComponent::Mind(m)) = self
            .actors
            .component_mut(self.binding.actor, Slot::Mind)
            .map_err(error)?
        else {
            unreachable!()
        };
        m.words.insert(offset, value);
        Ok(())
    }
    fn write_byte(&mut self, offset: u32, value: u8) -> aa_mind::Result<()> {
        self.current()?;
        let Some(RuntimeComponent::Mind(m)) = self
            .actors
            .component_mut(self.binding.actor, Slot::Mind)
            .map_err(error)?
        else {
            unreachable!()
        };
        m.bytes.insert(offset, value);
        Ok(())
    }
    fn organization_fields(
        &self,
    ) -> aa_mind::Result<&aa_actors::ground_states::OrganizationState<String>> {
        match self
            .actors
            .component(self.binding.actor, Slot::Organization)
            .map_err(error)?
        {
            Some(RuntimeComponent::Organization(v)) if v.header.owner == self.binding.actor => {
                Ok(v)
            }
            _ => required("CAISleep actual Organization owner absent"),
        }
    }
    fn with_world<T>(
        &mut self,
        arena: &mut Arena,
        f: impl FnOnce(&mut mind_world::World<'_, '_, Bound<'_, '_>>, &mut Arena) -> aa_mind::Result<T>,
    ) -> aa_mind::Result<T> {
        let mut context = Context {
            actors: self.actors,
            arena,
            runtime: self.runtime,
            queues: self.queues,
            owners: self.owners,
        };
        self.bound.with_world(&mut context, f)
    }
    fn arrived(
        &mut self,
        mind: MindId,
        member: u32,
        flag: u8,
        arena: &mut Arena,
    ) -> aa_mind::Result<()> {
        crate::stored_member_command::run_with_fields(
            self.bound.catalog,
            self.actors,
            self.bound.bank,
            crate::player_order_frame::Fields {
                spatial: self.bound.spatial,
                variables: self.bound.variables,
                presentation: self.bound.presentation,
                zones: self.bound.zones,
                random: self.bound.random,
                clock: self.bound.clock,
                inventory: self.bound.inventory,
                terrain: self.bound.terrain,
                mission: self.bound.mission,
                phase: self.bound.phase,
                current_settings: self.bound.current_settings,
            },
            self.runtime,
            mind,
            ActorId::from_raw(member),
            flag,
            arena,
            self.queues,
            self.owners,
            self.bound.pc,
        )
    }
    fn next_member(
        &self,
        index: &mut usize,
        attached: bool,
    ) -> aa_mind::Result<Option<(usize, ActorId)>> {
        loop {
            let slot = *index;
            let Some(member) = self.organization_fields()?.members.get(slot).copied() else {
                return Ok(None);
            };
            *index += 1;
            if member.is_null() {
                continue;
            }
            if attached {
                let Some(RuntimeComponent::Element(e)) = self
                    .actors
                    .component(member, Slot::Element)
                    .map_err(error)?
                else {
                    return required("4364C0 attached iteration actual Element absent");
                };
                if e.header.owner != member {
                    return required("4364C0 actual Element owner differs");
                }
                if !e.flag_1c {
                    continue;
                }
            }
            return Ok(Some((slot, member)));
        }
    }
    fn position(&self, member: ActorId) -> aa_mind::Result<[u32; 2]> {
        let p = self
            .actors
            .placement(key(self.actors, member)?, &*self.bound.spatial)
            .map_err(error)?
            .position_bits;
        Ok([p[0], p[1]])
    }
    fn all_at_slots(&self) -> aa_mind::Result<bool> {
        let mut index = 0;
        while let Some((slot, member)) = self.next_member(&mut index, true)? {
            let org = self.organization_fields()?;
            let offset = org
                .offsets_78
                .get(slot)
                .ok_or_else(|| error("520190 actual Organization offset slot absent"))?;
            let mut point = [0; 2];
            for axis in [1, 0] {
                let a =
                    aa_numeric::Binary::from_f32_bits(org.position_xy[axis].0).map_err(error)?;
                let b = aa_numeric::Binary::from_f32_bits(offset[axis].0).map_err(error)?;
                point[axis] = a
                    .add(&b, self.bound.pc)
                    .map_err(error)?
                    .store_f32()
                    .map_err(error)?;
            }
            if !aa_mind::cai_member_command::close(self.position(member)?, point, self.bound.pc)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
impl aa_mind::World for Sleep<'_, '_, '_, '_> {
    fn can_activate(&mut self, s: StateId, a: &mut Arena) -> aa_mind::Result<bool> {
        self.with_world(a, |w, a| w.can_activate(s, a))
    }
    fn enter(&mut self, s: StateId, a: &mut Arena) -> aa_mind::Result<()> {
        self.with_world(a, |w, a| w.enter(s, a))
    }
    fn leave(&mut self, s: StateId, a: &mut Arena) -> aa_mind::Result<()> {
        self.with_world(a, |w, a| w.leave(s, a))
    }
    fn schedule(&mut self, m: MindId, d: u32, data: u32, a: &mut Arena) -> aa_mind::Result<()> {
        self.with_world(a, |w, a| w.schedule(m, d, data, a))
    }
    fn reset_default(&mut self, m: MindId, a: &mut Arena) -> aa_mind::Result<()> {
        self.with_world(a, |w, a| w.reset_default(m, a))
    }
    fn state_changed(&mut self, m: MindId, a: &mut Arena) -> aa_mind::Result<()> {
        self.with_world(a, |w, a| w.state_changed(m, a))
    }
}
impl SleepThinkWorld for Sleep<'_, '_, '_, '_> {
    fn organization_status(&mut self, _: MindId, a: &Arena) -> aa_mind::Result<u32> {
        crate::mind_actions::organization_status(self.actors, a, self.binding.actor)
            .map(|v| v as u32)
    }
    fn exhausted_reaction(&mut self, _: MindId, _: &mut Arena) -> aa_mind::Result<bool> {
        required("CAISleep51F9F0 actual World+D8/SAI reaction/rout owner required")
    }
    fn resolve(&mut self, g: u32) -> aa_mind::Result<Option<u32>> {
        let id = ActorId::from_raw(g);
        Ok(self.actors.key(id).map(|_| id.raw()))
    }
    fn object_removed(&mut self, o: u32) -> aa_mind::Result<bool> {
        Ok(self.bound.actor_flags(ActorId::from_raw(o), self.actors)? & 1 != 0)
    }
    fn organization_follows(&mut self, _: MindId, o: u32, _: &mut Arena) -> aa_mind::Result<bool> {
        let _ = key(self.actors, ActorId::from_raw(o))?;
        required("CAISleep actual Organization virtual164 follow target predicate required")
    }
    fn mind_virtual_188(&mut self, _: MindId, _: u32, _: &mut Arena) -> aa_mind::Result<()> {
        required("CAISleep actual Regiment virtual188 follow target required")
    }
    fn mind_virtual_190(&mut self, _: MindId, _: &mut Arena) -> aa_mind::Result<()> {
        required("CAISleep actual Regiment virtual190 missing target callback required")
    }
    fn follow_activate(&mut self, _: MindId, _: u32, _: &mut Arena) -> aa_mind::Result<()> {
        required("CAISleep51D920 actual targeted Kill activation required")
    }
    fn embedded_think(&mut self, _: MindId, offset: u32, _: &mut Arena) -> aa_mind::Result<()> {
        Err(error(format!(
            "CAISleep embedded+{offset:X} Think required"
        )))
    }
    fn organization_4d7960(&mut self, _: MindId, _: &mut Arena) -> aa_mind::Result<()> {
        required("CAISleep4D7960 actual home loss callback required")
    }
    fn home_component(&mut self, o: u32) -> aa_mind::Result<Option<u32>> {
        let id = ActorId::from_raw(o);
        Ok(self
            .actors
            .component(id, Slot::Denizen)
            .map_err(error)?
            .map(|_| o))
    }
    fn home_settle(&mut self, _: u32, _: u32, _: u32) -> aa_mind::Result<()> {
        required("CAISleep56AEC0 actual Denizen settlement callback required")
    }
    fn mind_virtual_f4(&mut self, _: MindId, _: &mut Arena) -> aa_mind::Result<()> {
        required("CAISleep actual Regiment virtualF4 home callback required")
    }
    fn take_arrival(&mut self, _: MindId) -> aa_mind::Result<Option<u32>> {
        required("CAISleep actual owner uses native peek/resolve/remove order")
    }
    fn peek_arrival(&mut self, m: MindId, a: &Arena) -> aa_mind::Result<Option<u32>> {
        crate::stored_cai_arrivals::front(self.actors, a, self.binding.actor, m)
    }
    fn remove_arrival(&mut self, m: MindId, a: &Arena) -> aa_mind::Result<()> {
        crate::stored_cai_arrivals::pop_front(self.actors, a, self.binding.actor, m).map(|_| ())
    }
    fn member_organization(&mut self, o: u32) -> aa_mind::Result<u32> {
        let id = ActorId::from_raw(o);
        match self.actors.component(id, Slot::Element).map_err(error)? {
            Some(RuntimeComponent::Element(e)) if e.header.owner == id => {
                Ok(e.organization.map_or(0, ActorId::raw))
            }
            _ => required("CAISleep actual arrived member Element absent"),
        }
    }
    fn organization(&mut self, m: MindId, a: &Arena) -> aa_mind::Result<u32> {
        let owner = a.mind(m)?.actor;
        self.current()?;
        if owner != self.binding.actor.raw() {
            return required("CAISleep current Mind owner differs");
        }
        Ok(owner)
    }
    fn member_arrived(&mut self, m: MindId, o: u32, a: &mut Arena) -> aa_mind::Result<()> {
        self.arrived(m, o, 0, a)
    }
    fn sleep_point(&mut self, _: MindId) -> aa_mind::Result<[u32; 2]> {
        Ok([self.read(0x2b4)?, self.read(0x2b8)?])
    }
    fn set_sleep_point(&mut self, _: MindId, p: [u32; 2]) -> aa_mind::Result<()> {
        self.write(0x2b4, p[0])?;
        self.write(0x2b8, p[1])
    }
    fn null_point(&mut self) -> aa_mind::Result<[u32; 2]> {
        Ok(crate::actor_world::NATIVE_ZERO_VECTOR_B279C0.map(|v| v.0))
    }
    fn organization_move(&mut self, _: MindId, _: [u32; 2], _: &mut Arena) -> aa_mind::Result<()> {
        required("CAISleep57BF30 actual4EEBE0 relocation and57BEF0 current offsets required")
    }
    fn organization_busy(&mut self, _: StateId, a: &mut Arena) -> aa_mind::Result<bool> {
        let mut i = 0;
        while let Some((_, member)) = self.next_member(&mut i, false)? {
            let Some(RuntimeComponent::Mind(m)) =
                self.actors.component(member, Slot::Mind).map_err(error)?
            else {
                continue;
            };
            if a.mind(m.mind_id)?.actor != member.raw() {
                return required("520120 member Mind/Arena differs");
            }
            if aa_mind::cai_sleep::fields::member_busy(m.kind, || {
                let s = *m
                    .state_bindings
                    .get(&0x48)
                    .ok_or_else(|| error("535E70 actual embedded Deploy state absent"))?;
                Ok(a.state(s)?.status)
            })? {
                return Ok(true);
            }
        }
        Ok(false)
    }
    fn resume_pending(&mut self, m: MindId, a: &mut Arena) -> aa_mind::Result<bool> {
        self.with_world(a, |w, a| w.resume_pending_commands(m, a))
    }
    fn organization_flag310(&mut self, _: MindId, _: &Arena) -> aa_mind::Result<bool> {
        let source_pc = self
            .bound
            .bank
            .combat_source()
            .map_err(error)?
            .source_precision();
        let object = self
            .actors
            .object(key(self.actors, self.binding.actor)?)
            .map_err(error)?;
        let layout = self.bound.catalog.layout_data(source_pc).map_err(error)?;
        let source = layout
            .organization_component(
                self.bound.catalog.actors,
                &object.definition.proof.id,
                source_pc,
            )
            .map_err(error)?
            .ok_or_else(|| error("CAISleep current OrganizationData source absent"))?;
        Ok(source.rampages)
    }
    fn embedded_state(&mut self, _: MindId, o: u32) -> aa_mind::Result<StateId> {
        self.current()?
            .state_bindings
            .get(&o)
            .copied()
            .ok_or_else(|| error("CAISleep actual embedded state absent"))
    }
    fn organization_word74(&mut self, _: MindId, _: &Arena) -> aa_mind::Result<u32> {
        self.organization_fields()?
            .words
            .get(&0x74)
            .copied()
            .ok_or_else(|| error("CAISleep actual Organization+74 unwritten"))
    }
    fn sleep_word1c(&mut self, _: StateId) -> aa_mind::Result<u32> {
        self.read(0x50)
    }
    fn set_sleep_word1c(&mut self, _: StateId, v: u32) -> aa_mind::Result<()> {
        self.write(0x50, v)
    }
    fn sleep_byte10(&mut self, _: StateId) -> aa_mind::Result<bool> {
        Ok(self.byte(0x44)? != 0)
    }
    fn sleep_522f60(&mut self, _: StateId, a: &mut Arena) -> aa_mind::Result<()> {
        aa_mind::cai_sleep::fields::refresh_members(&mut Refresh {
            sleep: self,
            arena: a,
        })
    }
    fn sleep_523000(&mut self, _: StateId, a: &mut Arena) -> aa_mind::Result<()> {
        aa_mind::cai_sleep::fields::retry_refresh(&mut Refresh {
            sleep: self,
            arena: a,
        })
    }
}
pub(super) fn think(
    bound: &mut Bound<'_, '_>,
    context: &mut Context<'_, '_>,
    state: StateId,
) -> aa_mind::Result<bool> {
    let binding = mind_world::state_binding(context.actors, context.arena, state)?;
    if binding.kind != Kind::Regiment || binding.offset != 0x34 {
        return required("CAISleep Think requires actual Regiment+34 allocation");
    }
    let mut world = Sleep {
        bound,
        actors: context.actors,
        runtime: context.runtime,
        queues: context.queues,
        owners: context.owners,
        binding,
    };
    aa_mind::cai_sleep::think(state, context.arena, &mut world)
}

struct Refresh<'call, 'a, 'b, 'store, 'source> {
    sleep: &'call mut Sleep<'a, 'b, 'store, 'source>,
    arena: &'call mut Arena,
}
impl aa_mind::cai_sleep::fields::RefreshFields for Refresh<'_, '_, '_, '_, '_> {
    type Member = ActorId;
    fn retry(&mut self) -> aa_mind::Result<u32> {
        self.sleep.read(0x48)
    }
    fn store_retry(&mut self, v: u32) -> aa_mind::Result<()> {
        self.sleep.write(0x48, v)
    }
    fn next_attached(&mut self, index: &mut usize) -> aa_mind::Result<Option<ActorId>> {
        self.sleep
            .next_member(index, true)
            .map(|v| v.map(|(_, m)| m))
    }
    fn mode(&mut self) -> aa_mind::Result<u32> {
        self.sleep.read(0x28c)
    }
    fn primary(&mut self, member: ActorId) -> aa_mind::Result<bool> {
        let Some(RuntimeComponent::Element(e)) = self
            .sleep
            .actors
            .component(member, Slot::Element)
            .map_err(error)?
        else {
            return required("570650 actual current member Element absent");
        };
        if let Some(owner) = e.organization {
            match self
                .sleep
                .actors
                .component(owner, Slot::Organization)
                .map_err(error)?
            {
                Some(RuntimeComponent::Organization(org)) if org.header.owner == owner => Ok(org
                    .words
                    .get(&0x44)
                    .copied()
                    .ok_or_else(|| error("570650 actual Organization primary+44 unwritten"))?
                    == member.raw()),
                _ => required("570650 current Element Organization absent"),
            }
        } else {
            Ok(false)
        }
    }
    fn flag(&mut self) -> aa_mind::Result<u8> {
        self.sleep.byte(0x4c)
    }
    fn dispatch(&mut self, m: ActorId, f: u8) -> aa_mind::Result<()> {
        self.sleep
            .arrived(self.sleep.binding.mind, m.raw(), f, self.arena)
    }
    fn store_pending(&mut self, v: u8) -> aa_mind::Result<()> {
        self.sleep.write_byte(0x44, v)
    }
}
impl aa_mind::cai_sleep::fields::RetryFields for Refresh<'_, '_, '_, '_, '_> {
    fn any_active(&mut self) -> aa_mind::Result<bool> {
        let mut i = 0;
        while let Some((_, m)) = self.sleep.next_member(&mut i, true)? {
            if crate::stored_member_command::member_active_on_owners(self.sleep.actors, m)? {
                return Ok(true);
            }
        }
        Ok(false)
    }
    fn all_at_slots(&mut self) -> aa_mind::Result<bool> {
        self.sleep.all_at_slots()
    }
    fn retry(&mut self) -> aa_mind::Result<u32> {
        self.sleep.read(0x48)
    }
    fn store_pending(&mut self, v: u8) -> aa_mind::Result<()> {
        self.sleep.write_byte(0x44, v)
    }
    fn refresh(&mut self) -> aa_mind::Result<()> {
        aa_mind::cai_sleep::fields::refresh_members(self)
    }
}
