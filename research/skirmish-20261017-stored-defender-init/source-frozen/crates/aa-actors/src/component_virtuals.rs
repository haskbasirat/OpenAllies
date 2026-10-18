//! The component virtuals of every `4CF480` class that actor creation calls:
//! InitCreate (+4), InitExisting (+8), PostCreate (+C), +14 and the required
//! component (+20), dispatched by the constructed class ([`FactoryKind`]:
//! the 31 slots, with the three Mind classes of slot 4).
//!
//! Component fields are addressed by their native byte offsets through
//! [`ComponentWorld`] (the owners' typed states sit behind it); every other
//! owner's function is a world method named after the original.
//!
//! Evidence: research/phase8-actor-creation/native-component-leaves.json
//! (native_component_leaves.py: every leaf run natively on fixture
//! components with the class's real vtables), replayed by
//! crates/aa-actors/tests/native_actor_creation_leaves.rs.
use crate::actor_creation::{Error, Result};
use crate::components::{ComponentSlot, StateBits};
use crate::init_create::FactoryKind;
use aa_numeric::{Binary, Precision};
use aa_simulation::NativeRng;
use std::cmp::Ordering;

/// The vtable entries of a constructed class (the native leaf addresses).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Leaves {
    pub init_create: u32,
    pub init_existing: u32,
    pub post_create: u32,
    pub vf14: u32,
    pub requirement: u32,
}

/// Primary vtable entries +4 / +8 / +C / +14 / +20 of each class.
pub fn leaves(kind: FactoryKind) -> Leaves {
    let (a, b, c, d, e) = match kind {
        FactoryKind::OrganizationMind => (0x438ac0, 0x51d1b0, 0x4d1e10, 0x4b2830, 0x7c53e0),
        FactoryKind::AircraftMind => (0x438ac0, 0x517c60, 0x4d1e10, 0x4b2830, 0x55ea50),
        FactoryKind::ElementMind => (0x438ac0, 0x529ec0, 0x4d1e10, 0x4b2830, 0x5b6830),
        FactoryKind::Standard(slot) => match slot.index() {
            0 => (0x562310, 0x562290, 0x4d1e10, 0x4b2830, 0x580f70),
            1 => (0x438ac0, 0x6a71d0, 0x4d1e10, 0x4b2830, 0x580f70),
            2 => (0x438ac0, 0x6a71d0, 0x4d1e10, 0x4b2830, 0x580f70),
            3 => (0x438ac0, 0x576700, 0x4d1e10, 0x4b2830, 0x580f70),
            4 => (0, 0, 0, 0, 0),
            5 => (0x57db90, 0x57e000, 0x4d1e10, 0x4b2830, 0x580f70),
            6 => (0x563fd0, 0x6a71d0, 0x581880, 0x4b2830, 0x580f70),
            7 => (0x563fd0, 0x57c870, 0x4d1e10, 0x579dd0, 0x580f70),
            8 => (0x563fd0, 0x5704d0, 0x4d1e10, 0x4b2830, 0x580f70),
            9 => (0x56f250, 0x56f280, 0x4d1e10, 0x4b2830, 0x580f70),
            10 => (0x568750, 0x5687e0, 0x4d1e10, 0x568070, 0x580f70),
            11 => (0x563fd0, 0x58b0e0, 0x4d1e10, 0x4b2830, 0x580f70),
            12 => (0x58ca40, 0x58cc40, 0x4d1e10, 0x58d3b0, 0x580f70),
            13 => (0x438ac0, 0x6a71d0, 0x4d1e10, 0x4b2830, 0x589090),
            14 => (0x586f70, 0x585940, 0x4d1e10, 0x4b2830, 0x580f70),
            15 => (0x563fd0, 0x564d80, 0x4d1e10, 0x4b2830, 0x4c9fa0),
            16 => (0x438ac0, 0x581f30, 0x4d1e10, 0x4b2830, 0x580f70),
            17 => (0x438ac0, 0x5717f0, 0x4d1e10, 0x4b2830, 0x581370),
            18 => (0x438ac0, 0x562e80, 0x4d1e10, 0x4b2830, 0x7c53e0),
            19 => (0x575ac0, 0x5757c0, 0x4d1e10, 0x4b2830, 0x580f70),
            20 => (0x57db90, 0x566db0, 0x4d1e10, 0x4b2830, 0x6b2450),
            21 => (0x438ac0, 0x6a71d0, 0x4d1e10, 0x4b2830, 0x580f70),
            22 => (0x563fd0, 0x564d80, 0x4d1e10, 0x56ec90, 0x580f70),
            23 => (0x438ac0, 0x55e600, 0x4d1e10, 0x4b2830, 0x6b2450),
            24 => (0x438ac0, 0x6a71d0, 0x4d1e10, 0x4b2830, 0x7c7840),
            25 => (0x563fd0, 0x564d80, 0x4d1e10, 0x4b2830, 0x580f70),
            26 => (0x571cc0, 0x571cd0, 0x4d1e10, 0x4b2830, 0x580f70),
            27 => (0x438ac0, 0x93e8e0, 0x4d1e10, 0x4b2830, 0x580f70),
            28 => (0x438ac0, 0x6a71d0, 0x4d1e10, 0x4b2830, 0x580f70),
            29 => (0x563fd0, 0x6a71d0, 0x564be0, 0x563fe0, 0x580f70),
            _ => (0x574610, 0x6a71d0, 0x4d1e10, 0x4b2830, 0x580f70),
        },
    };
    Leaves { init_create: a, init_existing: b, post_create: c, vf14: d, requirement: e }
}

/// The component slot a constructed class lives in.
pub fn slot_of(kind: FactoryKind) -> ComponentSlot {
    match kind {
        FactoryKind::Standard(slot) => slot,
        _ => ComponentSlot::Mind,
    }
}

/// Component virtual +20 constants: 580F70 (-1), 7C53E0 (4), 55EA50 (0x17),
/// 5B6830 (8), 4C9FA0 (0), 581370 (6), 6B2450 (5), 7C7840 (3); Structure's
/// 589090 calls 4D1050 on its owner and returns -1.
pub fn requirement_constant(kind: FactoryKind) -> Option<ComponentSlot> {
    let slot = match leaves(kind).requirement {
        0x7c53e0 => 4,
        0x55ea50 => 0x17,
        0x5b6830 => 8,
        0x4c9fa0 => 0,
        0x581370 => 6,
        0x6b2450 => 5,
        0x7c7840 => 3,
        _ => return None,
    };
    ComponentSlot::from_index(slot)
}

/// The creation-block fields the leaves read (InitCreate: +38, +40, +48;
/// PostCreate: +34).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LeafParams {
    pub word_34: u32,
    pub mode: u32,
    pub word_40: u32,
    pub word_48: u32,
}

/// Services of the component leaves. Component and definition fields are
/// native byte offsets; lists are returned in native order.
pub trait ComponentWorld {
    type Actor: Copy;
    /// GWorld +2C (map version), +D4 RNG (`4F02B0`), +E0.
    fn world_version(&self) -> u32;
    fn rng(&mut self) -> &mut NativeRng;
    fn begin_time_bits(&self) -> u32;
    /// Actor +8 flags, +100/+104 state, +60 slots, +20/+24 position,
    /// virtual +108 owner (the +E8 GTID, 0: null).
    fn actor_flags(&self, actor: Self::Actor) -> Result<u32>;
    fn state_bits(&self, actor: Self::Actor) -> Result<StateBits>;
    fn has_component(&self, actor: Self::Actor, slot: ComponentSlot) -> Result<bool>;
    fn position_xy(&self, actor: Self::Actor) -> Result<[u32; 2]>;
    fn owner(&self, actor: Self::Actor) -> Result<u32>;
    /// Component fields (native offsets).
    fn component_word(&self, actor: Self::Actor, slot: ComponentSlot, offset: u32) -> Result<u32>;
    fn set_component_word(&mut self, actor: Self::Actor, slot: ComponentSlot, offset: u32, value: u32) -> Result<()>;
    fn component_byte(&self, actor: Self::Actor, slot: ComponentSlot, offset: u32) -> Result<u8>;
    fn set_component_byte(&mut self, actor: Self::Actor, slot: ComponentSlot, offset: u32, value: u8) -> Result<()>;
    /// ActorData fields (`[component+4]+4`, `529C30`).
    fn definition_word(&self, actor: Self::Actor, offset: u32) -> Result<u32>;
    fn definition_byte(&self, actor: Self::Actor, offset: u32) -> Result<u8>;
    /// ActorData lists: +34C (node +8 values), +354 (node +4 values), +1D4
    /// (initial properties, node +0 values).
    fn definition_list(&self, actor: Self::Actor, offset: u32) -> Result<Vec<u32>>;
    /// ActorData +4EC list: per node, its object's +184 is non-null.
    fn recruit_targets(&self, actor: Self::Actor) -> Result<Vec<bool>>;
    /// SVar `UnitSupplyEnabled` (561B40, static B3D654).
    fn unit_supply_enabled(&self) -> Result<bool>;
    /// The component Updatable (+8) has a pending event (+C non-null).
    fn scheduled(&self, actor: Self::Actor, slot: ComponentSlot) -> Result<bool>;
    /// `50AC00(data)` (queue 0) / `50AC60(data)` (queue 1) on the Updatable.
    fn add_periodic(&mut self, actor: Self::Actor, slot: ComponentSlot, queue: u8, data: u32) -> Result<()>;
    /// `50ACC0(delay, data)`.
    fn add_after(&mut self, actor: Self::Actor, slot: ComponentSlot, delay_bits: u32, data: u32) -> Result<()>;
    /// `50B220(queue, data)`.
    fn has_scheduled(&mut self, actor: Self::Actor, slot: ComponentSlot, queue: u8, data: u32) -> Result<bool>;
    /// Actor zones: `4D69A0(index)` (aa_zones::component::create_zone),
    /// `+14C > index && [+148][index]` non-null, `515F40(x, y, radius)` and
    /// `515940(bit, on)` on zone slot `index`.
    fn create_zone(&mut self, actor: Self::Actor, index: u32) -> Result<bool>;
    fn zone_present(&self, actor: Self::Actor, index: u32) -> Result<bool>;
    fn move_zone(&mut self, actor: Self::Actor, index: u32, xy: [u32; 2], radius_bits: u32) -> Result<()>;
    fn zone_flag(&mut self, actor: Self::Actor, index: u32, bit: u32, on: u32) -> Result<()>;
    /// `[B39F58]+74` and the tile size (the `4BB000` radius).
    fn thing_manager_74_bits(&self) -> Result<u32>;
    fn tile_size_bits(&self) -> Result<u32>;
    /// `4D64F0(state, enabled)` on the actor.
    fn set_state(&mut self, actor: Self::Actor, state: u32, enabled: bool) -> Result<()>;
    /// `4D1050` on the actor.
    fn query_4d1050(&mut self, actor: Self::Actor) -> Result<bool>;
    /// `4D4CF0` on the actor.
    fn refresh_4d4cf0(&mut self, actor: Self::Actor) -> Result<()>;
    /// `4CF110` on the actor (modifier context).
    fn modifier_context(&mut self, actor: Self::Actor) -> Result<u32>;
    /// `404ED0` (Settlement.cpp 0x68) "Unable to create all wall
    /// placeholder for settlement: %s".
    fn log_settlement_walls(&mut self, actor: Self::Actor) -> Result<()>;
    // ---- per-class callees
    /// Body `561E50(requested, token, notify, source)`.
    fn body_set_health(&mut self, actor: Self::Actor, requested_bits: u32, token: u32, notify: u32, source: u32) -> Result<()>;
    /// `565070` on Construction (+9C): None for null.
    fn construction_565070(&mut self, actor: Self::Actor) -> Result<Option<u32>>;
    /// Body `561D40(context, value, 1)` (`4CF110`'s context, `565070`'s value).
    fn body_561d40(&mut self, actor: Self::Actor, context: u32, value: u32) -> Result<()>;
    /// Properties (+78) `581420(0)`.
    fn properties_581420(&mut self, actor: Self::Actor) -> Result<()>;
    /// Properties `5817A0(property, 0, 0)` (PostCreate initial properties).
    fn properties_add_initial(&mut self, actor: Self::Actor, property: u32) -> Result<()>;
    /// Denizen +18 inventory: per slot, the target definition's `45D730(0)`
    /// (ST0), stored at the slot +8.
    fn denizen_slot_count(&self, actor: Self::Actor) -> Result<usize>;
    fn denizen_target_health(&mut self, actor: Self::Actor, index: usize) -> Result<Binary>;
    fn set_denizen_slot_health(&mut self, actor: Self::Actor, index: usize, bits: u32) -> Result<()>;
    /// Settlement `586BC0(1)` (wall placeholders), `584CF0`.
    fn settlement_create_walls(&mut self, actor: Self::Actor) -> Result<bool>;
    fn settlement_584cf0(&mut self, actor: Self::Actor) -> Result<()>;
    /// Supply `58AE10(0)`.
    fn supply_58ae10(&mut self, actor: Self::Actor) -> Result<()>;
    /// Hq: Kingdom +234 non-null, `4E2640` (HQ index), the
    /// "hq_error_no_badge|s" message (`4B9F40`, `40C230(B399B0, 0x11, actor,
    /// owner)`).
    fn owner_hq_table(&self, owner: u32) -> Result<bool>;
    fn allocate_hq_index(&mut self, owner: u32) -> Result<i32>;
    fn hq_no_badge_message(&mut self, actor: Self::Actor, owner: u32) -> Result<()>;
    /// Performer `57DCF0(0)` (combat performer::gate_by_rank), `57E430`.
    fn performer_gate_by_rank(&mut self, actor: Self::Actor) -> Result<()>;
    fn performer_refresh(&mut self, actor: Self::Actor) -> Result<()>;
    /// Organization `57C450(definition +2F0)`, `57B390`, `577F30(flag)`
    /// (flag = +70 ? `[[+70]+C]+1A` : 0), members (+28 array, +2C count,
    /// non-null entries) and their Element `5709D0`.
    fn organization_set_definition_layout(&mut self, actor: Self::Actor) -> Result<()>;
    fn organization_57b390(&mut self, actor: Self::Actor) -> Result<()>;
    fn organization_70_flag(&self, actor: Self::Actor) -> Result<u8>;
    fn organization_577f30(&mut self, actor: Self::Actor, flag: u8) -> Result<()>;
    fn organization_members(&self, actor: Self::Actor) -> Result<Vec<Self::Actor>>;
    fn element_5709d0(&mut self, member: Self::Actor) -> Result<()>;
    /// ActorData getters through the modifier system (ST0 results):
    /// `45D5B0(context)` (Element), `45D4C0(0)` / `45D6C0(0)` (ranges).
    fn element_range(&mut self, actor: Self::Actor, context: u32) -> Result<Binary>;
    fn organization_range(&mut self, actor: Self::Actor) -> Result<Binary>;
    fn defender_range(&mut self, actor: Self::Actor) -> Result<Binary>;
    /// Experience `45DB90(value)` (rank of a value) / `5716B0(rank)`.
    fn experience_rank(&mut self, actor: Self::Actor, value: u32) -> Result<u32>;
    fn experience_set_rank(&mut self, actor: Self::Actor, rank: u32) -> Result<()>;
    /// Mover (+6C) +24 and `576B20(value, 1.0f)`.
    fn mover_24(&self, actor: Self::Actor) -> Result<u32>;
    fn mover_576b20(&mut self, actor: Self::Actor, value_bits: u32, scale_bits: u32) -> Result<()>;
    /// Mind InitExisting (`529EC0` / `517C60`, aa-mind).
    fn mind_init_existing(&mut self, actor: Self::Actor, kind: FactoryKind) -> Result<bool>;
    /// Wall InitExisting `58CC40` (grid cells under shape B, cell object
    /// virtual +5C(4)).
    fn wall_grid_init(&mut self, actor: Self::Actor) -> Result<()>;
    /// SVar GateZoneRadius (`4BE0D0` once, latch B3D6CC; `4BE0F0`).
    fn gate_zone_radius(&mut self) -> Result<u32>;
    /// Detection +14 (`56EC90`, aa_zones::component::activate).
    fn detection_activate(&mut self, actor: Self::Actor) -> Result<()>;
    /// `5644F0(actor, word, &City+10)` (City PostCreate).
    fn city_create_with_field(&mut self, actor: Self::Actor, word_34: u32) -> Result<()>;
}

fn f32b(bits: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}
/// `fcomp` greater-than (C0 and C3 clear; NaN is false).
fn greater_than_zero(bits: u32) -> bool {
    let v = f32::from_bits(bits);
    v > 0.0
}

///The services actually reached by Denizen568750. The full component adapter
///and the live arena use this same kernel without unrelated success stubs.
pub trait DenizenCreateWorld {
    type Actor:Copy;
    fn denizen_construction(&self,actor:Self::Actor)->Result<bool>;
    fn denizen_count(&self,actor:Self::Actor)->Result<usize>;
    fn denizen_health(&mut self,actor:Self::Actor,index:usize)->Result<Binary>;
    fn denizen_store_health(&mut self,actor:Self::Actor,index:usize,bits:u32)->Result<()>;
    fn denizen_guard_range(&self,actor:Self::Actor)->Result<u32>;
    fn denizen_inventory_head_present(&self,actor:Self::Actor)->Result<bool>;
    fn denizen_schedule(&mut self,actor:Self::Actor)->Result<()>;
    fn denizen_delay(&self,actor:Self::Actor)->Result<u32>;
    fn denizen_store_time(&mut self,actor:Self::Actor,bits:u32)->Result<()>;
}
impl<W:ComponentWorld> DenizenCreateWorld for W {
    type Actor=W::Actor;
    fn denizen_construction(&self,a:Self::Actor)->Result<bool>{Ok(self.state_bits(a)?.construction())}
    fn denizen_count(&self,a:Self::Actor)->Result<usize>{self.denizen_slot_count(a)}
    fn denizen_health(&mut self,a:Self::Actor,i:usize)->Result<Binary>{self.denizen_target_health(a,i)}
    fn denizen_store_health(&mut self,a:Self::Actor,i:usize,b:u32)->Result<()>{self.set_denizen_slot_health(a,i,b)}
    fn denizen_guard_range(&self,a:Self::Actor)->Result<u32>{self.definition_word(a,0x364)}
    fn denizen_inventory_head_present(&self,a:Self::Actor)->Result<bool>{Ok(self.definition_word(a,0x35c)?!=0)}
    fn denizen_schedule(&mut self,a:Self::Actor)->Result<()>{self.add_periodic(a,ComponentSlot::Denizen,1,0)}
    fn denizen_delay(&self,a:Self::Actor)->Result<u32>{self.definition_word(a,0x3a4)}
    fn denizen_store_time(&mut self,a:Self::Actor,b:u32)->Result<()>{self.set_component_word(a,ComponentSlot::Denizen,0x34,b)}
}
///Native568750, including committed slot writes before queue/final-time reads.
pub fn init_create_denizen<W:DenizenCreateWorld>(actor:W::Actor,params:Option<&LeafParams>,world:&mut W)->Result<bool>{
    if !world.denizen_construction(actor)? && let Some(p)=params && p.mode!=4 {
        for index in 0..world.denizen_count(actor)? {
            let health=world.denizen_health(actor,index)?.store_f32()?;
            world.denizen_store_health(actor,index,health)?;
        }
    }
    if greater_than_zero(world.denizen_guard_range(actor)?) || world.denizen_inventory_head_present(actor)? {
        world.denizen_schedule(actor)?;
    }
    //FLD quiets a signalling NaN before FCHS/FSTP flips its sign.
    let mut value=world.denizen_delay(actor)?;
    if value&0x7f80_0000==0x7f80_0000 && value&0x007f_ffff!=0 {value|=0x0040_0000;}
    world.denizen_store_time(actor,value^0x8000_0000)?;
    Ok(true)
}

/// Component virtual +20.
pub fn requirement<W: ComponentWorld>(kind: FactoryKind, actor: W::Actor, world: &mut W) -> Result<Option<ComponentSlot>> {
    if leaves(kind).requirement == 0x589090 {
        world.query_4d1050(actor)?;
    }
    Ok(requirement_constant(kind))
}

/// `4BB000` on `[B39F58]` (the discovery / settlement-space zone radius):
/// tile size + tile size + `[B39F58]+74` + 0.0001f in the x87 at `pc`,
/// stored as binary32 by the caller's `fstp dword`. The single port, shared
/// by actor creation (4D7230, 585940) and PostLoad (4CFF70).
pub fn discovery_zone_radius(tile_bits: u32, manager_74_bits: u32, pc: Precision) -> aa_numeric::Result<u32> {
    let tile = Binary::from_f32_bits(tile_bits)?;
    tile.add(&tile, pc)?
        .add(&Binary::from_f32_bits(manager_74_bits)?, pc)?
        .add(&Binary::from_f32_bits(0x38d1_b717)?, pc)?
        .store_f32()
}

fn settlement_zone_radius<W: ComponentWorld>(world: &W, pc: Precision) -> Result<u32> {
    Ok(discovery_zone_radius(world.tile_size_bits()?, world.thing_manager_74_bits()?, pc)?)
}

fn move_zone_here<W: ComponentWorld>(world: &mut W, actor: W::Actor, index: u32, radius_bits: u32) -> Result<()> {
    if !world.zone_present(actor, index)? {
        return Err(Error::NullDereference("515F40 on a null zone"));
    }
    let xy = world.position_xy(actor)?;
    world.move_zone(actor, index, xy, radius_bits)
}

/// Component virtual +4 (InitCreate).
pub fn init_create<W: ComponentWorld>(
    kind: FactoryKind,
    actor: W::Actor,
    params: Option<&LeafParams>,
    world: &mut W,
) -> Result<bool> {
    let slot = slot_of(kind);
    match leaves(kind).init_create {
        0x438ac0 => Ok(true),
        0x563fd0 => {
            world.add_periodic(actor, slot, 1, 0)?;
            Ok(true)
        }
        0x562310 => {
            if world.state_bits(actor)?.construction() {
                world.body_set_health(actor, 0x3f80_0000, 0, 1, 0)?;
            }
            if let Some(p) = params
                && body_supply_enabled(actor, world)?
                && p.word_40 == 0
                && p.word_48 == 0
                && !world.has_scheduled(actor, slot, 1, 0)?
            {
                world.add_periodic(actor, slot, 1, 0)?;
            }
            Ok(true)
        }
        0x57db90 => {
            if world.has_component(actor, ComponentSlot::Properties)? {
                world.properties_581420(actor)?;
            }
            Ok(true)
        }
        0x56f250 => {
            if world.definition_byte(actor, 0x348)? != 0 {
                world.add_periodic(actor, slot, 1, 0)?;
            }
            Ok(true)
        }
        0x568750 => init_create_denizen(actor,params,world),
        0x58ca40 => {
            if world.definition_byte(actor, 0x42c)? != 0 && world.definition_byte(actor, 0x42d)? != 0 {
                world.add_periodic(actor, slot, 1, 0)?;
            }
            Ok(true)
        }
        0x586f70 => {
            if !world.settlement_create_walls(actor)? {
                world.log_settlement_walls(actor)?;
                return Ok(false);
            }
            world.add_periodic(actor, slot, 1, 0)?;
            let count = world.component_word(actor, slot, 0x28)?;
            let draw = world.rng().next_u32();
            if count == 0 {
                return Err(Error::DivideByZero("586FCE rng % Settlement +28"));
            }
            world.set_component_word(actor, slot, 0x78, draw % count)?;
            let draw = world.rng().next_u32();
            world.set_component_byte(actor, slot, 0x7c, (draw & 1) as u8)?;
            Ok(true)
        }
        0x575ac0 => {
            let state = world.state_bits(actor)?;
            let disabled = world.definition_word(actor, 0x534)? != 0
                || state.low & (1 << 26) != 0
                || state.low & (1 << 25) != 0;
            world.set_state(actor, 0x22, !disabled)?;
            Ok(true)
        }
        0x571cc0 => {
            world.add_periodic(actor, slot, 0, 0)?;
            Ok(true)
        }
        0x574610 => {
            let owner = world.owner(actor)?;
            if world.component_word(actor, slot, 0x10)? as i32 == -1 && owner != 0 && world.owner_hq_table(owner)? {
                let index = world.allocate_hq_index(owner)?;
                world.set_component_word(actor, slot, 0x10, index as u32)?;
            }
            if owner != 0 && world.owner_hq_table(owner)? && world.component_word(actor, slot, 0x10)? as i32 == -1 {
                world.hq_no_badge_message(actor, owner)?;
                return Ok(false);
            }
            Ok(true)
        }
        other => Err(Error::Unsupported(leaf_name(other))),
    }
}

/// `561B40`: UnitSupplyEnabled, no Organization (+7C), ActorData +2A8 >
/// 0.0.
fn body_supply_enabled<W: ComponentWorld>(actor: W::Actor, world: &W) -> Result<bool> {
    if !world.unit_supply_enabled()? || world.has_component(actor, ComponentSlot::Organization)? {
        return Ok(false);
    }
    Ok(greater_than_zero(world.definition_word(actor, 0x2a8)?))
}

fn leaf_name(address: u32) -> &'static str {
    match address {
        0 => "slot 4 has no Standard class (a Mind kind is required)",
        _ => "component leaf outside the 4CF480 classes",
    }
}

/// Map-version gate of the InitExisting reschedules: GWorld +2C < 0x16 and
/// no pending event.
///The exact services reached by Recruit581F30. Targets are immutable source
///identities; classification and each store are performed in native order.
pub trait RecruitInitWorld {
    type Actor:Copy;
    type Target;
    fn recruit_init_targets(&mut self,actor:Self::Actor)->Result<Vec<Self::Target>>;
    fn recruit_init_location(&mut self,target:&Self::Target)->Result<bool>;
    fn recruit_init_store(&mut self,actor:Self::Actor,offset:u32,value:u8)->Result<()>;
}
impl<W:ComponentWorld> RecruitInitWorld for W {
    type Actor=W::Actor;type Target=bool;
    fn recruit_init_targets(&mut self,a:Self::Actor)->Result<Vec<bool>>{self.recruit_targets(a)}
    fn recruit_init_location(&mut self,t:&bool)->Result<bool>{Ok(*t)}
    fn recruit_init_store(&mut self,a:Self::Actor,o:u32,v:u8)->Result<()>{self.set_component_byte(a,ComponentSlot::Recruit,o,v)}
}
pub fn init_recruit<W:RecruitInitWorld>(actor:W::Actor,world:&mut W)->Result<bool>{
    for target in world.recruit_init_targets(actor)?{
        let offset=if world.recruit_init_location(&target)?{0x10}else{0x11};
        world.recruit_init_store(actor,offset,1)?;
    }Ok(true)
}
///5757C0 reads4D1050 first, captures554 once, schedules on the SAME
///embedded Updatable, then reads current WorldE0 and stores captured delay+E0.
pub trait MobileInitWorld {
    type Actor:Copy;
    fn mobile_init_active(&mut self,actor:Self::Actor)->Result<bool>;
    fn mobile_init_delay(&mut self,actor:Self::Actor)->Result<u32>;
    fn mobile_init_schedule(&mut self,actor:Self::Actor,delay:u32)->Result<()>;
    fn mobile_init_begin(&mut self)->Result<u32>;
    fn mobile_init_store(&mut self,actor:Self::Actor,value:u32)->Result<()>;
}
impl<W:ComponentWorld> MobileInitWorld for W {
    type Actor=W::Actor;
    fn mobile_init_active(&mut self,a:Self::Actor)->Result<bool>{self.query_4d1050(a)}
    fn mobile_init_delay(&mut self,a:Self::Actor)->Result<u32>{self.definition_word(a,0x554)}
    fn mobile_init_schedule(&mut self,a:Self::Actor,d:u32)->Result<()>{self.add_after(a,ComponentSlot::MobileDeployment,d,0)}
    fn mobile_init_begin(&mut self)->Result<u32>{Ok(self.begin_time_bits())}
    fn mobile_init_store(&mut self,a:Self::Actor,v:u32)->Result<()>{self.set_component_word(a,ComponentSlot::MobileDeployment,0x14,v)}
}
pub fn init_mobile_deployment<W:MobileInitWorld>(actor:W::Actor,world:&mut W,pc:Precision)->Result<bool>{
    if world.mobile_init_active(actor)?{
        let delay=world.mobile_init_delay(actor)?;
        world.mobile_init_schedule(actor,delay)?;
        let at=f32b(delay)?.add(&f32b(world.mobile_init_begin()?)?,pc)?.store_f32()?;
        world.mobile_init_store(actor,at)?;
    }Ok(true)
}
fn old_map_unscheduled<W: ComponentWorld>(world: &W, actor: W::Actor, slot: ComponentSlot) -> Result<bool> {
    Ok(world.world_version() < 0x16 && !world.scheduled(actor, slot)?)
}

/// Component virtual +8 (InitExisting).
#[path = "defender_init.rs"]
pub mod defender;
struct Defender<'a, W: ComponentWorld> { actor: W::Actor, world: &'a mut W }
impl<W: ComponentWorld> defender::World for Defender<'_, W> {
    fn zone_present(&mut self)->Result<bool>{self.world.zone_present(self.actor,4)}
    fn definition_mask(&mut self)->Result<u32>{self.world.definition_word(self.actor,0x174)}
    fn range(&mut self,organization:bool)->Result<u32>{
        let value=if organization{self.world.organization_range(self.actor)?}else{self.world.defender_range(self.actor)?};
        Ok(value.store_f32()?)
    }
    fn store_range(&mut self,bits:u32)->Result<()>{self.world.set_component_word(self.actor,ComponentSlot::Defender,0x2c,bits)}
    fn create_zone(&mut self)->Result<bool>{self.world.create_zone(self.actor,4)}
    fn aircraft_present(&mut self)->Result<bool>{self.world.has_component(self.actor,ComponentSlot::Aircraft)}
    fn aircraft_zone_flag(&mut self)->Result<()>{self.world.zone_flag(self.actor,4,1,1)}
    fn position(&mut self)->Result<[u32;2]>{self.world.position_xy(self.actor)}
    fn move_zone(&mut self,position:[u32;2],range:u32)->Result<()>{self.world.move_zone(self.actor,4,position,range)}
    fn performer_present(&mut self)->Result<bool>{self.world.has_component(self.actor,ComponentSlot::Performer)}
    fn refresh_performer(&mut self)->Result<()>{self.world.performer_refresh(self.actor)}
}
pub fn init_existing<W: ComponentWorld>(kind: FactoryKind, actor: W::Actor, world: &mut W, pc: Precision) -> Result<bool> {
    let slot = slot_of(kind);
    match leaves(kind).init_existing {
        0x6a71d0 => Ok(true),
        0x562290 => {
            if !world.state_bits(actor)?.construction() {
                let health = world.component_word(actor, slot, 0x10)?;
                let maximum = world.component_word(actor, slot, 0x14)?;
                // fucompp; test ah,44; jnp: only exactly equal skips (NaN
                // differs).
                let differs = f32::from_bits(health) != f32::from_bits(maximum);
                if world.has_component(actor, ComponentSlot::Construction)?
                    && let Some(value) = world.construction_565070(actor)?
                {
                    let context = world.modifier_context(actor)?;
                    world.body_561d40(actor, context, value)?;
                }
                if differs {
                    // 5622E9: both fields are read again after 561D40.
                    let health = world.component_word(actor, slot, 0x10)?;
                    let maximum = world.component_word(actor, slot, 0x14)?;
                    world.set_component_word(actor, slot, 0x10, maximum)?;
                    world.body_set_health(actor, health, 0, 1, 0)?;
                }
            }
            Ok(true)
        }
        0x576700 => {
            if world.component_byte(actor, slot, 0x56)? != 0 {
                if old_map_unscheduled(world, actor, slot)? {
                    world.add_periodic(actor, slot, 0, 0)?;
                }
                world.set_state(actor, 0x18, true)?;
            }
            Ok(true)
        }
        0x51d1b0 | 0x517c60 | 0x529ec0 => world.mind_init_existing(actor, kind),
        0x57e000 => {
            world.performer_gate_by_rank(actor)?;
            Ok(true)
        }
        0x57c870 => {
            if world.component_word(actor, slot, 0x2c)? == 0 {
                world.organization_set_definition_layout(actor)?;
            }
            world.refresh_4d4cf0(actor)?;
            world.organization_57b390(actor)?;
            let flag = world.organization_70_flag(actor)?;
            world.organization_577f30(actor, flag)?;
            if old_map_unscheduled(world, actor, slot)? {
                world.add_periodic(actor, slot, 1, 0)?;
            }
            Ok(true)
        }
        0x5704d0 => {
            let context = world.modifier_context(actor)?;
            let range = world.element_range(actor, context)?.store_f32()?;
            world.set_component_word(actor, slot, 0x20, range)?;
            if old_map_unscheduled(world, actor, slot)? {
                world.add_periodic(actor, slot, 1, 0)?;
            }
            Ok(true)
        }
        0x56f280 => {
            if !world.scheduled(actor, slot)? && world.definition_byte(actor, 0x348)? != 0 {
                world.add_periodic(actor, slot, 1, 0)?;
            }
            Ok(true)
        }
        0x5687e0 => {
            let mut mask = world.component_word(actor, slot, 0x38)?;
            for v in world.definition_list(actor, 0x34c)? {
                mask |= v;
            }
            for v in world.definition_list(actor, 0x354)? {
                mask |= v;
            }
            world.set_component_word(actor, slot, 0x38, mask)?;
            if old_map_unscheduled(world, actor, slot)?
                && (greater_than_zero(world.definition_word(actor, 0x364)?)
                    || world.definition_word(actor, 0x35c)? != 0)
            {
                world.add_periodic(actor, slot, 1, 0)?;
            }
            Ok(true)
        }
        0x58b0e0 => {
            if !world.scheduled(actor, slot)? {
                world.add_periodic(actor, slot, 1, 0)?;
            }
            if !world.zone_present(actor, 0)? {
                world.create_zone(actor, 0)?;
            }
            world.supply_58ae10(actor)?;
            Ok(true)
        }
        0x58cc40 => {
            world.wall_grid_init(actor)?;
            Ok(true)
        }
        0x585940 => {
            if world.create_zone(actor, 2)? {
                let radius = settlement_zone_radius(world, pc)?;
                move_zone_here(world, actor, 2, radius)?;
            }
            world.settlement_584cf0(actor)?;
            if old_map_unscheduled(world, actor, slot)? {
                world.add_periodic(actor, slot, 1, 0)?;
            }
            Ok(true)
        }
        0x564d80 => {
            if old_map_unscheduled(world, actor, slot)? {
                world.add_periodic(actor, slot, 1, 0)?;
            }
            Ok(true)
        }
        0x581f30 => init_recruit(actor,world),
        0x5717f0 => {
            let value = world.component_word(actor, slot, 0x8)?;
            let rank = world.experience_rank(actor, value)?;
            if world.component_word(actor, slot, 0xc)? != rank {
                world.experience_set_rank(actor, rank)?;
            }
            Ok(true)
        }
        0x562e80 => {
            world.refresh_4d4cf0(actor)?;
            Ok(true)
        }
        0x5757c0 => init_mobile_deployment(actor,world,pc),
        0x566db0 => defender::initialize(&mut Defender{actor,world}),
        0x55e600 => {
            let mut v = world.mover_24(actor)?;
            // fcomp 0.1f; test ah,5; jp: replace when less (ordered).
            if f32::from_bits(v) < f32::from_bits(0x3dcc_cccd) {
                v = 0x3dcc_cccd;
            }
            world.mover_576b20(actor, v, 0x3f80_0000)?;
            Ok(true)
        }
        0x571cd0 => {
            if old_map_unscheduled(world, actor, slot)? {
                world.add_periodic(actor, slot, 0, 0)?;
            }
            Ok(true)
        }
        0x93e8e0 => {
            world.add_periodic(actor, slot, 0, 0)?;
            Ok(true)
        }
        other => Err(Error::Unsupported(leaf_name(other))),
    }
}

/// Component virtual +C (PostCreate(params, flag)).
pub fn post_create<W: ComponentWorld>(
    kind: FactoryKind,
    actor: W::Actor,
    params: &LeafParams,
    flag: u32,
    world: &mut W,
) -> Result<()> {
    match leaves(kind).post_create {
        0x4d1e10 => Ok(()),
        0x581880 => {
            for property in world.definition_list(actor, 0x1d4)? {
                world.properties_add_initial(actor, property)?;
            }
            Ok(())
        }
        0x564be0 => {
            if flag as u8 == 0 {
                world.city_create_with_field(actor, params.word_34)?;
            }
            Ok(())
        }
        other => Err(Error::Unsupported(leaf_name(other))),
    }
}

///563FE0's granular services. The held owner can bind these without supplying
///unrelated component callbacks. Reads remain after the reached CreateZone8.
pub trait CityAfterWorld {
    fn create_zone8(&mut self) -> Result<bool>;
    fn radius_5b4(&mut self) -> Result<u32>;
    fn position_xy(&mut self) -> Result<[u32; 2]>;
    fn move_zone8(&mut self, xy: [u32; 2], radius: u32) -> Result<()>;
}
pub fn city_after_init(world: &mut impl CityAfterWorld) -> Result<()> {
    if world.create_zone8()? {
        let radius = world.radius_5b4()?;
        let xy = world.position_xy()?;
        world.move_zone8(xy, radius)?;
    }
    Ok(())
}
struct CityAfter<'a, W: ComponentWorld> { world: &'a mut W, actor: W::Actor }
impl<W: ComponentWorld> CityAfterWorld for CityAfter<'_, W> {
    fn create_zone8(&mut self) -> Result<bool> { self.world.create_zone(self.actor, 8) }
    fn radius_5b4(&mut self) -> Result<u32> { self.world.definition_word(self.actor, 0x5b4) }
    fn position_xy(&mut self) -> Result<[u32; 2]> { self.world.position_xy(self.actor) }
    fn move_zone8(&mut self, xy: [u32; 2], radius: u32) -> Result<()> { self.world.move_zone(self.actor, 8, xy, radius) }
}

/// Component virtual +14.
pub fn vf14<W: ComponentWorld>(kind: FactoryKind, actor: W::Actor, world: &mut W) -> Result<()> {
    let slot = slot_of(kind);
    match leaves(kind).vf14 {
        0x4b2830 => Ok(()),
        0x579dd0 => {
            if !world.zone_present(actor, 4)? {
                let range = world.organization_range(actor)?.store_f32()?;
                world.set_component_word(actor, slot, 0xac, range)?;
                if world.create_zone(actor, 4)? {
                    let radius = world.component_word(actor, slot, 0xac)?;
                    move_zone_here(world, actor, 4, radius)?;
                }
            }
            for member in world.organization_members(actor)? {
                world.element_5709d0(member)?;
            }
            Ok(())
        }
        0x568070 => {
            let range = world.definition_word(actor, 0x364)?;
            if f32::from_bits(range) == 0.0 || world.actor_flags(actor)? & 1 != 0 {
                return Ok(());
            }
            world.create_zone(actor, 5)?;
            if world.component_word(actor, slot, 0x38)? & 0x20 != 0 {
                if !world.zone_present(actor, 5)? {
                    return Err(Error::NullDereference("515940 on a null zone"));
                }
                world.zone_flag(actor, 5, 1, 1)?;
            }
            move_zone_here(world, actor, 5, range)
        }
        0x58d3b0 => {
            if world.definition_byte(actor, 0x42c)? != 0
                && world.definition_byte(actor, 0x42d)? != 0
                && world.create_zone(actor, 3)?
            {
                let radius = world.gate_zone_radius()?;
                move_zone_here(world, actor, 3, radius)?;
            }
            Ok(())
        }
        0x56ec90 => world.detection_activate(actor),
        0x563fe0 => city_after_init(&mut CityAfter { world, actor }),
        other => Err(Error::Unsupported(leaf_name(other))),
    }
}

/// Compare helper used by tests: `Ordering` of two binary32 values as x87
/// `fcomp` sees them (None: unordered).
pub fn fcomp(a: u32, b: u32) -> Option<Ordering> {
    f32::from_bits(a).partial_cmp(&f32::from_bits(b))
}
