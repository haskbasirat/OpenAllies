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
//!   `52B530`/`52A930` from aa_mind::rout_select / rout_score).
//! - aa_mind::cai_engage_think step 1 (wavering while engaged -> rout) and
//!   aa_mind::cai_sleep step 2 (exhausted while resting -> rout).
//! - aa_body::experience_amounts (`571820/5718B0/5718E0`), aa_body::experience
//!   award routing `4CE770` and aa_body::promotion `571A60` (every crossed
//!   level in list order).
use crate::combat::{
    self, CombatState, EPSILON, Rout, SLEEP_THINK_SECONDS, ENGAGE_THINK_SECONDS, TargetRef,
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
    if o.routing.is_some() {
        return d(false, true, false); // mind_routing
    }
    if o.exhausted {
        return d(true, true, false); // mind_exhausted (priority 12)
    }
    if o.engaging() {
        return d(false, false, false); // mind_engaged
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
    let i = id as usize;
    if !world.orgs[i].alive {
        return;
    }
    let max = st.orgs[i].morale_max;
    let m = &mut world.orgs[i].morale;
    *m = (*m + amount).min(max);
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
    world.orgs[i].morale = (world.orgs[i].morale - amount).max(0.0);
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

/// `578F10` slayer gain of the killer's organization.
pub(crate) fn slayer(world: &mut World, st: &mut CombatState, id: OrgId, killer_value: f32, victim_value: f32) {
    if killer_value == 0.0 {
        return;
    }
    let r = st.rules();
    let factor = ((victim_value / killer_value + 1.0) * 0.5).clamp(r.morale_slayer_min, r.morale_slayer_max);
    let amount = factor * r.morale_slayer_base;
    gain(world, st, id, amount);
}

/// `579010` breaker gain of an attacker when its target routed.
fn breaker(world: &mut World, st: &mut CombatState, id: OrgId, routed: OrgId) {
    let own = st.orgs[id as usize].morale_max;
    if own == 0.0 {
        return;
    }
    let r = st.rules();
    let ratio = (st.orgs[routed as usize].morale_max / own).clamp(r.morale_breaker_min, r.morale_breaker_max);
    let amount = ratio * r.morale_breaker_base;
    gain(world, st, id, amount);
}

/// `4CE770 -> 571A60`: add experience, cross every reached level.
pub(crate) fn award(world: &mut World, st: &mut CombatState, id: OrgId, amount: f32) {
    let i = id as usize;
    if amount < EPSILON || !world.orgs[i].alive {
        return;
    }
    world.orgs[i].experience += amount;
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

/// `51F190` rout enter with the `4D3FA0` rout handler.
pub(crate) fn begin_rout(world: &mut World, st: &mut CombatState, id: OrgId) {
    let i = id as usize;
    if st.orgs[i].routing.is_some() || !world.orgs[i].alive || st.orgs[i].destroyed {
        return;
    }
    let now = world.time;
    st.orgs[i].routing = Some(Rout { since: now, destination: None, next_think: now });
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
    rout_think(world, st, id);
}

/// `521240` while routing: a destination (`52B530`) carried out by the
/// movement system through the organization's order.
fn rout_think(world: &mut World, st: &mut CombatState, id: OrgId) {
    let i = id as usize;
    let now = world.time;
    let Some(rout) = st.orgs[i].routing.clone() else { return };
    let wanted = rout.destination.map(|to| Order::Move { to, attack_move: false });
    if wanted.is_some() && wanted.as_ref() == Some(&world.orgs[i].order) {
        return;
    }
    if let Some(order) = wanted {
        // Idle after the move: arrived (Mind +144), wait for RoutingTime.
        // Any other order: the routing regiment does not take it.
        if world.orgs[i].order != Order::Idle {
            world.orgs[i].order = order;
        }
        return;
    }
    if now < rout.next_think {
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
    // 57A0A0: replenish (the supply bonus unless out of supply, +100 bit19).
    if desc.morale_replenish {
        // Organization state 19 out_of_supply (+100 bit19), kept by supply.rs.
        let in_supply = world.books.org(id).is_none_or(|b| !b.out_of_supply);
        let rate = rules.morale_replenish + if in_supply { rules.morale_supply_bonus } else { 0.0 };
        let recovery = (1.0 + lvl.morale_recovery).max(rules.clamp_multiply);
        gain(world, st, id, rate * dt * recovery);
    }
    // 57A0A0: engagement loss while targeted by an enemy (+118), from the
    // second consecutive tick; MORALE_LOSS_RATE divides it.
    let engaged = !st.orgs[i].engaged_by.is_empty();
    if st.orgs[i].engaged_latch && engaged {
        let slots = st.orgs[i].slots.max(1) as f32;
        let fraction = st.orgs[i].living.max(0) as f32 / slots;
        let mut loss = ((rules.morale_engagement_loss_factor_max - 1.0) * (1.0 - fraction) + 1.0) * rules.morale_engagement_loss * dt;
        if st.orgs[i].entrenched {
            // organization_entrenched MORALE_LOSS_RATE, when the regiment
            // itself meets the property's required_properties.
            let rate = 1.0 + st.extras().def(world.orgs[i].def).entrench_morale_loss;
            loss /= rate.max(rules.clamp_multiply);
        }
        lose(world, st, id, loss);
    }
    st.orgs[i].engaged_latch = engaged;
    if !world.orgs[i].alive || st.orgs[i].destroyed {
        return;
    }
    // 579600: engagement experience from the second consecutive engaged tick.
    if let Some(t) = st.orgs[i].current_target().filter(|_| st.orgs[i].routing.is_none()) {
        if st.orgs[i].engage_latch && combat::target_alive(world, t) {
            let (victim, fake) = target_value(world, st, t);
            let source = combat::org_value(world, st, id);
            let rate = combat::experience_ratio(victim, fake, source, &rules) * rules.experience_engage;
            award(world, st, id, rate * dt);
        }
        st.orgs[i].engage_latch = true;
    } else {
        st.orgs[i].engage_latch = false;
    }
    // 578000 desired (the organization's 501530) and 5785D0 approach.
    let desc = descriptor(world, st, id);
    let o = &mut st.orgs[i];
    o.attack_efficiency_desired = if desc.broken { 0.5 } else { 1.0 };
    if o.attack_efficiency > o.attack_efficiency_desired {
        o.attack_efficiency = (o.attack_efficiency - dt * rules.attack_efficiency_falling).max(o.attack_efficiency_desired);
    } else if o.attack_efficiency < o.attack_efficiency_desired {
        o.attack_efficiency = (o.attack_efficiency + dt * rules.attack_efficiency_climbing).min(o.attack_efficiency_desired);
    }
    // 521240: the rout lasts RoutingTime, then the Mind returns to rest.
    if let Some(rout) = st.orgs[i].routing.clone() {
        if now - rout.since >= rules.routing_time {
            st.orgs[i].routing = None;
            st.orgs[i].next_sleep_think = now + SLEEP_THINK_SECONDS;
            world.orgs[i].order = Order::Idle;
        } else {
            rout_think(world, st, id);
        }
    }
    // 57C8E0 entrenchment: entrenchment_time / ENTRENCHMENT_RATE after
    // entering an entrenchable state (resting); left when the regiment moves.
    let desc = descriptor(world, st, id);
    if desc.entrenchable {
        let since = *st.orgs[i].rest_since.get_or_insert(now);
        let delay = world.data.def(world.orgs[i].def).entrenchment_time;
        if !st.orgs[i].entrenched && delay > 0.0 && since + delay <= now {
            st.orgs[i].entrenched = true;
        }
    } else if !(st.orgs[i].engaging() && st.orgs[i].auto_order.is_some()) {
        st.orgs[i].rest_since = None;
        st.orgs[i].entrenched = false;
    }
    // Think reactions: engage (1.0 s) wavering while targeted, sleep (2.0 s)
    // exhausted while resting.
    if st.orgs[i].routing.is_none() && now >= st.orgs[i].next_sleep_think {
        let engaging = st.orgs[i].engaging();
        st.orgs[i].next_sleep_think = now + if engaging { ENGAGE_THINK_SECONDS } else { SLEEP_THINK_SECONDS };
        let s = state(&st.orgs[i], world.orgs[i].morale);
        if (engaging && s == 1 && engaged) || (!engaging && s == 0) {
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
