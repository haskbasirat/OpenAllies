//!529D00/524910 on the retained Regiment Mind's secondary Updatable.
//!The delivery borrows one Store/Arena split and the current World owners.
//!Positive unbound timed-list/Engage/derived virtuals remain reached errors.
use crate::{
    actor_world::{
        ActorAccess, ActorStore, ObjectKey, RuntimeComponent, RuntimeObject, Source, SpecialSource,
    },
    battle_terrain::BattleTerrain,
    catalog::Catalog,
    definitions::Bank,
    mind_world,
    scheduler_owners::{Owners, Target},
    spatial_world::SpatialWorld,
};
use aa_actors::{ActorId, components::ComponentSlot as Slot};
use aa_mind::{
    Arena, MindId, StateId,
    construct::{Installed, Kind},
    regiment_update as native,
};
use aa_numeric::Precision;
use aa_world::queues::{Delivery, Queues};

fn error(value: impl std::fmt::Display) -> aa_mind::Error {
    aa_mind::Error::World(value.to_string())
}
fn required<T>(value: &str) -> aa_mind::Result<T> {
    Err(error(value))
}

///Actual callback owners, lent only for this scheduler delivery. In particular,
///phase is the live World field after activation, not an inferred load phase.
pub(crate) struct Context<'a> {
    pub(crate) actors: &'a mut ActorStore,
    pub(crate) bank: &'a mut Bank,
    pub(crate) spatial: &'a mut SpatialWorld,
    pub(crate) variables: &'a mut crate::load_view_variables::State,
    pub(crate) presentation: &'a mut crate::fx_manager::State<crate::fx_graph::Command>,
    pub(crate) zones: &'a aa_zones::tile::ZoneMap,
    pub(crate) runtime: &'a mut mind_world::Runtime,
    pub(crate) queues: &'a mut Queues,
    pub(crate) owners: &'a mut Owners,
    pub(crate) random: &'a mut aa_simulation::NativeRng,
    pub(crate) clock: &'a aa_simulation::NativeClock,
    pub(crate) inventory: &'a aa_mapworld::WorldInventory,
    pub(crate) terrain: &'a BattleTerrain,
    pub(crate) mission: &'a crate::mission_fields::State,
    pub(crate) phase: &'a u32,
    pub(crate) current_settings: crate::stored_actor_state_effects::CurrentSettings<'a>,
    pub(crate) precision: Precision,
}

struct Bound<'a, 'store, 'source> {
    catalog: &'a Catalog<'source>,
    actors: ActorAccess<'store>,
    arena: &'a mut Arena,
    bank: &'a mut Bank,
    spatial: &'a mut SpatialWorld,
    variables: &'a mut crate::load_view_variables::State,
    presentation: &'a mut crate::fx_manager::State<crate::fx_graph::Command>,
    zones: &'a aa_zones::tile::ZoneMap,
    runtime: &'a mut mind_world::Runtime,
    queues: &'a mut Queues,
    owners: &'a mut Owners,
    random: &'a mut aa_simulation::NativeRng,
    clock: &'a aa_simulation::NativeClock,
    inventory: &'a aa_mapworld::WorldInventory,
    terrain: &'a BattleTerrain,
    mission: &'a crate::mission_fields::State,
    phase: &'a u32,
    current_settings: crate::stored_actor_state_effects::CurrentSettings<'a>,
    object: ObjectKey,
    mind: MindId,
    pc: Precision,
}
impl Bound<'_, '_, '_> {
    fn object(&self, actor: ActorId) -> aa_mind::Result<&RuntimeObject> {
        let key = self
            .actors
            .key(actor)
            .ok_or_else(|| error("Mind Update current Actor generation absent"))?;
        let object = self.actors.object(key).map_err(error)?;
        if object.definition.native_class != 3 || object.actor.is_none() || object.object.is_none()
        {
            return required("Mind Update actual ordinary Actor virtual dispatch required");
        }
        crate::live_object_init::Source::bind_in_bank(self.catalog, self.bank, &object.definition)
            .map_err(error)?;
        if !crate::load_spatial::same_proof(&object.thing.definition, &object.definition.proof) {
            return required("Mind Update same Actor/source identity differs");
        }
        Ok(object)
    }
    ///The scheduled subobject remains the captured allocation even if a native
    ///child changes its current Mind.actor field before the final flag read.
    fn installed(&self, mind: MindId) -> aa_mind::Result<&Installed> {
        if mind != self.mind {
            return required("Mind Update callback changed its Updatable allocation");
        }
        let object = self.actors.object(self.object).map_err(error)?;
        match object.components().map_err(error)?.get(Slot::Mind) {
            Some(RuntimeComponent::Mind(m)) if m.kind == Kind::Regiment && m.mind_id == mind => {
                Ok(m)
            }
            _ => required("Mind Update captured Regiment allocation replaced"),
        }
    }
    fn actor_word(&self, actor: ActorId, offset: u32) -> aa_mind::Result<u32> {
        self.object(actor)?
            .actor
            .as_ref()
            .unwrap()
            .words
            .get(&offset)
            .copied()
            .ok_or_else(|| error(format!("Mind Update Actor+{offset:X} is unbound")))
    }
    fn queue_owner(&self, mind: MindId) -> aa_mind::Result<aa_world::queues::Owner> {
        self.installed(mind)?;
        self.owners
            .get(Target::Component {
                object: self.object,
                slot: Slot::Mind as u8,
            })
            .map_err(error)
    }
}
impl native::BaseWorld for Bound<'_, '_, '_> {
    type Actor = ActorId;
    type Kingdom = ActorId;
    fn actor(&mut self, mind: MindId) -> aa_mind::Result<ActorId> {
        self.installed(mind)?;
        let actor = ActorId::from_raw(self.arena.mind(mind)?.actor);
        self.object(actor)?;
        Ok(actor)
    }
    fn actor_has_state10(&mut self, actor: ActorId) -> aa_mind::Result<bool> {
        Ok(self.object(actor)?.components().map_err(error)?.state.low & (1 << 10) != 0)
    }
    fn current_owner108(&mut self, actor: ActorId) -> aa_mind::Result<Option<ActorId>> {
        //Ordinary4D1E80 returns its current E8 pointer, represented by the
        //already-resolved portable Kingdom identity. No Element parent walk.
        let raw = self.actor_word(actor, 0xe8)?;
        if raw == 0 {
            return Ok(None);
        }
        let owner = ActorId::from_raw(raw);
        let key = self
            .actors
            .key(owner)
            .ok_or_else(|| error("Mind Update current E8 owner generation absent"))?;
        if self
            .actors
            .object(key)
            .map_err(error)?
            .definition
            .native_class
            != 4
        {
            return required("Mind Update actual owner108 is not Kingdom class4");
        }
        Ok(Some(owner))
    }
    fn kingdom_byte239(&mut self, owner: ActorId) -> aa_mind::Result<u8> {
        let key = self
            .actors
            .key(owner)
            .ok_or_else(|| error("Mind Update current Kingdom absent"))?;
        let Source::Special(s) = &self.actors.entity(key).map_err(error)?.source else {
            return required("Mind Update actual derived Kingdom storage required");
        };
        if !matches!(s.source, SpecialSource::Kingdom(_)) {
            return required("Mind Update source is not Kingdom");
        }
        crate::trigger_world::shell_flag_239(
            s.derived
                .as_ref()
                .ok_or_else(|| error("Mind Update Kingdom derived owner absent"))?,
        )
        .map_err(error)
    }
    fn attached_to_organization(&mut self, actor: ActorId) -> aa_mind::Result<bool> {
        self.object(actor)?;
        match self.actors.component(actor, Slot::Element).map_err(error)? {
            None => Ok(false),
            Some(RuntimeComponent::Element(e)) if e.header.owner == actor => {
                Ok(e.organization.is_some())
            }
            _ => required("Mind Update actual Element allocation/owner differs"),
        }
    }
    fn current_home_4d11a0(&mut self, actor: ActorId) -> aa_mind::Result<bool> {
        self.object(actor)?;
        let mind = match self.actors.component(actor, Slot::Mind).map_err(error)? {
            None => return Ok(false),
            Some(RuntimeComponent::Mind(m)) => m,
            _ => return required("4D11A0 actual typed Mind is absent"),
        };
        let core = self.arena.mind(mind.mind_id)?;
        if core.actor != actor.raw() {
            return required("4D11A0 same Mind/core Actor differs");
        }
        //Identity-only lookup; no target component fields are needed.
        Ok(self
            .actors
            .key(ActorId::from_raw(core.words_24_30[1]))
            .is_some())
    }
    fn mind_virtual1a8(&mut self, mind: MindId) -> aa_mind::Result<()> {
        self.installed(mind)?;
        required("522270 actual Regiment virtual1A8 member state clearing required")
    }
    fn timed_list_4d37e0(&mut self, actor: ActorId) -> aa_mind::Result<()> {
        if self.actor_word(actor, 0x130)? == 0 {
            Ok(())
        } else {
            required("4D37E0 actual positive timed reciprocal actor-list owner required")
        }
    }
}
impl native::World for Bound<'_, '_, '_> {
    type State = StateId;
    fn actor_flags(&mut self, actor: ActorId) -> aa_mind::Result<u32> {
        self.object(actor)?;
        let key = self
            .actors
            .key(actor)
            .ok_or_else(|| error("Mind Update flag Actor absent"))?;
        Ok(self
            .actors
            .placement(key, &*self.spatial)
            .map_err(error)?
            .flags)
    }
    fn member_count_57a730(&mut self, actor: ActorId) -> aa_mind::Result<u32> {
        self.object(actor)?;
        match self
            .actors
            .component(actor, Slot::Organization)
            .map_err(error)?
        {
            Some(RuntimeComponent::Organization(org)) if org.header.owner == actor => {
                u32::try_from(org.members.iter().filter(|m| !m.is_null()).count()).map_err(error)
            }
            _ => required("57A730 actual current Organization members required"),
        }
    }
    fn refresh_engage(&mut self, mind: MindId) -> aa_mind::Result<()> {
        let installed = self.installed(mind)?;
        let engage = *installed
            .state_bindings
            .get(&0x1cc)
            .ok_or_else(|| error("523EA0 actual embedded Engage absent"))?;
        if self.arena.state(engage)?.parent != mind {
            return required("523EA0 same Engage parent differs");
        }
        //The actual native predicate precedes both loops even when empty.
        let _ = aa_mind::is_current(engage, self.arena)?;
        let owner = self
            .actors
            .id(self.object)
            .ok_or_else(|| error("523EA0 captured owner absent"))?;
        crate::stored_cai_arrivals::ensure(&mut self.actors, self.arena, owner, mind)?;
        let targets = &self
            .actors
            .regiment_owned
            .get(&mind)
            .expect("same validated owner")
            .engage_targets;
        if targets.nodes.is_empty() {
            Ok(())
        } else {
            required("523EA0 actual positive Engage target/world services required")
        }
    }
    fn current_state(&mut self, mind: MindId) -> aa_mind::Result<StateId> {
        self.installed(mind)?;
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
        //Same derived coordinator, with the actual live frame capabilities.
        let _ = crate::stored_mind_init_world::think_with_fields(
            self.catalog,
            &mut self.actors,
            self.bank,
            crate::player_order_frame::Fields {
                spatial: self.spatial,
                variables: self.variables,
                presentation: self.presentation,
                zones: self.zones,
                random: self.random,
                clock: self.clock,
                inventory: self.inventory,
                terrain: Some(self.terrain),
                mission: Some(self.mission),
                phase: Some(self.phase),
                current_settings: self.current_settings,
            },
            self.runtime,
            state,
            self.arena,
            self.queues,
            self.owners,
            self.pc,
        )?;
        Ok(())
    }
    fn updatable_head_present(&mut self, mind: MindId) -> aa_mind::Result<bool> {
        Ok(self.queues.owner_head(self.queue_owner(mind)?).is_some())
    }
    fn schedule(&mut self, mind: MindId, delay_bits: u32, payload: u32) -> aa_mind::Result<()> {
        let owner = self.queue_owner(mind)?;
        //Regiment virtual4=50AD80, virtual10=529CD0 cancels first. The same
        //World+208 deferred queue is portable queue4 in mind_world::schedule.
        self.queues.cancel_owner(owner).map_err(error)?;
        self.queues
            .add_after(4, owner, payload, delay_bits)
            .map_err(error)?;
        Ok(())
    }
    fn clear_arrivals(&mut self, mind: MindId) -> aa_mind::Result<()> {
        self.installed(mind)?;
        //523EA0 established this same retained list before Think. Native
        //763B90 owns no Actor lookup: a changed current Mind.actor must not
        //turn its final list destruction into a foreign-owner refusal.
        let owned = self
            .actors
            .regiment_owned
            .get_mut(&mind)
            .ok_or_else(|| error("763B90 previously validated Regiment list owner absent"))?;
        while owned.tracked_words.pop_front().is_some() {}
        Ok(())
    }
}

pub(crate) fn run(
    catalog: &Catalog<'_>,
    context: &mut Context<'_>,
    event: Delivery,
) -> aa_world::queues::Result<()> {
    let mut run = || -> aa_mind::Result<()> {
        if context.queues.precision != context.precision
            || context.spatial.index().precision() != context.precision
        {
            return required("scheduled Mind live queue/spatial precision differs");
        }
        let target = context.owners.target(event.owner).map_err(error)?;
        let Target::Component { object, slot } = target else {
            return required("scheduled Mind target is not actual component");
        };
        if slot != Slot::Mind as u8 {
            return required("scheduled Mind target has another component slot");
        }
        if context.owners.get(target).map_err(error)? != event.owner {
            return required("scheduled Mind inverse owner differs");
        }
        let actor = context
            .actors
            .id(object)
            .ok_or_else(|| error("scheduled Mind current Actor generation absent"))?;
        let (arena, actors) = context.actors.split_minds();
        let installed = match actors.component(actor, Slot::Mind).map_err(error)? {
            Some(RuntimeComponent::Mind(m)) => m,
            _ => return required("scheduled Mind actual typed payload absent"),
        };
        let mind = installed.mind_id;
        if installed.kind != Kind::Regiment {
            return required("scheduled Element/Aircraft Update has no concrete derived backend");
        }
        if arena.mind(mind)?.actor != actor.raw() {
            return required("scheduled Mind Arena/Actor owner differs");
        }
        let mut bound = Bound {
            catalog,
            actors,
            arena,
            bank: context.bank,
            spatial: context.spatial,
            variables: context.variables,
            presentation: context.presentation,
            zones: context.zones,
            runtime: context.runtime,
            queues: context.queues,
            owners: context.owners,
            random: context.random,
            clock: context.clock,
            inventory: context.inventory,
            terrain: context.terrain,
            mission: context.mission,
            phase: context.phase,
            current_settings: context.current_settings,
            object,
            mind,
            pc: context.precision,
        };
        bound.object(actor)?;
        if crate::catalog::mind_kind(
            bound
                .object(actor)?
                .definition
                .component_mask
                .ok_or_else(|| error("scheduled Mind source mask absent"))?,
        ) != Some(Kind::Regiment)
        {
            return required("scheduled Mind source factory differs");
        }
        native::update(&mut bound, mind)
    };
    run().map_err(|e| aa_world::queues::Error::Callback(e.to_string()))
}
