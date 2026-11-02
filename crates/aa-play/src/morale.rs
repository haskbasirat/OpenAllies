//! Morale loss under fire, the breakpoint, routing and regrouping,
//! entrenchment, attack efficiency and experience. Owner: combat.
//!
//! Plain `f32` rendition of the exact ports (read them for the native detail):
//! - aa_combat::morale: state `578140` (0 exhausted at morale 0, 1 wavering
//!   at or below the breakpoint, 2 steady), gain `5786D0`, loss `579660` and
//!   its transition reactions (`51F9F0` exhausted, `51F920` wavering, both
//!   in aa_mind::cai_reactions), slayer `578F10`, breaker `579010`, retreat
//!   penalty `579AB0`, the morale tick `57A0A0` (MoraleReplenish +
//!   MoraleSupplyBonus while the Mind state allows morale_replenish; the
//!   engagement loss `((FactorMax-1)*(1-living/slots)+1)*Loss*dt` while the
//!   organization is engaged by an enemy), attack efficiency `5785D0`,
//!   engagement experience `579600`, rout handler `4D3FA0`.
//! - aa_combat::organization: member removal `57CD70` (MoraleDeathFactor *
//!   member morale_value, called from combat.rs), the entrenchment check of
//!   `57C8E0` (entrenchment_time / ENTRENCHMENT_RATE in an entrenchable state).
//! - aa_mind::cai_rout: rout enter `51F190` (halt, engage list cleared,
//!   pending commands released), Think `521240` (RoutingTime, destination
//!   `52B530`/`52A930` from aa_mind::rout_select / rout_score). The rout
//!   ends through the CAI Mind: RoutingTime runs out (`+178`, `522B90`) or
//!   the destination is reached (`+144`, `5222E0`); a regiment still
//!   exhausted (`578140` state 0) takes the exhausted state (`51FAE0`,
//!   Think `51E260`: rest until the morale is steady), otherwise the
//!   time-out returns it to rest and the arrival to the reform state
//!   (aa_mind::cai_thinks::reform `51DE70`).
//! - aa_mind::cai_engage_think step 1 (wavering while engaged -> rout) and
//!   aa_mind::cai_sleep step 2 (exhausted while resting -> rout).
//! - aa_body::experience_amounts (`571820/5718B0/5718E0`), aa_body::experience
//!   award routing `4CE770` and aa_body::promotion `571A60` (every crossed
//!   level in list order).
//!
//! The arithmetic follows the ports' x87 chains at PC53 with the original
//! binary32 stores (`combat::x87`).
use crate::combat::{
    self, CombatState, EPSILON, Reform, Rout, SLEEP_THINK_SECONDS, ENGAGE_THINK_SECONDS, TargetRef,
    x87::{self, load as ld, store as fstp},
};
use crate::map::{BLOCKED, NOT_LAND_PASSABLE};
use crate::world::{Event, Order, OrgId, World};
use crate::DT;
use glam::Vec2;

pub fn tick(world: &mut World) {
    let mut st = std::mem::take(&mut world.combat);
    st.prepare(world);
    for id in 0..world.orgs.len() as OrgId {
        org_tick(world, &mut st, id);
    }
    world.combat = st;
}

/// The Mind state descriptor of actor state group 4 (actor_states.tgi /
/// actor_states_aa.tgi): morale_replenish, broken formation (the layout's
/// BrokenProperty, ATTACK_EFFICIENCY_DESIRED x0.5) and entrenchable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Descriptor {
    pub morale_replenish: bool,
    pub broken: bool,
    pub entrenchable: bool,
}

pub fn descriptor(world: &World, st: &CombatState, id: OrgId) -> Descriptor {
    let o = &st.orgs[id as usize];
    let d = |morale_replenish, broken, entrenchable| Descriptor { morale_replenish, broken, entrenchable };
    // 4D49B0(4): the regiment's exclusive Mind state. Actor state 5
    // (mind_exhausted, set by the morale at zero) is outside group 4 (0x474270
    // gives it another group, 0x477FC0). The exhausted Mind state (Mind +270,
    // vtable 0xA32AEC) reports actor state 0xC at vtable +0x20 (0x7C5440):
    // mind_resting, so a regiment in it reads as resting below.
    if o.routing.is_some() {
        return d(false, true, false); // mind_routing
    }
    if o.engaging() {
        return d(false, false, false); // mind_engaged
    }
    if o.reform.is_some() {
        return d(true, false, false); // mind_reforming (DESIRED formation)
    }
    match world.orgs[id as usize].order {
        Order::Idle => d(true, false, true),           // mind_resting
        Order::Attack { .. } => d(false, false, false), // mind_attacking
        _ => d(false, false, false),                    // mind_moving
    }
}

/// `578140` (a busy Mind and the `+5C` decline are not modelled).
pub fn state(o: &combat::OrgCombat, morale: f32) -> u8 {
    if o.morale_max <= 0.0 || morale > o.breakpoint {
        2
    } else if morale == 0.0 {
        0
    } else {
        1
    }
}

fn level<'a>(world: &World, st: &'a CombatState, id: OrgId) -> Option<&'a combat::ExperienceLevel> {
    let s = &st.orgs[id as usize];
    st.extras().def(world.orgs[id as usize].def).levels.get(s.level)
}

/// Maximum morale (the elements' morale values with the level MORALE add),
/// the breakpoint and the members' maximum health.
fn refresh_level_values(world: &mut World, st: &mut CombatState, id: OrgId) {
    let i = id as usize;
    let data = world.data.clone();
    let def = data.def(world.orgs[i].def);
    let (morale_add, breakpoint_add, health) =
        level(world, st, id).map_or((0.0, 0.0, 0.0), |l| (l.morale, l.morale_breakpoint, l.health));
    let mut max = 0.0;
    if def.slots.is_empty() {
        for &m in &world.orgs[i].members {
            max += data.def(world.actors[m as usize].def).morale_value + morale_add;
        }
    } else {
        for &(d, _) in def.slots.iter().filter(|(d, _)| *d != u32::MAX) {
            max += data.def(d).morale_value + morale_add;
        }
    }
    st.orgs[i].morale_max = max;
    st.orgs[i].breakpoint = def.morale_breakpoint - breakpoint_add;
    let _ = health;
    for m in world.orgs[i].members.clone() {
        // (core) 502A70: the level amount adds to the handicap/technology
        // HEALTH amounts (and the enchantments') in one multiplier.
        crate::combat::refresh_max_health(world, st, m);
    }
}

/// First sight of an organization: its level from its experience (e.g. a
/// StartingExperience set by production), its maximum morale, and full
/// morale (the value written by whoever created the organization is
/// replaced: a new OrganizationComponent starts at its maximum).
pub(crate) fn init_org(world: &mut World, st: &mut CombatState, id: OrgId) {
    let i = id as usize;
    let def = world.orgs[i].def;
    let levels = &st.extras().def(def).levels;
    let xp = world.orgs[i].experience;
    st.orgs[i].level = levels.iter().rposition(|l| l.required <= xp).unwrap_or(0);
    refresh_level_values(world, st, id);
    world.orgs[i].morale = st.orgs[i].morale_max;
}

/// `5786D0`: add, clamp to the maximum, clear `mind_exhausted` unless at 0.
pub(crate) fn gain(world: &mut World, st: &mut CombatState, id: OrgId, amount: f32) {
    gain_x87(world, st, id, ld(amount));
}

/// The shared tail of `5786D0`/`578F10`/`579010` (aa_combat::morale
/// `gain_amount`): the x87 `amount` plus the morale, stored, then clamped to
/// the maximum; state 5 is cleared unless the state is exhausted.
fn gain_x87(world: &mut World, st: &mut CombatState, id: OrgId, amount: f64) {
    let i = id as usize;
    if !world.orgs[i].alive {
        return;
    }
    let max = st.orgs[i].morale_max;
    let m = &mut world.orgs[i].morale;
    *m = fstp(x87::add(amount, ld(*m)));
    if max < *m {
        *m = max;
    }
    if state(&st.orgs[i], world.orgs[i].morale) != 0 {
        st.orgs[i].exhausted = false;
    }
}

/// `579660` / `lose_amount`: subtract (not below 0); a transition to
/// exhausted or wavering triggers the Mind reaction.
pub(crate) fn lose(world: &mut World, st: &mut CombatState, id: OrgId, amount: f32) {
    let i = id as usize;
    if !world.orgs[i].alive || st.orgs[i].destroyed || amount <= 0.0 {
        return;
    }
    let before = state(&st.orgs[i], world.orgs[i].morale);
    let m = fstp(x87::sub(ld(world.orgs[i].morale), ld(amount)));
    world.orgs[i].morale = if m < 0.0 { 0.0 } else { m };
    let after = state(&st.orgs[i], world.orgs[i].morale);
    if after == 0 {
        st.orgs[i].exhausted = true;
    }
    if after != before {
        match after {
            0 => exhausted_reaction(world, st, id),
            1 => begin_rout(world, st, id), // 51F920 wavering
            _ => {}
        }
    }
}

/// `51F9F0`: no rout while engaging or already routing.
fn exhausted_reaction(world: &mut World, st: &mut CombatState, id: OrgId) {
    let o = &st.orgs[id as usize];
    if o.engaging() || o.routing.is_some() {
        return;
    }
    begin_rout(world, st, id);
}

/// `579AB0` on a retreat order out of an engagement (MoraleRetreatPenalty).
pub(crate) fn retreat_penalty(world: &mut World, st: &mut CombatState, id: OrgId) {
    let p = st.rules().morale_retreat_penalty;
    lose(world, st, id, p);
}

/// `578F10` slayer gain of the killer's organization:
/// `((victim/killer + 1) * 0.5)` clamped to [MoraleSlayerFactorMin,
/// MoraleSlayerFactorMax], times MoraleSlayerBase, all unstored.
pub(crate) fn slayer(world: &mut World, st: &mut CombatState, id: OrgId, killer_value: f32, victim_value: f32) {
    if killer_value == 0.0 {
        return;
    }
    let r = st.rules();
    let mut factor = x87::mul(x87::add(x87::div(ld(victim_value), ld(killer_value)), 1.0), 0.5);
    if factor < ld(r.morale_slayer_min) {
        factor = ld(r.morale_slayer_min);
    } else if factor > ld(r.morale_slayer_max) {
        factor = ld(r.morale_slayer_max);
    }
    let amount = x87::mul(factor, ld(r.morale_slayer_base));
    gain_x87(world, st, id, amount);
}

/// `579010` breaker gain of an attacker when its target routed: the routed
/// maximum over this maximum, clamped to [MoraleBreakerFactorMin,
/// MoraleBreakerFactorMax], times MoraleBreakerBase, unstored.
fn breaker(world: &mut World, st: &mut CombatState, id: OrgId, routed: OrgId) {
    let own = st.orgs[id as usize].morale_max;
    if own == 0.0 {
        return;
    }
    let r = st.rules();
    let mut ratio = x87::div(ld(st.orgs[routed as usize].morale_max), ld(own));
    if ratio < ld(r.morale_breaker_min) {
        ratio = ld(r.morale_breaker_min);
    } else if ratio > ld(r.morale_breaker_max) {
        ratio = ld(r.morale_breaker_max);
    }
    let amount = x87::mul(ratio, ld(r.morale_breaker_base));
    gain_x87(world, st, id, amount);
}

/// `4CE770 -> 571A60`: add experience, cross every reached level.
pub(crate) fn award(world: &mut World, st: &mut CombatState, id: OrgId, amount: f32) {
    let i = id as usize;
    if amount < EPSILON || !world.orgs[i].alive {
        return;
    }
    world.orgs[i].experience = fstp(x87::add(ld(world.orgs[i].experience), ld(amount)));
    let levels_len = st.extras().def(world.orgs[i].def).levels.len();
    let mut changed = false;
    while st.orgs[i].level + 1 < levels_len {
        let next = &st.extras().def(world.orgs[i].def).levels[st.orgs[i].level + 1];
        if next.required > world.orgs[i].experience {
            break;
        }
        st.orgs[i].level += 1;
        changed = true;
        world.events.push(Event::Promoted { org: id, level: st.orgs[i].level as u32 });
    }
    if changed {
        refresh_level_values(world, st, id);
    }
}

fn target_value(world: &World, st: &CombatState, t: TargetRef) -> (f32, f32) {
    match t {
        TargetRef::Org(o) => (combat::org_value(world, st, o), st.extras().def(world.orgs[o as usize].def).fake_cv_value),
        TargetRef::Actor(a) => {
            let ex = st.extras().def(world.actors[a as usize].def);
            (ex.combat_value, ex.fake_cv_value)
        }
    }
}

/// `52A810(rout, 2)` from a morale reaction: the rout state replaces the
/// Mind's states (`52A420`) unless the regiment is routing already.
pub(crate) fn begin_rout(world: &mut World, st: &mut CombatState, id: OrgId) {
    let i = id as usize;
    if st.orgs[i].routing.is_some() || !world.orgs[i].alive || st.orgs[i].destroyed {
        return;
    }
    enter_rout(world, st, id);
}

/// `52A420` + `51F190` KKC_CAIStateRout::OnEnter with the `4D3FA0` rout
/// handler: the states are cleared (a running rout move stops, `51E130`),
/// the entry time is now and the destination is chosen anew.
fn enter_rout(world: &mut World, st: &mut CombatState, id: OrgId) {
    let i = id as usize;
    let now = world.time;
    st.orgs[i].routing = Some(Rout { since: now, destination: None, next_think: now });
    st.orgs[i].reform = None;
    st.orgs[i].exhausted_state = false;
    world.events.push(Event::Routed { org: id });
    // 4D3FA0: every live engaged-by attacker gets breaker morale and the rout experience.
    let attackers = st.orgs[i].engaged_by.clone();
    let rules = st.rules.clone().expect("prepared");
    for a in attackers {
        if !world.orgs.get(a as usize).is_some_and(|o| o.alive) {
            continue;
        }
        breaker(world, st, a, id);
        let (victim, fake) = target_value(world, st, TargetRef::Org(id));
        let source = combat::org_value(world, st, a);
        let amount = combat::experience_clamped(combat::experience_ratio(victim, fake, source, &rules), rules.experience_rout, &rules);
        award(world, st, a, amount);
    }
    // 51F040 engage list cleared, 52C0D0 pending commands released.
    let o = &mut st.orgs[i];
    o.engage.clear();
    o.explicit = None;
    o.saved_order = None;
    o.auto_order = None;
    o.entrenched = false;
    o.rest_since = None;
    for m in world.orgs[i].members.clone() {
        world.actors[m as usize].combat.target = None;
    }
    // The first Think: no move is active, so a destination is chosen.
    rout_destination(world, st, id);
}

/// binary32 0.5: `52A1F0` arrival distance of the rout move.
const ARRIVAL: f32 = 0.5;

/// `521240` while routing: RoutingTime after the entry ends the rout
/// (`+178`); otherwise a destination (`52B530`, a retry in 2.0 s when none
/// is found) carried out by the movement system through the organization's
/// order, until the regiment is within 0.5 of it (`52A1F0` -> `+144`). A
/// move the movement system ended elsewhere is a finished move state: a new
/// destination is chosen. The routing regiment takes no other order.
fn rout_think(world: &mut World, st: &mut CombatState, id: OrgId) {
    let i = id as usize;
    let now = world.time;
    let Some(rout) = st.orgs[i].routing.clone() else { return };
    let routing_time = st.rules().routing_time;
    // `fcomp; test ah,1; jne`: the time-out unless (now - entry) < RoutingTime.
    if !(x87::sub(ld(now), ld(rout.since)) < ld(routing_time)) {
        rout_over(world, st, id, false);
        return;
    }
    if let Some(to) = rout.destination {
        let wanted = Order::Move { to, attack_move: false };
        // 52A1F0: dx stored, dy*dy + dx*dx below 0.25 ([A033BC]).
        let near = |p: Vec2| {
            let dx = ld(fstp(x87::sub(ld(to.x), ld(p.x))));
            let dy = x87::sub(ld(to.y), ld(p.y));
            x87::add(x87::mul(dy, dy), x87::mul(dx, dx)) < ld(ARRIVAL * ARRIVAL)
        };
        if near(st.orgs[i].position) || (world.orgs[i].order == Order::Idle && near(world.orgs[i].anchor)) {
            rout_over(world, st, id, true);
            return;
        }
        if world.orgs[i].order == wanted {
            return;
        }
        if world.orgs[i].order != Order::Idle {
            world.orgs[i].order = wanted;
            return;
        }
        // The move ended short of the destination: choose another.
        if let Some(r) = st.orgs[i].routing.as_mut() {
            r.destination = None;
            r.next_think = now;
        }
    }
    rout_destination(world, st, id);
}

/// `521240` with the move state inactive: `52B530` chooses a destination
/// and the move starts (`520440`), or the Think retries in 2.0 s.
fn rout_destination(world: &mut World, st: &mut CombatState, id: OrgId) {
    let i = id as usize;
    let now = world.time;
    let next = st.orgs[i].routing.as_ref().map_or(now, |r| r.next_think);
    if now < next {
        return;
    }
    match select_destination(world, st, id) {
        Some(to) => {
            world.orgs[i].order = Order::Move { to, attack_move: false };
            if let Some(r) = st.orgs[i].routing.as_mut() {
                r.destination = Some(to);
            }
        }
        None => {
            if let Some(r) = st.orgs[i].routing.as_mut() {
                r.next_think = now + SLEEP_THINK_SECONDS;
            }
        }
    }
}

/// The rout's end through the CAI Mind: RoutingTime (`+178`, `522B90`) or
/// the destination reached (`+144`, `5222E0`). Still exhausted (`578140`
/// state 0): `51FAE0` puts the exhausted state (Mind `+270`) in place of the
/// rout (`52A810(2)`). Otherwise the time-out pops the rout (`52A340`, back
/// to rest) and the arrival enters the reform state (`52A810(reform, 1)`).
fn rout_over(world: &mut World, st: &mut CombatState, id: OrgId, arrived: bool) {
    let i = id as usize;
    let now = world.time;
    st.orgs[i].routing = None;
    st.orgs[i].next_sleep_think = now + SLEEP_THINK_SECONDS;
    world.orgs[i].order = Order::Idle;
    if state(&st.orgs[i], world.orgs[i].morale) == 0 {
        st.orgs[i].exhausted_state = true;
    } else if arrived {
        st.orgs[i].reform = Some(Reform { since: now, next_think: now });
    }
}

/// `51DE70` KKC_CAIStateReform::Think (aa_mind::cai_thinks::reform): while
/// current, ReformTime after the start ends it (`+178` -> `+11C`, rest);
/// otherwise the next Think in 2.0 s, or in 1.0 s while the leader is still
/// moving (`4CEDE0`: its current action is its mover), else it ends (`+11C`).
fn reform_think(world: &mut World, st: &mut CombatState, id: OrgId) {
    let i = id as usize;
    let now = world.time;
    let Some(r) = st.orgs[i].reform.clone() else { return };
    if st.orgs[i].engaging() || st.orgs[i].routing.is_some() || world.orgs[i].order != Order::Idle {
        // Another Mind state took over (an engagement or a new order).
        st.orgs[i].reform = None;
        return;
    }
    if now < r.next_think {
        return;
    }
    let reform_time = st.rules().reform_time;
    if !(x87::sub(ld(now), ld(r.since)) < ld(reform_time)) {
        st.orgs[i].reform = None;
        return;
    }
    let leader = world.orgs[i].members.iter().copied().find(|&m| combat::alive(world, m));
    let moving = leader.is_some_and(|l| world.actors[l as usize].motion.velocity.length_squared() > EPSILON);
    if moving {
        if let Some(r) = st.orgs[i].reform.as_mut() {
            r.next_think = now + 1.0;
        }
    } else {
        st.orgs[i].reform = None;
    }
}

/// `52A690`: how much of `delta` fits along `dir` beyond `base` inside the map.
fn reachable(world: &World, origin: Vec2, dir: Vec2, base: f32, delta: f32) -> f32 {
    let map = &world.map;
    if map.width == 0 || map.height == 0 {
        return delta;
    }
    let size = Vec2::new(map.width as f32 * map.cell_size, map.height as f32 * map.cell_size);
    let mut t_max = f32::MAX;
    for k in 0..2 {
        let (o, d, s) = (origin[k], dir[k], size[k]);
        if d > 1e-6 {
            t_max = t_max.min((s - o) / d);
        } else if d < -1e-6 {
            t_max = t_max.min(-o / d);
        }
    }
    (t_max - base).clamp(0.0, delta)
}

/// `4F46E0` reduced to the passability of the candidate cell.
fn open(world: &World, p: Vec2) -> bool {
    let map = &world.map;
    if map.width == 0 || map.height == 0 {
        return true;
    }
    map.cell_flags(p) & (BLOCKED | NOT_LAND_PASSABLE) == 0
}

fn normalized(v: Vec2) -> Vec2 {
    let l = v.length();
    if l > EPSILON { v / l } else { Vec2::new(0.0, -1.0) }
}

/// `52B530` with the `52A930` score (enemy distance and angle, moved
/// distance, allied supply sources, fuzziness). The enemy ZOC/ZOG penalties
/// and the friendly ZOS bonus need the zone solver and are not applied.
fn select_destination(world: &mut World, st: &CombatState, id: OrgId) -> Option<Vec2> {
    let i = id as usize;
    let r = st.rules.clone().expect("prepared");
    let o = &st.orgs[i];
    let origin = o.position;
    let def = world.data.def(world.orgs[i].def);
    let min_distance = def.control_range * if o.exhausted { 0.75 } else { 1.0 };
    let enemies: Vec<Vec2> = o
        .engaged_by
        .iter()
        .filter(|&&e| world.orgs.get(e as usize).is_some_and(|x| x.alive))
        .map(|&e| st.orgs[e as usize].position)
        .collect();
    // Kingdom +264 supply sources of allied kingdoms (supply.rs zones).
    let team = world.kingdoms.iter().find(|k| k.id == world.orgs[i].kingdom).map(|k| k.team);
    let sources: Vec<(Vec2, f32)> = world
        .books
        .zones
        .iter()
        .filter(|z| team.is_some() && world.kingdoms.iter().find(|k| k.id == z.kingdom).map(|k| k.team) == team)
        .map(|z| (z.centre, z.radius))
        .collect();
    let mut best: Option<(f32, Vec2)> = None;
    let mut offer = |world: &mut World, cand: Vec2| {
        if !open(world, cand) {
            return;
        }
        let cand_dir = normalized(cand - origin);
        let mut score = r.rout_base_value;
        let n = enemies.len() as f32;
        for &e in &enemies {
            let enemy_dir = normalized(e - origin);
            let distance = e.distance(cand);
            let angle = enemy_dir.angle_to(cand_dir).abs().to_degrees();
            score += distance.min(r.rout_factor_enemy_dist_cap) * r.rout_factor_enemy_dist / n + angle * r.rout_factor_enemy_angle / n;
        }
        score -= cand.distance(origin).min(r.rout_factor_moved_dist_cap) * r.rout_factor_moved_dist;
        let mut nearest = f32::MAX;
        for &(s, radius) in &sources {
            let d2 = cand.distance_squared(s);
            nearest = nearest.min(d2);
            if d2 < radius * radius {
                score += r.rout_bonus_supply_in_range;
            }
        }
        if nearest < f32::MAX {
            score -= nearest.sqrt().min(r.rout_factor_supply_dist_cap) * r.rout_factor_supply_dist;
        }
        score += world.random() * r.rout_fuzziness;
        if best.is_none_or(|(b, _)| score > b) {
            best = Some((score, cand));
        }
    };
    for _ in 0..r.rout_random_candidates {
        let angle = world.random() * std::f32::consts::TAU;
        let dir = Vec2::new(angle.cos(), angle.sin());
        let allowed = reachable(world, origin, dir, min_distance, r.rout_max_distance_delta);
        let distance = min_distance + world.random() * allowed;
        let cand = origin + dir * distance;
        if cand != origin {
            offer(world, cand);
        }
    }
    for &(s, _) in &sources {
        for _ in 0..r.rout_supply_candidates {
            let to = normalized(s - origin);
            let spread = (world.random() * 2.0 * r.rout_supply_max_angle - r.rout_supply_max_angle).to_radians();
            let dir = Vec2::from_angle(spread).rotate(to);
            let allowed = reachable(world, origin, dir, min_distance, r.rout_max_distance_delta);
            let distance = min_distance + world.random() * allowed;
            offer(world, origin + dir * distance);
        }
    }
    best.map(|(_, p)| p)
}

/// One organization's morale tick (`57C8E0` order: morale `57A0A0`,
/// engagement experience `579600`, attack efficiency `5785D0`, rout,
/// entrenchment, the engage/sleep Think reactions).
fn org_tick(world: &mut World, st: &mut CombatState, id: OrgId) {
    let i = id as usize;
    if !world.orgs[i].alive || st.orgs[i].destroyed {
        return;
    }
    let now = world.time;
    let dt = DT;
    let rules = st.rules.clone().expect("prepared");
    let desc = descriptor(world, st, id);
    let lvl = level(world, st, id).cloned().unwrap_or_default();
    // 57A0A0: replenish (the supply bonus unless out of supply, +100 bit19):
    // fstp(rate * dt), then fstp(that * MORALE_RECOVERY_RATE (+44)).
    if desc.morale_replenish {
        // Organization state 19 out_of_supply (+100 bit19), kept by supply.rs.
        let in_supply = world.books.org(id).is_none_or(|b| !b.out_of_supply);
        let mut rate = ld(rules.morale_replenish);
        if in_supply {
            rate = x87::add(rate, ld(rules.morale_supply_bonus));
        }
        let recovery = (1.0 + lvl.morale_recovery).max(rules.clamp_multiply);
        let amount = fstp(x87::mul(ld(fstp(x87::mul(rate, ld(dt)))), ld(recovery)));
        gain(world, st, id, amount);
    }
    // 57A0A0: engagement loss while targeted by an enemy (+118), from the
    // second consecutive tick: fstp(((FactorMax - 1) * (1 - living/slots)
    // + 1) * Loss * dt), then fstp(that / MORALE_LOSS_RATE (+48)).
    let engaged = !st.orgs[i].engaged_by.is_empty();
    if st.orgs[i].engaged_latch && engaged {
        let slots = st.orgs[i].slots.max(1);
        let fraction = x87::div(f64::from(st.orgs[i].living), f64::from(slots));
        let factor = x87::add(x87::mul(x87::sub(ld(rules.morale_engagement_loss_factor_max), 1.0), x87::sub(1.0, fraction)), 1.0);
        let mut loss = fstp(x87::mul(x87::mul(factor, ld(rules.morale_engagement_loss)), ld(dt)));
        if st.orgs[i].entrenched {
            // organization_entrenched MORALE_LOSS_RATE, when the regiment
            // itself meets the property's required_properties.
            let rate = 1.0 + st.extras().def(world.orgs[i].def).entrench_morale_loss;
            loss = fstp(x87::div(ld(loss), ld(rate.max(rules.clamp_multiply))));
        }
        lose(world, st, id, loss);
    }
    st.orgs[i].engaged_latch = engaged;
    if !world.orgs[i].alive || st.orgs[i].destroyed {
        return;
    }
    // 579600: engagement experience from the second consecutive engaged
    // tick, fstp(5718B0 ratio * ExperienceEngageMultiplier * dt).
    if let Some(t) = st.orgs[i].current_target().filter(|_| st.orgs[i].routing.is_none()) {
        if st.orgs[i].engage_latch && combat::target_alive(world, t) {
            let (victim, fake) = target_value(world, st, t);
            let source = combat::org_value(world, st, id);
            let rate = x87::mul(combat::experience_ratio(victim, fake, source, &rules), ld(rules.experience_engage));
            award(world, st, id, fstp(x87::mul(rate, ld(dt))));
        }
        st.orgs[i].engage_latch = true;
    } else {
        st.orgs[i].engage_latch = false;
    }
    // 578000 desired (the organization's 501530) and 5785D0 approach:
    // fstp(actual - dt * falling) not below the desired value, or
    // fstp(dt * climbing + actual) not above it.
    let desc = descriptor(world, st, id);
    let o = &mut st.orgs[i];
    o.attack_efficiency_desired = if desc.broken { 0.5 } else { 1.0 };
    if o.attack_efficiency > o.attack_efficiency_desired {
        o.attack_efficiency = fstp(x87::sub(ld(o.attack_efficiency), x87::mul(ld(dt), ld(rules.attack_efficiency_falling))));
        if o.attack_efficiency_desired > o.attack_efficiency {
            o.attack_efficiency = o.attack_efficiency_desired;
        }
    } else if o.attack_efficiency < o.attack_efficiency_desired {
        o.attack_efficiency = fstp(x87::add(x87::mul(ld(dt), ld(rules.attack_efficiency_climbing)), ld(o.attack_efficiency)));
        if o.attack_efficiency_desired < o.attack_efficiency {
            o.attack_efficiency = o.attack_efficiency_desired;
        }
    }
    // 521240 rout Think (RoutingTime, destination, arrival), then the
    // reform Think 51DE70 after an arrival.
    if st.orgs[i].routing.is_some() {
        rout_think(world, st, id);
    }
    reform_think(world, st, id);
    // 57C8E0 entrenchment: the unstored 45D520 delay plus the +B4 start not
    // after now, once in an entrenchable state (resting); left when the
    // regiment moves.
    let desc = descriptor(world, st, id);
    if desc.entrenchable {
        let since = *st.orgs[i].rest_since.get_or_insert(now);
        let delay = world.data.def(world.orgs[i].def).entrenchment_time;
        if !st.orgs[i].entrenched && delay > 0.0 && x87::add(ld(delay), ld(since)) <= ld(now) {
            st.orgs[i].entrenched = true;
        }
    } else if !(st.orgs[i].engaging() && st.orgs[i].auto_order.is_some()) {
        st.orgs[i].rest_since = None;
        st.orgs[i].entrenched = false;
    }
    // Think reactions: engage (1.0 s, 526A70 step 1) wavering while
    // targeted; the exhausted state (51E260) ends once the morale is steady
    // (state 2), its embedded sleep Think not being current; otherwise the
    // sleep Think (2.0 s, 524D70 step 2) routs an exhausted regiment.
    // A new order replaces the exhausted Mind state (52A420), as it does the
    // reform state above.
    if st.orgs[i].exhausted_state && world.orgs[i].order != Order::Idle {
        st.orgs[i].exhausted_state = false;
    }
    if st.orgs[i].routing.is_none() && now >= st.orgs[i].next_sleep_think {
        let engaging = st.orgs[i].engaging();
        st.orgs[i].next_sleep_think = now + if engaging { ENGAGE_THINK_SECONDS } else { SLEEP_THINK_SECONDS };
        let s = state(&st.orgs[i], world.orgs[i].morale);
        if engaging {
            if s == 1 && engaged {
                begin_rout(world, st, id);
            }
        } else if st.orgs[i].exhausted_state {
            if s == 2 {
                st.orgs[i].exhausted_state = false;
            }
        } else if s == 0 {
            begin_rout(world, st, id);
        }
    }
    st.orgs[i].previous_order = world.orgs[i].order.clone();
}

/// Morale state (0 exhausted, 1 wavering, 2 steady) of an organization.
pub fn morale_state(world: &World, org: OrgId) -> Option<u8> {
    let s = world.combat.org(org)?;
    Some(state(s, world.orgs.get(org as usize)?.morale))
}
