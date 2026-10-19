//! SupplyComponent periodic Update on the live retained World.
//!
//! Supply+8 is its Updatable (vtable A40098; queue1 via 50AC60 at 58B0E0).
//! Periodic 50B330 calls +14 = 58B120, a thunk passing the elapsed seconds to
//! 58AE10. The native body is load_supply_init_world::tick (with 58ACE0
//! provides and 4D7CE0 primary update), shared with the saved/fresh Init
//! adapters; this module binds the same kernels to the live owners instead of
//! the load-time Staged graph. 4F0460 reads [World+28]+A8 (null -> false).
//! State-changing leaves without a live binding (relay collection 58AE64,
//! 4D0A80 relay state, 4D64F0 state 19 + action rebuild, Economy refresh
//! 570020) return explicit errors at their call; zone release/move use the
//! same 514CE0/515F40 zone kernels through stored_zone_activation.
use crate::{
    actor_world::{ActorStore, ObjectKey, RuntimeComponent, Source},
    catalog::Catalog,
    definitions::Bank,
    load_supply_init_world::{PrimaryWorld, ProvideWorld, TickWorld},
};
use aa_actors::{
    ActorId,
    actor_creation::{Error, Result},
    components::ComponentSlot as Slot,
    supply_state::SupplyState,
};
use aa_numeric::Precision;
use aa_simulation::NativeClock;
use aa_world::queues::{Delivery, Error as QueueError};

fn error(e: impl std::fmt::Display) -> Error {
    Error::World(e.to_string())
}
fn required<T>(s: &'static str) -> Result<T> {
    Err(Error::Unsupported(s))
}

pub(crate) struct Context<'a, 'source> {
    pub catalog: &'a Catalog<'source>,
    pub bank: &'a Bank,
    pub actors: &'a mut ActorStore,
    pub world: &'a mut crate::load_world::World,
    pub clock: &'a NativeClock,
    pub phase: u32,
    pub source_precision: Precision,
    pub precision: Precision,
}

struct Live<'a, 'b, 'source> {
    cx: &'a mut Context<'b, 'source>,
    key: ObjectKey,
    owner: ActorId,
}
impl Live<'_, '_, '_> {
    fn low(&self) -> Result<u32> {
        Ok(self.cx.actors.object(self.key).map_err(error)?.components().map_err(error)?.state.low)
    }
    fn high(&self) -> Result<u32> {
        Ok(self.cx.actors.object(self.key).map_err(error)?.components().map_err(error)?.state.high)
    }
    fn supply(&self) -> Result<&SupplyState> {
        match self.cx.actors.component(self.owner, Slot::Supply).map_err(error)? {
            Some(RuntimeComponent::Supply(v)) if v.header.owner == self.owner => Ok(v),
            _ => required("58AE10 live Supply allocation/owner differs"),
        }
    }
    fn supply_mut(&mut self) -> Result<&mut SupplyState> {
        match self.cx.actors.component_mut(self.owner, Slot::Supply).map_err(error)? {
            Some(RuntimeComponent::Supply(v)) if v.header.owner == self.owner => Ok(v),
            _ => required("58AE10 live Supply allocation/owner differs"),
        }
    }
    fn source(&self) -> Result<aa_rules::construction::SupplyFields> {
        let object = self.cx.actors.object(self.key).map_err(error)?;
        self.cx
            .catalog
            .actors
            .native_supply_fields(&object.definition.proof.id, self.cx.source_precision)
            .map_err(error)?
            .ok_or(Error::Unsupported("58AE10 actual completed SupplyData absent"))
    }
    fn pose(&self) -> Result<crate::actor_world::PlacementState> {
        let spatial = self.cx.world.live_spatial().map_err(error)?;
        self.cx.actors.placement(self.key, spatial).map_err(error)
    }
    fn zone(&self) -> Result<Option<&aa_zones::zone::Zone>> {
        Ok(self
            .cx
            .actors
            .object(self.key)
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
        let phase = self.cx.phase;
        crate::stored_zone_activation::operate_slot_in_phase(
            self.cx.catalog,
            self.cx.bank,
            self.cx.actors,
            self.cx.world,
            self.cx.clock,
            &phase,
            self.key,
            0,
            movement,
            None,
            self.cx.precision,
        )
        .map_err(error)
    }
}
impl TickWorld for Live<'_, '_, '_> {
    fn relay(&mut self) -> Result<bool> {
        Ok(self.source()?.is_relay)
    }
    fn suspended(&mut self) -> Result<bool> {
        //4F0460: null World+28 is false, otherwise its +A8 is nonzero.
        let mode = crate::live_placement::read_settings_mode(
            true,
            None,
            self.cx.world.global_settings_mode,
            Some(&self.cx.world.map_mode),
        )
        .map_err(error)?;
        Ok(mode.is_some_and(|mode| mode != 0))
    }
    fn clear_relay(&mut self) -> Result<()> {
        self.supply_mut()?.word_1c = 0;
        Ok(())
    }
    fn search_relays(&mut self) -> Result<()> {
        required("58AE64 live relay candidate collection (owner/shape/ZoneMap)")
    }
    fn update_supply_state(&mut self) -> Result<()> {
        if self.source()?.is_relay {
            return required("4D0A80 live relay/Element/Kingdom/territory services");
        }
        crate::load_supply_init_world::primary_update(self).map(|_| ())
    }
    fn out_of_supply(&mut self) -> Result<bool> {
        Ok(self.low()? & 0x80000 != 0)
    }
    fn stockpile(&mut self) -> Result<u32> {
        Ok(self.supply()?.stockpile.0)
    }
    fn maximum(&mut self) -> Result<u32> {
        self.source()?
            .stockpile_max_bits
            .ok_or(Error::Unsupported("58AFD5 actual relay stockpile maximum absent"))
    }
    fn recharge(&mut self) -> Result<u32> {
        Ok(self.cx.world.variables.supply_recharge_rate_bits())
    }
    fn store_stockpile(&mut self, bits: u32) -> Result<()> {
        self.supply_mut()?.stockpile = aa_body::F32(bits);
        Ok(())
    }
    fn refresh_economy(&mut self) -> Result<()> {
        required("570020 live Economy refresh after relay depletion")
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
        let p = self.pose()?;
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
impl ProvideWorld for Live<'_, '_, '_> {
    fn owner(&mut self) -> Result<u32> {
        self.cx
            .actors
            .object(self.key)
            .map_err(error)?
            .actor
            .as_ref()
            .ok_or(Error::NullDereference("Supply actual Actor"))?
            .words
            .get(&0xe8)
            .copied()
            .ok_or(Error::Unsupported("58ACE8 live ActorE8 uninitialized"))
    }
    fn nonmajor(&mut self, raw: u32) -> Result<u32> {
        let key = self
            .cx
            .actors
            .key(ActorId::from_raw(raw))
            .ok_or(Error::Unsupported("58AD01 actual owner Kingdom generation absent"))?;
        let runtime = self.cx.actors.object(key).map_err(error)?;
        if runtime.definition.native_class != 4 || runtime.actor.is_none() {
            return required("58AD01 actual Kingdom constructor differs");
        }
        let Source::Special(s) = &self.cx.actors.entity(key).map_err(error)?.source else {
            return required("58AD01 actual Kingdom derived owner absent");
        };
        s.derived
            .as_ref()
            .and_then(|s| s.words.get(&0x190))
            .copied()
            .ok_or(Error::Unsupported("58AD01 actual Kingdom190 uninitialized"))
    }
    fn high(&mut self) -> Result<u32> {
        Live::high(self)
    }
    fn low(&mut self) -> Result<u32> {
        Live::low(self)
    }
    fn destroyed(&mut self) -> Result<bool> {
        Ok(self.pose()?.flags & 1 != 0)
    }
    fn under_siege_enabled(&mut self) -> Result<bool> {
        Ok(self.cx.world.variables.supply_provide_under_siege())
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
        match self.cx.actors.component(self.owner, Slot::City).map_err(error)? {
            None => Ok(false),
            Some(RuntimeComponent::City(c)) if c.header.owner == self.owner => Ok(c.byte_20 != 0),
            _ => required("58AD93 actual City payload/owner differs"),
        }
    }
}
impl PrimaryWorld for Live<'_, '_, '_> {
    fn out(&mut self) -> Result<bool> {
        Ok(self.low()? & 0x80000 != 0)
    }
    fn supplied_state(&mut self) -> Result<()> {
        required("4D64F0 live state 19 transition (enter/leave effects)")
    }
    fn refresh_actions(&mut self) -> Result<()> {
        required("live Supply action rebuild after state 19")
    }
}

/// One queue1 periodic delivery to Supply+8 (+14 = 58B120 -> 58AE10).
pub(crate) fn run(key: ObjectKey, event: Delivery, cx: &mut Context<'_, '_>) -> aa_world::queues::Result<()> {
    let fail = |e: Error| QueueError::Callback(format!("live Supply 58AE10: {e}"));
    let seconds = event.elapsed_bits.ok_or_else(|| {
        QueueError::Callback("live Supply: +18/+1C are 943800 RET4; deferred delivery is invalid".into())
    })?;
    let owner = cx
        .actors
        .id(key)
        .ok_or_else(|| QueueError::Callback("live Supply: owner has no live GTID".into()))?;
    let precision = cx.precision;
    let mut live = Live { cx, key, owner };
    live.supply().map_err(fail)?;
    crate::load_supply_init_world::tick(&mut live, seconds, precision).map_err(fail)
}
