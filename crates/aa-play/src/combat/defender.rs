//! The DefenderComponent (actor `+0xB0`, slot 20, vtables 0xA3BBD4 / 0xA3BBAC):
//! the guns of a building, a corps or a bunker. Owner: combat.
//!
//! It acts on its ZOC being entered, not on a scan:
//!
//! * `0x567840`, reached from `0x514DA0` -> `0x514E40` when an enemy takes a tile
//!   of the zone, keeps the intruder on the `+0x1C` list if an ability could use
//!   it (`0x5673B0`), and with the list's first intruder the Updatable joins
//!   queue 0 (`0x50AC00(0)`, one draw) with a deferred update in one second
//!   (`0x50ACC0(1.0, 0)`).
//! * The deferred `0x567BF0` (Updatable `+0x18`, one second) drops the intruders
//!   out of the zone (virtual `+0x130(zone 4, 1)`) or out of reach (`0x5673B0`).
//!   An empty list clears every ability's target (`0x567140(ability, 0)`) and
//!   leaves the queues (`0x50B250`); otherwise it re-arms.
//! * The Tick `0x567960` (queue 0, every update) goes over the abilities: a target
//!   that went away or is refused (`0x4C8F20` other than busy, codes 7 and 8) is
//!   cleared; one that is missing or poses no threat (`0x566D40` below 2) is
//!   chosen again over the intruders (`0x567530`: the highest `0x566D40`, then
//!   the nearest, members standing for an organization); then the ability is
//!   used, after its turret has turned to it (`0x4C81D0`, `0x57E730`).
//! * `0x567140` writes the ability's target and, through `0x4D3990`, the
//!   defender (its organization if it has one, `0x4CE6C0`) on the `+0x118` of the
//!   target's organization or the target: [`DefenderState::engaged_with`].
//!
//! The admission `0x4C8F20` is [`super::admission`]; the ability's use is
//! [`super::use_weapon`]. No getter of these has been met in a film yet, so what
//! is here is compared with the code and not with a recording.
use super::{CombatState, TargetRef};
use crate::world::{ActorId, World};
use crate::zones::Key;

/// The component slot of the Defender.
pub const SLOT: u8 = 20;
/// `0x50ACC0(0x3F800000)`.
const LIST_SECONDS: f32 = 1.0;

/// One thing's Defender. `+0x1C` holds GTIDs; a dead one simply stops resolving,
/// so a dead thing here stands for an entry that no longer resolves.
#[derive(Clone, Debug, Default)]
pub struct DefenderState {
    /// `+0x1C`: the intruders, head first (`0x4D0900`).
    pub intruders: Vec<TargetRef>,
    /// Ability `+0x14` per ability of the definition's Defender list: the weapon
    /// index in `Def::weapons` and the target.
    pub targets: Vec<(usize, Option<ActorId>)>,
}

impl DefenderState {
    /// The `+0x118` engagements this Defender holds: what `0x567140` registered
    /// with `0x4D3990`, the target's organization or the target.
    pub fn engaged_with(&self, world: &World) -> Vec<TargetRef> {
        self.targets
            .iter()
            .filter_map(|&(_, t)| {
                let t = t?;
                Some(match world.actors.get(t as usize)?.org {
                    Some(o) => TargetRef::Org(o),
                    None => TargetRef::Actor(t),
                })
            })
            .collect()
    }
}

fn to_ref(k: Key) -> TargetRef {
    match k {
        Key::Actor(a) => TargetRef::Actor(a),
        Key::Org(o) => TargetRef::Org(o),
    }
}

/// `0x567BF0`'s and `0x567840`'s read of the abilities: the Defender list of
/// the definition (ActorData `+0x21C`, the constructor `0x567CD0`, source order).
fn abilities(world: &World, a: ActorId) -> Vec<usize> {
    let Some(x) = world.actors.get(a as usize) else { return Vec::new() };
    world.data.def(x.def).weapons.iter().enumerate().filter(|(_, w)| w.defender).map(|(i, _)| i).collect()
}

/// `0x5673B0`: some ability could be used on `t`: an actor, targetable (`+8` bit
/// 14), that one admits by its filter (`0x4C8740`), turret arc (`0x4C84B0`) and
/// minimum range (`0x4C8460`) and, with a projectile (definition `+0x3C`), within
/// its range (`0x4BE910`, centre to centre); an organization by any member.
fn reachable(world: &World, st: &CombatState, a: ActorId, t: TargetRef) -> bool {
    match t {
        TargetRef::Org(o) => world.orgs.get(o as usize).is_some_and(|g| g.alive && g.members.iter().any(|&m| reachable(world, st, a, TargetRef::Actor(m)))),
        TargetRef::Actor(m) => {
            let Some(x) = world.actors.get(m as usize).filter(|x| x.alive && !x.removed) else { return false };
            if !world.data.def(x.def).targetable {
                return false;
            }
            let shooter = &world.actors[a as usize];
            let data = world.data.clone();
            let def = data.def(shooter.def);
            let distance = shooter.pos.truncate().distance(x.pos.truncate());
            def.weapons.iter().filter(|w| w.defender).any(|w| {
                super::admits(world, st, a, w, m)
                    && super::in_turret_arc(world, a, w, x.pos.truncate())
                    && (w.min_range <= 0.0 || super::in_range(world, a, m, w.min_range, true))
                    // definition +0x3C: only a projectile's range limits it here
                    && (w.projectile.is_none() || distance <= w.range)
            })
        }
    }
}

/// `0x566D40` of a candidate: 3 for an aggressive Mind, 2 for what can hurt
/// (`0x4D03F0` false), then 1 for a body that can be taken or destroyed
/// (Body and `+0x291` and not invulnerable, or `0x4D1120`), else 0.
fn threat(world: &World, st: &CombatState, t: ActorId) -> u32 {
    if super::mind_aggressive(world, st, TargetRef::Actor(t)) {
        return 3;
    }
    if !super::harmless(world, t) {
        return 2;
    }
    let d = world.data.def(world.actors[t as usize].def);
    let body = d.health > 0.0;
    if (body && d.destructible && !d.invulnerable) || super::capturable(world, t) { 1 } else { 0 }
}

/// `0x567530`: the candidate of one ability over one intruder, members standing
/// for an organization; the best so far is the highest `0x566D40`, then the nearest
/// (`0x45E790`, the Defender's position to the candidate's).
fn better(world: &World, st: &CombatState, a: ActorId, weapon: usize, t: TargetRef, best: &mut (i64, f32), out: &mut Option<ActorId>) {
    match t {
        TargetRef::Org(o) => {
            let Some(g) = world.orgs.get(o as usize) else { return };
            for &m in &g.members {
                better(world, st, a, weapon, TargetRef::Actor(m), best, out);
            }
        }
        TargetRef::Actor(m) => {
            let Some(x) = world.actors.get(m as usize).filter(|x| x.alive && !x.removed) else { return };
            let p = i64::from(threat(world, st, m));
            if p == 0 || p < best.0 {
                return;
            }
            if super::admission(world, st, a, weapon, m) != 0 {
                return;
            }
            let distance = world.actors[a as usize].pos.truncate().distance(x.pos.truncate());
            if p > best.0 || distance < best.1 {
                *best = (p, distance);
                *out = Some(m);
            }
        }
    }
}

/// `0x567840`: `owner`'s ZOC was entered by `who` (its organization when attached,
/// `0x514DA0`). The Defender of a dead thing does nothing, one without abilities
/// never admits. The first intruder starts the Updatable when it is on no queue
/// and the world is not loading (`0x4F0460`).
pub(crate) fn entered(world: &mut World, owner: Key, who: Key) {
    let Key::Actor(a) = owner else { return };
    let mut st = std::mem::take(&mut world.combat);
    entered_with(world, &mut st, a, who);
    world.combat = st;
}

fn entered_with(world: &mut World, st: &mut CombatState, a: ActorId, who: Key) {
    if !world.actors.get(a as usize).is_some_and(|x| x.alive && !x.removed) {
        return;
    }
    let t = to_ref(who);
    if st.actors.get(a as usize).is_some_and(|s| s.defender.intruders.contains(&t)) || !reachable(world, st, a, t) {
        return;
    }
    let row = abilities(world, a);
    let s = &mut st.actors[a as usize].defender;
    if s.targets.len() != row.len() {
        s.targets = row.into_iter().map(|w| (w, None)).collect();
    }
    s.intruders.insert(0, t);
    let owner = crate::scheduler::Owner::Component(a, SLOT);
    // 0x4F0460: not while the world loads (GWorld +28's mode, gone once it runs).
    if !world.is_scheduled(owner) && world.session_started {
        world.schedule_periodic_at(crate::trace::sites::DEFENDER_ENTERED, crate::scheduler::EVERY_TICK, owner, 0);
        world.schedule_after(crate::scheduler::DEFERRED, owner, 0, LIST_SECONDS);
    }
}

/// The deferred `0x567BF0` of actor `a`'s Defender.
pub(crate) fn deferred(world: &mut World, a: ActorId) {
    let mut st = std::mem::take(&mut world.combat);
    let mut list = st.actors.get_mut(a as usize).map_or_else(Vec::new, |s| std::mem::take(&mut s.defender.intruders));
    // zone 4 and virtual +0x130(zone, 1): the thing or a member stands in it
    let engine = world.fog.engine.as_ref();
    list.retain(|&t| target_exists(world, t) && engine.is_some_and(|e| e.in_zoc(world, key_of(t), a)) && reachable(world, &st, a, t));
    let empty = list.is_empty();
    if let Some(s) = st.actors.get_mut(a as usize) {
        s.defender.intruders = list;
        if empty {
            for row in &mut s.defender.targets {
                row.1 = None;
            }
        }
    }
    world.combat = st;
    let owner = crate::scheduler::Owner::Component(a, SLOT);
    if empty {
        world.unschedule(owner);
    } else {
        world.schedule_after(crate::scheduler::DEFERRED, owner, 0, LIST_SECONDS);
    }
}

fn key_of(t: TargetRef) -> Key {
    match t {
        TargetRef::Actor(a) => Key::Actor(a),
        TargetRef::Org(o) => Key::Org(o),
    }
}

fn target_exists(world: &World, t: TargetRef) -> bool {
    match t {
        TargetRef::Actor(a) => world.actors.get(a as usize).is_some_and(|x| x.alive && !x.removed),
        TargetRef::Org(o) => world.orgs.get(o as usize).is_some_and(|g| g.alive),
    }
}

/// The Tick `0x567960` of actor `a`'s Defender.
pub(crate) fn tick(world: &mut World, a: ActorId) {
    if !world.actors.get(a as usize).is_some_and(|x| x.alive && !x.removed) {
        return;
    }
    // Defender +0x30 (0x566F80: the modifiers changed, +0x2C the new radius): the first
    // Tick after it moves the ZOC (0x515F40 at the thing's position).
    if let Some(mut e) = crate::fog::take_engine(world) {
        let key = Key::Actor(a);
        let control = e.def_of(world, key).control;
        let radius = match crate::zones::modifier_add(world, key, "CONTROL_RANGE") {
            Some(add) => crate::zones::clamp_add(control, add, world.data.rule_ci("ModifierAttributeClampAdd").unwrap_or(1.0)),
            None => control,
        };
        e.zoc_radius(world, key, radius);
        crate::fog::put_engine(world, e);
    }
    let mut st = std::mem::take(&mut world.combat);
    if st.actors.len() < world.actors.len() || st.orgs.len() < world.orgs.len() || st.rules.is_none() {
        st.prepare(world);
    }
    let count = st.actors[a as usize].defender.targets.len();
    for i in 0..count {
        let (weapon, mut target) = st.actors[a as usize].defender.targets[i];
        if let Some(t) = target {
            if !world.actors.get(t as usize).is_some_and(|x| x.alive && !x.removed) {
                target = None;
            } else {
                match super::admission(world, &st, a, weapon, t) {
                    0 => {}
                    // the ability is not ready: nothing for it in this update
                    7 | 8 => continue,
                    _ => target = None,
                }
            }
        }
        if target.is_none_or(|t| threat(world, &st, t) < 2) {
            let (mut best, mut chosen) = ((-1i64, f32::MAX), None);
            let list = st.actors[a as usize].defender.intruders.clone();
            for t in list {
                better(world, &st, a, weapon, t, &mut best, &mut chosen);
            }
            target = chosen;
        }
        st.actors[a as usize].defender.targets[i].1 = target;
        let Some(t) = target else { continue };
        // 0x4C81D0 / 0x57E730: a turret that turns has to be on it first
        if !super::turret_ready(world, &mut st, a as usize, weapon, t) {
            continue;
        }
        super::use_weapon(world, &mut st, a, weapon, t);
    }
    world.combat = st;
}
