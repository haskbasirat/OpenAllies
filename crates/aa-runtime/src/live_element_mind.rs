//! Element Mind (UAI) queue4 Update on the live retained World.
//!
//! The Mind+8 Updatable of an Element Mind (vtable A357E8) has +1C = 531D30,
//! ported as aa_mind::element_update::update. Its base 529D00 and the current
//! state's Think run on the SAME Installed payload, Arena, queues and owners
//! as the Regiment path (stored_mind_update). The Mind primitives (enter,
//! leave, schedule 52DC10, 52BF30 resume) are the shared mind_world::World over
//! stored_mind_init_world::Bound services.
//!
//! Bound Thinks: Sleep (Mind+68, 52F950, aa_mind::uai_thinks::uai_sleep).
//! Reached leaves without a live binding return explicit errors at their call.
use crate::{
    actor_world::{ActiveComponent, ActorAccess, ObjectKey, RuntimeComponent, Source, SpecialSource},
    catalog::Catalog,
    mind_world,
    scheduler_owners::{Owners, Target},
    stored_mind_init_world::Bound,
};
use aa_actors::{ActorId, components::ComponentSlot as Slot};
use aa_mind::{
    Arena, MindId, StateId,
    construct::{Installed, Kind},
    element_update,
    regiment_update::BaseWorld,
    uai_thinks::{ThinkWorld, uai_sleep::SleepThinkWorld},
};
use aa_numeric::Precision;
use aa_world::queues::{Delivery, Queues};

fn error(value: impl std::fmt::Display) -> aa_mind::Error {
    aa_mind::Error::World(format!("live Element Mind: {value}"))
}
fn required<T>(leaf: &str) -> aa_mind::Result<T> {
    Err(error(format!("required native {leaf} is not bound")))
}

/// Shared field access over the same split ActorStore loan.
struct Fields<'a, 'b, 'store, 'source> {
    bound: &'a mut Bound<'b, 'source>,
    actors: &'a mut ActorAccess<'store>,
    runtime: &'a mut mind_world::Runtime,
    queues: &'a mut Queues,
    owners: &'a mut Owners,
    object: ObjectKey,
    mind: MindId,
}
impl Fields<'_, '_, '_, '_> {
    fn installed(&self) -> aa_mind::Result<&Installed> {
        match self.actors.object(self.object).map_err(error)?.components().map_err(error)?.get(Slot::Mind) {
            Some(RuntimeComponent::Mind(m)) if m.kind == Kind::Element && m.mind_id == self.mind => Ok(m),
            _ => required("captured Element Mind allocation replaced"),
        }
    }
    fn installed_mut(&mut self) -> aa_mind::Result<&mut Installed> {
        let mind = self.mind;
        match self
            .actors
            .object_mut(self.object)
            .map_err(error)?
            .components_mut()
            .map_err(error)?
            .get_mut(Slot::Mind)
        {
            Some(RuntimeComponent::Mind(m)) if m.kind == Kind::Element && m.mind_id == mind => Ok(m),
            _ => required("captured Element Mind allocation replaced"),
        }
    }
    fn mind_word(&self, offset: u32) -> aa_mind::Result<u32> {
        self.installed()?
            .words
            .get(&offset)
            .copied()
            .ok_or_else(|| error(format!("Mind+{offset:X} is uninitialized")))
    }
    fn key(&self, actor: ActorId) -> aa_mind::Result<ObjectKey> {
        self.actors
            .key(actor)
            .ok_or_else(|| error(format!("{:#x} has no live GTID", actor.raw())))
    }
    fn actor_word(&self, actor: ActorId, offset: u32) -> aa_mind::Result<u32> {
        self.actors
            .object(self.key(actor)?)
            .map_err(error)?
            .actor
            .as_ref()
            .and_then(|a| a.words.get(&offset))
            .copied()
            .ok_or_else(|| error(format!("Actor+{offset:X} is uninitialized")))
    }
    fn state_low(&self, actor: ActorId) -> aa_mind::Result<u32> {
        Ok(self.actors.object(self.key(actor)?).map_err(error)?.components().map_err(error)?.state.low)
    }
    fn state_high(&self, actor: ActorId) -> aa_mind::Result<u32> {
        Ok(self.actors.object(self.key(actor)?).map_err(error)?.components().map_err(error)?.state.high)
    }
    fn flags8(&self, actor: ActorId) -> aa_mind::Result<u32> {
        Ok(self
            .actors
            .placement(self.key(actor)?, &*self.bound.spatial)
            .map_err(error)?
            .flags)
    }
    fn component(&self, actor: ActorId, slot: Slot) -> aa_mind::Result<Option<&RuntimeComponent>> {
        self.actors.component(actor, slot).map_err(error)
    }
    fn owner_of(&self, actor: ActorId, slot: Slot) -> aa_mind::Result<aa_world::queues::Owner> {
        self.owners
            .get(Target::Component { object: self.key(actor)?, slot: slot as u8 })
            .map_err(error)
    }
    fn element(&self, actor: ActorId) -> aa_mind::Result<Option<&aa_actors::ground_states::ElementState<String>>> {
        match self.component(actor, Slot::Element)? {
            None => Ok(None),
            Some(RuntimeComponent::Element(e)) if e.header.owner == actor => Ok(Some(e)),
            Some(_) => required("Element allocation/owner differs"),
        }
    }
    fn with_world<T>(
        &mut self,
        arena: &mut Arena,
        f: impl FnOnce(&mut mind_world::World<'_, '_, Bound<'_, '_>>, &mut Arena) -> aa_mind::Result<T>,
    ) -> aa_mind::Result<T> {
        let mut w = mind_world::World {
            actors: self.actors.reborrow(),
            runtime: &mut *self.runtime,
            queues: &mut *self.queues,
            owners: &mut *self.owners,
            services: &mut *self.bound,
        };
        f(&mut w, arena)
    }
}

/// 531D30 world, holding the Arena beside the field loans.
struct Update<'a, 'b, 'store, 'source> {
    fields: Fields<'a, 'b, 'store, 'source>,
    arena: &'a mut Arena,
}
impl BaseWorld for Update<'_, '_, '_, '_> {
    type Actor = ActorId;
    type Kingdom = ActorId;
    fn actor(&mut self, mind: MindId) -> aa_mind::Result<ActorId> {
        if mind != self.fields.mind {
            return required("Element Update callback changed its Updatable allocation");
        }
        self.fields.installed()?;
        Ok(ActorId::from_raw(self.arena.mind(mind)?.actor))
    }
    fn actor_has_state10(&mut self, actor: ActorId) -> aa_mind::Result<bool> {
        Ok(self.fields.state_low(actor)? & (1 << 10) != 0)
    }
    fn current_owner108(&mut self, actor: ActorId) -> aa_mind::Result<Option<ActorId>> {
        let raw = self.fields.actor_word(actor, 0xe8)?;
        Ok((raw != 0).then_some(ActorId::from_raw(raw)))
    }
    fn kingdom_byte239(&mut self, owner: ActorId) -> aa_mind::Result<u8> {
        let key = self.fields.key(owner)?;
        let Source::Special(s) = &self.fields.actors.entity(key).map_err(error)?.source else {
            return required("derived Kingdom storage");
        };
        if !matches!(s.source, SpecialSource::Kingdom(_)) {
            return required("Actor+E8 is not a Kingdom");
        }
        crate::trigger_world::shell_flag_239(s.derived.as_ref().ok_or_else(|| error("Kingdom derived owner absent"))?)
            .map_err(error)
    }
    fn attached_to_organization(&mut self, actor: ActorId) -> aa_mind::Result<bool> {
        Ok(self.fields.element(actor)?.is_some_and(|e| e.organization.is_some()))
    }
    fn current_home_4d11a0(&mut self, actor: ActorId) -> aa_mind::Result<bool> {
        let core = self.arena.mind(self.fields.mind)?;
        if core.actor != actor.raw() {
            return required("4D11A0 same Mind/core Actor differs");
        }
        Ok(self.fields.actors.key(ActorId::from_raw(core.words_24_30[1])).is_some())
    }
    fn mind_virtual1a8(&mut self, _: MindId) -> aa_mind::Result<()> {
        required("Element Mind virtual +1A8 (dissolving-kingdom reset)")
    }
    fn timed_list_4d37e0(&mut self, actor: ActorId) -> aa_mind::Result<()> {
        if self.fields.actor_word(actor, 0x130)? == 0 {
            Ok(())
        } else {
            required("4D37E0 positive timed reciprocal actor list")
        }
    }
}
impl element_update::World for Update<'_, '_, '_, '_> {
    type State = StateId;
    fn actor_flags8(&mut self, actor: ActorId) -> aa_mind::Result<u32> {
        self.fields.flags8(actor)
    }
    fn actor_state_high(&mut self, actor: ActorId) -> aa_mind::Result<u32> {
        self.fields.state_high(actor)
    }
    fn expire_bad_targets(&mut self, _: MindId) -> aa_mind::Result<()> {
        //Mind+1B0 head; the SVar is read only for a non-empty list.
        if self.fields.mind_word(0x1b0)? == 0 {
            Ok(())
        } else {
            required("531DBD Mind+1B0 bad-target nodes (52F370, UAIBadTargetExpirationSeconds)")
        }
    }
    fn current_state(&mut self, mind: MindId) -> aa_mind::Result<StateId> {
        let state = self
            .arena
            .mind(mind)?
            .stack
            .front()
            .copied()
            .flatten()
            .ok_or_else(|| error("529F70 native null current-state Think dereference"))?;
        if self.arena.state(state)?.parent != mind {
            return required("529F70 current state parent differs");
        }
        Ok(state)
    }
    fn think(&mut self, state: StateId) -> aa_mind::Result<()> {
        let offset = self
            .fields
            .installed()?
            .state_bindings
            .iter()
            .find_map(|(&o, &id)| (id == state).then_some(o))
            .ok_or_else(|| error("current state is absent from its Element Mind"))?;
        match offset {
            aa_mind::uai_thinks::element::SLEEP => {
                let pc = self.fields.bound.pc;
                let mut sleep = Sleep { fields: &mut self.fields };
                aa_mind::uai_thinks::uai_sleep::think(state, self.arena, &mut sleep, pc).map(|_| ())
            }
            other => required(&format!("Element Mind+{other:X} state Think")),
        }
    }
    fn mover_active_unlinked(&mut self, actor: ActorId) -> aa_mind::Result<bool> {
        if self.fields.component(actor, Slot::Mover)?.is_none() {
            return Ok(false);
        }
        let key = self.fields.key(actor)?;
        let active = match self.fields.actors.object(key).map_err(error)?.active_component {
            ActiveComponent::Slot(s) => s == Slot::Mover,
            ActiveComponent::None => false,
            ActiveComponent::UnresolvedNativeWord(_) => return required("unresolved Actor+E4"),
        };
        if !active {
            return Ok(false);
        }
        let owner = self.fields.owner_of(actor, Slot::Mover)?;
        Ok(self.fields.queues.owner_head(owner).is_none())
    }
    fn mind_word88(&mut self, _: MindId) -> aa_mind::Result<u32> {
        self.fields.mind_word(0x88)
    }
    fn deactivate_4cfc80(&mut self, _: ActorId) -> aa_mind::Result<()> {
        required("4CFC80 active Mover deactivation after Think")
    }
    fn updatable_linked(&mut self, _: MindId) -> aa_mind::Result<bool> {
        let actor = ActorId::from_raw(self.arena.mind(self.fields.mind)?.actor);
        let owner = self.fields.owner_of(actor, Slot::Mind)?;
        Ok(self.fields.queues.owner_head(owner).is_some())
    }
    fn active_component_linked(&mut self, actor: ActorId) -> aa_mind::Result<bool> {
        let key = self.fields.key(actor)?;
        match self.fields.actors.object(key).map_err(error)?.active_component {
            ActiveComponent::None => Ok(false),
            ActiveComponent::Slot(Slot::Character) => Ok(false),
            ActiveComponent::Slot(s) => {
                let owner = self.fields.owner_of(actor, s)?;
                Ok(self.fields.queues.owner_head(owner).is_some())
            }
            ActiveComponent::UnresolvedNativeWord(_) => required("unresolved Actor+E4"),
        }
    }
    fn transport_blocks(&mut self, _: MindId, actor: ActorId) -> aa_mind::Result<bool> {
        if self.fields.component(actor, Slot::Transport)?.is_some() {
            return required("531F1E Transport+38 / Join(Mind+164) reschedule gate");
        }
        Ok(false)
    }
    fn reschedule(&mut self, mind: MindId, delay_bits: u32) -> aa_mind::Result<()> {
        self.fields.with_world(self.arena, |w, arena| w.schedule(mind, delay_bits, 0, arena))
    }
    fn summoned(&mut self, actor: ActorId) -> aa_mind::Result<bool> {
        match self.fields.component(actor, Slot::Properties)? {
            None => Ok(false),
            Some(RuntimeComponent::Properties(p)) => Ok(p.modifier.flags & 0x10 != 0),
            Some(_) => required("Properties slot payload"),
        }
    }
    fn in_combat(&mut self, _: MindId) -> aa_mind::Result<bool> {
        required("52D730 Mind virtual +94 in-combat predicate")
    }
    fn summoned_dies(&mut self, _: ActorId) -> aa_mind::Result<()> {
        required("4D7960 summoned actor death")
    }
}

/// 52F950 world over the same loans (the Arena is the Think's own argument).
struct Sleep<'c, 'a, 'b, 'store, 'source> {
    fields: &'c mut Fields<'a, 'b, 'store, 'source>,
}
impl Sleep<'_, '_, '_, '_, '_> {
    fn mind_actor(&self, arena_actor: Option<u32>) -> aa_mind::Result<ActorId> {
        let _ = arena_actor;
        let key = self.fields.object;
        self.fields.actors.id(key).ok_or_else(|| error("Element Mind actor has no live GTID"))
    }
}
impl aa_mind::World for Sleep<'_, '_, '_, '_, '_> {
    fn can_activate(&mut self, s: StateId, a: &mut Arena) -> aa_mind::Result<bool> {
        self.fields.with_world(a, |w, a| w.can_activate(s, a))
    }
    fn enter(&mut self, s: StateId, a: &mut Arena) -> aa_mind::Result<()> {
        self.fields.with_world(a, |w, a| w.enter(s, a))
    }
    fn leave(&mut self, s: StateId, a: &mut Arena) -> aa_mind::Result<()> {
        self.fields.with_world(a, |w, a| w.leave(s, a))
    }
    fn schedule(&mut self, m: MindId, d: u32, data: u32, a: &mut Arena) -> aa_mind::Result<()> {
        self.fields.with_world(a, |w, a| w.schedule(m, d, data, a))
    }
    fn reset_default(&mut self, m: MindId, a: &mut Arena) -> aa_mind::Result<()> {
        self.fields.with_world(a, |w, a| w.reset_default(m, a))
    }
    fn state_changed(&mut self, m: MindId, a: &mut Arena) -> aa_mind::Result<()> {
        self.fields.with_world(a, |w, a| w.state_changed(m, a))
    }
}
impl ThinkWorld for Sleep<'_, '_, '_, '_, '_> {
    fn embedded_state(&mut self, _: MindId, offset: u32) -> aa_mind::Result<StateId> {
        self.fields
            .installed()?
            .state_bindings
            .get(&offset)
            .copied()
            .ok_or_else(|| error(format!("Element Mind+{offset:X} state absent")))
    }
    fn embedded_think(&mut self, _: MindId, offset: u32, _: &mut Arena) -> aa_mind::Result<()> {
        required(&format!("Element Mind+{offset:X} embedded Think"))
    }
    fn resolve(&mut self, gtid: u32) -> aa_mind::Result<Option<u32>> {
        Ok(self.fields.actors.key(ActorId::from_raw(gtid)).map(|_| gtid))
    }
    fn object_gtid(&mut self, object: u32) -> aa_mind::Result<u32> {
        Ok(object)
    }
    fn null_point(&mut self) -> aa_mind::Result<[u32; 2]> {
        let [x, y] = crate::actor_world::NATIVE_ZERO_VECTOR_B279C0;
        Ok([x.0, y.0])
    }
    fn mind_virtual_140(&mut self, _: MindId, _: u32, _: &mut Arena) -> aa_mind::Result<()> {
        required("Element Mind virtual +140 lost object")
    }
    fn current_performer(&mut self, _: MindId) -> aa_mind::Result<Option<u32>> {
        required("Actor+E4 current performer identity")
    }
    fn locomotion(&mut self, _: MindId) -> aa_mind::Result<Option<u32>> {
        required("Actor+6C locomotion identity")
    }
    fn locomotion_follow(&mut self, _: u32, _: u32, _: u32) -> aa_mind::Result<()> {
        required("576AD0 locomotion follow")
    }
}
impl SleepThinkWorld for Sleep<'_, '_, '_, '_, '_> {
    fn actor_word110(&mut self, _: MindId) -> aa_mind::Result<u32> {
        //Actor+110 overlap-list head; the typed list is its sole owner.
        let actor = self.mind_actor(None)?;
        let key = self.fields.key(actor)?;
        let shell = self
            .fields
            .actors
            .object(key)
            .map_err(error)?
            .actor
            .as_ref()
            .ok_or_else(|| error("Actor shell absent"))?;
        Ok(u32::from(shell.overlap_list_110.head().is_some()))
    }
    fn actor_flags100(&mut self, _: MindId) -> aa_mind::Result<u32> {
        let actor = self.mind_actor(None)?;
        self.fields.state_low(actor)
    }
    fn actor_pending_point(&mut self, _: MindId) -> aa_mind::Result<Option<[u32; 2]>> {
        required("4D1820 overlap escape point")
    }
    fn move_activate(&mut self, _: MindId, _: aa_navigation::Activation, _: &mut Arena) -> aa_mind::Result<bool> {
        required("52DF60 Element Move activation")
    }
    fn membership(&mut self, _: MindId) -> aa_mind::Result<Option<u32>> {
        let actor = self.mind_actor(None)?;
        Ok(self.fields.element(actor)?.map(|_| actor.raw()))
    }
    fn membership_organization(&mut self, membership: u32) -> aa_mind::Result<Option<u32>> {
        Ok(self
            .fields
            .element(ActorId::from_raw(membership))?
            .and_then(|e| e.organization)
            .map(ActorId::raw))
    }
    fn membership_detached(&mut self, membership: u32) -> aa_mind::Result<bool> {
        Ok(self
            .fields
            .element(ActorId::from_raw(membership))?
            .ok_or_else(|| error("membership Element absent"))?
            .flag_1c)
    }
    fn membership_index(&mut self, membership: u32) -> aa_mind::Result<u32> {
        Ok(self
            .fields
            .element(ActorId::from_raw(membership))?
            .ok_or_else(|| error("membership Element absent"))?
            .slot_index as u32)
    }
    fn mind_virtual_fc(&mut self, _: MindId, _: &mut Arena) -> aa_mind::Result<()> {
        required("52F160 Element Mind virtual +FC reattach")
    }
    fn actor_word_ac(&mut self, _: MindId) -> aa_mind::Result<u32> {
        let actor = self.mind_actor(None)?;
        Ok(u32::from(self.fields.component(actor, Slot::MobileDeployment)?.is_some()))
    }
    fn object_word_ac(&mut self, object: u32) -> aa_mind::Result<u32> {
        Ok(u32::from(self.fields.component(ActorId::from_raw(object), Slot::MobileDeployment)?.is_some()))
    }
    fn organization_leader(&mut self, organization: u32) -> aa_mind::Result<Option<u32>> {
        match self.fields.component(ActorId::from_raw(organization), Slot::Organization)? {
            Some(RuntimeComponent::Organization(o)) => {
                let leader = o.words.get(&0x44).copied().ok_or_else(|| error("Organization+44 unwritten"))?;
                Ok((leader != 0).then_some(leader))
            }
            _ => required("Element+10 Organization(+7C) absent"),
        }
    }
    fn formation_offset(&mut self, _: u32, _: u32) -> aa_mind::Result<[u32; 2]> {
        required("Organization+78 formation offset")
    }
    fn world_time(&mut self) -> aa_mind::Result<u32> {
        Ok(self.fields.bound.clock.begin.0)
    }
    fn sleep_time(&mut self, _: StateId) -> aa_mind::Result<u32> {
        self.fields.mind_word(0x68 + 0x10)
    }
    fn set_sleep_time(&mut self, _: StateId, bits: u32) -> aa_mind::Result<()> {
        self.fields.installed_mut()?.words.insert(0x68 + 0x10, bits);
        Ok(())
    }
    fn leader_4d1080(&mut self, _: u32) -> aa_mind::Result<bool> {
        required("4D1080 leader posture predicate")
    }
    fn leader_4d10a0(&mut self, _: u32) -> aa_mind::Result<bool> {
        required("4D10A0 leader posture predicate")
    }
    fn leader_4d0980(&mut self, _: u32) -> aa_mind::Result<bool> {
        required("4D0980 leader posture predicate")
    }
    fn actor_4d10a0(&mut self, _: MindId) -> aa_mind::Result<bool> {
        required("4D10A0 actor posture predicate")
    }
    fn definition_534_54(&mut self, _: MindId) -> aa_mind::Result<u32> {
        required("ActorData+534 +54 reach")
    }
    fn leader_virtual_0(&mut self, _: u32) -> aa_mind::Result<aa_numeric::Binary> {
        required("leader virtual +0")
    }
    fn normalize(&mut self, _: [u32; 2]) -> aa_mind::Result<[u32; 2]> {
        required("409230 normalize")
    }
    fn object_position(&mut self, object: u32) -> aa_mind::Result<[u32; 2]> {
        let key = self.fields.key(ActorId::from_raw(object))?;
        let p = self.fields.actors.placement(key, &*self.fields.bound.spatial).map_err(error)?;
        Ok([p.position_bits[0], p.position_bits[1]])
    }
    fn issue_action(&mut self, _: MindId, _: &aa_mind::cai_thinks::unit_order::StackAction, _: &mut Arena) -> aa_mind::Result<bool> {
        required("4D9E20/4D9B60 stack action delivery")
    }
    fn resume_pending(&mut self, mind: MindId, arena: &mut Arena) -> aa_mind::Result<bool> {
        self.fields.with_world(arena, |w, a| w.resume_pending_commands(mind, a))
    }
}

/// One queue4 delivery to an Element Mind (+1C = 531D30).
pub(crate) fn run(
    catalog: &Catalog<'_>,
    context: &mut crate::stored_mind_update::Context<'_>,
    event: Delivery,
) -> aa_world::queues::Result<()> {
    let mut go = || -> aa_mind::Result<()> {
        if event.queue != 4 || event.elapsed_bits.is_some() {
            return required("Element Mind +14/+18 are 4D1E10/943800; only queue4 reaches 531D30");
        }
        let target = context.owners.target(event.owner).map_err(error)?;
        let Target::Component { object, slot } = target else {
            return required("scheduled Mind target is not an actual component");
        };
        if slot != Slot::Mind as u8 {
            return required("scheduled Mind target has another component slot");
        }
        let actor = context.actors.id(object).ok_or_else(|| error("Mind Actor generation absent"))?;
        let (arena, mut actors) = context.actors.split_minds();
        let mind = match actors.component(actor, Slot::Mind).map_err(error)? {
            Some(RuntimeComponent::Mind(m)) if m.kind == Kind::Element => m.mind_id,
            _ => return required("scheduled Element Mind payload absent"),
        };
        if arena.mind(mind)?.actor != actor.raw() {
            return required("Element Mind Arena/Actor owner differs");
        }
        let mut bound = Bound {
            catalog,
            bank: &mut *context.bank,
            spatial: &mut *context.spatial,
            variables: &mut *context.variables,
            inventory: context.inventory,
            current_settings: context.current_settings,
            random: &mut *context.random,
            clock: context.clock,
            presentation: &mut *context.presentation,
            zones: context.zones,
            terrain: Some(context.terrain),
            mission: Some(context.mission),
            phase: Some(context.phase),
            pc: context.precision,
        };
        let mut update = Update {
            fields: Fields {
                bound: &mut bound,
                actors: &mut actors,
                runtime: &mut *context.runtime,
                queues: &mut *context.queues,
                owners: &mut *context.owners,
                object,
                mind,
            },
            arena,
        };
        element_update::update(&mut update, mind)
    };
    go().map_err(|e| aa_world::queues::Error::Callback(e.to_string()))
}

pub(crate) fn is_element(context: &crate::stored_mind_update::Context<'_>, object: ObjectKey) -> bool {
    matches!(
        context.actors.object(object).ok().and_then(|o| o.components().ok()).and_then(|c| c.get(Slot::Mind)),
        Some(RuntimeComponent::Mind(m)) if m.kind == Kind::Element
    )
}
#[allow(dead_code)]
fn _precision_marker(_: Precision) {}
