//! `KKC_CAIStateGuard` (regiment Mind +17C, vtable 0xA32BC8): guard an actor or
//! a point. A rendition of the Think 0x5294F0 (exact port with fixtures:
//! `aa_mind::cai_guard`) over aa-play's orders.
//!
//! Activation 0x51DC80 (a thing) and 0x51DD30 (a point); re-issuing the same
//! guard only reschedules (0.6 s). The stand point of a guarded thing is
//! 0x51EE20: the thing's position plus the direction (`aux`, else from the thing
//! to the regiment, else the regiment's facing) times its radius plus half the
//! regiment's +2FC radius; it moves only when it differs by more than 0.0002 in
//! `|dx| + |dy|` (0x51D510).
//!
//! The Think: the guarded thing dead or the regiment following something ends
//! the state (Mind +140); the threat list is the denizen's intruders, else the
//! thing's +118 list (who is engaged with it), else what it attacks (its Mind
//! +90). The nearest threat wins, the current target biased by
//! `GuardOldTargetBias`. A threat in bombard range is bombarded (0x527CE0 case 3),
//! another is killed (0x51D920); no threat: walk to the stand point and sleep.
//!
//! Compare against the original: a denizen's threat list is read as the hostile
//! organizations inside its guard range (`intruders`), as `crate::denizen` has no
//! TickGuard 0x56A7E0; the home (Mind +28 / +30) of sallying militia has no host;
//! the settled check 0x585160 reads as false.
use super::guess::bombard_layout;
use super::{MindState, hostile_to};
use crate::combat::{CombatState, SLEEP_THINK_SECONDS, TargetRef};
use crate::world::{ActorId, AbilityTarget, Order, OrgId, Unit, World};
use glam::Vec2;

/// 0x3F19999A: the reschedule when the same guard is ordered again.
const REISSUE_SECONDS: f32 = 0.6;
/// A32B58: the stand point changed.
const MOVED: f32 = 0.0002;
/// A329E4: squared length below which `aux` is no direction.
const NO_DIRECTION: f32 = 0.0001;

/// The Guard state.
#[derive(Clone, Debug)]
pub struct Guard {
    /// State +18: the guarded thing.
    guarded: Option<Unit>,
    /// State +1C / +20: where it stands.
    point: Vec2,
    /// State +24 / +28.
    aux: Vec2,
    /// State +14: the move to the stand point was ordered.
    placed: bool,
    next_think: f32,
}

/// 0x51DC80 / 0x51DD30; false when refused.
pub(super) fn begin(world: &mut World, o: OrgId, target: Option<Unit>, at: Vec2, aux: Option<Vec2>) -> bool {
    let now = world.time;
    let aux = aux.unwrap_or(Vec2::ZERO);
    if let Some(MindState::Guard(g)) = world.orders.orgs.get_mut(&o)
        && g.guarded == target
        && (target.is_some() || g.point == at)
    {
        g.aux = aux;
        g.next_think = g.next_think.min(now + REISSUE_SECONDS);
        return true;
    }
    let point = match target {
        Some(t) => {
            // 0x51DC80: the target must not be this regiment (0x529C30).
            if t == Unit::Org(o) {
                return false;
            }
            let Some(p) = position(world, t) else { return false };
            p
        }
        None => at,
    };
    // 52A810(2) from any state of priority <= 2 (0x529FC0).
    if world.combat.org(o).is_some_and(|c| c.routing.is_some() || c.exhausted_state) {
        return false;
    }
    if let Some(c) = world.combat.orgs.get_mut(o as usize) {
        c.task = None;
        c.task_order = None;
    }
    world.orders.orgs.insert(o, MindState::Guard(Guard { guarded: target, point, aux, placed: false, next_think: now }));
    true
}

pub(super) fn position(world: &World, t: Unit) -> Option<Vec2> {
    match t {
        Unit::Actor(a) => world.actor(a).filter(|x| x.alive).map(|x| x.pos.truncate()),
        Unit::Org(o) => {
            let org = world.orgs.get(o as usize).filter(|x| x.alive)?;
            Some(world.combat.org(o).filter(|c| c.initialized).map_or(org.anchor, |c| c.position))
        }
    }
}

/// The shape's +4: a circle's radius, half a box's shorter side.
fn shape_radius(d: &crate::data::Def) -> f32 {
    let Some(s) = d.blocking_size else { return 0.0 };
    match d.blocking_shape.as_deref().map(str::to_ascii_uppercase).as_deref() {
        Some("CIRCLE") => s[0].max(0.0),
        Some(_) => 0.5 * s[0].min(s[1]).max(0.0),
        None => 0.0,
    }
}

/// Who has `guarded` on an engage list (Actor +118).
fn attackers(world: &World, st: &CombatState, guarded: Unit) -> Vec<OrgId> {
    let org = match guarded {
        Unit::Org(o) => Some(o),
        Unit::Actor(a) => world.actors.get(a as usize).and_then(|x| x.org),
    };
    let actor = match guarded {
        Unit::Actor(a) => Some(a),
        Unit::Org(_) => None,
    };
    let mut out = Vec::new();
    for (i, c) in st.orgs.iter().enumerate() {
        if !world.orgs.get(i).is_some_and(|x| x.alive) {
            continue;
        }
        if c.engage.iter().any(|t| match t.target {
            TargetRef::Org(x) => Some(x) == org,
            TargetRef::Actor(x) => Some(x) == actor,
        }) {
            out.push(i as OrgId);
        }
    }
    out
}

/// 0x569300 of a guarded denizen: its intruders. OnIntruder 0x568D80 adds a
/// hostile thing's organization as it enters the guard zone, TickGuard 0x56A7E0
/// prunes it as it leaves; both are read here as the organizations inside the
/// guard range now. `None` for a thing without a DenizenComponent.
fn intruders(world: &World, st: &CombatState, guarded: Unit) -> Option<Vec<OrgId>> {
    let Unit::Actor(a) = guarded else { return None };
    let x = world.actors.get(a as usize)?;
    let dd = world.data.def(x.def).denizen.as_ref()?;
    let at = x.pos.truncate();
    let r2 = dd.guard_range * dd.guard_range;
    let mut out = Vec::new();
    for (i, c) in st.orgs.iter().enumerate() {
        let Some(org) = world.orgs.get(i).filter(|o| o.alive) else { continue };
        if crate::setup::relation(world, x.kingdom, Some(org.kingdom)) == crate::books::ENEMY && c.position.distance_squared(at) < r2 {
            out.push(i as OrgId);
        }
    }
    Some(out)
}

/// The guarded thing's own target (Mind vt+90).
fn own_target(world: &World, st: &CombatState, guarded: Unit) -> Option<ActorId> {
    match guarded {
        Unit::Org(o) => st.orgs.get(o as usize)?.current_target().and_then(|t| crate::combat::representative(world, t)),
        Unit::Actor(a) => {
            let t = world.actors.get(a as usize)?.combat.target?;
            crate::combat::alive(world, t).then_some(t)
        }
    }
}

/// 0x51EE20: the stand point of a guarded thing; true when it moved.
fn replace(world: &World, st: &CombatState, o: OrgId, g: &mut Guard) -> bool {
    let Some(guarded) = g.guarded else { return false };
    let Some(at) = position(world, guarded) else { return false };
    let own = st.orgs[o as usize].position;
    let direction = if g.aux.length_squared() <= NO_DIRECTION {
        let d = own - at;
        if d.length() == 0.0 {
            // Organization +2C / +30, its facing: the formation heading.
            let h = world.orgs[o as usize].heading;
            Vec2::new(h.cos(), h.sin())
        } else {
            d.normalize()
        }
    } else {
        g.aux
    };
    let own_radius = world.data.def(world.orgs[o as usize].def).control_range;
    // Thing vt+0 = 0x4D1E30: the shape's radius.
    let shape = match guarded {
        Unit::Actor(a) => world.actors.get(a as usize).map(|x| x.def),
        Unit::Org(g) => world.orgs.get(g as usize).map(|x| x.def),
    };
    let radius = shape.map_or(0.0, |d| shape_radius(world.data.def(d)));
    let p = at + direction * (own_radius * 0.5 + radius);
    if (p.x - g.point.x).abs() + (p.y - g.point.y).abs() <= MOVED {
        return false;
    }
    g.point = p;
    true
}

/// 0x5294F0.
pub(super) fn think(world: &mut World, st: &mut CombatState, o: OrgId) {
    let now = world.time;
    let i = o as usize;
    let Some(MindState::Guard(g)) = world.orders.orgs.get(&o) else { return };
    if now < g.next_think {
        return;
    }
    let mut g = g.clone();
    g.next_think = now + SLEEP_THINK_SECONDS;
    let kingdom = world.orgs[i].kingdom;
    if let Some(guarded) = g.guarded {
        // Step 1: a guarded thing that is gone ends the state (Mind +140).
        if position(world, guarded).is_none() {
            world.orders.orgs.remove(&o);
            if world.orgs[i].order != Order::Idle {
                world.orgs[i].order = Order::Idle;
            }
            return;
        }
        // Step 3: the threat list.
        let list = intruders(world, st, guarded).unwrap_or_else(|| attackers(world, st, guarded));
        let current = match world.orgs[i].order {
            Order::Attack { target } => Some(target),
            _ => None,
        };
        let mut best: Option<ActorId> = None;
        let mut best_score = f32::MAX;
        let own = st.orgs[i].position;
        let bias = world.data.rule_ci("GuardOldTargetBias").unwrap_or(0.0);
        for &e in &list {
            let Some(rep) = crate::ids::target_actor(world, Unit::Org(e)) else { continue };
            if !hostile_to(world, kingdom, rep) {
                continue;
            }
            let at = st.orgs[e as usize].position;
            let d2 = at.distance_squared(own);
            let is_current = current.is_some_and(|c| world.actors[c as usize].org == Some(e));
            let score = if is_current {
                let v = (d2.sqrt() - bias).max(0.0);
                v * v
            } else {
                d2
            };
            if score < best_score {
                best_score = score;
                best = Some(rep);
            }
        }
        if list.is_empty() {
            best = own_target(world, st, guarded).filter(|&t| hostile_to(world, kingdom, t));
        }
        if let Some(t) = best {
            let same = current.is_some_and(|c| c == t || (world.actors[c as usize].org.is_some() && world.actors[c as usize].org == world.actors[t as usize].org));
            if !same {
                engage(world, st, o, t);
            }
            g.placed = false;
            world.orders.orgs.insert(o, MindState::Guard(g));
            return;
        }
        // The threat is gone: a kill or bombard of it ends (0x527CE0 leaves them).
        if current.is_some() {
            world.orgs[i].order = Order::Idle;
            g.placed = false;
        }
        if replace(world, st, o, &mut g) {
            g.placed = false;
        }
    }
    // Step 4: 0x527CE0(2, 0) once, then the move runs; else the sleep state.
    if !g.placed {
        g.placed = true;
        if st.orgs[i].position.distance_squared(g.point) > 0.0 {
            world.orgs[i].order = Order::Move { to: g.point, attack_move: false };
        }
    }
    world.orders.orgs.insert(o, MindState::Guard(g));
}

/// 0x527CE0(3, t) when in bombard range (`4D4FF0(t, 3, 0) == 0`), else
/// 0x51D920(0, t): the kill state.
fn engage(world: &mut World, st: &mut CombatState, o: OrgId, t: ActorId) {
    let i = o as usize;
    let at = world.actors[t as usize].pos.truncate();
    let in_range = bombard_layout(world, o) && world.orgs[i].members.iter().any(|&m| crate::combat::alive(world, m) && crate::combat::tasks::point_relation(world, m, at) == 0);
    if in_range {
        crate::combat::tasks::begin_guard_bombard(world, st, o, AbilityTarget::Actor(t));
    } else {
        world.orgs[i].order = Order::Attack { target: t };
    }
}
