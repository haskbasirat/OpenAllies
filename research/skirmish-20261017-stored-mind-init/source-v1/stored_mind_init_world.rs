//! Fresh component4 InitExisting over the canonical Store Arena/World runtime.
//! Native529EC0/51D1B0 reuse mind_world's existing coordinator and Element reset.
//! No saved LoadShell, copied Arena, replacement timer or temporary Runtime is
//! constructed. Positive children retain their actual reached error boundary.
use crate::{
    actor_world::{ActorAccess, ActorStore, ObjectKey, RuntimeComponent},
    catalog::Catalog,
    definitions::Bank,
    mind_world::{self, Context, Services, StateBinding},
    restoration::Staged,
    spatial_world::SpatialWorld,
};
use aa_actors::{ActorId, components::ComponentSlot as Slot, state};
use aa_mind::{
    Arena, MindId, StateId,
    construct::{Installed, Kind},
};
use aa_numeric::Precision;
use std::sync::Arc;

type Result<T> = std::result::Result<T, String>;
fn error(e: impl std::fmt::Display) -> aa_mind::Error {
    aa_mind::Error::World(e.to_string())
}
fn required<T>(s: &str) -> aa_mind::Result<T> {
    Err(error(s))
}

fn checked(
    catalog: &Catalog<'_>,
    actors: &ActorStore,
    bank: &Bank,
    key: ObjectKey,
) -> Result<(ActorId, MindId, Kind)> {
    catalog.require_source_owner()?;
    let source = bank.combat_source()?;
    source.completed_means()?;
    if source.source_identities() != catalog.actors.sources.as_slice() {
        return Err("fresh Mind moved source universe differs".into());
    }
    let owner = actors.id(key).ok_or("fresh Mind actual GTID absent")?;
    let entity = actors.entity(key).map_err(|e| e.to_string())?;
    let object = actors.object(key).map_err(|e| e.to_string())?;
    let binding = entity
        .bindings
        .as_ref()
        .ok_or("fresh Mind actual source allocation absent")?;
    let canonical =
        catalog.actor_bindings(&object.definition.proof.id, source.source_precision())?;
    if !Arc::ptr_eq(binding, &canonical)
        || binding.precision != source.source_precision()
        || object.definition.native_class != 3
        || object.actor.is_none()
        || object.definition.component_mask != binding.definition.component_mask
        || !crate::load_spatial::same_proof(&object.thing.definition, &object.definition.proof)
    {
        return Err("fresh Mind Actor/source/factory identity differs".into());
    }
    crate::live_object_init::Source::bind_in_bank(catalog, bank, &object.definition)
        .map_err(|e| e.to_string())?;
    let installed = match object
        .components()
        .map_err(|e| e.to_string())?
        .get(Slot::Mind)
    {
        Some(RuntimeComponent::Mind(m)) => m,
        _ => return Err("fresh Mind typed slot4 absent".into()),
    };
    let kind = crate::catalog::mind_kind(
        object
            .definition
            .component_mask
            .ok_or("fresh Mind source mask absent")?,
    )
    .ok_or("fresh Mind source factory has no concrete kind")?;
    if installed.kind != kind
        || actors
            .minds
            .mind(installed.mind_id)
            .map_err(|e| e.to_string())?
            .actor
            != owner.raw()
    {
        return Err("fresh Mind installed/core owner differs".into());
    }
    let mut states = std::collections::BTreeSet::new();
    for id in installed.state_bindings.values() {
        if !states.insert(*id)
            || actors.minds.state(*id).map_err(|e| e.to_string())?.parent != installed.mind_id
        {
            return Err("fresh Mind actual embedded allocation differs".into());
        }
    }
    if actors
        .minds
        .mind(installed.mind_id)
        .map_err(|e| e.to_string())?
        .stack
        .iter()
        .flatten()
        .any(|id| !states.contains(id))
    {
        return Err("fresh Mind current stack has foreign embedded state".into());
    }
    Ok((owner, installed.mind_id, kind))
}

///All concrete native Mind +14 hooks select4B2830's pure return.
pub fn after_init(
    catalog: &Catalog<'_>,
    actors: &ActorStore,
    bank: &Bank,
    key: ObjectKey,
) -> Result<()> {
    checked(catalog, actors, bank, key)?;
    Ok(())
}

pub fn init_existing(
    catalog: &Catalog<'_>,
    staged: &mut Staged,
    key: ObjectKey,
    pc: Precision,
) -> Result<bool> {
    let (_, id, kind) = checked(catalog, &staged.actors, &staged.definitions, key)?;
    if kind == Kind::Aircraft {
        return Err(
            "fresh Aircraft517C60 actual flight/queued initialization owner required".into(),
        );
    }
    if staged.actors.restored_commands.is_some() {
        return Err(
            "fresh Mind requires one completed saved-command ownership transfer before reset"
                .into(),
        );
    }
    let world = staged
        .load_world
        .as_mut()
        .ok_or("fresh Mind actual World absent")?;
    let crate::load_world::SpatialOwner::Adopted(spatial) = &mut world.spatial else {
        return Err("fresh Mind actual adopted spatial owner absent".into());
    };
    if spatial.error.is_some()
        || !spatial.pending.is_empty()
        || spatial.world.index().precision() != pc
        || staged.queues.precision != pc
    {
        return Err("fresh Mind actual spatial/queue live PC differs".into());
    }
    let (arena, actors) = staged.actors.split_minds();
    let mut services = Bound {
        catalog,
        bank: &mut staged.definitions,
        spatial: &mut spatial.world,
        variables: &world.variables,
        inventory: &staged.inventory,
        random: &mut staged.rng,
        clock: &staged.clock,
        pc,
    };
    let mut w = mind_world::World {
        actors,
        runtime: &mut world.mind_runtime,
        queues: &mut staged.queues,
        owners: &mut staged.owners,
        services: &mut services,
    };
    w.init_load(id, arena).map_err(|e| e.to_string())
}

struct Bound<'a, 'source> {
    catalog: &'a Catalog<'source>,
    bank: &'a mut Bank,
    spatial: &'a mut SpatialWorld,
    variables: &'a crate::load_view_variables::State,
    inventory: &'a aa_mapworld::WorldInventory,
    random: &'a mut aa_simulation::NativeRng,
    clock: &'a aa_simulation::NativeClock,
    pc: Precision,
}
fn installed<'a>(
    a: &'a ActorAccess<'_>,
    arena: &Arena,
    id: MindId,
) -> aa_mind::Result<&'a Installed> {
    let owner = ActorId::from_raw(arena.mind(id)?.actor);
    match a.component(owner, Slot::Mind).map_err(error)? {
        Some(RuntimeComponent::Mind(m)) if m.mind_id == id => Ok(m),
        _ => required("fresh Mind current component/core differs"),
    }
}
fn key(a: &ActorAccess<'_>, owner: ActorId) -> aa_mind::Result<ObjectKey> {
    a.key(owner)
        .ok_or_else(|| error("fresh Mind current Actor generation absent"))
}
fn word(a: &ActorAccess<'_>, owner: ActorId, offset: u32) -> aa_mind::Result<u32> {
    a.object(key(a, owner)?)
        .map_err(error)?
        .actor
        .as_ref()
        .and_then(|a| a.words.get(&offset))
        .copied()
        .ok_or_else(|| error(format!("fresh Mind Actor+{offset:X} is uninitialized")))
}
impl Bound<'_, '_> {
    fn with_world<T>(
        &mut self,
        c: &mut Context<'_, '_>,
        f: impl FnOnce(&mut mind_world::World<'_, '_, Self>, &mut Arena) -> aa_mind::Result<T>,
    ) -> aa_mind::Result<T> {
        let mut w = mind_world::World {
            actors: c.actors.reborrow(),
            runtime: &mut *c.runtime,
            queues: &mut *c.queues,
            owners: &mut *c.owners,
            services: self,
        };
        f(&mut w, c.arena)
    }
    fn activate(
        &mut self,
        c: &mut Context<'_, '_>,
        id: MindId,
        offset: u32,
    ) -> aa_mind::Result<bool> {
        let state = *installed(c.actors, c.arena, id)?
            .state_bindings
            .get(&offset)
            .ok_or_else(|| error("fresh Mind embedded state absent"))?;
        self.with_world(c, |w, arena| aa_mind::activate(state, 2, arena, w))
    }
    fn sync_regiment(&mut self, id: MindId, c: &mut Context<'_, '_>) -> aa_mind::Result<()> {
        let top = c
            .arena
            .mind(id)?
            .stack
            .front()
            .copied()
            .flatten()
            .ok_or(aa_mind::Error::MissingActiveState)?;
        let binding = mind_world::state_binding(c.actors, c.arena, top)?;
        let index = match binding.offset {
            0x34 | 0x270 => 12,
            0x54 => {
                let disable = *installed(c.actors, c.arena, id)?
                    .state_bindings
                    .get(&0x34)
                    .ok_or_else(|| error("fresh Regiment Disable state absent"))?;
                if c.arena.state(disable)?.status == 2 {
                    12
                } else {
                    17
                }
            }
            _ => {
                return required("fresh Regiment current state virtual20 requires its native leaf");
            }
        };
        self.set_actor_state(binding.actor, index, c)?;
        let object = c
            .actors
            .object(key(c.actors, binding.actor)?)
            .map_err(error)?;
        let components = object.components().map_err(error)?;
        let enabled = if components.present(Slot::Organization) && components.state.low & 0x10 == 0
        {
            word(c.actors, binding.actor, 0x118)? != 0
        } else {
            false
        };
        self.set_state(binding.actor, 40, enabled, c)
    }
    fn set_state(
        &mut self,
        actor: ActorId,
        index: u8,
        enabled: bool,
        c: &mut Context<'_, '_>,
    ) -> aa_mind::Result<()> {
        let state = c
            .actors
            .object(key(c.actors, actor)?)
            .map_err(error)?
            .components()
            .map_err(error)?
            .state;
        let flags = self.actor_flags(actor, c.actors)?;
        let mut fields = state::Fields {
            bits: u64::from(state.low) | (u64::from(state.high) << 32),
            thing_flags: flags,
        };
        let mut effects = StateEffects {
            owner: actor,
            services: self,
            context: c,
        };
        let result = state::set(&mut fields, index, enabled, &mut effects);
        effects.commit(&fields)?;
        result.map_err(error)
    }
}
struct StateEffects<'a, 'b, 'context, 'store, 'source> {
    owner: ActorId,
    services: &'a mut Bound<'b, 'source>,
    context: &'a mut Context<'context, 'store>,
}
impl StateEffects<'_, '_, '_, '_, '_> {
    fn commit(&mut self, f: &state::Fields) -> aa_mind::Result<()> {
        let k = key(self.context.actors, self.owner)?;
        let s = &mut self
            .context
            .actors
            .object_mut(k)
            .map_err(error)?
            .components_mut()
            .map_err(error)?
            .state;
        s.low = f.bits as u32;
        s.high = (f.bits >> 32) as u32;
        let old = self.services.actor_flags(self.owner, self.context.actors)?;
        if old != f.thing_flags {
            if let Ok(p) = self.context.actors.staged_placement_mut(k) {
                p.flags = f.thing_flags;
            } else if f.thing_flags == old | 0x1000 {
                self.services.spatial.mark_falling(k).map_err(error)?;
            } else {
                return required("fresh Mind state changed unsupported published flags");
            }
        }
        Ok(())
    }
    fn reread(&self, f: &mut state::Fields) -> aa_mind::Result<()> {
        let s = self
            .context
            .actors
            .object(key(self.context.actors, self.owner)?)
            .map_err(error)?
            .components()
            .map_err(error)?
            .state;
        f.bits = u64::from(s.low) | (u64::from(s.high) << 32);
        f.thing_flags = self.services.actor_flags(self.owner, self.context.actors)?;
        Ok(())
    }
}
impl state::Effects for StateEffects<'_, '_, '_, '_, '_> {
    fn enter(
        &mut self,
        f: &mut state::Fields,
        _: &'static state::Descriptor,
        _: Option<&'static state::Descriptor>,
    ) -> state::Result<()> {
        self.commit(f)
            .map_err(|e| state::Error::Callback(e.to_string()))?;
        Err(state::Error::Callback(
            "fresh4D4A90 actual state Property/Organization/stance/WhenEnter effects required"
                .into(),
        ))
    }
    fn leave(&mut self, f: &mut state::Fields, _: &'static state::Descriptor) -> state::Result<()> {
        self.commit(f)
            .map_err(|e| state::Error::Callback(e.to_string()))?;
        Err(state::Error::Callback(
            "fresh4D16B0 actual state leave effects required".into(),
        ))
    }
    fn rebuild_abilities(&mut self, f: &mut state::Fields) -> state::Result<()> {
        self.commit(f)
            .map_err(|e| state::Error::Callback(e.to_string()))?;
        let result = self.services.rebuild_actions(self.owner, self.context);
        self.reread(f)
            .map_err(|e| state::Error::Callback(e.to_string()))?;
        result.map_err(|e| state::Error::Callback(e.to_string()))
    }
}

impl Services for Bound<'_, '_> {
    fn can_activate_other(
        &mut self,
        _: StateId,
        b: StateBinding,
        c: &mut Context<'_, '_>,
    ) -> aa_mind::Result<bool> {
        if b.kind == Kind::Regiment && b.offset == 0x34 {
            let retreat = *installed(c.actors, c.arena, b.mind)?
                .state_bindings
                .get(&0x1f0)
                .ok_or_else(|| error("Regiment Retreat binding absent"))?;
            //51D650: Retreat is accepted directly;529F80 accepts an actually
            //empty/null head for fixed priority2 without reading metadata.
            return match c.arena.mind(b.mind)?.stack.front().copied().flatten() {
                None => Ok(true),
                Some(id) if id == retreat => Ok(true),
                Some(_) => required("51D650 current Regiment metadata priority binding required"),
            };
        }
        required("fresh derived Mind state activation predicate requires native owner")
    }
    fn enter_other(
        &mut self,
        id: StateId,
        b: StateBinding,
        c: &mut Context<'_, '_>,
    ) -> aa_mind::Result<()> {
        if b.kind == Kind::Regiment && b.offset == 0x34 {
            //51D0F0: Arena alone owns+C status; Installed alone owns payload.
            c.arena.state_mut(id)?.status = 2;
            let Some(RuntimeComponent::Mind(m)) =
                c.actors.component_mut(b.actor, Slot::Mind).map_err(error)?
            else {
                return required("Regiment actual Mind absent");
            };
            if m.mind_id != b.mind {
                return required("Regiment enter current Mind allocation differs");
            }
            m.words.insert(0x50, 0);
            m.bytes.insert(0x44, 0);
            m.bytes.insert(0x4c, 0);
            m.words.insert(0x48, 0);
            return Ok(());
        }
        if b.kind == Kind::Regiment && b.offset == 0x54 {
            c.arena.state_mut(id)?.status = 2;
            return Ok(());
        }
        required("fresh derived Mind state Enter requires its actual native effects")
    }
    fn leave_other(
        &mut self,
        id: StateId,
        b: StateBinding,
        c: &mut Context<'_, '_>,
    ) -> aa_mind::Result<()> {
        if b.kind == Kind::Regiment && matches!(b.offset, 0x34 | 0x54) {
            c.arena.state_mut(id)?.status = 0;
            return Ok(());
        }
        required("fresh derived Mind state Leave requires its actual native effects")
    }
    fn actor_state_other(
        &mut self,
        _: StateId,
        _: StateBinding,
        _: &mut Context<'_, '_>,
    ) -> aa_mind::Result<u8> {
        required("fresh active Element virtual20 source binding required")
    }
    fn set_actor_state(
        &mut self,
        a: ActorId,
        index: u8,
        c: &mut Context<'_, '_>,
    ) -> aa_mind::Result<()> {
        self.set_state(a, index, true, c)
    }
    fn rebuild_actions(&mut self, a: ActorId, c: &mut Context<'_, '_>) -> aa_mind::Result<()> {
        let key = key(c.actors, a)?;
        crate::stored_actor_actions::rebuild_on_owners(
            self.catalog,
            c.actors,
            c.arena,
            self.bank,
            key,
            self.inventory,
            &*self.spatial,
            Some(self.variables),
            self.pc,
        )
        .map_err(error)
    }
    fn default_stance(&mut self, a: ActorId, c: &mut Context<'_, '_>) -> aa_mind::Result<()> {
        let key = key(c.actors, a)?;
        crate::map_character_stance::default_stance_on_owners(
            self.catalog,
            c.actors,
            self.spatial,
            c.queues,
            c.owners,
            self.random,
            self.clock,
            &*self.bank,
            key,
            self.pc,
        )
        .map_err(error)
    }
    fn reset_other(
        &mut self,
        id: MindId,
        kind: Kind,
        c: &mut Context<'_, '_>,
    ) -> aa_mind::Result<()> {
        if kind != Kind::Regiment {
            return required("fresh non-Regiment derived default reset requires its native owner");
        }
        reset_regiment(&mut RegimentReset {
            bound: self,
            context: c,
            id,
        })
    }

    fn sync_state_other(
        &mut self,
        id: MindId,
        kind: Kind,
        c: &mut Context<'_, '_>,
    ) -> aa_mind::Result<()> {
        if kind == Kind::Regiment {
            self.sync_regiment(id, c)
        } else {
            required("fresh derived Mind state sync native owner required")
        }
    }
    fn facing(&self, a: ActorId, actors: &ActorAccess<'_>) -> aa_mind::Result<[u32; 2]> {
        let p = actors
            .placement(key(actors, a)?, &*self.spatial)
            .map_err(error)?;
        Ok([p.facing_bits[0], p.facing_bits[1]])
    }
    fn set_facing(
        &mut self,
        _: ActorId,
        _: [u32; 2],
        _: &mut Context<'_, '_>,
    ) -> aa_mind::Result<()> {
        required("fresh Mind facing callback requires same spatial transform effects")
    }
    fn actor_flags(&self, a: ActorId, actors: &ActorAccess<'_>) -> aa_mind::Result<u32> {
        Ok(actors
            .placement(key(actors, a)?, &*self.spatial)
            .map_err(error)?
            .flags)
    }
    fn has_live_group(&self, a: ActorId, actors: &ActorAccess<'_>) -> aa_mind::Result<bool> {
        Ok(actors
            .key(ActorId::from_raw(word(actors, a, 0x164)?))
            .is_some())
    }
    fn finish_group_commands(
        &mut self,
        _: ActorId,
        _: &mut Context<'_, '_>,
    ) -> aa_mind::Result<()> {
        required("fresh52BF30 positive Group4D1900/4DE7A0 owner required")
    }
    fn invoke_command(
        &mut self,
        _: MindId,
        _: u32,
        _: u32,
        _: &mut aa_mind::command::Context,
        _: &mut Context<'_, '_>,
    ) -> aa_mind::Result<bool> {
        required("fresh Mind positive queued command subtype dispatch required")
    }
    fn deliver_stop(&mut self, _: ActorId, _: &mut Context<'_, '_>) -> aa_mind::Result<bool> {
        required("fresh Mind positive rejected-command Stop delivery required")
    }
    fn defers_other(
        &mut self,
        _: StateId,
        _: StateBinding,
        _: &mut Context<'_, '_>,
    ) -> aa_mind::Result<bool> {
        required("fresh derived Mind rejected-command deferral predicate required")
    }
}

//One coordinator for51E3A0. Callback failures stop at their native child entry;
//the already-written shared guard remains set, exactly as the stopped prefix.
trait RegimentResetWorld {
    fn clear_current(&mut self) -> aa_mind::Result<()>;
    fn resetting(&mut self) -> bool;
    fn store_resetting(&mut self, value: bool);
    fn resume(&mut self) -> aa_mind::Result<bool>;
    fn home(&mut self) -> aa_mind::Result<Option<ActorId>>;
    fn home_removed(&mut self, home: ActorId) -> aa_mind::Result<bool>;
    fn join(&mut self, home: ActorId) -> aa_mind::Result<()>;
    fn denizen_add(&mut self, home: ActorId) -> aa_mind::Result<()>;
    fn rampages(&mut self) -> aa_mind::Result<bool>;
    fn activate(&mut self, offset: u32) -> aa_mind::Result<()>;
    fn schedule(&mut self) -> aa_mind::Result<()>;
}
fn reset_regiment(w: &mut impl RegimentResetWorld) -> aa_mind::Result<()> {
    w.clear_current()?;
    if w.resetting() {
        return Ok(());
    }
    w.store_resetting(true);
    if w.resume()? {
        w.store_resetting(false);
        return Ok(());
    }
    if let Some(home) = w.home()?
        && !w.home_removed(home)?
    {
        w.join(home)?;
        w.denizen_add(home)?;
        w.store_resetting(false);
        return Ok(());
    }
    if w.rampages()? {
        w.activate(0x54)?;
    } else {
        w.activate(0x34)?;
        w.schedule()?;
    }
    w.store_resetting(false);
    Ok(())
}
struct RegimentReset<'a, 'b, 'context, 'store, 'source> {
    bound: &'a mut Bound<'b, 'source>,
    context: &'a mut Context<'context, 'store>,
    id: MindId,
}
impl RegimentResetWorld for RegimentReset<'_, '_, '_, '_, '_> {
    fn clear_current(&mut self) -> aa_mind::Result<()> {
        let id = self.id;
        self.bound.with_world(self.context, |w, arena| {
            aa_mind::command::set_current(id, None, arena, w)
        })
    }
    fn resetting(&mut self) -> bool {
        self.context.runtime.regiment_resetting()
    }
    fn store_resetting(&mut self, value: bool) {
        self.context.runtime.set_regiment_resetting(value);
    }
    fn resume(&mut self) -> aa_mind::Result<bool> {
        let id = self.id;
        self.bound.with_world(self.context, |w, arena| {
            w.resume_pending_commands(id, arena)
        })
    }
    fn home(&mut self) -> aa_mind::Result<Option<ActorId>> {
        //4D11A0 re-reads Actor+70 after pending-command callbacks. A current
        //different Mind allocation must not read the original Mind's home.
        let owner = ActorId::from_raw(self.context.arena.mind(self.id)?.actor);
        let mind = match self
            .context
            .actors
            .component(owner, Slot::Mind)
            .map_err(error)?
        {
            None => return Ok(None),
            Some(RuntimeComponent::Mind(m)) => m.mind_id,
            Some(_) => return required("4D11A0 current typed Mind slot differs"),
        };
        let home = ActorId::from_raw(self.context.arena.mind(mind)?.words_24_30[1]);
        Ok(self.context.actors.key(home).map(|_| home))
    }
    fn home_removed(&mut self, home: ActorId) -> aa_mind::Result<bool> {
        Ok(self.bound.actor_flags(home, self.context.actors)? & 1 != 0)
    }
    fn join(&mut self, _: ActorId) -> aa_mind::Result<()> {
        required("51E3DC positive Join24C current home activation required")
    }
    fn denizen_add(&mut self, _: ActorId) -> aa_mind::Result<()> {
        required("51E3EF positive home Denizen56C210 owner required")
    }
    fn rampages(&mut self) -> aa_mind::Result<bool> {
        let owner = ActorId::from_raw(self.context.arena.mind(self.id)?.actor);
        let pc = self
            .bound
            .bank
            .combat_source()
            .map_err(error)?
            .source_precision();
        let object = self
            .context
            .actors
            .object(key(self.context.actors, owner)?)
            .map_err(error)?;
        let data = self.bound.catalog.layout_data(pc).map_err(error)?;
        let source = data
            .organization_component(self.bound.catalog.actors, &object.definition.proof.id, pc)
            .map_err(error)?
            .ok_or_else(|| error("51E40B actual OrganizationData source absent"))?;
        Ok(source.rampages)
    }
    fn activate(&mut self, offset: u32) -> aa_mind::Result<()> {
        let _ = self.bound.activate(self.context, self.id, offset)?;
        Ok(())
    }
    fn schedule(&mut self) -> aa_mind::Result<()> {
        let id = self.id;
        self.bound.with_world(self.context, |w, arena| {
            w.schedule(id, 0x3f19999a, 0, arena)
        })
    }
}

#[cfg(test)]
#[path = "stored_mind_init_world_tests.rs"]
mod tests;
