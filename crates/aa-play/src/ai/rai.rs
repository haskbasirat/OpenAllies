//! The reactive AI of one player (`7B8AC0`, every `ReactiveAI/think_frequency`
//! seconds), in the original's order:
//! - `7B7250`: unit retreat, the immediate fight around each engaged actor;
//! - `7B64F0` over the RECOVER goals: the retreating actors go on or come back;
//! - `7B7750` over the selected DEFEND_REGION goals and `7B7D30` over the
//!   selected ATTACK goals: the goal's actors take targets (`7B5300`) or go
//!   back to their region (the goal's +5C, guard);
//! - `7B4FE0`: the formation of each organization, by its goal and how
//!   threatened its region is.
//!
//! `7AD550` then spaces the queued orders (`enqueue`). The ego fields are the
//! ones `4B0560` loads: +89C unit_retreat_max_health, +8A0 unit_retreat_depth,
//! +8A4 unit_retreat_cv_ratio, +8A8 unit_retreat_probability, +8B0
//! attack_distance, +8B4 min_attack_advantage, +8B8 max_attack_advantage, +8BC
//! threatened_region_influence_ratio. The template's region_retreat_* and
//! halt_retreat_cv_ratio have no reader in the original.
//!
//! A rendition, not yet exact: the actor lists hold organizations in the
//! order of the view, the goals' actor lists come from `goals`, and `7B5970`
//! (the bombarders' targets) is reduced to the nearest target in reach.
use super::data::{AiData, RECOVER};
use super::goals::{Assignment, Ctx, Key};
use super::regions::{Influence, Regions};
use super::view::{OrgView, View};
use super::{Bound, Player, SaiRng, enqueue};
use crate::combat::TargetRef;
use crate::orders::ActorOrder;
use crate::world::{ActorId, Command, Order, OrgId, Unit};
use glam::Vec2;
use std::collections::{BTreeMap, HashMap, VecDeque};

/// The kind of a SAI record's order state (`7A1070`, state +0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateKind {
    /// 0: `7B5300` and the attack goals' +58: fighting state +4.
    Attack,
    /// 1: `7B7750` and `7CA1D0`: holding region state +8.
    Defend,
    /// 2: `7B64F0` and `7C8AF0`: going to the supply thing state +4, at +C.
    Recover,
}

/// The order state of a SAI record (record +30, `7A1070`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrderState {
    pub kind: StateKind,
    /// +4: the thing fought, or the supply thing gone to.
    pub target: Option<Unit>,
    /// +8: the region held.
    pub region: Option<u32>,
    /// +C, +10: the place retreated to (-1, -1 is none).
    pub retreat: Option<Vec2>,
}

impl OrderState {
    pub fn new(kind: StateKind) -> Self {
        OrderState { kind, target: None, region: None, retreat: None }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Memory {
    /// The SAI records of the player's organizations (actor +154).
    pub states: BTreeMap<OrgId, OrderState>,
    /// Retreat orders given: (time, organization).
    pub retreats: Vec<(f32, OrgId)>,
}

impl Memory {
    pub(crate) fn recovering(&self, o: OrgId) -> bool {
        self.states.get(&o).is_some_and(|s| s.kind == StateKind::Recover)
    }
}

/// Whether the organization moves (`4CEDE0`: its Mind's move state with a
/// leader on the way, or its place not its destination yet).
fn moving(o: &OrgView) -> bool {
    matches!(o.order, Order::Move { .. })
}

/// `7A5E10` and `7C9CE0`'s test of a RECOVER goal R, `7C8AA0`: the actor's morale
/// fraction below critical_morale (`7C56C0`, +6E4) or a friendly supply thing
/// stands (`7C6570`). The movement flag of `4CF110` is not read here.
pub(crate) fn can_recover(c: &Ctx<'_, '_>, o: &OrgView) -> bool {
    o.morale < c.ego.recover.critical_morale || supply_exists(c)
}

/// The allied things with a supply component (Kingdom +264, `58AC70`).
fn supply_things<'a>(c: &'a Ctx<'_, '_>) -> impl Iterator<Item = &'a super::view::BuildingView> + 'a {
    c.view.buildings.iter().filter(|b| b.supply_range > 0.0 && b.owner.is_some_and(|o| c.view.allied[o]))
}

/// `7C6570`: an allied kingdom has a thing with an active supply component.
fn supply_exists(c: &Ctx<'_, '_>) -> bool {
    supply_things(c).next().is_some()
}

/// `54F8B0(kingdom, 0)` on a tile: inside a friendly supply zone.
fn friendly_supply(c: &Ctx<'_, '_>, at: Vec2) -> bool {
    supply_things(c).any(|b| b.pos.distance_squared(at) < b.supply_range * b.supply_range)
}

/// `52BB40`: for every allied supply thing `RecoverSupplyZoneCandidates` draws
/// within 0.9 of its radius; the best of the `52A930` recover values among the
/// places that are open and friendly supply. A rendition: `aa_mind::recover_select`
/// is the exact port, which needs the world services of `52A930` bound. It draws
/// from the SAI RNG, as `7C8AF0` passes it.
fn recover_point(c: &Ctx<'_, '_>, o: &OrgView, goal_region: Option<u32>, rng: &mut SaiRng) -> Option<(Vec2, ActorId)> {
    let w = c.view.world;
    let rule = |name: &str, shipped: f32| super::svar(w, name, shipped);
    let candidates = rule("RecoverSupplyZoneCandidates", 5.0) as i32;
    let base = rule("RecoverBaseValue", 0.0);
    let moved = rule("RecoverFactorMovedDist", 0.0);
    let moved_cap = rule("RecoverFactorMovedDistCap", f32::MAX);
    let supply = rule("RecoverFactorSupplyDist", 0.0);
    let supply_cap = rule("RecoverFactorSupplyDistCap", f32::MAX);
    let fuzz = rule("RecoverFuzziness", 0.0);
    let from = goal_region.map_or(o.pos, |r| c.regions.centers[r as usize]);
    let mut best: Option<(f32, Vec2, ActorId)> = None;
    for z in supply_things(c) {
        for _ in 0..candidates.max(0) {
            let angle = rng.unit() * std::f32::consts::TAU;
            let distance = rng.unit() * z.supply_range * 0.9;
            let p = z.pos + Vec2::new(angle.cos(), angle.sin()) * distance;
            if p == o.pos || !c.land(p) {
                continue;
            }
            let value = base - p.distance(from).min(moved_cap) * moved - p.distance(z.pos).min(supply_cap) * supply + rng.unit() * fuzz;
            if best.is_none_or(|(b, _, _)| value > b) {
                best = Some((value, p, z.id));
            }
        }
    }
    best.map(|(_, p, a)| (p, a))
}

/// `7C8AF0`, the order of a RECOVER goal (`7C9E20`) and of `7C9CE0`: the record
/// keeps its place while it stays friendly supply and the actor is not under
/// attack; an actor at rest in supply stays; else `52BB40` finds one. Retreat
/// there (`7C57C0`, 0x44), stop when there; with nowhere to go, an actor under
/// attack routs, another stops.
pub(crate) fn retreat_order(c: &Ctx<'_, '_>, player: &mut Player, o: usize, goal_region: Option<u32>, rng: &mut SaiRng) -> Command {
    let ov = &c.view.orgs[o];
    let me = c.view.kingdom;
    let attacked = ov.engaged;
    let state = player.rai.states.entry(ov.id).or_insert_with(|| OrderState::new(StateKind::Recover));
    let mut place = state.retreat;
    match place {
        Some(p) => {
            if attacked || !friendly_supply(c, p) {
                place = None;
            }
        }
        None => {
            if goal_region.is_none() && !attacked && friendly_supply(c, ov.pos) {
                place = Some(ov.pos);
            }
        }
    }
    if place.is_none()
        && let Some((p, provider)) = recover_point(c, ov, goal_region, rng)
    {
        place = Some(p);
        state.target = Some(Unit::Actor(provider));
    }
    state.retreat = place;
    let units = vec![Unit::Org(ov.id)];
    player.rai.retreats.push((c.now, ov.id));
    match place {
        Some(p) if p != ov.pos => Command::Order { kingdom: me, units, order: ActorOrder::Retreat { target: None, at: p } },
        Some(_) => Command::Stop { kingdom: me, units },
        None if attacked => Command::Order { kingdom: me, units, order: ActorOrder::Rout },
        None => Command::Stop { kingdom: me, units },
    }
}

/// `7C9CE0`: the RECOVER goal R of the actor wants it, against `value`, what it
/// is worth to the goal holding it: R's base above zero, R valid (`7C8AA0`), and
/// the actor in no order state or the recover state (R's +40, `7C5470`) or `value`
/// below R's base. A rendition of how the state ends: an actor at rest with
/// nothing to recover comes out of it (what ends it natively under a defend goal,
/// whose +74 is empty, is not read).
pub(crate) fn recover_wanted(c: &Ctx<'_, '_>, player: &mut Player, o: usize, value: f32) -> bool {
    let ov = &c.view.orgs[o];
    let base = player.memory.recover_base.get(&ov.id).copied().unwrap_or(0.0);
    if base <= 0.0 || !can_recover(c, ov) {
        if player.rai.recovering(ov.id) && !moving(ov) {
            player.rai.states.remove(&ov.id);
        }
        return false;
    }
    let state = player.rai.states.get(&ov.id).map(|s| s.kind);
    let held = matches!(state, None | Some(StateKind::Recover));
    !(!held && value >= base)
}

/// `7A5E10`, on a combat goal's actors: those its RECOVER goal wants go there
/// unless one of the goal's is committed (`7C5290`: fighting, near the goal's
/// region), and when the ones staying are under cv_advantage_target (+304)
/// times the region's hostile influence, all go. Returns those that stay.
pub(crate) fn retreat_check(c: &Ctx<'_, '_>, player: &mut Player, region: u32, value: impl Fn(usize) -> f32, list: Vec<usize>, rng: &mut SaiRng, commands: &mut Vec<Command>) -> Vec<usize> {
    let mut stay = Vec::new();
    let mut leave = Vec::new();
    let mut committed = false;
    for &o in &list {
        let ov = &c.view.orgs[o];
        let wanted = recover_wanted(c, player, o, value(o));
        let fighting = player.rai.states.get(&ov.id).is_some_and(|s| s.kind == StateKind::Attack) && c.regions.hops(c.region_of(ov.pos), region) as i32 <= c.ego.attack_commit_distance;
        if fighting {
            committed = true;
        }
        if !fighting && wanted {
            leave.push(o);
        } else {
            stay.push(o);
        }
    }
    if leave.is_empty() {
        return list;
    }
    if !committed {
        let cv: f32 = stay.iter().map(|&o| c.view.orgs[o].cv).sum();
        if cv < c.ego.offensive.cv_advantage_target * c.hostile(region, false) {
            for &o in &list {
                let order = retreat_order(c, player, o, Some(region), rng);
                commands.push(order);
            }
            return Vec::new();
        }
    }
    for &o in &leave {
        let order = retreat_order(c, player, o, Some(region), rng);
        commands.push(order);
    }
    stay
}

/// `7A5E10`'s list as one of the player's: an organization of the view.
fn index_of(view: &View<'_>) -> HashMap<OrgId, usize> {
    view.orgs.iter().enumerate().map(|(i, o)| (o.id, i)).collect()
}

/// The things an organization is fighting (+128) and those attacking it (+118).
fn engage_graph(view: &View<'_>, o: OrgId) -> Vec<OrgId> {
    let w = view.world;
    let mut out = Vec::new();
    if let Some(s) = w.combat.org(o) {
        for t in &s.engage {
            match t.target {
                TargetRef::Org(x) => out.push(x),
                TargetRef::Actor(a) => {
                    if let Some(x) = w.actors.get(a as usize).and_then(|a| a.org) {
                        out.push(x);
                    }
                }
            }
        }
        out.extend(s.engaged_by.iter().copied());
    }
    for &m in &w.orgs[o as usize].members {
        if let Some(x) = w.actors.get(m as usize).and_then(|a| a.combat.target).and_then(|t| w.actors.get(t as usize)).and_then(|t| t.org) {
            out.push(x);
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// `7B7250`: an actor with attackers or targets, in no recover state, under
/// unit_retreat_max_health, whose RECOVER goal is valid: the fight around it, to
/// unit_retreat_depth steps of who fights whom (`7BA3C0`, even steps its side,
/// odd steps the other's), and when its side's value over the other's is under
/// unit_retreat_cv_ratio, with unit_retreat_probability, it goes to its RECOVER
/// goal (`7B5210`).
fn unit_retreat(c: &Ctx<'_, '_>, player: &mut Player, index: &HashMap<OrgId, usize>, rng: &mut SaiRng, commands: &mut Vec<Command>, log: &mut Vec<String>) {
    let r = &c.ego.reactive;
    for i in 0..c.view.orgs.len() {
        let ov = &c.view.orgs[i];
        if ov.owner != c.view.me || !ov.military || player.pending(ov.id, c.now) || !ov.engaged || player.rai.recovering(ov.id) || ov.health >= r.unit_retreat_max_health || !can_recover(c, ov) {
            continue;
        }
        let mut seen: HashMap<OrgId, i32> = HashMap::new();
        seen.insert(ov.id, 0);
        let mut queue = VecDeque::from([ov.id]);
        let (mut friend, mut foe) = (0.0f32, 0.0f32);
        while let Some(t) = queue.pop_front() {
            let depth = seen[&t];
            if depth < r.unit_retreat_depth {
                for x in engage_graph(c.view, t) {
                    if !seen.contains_key(&x) {
                        seen.insert(x, depth + 1);
                        queue.push_back(x);
                    }
                }
            }
            let cv = index.get(&t).map_or(0.0, |&j| c.view.orgs[j].cv);
            if depth % 2 == 0 {
                friend += cv;
            } else {
                foe += cv;
            }
        }
        if foe > 0.0 && friend / foe < r.unit_retreat_cv_ratio && rng.unit() <= r.unit_retreat_probability {
            to_recover(c, player, i, rng, commands);
            log.push(format!("retreat {} (health {:.2})", c.def(ov.def).id, ov.health));
        }
    }
}

/// `7B5210`: out of its goal into its RECOVER goal, which sends its order.
fn to_recover(c: &Ctx<'_, '_>, player: &mut Player, o: usize, rng: &mut SaiRng, commands: &mut Vec<Command>) {
    let ov = &c.view.orgs[o];
    if player.rai.recovering(ov.id) || player.memory.assigned.get(&ov.id).is_some_and(|a| a.key == Key::Recover(ov.id)) {
        return;
    }
    let region = c.region_of(ov.pos);
    player.memory.assigned.insert(ov.id, Assignment { key: Key::Recover(ov.id), target: ov.pos, target_actor: None, thing: None, region, since: c.now, combat: false, hold: 0.0 });
    let order = retreat_order(c, player, o, None, rng);
    commands.push(order);
}

/// `7B64F0` for the RECOVER goal holding `o`: the record goes into the recover
/// state and has its order. With a supply thing named: gone, the actor comes back
/// to its region's DEFEND_REGION goal when it no longer needs recovering (morale
/// over critical, health over unit_retreat_max_health) or R is no longer valid;
/// else, or while it stands, moving or there nothing is sent, else the order again.
fn recover_think(c: &Ctx<'_, '_>, player: &mut Player, o: usize, rng: &mut SaiRng, commands: &mut Vec<Command>) {
    let ov = &c.view.orgs[o];
    let w = c.view.world;
    let state = player.rai.states.entry(ov.id).or_insert_with(|| OrderState::new(StateKind::Recover));
    state.kind = StateKind::Recover;
    let target = state.target;
    let retreat = state.retreat;
    let Some(target) = target.and_then(|t| crate::ids::target_actor(w, t)).filter(|&t| w.actor(t).is_some_and(|a| a.alive)) else {
        let order = retreat_order(c, player, o, None, rng);
        commands.push(order);
        return;
    };
    let active = supply_things(c).any(|b| b.id == target);
    if !active {
        let low_morale = ov.morale < c.ego.recover.critical_morale;
        if (!low_morale && ov.health >= c.ego.reactive.unit_retreat_max_health) || !can_recover(c, ov) {
            let region = c.region_of(ov.pos);
            player.rai.states.remove(&ov.id);
            let thing = player.memory.defend_things.get(&region).copied().flatten();
            let target_actor = thing.and_then(|t| crate::ids::target_actor(w, t));
            player.memory.assigned.insert(ov.id, Assignment { key: Key::Defend(region), target: c.regions.centers[region as usize], target_actor, thing, region, since: c.now, combat: true, hold: 0.0 });
            return;
        }
    } else if retreat.is_some() && (moving(ov) || retreat == Some(ov.pos)) {
        return;
    }
    let order = retreat_order(c, player, o, None, rng);
    commands.push(order);
}

/// One thing of a `7B01F0` hostile list: `{thing, value assigned to it}`.
struct Hostile {
    thing: Unit,
    actor: ActorId,
    pos: Vec2,
    cv: f32,
    /// Thing +D4: a target its actors walk onto (`4DDE60`, move) instead of
    /// killing it (`4CA330`).
    city: bool,
    engaged: bool,
    assigned: f32,
}

/// `7B01F0`: the things hostile to the player in the regions within `depth` of
/// `region` (`7BB1C0`) that it knows of: the mobile ones, and the structures
/// without a settlement of their own.
fn hostile_list(c: &Ctx<'_, '_>, region: u32, depth: i32) -> Vec<Hostile> {
    let w = c.view.world;
    let mut out: Vec<Hostile> = Vec::new();
    for r in c.regions.within(region, depth.max(0) as u32) {
        for h in c.view.orgs.iter().filter(|h| c.view.hostile[h.owner] && c.region_of(h.pos) == r) {
            let thing = Unit::Org(h.id);
            let Some(actor) = crate::ids::target_actor(w, thing) else { continue };
            out.push(Hostile { thing, actor, pos: h.pos, cv: h.cv, city: false, engaged: h.engaged, assigned: 0.0 });
        }
        for b in c.view.buildings.iter().filter(|b| b.owner.is_some_and(|o| c.view.hostile[o]) && !b.city && c.region_of(b.pos) == r) {
            let d = c.def(b.def);
            if d.invulnerable || !d.targetable {
                continue;
            }
            out.push(Hostile { thing: Unit::Actor(b.id), actor: b.id, pos: b.pos, cv: b.cv, city: false, engaged: false, assigned: 0.0 });
        }
    }
    out
}

/// `7B5300(hostile, actors, advantage)`: each thing in turn takes the nearest
/// actors (`7ADEC0` with `7AD450`) until they hold `advantage` times its value;
/// an actor already on it is taken off the list without an order, one that
/// cannot attack it (`4D4FF0`) waits for the next thing. What is left of the list
/// stays for the caller.
fn assign(c: &Ctx<'_, '_>, player: &mut Player, hostile: &mut [Hostile], actors: &mut Vec<usize>, advantage: f32, commands: &mut Vec<Command>) {
    let me = c.view.kingdom;
    let mut deferred: Vec<usize> = Vec::new();
    for h in hostile.iter_mut() {
        if actors.is_empty() {
            break;
        }
        let need = h.cv * advantage;
        if h.assigned > need {
            continue;
        }
        for d in deferred.drain(..) {
            actors.insert(0, d);
        }
        actors.sort_by(|&a, &b| {
            let (a, b) = (c.view.orgs[a].pos.distance_squared(h.pos), c.view.orgs[b].pos.distance_squared(h.pos));
            a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal)
        });
        while !actors.is_empty() && h.assigned <= need {
            let o = actors.remove(0);
            let ov = &c.view.orgs[o];
            if ov.cv <= 0.0 {
                deferred.insert(0, o);
                continue;
            }
            let state = player.rai.states.get(&ov.id).copied();
            if state.is_some_and(|s| s.kind == StateKind::Attack && s.target == Some(h.thing)) {
                continue;
            }
            h.assigned += ov.cv;
            let region = state.and_then(|s| s.region);
            player.rai.states.insert(ov.id, OrderState { kind: StateKind::Attack, target: Some(h.thing), region, retreat: None });
            let units = vec![Unit::Org(ov.id)];
            commands.push(if h.city { Command::Follow { kingdom: me, units, target: h.actor } } else { Command::Attack { kingdom: me, units, target: h.actor } });
        }
    }
}

/// The +5C of a DEFEND_REGION goal (`7C5C60`): guard the goal's target thing
/// (`4DD520`, flags 0x49), else stand at the centre of the region the record
/// holds, else of the goal's (`542A30`, 0x4C). The direction is the zero vector
/// of [B279C0].
pub(crate) fn guard_order(c: &Ctx<'_, '_>, o: OrgId, thing: Option<Unit>, region: u32) -> Command {
    let at = c.regions.centers[region as usize];
    Command::Order { kingdom: c.view.kingdom, units: vec![Unit::Org(o)], order: ActorOrder::Guard { target: thing, at, aux: Some(Vec2::ZERO) } }
}

/// `7B7750`: a DEFEND_REGION goal's actors that are not recovering, free or
/// busy with an attacker (`432ED0`, `4CFE90`), against the hostile things within
/// attack_distance. Outnumbered (under min_attack_advantage) only the things in a
/// fight count (`7AEDE0`). The free take targets up to min, then max advantage;
/// with none free the busy up to min. The rest hold the goal's region, and one
/// standing outside it is told to guard again.
fn defend_think(c: &Ctx<'_, '_>, player: &mut Player, region: u32, actors: &[usize], commands: &mut Vec<Command>) {
    let r = &c.ego.reactive;
    let mut free = Vec::new();
    let mut busy = Vec::new();
    for &o in actors {
        let ov = &c.view.orgs[o];
        if player.rai.recovering(ov.id) {
            continue;
        }
        if ov.engaged {
            busy.push(o);
        } else {
            free.push(o);
        }
    }
    let mut hostile = hostile_list(c, region, r.attack_distance);
    let own: f32 = free.iter().chain(busy.iter()).map(|&o| c.view.orgs[o].cv).sum();
    let foe: f32 = hostile.iter().map(|h| h.cv).sum();
    if foe != 0.0 && own / foe < r.min_attack_advantage {
        hostile.retain(|h| h.engaged);
    }
    assign(c, player, &mut hostile, &mut free, r.min_attack_advantage, commands);
    assign(c, player, &mut hostile, &mut free, r.max_attack_advantage, commands);
    if free.is_empty() {
        assign(c, player, &mut hostile, &mut busy, r.min_attack_advantage, commands);
    }
    for o in free {
        let ov = &c.view.orgs[o];
        let held = match player.rai.states.get(&ov.id) {
            Some(s) if s.kind == StateKind::Defend && s.region.is_some() => s.region,
            Some(s) if s.region.is_some() => s.region,
            _ => Some(region),
        };
        player.rai.states.insert(ov.id, OrderState { kind: StateKind::Defend, target: None, region: held, retreat: None });
        let held = held.unwrap_or(region);
        if !moving(ov) && c.region_of(ov.pos) != held {
            let thing = player.memory.defend_things.get(&held).copied().flatten();
            commands.push(guard_order(c, ov.id, thing, held));
        }
    }
}

/// `7B7D30`: an attack goal's actors that are not waiting for an order or
/// recovering: with an attacker (`432ED0`, `4CFE90`), or within attack_distance
/// of the goal's region, or a bombarder (`7A22A0`). The target thing itself first
/// (up to max advantage), then the hostile things around the region, up to min and
/// max; bombarders fire at what is in reach (`7B5970`). A settlement target comes
/// last, as a move onto it.
fn attack_think(c: &Ctx<'_, '_>, player: &mut Player, a: &Assignment, actors: &[usize], commands: &mut Vec<Command>) {
    let r = &c.ego.reactive;
    let w = c.view.world;
    let me = c.view.kingdom;
    let mut near = Vec::new();
    let mut bombarders = Vec::new();
    for &o in actors {
        let ov = &c.view.orgs[o];
        if player.pending(ov.id, c.now) || player.rai.recovering(ov.id) || ov.engaged {
            continue;
        }
        let hops = c.regions.hops(c.region_of(ov.pos), a.region) as i32;
        if super::goals::bombards(c, ov) {
            bombarders.push(o);
        } else if hops <= r.attack_distance {
            near.push(o);
        }
    }
    let target = a.thing.and_then(|t| {
        let actor = crate::ids::target_actor(w, t)?;
        let alive = w.actor(actor).is_some_and(|x| x.alive && !x.removed);
        (alive && crate::orders::hostile_to(w, me, actor)).then_some((t, actor))
    });
    let city = target.is_some_and(|(_, t)| super::goals::city_target(c, Some(t)));
    let mut hostile = Vec::new();
    if let Some((thing, actor)) = target.filter(|_| !city) {
        let cv = match thing {
            Unit::Org(o) => c.view.orgs.iter().find(|x| x.id == o).map_or(0.0, |x| x.cv),
            Unit::Actor(b) => c.view.buildings.iter().find(|x| x.id == b).map_or(0.0, |x| x.cv),
        };
        let mut first = [Hostile { thing, actor, pos: w.actors[actor as usize].pos.truncate(), cv, city: false, engaged: false, assigned: 0.0 }];
        assign(c, player, &mut first, &mut near, r.max_attack_advantage, commands);
    }
    if !near.is_empty() || !bombarders.is_empty() {
        hostile = hostile_list(c, a.region, r.attack_distance);
    }
    if !near.is_empty() && !hostile.is_empty() {
        assign(c, player, &mut hostile, &mut near, r.min_attack_advantage, commands);
        assign(c, player, &mut hostile, &mut near, r.max_attack_advantage, commands);
    }
    for o in bombarders {
        let ov = &c.view.orgs[o];
        let reach = super::goals::bombard_range(c, ov);
        let best = hostile.iter().filter(|h| h.pos.distance(ov.pos) <= reach).min_by(|x, y| x.pos.distance_squared(ov.pos).partial_cmp(&y.pos.distance_squared(ov.pos)).unwrap_or(std::cmp::Ordering::Equal));
        if let Some(h) = best {
            if player.rai.states.get(&ov.id).is_some_and(|s| s.kind == StateKind::Attack && s.target == Some(h.thing)) {
                continue;
            }
            player.rai.states.insert(ov.id, OrderState { kind: StateKind::Attack, target: Some(h.thing), region: None, retreat: None });
            commands.push(Command::Bombard { kingdom: me, units: vec![Unit::Org(ov.id)], target: crate::world::AbilityTarget::Actor(h.actor) });
        }
    }
    if let Some((thing, actor)) = target.filter(|_| city) {
        let mut last = [Hostile { thing, actor, pos: w.actors[actor as usize].pos.truncate(), cv: 0.0, city: true, engaged: false, assigned: 0.0 }];
        assign(c, player, &mut last, &mut near, r.max_attack_advantage, commands);
    }
}

/// `7BAFD0`: how a region stands for the player: 3 with a known hostile thing of
/// value in it or next to it (`7BA740`), 1 while the hostile over the own
/// influence is under threatened_region_influence_ratio (+8BC), else 2.
fn region_class(c: &Ctx<'_, '_>, region: u32) -> usize {
    let hop = c.regions.within(region, 1);
    let hostile_in = |r: u32| c.view.orgs.iter().any(|h| c.view.hostile[h.owner] && h.cv > 0.0 && c.region_of(h.pos) == r) || c.view.buildings.iter().any(|b| b.owner.is_some_and(|o| c.view.hostile[o]) && b.cv > 0.0 && c.region_of(b.pos) == r);
    if hop.iter().any(|&r| hostile_in(r)) {
        return 3;
    }
    let own = c.inf.at(&c.inf.mobile, region, c.view.me) + c.inf.at(&c.inf.settlement, region, c.view.me);
    if own != 0.0 && c.hostile(region, false) / own < c.ego.reactive.threatened_region_influence_ratio {
        1
    } else {
        2
    }
}

/// `7B4FE0`: an organization that takes a formation (actor +7C, the list of pool
/// category 9) not disabled: its formation by goal (`4A9A60`: GoalFormation of the
/// goal's type, DefaultFormation without a goal) and region class, or the
/// stopped_formation (+91C) for one at rest with a stopped formation named. A
/// formation it does not have (`4C1CC0`), or none, changes nothing.
fn formations(c: &Ctx<'_, '_>, player: &Player, commands: &mut Vec<Command>) {
    let w = c.view.world;
    // 7BAFD0 keeps each region's class for the think ([B28A8C]).
    let mut classes: HashMap<u32, usize> = HashMap::new();
    for ov in c.view.orgs.iter().filter(|o| o.owner == c.view.me) {
        let def = c.def(ov.def);
        if def.formations.len() < 2 || ov.routing {
            continue;
        }
        let pending = player.pending(ov.id, c.now);
        let wanted = if moving(ov) || pending || c.ego.stopped_formation.is_none() {
            let kind = player.memory.assigned.get(&ov.id).map(|a| match a.key {
                Key::Attack(_) => super::data::ATTACK_REGION,
                Key::Defend(_) => super::data::DEFEND_REGION,
                Key::Explore(..) => super::data::EXPLORE,
                Key::Bombard(_) => super::data::BOMBARD,
                Key::Recover(_) => RECOVER,
                _ => usize::MAX,
            });
            let list = kind.and_then(|k| c.ego.goal_formations.get(&k)).unwrap_or(&c.ego.default_formation);
            let region = c.region_of(ov.pos);
            list[*classes.entry(region).or_insert_with(|| region_class(c, region)) - 1].as_deref()
        } else {
            c.ego.stopped_formation.as_deref()
        };
        let Some(wanted) = wanted.and_then(|id| def.formations.iter().position(|f| f.id.eq_ignore_ascii_case(id))) else { continue };
        // 7B4FE0 compares with the organization's +70, what was last ordered, not with +74, what
        // its Mind state asks now (a column on the way, the ordered one when it fights).
        if crate::player_orders::desired_formation(w, ov.id) != Some(wanted) {
            commands.push(Command::SetFormation { kingdom: c.view.kingdom, orgs: vec![ov.id], formation: wanted });
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn think(view: &View<'_>, bound: &Bound, data: &AiData, regions: &Regions, inf: &Influence, player: &mut Player, rng: &mut SaiRng, log: &mut Vec<String>) {
    let ego = &data.egos[player.ego];
    let c = Ctx { view, bound, ego, regions, inf, now: view.world.time };
    let index = index_of(view);
    let r = &ego.reactive;
    player.rai.states.retain(|o, _| index.contains_key(o));
    player.memory.assigned.retain(|o, _| index.contains_key(o));
    let mut commands = Vec::new();
    unit_retreat(&c, player, &index, rng, &mut commands, log);
    // 7B64F0 over the lists of the RECOVER goals.
    let recovering: Vec<usize> = player.memory.assigned.iter().filter(|(_, a)| matches!(a.key, Key::Recover(_))).filter_map(|(o, _)| index.get(o).copied()).collect();
    for o in recovering {
        recover_think(&c, player, o, rng, &mut commands);
    }
    // The selected goals: DEFEND_REGION (7B7750), ATTACK_REGION and ATTACK_ACTOR
    // (7B7D30), each with its actors in list order.
    let mut groups: BTreeMap<Key, (Assignment, Vec<usize>)> = BTreeMap::new();
    for (o, a) in &player.memory.assigned {
        if !a.combat {
            continue;
        }
        let Some(&i) = index.get(o) else { continue };
        groups.entry(a.key).or_insert_with(|| (a.clone(), Vec::new())).1.push(i);
    }
    for (key, (a, actors)) in &groups {
        if let Key::Defend(region) = key {
            defend_think(&c, player, *region, actors, &mut commands);
        } else if matches!(key, Key::Attack(_)) {
            attack_think(&c, player, a, actors, &mut commands);
        }
    }
    formations(&c, player, &mut commands);
    let profile = &data.profiles[player.profile];
    enqueue(player, data, c.now, r.reaction_time, profile.max_rai_commands, commands);
}
