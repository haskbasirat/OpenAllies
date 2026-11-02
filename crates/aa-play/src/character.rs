//! The Character component in the simulation. Owner: core.
//!
//! A Character decides ticks, not only pictures: its default stance draws idle
//! repetitions and arms a deferred event `repetitions * length` later, and that
//! event draws a fidget and arms again with the fidget's length. So the
//! animation lengths live in the simulation (`Def::character`), and aa-bevy's
//! view copy in `battle_view` stays a view.
//!
//! Native, one thing at a time:
//! - `actor+0xE4` is the active component. 0x4CFC10 makes a component active:
//!   the old one's vf+0x40 runs, then the new one's vf+0x3C. For a Character
//!   both are 0x4DB080: its events leave the queue (0x50B250). For a Mover vf+0x40
//!   is 0x576B60. [`mover_started`] and [`mover_stopped`] are those two sides.
//! - 0x563BA0 default stance (reached through 0x4CFC80 with a Character): does
//!   nothing when it is active, playing idle or fidget and has an event; else it
//!   activates, takes the idle of the current set with its repetitions (getter
//!   0x563C59, then 0x49B4D0) or, with a Performer busy with an ability, that
//!   ability's idle (no draw), sets mode 0 and arms data 0.
//! - 0x563CA0 fidget: the same activation; with `CharacterFidget` and no ability it
//!   takes a fidget (getter 0x563D12, then 0x45EB70 -> 0x45DA40: no draw for a single
//!   one, `draw % n` with no weights, else weighted), mode 1, arms data 1. Else 0x563BA0.
//! - 0x563E10 the deferred update, data 0: 0x563CA0; data 1: 0x563BA0.
//! - 0x4D6FE0 at creation: 0x4CFC80 (`created`).
//!
//! What stands in for native state, labelled:
//! - Performer +0x34 and its active test: `Actor::combat.target` and the first
//!   weapon's idle. The combat lane holds the real ability; no film reaches this
//!   before the fighting is exact.
//! - 0x4D03D0 (Properties +0x1C bit 1, the stun of a special operation) has no
//!   flag in aa-play: nothing sets it yet, so the 0.0001 s re-arm is not taken.
//! - The kinds 3 (die) and 5 (summon) of 0x563D50 and their data 2, 3, 5: 0x563D50
//!   needs record +0x51, which no recruit in a film sets. Death cancels through
//!   the `alive` test here, the corpse timer is combat's.
use crate::data::{AnimSet, CharacterDef};
use crate::scheduler::{DEFERRED, Owner};
use crate::trace::sites;
use crate::world::{ActorId, World};

/// aa-rules COMPONENTS slot of the Character.
pub const SLOT: u8 = 2;
/// The 0.0001 s of 0x563E10's re-arm (0x38D1B717).
const STUNNED_REARM: u32 = 0x38d1_b717;

/// Component +0x28 and whether `actor+0xE4` is this Character.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CharacterState {
    pub active: bool,
    pub mode: u32,
}

fn def_of(world: &World, a: ActorId) -> Option<&CharacterDef> {
    world.data.def(world.actors.get(a as usize)?.def).character.as_ref()
}

/// 0x4D0720: the set of the actor's stance (Actor +0x15C through 0x45E5E0), else
/// of its variant (Object +0x40, -1 for an actor no aspect was drawn for),
/// through 0x45E690: the last, base set for -1 or an aspect without a set.
fn current_set<'a>(world: &World, a: ActorId, def: &'a CharacterDef) -> &'a AnimSet {
    let actor = &world.actors[a as usize];
    let aspect = stance_aspect(world, actor);
    aspect.and_then(|id| def.aspects.get(id)).unwrap_or(&def.base)
}

/// 0x45E620 (0x4D0050 / 0x4D16B0 keep it current): the last stance of the list
/// whose health and actor states apply.
fn stance_aspect<'a>(world: &'a World, actor: &crate::world::Actor) -> Option<&'a str> {
    let stances = &world.data.def(actor.def).stances;
    if stances.is_empty() {
        return None;
    }
    let max = world.combat.actor(actor.id).map_or(0.0, |c| c.max_health);
    let max = if max > 0.0 { max } else { world.data.def(actor.def).health.max(1.0) };
    let health = actor.health / max;
    let org = actor.org.and_then(|o| world.combat.org(o));
    let state = |s: &str| match s {
        // The organization's Mind state (actor_states_aa.tgi group 4).
        "mind_engaged" => org.is_some_and(|o| o.routing.is_none() && !o.exhausted && o.engaging()),
        _ => false,
    };
    stances.iter().rev().find(|s| health <= s.health + 1e-6 && (s.states.is_empty() || s.states.iter().any(|x| state(x))))?.aspect.as_deref()
}

/// The ability idle's length when the Performer is busy (0x4D0750 -> 0x4C81B0).
/// A rendition: see the module documentation.
fn busy_ability_idle(world: &World, a: ActorId) -> Option<f32> {
    let actor = &world.actors[a as usize];
    actor.combat.target?;
    world.data.def(actor.def).weapons.first().map(|w| w.idle_length).filter(|l| *l > 0.0)
}

/// 0x4CFC10(Character): vf+0x38. The Character's own events leave (vf+0x3C
/// 0x4DB080); a dead actor refuses (Thing flag 1 with vf+0x44 false).
fn activate(world: &mut World, a: ActorId) -> bool {
    if !world.actors[a as usize].alive {
        return false;
    }
    world.unschedule(Owner::Component(a, SLOT));
    world.books.actor_mut(a).character.active = true;
    true
}

/// 0x563BA0. `site` names the caller's chain in the census.
pub fn default_stance_at(world: &mut World, a: ActorId, site: &'static str) {
    let Some(def) = def_of(world, a).cloned() else { return };
    let state = world.books.actor(a).map_or(CharacterState::default(), |b| b.character);
    if state.active && matches!(state.mode, 0 | 1) && world.is_scheduled(Owner::Component(a, SLOT)) {
        return;
    }
    if !activate(world, a) {
        return;
    }
    let delay = match busy_ability_idle(world, a) {
        Some(length) => length,
        None => {
            let set = current_set(world, a, &def);
            let length = set.first_idle;
            // 0x49B4D0: `draw % (max - min) + min`, the width unsigned.
            let draw = world.random_u32_at(site);
            let [min, max] = def.idle_loops;
            let width = max.wrapping_sub(min) as u32;
            let reps = if width == 0 { min } else { (draw % width).wrapping_add(min as u32) as i32 };
            (f64::from(reps) * f64::from(length)) as f32
        }
    };
    world.books.actor_mut(a).character.mode = 0;
    world.schedule_after(DEFERRED, Owner::Component(a, SLOT), 0, delay);
}

/// 0x563CA0.
fn fidget(world: &mut World, a: ActorId) {
    let Some(def) = def_of(world, a).cloned() else { return };
    if !activate(world, a) {
        return;
    }
    let enabled = world.data.rule_ci("CharacterFidget").is_none_or(|v| v != 0.0);
    if busy_ability_idle(world, a).is_none() && enabled {
        let set = current_set(world, a, &def);
        let total = set.fidget_weight;
        let list = set.fidgets.clone();
        // The getter always; 0x45DA40 draws only with more than one to choose.
        world.trace.log(crate::trace::SKind::RngGetter(sites::CHARACTER_TICK_FIDGET), world.native_rng.state);
        let chosen = pick(&list, total, || world.native_rng.next_u32());
        if let Some(length) = chosen {
            world.books.actor_mut(a).character.mode = 1;
            world.schedule_after(DEFERRED, Owner::Component(a, SLOT), 1, length);
            return;
        }
    }
    default_stance_at(world, a, sites::CHARACTER_TICK_STANCE);
}

/// 0x45DA40 over (length, weight): the only one with no draw; `draw % n` when the
/// total weight is 0; else the first whose weight takes the running remainder to
/// zero or below, in binary64 (`fild`, the 2^32 fix, `* K * total`, then `- weight`).
/// `None`: an empty list (the original goes on with a null animation; aa-play takes
/// the stance instead, no shipped Character lacks a fidget: 0x410C40 requires it).
fn pick(list: &[(f32, f32)], total: f32, mut draw: impl FnMut() -> u32) -> Option<f32> {
    match list.len() {
        0 => None,
        1 => Some(list[0].0),
        n => {
            let v = draw();
            if total == 0.0 {
                return Some(list[(v % n as u32) as usize].0);
            }
            let mut left = crate::trace::native_unit_f64(v) * f64::from(total);
            for &(length, weight) in list {
                left -= f64::from(weight);
                if left <= 0.0 {
                    return Some(length);
                }
            }
            None
        }
    }
}

/// 0x563E10 with `data`, the event's word.
pub fn deliver(world: &mut World, a: ActorId, data: u32) {
    let Some(actor) = world.actors.get(a as usize) else { return };
    if !actor.alive || actor.removed {
        return;
    }
    let state = world.books.actor(a).map_or(CharacterState::default(), |b| b.character);
    if !state.active || state.mode != data {
        return;
    }
    if stunned(world, a) {
        world.schedule_after(DEFERRED, Owner::Component(a, SLOT), data, f32::from_bits(STUNNED_REARM));
        return;
    }
    match data {
        0 => {
            // The Performer test of 0x563E10: an active Performer skips the fidget.
            fidget(world, a)
        }
        1 | 2 | 5 => default_stance_at(world, a, sites::CHARACTER_TICK_STANCE),
        _ => {}
    }
}

/// 0x4D03D0: Properties +0x1C bit 1. See the module documentation.
fn stunned(_world: &World, _a: ActorId) -> bool {
    false
}

/// 0x4CFC80 at creation (0x4D6FE0), between the Mind jitter and the trailer.
pub fn created(world: &mut World, a: ActorId) {
    default_stance_at(world, a, sites::CHARACTER_CREATION);
}

/// 0x4CFC80 when a Mover stops, for an actor with a Character: 0x563BA0. The Mover's
/// own leave 0x576B60 stays with the movement code. An actor without a Character
/// does nothing here.
pub fn mover_stopped(world: &mut World, a: ActorId) {
    default_stance_at(world, a, sites::CHARACTER_NODE);
}

/// [`mover_stopped`] from a Mind state, for the census's naming.
pub fn mover_stopped_at(world: &mut World, a: ActorId, site: &'static str) {
    default_stance_at(world, a, site);
}

/// 0x4CFC10(Mover) from 0x576B20: the Character stops being active and its events
/// leave the queue (vf+0x40 0x4DB080). Whether a pending one is there does not
/// matter; the mode stays until the next activation sets it.
pub fn mover_started(world: &mut World, a: ActorId) {
    if def_of(world, a).is_none() {
        return;
    }
    world.unschedule(Owner::Component(a, SLOT));
    world.books.actor_mut(a).character.active = false;
}

/// Whether `a` plays its Character: `actor+0xE4` is the Character.
pub fn active(world: &World, a: ActorId) -> bool {
    world.books.actor(a).is_some_and(|b| b.character.active)
}
