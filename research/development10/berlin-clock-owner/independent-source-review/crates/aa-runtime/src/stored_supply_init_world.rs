//!58B0E0→58AE10(0) on the SAME stored Supply, Zone0, queues and current Vars.
//!The saved and fresh adapters share native Supply kernels; no Record/pose clone.
use crate::load_supply_init_world::{InitWorld, PrimaryWorld, ProvideWorld, TickWorld};
use crate::{
    actor_world::{ActorStore, ObjectKey, RuntimeComponent, Source},
    catalog::Catalog,
    definitions::Bank,
    restoration::Staged,
};
use aa_actors::{
    ActorId,
    actor_creation::{Error, Result},
    components::ComponentSlot as Slot,
    state,
    supply_state::SupplyState,
};
use aa_numeric::Precision;
fn error(e: impl std::fmt::Display) -> Error {
    Error::World(e.to_string())
}
fn required<T>(s: &'static str) -> Result<T> {
    Err(Error::Unsupported(s))
}
fn supply(actors: &ActorStore, owner: ActorId) -> Result<&SupplyState> {
    match actors.component(owner, Slot::Supply).map_err(error)? {
        Some(RuntimeComponent::Supply(v)) if v.header.owner == owner => Ok(v),
        _ => required("58B0E0 stored Supply allocation/owner differs"),
    }
}
fn checked(
    catalog: &Catalog<'_>,
    actors: &ActorStore,
    bank: &Bank,
    key: ObjectKey,
) -> Result<(ActorId, Precision)> {
    let (owner, pc) =
        crate::stored_component_init_source::checked(catalog, actors, bank, key, Slot::Supply)?;
    supply(actors, owner)?;
    let leaf = aa_actors::component_virtuals::leaves(
        aa_actors::init_create::FactoryKind::Standard(Slot::Supply),
    );
    if leaf.init_existing != 0x58b0e0 || leaf.vf14 != 0x4b2830 {
        return required("58B0E0 actual Supply vtable differs");
    };
    Ok((owner, pc))
}
fn pose(staged: &Staged, key: ObjectKey) -> Result<crate::actor_world::PlacementState> {
    if let Ok(p) = staged.actors.staged_placement(key) {
        return Ok(p);
    }
    let world = staged
        .load_world
        .as_ref()
        .ok_or_else(|| error("stored Supply actual placement World absent"))?;
    staged
        .actors
        .placement(key, world.live_spatial().map_err(error)?)
        .map_err(error)
}
struct Live<'a, 'source> {
    catalog: &'a Catalog<'source>,
    world: crate::stored_component_schedule::World<'a>,
    owner: ActorId,
    source_pc: Precision,
}
impl Live<'_, '_> {
    fn low(&self) -> Result<u32> {
        Ok(self
            .world
            .staged
            .actors
            .object(self.world.key)
            .map_err(error)?
            .components()
            .map_err(error)?
            .state
            .low)
    }
    fn supply(&self) -> Result<&SupplyState> {
        supply(&self.world.staged.actors, self.owner)
    }
    fn supply_mut(&mut self) -> Result<&mut SupplyState> {
        match self
            .world
            .staged
            .actors
            .component_mut(self.owner, Slot::Supply)
            .map_err(error)?
        {
            Some(RuntimeComponent::Supply(v)) if v.header.owner == self.owner => Ok(v),
            _ => required("58AE10 current stored Supply allocation differs"),
        }
    }
    fn source(&self) -> Result<aa_rules::construction::SupplyFields> {
        let object = self
            .world
            .staged
            .actors
            .object(self.world.key)
            .map_err(error)?;
        self.catalog
            .actors
            .native_supply_fields(&object.definition.proof.id, self.source_pc)
            .map_err(error)?
            .ok_or(Error::Unsupported(
                "58AE10 actual completed SupplyData absent",
            ))
    }
    fn zone(&self) -> Result<Option<&aa_zones::zone::Zone>> {
        Ok(self
            .world
            .staged
            .actors
            .object(self.world.key)
            .map_err(error)?
            .actor
            .as_ref()
            .ok_or(Error::NullDereference("Supply actual Actor"))?
            .zones_148
            .zones()
            .slots
            .first()
            .and_then(Option::as_ref))
    }
    fn operate(&mut self, movement: Option<([u32; 2], u32)>) -> Result<()> {
        let staged = &mut *self.world.staged;
        let world = staged
            .load_world
            .as_mut()
            .ok_or_else(|| error("Supply actual persistent ZoneMap World absent"))?;
        crate::stored_zone_activation::operate_slot(
            self.catalog,
            &staged.definitions,
            &mut staged.actors,
            world,
            &staged.clock,
            self.world.key,
            0,
            movement,
            staged.inactive_load.as_ref(),
            self.world.precision,
        )
        .map_err(error)
    }
    fn vars(&self) -> Result<&crate::load_view_variables::State> {
        let vars = &self
            .world
            .staged
            .load_world
            .as_ref()
            .ok_or_else(|| error("Supply actual World variables absent"))?
            .variables;
        vars.validate_in_bank(self.catalog, &self.world.staged.definitions)
            .map_err(error)?;
        vars.require_published().map_err(error)?;
        Ok(vars)
    }
    fn rebuild(&mut self) -> Result<()> {
        let staged = &mut *self.world.staged;
        let world = staged
            .load_world
            .as_ref()
            .ok_or_else(|| error("Supply action refresh actual World absent"))?;
        crate::stored_actor_actions::rebuild_with_variables(
            self.catalog,
            &mut staged.actors,
            &mut staged.definitions,
            self.world.key,
            &staged.inventory,
            world.live_spatial().map_err(error)?,
            Some(&world.variables),
            self.world.precision,
        )
        .map_err(error)
    }
}
impl InitWorld for Live<'_, '_> {
    fn schedule(&mut self) -> Result<()> {
        if !self.world.pending()? {
            self.world.add_periodic(1)?
        }
        Ok(())
    }
    fn zone_present(&mut self) -> Result<bool> {
        Ok(self.zone()?.is_some())
    }
    fn create_zone(&mut self) -> Result<()> {
        let flags = pose(self.world.staged, self.world.key)?.flags;
        let staged = &mut *self.world.staged;
        let identities = &mut staged
            .load_world
            .as_mut()
            .ok_or_else(|| error("Supply actual zone identity owner absent"))?
            .zone_identities;
        let actor = staged
            .actors
            .object_mut(self.world.key)
            .map_err(error)?
            .actor
            .as_mut()
            .ok_or(Error::NullDereference("Supply actual Actor"))?;
        crate::load_detection_activation::create_slot_fields(
            self.owner,
            flags,
            actor.zones_148.zones_mut(),
            identities,
            0,
        )
        .map(|_| ())
        .map_err(error)
    }
    fn tick(&mut self) -> Result<()> {
        crate::load_supply_init_world::tick(self, 0, self.world.precision)
    }
}
impl TickWorld for Live<'_, '_> {
    fn relay(&mut self) -> Result<bool> {
        Ok(self.source()?.is_relay)
    }
    fn suspended(&mut self) -> Result<bool> {
        let w = self
            .world
            .staged
            .load_world
            .as_ref()
            .ok_or_else(|| error("Supply actual MapResource settings World absent"))?;
        let mode = crate::live_placement::read_settings_mode(
            true,
            None,
            w.global_settings_mode,
            Some(&w.map_mode),
        )
        .map_err(error)?
        .ok_or(Error::NullDereference("4F0460 actual World+28 MapResource"))?;
        Ok(mode != 0)
    }
    fn clear_relay(&mut self) -> Result<()> {
        self.supply_mut()?.word_1c = 0;
        Ok(())
    }
    fn search_relays(&mut self) -> Result<()> {
        required("58AE64 stored actual owner/shape/ZoneMap relay collection")
    }
    fn update_supply_state(&mut self) -> Result<()> {
        if self.source()?.is_relay {
            return required("4D0A80 stored relay/Element/Kingdom/territory services");
        };
        crate::load_supply_init_world::primary_update(self).map(|_| ())
    }
    fn out_of_supply(&mut self) -> Result<bool> {
        Ok(self.low()? & 0x80000 != 0)
    }
    fn stockpile(&mut self) -> Result<u32> {
        Ok(self.supply()?.stockpile.0)
    }
    fn maximum(&mut self) -> Result<u32> {
        self.source()?.stockpile_max_bits.ok_or(Error::Unsupported(
            "58AFD5 actual relay stockpile maximum absent",
        ))
    }
    fn recharge(&mut self) -> Result<u32> {
        Ok(self.vars()?.supply_recharge_rate_bits())
    }
    fn store_stockpile(&mut self, bits: u32) -> Result<()> {
        self.supply_mut()?.stockpile = aa_body::F32(bits);
        Ok(())
    }
    fn refresh_economy(&mut self) -> Result<()> {
        let staged = &mut *self.world.staged;
        let world = staged
            .load_world
            .as_ref()
            .ok_or_else(|| error("Supply actual Economy refresh World absent"))?;
        let (_, mut actors) = staged.actors.split_minds();
        crate::stored_economy_refresh::refresh(
            self.catalog,
            &mut actors,
            &mut staged.definitions,
            world.live_spatial().map_err(error)?,
            &world.variables,
            self.owner,
            self.world.precision,
        )
        .map_err(error)
    }
    fn provides(&mut self) -> Result<bool> {
        crate::load_supply_init_world::provides(self)
    }
    fn zone_active(&mut self) -> Result<bool> {
        Ok(self.zone()?.is_some_and(|z| z.active()))
    }
    fn clear_zone(&mut self) -> Result<()> {
        self.operate(None)
    }
    fn dirty(&mut self) -> Result<bool> {
        Ok(self.supply()?.byte_14 != 0)
    }
    fn radius(&mut self) -> Result<u32> {
        Ok(self.supply()?.supply_range.0)
    }
    fn position(&mut self) -> Result<[u32; 2]> {
        let p = pose(self.world.staged, self.world.key)?;
        Ok([p.position_bits[0], p.position_bits[1]])
    }
    fn move_zone(&mut self, p: [u32; 2], r: u32) -> Result<()> {
        self.operate(Some((p, r)))
    }
    fn clear_dirty(&mut self) -> Result<()> {
        self.supply_mut()?.byte_14 = 0;
        Ok(())
    }
}
impl ProvideWorld for Live<'_, '_> {
    fn owner(&mut self) -> Result<u32> {
        self.world
            .staged
            .actors
            .object(self.world.key)
            .map_err(error)?
            .actor
            .as_ref()
            .ok_or(Error::NullDereference("Supply actual Actor"))?
            .words
            .get(&0xe8)
            .copied()
            .ok_or(Error::Unsupported(
                "58ACE8 stored actual ActorE8 uninitialized",
            ))
    }
    fn nonmajor(&mut self, raw: u32) -> Result<u32> {
        let id = ActorId::from_raw(raw);
        let key = self.world.staged.actors.key(id).ok_or(Error::Unsupported(
            "58AD01 actual owner Kingdom generation absent",
        ))?;
        let runtime = self.world.staged.actors.object(key).map_err(error)?;
        crate::live_object_init::Source::bind_in_bank(
            self.catalog,
            &self.world.staged.definitions,
            &runtime.definition,
        )?;
        if runtime.definition.native_class != 4 || runtime.actor.is_none() {
            return required("58AD01 actual Kingdom constructor differs");
        };
        let Source::Special(s) = &self.world.staged.actors.entity(key).map_err(error)?.source
        else {
            return required("58AD01 actual Kingdom derived owner absent");
        };
        s.derived
            .as_ref()
            .and_then(|s| s.words.get(&0x190))
            .copied()
            .ok_or(Error::Unsupported("58AD01 actual Kingdom190 uninitialized"))
    }
    fn high(&mut self) -> Result<u32> {
        Ok(self
            .world
            .staged
            .actors
            .object(self.world.key)
            .map_err(error)?
            .components()
            .map_err(error)?
            .state
            .high)
    }
    fn low(&mut self) -> Result<u32> {
        Live::low(self)
    }
    fn destroyed(&mut self) -> Result<bool> {
        Ok(pose(self.world.staged, self.world.key)?.flags & 1 != 0)
    }
    fn under_siege_enabled(&mut self) -> Result<bool> {
        Ok(self.vars()?.supply_provide_under_siege())
    }
    fn under_siege(&mut self) -> Result<bool> {
        Ok(self.low()? & 0x100000 != 0)
    }
    fn is_relay(&mut self) -> Result<bool> {
        Ok(self.source()?.is_relay)
    }
    fn stockpile_supplied(&mut self) -> Result<bool> {
        Ok(self.low()? & 0x80000 == 0 || f32::from_bits(self.supply()?.stockpile.0) > 0.0)
    }
    fn range(&mut self) -> Result<u32> {
        Ok(self.supply()?.supply_range.0)
    }
    fn city(&mut self) -> Result<bool> {
        match self
            .world
            .staged
            .actors
            .component(self.owner, Slot::City)
            .map_err(error)?
        {
            None => Ok(false),
            Some(RuntimeComponent::City(c)) if c.header.owner == self.owner => Ok(c.byte_20 != 0),
            _ => required("58AD93 actual City payload/owner differs"),
        }
    }
}
impl PrimaryWorld for Live<'_, '_> {
    fn out(&mut self) -> Result<bool> {
        Ok(self.low()? & 0x80000 != 0)
    }
    fn supplied_state(&mut self) -> Result<()> {
        set_supplied(
            self.world.staged,
            self.catalog,
            self.world.key,
            self.world.precision,
        )
    }
    fn refresh_actions(&mut self) -> Result<()> {
        self.rebuild()
    }
}
pub fn init_existing(
    catalog: &Catalog<'_>,
    staged: &mut Staged,
    key: ObjectKey,
    pc: Precision,
) -> Result<bool> {
    let (owner, source_pc) = checked(catalog, &staged.actors, &staged.definitions, key)?;
    crate::load_supply_init_world::initialize(&mut Live {
        catalog,
        world: crate::stored_component_schedule::World::new(staged, key, Slot::Supply, pc)?,
        owner,
        source_pc,
    })
}
pub fn after_init(
    catalog: &Catalog<'_>,
    actors: &ActorStore,
    bank: &Bank,
    key: ObjectKey,
) -> Result<()> {
    checked(catalog, actors, bank, key)?;
    Ok(())
}

struct StateEffects<'a, 'source> {
    staged: &'a mut Staged,
    catalog: &'a Catalog<'source>,
    key: ObjectKey,
    pc: Precision,
}
impl StateEffects<'_, '_> {
    fn commit(&mut self, f: &state::Fields) -> std::result::Result<(), String> {
        let s = &mut self
            .staged
            .actors
            .object_mut(self.key)
            .map_err(|e| e.to_string())?
            .components_mut()
            .map_err(|e| e.to_string())?
            .state;
        s.low = f.bits as u32;
        s.high = (f.bits >> 32) as u32;
        Ok(())
    }
    fn reread(&self, f: &mut state::Fields) -> std::result::Result<(), String> {
        let s = self
            .staged
            .actors
            .object(self.key)
            .map_err(|e| e.to_string())?
            .components()
            .map_err(|e| e.to_string())?
            .state;
        f.bits = u64::from(s.low) | (u64::from(s.high) << 32);
        f.thing_flags = pose(self.staged, self.key)
            .map_err(|e| e.to_string())?
            .flags;
        Ok(())
    }
}
impl state::Effects for StateEffects<'_, '_> {
    fn leave(&mut self, f: &mut state::Fields, d: &'static state::Descriptor) -> state::Result<()> {
        self.commit(f).map_err(state::Error::Callback)?;
        let r = crate::stored_actor_state_effects::leave_staged(
            self.staged,
            self.catalog,
            self.key,
            d.index,
            self.pc,
        );
        self.reread(f).map_err(state::Error::Callback)?;
        r.map_err(state::Error::Callback)
    }
    fn enter(
        &mut self,
        f: &mut state::Fields,
        d: &'static state::Descriptor,
        o: Option<&'static state::Descriptor>,
    ) -> state::Result<()> {
        self.commit(f).map_err(state::Error::Callback)?;
        let r = crate::stored_actor_state_effects::enter_staged(
            self.staged,
            self.catalog,
            self.key,
            d.index,
            o.map(|d| d.index),
            self.pc,
        );
        self.reread(f).map_err(state::Error::Callback)?;
        r.map_err(state::Error::Callback)
    }
    fn rebuild_abilities(&mut self, f: &mut state::Fields) -> state::Result<()> {
        self.commit(f).map_err(state::Error::Callback)?;
        let staged = &mut *self.staged;
        let r = (|| {
            let w = staged
                .load_world
                .as_ref()
                .ok_or("Supply state rebuild actual World absent")?;
            crate::stored_actor_actions::rebuild_with_variables(
                self.catalog,
                &mut staged.actors,
                &mut staged.definitions,
                self.key,
                &staged.inventory,
                w.live_spatial()?,
                Some(&w.variables),
                self.pc,
            )
        })();
        self.reread(f).map_err(state::Error::Callback)?;
        r.map_err(state::Error::Callback)
    }
}
fn set_supplied(
    staged: &mut Staged,
    catalog: &Catalog<'_>,
    key: ObjectKey,
    pc: Precision,
) -> Result<()> {
    let s = staged
        .actors
        .object(key)
        .map_err(error)?
        .components()
        .map_err(error)?
        .state;
    let mut f = state::Fields {
        bits: u64::from(s.low) | (u64::from(s.high) << 32),
        thing_flags: pose(staged, key)?.flags,
    };
    let mut effects = StateEffects {
        staged,
        catalog,
        key,
        pc,
    };
    let r = state::set(&mut f, 19, false, &mut effects).map_err(error);
    effects.commit(&f).map_err(error)?;
    r
}

#[cfg(test)]
#[path = "stored_supply_init_world_tests.rs"]
mod tests;
