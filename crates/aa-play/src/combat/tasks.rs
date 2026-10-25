//! The HUD orders on the ability and builder ports: bombard, repair,
//! construct, air attack and air recon. Owner: core.
//!
//! Each order puts an organization (or an actor outside one) in a Mind state
//! that replaces the engage Think while it lasts (`OrgCombat::task`). A new
//! order the player or the opponent gives (the organization's order differs
//! from the one the task wrote last) ends it, as a new command replaces the
//! Mind state.
//!
//! Bombard (actor command 1): the regiment state `KKC_CAIStateBombard::Think`
//! 0x5207F0 and the element state `KKC_UAIStateBombard::Think` 0x5353E0.
//! - The relation to the point (0x4D5180 / Performer 0x57EAF0) is the least
//!   one over the members and their bombard abilities (ability flag bit 3,
//!   with a range, or flag bit 5 target_location): 3 when the ability's
//!   turret cannot turn to it (0x4C84B0 -> 0x4C76E0), 2 inside its minimum
//!   range (0x4C8460), 0 within its range (0x4BE910), else 1.
//! - 0 fires: the move state is left, the members bombard the point (or the
//!   actor) and the next Think follows in 1.0 s.
//! - 1 approaches: the organization moves toward the point (0x520440).
//! - Anything else ends the state (Mind virtual +144). An actor target that is
//!   gone or no longer hostile ends it too (virtual +5C, Mind +140).
//! - An actor outside an organization without a mover cannot approach (Mind
//!   virtual +124).
//! - Firing (0x5340C0(null, &point, &record, 1) -> 0x5307A0): of the bombard
//!   abilities with relation 0 the best ready one by its use weights, as the
//!   ability decision weighs them, through the turret aim; a projectile flies
//!   to the point and the area effect hits every hostile there.
//!
//! Compare against the original: members out of range hold their formation
//! place instead of approaching on their own (the element state's approach);
//! the approach re-thinks every 0.25 s; an actor target is aimed at its
//! current position with the actor relation of a point (0x4D4FF0(target, 3,
//! 1) through 0x57E920 is not ported).
use super::{alive, deliver, targetable, turret_aim, turret_can_turn, turret_in_arc, CombatState, Projectile};
use crate::data::{DefId, Weapon};
use crate::world::{AbilityTarget, Activity, ActorId, Command, Event, Order, OrgId, Unit, World};
use glam::Vec2;

/// Ability definition flag bits (+0x54): bombard and target_location.
pub const BOMBARD_FLAG: u32 = 1 << 3;
pub const TARGET_LOCATION_FLAG: u32 = 1 << 5;
/// 0x5207F0 re-thinks 1.0 s after firing.
const FIRE_THINK_SECONDS: f32 = 1.0;
/// Re-think while approaching (see the module documentation).
const APPROACH_THINK_SECONDS: f32 = 0.25;
/// MoverComponent (aa-rules COMPONENTS bit 3).
const MOVER_BIT: u32 = 3;

/// The Mind state an order puts an organization (or a lone actor) in.
#[derive(Clone, Debug, PartialEq)]
pub enum Task {
    Bombard(AbilityTarget),
    Repair(ActorId),
    Construct { def: DefId, at: Vec2, facing: f32 },
}

/// What a member uses its task ability on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Aim {
    Point(Vec2),
    Actor(ActorId),
    /// A friendly actor this one repairs (or builds).
    Repair(ActorId),
}

/// 0x5359D0 waits 0.6 s while its repair ability recharges.
const REPAIR_WAIT_SECONDS: f32 = 0.6;

/// An ability with a HEAL effect (the repair abilities of engineers).
fn heals(w: &Weapon) -> bool {
    w.effects.iter().any(|e| e.kind_name == "HEAL")
}

/// Distance from actor `a` to the body of `t`: to the edge of its blocking
/// footprint (rectangle across / along its facing, circle radius) when it
/// has one, else between the positions. Compare against the original: the
/// native shape/shape range predicate (aa-combat `range_test`) also takes the
/// source's own shape.
fn reach(world: &World, a: ActorId, t: ActorId) -> f32 {
    let p = world.actors[a as usize].pos.truncate();
    let target = &world.actors[t as usize];
    let c = target.pos.truncate();
    let d = world.data.def(target.def);
    match d.blocking_size {
        Some(size) if d.blocking_shape.as_deref().is_some_and(|s| s.eq_ignore_ascii_case("CIRCLE")) => (p.distance(c) - size[0].max(0.0)).max(0.0),
        Some(size) => {
            let (s, cos) = target.facing.sin_cos();
            let forward = Vec2::new(cos, s);
            let across = Vec2::new(-s, cos);
            let v = p - c;
            let dx = (v.dot(across).abs() - size[0].max(0.0) * 0.5).max(0.0);
            let dy = (v.dot(forward).abs() - size[1].max(0.0) * 0.5).max(0.0);
            (dx * dx + dy * dy).sqrt()
        }
        None => p.distance(c),
    }
}

/// 0x5340C0 for actor `a`'s repair abilities against `t`: the best admitted
/// one by the decision weights and its status (0 use, 1 recharging, 2 out of
/// range), None when none admits the target.
fn repair_choice(world: &World, st: &CombatState, a: ActorId, t: ActorId) -> Option<(usize, u8)> {
    let actor = &world.actors[a as usize];
    let def = world.data.def(actor.def);
    let distance = reach(world, a, t);
    let mut best: Option<(usize, u8, f32)> = None;
    for (i, w) in def.weapons.iter().enumerate() {
        if !heals(w) || !super::admits(world, st, a, w, t) {
            continue;
        }
        let left = actor.combat.recharge.get(i).copied().unwrap_or(0.0);
        let status = super::weapon_status(w, left, distance);
        if status == 3 {
            continue;
        }
        let score = match status {
            0 => w.use_weight * w.priority + w.use_bonus + def.performer_use_bonus,
            1 => w.wait_weight * w.priority + w.wait_bonus + def.performer_wait_bonus,
            _ => w.move_weight * w.priority + w.move_bonus + def.performer_move_bonus,
        };
        if best.is_none_or(|(_, _, s)| score > s) {
            best = Some((i, status, score));
        }
    }
    best.map(|(i, s, _)| (i, s))
}

/// Whether actor `a` can repair `t` at all (0x4D4FF0(target, 1, 0) = 0).
fn can_repair(world: &World, st: &CombatState, a: ActorId, t: ActorId) -> bool {
    repair_choice(world, st, a, t).is_some()
}

/// Body +14 of `t`.
fn max_health(world: &World, st: &CombatState, t: ActorId) -> f32 {
    st.actors.get(t as usize).filter(|c| c.initialized && c.max_health > 0.0).map_or_else(
        || {
            let a = &world.actors[t as usize];
            crate::production::max_health(world, a.kingdom, a.def)
        },
        |c| c.max_health,
    )
}

/// HealEffect Apply 0x4CA0B0 then Body 0x562BA0 / 0x561E50: the health
/// rises by the effect amount up to the maximum; a structure under
/// construction that becomes whole is finished (0x566AC0 -> 0x565B00).
/// Compare against the original: the source modifier's heal bonus
/// (0x501B70) is not applied.
fn heal(world: &mut World, st: &mut CombatState, shooter: ActorId, w: &Weapon, t: ActorId) {
    let amount: f32 = w.effects.iter().filter(|e| e.kind_name == "HEAL").map(|e| e.amount.max(0.0)).sum();
    let max = max_health(world, st, t);
    let before = world.actors[t as usize].health;
    if before >= max || amount <= 0.0 {
        return;
    }
    let after = (before + amount).min(max);
    world.actors[t as usize].health = after;
    world.events.push(Event::Healed { shooter, target: t, amount: after - before });
    if after >= max && world.books.actor(t).is_some_and(|b| b.constructing) {
        finish_construction(world, t);
    }
}

/// OnConstructionFinished 0x565B00 for `t`: actor state 21 off; the
/// structure works from now on.
pub(crate) fn finish_construction(world: &mut World, t: ActorId) {
    world.books.actor_mut(t).constructing = false;
    if let Some(k) = world.actors[t as usize].kingdom {
        world.events.push(Event::ConstructionFinished { actor: t, kingdom: k });
    }
}

fn bombards(w: &Weapon) -> bool {
    w.flags & BOMBARD_FLAG != 0 && (w.range > 0.0 || w.flags & TARGET_LOCATION_FLAG != 0)
}

/// 0x4C84B0 / 0x4C8460 / 0x4BE910 for one ability at `point`.
fn point_status(world: &World, a: ActorId, w: &Weapon, point: Vec2, check_range: bool) -> u8 {
    let actor = &world.actors[a as usize];
    let def = world.data.def(actor.def);
    let pos = actor.pos.truncate();
    if let Some(t) = usize::try_from(w.turret).ok().and_then(|i| def.turrets.get(i))
        && turret_can_turn(t)
        && !turret_in_arc(t, pos, actor.facing, point)
    {
        return 3;
    }
    let distance = pos.distance(point);
    if w.min_range >= super::EPSILON && distance < w.min_range {
        return 2;
    }
    if !check_range || distance * distance <= w.range * w.range {
        0
    } else {
        1
    }
}

/// 0x57EAF0 for actor `a`: the least relation of its bombard abilities to
/// `point`, 4 without one.
pub fn point_relation(world: &World, a: ActorId, point: Vec2) -> u8 {
    let def = world.data.def(world.actors[a as usize].def);
    let mut best = 4;
    for w in def.weapons.iter().filter(|w| bombards(w)) {
        best = best.min(point_status(world, a, w, point, true));
        if best == 0 {
            break;
        }
    }
    best
}

/// Whether actor `a` has a bombard ability (the command mask 0x530510).
pub fn can_bombard(world: &World, a: ActorId) -> bool {
    world.actors.get(a as usize).is_some_and(|x| world.data.def(x.def).weapons.iter().any(bombards))
}

fn members(world: &World, o: OrgId) -> Vec<ActorId> {
    world.orgs[o as usize].members.iter().copied().filter(|&m| alive(world, m)).collect()
}

/// The point a bombard aims at now; None when its actor target is lost
/// (gone or no longer hostile: virtual +5C).
fn bombard_point(world: &World, st: &CombatState, kingdom: Option<crate::world::KingdomId>, target: AbilityTarget) -> Option<(Vec2, Aim)> {
    match target {
        AbilityTarget::Point(p) => Some((p, Aim::Point(p))),
        AbilityTarget::Actor(t) => {
            let a = world.actors.get(t as usize)?;
            (alive(world, t) && targetable(world, t) && st.hostile(kingdom, a.kingdom)).then(|| (a.pos.truncate(), Aim::Actor(t)))
        }
    }
}

/// Apply a HUD order; Err is the refusal reason.
pub fn command(world: &mut World, command: &Command) -> Result<(), String> {
    let mut st = std::mem::take(&mut world.combat);
    st.prepare(world);
    let result = apply(world, &mut st, command);
    world.combat = st;
    result
}

fn apply(world: &mut World, st: &mut CombatState, command: &Command) -> Result<(), String> {
    match command {
        Command::Bombard { kingdom, units, target } => {
            if let AbilityTarget::Actor(t) = *target
                && !world.actor(t).is_some_and(|a| a.alive && st.hostile(Some(*kingdom), a.kingdom))
            {
                return Err("no enemy to bombard there".into());
            }
            let mut any = false;
            for unit in units {
                match own_unit(world, *kingdom, *unit) {
                    Some(Unit::Org(o)) if members(world, o).iter().any(|&m| can_bombard(world, m)) => {
                        begin_org(world, st, o, Task::Bombard(*target));
                        any = true;
                    }
                    Some(Unit::Actor(a)) if can_bombard(world, a) => {
                        begin_actor(world, st, a, Task::Bombard(*target));
                        any = true;
                    }
                    _ => {}
                }
            }
            if any { Ok(()) } else { Err("nothing selected can bombard".into()) }
        }
        Command::Repair { kingdom, units, target } => {
            let t = *target;
            if !world.actor(t).is_some_and(|a| a.alive) {
                return Err("nothing to repair there".into());
            }
            let mut any = false;
            for unit in units {
                match own_unit(world, *kingdom, *unit) {
                    Some(Unit::Org(o)) if members(world, o).iter().any(|&m| can_repair(world, st, m, t)) => {
                        begin_org(world, st, o, Task::Repair(t));
                        any = true;
                    }
                    Some(Unit::Actor(a)) if can_repair(world, st, a, t) => {
                        begin_actor(world, st, a, Task::Repair(t));
                        any = true;
                    }
                    _ => {}
                }
            }
            if any { Ok(()) } else { Err("nothing selected can repair that".into()) }
        }
        Command::Construct { kingdom, units, def, at, facing } => {
            if (*def as usize) >= world.data.defs.len() {
                return Err("unknown structure".into());
            }
            let mut any = false;
            for unit in units {
                // 0x530510: construct needs a builder of the structure and a mover.
                if let Some(Unit::Org(o)) = own_unit(world, *kingdom, *unit)
                    && builder_for(world, o, *def).is_some()
                    && members(world, o).iter().any(|&m| world.data.def(world.actors[m as usize].def).components & (1 << MOVER_BIT) != 0)
                {
                    begin_org(world, st, o, Task::Construct { def: *def, at: *at, facing: *facing });
                    any = true;
                }
            }
            if any { Ok(()) } else { Err("nothing selected can build that".into()) }
        }
        other => Err(format!("{other:?}: not available yet")),
    }
}

/// The unit as the organization or lone actor of `kingdom` the order goes to
/// (a member's order goes to its organization).
fn own_unit(world: &World, kingdom: crate::world::KingdomId, unit: Unit) -> Option<Unit> {
    match unit {
        Unit::Org(o) => world.orgs.get(o as usize).filter(|x| x.alive && x.kingdom == kingdom).map(|_| Unit::Org(o)),
        Unit::Actor(a) => {
            let x = world.actor(a).filter(|x| x.alive && x.kingdom == Some(kingdom))?;
            match x.org {
                Some(o) => own_unit(world, kingdom, Unit::Org(o)),
                None => Some(Unit::Actor(a)),
            }
        }
    }
}

fn begin_org(world: &mut World, st: &mut CombatState, o: OrgId, task: Task) {
    let now = world.time;
    let s = &mut st.orgs[o as usize];
    s.task = Some(task);
    s.explicit = None;
    s.auto_order = None;
    s.saved_order = None;
    s.task_think = now;
    world.orgs[o as usize].order = Order::Idle;
    s.task_order = Some(Order::Idle);
    for m in world.orgs[o as usize].members.clone() {
        world.actors[m as usize].combat.target = None;
        st.actors[m as usize].aim = None;
    }
}

fn begin_actor(world: &mut World, st: &mut CombatState, a: ActorId, task: Task) {
    let now = world.time;
    let s = &mut st.actors[a as usize];
    s.task = Some(task);
    s.task_think = now;
    s.aim = None;
    world.actors[a as usize].combat.target = None;
    world.actors[a as usize].order = Order::Idle;
    s.task_order = Some(Order::Idle);
}

fn end_org(world: &mut World, st: &mut CombatState, o: OrgId, order: Option<Order>) {
    st.orgs[o as usize].task = None;
    st.orgs[o as usize].task_order = None;
    for m in world.orgs[o as usize].members.clone() {
        st.actors[m as usize].aim = None;
    }
    if let Some(order) = order {
        world.orgs[o as usize].order = order;
    }
}

fn set_org_order(world: &mut World, st: &mut CombatState, o: OrgId, order: Order) {
    if world.orgs[o as usize].order != order {
        world.orgs[o as usize].order = order.clone();
    }
    st.orgs[o as usize].task_order = Some(order);
}

/// The task Think of organization `o`; false when it has none (the engage
/// Think runs instead).
pub(super) fn org_think(world: &mut World, st: &mut CombatState, o: OrgId) -> bool {
    let i = o as usize;
    let Some(task) = st.orgs[i].task.clone() else { return false };
    // A new order replaced the Mind state.
    if st.orgs[i].task_order.as_ref().is_some_and(|t| *t != world.orgs[i].order) {
        end_org(world, st, o, None);
        return false;
    }
    for m in world.orgs[i].members.clone() {
        world.actors[m as usize].combat.target = None;
    }
    let now = world.time;
    if now < st.orgs[i].task_think {
        return true;
    }
    match task {
        Task::Bombard(target) => org_bombard(world, st, o, target),
        Task::Repair(target) => org_repair(world, st, o, target),
        Task::Construct { def, at, facing } => org_construct(world, st, o, def, at, facing),
    }
    true
}

/// 0x5359D0 for the members of organization `o` repairing `t`: a whole body
/// ends the state (Mind +168), so does a target no member can repair (Mind
/// +140); members in range use their repair ability, a recharging one waits
/// 0.6 s with the organization stopped, and with nobody in range the
/// organization goes to the target. Compare against the original: the
/// members out of range hold their formation place.
fn org_repair(world: &mut World, st: &mut CombatState, o: OrgId, t: ActorId) {
    let i = o as usize;
    let now = world.time;
    if !alive(world, t) || world.actors[t as usize].health >= max_health(world, st, t) {
        end_org(world, st, o, Some(Order::Idle));
        return;
    }
    let members = members(world, o);
    let choices: Vec<(ActorId, Option<(usize, u8)>)> = members.iter().map(|&m| (m, repair_choice(world, st, m, t))).collect();
    if choices.iter().all(|(_, c)| c.is_none()) {
        end_org(world, st, o, Some(Order::Idle));
        return;
    }
    let near = choices.iter().any(|(_, c)| matches!(c, Some((_, 0 | 1))));
    // Every member that can repair it aims at it; one out of range walks to
    // it on its own (the element state's approach: `approach_goal`).
    for &(m, c) in &choices {
        st.actors[m as usize].aim = c.is_some().then_some(Aim::Repair(t));
    }
    if near {
        set_org_order(world, st, o, Order::Idle);
        let waiting = !choices.iter().any(|(_, c)| matches!(c, Some((_, 0))));
        st.orgs[i].task_think = now + if waiting { REPAIR_WAIT_SECONDS } else { APPROACH_THINK_SECONDS };
    } else {
        let to = approach_point(world, t, st.orgs[i].position);
        set_org_order(world, st, o, Order::Move { to, attack_move: false });
        st.orgs[i].task_think = now + APPROACH_THINK_SECONDS;
    }
}

/// The first member of `o` in slot order whose builder lists `def`
/// (0x57ACC0: Builder +0xA8, ActorData +0x504).
fn builder_for(world: &World, o: OrgId, def: DefId) -> Option<ActorId> {
    let mut members = members(world, o);
    members.sort_by_key(|&m| world.actors[m as usize].slot.unwrap_or(u16::MAX));
    members.into_iter().find(|&m| world.data.def(world.actors[m as usize].def).builds.contains(&def))
}

/// `KKC_CAIStateConstruct::Think` 0x51DB80 for organization `o`: outside
/// the point's reach (0x4F2420 on the organization's zone 4) the move state
/// takes it there; inside, 0x57ACC0 has its builder place the structure
/// (BuildActor 0x563160, `crate::construct`), then the regiment builds it up
/// (Mind +170 with the new structure: the repair state on it); a refused
/// placement ends the state (Mind +174). Compare against the original: zone
/// 4 is taken as the circle of the organization's control_range, not its
/// tile flood, and Mind +170 is read as the repair state.
fn org_construct(world: &mut World, st: &mut CombatState, o: OrgId, def: DefId, at: Vec2, facing: f32) {
    let i = o as usize;
    let now = world.time;
    let reach = world.data.def(world.orgs[i].def).control_range;
    if st.orgs[i].position.distance(at) > reach {
        set_org_order(world, st, o, Order::Move { to: at, attack_move: false });
        st.orgs[i].task_think = now + APPROACH_THINK_SECONDS;
        return;
    }
    let Some(builder) = builder_for(world, o, def) else {
        end_org(world, st, o, Some(Order::Idle));
        return;
    };
    set_org_order(world, st, o, Order::Idle);
    match crate::construct::build_actor(world, builder, def, at, facing) {
        Some(built) => {
            // The new structure joins the combat tables of this tick.
            st.prepare(world);
            st.orgs[i].task = Some(Task::Repair(built));
            st.orgs[i].task_think = now;
        }
        None => end_org(world, st, o, Some(Order::Idle)),
    }
}

/// For the movement owner: where member `m` walks on its own for its task
/// ability (the element states' approach: 0x5359D0 status 2, 0x5353E0
/// relation 1) and within what distance of it the walk may end; None when
/// it is in range or has no aim. The organization itself waits in place.
pub fn approach_goal(world: &World, m: ActorId) -> Option<(Vec2, f32)> {
    let st = &world.combat;
    let aim = st.actors.get(m as usize)?.aim?;
    match aim {
        Aim::Repair(t) => {
            if !alive(world, t) {
                return None;
            }
            let (weapon, status) = repair_choice(world, st, m, t)?;
            (status == 2).then(|| {
                let range = world.data.def(world.actors[m as usize].def).weapons[weapon].range;
                (approach_point(world, t, world.actors[m as usize].pos.truncate()), range)
            })
        }
        Aim::Point(p) => (point_relation(world, m, p) == 1).then(|| (p, bombard_range(world, m))),
        Aim::Actor(t) => {
            let p = world.actors.get(t as usize)?.pos.truncate();
            (point_relation(world, m, p) == 1).then(|| (p, bombard_range(world, m)))
        }
    }
}

/// For the movement owner: member `m` is busy with its organization's task
/// (an aim to bombard or repair, in range or walking there): the slot reform
/// leaves it where it is.
pub fn busy(world: &World, m: ActorId) -> bool {
    world.combat.actors.get(m as usize).is_some_and(|a| a.aim.is_some())
}

/// The longest range of `m`'s bombard abilities.
fn bombard_range(world: &World, m: ActorId) -> f32 {
    world.data.def(world.actors[m as usize].def).weapons.iter().filter(|w| bombards(w)).map(|w| w.range).fold(0.0, f32::max)
}

/// Where to go to reach the body of `t` from `from`: the nearest point of
/// its footprint grown by half a metre (its position without one). Compare
/// against the original: the locomotion follows the object itself (0x576AD0).
fn approach_point(world: &World, t: ActorId, from: Vec2) -> Vec2 {
    let target = &world.actors[t as usize];
    let c = target.pos.truncate();
    let d = world.data.def(target.def);
    let Some(size) = d.blocking_size else { return c };
    let margin = 0.5;
    if d.blocking_shape.as_deref().is_some_and(|s| s.eq_ignore_ascii_case("CIRCLE")) {
        let dir = (from - c).normalize_or_zero();
        return c + dir * (size[0].max(0.0) + margin);
    }
    let (s, cos) = target.facing.sin_cos();
    let forward = Vec2::new(cos, s);
    let across = Vec2::new(-s, cos);
    let v = from - c;
    let (hw, hh) = (size[0].max(0.0) * 0.5 + margin, size[1].max(0.0) * 0.5 + margin);
    let (mut x, mut y) = (v.dot(across).clamp(-hw, hw), v.dot(forward).clamp(-hh, hh));
    // From inside the grown footprint: out through the nearest side.
    if x.abs() < hw && y.abs() < hh {
        if hw - x.abs() < hh - y.abs() {
            x = hw.copysign(x);
        } else {
            y = hh.copysign(y);
        }
    }
    c + across * x + forward * y
}

/// 0x5207F0 for organization `o`.
fn org_bombard(world: &mut World, st: &mut CombatState, o: OrgId, target: AbilityTarget) {
    let i = o as usize;
    let now = world.time;
    let kingdom = Some(world.orgs[i].kingdom);
    let Some((point, aim)) = bombard_point(world, st, kingdom, target) else {
        // Mind +140: the target is lost.
        end_org(world, st, o, Some(Order::Idle));
        return;
    };
    let members = members(world, o);
    let relation = members.iter().map(|&m| point_relation(world, m, point)).min().unwrap_or(4);
    match relation {
        0 => {
            set_org_order(world, st, o, Order::Idle);
            for m in members {
                st.actors[m as usize].aim = can_bombard(world, m).then_some(aim);
            }
            st.orgs[i].task_think = now + FIRE_THINK_SECONDS;
        }
        1 => {
            set_org_order(world, st, o, Order::Move { to: point, attack_move: false });
            for m in members {
                st.actors[m as usize].aim = None;
            }
            st.orgs[i].task_think = now + APPROACH_THINK_SECONDS;
        }
        // Mind +144: the point cannot be bombarded.
        _ => end_org(world, st, o, Some(Order::Idle)),
    }
}

/// The task Think of actor `a` outside an organization; false without one.
pub(super) fn lone_think(world: &mut World, st: &mut CombatState, a: ActorId) -> bool {
    let i = a as usize;
    let Some(task) = st.actors[i].task.clone() else { return false };
    if st.actors[i].task_order.as_ref().is_some_and(|t| *t != world.actors[i].order) {
        st.actors[i].task = None;
        st.actors[i].task_order = None;
        st.actors[i].aim = None;
        return false;
    }
    world.actors[i].combat.target = None;
    let now = world.time;
    if now < st.actors[i].task_think {
        return true;
    }
    let end = |world: &mut World, st: &mut CombatState| {
        st.actors[i].task = None;
        st.actors[i].task_order = None;
        st.actors[i].aim = None;
        world.actors[i].order = Order::Idle;
    };
    match task {
        Task::Bombard(target) => {
            let kingdom = world.actors[i].kingdom;
            let Some((point, aim)) = bombard_point(world, st, kingdom, target) else {
                end(world, st);
                return true;
            };
            match point_relation(world, a, point) {
                0 => {
                    world.actors[i].order = Order::Idle;
                    st.actors[i].task_order = Some(Order::Idle);
                    st.actors[i].aim = Some(aim);
                    st.actors[i].task_think = now + FIRE_THINK_SECONDS;
                }
                // 0x5353E0 approach: without a locomotion Mind virtual +124.
                1 if world.data.def(world.actors[i].def).components & (1 << MOVER_BIT) != 0 => {
                    let order = Order::Move { to: point, attack_move: false };
                    world.actors[i].order = order.clone();
                    st.actors[i].task_order = Some(order);
                    st.actors[i].aim = None;
                    st.actors[i].task_think = now + APPROACH_THINK_SECONDS;
                }
                _ => end(world, st),
            }
        }
        Task::Repair(t) => {
            if !alive(world, t) || world.actors[t as usize].health >= max_health(world, st, t) {
                end(world, st);
                return true;
            }
            match repair_choice(world, st, a, t) {
                Some((_, 0 | 1)) => {
                    world.actors[i].order = Order::Idle;
                    st.actors[i].task_order = Some(Order::Idle);
                    st.actors[i].aim = Some(Aim::Repair(t));
                    st.actors[i].task_think = now + APPROACH_THINK_SECONDS;
                }
                Some(_) if world.data.def(world.actors[i].def).components & (1 << MOVER_BIT) != 0 => {
                    let to = approach_point(world, t, world.actors[i].pos.truncate());
                    let order = Order::Move { to, attack_move: false };
                    world.actors[i].order = order.clone();
                    st.actors[i].task_order = Some(order);
                    st.actors[i].aim = None;
                    st.actors[i].task_think = now + APPROACH_THINK_SECONDS;
                }
                _ => end(world, st),
            }
        }
        _ => end(world, st),
    }
    true
}

/// 0x5359D0 status 0: the performer uses the chosen repair ability on `t`
/// (0x57F290), facing it.
fn repair_fire(world: &mut World, st: &mut CombatState, id: usize, t: ActorId) {
    if !alive(world, t) {
        st.actors[id].aim = None;
        return;
    }
    let Some((weapon, 0)) = repair_choice(world, st, id as ActorId, t) else { return };
    let data = world.data.clone();
    let w = &data.def(world.actors[id].def).weapons[weapon];
    let multiplier = crate::special_ops::mods(world, id as ActorId).map_or(1.0, |m| (1.0 + m.recharge).max(st.rules().clamp_multiply));
    world.actors[id].combat.recharge[weapon] = w.recharge / multiplier;
    world.set_activity(id as ActorId, Activity::Attacking);
    let delta = world.actors[t as usize].pos.truncate() - world.actors[id].pos.truncate();
    if world.actors[id].motion.velocity.length_squared() < 1e-4 && delta.length_squared() > 0.01 {
        world.actors[id].facing = delta.y.atan2(delta.x);
    }
    heal(world, st, id as ActorId, w, t);
}

/// Use the actor's task ability on its aim (a bombard: 0x5340C0 with the
/// point, 0x5307A0). Called by `fire` for an actor without a combat target.
pub(super) fn fire_aim(world: &mut World, st: &mut CombatState, id: usize, aim: Aim) {
    let now = world.time;
    let data = world.data.clone();
    let (point, target) = match aim {
        Aim::Repair(t) => {
            repair_fire(world, st, id, t);
            return;
        }
        Aim::Point(p) => (p, None),
        Aim::Actor(t) if alive(world, t) && targetable(world, t) => (world.actors[t as usize].pos.truncate(), Some(t)),
        Aim::Actor(_) => {
            st.actors[id].aim = None;
            return;
        }
    };
    let def = data.def(world.actors[id].def);
    // The best ready bombard ability in range (the use weights of 52ED30).
    let mut best: Option<(usize, f32)> = None;
    for (i, w) in def.weapons.iter().enumerate() {
        if !bombards(w) || point_status(world, id as ActorId, w, point, true) != 0 {
            continue;
        }
        if world.actors[id].combat.recharge.get(i).copied().unwrap_or(0.0) > 0.0 {
            continue;
        }
        let score = w.use_weight * w.priority + w.use_bonus + def.performer_use_bonus;
        if best.is_none_or(|(_, s)| score > s) {
            best = Some((i, score));
        }
    }
    let Some((weapon, _)) = best else { return };
    let w = &def.weapons[weapon];
    if let Some((ti, td)) = usize::try_from(w.turret).ok().and_then(|i| def.turrets.get(i).map(|t| (i, t)))
        && turret_can_turn(td)
        && !turret_aim(world, st, id, ti, td, point)
    {
        return;
    }
    let multiplier = crate::special_ops::mods(world, id as ActorId).map_or(1.0, |m| (1.0 + m.recharge).max(st.rules().clamp_multiply));
    world.actors[id].combat.recharge[weapon] = w.recharge / multiplier;
    match target {
        Some(t) => world.events.push(Event::Fired { shooter: id as ActorId, target: t, weapon: weapon as u32 }),
        None => world.events.push(Event::FiredAt { shooter: id as ActorId, at: point, weapon: weapon as u32 }),
    }
    world.set_activity(id as ActorId, Activity::Attacking);
    let delta = point - world.actors[id].pos.truncate();
    if w.turret < 0 && world.actors[id].motion.velocity.length_squared() < 1e-4 && delta.length_squared() > 0.01 {
        world.actors[id].facing = delta.y.atan2(delta.x);
    }
    if w.projectile_speed > 0.0 {
        let flight = delta.length() / w.projectile_speed;
        st.projectiles.push(Projectile { shooter: id as ActorId, weapon, target, point, impact_at: now + flight });
    } else {
        deliver(world, st, id as ActorId, weapon, target, point);
    }
}
