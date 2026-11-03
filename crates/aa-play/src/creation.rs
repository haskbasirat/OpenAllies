//! The world-RNG draws of thing creation and of the battle start, in the
//! original's order. Owner: core.
//!
//! Every new thing (KKC_GWorld::CreateGThing 0x4F9C10) draws, as the films
//! show (measured live on Berlin by the parity harness):
//! 1. its random scale (GObject InitCreate 0x4E9FC0 -> 0x49B440);
//! 2. an element's mind constructor jitter (0x4CF480 -> 0x535BA0 -> 0x530E50;
//!    not for an aircraft or organization Mind), then a Character's default
//!    stance (0x4D6FE0 -> 0x4CFC80 -> 0x563BA0, `crate::character`);
//! 3. the hitched trailer its GActor InitCreate creates (0x4D70F2: the
//!    trailer is a new thing with these same draws);
//! 4. the component InitCreates in slot order (0x4D6F10): a lone Body that bleeds
//!    out of supply registers on queue 1 (0x562310, 0x561B40: a ship), every component
//!    whose InitCreate is 0x563FD0 (Properties, Organization, Element,
//!    Supply, Construction, Detection, Lair, City) registers on the periodic
//!    queue 1 (0x50AC60 -> 0x50B2C0, one draw for its bucket), the Economy
//!    component (0x56F250) when its definition requires supply (+0x348), the
//!    HQ component (0x574610 -> 0x4E2640);
//! 5. a mover's registration tail (0x4F9F88 -> 0x4D7520 -> 0x50AB10).
//! A thing restored from the map draws instead at its InitExisting: one
//! draw per DynamicDoodad component (Disturbable 0x5904A0, Breakable
//! 0x58F960) and one for a Supply component registering on queue 1
//! (0x58B0E0; the queue Load 0x50B8B0 that follows replaces the queue with
//! the saved lists, so that entry does not survive). Each draw is a GetRNG
//! 0x4F02B0 getter plus one 0x85D840 draw.
//!
//! The queue-1 registrations are real entries of `World::sched`
//! (`crate::scheduler`): the bucket is the draw % 16, inserted at the head.
//! The other draws register nothing: the HQ InitCreate's 0x4E2640 picks the
//! kingdom's random free HQ number (draw % kingdom+0x234+0x40 count, list
//! +0x36C) and 0x50AB10 a random countdown phase (-(draw % rule +0x28));
//! aa-play keeps neither value.
use crate::data::{Def, DefId};
use crate::scheduler::{Owner, EVERY_SECOND};
use crate::trace::sites;
use crate::world::{ActorId, OrgId, World};

/// Component slots (aa-rules COMPONENTS bits) whose InitCreate is 0x563FD0.
const QUEUE_ONE: [u32; 8] = [6, 7, 8, 11, 15, 22, 25, 29];
const ECONOMY: u32 = 9;
const HQ: u32 = 30;
const ELEMENT: u32 = 8;
const MOVER: u32 = 3;
const SUPPLY: u32 = 11;
const DENIZEN: u32 = 10;
const ORGANIZATION: u32 = 7;
const AIRCRAFT: u32 = 23;
const CHARACTER: u32 = 2;
const BODY: u32 = 0;

fn has(d: &Def, bit: u32) -> bool {
    d.components & (1 << bit) != 0
}

/// The thing a creation makes, which owns its components' registrations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Made {
    Actor(ActorId),
    Org(OrgId),
    /// A thing aa-play does not keep apart (an actor's hitched trailer).
    Other,
}

impl Made {
    fn component(self, slot: u32) -> Option<Owner> {
        match self {
            Made::Actor(a) => Some(Owner::Component(a, slot as u8)),
            Made::Org(o) => Some(Owner::OrgComponent(o, slot as u8)),
            Made::Other => None,
        }
    }
}

/// The components of `d` that register on queue 1 at InitCreate, slot
/// order: the 0x563FD0 ones and the Economy component when the definition
/// requires supply (0x56F250, element +0x348).
pub fn queue_one_slots(d: &Def) -> impl Iterator<Item = u32> + '_ {
    (0..31).filter(move |&bit| has(d, bit) && (QUEUE_ONE.contains(&bit) || (bit == ECONOMY && d.supply_required)))
}

/// The draws and registrations of creating a thing of `def` (0x4F9C10) as
/// `made`; an actor's trailer then takes its own GTID.
pub fn created(world: &mut World, def: DefId, made: Made) {
    let data = world.data.clone();
    let d = data.def(def);
    // 1. 0x49B440 random scale.
    world.random_u32_at(sites::RANDOM_SCALE);
    // 2. 0x530E50 mind jitter: only an element Mind draws (0x4CF480 case 4
    // takes ActorData+0x174 bit 7, then 23, then 8: an organization makes a
    // regiment Mind and an aircraft an aircraft Mind, neither drawing).
    if has(d, ELEMENT) && !has(d, ORGANIZATION) && !has(d, AIRCRAFT) {
        let word = world.random_u32_at(sites::MIND_JITTER);
        if let Made::Actor(a) = made {
            world.books.actor_mut(a).ability_jitter = crate::combat::mind_jitter(word);
        }
    }
    // 0x4D6FE0 -> 0x4CFC80: a Character starts its default stance.
    if let Made::Actor(a) = made
        && has(d, CHARACTER)
    {
        crate::character::created(world, a);
    }
    // 3. The hitched trailer (a thing of its own).
    if let Some(t) = d.trailer.filter(|&t| t != def) {
        if let Made::Actor(a) = made {
            world.ids.created_trailer(a);
        }
        created(world, t, Made::Other);
    }
    // 4. The component InitCreates, slot order. An in-place unpack (state bit 26)
    // assembles only what `aa_actors` `assemble_initial` allows; the rest comes when
    // the unpacking ends ([`unpack_end`]).
    let unpacking = world.ids.unpack_scope;
    for bit in 0..31 {
        if !has(d, bit) || (unpacking && !assembled_while_unpacking(bit)) {
            continue;
        }
        if bit == BODY {
            // 0x562310 with a record (record +0x40 and +0x48 are 0 for a lone
            // thing; no element with a shortfall rate is ever an organization's).
            if body_registers(world, d) {
                match made.component(bit) {
                    Some(owner) => world.schedule_periodic_at(sites::BODY_INIT, EVERY_SECOND, owner, 0),
                    None => {
                        world.random_u32_at(sites::BODY_INIT);
                    }
                }
            }
        } else if QUEUE_ONE.contains(&bit) || (bit == ECONOMY && d.supply_required) {
            let site = if bit == ECONOMY { sites::ECONOMY_INIT } else { sites::COMPONENT_INIT };
            match made.component(bit) {
                Some(owner) => world.schedule_periodic_at(site, EVERY_SECOND, owner, 0),
                None => {
                    world.random_u32_at(site);
                }
            }
        } else if bit == HQ {
            world.random_u32_at(sites::HQ_NUMBER);
        } else if bit == DENIZEN {
            // 0x568750 (with the InitExisting 0x5687E0 a created actor also
            // runs): queue 1 when the guard range is positive or the
            // inventory is not empty.
            let registers = match made {
                Made::Actor(a) => crate::denizen::created(world, a),
                _ => d.denizen.as_ref().is_some_and(crate::denizen::registers),
            };
            if registers {
                match made.component(bit) {
                    Some(owner) => world.schedule_periodic_at(sites::COMPONENT_INIT, EVERY_SECOND, owner, 0),
                    None => {
                        world.random_u32_at(sites::COMPONENT_INIT);
                    }
                }
            }
        }
    }
    // 5. 0x50AB10, from 0x4D7450 only when a TrackInfo is CONTINUOUS (the Berlin
    // film: trucks and trailers draw it, infantry do not).
    if has(d, MOVER) && d.continuous_tracks {
        world.random_u32_at(sites::MOVER_COUNTDOWN);
    }
    // The AircraftComponent (constructor 0x55E8E0: definition speed
    // 0x45D6F0(0), turn radius +0x234) and the aircraft Mind (0x4CF480
    // case 4); neither draws.
    if let Made::Actor(a) = made
        && has(d, AIRCRAFT)
    {
        let flight = aa_movement::aircraft::AircraftState::new(d.speed_max, d.turn_radius);
        world.books.actor_mut(a).aircraft = Some(crate::books::AircraftBooks { flight, hidden: false, denizen: false, scale: 1.0, mana: d.mana_max });
        crate::combat::air::created(world, a);
        // The Mind's InitExisting 0x517C60 (4D7230 runs it for a created
        // actor): aai_queued.
        crate::combat::air::init_existing(world, a);
    }
}

/// Whether 0x4D21A0 makes component `bit` of a thing in state bit 26, an in-place
/// unpack (`aa_actors` `assemble_initial`): Construction, Body, Properties, Wall,
/// Structure, Detection, Element, Economy, MobileDeployment and City, as far as the
/// definition has them.
fn assembled_while_unpacking(bit: u32) -> bool {
    matches!(bit, BODY | 6 | ELEMENT | ECONOMY | 12 | 13 | 15 | 19 | 22 | 29)
}

/// The deferred end of an unpacking, 0x576319 -> 0x4D21A0(1): the components left out
/// at creation are made, and each runs its InitCreate in slot order (0x4D3068, the
/// Berlin film's Supply; a depot's Supply and Lair, two draws at (2v2)Kursk 1352 and
/// (4FFA)Farmhouses 1209). InitExisting follows, as for any made component.
pub fn unpack_end(world: &mut World, a: ActorId) {
    let data = world.data.clone();
    let d = data.def(world.actors[a as usize].def);
    for bit in 0..31 {
        if !has(d, bit) || assembled_while_unpacking(bit) {
            continue;
        }
        if QUEUE_ONE.contains(&bit) {
            let owner = Owner::Component(a, bit as u8);
            world.schedule_periodic_at(sites::UNPACK_END, EVERY_SECOND, owner, 0);
        } else if bit == HQ {
            world.random_u32_at(sites::HQ_NUMBER);
        } else if bit == DENIZEN && crate::denizen::created(world, a) {
            world.schedule_periodic_at(sites::UNPACK_END, EVERY_SECOND, Owner::Component(a, bit as u8), 0);
        }
    }
    if has(d, SUPPLY) {
        crate::supply::supplier_tick(world, a, 0.0);
    }
}

/// 0x4C8CB0: an ability's projectile is a thing of its own. The next-fit id
/// (0x4F9C10), the draws of its creation, then its launch 0x580BC0, whose 0x50AC00(0)
/// registers it on queue 0. Measured on the fight films: every projectile of every
/// definition is these three getters at 4E9FF8, 50B2E4 (from 0x563FD0) and 50B2E4
/// (from 0x580BE1). The caller gives the id back at impact.
pub fn projectile(world: &mut World, def: DefId) -> Option<crate::ids::Gtid> {
    let id = world.ids.created_transient(def);
    created(world, def, Made::Other);
    world.random_u32_at(sites::PROJECTILE_LAUNCH);
    id
}

/// 0x4F9C10 of a transport (0x582426 for a recruit at a location, 0x4CB376 for a summon):
/// the next-fit id, the draws of its creation, then its InitExisting's Mover 0x55E632 ->
/// 0x576B20, one getter on queue 0. Measured on the fight films, every transport of
/// every nation (`rus_transport_air`, Berlin2v2_1 3841 and 13681). aa-play keeps no actor
/// for it: the flight is the time to the drop. The caller gives the id back at the drop;
/// natively it flies on and leaves the map, and a next-fit id is not taken again by then.
pub fn transport(world: &mut World, def: DefId) -> Option<crate::ids::Gtid> {
    let id = world.ids.created_transient(def);
    created(world, def, Made::Other);
    if has(world.data.def(def), AIRCRAFT) {
        world.random_u32_at(sites::INIT_EXISTING_AIRCRAFT);
    }
    id
}

/// What a transport drops starts to fall (end of 0x58BCB0: 0x5770A0 -> 0x50AC00, one
/// getter on queue 0, after the thing's creation draws). aa-play places it on the ground.
pub fn cargo_fall(world: &mut World) {
    world.random_u32_at(sites::CARGO_FALL);
}

/// 0x561B40: UnitSupplyEnabled, no OrganizationComponent, ActorData +0x2A8 above 0.
fn body_registers(world: &World, d: &Def) -> bool {
    world.data.rule_ci("UnitSupplyEnabled").is_some_and(|v| v != 0.0) && !has(d, ORGANIZATION) && d.unit_shortfall_damage_rate > 0.0
}

/// The draws of restoring a map thing of `def` (its InitExisting).
pub fn restored(world: &mut World, def: DefId) {
    let data = world.data.clone();
    let d = data.def(def);
    for _ in 0..d.doodad_components {
        world.random_u32_at(sites::INIT_EXISTING_DOODAD);
    }
    if has(d, SUPPLY) {
        world.random_u32_at(sites::INIT_EXISTING_SUPPLY);
    }
}

/// A map record on its own in the record pass: the Load of a saved Mind constructs it
/// (0x4D8290 -> 0x4D833E -> 0x4CF480, the element's jitter 0x530E50; the eight cows of
/// (2v2)Market_Garden), then its InitExisting ([`restored`]).
pub fn restored_thing(world: &mut World, def: DefId, placed: &crate::map::PlacedThing) {
    let data = world.data.clone();
    let d = data.def(def);
    if placed.mind.is_some() && has(d, ELEMENT) && !has(d, ORGANIZATION) && !has(d, AIRCRAFT) {
        world.random_u32_at(sites::MIND_JITTER);
    }
    restored(world, def);
}

/// The draws a saved game's record makes in the world load's record pass (4FAE80 on a
/// saved stream, measured on native traces of films of saved games), before its
/// publish 4FB514: the Load's Mind jitter, then the InitExisting 0x4D7230: its
/// components in slot order, the Economy 0x56F280 when the definition requires supply,
/// the Supply 0x58B0E0, each DynamicDoodad, an aircraft's mover 0x576B20, and the
/// mover's countdown 0x50AB10.
pub fn saved_record(world: &mut World, def: DefId) {
    let data = world.data.clone();
    let d = data.def(def);
    if has(d, ELEMENT) && !has(d, ORGANIZATION) && !has(d, AIRCRAFT) {
        world.random_u32_at(sites::MIND_JITTER);
    }
    if has(d, ECONOMY) && d.supply_required {
        world.random_u32_at(sites::ECONOMY_INIT);
    }
    if has(d, SUPPLY) {
        world.random_u32_at(sites::INIT_EXISTING_SUPPLY);
    }
    for _ in 0..d.doodad_components {
        world.random_u32_at(sites::INIT_EXISTING_DOODAD);
    }
    if has(d, AIRCRAFT) {
        world.random_u32_at(sites::INIT_EXISTING_AIRCRAFT);
    }
    if has(d, MOVER) && d.continuous_tracks {
        world.random_u32_at(sites::MOVER_COUNTDOWN);
    }
}

/// A placed organization's turn in the world load's record pass: the Load of each
/// embedded member (57D130 -> 4FC200 -> 4FAE80), whose Mind constructor draws its
/// jitter (0x4D833E -> 0x4CF614 -> 0x535BA0 -> 0x530E50), then the organization's
/// state change 0x4D4AAC -> 0x57C400 -> 0x57BFF0 -> 0x57BEF0: with a formation that is
/// not `organized` each position without a member scatters, two draws (0x5797E0:
/// 0x5798BB, 0x5798CD). The primary slot and the leader's are the origin, no draw.
/// Measured on (2v2)Kursk, the four horses of a seven-slot herd and three pairs.
pub fn restored_organization(world: &mut World, def: DefId, org: &crate::map::PlacedOrganization) {
    let data = world.data.clone();
    for &(_, m) in &org.members {
        let Some(member) = world.map.placed.get(m).and_then(|p| data.find(&p.thing_id)) else { continue };
        let md = data.def(member);
        if has(md, ELEMENT) && !has(md, ORGANIZATION) && !has(md, AIRCRAFT) {
            world.random_u32_at(sites::MIND_JITTER);
        }
    }
    let d = data.def(def);
    let organized = usize::try_from(org.formation).ok().and_then(|f| d.formations.get(f)).is_none_or(|f| f.organized);
    if !organized {
        let leader = usize::try_from(org.leader).ok();
        for slot in 0..d.slots.len() {
            let held = org.members.iter().any(|&(s, _)| s == slot as i32);
            if held || Some(slot) == leader || Some(slot as u16) == d.leader_slot {
                continue;
            }
            world.random_u32_at(sites::ORGANIZATION_SCATTER);
            world.random_u32_at(sites::ORGANIZATION_SCATTER);
        }
    }
    restored(world, def);
}

/// `0x57BEF0` on a formation that is not `organized`: ComputeSlotPosition `0x5797E0`
/// for every slot (`aa_movement::formation::compute_slot_position`). The leader's and
/// the primary slot are the origin; an attached member's offset is where it stands from
/// the leader; a detached one keeps its offset; an empty slot takes a table direction
/// and `4 * unit + 0.5`, two getters (`0x444A90`, `0x45D400`). The Kursk film: a
/// seven-slot herd of four, three pairs at each turn of the organization (0x57BF30),
/// and each member's next goal is the wander point plus where it stood. `None` for an
/// `organized` formation, which draws nothing: its table is
/// `crate::formation::organization_slots`.
pub(crate) fn organization_scatter(world: &mut World, org: OrgId, facing: glam::Vec2, standing: &[Option<(ActorId, glam::Vec2, bool)>], leader: Option<(ActorId, usize)>, previous: &[glam::Vec2]) -> Option<Vec<glam::Vec2>> {
    use aa_movement::formation as native;
    struct Draws<'a>(&'a mut World);
    impl native::FormationWorld for Draws<'_> {
        fn next_random_u32(&mut self) -> aa_movement::Result<u32> {
            Ok(self.0.random_u32_at(sites::ORGANIZATION_SCATTER))
        }
        fn mind_leader_changed(&mut self) -> aa_movement::Result<()> {
            Ok(())
        }
        fn detached_member(&mut self, _: u32, _: u32, _: [f32; 2]) -> aa_movement::Result<()> {
            Ok(())
        }
    }
    let data = world.data.clone();
    let d = data.def(world.orgs[org as usize].def);
    let formation = crate::player_orders::formation(world, org).and_then(|f| d.formations.get(f))?;
    if formation.organized {
        return None;
    }
    let slots = standing.len();
    let member = |slot: usize, id: ActorId, pos: glam::Vec2, attached: bool| native::Member { id, position: [pos.x, pos.y], slot: slot as i32, attached };
    let mut organization = native::Organization {
        owner_id: 0,
        facing: [facing.x, facing.y],
        owner_has_mind: false,
        center: [0.0, 0.0],
        primary_slot: d.leader_slot.map_or(-1, i32::from),
        group_sizes: Vec::new(),
        slot_count: slots as i32,
        members: standing.iter().enumerate().map(|(s, m)| m.map(|(id, pos, attached)| member(s, id, pos, attached))).collect(),
        leader: leader.and_then(|(id, s)| standing.get(s).copied().flatten().map(|(_, pos, attached)| member(s, id, pos, attached))),
        formation: Some(native::FormationShape { oriented: false, offsets: Vec::new() }),
        slot_positions: (0..slots).map(|s| previous.get(s).map_or([0.0; 2], |v| [v.x, v.y])).collect(),
    };
    let mut draws = Draws(world);
    for slot in 0..slots as i32 {
        // Only an attached member without a leader errs: a native null read.
        let _ = native::compute_slot_position(&mut organization, slot, aa_numeric::Precision::Double53, &mut draws);
    }
    Some(organization.slot_positions.iter().map(|p| glam::Vec2::new(p[0], p[1])).collect())
}

/// Whether placed thing `p` of `def` is re-created at the end of the map load
/// (`aa_mapworld::class_conversion::check_class`): saved as an Object and now of
/// another class. It has no InitExisting during the record load.
pub fn is_converted(p: &crate::map::PlacedThing, d: &Def) -> bool {
    p.class_code != 0 && aa_mapworld::class_conversion::check_class(p.class_code, d.runtime_class, true) == aa_mapworld::class_conversion::ClassCheck::Convert
}

/// The last step of the map load, 0x4FD47C (`replace_converted`) for a DynamicDoodad:
/// 0x4F9C10 with an A25B1C block, the saved id requested, in the record order of
/// the conversion vector. The draws, measured on (2v2)Comrades and (4FFA)Farmhouses:
/// the constructor's facing (0x4EF2A0, random_orientation and not axis_locked), the
/// InitCreate's scale 0x4E9FC0, each component's InitCreate 0x4DCBDD, then its
/// InitExisting 0x4DC730. A component's 0x5904A0 arms `u * update_time`; a
/// Breakable's 0x58F960 then cancels itself in state 0 (0x50B250).
pub fn converted(world: &mut World, def: DefId, index: u32, saved_variant: i32) {
    let data = world.data.clone();
    let d = data.def(def);
    if d.random_orientation && !d.axis_locked {
        world.random_u32_at(sites::THING_FACING);
    }
    // 0x4EA5E0: `creation_variant` keeps a saved variant; only a -1 draws. No
    // record of a shipped map reaches this.
    if d.variations_enabled && saved_variant == -1 {
        world.random_u32_at(sites::THING_VARIANT);
    }
    world.random_u32_at(sites::RANDOM_SCALE);
    for _ in 0..2 {
        init_components(world, d, index);
    }
}

/// A DynamicDoodad's InitCreate or InitExisting in a generated world
/// (`crate::scheduler::replay_generated`).
pub(crate) fn generated_doodad(world: &mut World, index: u32) {
    let data = world.data.clone();
    let Some(def) = world.map.placed.get(index as usize).and_then(|p| data.find(&p.thing_id)) else { return };
    init_components(world, data.def(def), index);
}

/// 0x5904A0 and 0x58F960 over a doodad's components, slot order.
fn init_components(world: &mut World, d: &Def, index: u32) {
    let Some(doodad) = d.doodad.as_ref() else { return };
    if let Some(c) = doodad.disturbable {
        let delay = arm_delay(world, c.update_time);
        world.schedule_after(crate::scheduler::DEFERRED, Owner::Doodad(index, 0), 0, delay);
    }
    if let Some(c) = doodad.breakable {
        let delay = arm_delay(world, c.update_time);
        world.schedule_after(crate::scheduler::DEFERRED, Owner::Doodad(index, 1), 0, delay);
        // A fresh Breakable is in state 0: 0x58F960's 0x50B250.
        world.unschedule(Owner::Doodad(index, 1));
    }
}

/// 0x5904A0's delay: `fild`, the 2^32 fix, `fmul` K, `fmul` update_time (PC53).
fn arm_delay(world: &mut World, update_time: f32) -> f32 {
    let draw = world.random_u32_at(sites::INIT_EXISTING_DOODAD);
    (crate::trace::native_unit_f64(draw) * f64::from(update_time)) as f32
}

/// One player kingdom's random picks of WorldParameters 0x50E8B0: the nation
/// (`do { draw } while (!selectable)` over the nation table, `draw % count`;
/// the table is in id order) and the general (0x474B40: the nation's
/// eligible factions, `draw % n`, no draw for a single one).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StartPick {
    pub random_nation: bool,
    pub random_general: bool,
}

/// A side of the map no player took: WorldParameters 0x50E8B0 resolves it in its turn
/// all the same, then destroys it (`crate::ids::NativeIds::destroy_map_kingdom`). Measured
/// on a film of (2v2)Berlin with one player: four kingdoms' picks in the trace.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ClosedSide {
    /// Its map slot, for the turn.
    pub slot: usize,
    /// The nation the creator holds; empty for random.
    pub nation: String,
    pub pick: StartPick,
}

/// The nation 0x50E8B0 picks at random.
pub fn random_nation(world: &mut World) -> Option<String> {
    let mut nations: Vec<String> = world.data.nations.iter().map(|n| n.id.clone()).collect();
    nations.sort();
    if nations.is_empty() {
        return None;
    }
    let v = world.random_u32_at(sites::NATION_PICK);
    Some(nations[(v % nations.len() as u32) as usize].clone())
}

/// The general 0x474B40 picks at random for `nation`.
pub fn random_general(world: &mut World, nation: &str) -> Option<String> {
    let eligible = world.data.nation(nation)?.eligible_factions.clone();
    match eligible.len() {
        0 => None,
        1 => Some(eligible[0].clone()),
        n => {
            let v = world.random_u32_at(sites::FACTION_PICK);
            Some(eligible[(v % n as u32) as usize].clone())
        }
    }
}
