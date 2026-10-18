//! Actual538340 contextual Move binding after command authorization. ActorStore,
//! Mind Arena, Commands, typed Move payload and Queues remain the sole mutable
//! owners. Native activation/target/Performer effects are required services.
use crate::{
    actor_world::RuntimeComponent, mind_world, scheduler_owners::Target,
    spatial_world::SpatialWorld,
};
use aa_actors::{ActorId, components::ComponentSlot};
use aa_mind::{Error, MindId, Result, StateId, construct::Kind, element_command};
use aa_navigation::Activation;
use aa_numeric::Precision;

fn fail(value: impl std::fmt::Display) -> Error {
    Error::World(format!("live Element Move command: {value}"))
}

pub struct Context<'a, 'store> {
    pub mind: mind_world::Context<'a, 'store>,
    pub spatial: &'a mut SpatialWorld,
}
/// These leaves must execute against the supplied real context. They are not
/// acceptance predicates, recorder callbacks or permission to assign positions.
/// Unsupported reached services return errors, preserving preceding effects.
pub trait Services: mind_world::Services {
    /// Performer57E280 on the actual component of this owner.
    fn indirect_fire(
        &mut self,
        performer_owner: ActorId,
        context: &mut Context<'_, '_>,
    ) -> Result<bool>;
    /// Actual target virtual+16C, including its repeated invocation.
    fn target_accepts(
        &mut self,
        target: ActorId,
        actor: ActorId,
        context: &mut Context<'_, '_>,
    ) -> Result<bool>;
    fn activate_sleep(
        &mut self,
        state: StateId,
        mode: u32,
        context: &mut Context<'_, '_>,
    ) -> Result<bool>;
    fn activate_move(
        &mut self,
        state: StateId,
        request: Activation,
        context: &mut Context<'_, '_>,
    ) -> Result<bool>;
    fn activate_kill(
        &mut self,
        state: StateId,
        mode: u32,
        target: ActorId,
        context: &mut Context<'_, '_>,
    ) -> Result<bool>;
    fn activate_bombard(
        &mut self,
        state: StateId,
        mode: u32,
        target: ActorId,
        context: &mut Context<'_, '_>,
    ) -> Result<bool>;
    fn activate_follow(
        &mut self,
        state: StateId,
        mode: u32,
        target: ActorId,
        movement_mode: u32,
        speed_bits: u32,
        auxiliary_bits: [u32; 2],
        context: &mut Context<'_, '_>,
    ) -> Result<bool>;
}
pub struct World<'a, 'store, S> {
    pub context: Context<'a, 'store>,
    pub services: &'a mut S,
}
impl<S: Services> World<'_, '_, S> {
    pub fn execute(&mut self, mind: MindId, command: u32, precision: Precision) -> Result<bool> {
        element_command::move_command(mind, command, self, precision)
    }
    fn installed(&self, mind: MindId) -> Result<&aa_mind::construct::Installed> {
        let actor = ActorId::from_raw(self.context.mind.arena.mind(mind)?.actor);
        match self
            .context
            .mind
            .actors
            .component(actor, ComponentSlot::Mind)
            .map_err(fail)?
        {
            Some(RuntimeComponent::Mind(value))
                if value.mind_id == mind && value.kind == Kind::Element =>
            {
                Ok(value)
            }
            _ => Err(fail("required canonical Element Mind identity is absent")),
        }
    }
    fn state(&self, mind: MindId, offset: u32) -> Result<StateId> {
        let state = *self
            .installed(mind)?
            .state_bindings
            .get(&offset)
            .ok_or_else(|| fail(format!("embedded state{offset:#x} is absent")))?;
        let binding =
            mind_world::state_binding(self.context.mind.actors, self.context.mind.arena, state)?;
        if binding.mind != mind || binding.offset != offset || binding.kind != Kind::Element {
            return Err(fail("embedded state identity/parent mismatch"));
        }
        Ok(state)
    }
    fn validate_state(&self, state: StateId, offset: u32) -> Result<()> {
        let binding =
            mind_world::state_binding(self.context.mind.actors, self.context.mind.arena, state)?;
        if binding.kind != Kind::Element || binding.offset != offset {
            return Err(fail("activation has the wrong embedded state"));
        }
        Ok(())
    }
    fn actor_key(&self, actor: ActorId) -> Result<crate::actor_world::ObjectKey> {
        self.context
            .mind
            .actors
            .key(actor)
            .ok_or_else(|| fail("actor GTID is stale or null"))
    }
}
impl<S: Services> element_command::World for World<'_, '_, S> {
    fn command(&self, id: u32) -> Result<element_command::Command> {
        let value = self.context.mind.runtime.commands.value(id)?;
        Ok(element_command::Command {
            flags: value.flags,
            target: value.word10,
            position_bits: value.position_bits,
            auxiliary_bits: value.auxiliary_bits,
            speed_bits: value.scalar_bits,
        })
    }
    fn element(&self, _: MindId) -> Result<element_command::Element> {
        //538340 has no aggregate read. Returning a snapshot here would force
        //unreached payload/queue reads or fabricate unknown fields.
        Err(fail(
            "aggregate Element snapshot is not a native operation; granular reads are required",
        ))
    }
    fn element_actor(&self, mind: MindId) -> Result<u32> {
        self.installed(mind)?;
        Ok(self.context.mind.arena.mind(mind)?.actor)
    }
    fn move_state(&self, mind: MindId) -> Result<StateId> {
        self.state(mind, 0x7c)
    }
    fn sleep_state(&self, mind: MindId) -> Result<StateId> {
        self.state(mind, 0x68)
    }
    fn kill_state(&self, mind: MindId) -> Result<StateId> {
        self.state(mind, 0xf8)
    }
    fn bombard_state(&self, mind: MindId) -> Result<StateId> {
        self.state(mind, 0x10c)
    }
    fn move_destination(&self, mind: MindId) -> Result<[u32; 2]> {
        let state = self.state(mind, 0x7c)?;
        Ok(self
            .context
            .mind
            .runtime
            .movement
            .handle(state)?
            .read()?
            .plan
            .movement
            .destination_bits)
    }
    fn move_target(&self, mind: MindId) -> Result<u32> {
        let state = self.state(mind, 0x7c)?;
        Ok(self
            .context
            .mind
            .runtime
            .movement
            .handle(state)?
            .read()?
            .plan
            .movement
            .target_id)
    }
    fn update_pending(&self, mind: MindId) -> Result<bool> {
        let actor = ActorId::from_raw(self.element_actor(mind)?);
        let owner = self
            .context
            .mind
            .owners
            .get(Target::Component {
                object: self.actor_key(actor)?,
                slot: ComponentSlot::Mind as u8,
            })
            .map_err(fail)?;
        Ok(self.context.mind.queues.has_owner(owner))
    }
    fn actor(&self, _: u32) -> Result<element_command::Actor> {
        Err(fail(
            "aggregate Actor snapshot is not a native operation; granular reads are required",
        ))
    }
    fn actor_flags(&self, raw: u32) -> Result<u32> {
        let actor = ActorId::from_raw(raw);
        let placement = self
            .context
            .mind
            .actors
            .placement(self.actor_key(actor)?, self.context.spatial)
            .map_err(fail)?;
        Ok(placement.flags)
    }
    fn actor_position(&self, raw: u32) -> Result<[u32; 2]> {
        let actor = ActorId::from_raw(raw);
        let placement = self
            .context
            .mind
            .actors
            .placement(self.actor_key(actor)?, self.context.spatial)
            .map_err(fail)?;
        Ok([placement.position_bits[0], placement.position_bits[1]])
    }
    fn actor_performer(&self, raw: u32) -> Result<Option<u32>> {
        let actor = ActorId::from_raw(raw);
        Ok(
            match self
                .context
                .mind
                .actors
                .component(actor, ComponentSlot::Performer)
                .map_err(fail)?
            {
                None => None,
                Some(RuntimeComponent::Performer(p)) if p.state.owner == raw => Some(raw),
                _ => return Err(fail("Performer slot/owner identity mismatch")),
            },
        )
    }
    fn resolve(&self, raw: u32) -> Result<Option<u32>> {
        Ok(self
            .context
            .mind
            .actors
            .key(ActorId::from_raw(raw))
            .map(|_| raw))
    }
    fn active(&self, state: StateId) -> Result<bool> {
        let binding =
            mind_world::state_binding(self.context.mind.actors, self.context.mind.arena, state)?;
        Ok(self
            .context
            .mind
            .arena
            .mind(binding.mind)?
            .stack
            .front()
            .copied()
            .flatten()
            == Some(state))
    }
    fn schedule(&mut self, mind: MindId, delay_bits: u32, data: u32) -> Result<()> {
        self.installed(mind)?;
        mind_world::World {
            actors: self.context.mind.actors.reborrow(),
            runtime: self.context.mind.runtime,
            queues: self.context.mind.queues,
            owners: self.context.mind.owners,
            services: self.services,
        }
        .schedule(mind, delay_bits, data, self.context.mind.arena)
    }
    fn indirect_fire(&mut self, owner: u32) -> Result<bool> {
        let actor = ActorId::from_raw(owner);
        match self
            .context
            .mind
            .actors
            .component(actor, ComponentSlot::Performer)
            .map_err(fail)?
        {
            Some(RuntimeComponent::Performer(p)) if p.state.owner == owner => {}
            _ => return Err(fail("indirect-fire receiver is not the actual Performer")),
        }
        self.services.indirect_fire(actor, &mut self.context)
    }
    fn target_accepts(&mut self, target: u32, actor: u32) -> Result<bool> {
        self.services.target_accepts(
            ActorId::from_raw(target),
            ActorId::from_raw(actor),
            &mut self.context,
        )
    }
    fn activate_sleep(&mut self, state: StateId, mode: u32) -> Result<bool> {
        self.validate_state(state, 0x68)?;
        self.services.activate_sleep(state, mode, &mut self.context)
    }
    fn activate_move(&mut self, state: StateId, request: Activation) -> Result<bool> {
        self.validate_state(state, 0x7c)?;
        self.services
            .activate_move(state, request, &mut self.context)
    }
    fn activate_kill(&mut self, state: StateId, mode: u32, target: u32) -> Result<bool> {
        self.validate_state(state, 0xf8)?;
        self.services
            .activate_kill(state, mode, ActorId::from_raw(target), &mut self.context)
    }
    fn activate_bombard(&mut self, state: StateId, mode: u32, target: u32) -> Result<bool> {
        self.validate_state(state, 0x10c)?;
        self.services
            .activate_bombard(state, mode, ActorId::from_raw(target), &mut self.context)
    }
    fn activate_follow(
        &mut self,
        state: StateId,
        mode: u32,
        target: u32,
        movement_mode: u32,
        speed_bits: u32,
        auxiliary_bits: [u32; 2],
    ) -> Result<bool> {
        self.validate_state(state, 0x7c)?;
        self.services.activate_follow(
            state,
            mode,
            ActorId::from_raw(target),
            movement_mode,
            speed_bits,
            auxiliary_bits,
            &mut self.context,
        )
    }
}
