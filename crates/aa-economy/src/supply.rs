//! Original supply: SupplyComponent (component 11 at actor+0x8C, constructor
//! `0x58AA50`, vtable `0xA400C0`, updatable at +8 with vtable `0xA40098` whose
//! Tick slot 5 is the thunk `0x58B120` -> `0x58AE10`) and the actor in-supply
//! state driven by EconomyComponent::Tick `0x56F2B0`.
//!
//! Ported: `0x58AE10` supply Tick (relay search, stockpile drain/recharge,
//! supply-zone activation decision), `0x58ACE0` provides-supply predicate,
//! `0x58ADB0` relay chain, `0x58AC70` zone active, `0x4D0C90` stockpile
//! in-supply, `0x4D0A80` actor in supply, `0x4D7CE0` out-of-supply state
//! update, `0x56F2B0` EconomyComponent::Tick, plus leaves `0x44DEE0`,
//! `0x4CE830`, `0x4CF2C0`, `0x432EF0`, `0x529C30`, `0x4F0460` as field reads.
//!
//! Actor state bits live at actor+0x100 (64-bit, bit n = state n): 0x13 out of
//! supply (0x80000), 0x14 under siege, 0x15 (0x200000), 0x27 (+0x104 bit 7).
//! Spatial/territory code stays an explicit seam: tile ownership `0x54F8B0`,
//! covered-tile iteration `0x554DD0`/`0x554330`, relay candidate collection
//! `0x554CC0`/`0x54F8F0`, large-actor test `0x4EE700`, zone raster updates
//! `0x514CE0`/`0x515F40`, state transitions `0x4D64F0`, refreshes `0x4D1350`
//! and `0x4CF0F0`, and economy-tick unscheduling `0x50B1C0(1,0)`.
use crate::binary::Binary;
use crate::{EconomyError, F32Bits, X87Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeSet;

pub const STATE_OUT_OF_SUPPLY: u32 = 0x13;
pub const STATE_UNDER_SIEGE: u32 = 0x14;
pub const STATE_21: u32 = 0x15;
pub const STATE_39: u32 = 0x27;
/// Zone entry +8 sentinel meaning "inactive" (`0x58AC70`).
pub const INACTIVE_ZONE_BITS: F32Bits = F32Bits(0xbf80_0000);

fn bit(flags: u64, state: u32) -> bool {
    flags & (1u64 << state) != 0
}
fn b(value: F32Bits) -> Binary {
    Binary::from_bits(value)
}
fn finite(values: &[F32Bits]) -> Result<(), EconomyError> {
    if values.iter().all(|v| v.is_finite()) {
        Ok(())
    } else {
        Err(EconomyError::NonFiniteInput)
    }
}

/// SupplyComponent fields.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupplyComponent {
    /// +0x10 supply radius (ActorData supply_range at construction).
    pub radius: F32Bits,
    /// +0x14 zone geometry dirty flag.
    pub dirty: bool,
    /// +0x18 current supply stockpile.
    pub stockpile: F32Bits,
    /// +0x1C GTID of the relay this stockpile actor draws from (0 = none).
    pub relay: u32,
}

/// ActorData bytes/floats read by the supply code.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupplyDefinition {
    /// +0x424: stockpile relay (needs a connection); false = primary source.
    pub relay: bool,
    /// +0x428 stockpile maximum.
    pub stockpile_max: F32Bits,
    /// +0x348: the Economy component keeps ticking.
    pub economy_ticks: bool,
    /// +0x31D and +0x2F4 bytes read by `0x4D0A80`.
    pub byte_31d: bool,
    pub byte_2f4: bool,
}

/// Actor fields read by the supply code (native offsets).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupplyActor {
    /// +0x14 own GTID.
    pub gtid: u32,
    /// +8 flags (bit 0: destroyed).
    pub flags8: u32,
    /// +0x100/+0x104 state bits.
    pub states: u64,
    pub definition: SupplyDefinition,
    /// +0x8C.
    pub supply: Option<SupplyComponent>,
    /// +0x80 Element component present and its +0x10 organization actor.
    pub element: Option<Option<u32>>,
    /// +0x7C Organization component present and its +0xD4 word nonzero.
    pub organization_component: Option<bool>,
    /// +0x78 Properties component present and its +0x1C bit 5.
    pub properties_flag20: Option<bool>,
    /// +0xD4 City component present and its +0x20 byte.
    pub city_flag20: Option<bool>,
    /// +0x118, +0x128 and +0xD8 words nonzero.
    pub word_118: bool,
    pub word_128: bool,
    pub word_d8: bool,
    /// +0xD0 GTID tested by `0x44DEE0`.
    pub gtid_d0: u32,
    /// +0xE8 kingdom.
    pub kingdom: Option<u32>,
    /// +0x20/+0x24 position passed to `0x515F40`.
    pub position: [F32Bits; 2],
    /// +0x44/+0x48 tile column/row.
    pub tile: [u32; 2],
    /// +0x148/+0x14C first zone entry +8 word, `None` for an empty list or a
    /// null first entry.
    pub zone_word8: Option<F32Bits>,
}

/// Kingdom fields read by the supply code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KingdomSupply {
    /// +0x190: value 1 disables supply.
    pub word_190: u32,
    /// +0x1F0 upkeep-shortage ratio.
    pub shortage_ratio: F32Bits,
}

pub trait SupplyWorld {
    fn actor(&self, actor: u32) -> Result<&SupplyActor, EconomyError>;
    fn actor_mut(&mut self, actor: u32) -> Result<&mut SupplyActor, EconomyError>;
    /// Generation-checked GTID lookup (`0xB364C0`).
    fn resolve(&self, gtid: u32) -> Option<u32>;
    fn kingdom(&self, kingdom: u32) -> Result<KingdomSupply, EconomyError>;
    /// Actor virtual +0x108 (owner kingdom), called at each native call site.
    fn owner(&mut self, actor: u32) -> Result<Option<u32>, EconomyError>;
    /// `world.0x4F0460()`: world+0x28 object +0xA8 nonzero.
    fn world_suspended(&mut self) -> Result<bool, EconomyError>;
    /// `*(cache+0xFC)` byte (supply provided under siege).
    fn provide_under_siege(&self) -> bool;
    /// `*(cache+0x100)` SupplyStockpileRechargeRate.
    fn recharge_rate(&self) -> F32Bits;
    /// Map width (`world+0x30` map +0x1C) used for tile indices.
    fn map_width(&self) -> u32;
    fn large_actor(&mut self, actor: u32) -> Result<bool, EconomyError>;
    fn tile_supplied(
        &mut self,
        tile: u32,
        kingdom: Option<u32>,
        actor: u32,
    ) -> Result<bool, EconomyError>;
    /// Tiles covered by the actor shape, in native iteration order.
    fn covered_tiles(&mut self, actor: u32) -> Result<Vec<u32>, EconomyError>;
    /// Supply things near the actor for its owner kingdom, in list order.
    fn relay_candidates(
        &mut self,
        actor: u32,
        kingdom: Option<u32>,
    ) -> Result<Vec<u32>, EconomyError>;
    /// `0x4D64F0(state, enabled)`; must update `states` like the native call.
    fn set_state(&mut self, actor: u32, state: u32, enabled: bool) -> Result<(), EconomyError>;
    /// `0x4D1350`.
    fn refresh_actor(&mut self, actor: u32) -> Result<(), EconomyError>;
    /// `0x4CF0F0` (Economy refresh).
    fn refresh_economy(&mut self, actor: u32) -> Result<(), EconomyError>;
    /// "actor %d stopping economy ticks" + `0x50B1C0(1, 0)`.
    fn stop_economy_ticks(&mut self, actor: u32) -> Result<(), EconomyError>;
    /// `firstZoneEntry.0x514CE0()`.
    fn clear_zone(&mut self, actor: u32) -> Result<(), EconomyError>;
    /// `firstZoneEntry.0x515F40(x, y, radius)`; `None` entry when the list is
    /// empty or its first element is null.
    fn set_zone(
        &mut self,
        actor: u32,
        position: [F32Bits; 2],
        radius: F32Bits,
    ) -> Result<(), EconomyError>;
}

/// `0x58AC70`.
pub fn zone_active(actor: &SupplyActor) -> bool {
    actor.zone_word8.is_some_and(|w| w != INACTIVE_ZONE_BITS)
}

/// `0x4D0C90`: out-of-supply actors are still supplied while their stockpile
/// is positive.
pub fn stockpile_in_supply(actor: &SupplyActor) -> Result<bool, EconomyError> {
    if !bit(actor.states, STATE_OUT_OF_SUPPLY) {
        return Ok(true);
    }
    let Some(supply) = &actor.supply else {
        return Ok(false);
    };
    finite(&[supply.stockpile])?;
    Ok(b(supply.stockpile).cmp(&Binary::from_i32(0)) == Ordering::Greater)
}

/// `0x58ADB0`: follow relay GTIDs until a non-relay supplier. A cycle would
/// hang the native loop and is reported as unsupported.
pub fn relay_reaches_source(world: &impl SupplyWorld, start: u32) -> Result<bool, EconomyError> {
    let mut seen = BTreeSet::new();
    let mut actor = start;
    loop {
        let current = world.actor(actor)?;
        let Some(supply) = &current.supply else {
            return Ok(false);
        };
        if !current.definition.relay {
            return Ok(true);
        }
        if !seen.insert(actor) {
            return Err(EconomyError::World(
                "supply relay cycle: native 0x58ADB0 would not terminate".into(),
            ));
        }
        match world.resolve(supply.relay) {
            Some(next) => actor = next,
            None => return Ok(false),
        }
    }
}

/// `0x58ACE0`.
pub fn provides_supply(world: &mut impl SupplyWorld, actor: u32) -> Result<bool, EconomyError> {
    if world.owner(actor)?.is_none() {
        return Ok(false);
    }
    let owner = world.owner(actor)?.ok_or_else(|| {
        EconomyError::World("owner kingdom disappeared between native rereads".into())
    })?;
    if world.kingdom(owner)?.word_190 == 1 {
        return Ok(false);
    }
    let a = world.actor(actor)?.clone();
    if bit(a.states, STATE_39) || bit(a.states, STATE_21) || a.flags8 & 1 != 0 {
        return Ok(false);
    }
    if !world.provide_under_siege() && bit(a.states, STATE_UNDER_SIEGE) {
        return Ok(false);
    }
    if a.definition.relay && !stockpile_in_supply(&a)? {
        return Ok(false);
    }
    let supply = a
        .supply
        .as_ref()
        .ok_or_else(|| EconomyError::World("provides_supply requires a SupplyComponent".into()))?;
    finite(&[supply.radius])?;
    if b(supply.radius).cmp(&Binary::from_i32(0)) == Ordering::Less {
        return Ok(false);
    }
    if a.city_flag20 == Some(true) {
        return Ok(false);
    }
    Ok(true)
}

/// `0x4D0A80`.
pub fn in_supply(world: &mut impl SupplyWorld, actor: u32) -> Result<bool, EconomyError> {
    let a = world.actor(actor)?.clone();
    if a.supply.is_some() && !a.definition.relay {
        return Ok(true);
    }
    if let Some(Some(organization)) = a.element {
        let org = world.actor(organization)?;
        if org.definition.byte_2f4 {
            return Ok(!bit(org.states, STATE_OUT_OF_SUPPLY));
        }
    }
    let Some(kingdom) = a.kingdom else {
        return Ok(false);
    };
    let k = world.kingdom(kingdom)?;
    if k.word_190 == 1 {
        return Ok(false);
    }
    let modifier = a.properties_flag20 == Some(true);
    if let Some(org_d4) = a.organization_component {
        if a.word_128 || a.word_118 || org_d4 {
            return Ok(false);
        }
        if !a.definition.byte_31d && world.resolve(a.gtid_d0).is_none() && !a.word_d8 && !modifier {
            return Ok(false);
        }
    }
    finite(&[k.shortage_ratio])?;
    if b(k.shortage_ratio).cmp(&Binary::from_i32(0)) != Ordering::Greater && modifier {
        return Ok(true);
    }
    if a.supply.is_some() && a.definition.relay {
        // 0x4CE830 true -> 0x4CF2C0: the relay GTID still resolves.
        let relay = a.supply.as_ref().map(|s| s.relay).unwrap_or(0);
        return Ok(world.resolve(relay).is_some());
    }
    if !world.large_actor(actor)? {
        let width = world.map_width();
        let tile = a.tile[1].wrapping_mul(width).wrapping_add(a.tile[0]);
        return world.tile_supplied(tile, a.kingdom, actor);
    }
    for tile in world.covered_tiles(actor)? {
        if world.tile_supplied(tile, a.kingdom, actor)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// `0x4D7CE0`: when the computed supply equals the out-of-supply flag, the
/// state flips (`0x4D64F0(0x13, !in_supply)`) and `0x4D1350` runs.
pub fn update_supply_state(world: &mut impl SupplyWorld, actor: u32) -> Result<bool, EconomyError> {
    let supplied = in_supply(world, actor)?;
    let flagged = bit(world.actor(actor)?.states, STATE_OUT_OF_SUPPLY);
    if supplied != flagged {
        return Ok(false);
    }
    world.set_state(actor, STATE_OUT_OF_SUPPLY, !supplied)?;
    world.refresh_actor(actor)?;
    Ok(true)
}

/// `0x56F2B0` EconomyComponent::Tick.
pub fn economy_component_tick(
    world: &mut impl SupplyWorld,
    actor: u32,
) -> Result<(), EconomyError> {
    if !world.actor(actor)?.definition.economy_ticks {
        return world.stop_economy_ticks(actor);
    }
    update_supply_state(world, actor).map(|_| ())
}

/// `0x58AE10` SupplyComponent Tick.
pub fn supply_tick(
    world: &mut impl SupplyWorld,
    actor: u32,
    seconds: F32Bits,
    precision: X87Precision,
) -> Result<(), EconomyError> {
    finite(&[seconds])?;
    if world.actor(actor)?.definition.relay && !world.world_suspended()? {
        component(world, actor)?.relay = 0;
        let owner = world.owner(actor)?;
        for candidate in world.relay_candidates(actor, owner)? {
            let c = world.actor(candidate)?;
            if bit(c.states, STATE_OUT_OF_SUPPLY) {
                continue;
            }
            let gtid = c.gtid;
            if relay_reaches_source(world, candidate)? {
                component(world, actor)?.relay = gtid;
                break;
            }
        }
    }
    update_supply_state(world, actor)?;
    let a = world.actor(actor)?.clone();
    if a.definition.relay {
        let zero = Binary::from_i32(0);
        let mut supply = a
            .supply
            .clone()
            .ok_or_else(|| EconomyError::World("supply tick without component".into()))?;
        finite(&[supply.stockpile, a.definition.stockpile_max])?;
        if bit(a.states, STATE_OUT_OF_SUPPLY) {
            if b(supply.stockpile).cmp(&zero) == Ordering::Greater {
                supply.stockpile = b(supply.stockpile)
                    .sub(&b(seconds), precision)
                    .store_f32()?;
                if b(supply.stockpile).cmp(&zero) == Ordering::Less {
                    supply.stockpile = F32Bits::ZERO;
                }
                let depleted = b(supply.stockpile).cmp(&zero) != Ordering::Greater;
                component(world, actor)?.stockpile = supply.stockpile;
                if depleted {
                    world.refresh_economy(actor)?;
                }
            }
        } else if b(a.definition.stockpile_max).cmp(&b(supply.stockpile)) == Ordering::Greater {
            let rate = world.recharge_rate();
            finite(&[rate])?;
            let mut stock = b(rate)
                .mul(&b(seconds), precision)
                .add(&b(supply.stockpile), precision)
                .store_f32()?;
            if b(a.definition.stockpile_max).cmp(&b(stock)) == Ordering::Less {
                stock = a.definition.stockpile_max;
            }
            component(world, actor)?.stockpile = stock;
        }
    }
    let provides = provides_supply(world, actor)?;
    let a = world.actor(actor)?.clone();
    let active = zone_active(&a);
    if provides == active {
        if !active || !a.supply.as_ref().is_some_and(|s| s.dirty) {
            return Ok(());
        }
    } else if active {
        world.clear_zone(actor)?;
        if !world.actor(actor)?.supply.as_ref().is_some_and(|s| s.dirty) {
            return Ok(());
        }
    }
    let a = world.actor(actor)?.clone();
    let radius = a.supply.as_ref().map(|s| s.radius).unwrap_or(F32Bits::ZERO);
    world.set_zone(actor, a.position, radius)?;
    component(world, actor)?.dirty = false;
    Ok(())
}

fn component(
    world: &mut impl SupplyWorld,
    actor: u32,
) -> Result<&mut SupplyComponent, EconomyError> {
    world
        .actor_mut(actor)?
        .supply
        .as_mut()
        .ok_or_else(|| EconomyError::World("supply component missing".into()))
}
