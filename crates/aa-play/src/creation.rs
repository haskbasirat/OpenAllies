//! The world-RNG draws of thing creation and of the battle start, in the
//! original's order. Owner: core.
//!
//! Every new thing (KKC_GWorld::CreateGThing 0x4F9C10) draws, as the films
//! show (measured live on Berlin by the parity harness):
//! 1. its random scale (GObject InitCreate 0x4E9FC0 -> 0x49B440);
//! 2. an element's mind constructor jitter (0x4CF480 -> 0x535BA0 -> 0x530E50);
//! 3. the hitched trailer its GActor InitCreate creates (0x4D70F2: the
//!    trailer is a new thing with these same draws);
//! 4. the component InitCreates in slot order (0x4D6F10): every component
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
use crate::world::{ActorId, OrgId, World};

/// Component slots (aa-rules COMPONENTS bits) whose InitCreate is 0x563FD0.
const QUEUE_ONE: [u32; 8] = [6, 7, 8, 11, 15, 22, 25, 29];
const ECONOMY: u32 = 9;
const HQ: u32 = 30;
const ELEMENT: u32 = 8;
const MOVER: u32 = 3;
const SUPPLY: u32 = 11;
const DENIZEN: u32 = 10;

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
    world.random_u32();
    // 2. 0x530E50 mind jitter.
    if has(d, ELEMENT) {
        world.random_u32();
    }
    // 3. The hitched trailer (a thing of its own).
    if let Some(t) = d.trailer.filter(|&t| t != def) {
        if let Made::Actor(a) = made {
            world.ids.created_trailer(a);
        }
        created(world, t, Made::Other);
    }
    // 4. The component InitCreates, slot order. The in-place unpack's Supply
    // registers when the unpacking ends (the Berlin film: 0x576319 ->
    // 0x4D3068 -> 0x563FD0 from a deferred delivery, `crate::deploy`).
    let unpacking = world.ids.unpack_scope;
    for bit in 0..31 {
        if !has(d, bit) || (unpacking && bit == SUPPLY) {
            continue;
        }
        if QUEUE_ONE.contains(&bit) || (bit == ECONOMY && d.supply_required) {
            match made.component(bit) {
                Some(owner) => world.schedule_periodic(EVERY_SECOND, owner, 0),
                None => {
                    world.random_u32();
                }
            }
        } else if bit == HQ {
            world.random_u32();
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
                    Some(owner) => world.schedule_periodic(EVERY_SECOND, owner, 0),
                    None => {
                        world.random_u32();
                    }
                }
            }
        }
    }
    // 5. 0x50AB10.
    if has(d, MOVER) {
        world.random_u32();
    }
}

/// The draws of restoring a map thing of `def` (its InitExisting).
pub fn restored(world: &mut World, def: DefId) {
    let data = world.data.clone();
    let d = data.def(def);
    for _ in 0..d.doodad_components {
        world.random_u32();
    }
    if has(d, SUPPLY) {
        world.random_u32();
    }
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

/// The nation 0x50E8B0 picks at random.
pub fn random_nation(world: &mut World) -> Option<String> {
    let mut nations: Vec<String> = world.data.nations.iter().map(|n| n.id.clone()).collect();
    nations.sort();
    if nations.is_empty() {
        return None;
    }
    let v = world.random_u32();
    Some(nations[(v % nations.len() as u32) as usize].clone())
}

/// The general 0x474B40 picks at random for `nation`.
pub fn random_general(world: &mut World, nation: &str) -> Option<String> {
    let eligible = world.data.nation(nation)?.eligible_factions.clone();
    match eligible.len() {
        0 => None,
        1 => Some(eligible[0].clone()),
        n => {
            let v = world.random_u32();
            Some(eligible[(v % n as u32) as usize].clone())
        }
    }
}
