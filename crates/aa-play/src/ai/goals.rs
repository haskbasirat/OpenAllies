//! The SAI goal engine of one player (`7B4AD0` -> `7A9840`): ego choice,
//! candidate goals with their class base priorities, the `7A5200`
//! finalization, the `7A93B0`/`7A9150`/`7A7E30` selection and actor
//! assignment, and goal execution as commands.
//!
//! Goal classes played here and the original they follow:
//! - RECRUIT: the recruit planner `7CFEA0` (SpecificRecruitRequest list,
//!   `num_military_goals` military requests, at most `max_goals_per_think`
//!   goals, `default_max_recruited_per_think` per actor type), base priority
//!   set at creation (class `+78` is a bare `ret`, `4B2830`), affordability
//!   `7A3860` with `saving_time_limit` of income;
//! - UNPACK: `7CBC40` (regions within `adjacent_region_depth`, random
//!   island regions, the previous choice; the best `4A9AE0` site value);
//! - RECOVER: `7C6640` (`(1 - health^exponent_hp) * base_priority_hp`
//!   below `max_hp`, the same with the morale fraction);
//! - DEFEND_REGION: `7C5F80` -> `7BD360` region threat;
//! - ATTACK_REGION: `7C5AE0` -> `7C6BB0`/`7BCCB0` juiciest target value;
//! - EXPLORE: `7C29F0` (deep and close exploration goals);
//! - BOMBARD: one per foreign structure (`7A83A0`), `7C8750` base,
//!   `7C6500` actor value, `7C8340`/`7C84F0` orders;
//! - military priority with actors `7A3B00` -> `7C9080` -> `7ACF90` CV curve
//!   (`7C6A70` target/maximum CV, linked goals), ActorPreference modifiers
//!   and the travel weight `4A7130`.
use super::data::{self, AiData, Ego};
use super::regions::{Influence, Regions};
use super::value::{Census, Val};
use super::view::{View, has};
use super::{Bound, Player, SaiRng, enqueue};
use crate::data::{DefClass, DefId};
use crate::map::{BLOCKED, NOT_LAND_PASSABLE};
use crate::world::{AbilityTarget, ActorId, Command, Order, OrgId, Unit};
use glam::Vec2;
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Key {
    Unpack(ActorId),
    Recover(OrgId),
    Defend(u32),
    Attack(u32),
    Explore(u8, u8),
    /// Request index (`u32::MAX - n` for the n-th military request) and def.
    Recruit(u32, DefId),
    /// Researching building and the technology's index in its list.
    Research(ActorId, u32),
    Attach(OrgId),
    /// BOMBARD of a target actor (goal `+3C`).
    Bombard(ActorId),
    /// AIR_ATTACK on a target actor (goal `+3C`).
    AirAttack(ActorId),
    /// REPAIR of a damaged actor (goal `+3C`).
    Repair(ActorId),
}

#[derive(Clone, Debug)]
pub struct Assignment {
    pub key: Key,
    pub target: Vec2,
    pub target_actor: Option<ActorId>,
    /// Goal +4C: the thing the goal aims at (an organization or a structure).
    pub thing: Option<Unit>,
    pub region: u32,
    pub since: f32,
    /// Attack or defend goal (combat), recover, explore.
    pub combat: bool,
    /// What the actor was worth to the goal at the last think ([+2C], `7C5870`).
    pub hold: f32,
}

#[derive(Clone, Debug, Default)]
pub struct UnpackMemo {
    pub region: u32,
    pub site: Vec2,
    pub ordered_at: f32,
    /// Battle time the Deploy command left the send queue (0 = not yet).
    pub sent_at: f32,
    pub bad: Vec<u32>,
}

/// What the goal engine keeps between thinks.
#[derive(Clone, Debug, Default)]
pub struct Memory {
    /// Goals selected by the last think (status 2) and their fuzzy amount.
    pub selected: BTreeMap<Key, f32>,
    pub assigned: BTreeMap<OrgId, Assignment>,
    pub explore_region: BTreeMap<Key, u32>,
    pub explore_actor: BTreeMap<Key, OrgId>,
    pub unpack: BTreeMap<ActorId, UnpackMemo>,
    /// Recruit orders given: (time, def).
    pub recruited: Vec<(f32, DefId)>,
    /// Deploy orders given: (time, truck def).
    pub deployed: Vec<(f32, DefId)>,
    /// Attack orders given: (time, region, organizations).
    pub attacks: Vec<(f32, u32, usize)>,
    /// AIR_ATTACK goal `+40`: the battle time each target was last air
    /// attacked.
    pub air_attacked: BTreeMap<ActorId, f32>,
    /// The base priority (+38, `7C6640`) of each organization's RECOVER goal
    /// (record +3C) at the last think: none is 0.
    pub recover_base: BTreeMap<OrgId, f32>,
    /// The target (+4C, `7C5F80`) of each region's DEFEND_REGION goal (region
    /// entry +30) at the last think.
    pub defend_things: BTreeMap<u32, Option<Unit>>,
    /// The actors each selected goal holds (goal +0C, head first).
    pub holding: BTreeMap<Key, Vec<OrgId>>,
    /// The organizations `7A83A0` has registered (their SAI records exist).
    pub registered: std::collections::BTreeSet<OrgId>,
}

pub(crate) struct Ctx<'a, 'w> {
    pub view: &'a View<'w>,
    pub bound: &'a Bound,
    pub ego: &'a Ego,
    pub regions: &'a Regions,
    pub inf: &'a Influence,
    pub now: f32,
}

pub(crate) const RESOURCES: [&str; 3] = ["money", "ammo", "oil"];

impl Ctx<'_, '_> {
    pub fn def(&self, d: DefId) -> &crate::data::Def {
        self.view.world.data.def(d)
    }
    pub fn props(&self, d: DefId) -> &[String] {
        self.bound.props.get(d as usize).map_or(&[], Vec::as_slice)
    }
    pub fn cv(&self, d: DefId) -> f32 {
        self.bound.cv.get(d as usize).copied().unwrap_or(0.0)
    }
    /// Money/ammo/oil of a definition's cost, upkeep or production vector
    /// with its organization slots (`56F850`/`56F580`/`56F660`, the
    /// production owner's `quote`). Synthetic data without a resource
    /// registry uses the definitions' three-resource fields.
    pub fn quote(&self, d: DefId, kind: crate::production::Vector) -> [f32; 3] {
        let w = self.view.world;
        let mut out = [0.0; 3];
        if !w.data.resources.is_empty() {
            let v = crate::production::quote(w, d, kind);
            for (slot, name) in RESOURCES.iter().enumerate() {
                if let Some(i) = w.data.resource_index(name) {
                    out[slot] = v.get(i).copied().unwrap_or(0.0);
                }
            }
            return out;
        }
        let pick = |def: &crate::data::Def| match kind {
            crate::production::Vector::Cost => def.cost,
            crate::production::Vector::Upkeep => def.upkeep,
            crate::production::Vector::Production => def.production,
        };
        let def = self.def(d);
        out = pick(def);
        for (m, _) in &def.slots {
            if let Some(md) = w.data.defs.get(*m as usize) {
                let v = pick(md);
                for r in 0..3 {
                    out[r] += v[r];
                }
            }
        }
        out
    }
    /// Unit limit a recruit consumes (the LIMITED_CONSUMED upkeep).
    pub fn unit_need(&self, d: DefId) -> f32 {
        let w = self.view.world;
        if let Some(i) = w.data.resource_index("unit_limit_consumed") {
            return crate::production::quote(w, d, crate::production::Vector::Upkeep).get(i).copied().unwrap_or(0.0);
        }
        let def = self.def(d);
        let members: f32 = def.slots.iter().filter_map(|(m, _)| w.data.defs.get(*m as usize)).map(|m| m.unit_limit_consumed).sum();
        def.unit_limit_consumed.max(members)
    }
    fn sum(&self, values: &[f32], region: u32, pick: impl Fn(usize) -> bool) -> f32 {
        (0..self.inf.kingdoms).filter(|&k| pick(k)).map(|k| self.inf.at(values, region, k)).filter(|v| *v > 0.0).sum()
    }
    /// `7B96E0`: hostile mobile (+ settlement) influence.
    pub fn hostile(&self, region: u32, mobile_only: bool) -> f32 {
        let h = |k: usize| self.view.hostile.get(k).copied().unwrap_or(false);
        let m = self.sum(&self.inf.mobile, region, h);
        if mobile_only { m } else { m + self.sum(&self.inf.settlement, region, h) }
    }
    /// `7B9600`: allied (own included) mobile (+ settlement) influence.
    pub fn allied(&self, region: u32, mobile_only: bool) -> f32 {
        let a = |k: usize| self.view.allied.get(k).copied().unwrap_or(false);
        let m = self.sum(&self.inf.mobile, region, a);
        if mobile_only { m } else { m + self.sum(&self.inf.settlement, region, a) }
    }
    pub fn hostile_border(&self, region: u32) -> f32 {
        self.sum(&self.inf.border, region, |k| self.view.hostile.get(k).copied().unwrap_or(false))
    }
    pub fn allied_border(&self, region: u32) -> f32 {
        self.sum(&self.inf.border, region, |k| self.view.allied.get(k).copied().unwrap_or(false))
    }
    /// `4A7130`: `pow(clamp(1 - t/max_travel_time*0.8, 0.2, 1), exponent)`.
    pub fn travel_weight(&self, t: f32) -> f32 {
        let e = self.ego;
        if e.max_travel_time <= 0.0 {
            return 1.0;
        }
        (1.0 - t / e.max_travel_time * 0.8).clamp(0.2, 1.0).powf(e.travel_time_exponent)
    }
    pub fn travel_time(&self, from: Vec2, to: Vec2, speed: f32) -> f32 {
        from.distance(to) / speed.max(0.1)
    }
    pub fn region_of(&self, p: Vec2) -> u32 {
        self.regions.region_at(p)
    }
    pub fn land(&self, p: Vec2) -> bool {
        self.view.world.map.cell_flags(p) & (BLOCKED | NOT_LAND_PASSABLE) == 0
    }
}

/// `7ACF90`: the priority of a combat goal for an assigned CV.
pub fn cv_curve(ego: &Ego, base: f32, cv: f32, target: f32, max: f32, attack: bool, mut allied: f32) -> f32 {
    let curve = if attack { &ego.offensive } else { &ego.defensive };
    let mut cv = if cv < max { cv } else { max };
    let room = max - cv;
    if room <= allied {
        cv += room;
        allied -= room;
    } else {
        cv += allied;
        allied = 0.0;
    }
    if cv <= 0.0 {
        cv = 0.01;
    }
    if cv >= target {
        if target < cv {
            let mut mult = curve.cv_max_mult_over_target;
            if max + allied <= cv {
                cv = max + allied;
            }
            if max != target {
                mult = (mult - 1.0) * ((cv - target) / (max - target)) + 1.0;
            }
            return mult * base;
        }
        base
    } else if curve.cv_exponent_under_target > 0.0 {
        (cv / target).powf(curve.cv_exponent_under_target) * base
    } else {
        base
    }
}

#[derive(Clone, Debug)]
pub struct Goal {
    pub key: Key,
    pub kind: usize,
    /// Class base priority (`+78`).
    pub raw: f32,
    /// After `7A5200` (GoalPriority, fuzziness, inertia).
    pub base: f32,
    pub fuzzy: f32,
    pub region: u32,
    pub target: Vec2,
    pub target_actor: Option<ActorId>,
    /// Goal +4C: the thing itself (`target_actor` is a structure or a member).
    pub thing: Option<Unit>,
    pub attack: bool,
    pub target_cv: f32,
    pub max_cv: f32,
    pub static_cv: f32,
    pub allied_cv: f32,
    pub required: Option<usize>,
    pub cost: [f32; 3],
    pub recruit: Option<(DefId, ActorId)>,
    pub site: Option<(Vec2, f32, u32)>,
    pub explore: Option<usize>,
    /// RESEARCH: building and technology property; ATTACH: organization and HQ.
    pub research: Option<(ActorId, String)>,
    pub attach: Option<(OrgId, ActorId)>,
    /// AIR_ATTACK: the airfield assigned to it.
    pub airfield: Option<ActorId>,
    // assignment state
    pub actors: Vec<usize>,
    pub sum_cv: f32,
    pub sum_travel: f32,
    pub pref_add: f32,
    pub pref_counts: Vec<u32>,
    pub current: f32,
    pub selected: bool,
}

impl Goal {
    fn new(key: Key, kind: usize, raw: f32, region: u32, target: Vec2) -> Self {
        Goal {
            key,
            kind,
            raw,
            base: raw,
            fuzzy: 0.0,
            region,
            target,
            target_actor: None,
            thing: None,
            attack: false,
            target_cv: 0.0,
            max_cv: 0.0,
            static_cv: 0.0,
            allied_cv: 0.0,
            required: None,
            cost: [0.0; 3],
            recruit: None,
            site: None,
            explore: None,
            research: None,
            attach: None,
            airfield: None,
            actors: Vec::new(),
            sum_cv: 0.0,
            sum_travel: 0.0,
            pref_add: 0.0,
            pref_counts: Vec::new(),
            current: 0.0,
            selected: false,
        }
    }
    pub(super) fn combat(&self) -> bool {
        self.kind == data::ATTACK_REGION || self.kind == data::DEFEND_REGION
    }
}

#[allow(clippy::too_many_arguments)]
pub fn think(
    view: &View<'_>,
    bound: &Bound,
    data: &AiData,
    regions: &Regions,
    inf: &Influence,
    reach: &super::reach::Reach,
    player: &mut Player,
    rng: &mut SaiRng,
    log: &mut Vec<String>,
    logging: bool,
    air_orders: bool,
) {
    select_ego(view, data, player, rng, log);
    let ego = &data.egos[player.ego];
    let c = Ctx { view, bound, ego, regions, inf, now: view.world.time };
    update_knowledge(&c, player);
    update_flares(&c, player);
    let census = Census::build(&c);
    let val = Val::new(&c, &census);
    let mut goals = Vec::new();
    unpack_goals(&c, player, rng, reach, &mut goals);
    recover_goals(&c, player, &mut goals);
    defend_goals(&c, &val, &mut goals);
    attack_goals(&c, &val, &mut goals);
    bombard_goals(&c, &val, player, &mut goals);
    air_attack_goals(&c, &val, player, &mut goals);
    repair_goals(&c, &val, &mut goals);
    apply_flares(&c, player, &mut goals);
    explore_goals(&c, player, &mut goals);
    recruit_goals(&c, &val, data, rng, &mut goals);
    research_goals(&c, &mut goals);
    attach_goals(&c, &val, &mut goals);
    for g in &mut goals {
        finalize(g, ego, &player.memory, rng);
    }
    if logging {
        // The original's own dump, 7A5200 under SAIDebug with GoalPriorityCSV.
        log.extend(goals.iter().filter_map(|g| goal_row(&c, g, &player.memory)));
    }
    // 7A93B0: candidates sorted by base priority (comparator 7A30B0).
    goals.sort_by(|a, b| b.base.partial_cmp(&a.base).unwrap_or(std::cmp::Ordering::Equal));
    // 7A93B0, through the exact ports.
    player.memory.holding = super::selection::select(&c, player, &mut goals, air_orders, log);
    if logging {
        let bombarders: Vec<String> = c.view.orgs.iter().filter(|o| o.owner == c.view.me && bombards(&c, o)).map(|o| c.def(o.def).id.clone()).collect();
        if !bombarders.is_empty() {
            log.push(format!("  bombard pool {bombarders:?}"));
        }
        let mut shown = 0;
        for g in goals.iter().filter(|g| g.base > 0.0) {
            let other = matches!(g.key, Key::Attack(_) | Key::Defend(_));
            if other && shown >= 6 {
                continue;
            }
            shown += other as usize;
            log.push(format!(
                "  goal {} raw {:.1} base {:.0} current {:.0} actors {} cv {:.0}/{:.0}{}",
                match g.key {
                    Key::Recruit(_, d) => format!("Recruit {} cost {:?}", c.def(d).id, g.cost),
                    Key::Research(_, _) => format!("Research {}", g.research.as_ref().map_or("", |r| r.1.as_str())),
                    k => format!("{k:?}"),
                },
                g.raw,
                g.base,
                g.current,
                g.actors.len(),
                g.sum_cv,
                g.target_cv,
                if g.selected { " selected" } else { "" }
            ));
        }
    }
    player.memory.defend_things = goals.iter().filter(|g| g.kind == data::DEFEND_REGION).map(|g| (g.region, g.thing)).collect();
    execute(&c, data, player, rng, &goals, log);
    player.memory.selected = goals.iter().filter(|g| g.selected).map(|g| (g.key, g.fuzzy)).collect();
}

/// `7AD180`: keep the current ego while its filters hold, else `4A8910`.
fn select_ego(view: &View<'_>, data: &AiData, player: &mut Player, rng: &mut SaiRng, log: &mut Vec<String>) {
    let profile = &data.profiles[player.profile];
    let stat = |name: &str| -> f32 {
        match name {
            "money_rate" => view.income[0],
            "ammo_rate" => view.income[1],
            "oil_rate" => view.income[2],
            _ => 0.0,
        }
    };
    let passes = |e: usize| data.egos[e].filters.iter().all(|f| stat(&f.stat) >= f.min && stat(&f.stat) <= f.max);
    if profile.egos.contains(&player.ego) && passes(player.ego) {
        if player.ge_count == 0 {
            log.push(format!("ego {} (money_rate {:.1})", data.egos[player.ego].ids, view.income[0]));
        }
        return;
    }
    let mut best = None;
    let mut best_value = 0.0f32;
    for &e in &profile.egos {
        if !passes(e) {
            continue;
        }
        let value = rng.unit() * profile.ego_fuzziness + data.egos[e].priority;
        if best_value < value {
            best_value = value;
            best = Some(e);
        }
    }
    let chosen = best.unwrap_or_else(|| profile.egos.first().copied().unwrap_or(player.ego));
    if chosen != player.ego || player.ge_count == 0 {
        log.push(format!("ego {} (money_rate {:.1})", data.egos[chosen].ids, view.income[0]));
    }
    player.ego = chosen;
}

/// Regions within sight of the player's actors are scouted now.
fn update_knowledge(c: &Ctx<'_, '_>, player: &mut Player) {
    if player.scouted.len() != c.regions.len() {
        player.scouted = vec![f32::NEG_INFINITY; c.regions.len()];
    }
    let mut eyes: Vec<(Vec2, f32)> = c.view.orgs.iter().filter(|o| o.owner == c.view.me).map(|o| (o.pos, o.sight)).collect();
    for b in c.view.buildings.iter().filter(|b| b.owner == Some(c.view.me)) {
        let d = c.def(b.def);
        eyes.push((b.pos, d.sight_range.max(d.detection_range)));
    }
    for &(p, r) in &eyes {
        let r = r.max(1.0);
        for reg in c.regions.within(c.region_of(p), (r / c.regions.size).ceil() as u32) {
            if c.regions.centers[reg as usize].distance(p) <= r + c.regions.size * 0.5 {
                player.scouted[reg as usize] = c.now;
            }
        }
    }
}

/// Internal flares (`[InternalFlares]`, ego +8C0..+908): a structure that
/// loses health is defended (`7B1ED0`, `defend_attacked_target_flare_*`),
/// one taken by the enemy is counter-attacked (`7B2050`,
/// `counter_attack_captured_actor_flare_*`), one the player takes is
/// defended (`7B21C0`: `defend_captured_structure_flare_*`; a building that
/// belongs to a settlement, virtual +100, gets none unless it is its centre,
/// and then `defend_captured_settlement_flare_*`, which aa-play does not
/// tell apart: the goal CSV shows +500 for 180 s on a captured city). The
/// original makes them at the events (`4D3990`, `4D5230`); here each
/// goal-engine think compares.
fn update_flares(c: &Ctx<'_, '_>, player: &mut Player) {
    let f = &c.ego.flares;
    player.flares.retain(|(_, _, until)| *until > c.now);
    let mut health = BTreeMap::new();
    for b in c.view.buildings.iter().filter(|b| b.owner == Some(c.view.me)) {
        let a = &c.view.world.actors[b.id as usize];
        if player.structure_health.get(&b.id).is_some_and(|&h| a.health < h - 1e-3) {
            add_flare(player, Key::Defend(c.region_of(b.pos)), f.defend_attacked_target_priority, c.now + f.defend_attacked_target_duration);
        }
        // The start's cities change hands as the world is made (4D5230), with the SAI there:
        // the goal log has +500 on the starting city from the first think.
        if (player.foreign.contains(&b.id) || player.ge_count == 0 && is_city(c.def(b.def))) && admitted(c.def(b.def)) {
            add_flare(player, Key::Defend(c.region_of(b.pos)), f.defend_captured_structure_priority, c.now + f.defend_captured_structure_duration);
        }
        health.insert(b.id, a.health);
    }
    player.foreign = c.view.buildings.iter().filter(|b| b.owner.is_some_and(|o| o != c.view.me)).map(|b| b.id).collect();
    let last: Vec<ActorId> = player.structure_health.keys().copied().collect();
    for id in last {
        let a = &c.view.world.actors[id as usize];
        if a.alive && !a.removed && a.kingdom.is_some_and(|k| k != c.view.kingdom) && admitted(c.def(a.def)) {
            add_flare(player, Key::Attack(c.region_of(a.pos.truncate())), f.counter_attack_captured_actor_priority, c.now + f.counter_attack_captured_actor_duration);
        }
    }
    player.structure_health = health;
}

/// `7B0610`, a new flare that is not additive: whatever target they have, an existing one
/// that lasts and is worth no more than the new one goes, and one that lasts and is worth at
/// least as much leaves the new one out. So a newly captured city's flare ends the last one's,
/// as the goal log shows (Vranje to 120 s, Prevenza from 150 to 180, Pisciotta at 210 alone).
fn add_flare(player: &mut Player, key: Key, priority: f32, until: f32) {
    let mut i = 0;
    while i < player.flares.len() {
        let (_, p, u) = player.flares[i];
        if u > until || p > priority {
            if until <= u && priority <= p {
                return;
            }
            i += 1;
        } else {
            player.flares.remove(i);
        }
    }
    player.flares.push((key, priority, until));
}

/// Flares are general-power instances of the player (`7B0610`), scanned by
/// the class base priorities (`7C5AE0` / `7C5F80`): an ATTACK_REGION goal
/// counts the attack flares within `linked_region_distance` route hops of
/// its region (`572810`), a DEFEND_REGION goal the defend flares in its
/// region (`572940`); of the non-additive ones only the largest bonus is
/// added.
fn apply_flares(c: &Ctx<'_, '_>, player: &Player, goals: &mut [Goal]) {
    let reach = c.ego.linked_region_distance.max(0) as u32;
    for g in goals.iter_mut() {
        let mut max = 0.0f32;
        for (key, priority, _) in &player.flares {
            let matched = match *key {
                Key::Defend(r) => g.kind == data::DEFEND_REGION && g.region == r,
                Key::Attack(r) => g.kind == data::ATTACK_REGION && c.regions.hops(g.region, r) <= reach,
                _ => false,
            };
            // 571C60 / the largest-bonus update: a zero maximum or one
            // below the bonus takes it.
            if matched && (max == 0.0 || max < *priority) {
                max = *priority;
            }
        }
        g.raw += max;
    }
}

fn finalize(g: &mut Goal, ego: &Ego, memory: &Memory, rng: &mut SaiRng) {
    if g.raw <= 0.0 {
        g.base = g.raw;
        return;
    }
    let gp = ego.goal(g.kind);
    let fuzziness = if gp.fuzziness < 0.0 { ego.fuzziness } else { gp.fuzziness };
    let inertia = if gp.inertia < 0.0 { ego.goal_inertia } else { gp.inertia };
    let mut p = g.raw * gp.multiplier + gp.bonus;
    if p < gp.minimum {
        p = 0.0;
    }
    if p <= 0.0 {
        g.base = p;
        return;
    }
    // A recruit goal is made anew every think: no inertia and a fresh fuzz
    // (the goal CSV shows inertia 0 on all 91 RECRUIT rows, also for the
    // Mechanized HQ selected think after think).
    if g.kind != data::RECRUIT
        && let Some(&fuzzy) = memory.selected.get(&g.key)
    {
        g.fuzzy = fuzzy;
        g.base = p + fuzzy + inertia;
    } else {
        g.fuzzy = rng.unit() * fuzziness;
        g.base = p + g.fuzzy;
    }
}

/// One row of the original's goal CSV (`7A5200`, written under `SAIDebug` with the
/// SVar `GoalPriorityCSV`, opened by `7A5BF0`): goal type, label, class base priority
/// (+38 before the record), the ego's multiplier, bonus and minimum, fuzz (+18), inertia
/// (+28), the final base. Nothing is written for a goal whose priority before fuzz is
/// not above zero and at its minimum.
fn goal_row(c: &Ctx<'_, '_>, g: &Goal, memory: &Memory) -> Option<String> {
    let gp = c.ego.goal(g.kind);
    let before = g.raw * gp.multiplier + gp.bonus;
    if g.raw <= 0.0 || before <= 0.0 || before < gp.minimum {
        return None;
    }
    let name = match g.kind {
        data::ATTACK_REGION => "ATTACK_REGION",
        data::DEFEND_REGION => "DEFEND_REGION",
        data::AIR_ATTACK => "AIR_ATTACK",
        data::RECOVER => "RECOVER",
        data::EXPLORE => "EXPLORE",
        data::UNPACK => "UNPACK",
        data::RECRUIT => "RECRUIT",
        data::RESEARCH => "RESEARCH",
        data::BOMBARD => "BOMBARD",
        data::ATTACH => "ATTACH",
        data::REPAIR => "REPAIR",
        _ => "OTHER",
    };
    let inertia = if g.kind != data::RECRUIT && memory.selected.contains_key(&g.key) {
        if gp.inertia < 0.0 { c.ego.goal_inertia } else { gp.inertia }
    } else {
        0.0
    };
    let label = match g.kind {
        data::RECRUIT => g.recruit.map_or(String::new(), |(d, b)| format!("{} -> {}", c.def(d).id, c.def(c.view.world.actors[b as usize].def).id)),
        data::RESEARCH => g.research.as_ref().map_or(String::new(), |r| r.1.clone()),
        data::RECOVER => format!("region #{}", g.region),
        data::UNPACK => match g.key {
            Key::Unpack(t) => format!("{} -> region #{}", c.def(c.view.world.actors[t as usize].def).id, g.region),
            _ => String::new(),
        },
        data::EXPLORE => format!("{:?}", g.key),
        _ => format!("{} -> region #{}", g.target_actor.map_or("no target".to_owned(), |t| c.def(c.view.world.actors[t as usize].def).id.clone()), g.region),
    };
    Some(format!("csv {name}|{label}|{:.6}|{:.2}|{:.2}|{:.2}|{:.2}|{inertia:.2}|{:.6}", g.raw, gp.multiplier, gp.bonus, gp.minimum, g.fuzzy, g.base))
}

// ---------------------------------------------------------------------------
// Candidate goals

fn spread_offset(i: usize) -> Vec2 {
    let a = i as f32 * 2.399_963;
    Vec2::new(a.cos(), a.sin()) * (12.0 + 8.0 * (i as f32).sqrt())
}

/// `557010` -> `556940`: a place for a building of `def` in `region`. The search
/// starts at the region's centre and walks rings of the definition's larger side
/// (at least a tile): as many rings as `0.75 * 7AA180 / spacing` (rounded up),
/// from a random ring on, `6 * ring` places each, from a direction of the
/// `[B39AF8]` table (2048 entries), rotating by -2pi / places. The SAI's RNG gives
/// the direction and the first ring; a second try draws both again. A place must
/// lie on the map, out of hostile zones (`556480`, here the hostile supply radii),
/// and fit (`5553E0`, here `deploy::spot_clear` and the movement owner's test); the
/// truck must be able to drive there (`reach`, not the original's: its goal has no
/// such test and a refused unpack would otherwise repeat).
fn find_site(c: &Ctx<'_, '_>, def: DefId, region: u32, truck: ActorId, from: Vec2, reach: &super::reach::Reach, rng: &mut SaiRng) -> Option<Vec2> {
    let w = c.view.world;
    let d = c.def(def);
    let tile = w.map.cell_size;
    let spacing = d.blocking_size.map_or(0.0, |s| s[0].max(s[1])).max(tile);
    if spacing <= 0.0 {
        return None;
    }
    let rings = (c.regions.max_rectangle_width as f32 * 0.75 / spacing).ceil() as i32;
    let center = c.regions.centers[region as usize];
    let pad = w.data.rule_ci("BuildingPaddingMeters").unwrap_or(2.0);
    let radius = d.blocking_size.map_or(4.0, |s| s[0].max(s[1]) * 0.5);
    let fits = |p: Vec2| -> Option<Vec2> {
        if c.view.buildings.iter().any(|b| b.supply_range > 0.0 && b.owner.is_some_and(|o| c.view.hostile[o]) && b.pos.distance_squared(p) < b.supply_range * b.supply_range) {
            return None;
        }
        if !c.view.buildings.iter().all(|b| b.pos.distance(p) >= b.radius + radius + pad) {
            return None;
        }
        // The order names the place itself; the unpack snaps it (0x4F3640).
        let (at, _) = crate::deploy::placement(w, def, p, 0.0);
        (crate::deploy::spot_clear(w, def, at, Some(truck)).is_ok() && crate::movement::deploy_spot_problem(w, truck, at).is_none() && reach.reachable(from, at)).then_some(p)
    };
    for _ in 0..2 {
        let index = rng.next_u32() % 0x800;
        let angle = (f64::from(index) * std::f64::consts::TAU / 2048.0) as f32;
        let dir = Vec2::new(angle.cos(), angle.sin()) * spacing;
        let start = if rings >= 1 { (rng.next_u32() % rings as u32) as i32 } else { 0 };
        for step in 0..rings {
            let ring = (start + step) % rings;
            let places = (ring * 6).max(1);
            let mut offset = dir * ring as f32;
            let (sin, cos) = (-std::f32::consts::TAU / places as f32).sin_cos();
            for _ in 0..places {
                if let Some(at) = fits(center + offset) {
                    return Some(at);
                }
                offset = Vec2::new(offset.x * cos - offset.y * sin, sin * offset.x + offset.y * cos);
            }
        }
    }
    None
}

/// UNPACK goals (`7CBC40` with the `4A9AE0` value): the candidate regions
/// are valued first and searched for a reachable spot best first.
fn unpack_goals(c: &Ctx<'_, '_>, player: &mut Player, rng: &mut SaiRng, reach: &super::reach::Reach, goals: &mut Vec<Goal>) {
    let u = &c.ego.unpacking;
    let mine: Vec<_> = c.view.buildings.iter().filter(|b| b.owner == Some(c.view.me)).collect();
    let island_regions: Vec<u32> = (0..c.regions.len() as u32).filter(|&r| c.regions.passable[r as usize]).collect();
    for t in c.view.trucks.iter().filter(|t| t.owner == c.view.me) {
        let memo = player.memory.unpack.entry(t.id).or_default();
        if t.deploying {
            // A deploy order the deploy owner refused leaves the truck idle;
            // after a while the site is considered bad.
            continue;
        }
        if memo.ordered_at > 0.0 && c.now - memo.ordered_at < 15.0 {
            continue;
        }
        if memo.ordered_at > 0.0 {
            memo.bad.push(memo.region);
            memo.ordered_at = 0.0;
            memo.sent_at = 0.0;
        }
        let home = c.region_of(t.pos);
        let mut candidates = c.regions.within(home, u.adjacent_region_depth.max(0) as u32);
        let same_island: Vec<u32> = island_regions.iter().copied().filter(|&r| c.regions.same_island(r, home)).collect();
        for _ in 0..u.num_random_regions.max(0) {
            if same_island.is_empty() {
                break;
            }
            let r = same_island[rng.below(same_island.len())];
            if !candidates.contains(&r) {
                candidates.insert(0, r);
            }
        }
        if memo.region != 0 && !candidates.contains(&memo.region) {
            candidates.insert(0, memo.region);
        }
        // 7CBC40: each candidate has its place (557010) and that place its value (4A9AE0);
        // the best above the others wins.
        let mut best: Option<(f32, u32, Vec2)> = None;
        let mut seen: Vec<u32> = Vec::new();
        for r in candidates {
            if memo.bad.contains(&r) || !c.regions.same_island(r, home) || seen.contains(&r) {
                continue;
            }
            seen.push(r);
            let Some(site) = find_site(c, t.unpacks_to, r, t.id, t.pos, reach, rng) else { continue };
            let mut bonus = 0.0;
            for b in &mine {
                let d = b.pos.distance(site);
                if d < u.max_distance && u.max_distance > 0.0 {
                    bonus += u.max_friendly_building_bonus_per_building * ((u.max_distance - d) / u.max_distance).powf(u.distance_exponent);
                }
            }
            let bonus = bonus.min(u.max_friendly_building_bonus);
            let enemy = u.enemy_static_influence_mult * (c.hostile(r, false) - c.hostile(r, true)) + u.enemy_dynamic_influence_mult * c.hostile(r, true);
            let travel = (c.regions.hops(home, r) as f32 * u.travel_distance_mult).max(0.0).powf(u.travel_distance_exponent);
            let mut v = u.base_priority + bonus - enemy - travel + rng.unit() * u.fuzzy_factor;
            if r == memo.region {
                v += u.inertia;
            }
            if best.is_none_or(|(b, _, _)| v > b) {
                best = Some((v, r, site));
            }
        }
        let Some((value, region, site)) = best else { continue };
        let facing = facing_toward_enemy(c, site);
        let mut g = Goal::new(Key::Unpack(t.id), data::UNPACK, if value < 0.0 { 0.001 } else { value }, region, site);
        g.site = Some((site, facing, region));
        goals.push(g);
    }
    let live: Vec<ActorId> = c.view.trucks.iter().map(|t| t.id).collect();
    player.memory.unpack.retain(|k, _| live.contains(k));
}

fn facing_toward_enemy(c: &Ctx<'_, '_>, from: Vec2) -> f32 {
    let w = c.view.world;
    let mut target = Vec2::new(w.map.width as f32, w.map.height as f32) * w.map.cell_size * 0.5;
    let mut best = f32::MAX;
    for b in c.view.buildings.iter().filter(|b| b.owner.is_some_and(|o| c.view.hostile[o])) {
        let d = b.pos.distance(from);
        if d < best {
            best = d;
            target = b.pos;
        }
    }
    let v = target - from;
    if v.length_squared() < 1e-6 { 0.0 } else { v.y.atan2(v.x) }
}

/// RECOVER goals (`7C6640`).
fn recover_goals(c: &Ctx<'_, '_>, player: &mut Player, goals: &mut Vec<Goal>) {
    let r = &c.ego.recover;
    player.memory.recover_base.clear();
    for (i, o) in c.view.orgs.iter().enumerate() {
        if o.owner != c.view.me || !o.military || o.routing {
            continue;
        }
        // 7C6640: under the maximum, or in the recover state (record +30 kind 2).
        let recovering = player.rai.recovering(o.id);
        let mut need = 0.0f32;
        if o.health < r.max_hp || recovering {
            need = (1.0 - o.health.max(0.0).powf(r.exponent_hp)) * r.base_priority_hp;
        }
        if o.morale < r.max_morale || recovering {
            need = need.max((1.0 - o.morale.max(0.0).powf(r.exponent_morale)) * r.base_priority_morale);
        }
        // 7A83A0: an actor registered with its RECOVER goal above zero is that goal's at once:
        // selected (the goal's fuzz never drawn, -1 in the original's log), and the record in
        // the recover state (7A1070, 7A0FC0). A rendition of when: natively a regiment registers
        // as its first member stands, so the log has every fresh regiment in that state, at
        // 456.1 = (1 - (4/9)^3) * 500, and the same think its attack goal has it (the film's move
        // at 90.4 s). How it leaves RECOVER there is not read: 7A3740 asks for more than the
        // attack goals give. Here it registers at the think that first sees it, so a regiment
        // at 4 of 9 is over max_hp and goes straight to the attack goals, as the film does.
        let first = player.memory.registered.insert(o.id);
        if need <= 0.0 {
            continue;
        }
        if first {
            player.memory.selected.insert(Key::Recover(o.id), -1.0);
            player.memory.holding.insert(Key::Recover(o.id), vec![o.id]);
            player.rai.states.insert(o.id, super::rai::OrderState::new(super::rai::StateKind::Recover));
        }
        player.memory.recover_base.insert(o.id, need);
        let Some(dest) = supply_point(c, o.pos) else { continue };
        let mut g = Goal::new(Key::Recover(o.id), data::RECOVER, need, c.region_of(dest), dest);
        g.required = Some(i);
        goals.push(g);
    }
}

/// A point inside the nearest own supply zone, on the side of the actor.
pub(crate) fn supply_point(c: &Ctx<'_, '_>, from: Vec2) -> Option<Vec2> {
    let best = c
        .view
        .buildings
        .iter()
        .filter(|b| b.owner == Some(c.view.me) && b.supply_range > 0.0)
        .min_by(|a, b| a.pos.distance(from).partial_cmp(&b.pos.distance(from)).unwrap_or(std::cmp::Ordering::Equal))?;
    let dir = (from - best.pos).normalize_or_zero();
    Some(best.pos + dir * (best.radius + (best.supply_range * 0.4).min(20.0)))
}

/// DEFEND_REGION goals (`7C5F80` -> `7BD360`).
fn defend_goals(c: &Ctx<'_, '_>, val: &Val<'_, '_, '_>, goals: &mut Vec<Goal>) {
    let dp = &c.ego.defend;
    let mut regions: BTreeMap<u32, (f32, Vec2, f32, Option<ActorId>)> = BTreeMap::new();
    for b in &c.view.buildings {
        let Some(owner) = b.owner else { continue };
        if !c.view.allied[owner] {
            continue;
        }
        // 7AD340: (actor value without the distance multiplier + its
        // ActorPriority) ^ economic_value_exponent * multiplier.
        let kingdom = c.view.world.kingdoms[owner].id;
        let value = (val.actor_value(b.def, Some(kingdom), b.cv, false, b.pos, false) + val.actor_priority(b.def, false)).max(0.0);
        let mut threat = dp.economic_value_multiplier * value.powf(dp.economic_value_exponent);
        if owner != c.view.me {
            threat *= c.ego.defend_friendly_kingdom_actor_multiplier;
        }
        let r = c.region_of(b.pos);
        let e = regions.entry(r).or_insert((0.0, b.pos, 0.0, None));
        if threat > e.0 {
            e.0 = threat;
            e.1 = b.pos;
            // 7BD360: the most valuable of the region's is the goal's +4C (7C5F80).
            e.3 = Some(b.id);
        }
        if owner == c.view.me {
            e.2 += b.cv;
        }
    }
    let hostile_orgs: Vec<_> = c.view.orgs.iter().filter(|o| c.view.hostile[o.owner] && o.military).collect();
    for (r, (value, pos, static_cv, thing)) in regions {
        if value <= 0.0 {
            continue;
        }
        let center = c.regions.centers[r as usize];
        let hostile = c.hostile(r, false);
        let mut v = value + dp.enemy_influence_multiplier * hostile.max(0.0).powf(dp.enemy_influence_exponent);
        v += dp.enemy_border_influence_multiplier * c.hostile_border(r).max(0.0).powf(dp.enemy_border_influence_exponent);
        v += dp.friendly_border_influence_multiplier * c.allied_border(r).max(0.0).powf(dp.friendly_border_influence_exponent);
        let threat = dp.enemy_forces_threat_distance;
        if hostile_orgs.iter().any(|o| o.pos.distance_squared(center) < threat * threat) {
            v *= dp.enemy_forces_bonus;
        }
        let allied = c.allied(r, false);
        if hostile != 0.0 && (allied == 0.0 || dp.capture_risk_ratio <= hostile / allied) {
            v *= dp.capture_risk_bonus;
        }
        let mut g = Goal::new(Key::Defend(r), data::DEFEND_REGION, v, r, pos);
        g.target_actor = thing;
        g.thing = thing.map(Unit::Actor);
        let d = &c.ego.defensive;
        g.target_cv = (c.hostile(r, true) * d.cv_advantage_target).max(d.min_cv_to_assign);
        g.max_cv = g.target_cv * d.cv_advantage_max;
        g.static_cv = if d.count_static_cv { static_cv } else { 0.0 };
        g.allied_cv = (c.allied(r, true) - c.inf.at(&c.inf.mobile, r, c.view.me)).max(0.0) * c.ego.defend_allied_influence_cv_multiplier;
        goals.push(g);
    }
}

/// ATTACK_REGION goals (`7C5AE0`, value `7BCCB0`: the juiciest visible
/// hostile static or mobile actor of the region, `7AD9E0`).
fn attack_goals(c: &Ctx<'_, '_>, val: &Val<'_, '_, '_>, goals: &mut Vec<Goal>) {
    let mut best: BTreeMap<u32, (f32, Vec2, Option<ActorId>, Option<Unit>)> = BTreeMap::new();
    let mut home_islands: Vec<u32> = c.view.orgs.iter().filter(|o| o.owner == c.view.me).map(|o| c.regions.island[c.region_of(o.pos) as usize]).collect();
    home_islands.extend(c.view.buildings.iter().filter(|b| b.owner == Some(c.view.me)).map(|b| c.regions.island[c.region_of(b.pos) as usize]));
    let reachable = |r: u32| home_islands.contains(&c.regions.island[r as usize]);
    let mut consider = |r: u32, v: f32, p: Vec2, a: Option<ActorId>, t: Option<Unit>| {
        let v = v.max(0.001);
        let e = best.entry(r).or_insert((0.0, p, a, t));
        if v > e.0 {
            *e = (v, p, a, t);
        }
    };
    // 7BB5E0 over the region's static lists: the view holds exactly the
    // static actors the player knows (visible, seen static map objects,
    // remembered ones at their record position).
    for b in &c.view.buildings {
        if b.owner == Some(c.view.me) || !attack_target(c, b) {
            continue;
        }
        let r = c.region_of(b.pos);
        if !reachable(r) {
            continue;
        }
        // 7AD2D0: the actor value (with the distance multiplier) plus its
        // ActorPriority (one fewer counted, no one-time bonus).
        let owner = b.owner.map(|o| c.view.world.kingdoms[o].id);
        consider(r, val.actor_value(b.def, owner, b.cv, false, b.pos, true) + val.actor_priority(b.def, false), b.pos, Some(b.id), Some(Unit::Actor(b.id)));
    }
    // 7BA1F0: the kingdom's records of hostile actors gone since (7AD2D0
    // without an actor) keep the region a target without a target actor.
    for g in &c.view.ghosts {
        if !g.owner.is_some_and(|o| c.view.hostile[o]) || !admitted(c.def(g.def)) {
            continue;
        }
        let r = c.region_of(g.pos);
        if !reachable(r) {
            continue;
        }
        let owner = g.owner.map(|o| c.view.world.kingdoms[o].id);
        consider(r, val.actor_value(g.def, owner, 0.0, false, g.pos, true) + val.actor_priority(g.def, false), g.pos, None, None);
    }
    // 7BB5E0 over the mobile lists (79E380: visibility only; the view holds
    // the visible hostile organizations; without fog everything is visible,
    // 4EA1D0).
    for o in c.view.orgs.iter().filter(|o| c.view.hostile[o.owner] && o.cv > 0.0) {
        let r = c.region_of(o.pos);
        if reachable(r) {
            let owner = Some(c.view.world.kingdoms[o.owner].id);
            consider(r, val.actor_value(o.def, owner, o.cv, true, o.pos, true) + val.actor_priority(o.def, false), o.pos, None, Some(Unit::Org(o.id)));
        }
    }
    let oc = &c.ego.offensive;
    for (r, (value, pos, actor, thing)) in best {
        let mut g = Goal::new(Key::Attack(r), data::ATTACK_REGION, value, r, pos);
        g.attack = true;
        g.target_actor = actor;
        g.thing = thing;
        g.target_cv = (c.hostile(r, false) * oc.cv_advantage_target).max(oc.min_cv_to_assign);
        g.max_cv = g.target_cv * oc.cv_advantage_max;
        g.allied_cv = (c.allied(r, true) - c.inf.at(&c.inf.mobile, r, c.view.me)).max(0.0) * c.ego.attack_allied_influence_cv_multiplier;
        goals.push(g);
    }
}

/// BOMBARD goals: the actor registration `7A83A0` gives the player one per
/// foreign actor (class `A9ED50`, type 20, pool category 12: actors with a
/// bombard ability). Base priority `7C8750`: 0 once the target is gone or
/// invulnerable (`7C7930`, +8 bit 11), unless it is known (visible, +70,
/// or the kingdom's record, `4EA430`), static (+8 bit 29) or under attack
/// (`4CFE90`, its +118 attacker list), and hostile (player virtual +140);
/// then the attack value `7AD9E0`, times the Bombardement
/// unengaged_target_multiplier while neither the target region's
/// ATTACK_REGION goal (region entry +2C) is selected (status 2) nor the
/// target is under attack. The targets here: the hostile structures the view
/// holds and the visible members of hostile organizations under attack. The
/// goal's attack-flare scan (`572040` / `5720C0` on the target actor) is not
/// played: flares here are by region.
fn bombard_goals(c: &Ctx<'_, '_>, val: &Val<'_, '_, '_>, player: &Player, goals: &mut Vec<Goal>) {
    let w = c.view.world;
    let attacked = |t: ActorId| w.combat.actor(t).is_some_and(|s| s.targeted);
    let mut push = |t: ActorId, d: DefId, o: usize, cv: f32, mobile: bool, pos: Vec2| {
        if c.def(d).invulnerable {
            return;
        }
        let r = c.region_of(pos);
        let mut p = val.actor_value(d, Some(w.kingdoms[o].id), cv, mobile, pos, true) + val.actor_priority(d, false);
        if !player.memory.selected.contains_key(&Key::Attack(r)) && !attacked(t) {
            p *= c.ego.bombard_unengaged_target_multiplier;
        }
        let mut g = Goal::new(Key::Bombard(t), data::BOMBARD, p, r, pos);
        g.target_actor = Some(t);
        goals.push(g);
    };
    for b in &c.view.buildings {
        if let Some(o) = b.owner.filter(|&o| o != c.view.me && c.view.hostile[o]) {
            push(b.id, b.def, o, b.cv, false, b.pos);
        }
    }
    for ov in c.view.orgs.iter().filter(|o| c.view.hostile[o.owner]) {
        for &m in &w.orgs[ov.id as usize].members {
            if let Some(x) = w.actor(m).filter(|x| x.alive && !x.removed && attacked(x.id)) {
                push(m, x.def, ov.owner, c.cv(x.def), true, x.pos.truncate());
            }
        }
    }
}

/// AIR_ATTACK goals: with the SVar UseAirAttacks, `7A83A0` gives the player
/// one per foreign actor (class `A9ECC8`, type 4, pool category 13: the
/// air-attack denizens of its airfields, `7A1680`). Base priority `7C7E90`:
/// 0 once the target is gone or invulnerable (`7C7930`), within
/// min_time_between_attacks of the goal's last attack (+40), unless the
/// target is known (visible or the kingdom's record) and hostile; then the
/// attack value `7AD9E0`, times engaged_bonus while it is under attack
/// (`4CFE90`), target_damage_bonus at a health between the damage floor and
/// ceiling, absolute_min_health_engaged/unengaged_mult at or below
/// absolute_min_health, and support_ground_attack_bonus while the region's
/// ATTACK_REGION goal is selected (else no_bonus_penalty when no other
/// factor applied). Targets here: the hostile structures and the members
/// of the hostile organizations the view holds. Not played (see
/// [`AiState::air_orders`](super::AiState)): resupplying_mult, the flare scan
/// and `7C55B0`'s extra planes.
fn air_attack_goals(c: &Ctx<'_, '_>, val: &Val<'_, '_, '_>, player: &Player, goals: &mut Vec<Goal>) {
    let w = c.view.world;
    if w.data.rule_ci("UseAirAttacks").unwrap_or(1.0) == 0.0 {
        return;
    }
    let a = &c.ego.air;
    let base = |t: ActorId, cv: f32, mobile: bool, pos: Vec2, owner: usize, health: f32| -> Option<(f32, u32)> {
        let actor = w.actor(t).filter(|x| x.alive && !x.removed)?;
        let def = c.def(actor.def);
        if def.invulnerable {
            return None;
        }
        if let Some(&last) = player.memory.air_attacked.get(&t)
            && a.min_time_between_attacks > 0.0
            && last >= 0.0
            && a.min_time_between_attacks + last > c.now
        {
            return None;
        }
        let r = c.region_of(pos);
        let mut p = val.actor_value(actor.def, Some(w.kingdoms[owner].id), cv, mobile, pos, true) + val.actor_priority(actor.def, false);
        let mut scaled = false;
        let engaged = w.combat.actor(t).is_some_and(|s| s.targeted);
        if engaged {
            p *= a.engaged_bonus;
            scaled = true;
        }
        if health >= a.target_damage_floor && health <= a.target_damage_ceiling {
            p *= a.target_damage_bonus;
            scaled = true;
        }
        if health <= a.absolute_min_health {
            p *= if engaged { a.absolute_min_health_engaged_mult } else { a.absolute_min_health_unengaged_mult };
            scaled = true;
        }
        if player.memory.selected.contains_key(&Key::Attack(r)) {
            p *= a.support_ground_attack_bonus;
        } else if !scaled {
            p *= a.no_bonus_penalty;
        }
        Some((p, r))
    };
    // A goal at priority 0 neither draws its fuzz (7A5200) nor takes an
    // actor; only the positive ones are kept.
    let mut push = |t: ActorId, pos: Vec2, p: f32, r: u32| {
        if p <= 0.0 {
            return;
        }
        let mut g = Goal::new(Key::AirAttack(t), data::AIR_ATTACK, p, r, pos);
        g.target_actor = Some(t);
        goals.push(g);
    };
    for b in &c.view.buildings {
        let Some(o) = b.owner.filter(|&o| o != c.view.me && c.view.hostile[o]) else { continue };
        if let Some((p, r)) = base(b.id, b.cv, false, b.pos, o, b.health) {
            push(b.id, b.pos, p, r);
        }
    }
    for ov in c.view.orgs.iter().filter(|o| c.view.hostile[o.owner]) {
        for &m in &w.orgs[ov.id as usize].members {
            let Some(x) = w.actor(m).filter(|x| x.alive && !x.removed) else { continue };
            let max = w.combat.actor(m).filter(|s| s.initialized && s.max_health > 0.0).map_or(c.def(x.def).health, |s| s.max_health);
            let health = if max > 0.0 { (x.health / max).clamp(0.0, 1.0) } else { 1.0 };
            let pos = x.pos.truncate();
            let cv = c.cv(x.def) * health;
            if let Some((p, r)) = base(m, cv, true, pos, ov.owner, health) {
                push(m, pos, p, r);
            }
        }
    }
}

/// REPAIR goals: `7A83A0` gives the player one per actor with the +9C component, its
/// own or not. Base `7A4230`: 0 unless the owner is allied (virtual +138) and the health
/// under one; then the repair value `7AD3E0` (`4AC240` with no distance multiplier plus
/// `4A9DB0`) times what is missing, and the flares of kind 3 on the target (not played:
/// flares here are by region). The actors: pool 3 (`7A1150`, an ability that heals), each
/// able to reach it (`7A25F0` case 6). The structures here; a rendition of +9C.
fn repair_goals(c: &Ctx<'_, '_>, val: &Val<'_, '_, '_>, goals: &mut Vec<Goal>) {
    for b in &c.view.buildings {
        let Some(o) = b.owner.filter(|&o| c.view.allied[o]) else { continue };
        if b.health >= 1.0 || b.health <= 0.0 {
            continue;
        }
        let owner = c.view.world.kingdoms[o].id;
        let p = (val.actor_value(b.def, Some(owner), b.cv, false, b.pos, false) + val.actor_priority(b.def, false)) * (1.0 - b.health);
        if p <= 0.0 {
            continue;
        }
        let mut g = Goal::new(Key::Repair(b.id), data::REPAIR, p, c.region_of(b.pos), b.pos);
        g.target_actor = Some(b.id);
        goals.push(g);
    }
}

/// The player's airfields: its structures with an air-attack (or, with
/// `recon`, air-recon) denizen purchase (DenizenComponent AirAttackCost /
/// AirReconCost).
pub(super) fn airfields(c: &Ctx<'_, '_>, recon: bool) -> Vec<ActorId> {
    let w = c.view.world;
    c.view
        .buildings
        .iter()
        .filter(|b| {
            let d = c.def(b.def);
            b.owner == Some(c.view.me) && if recon { d.air_recon_cost.is_some() } else { d.air_attack_cost.is_some() } && w.books.actor(b.id).is_none_or(|bk| bk.deploy.is_none())
        })
        .map(|b| b.id)
        .collect()
}

/// An organization with a member that has a bombard ability (SAI actor
/// category bit 12, `7A1150`: an ability of the +1EC or +21C lists with
/// flag +54 bit 3).
pub(crate) fn bombards(c: &Ctx<'_, '_>, o: &super::view::OrgView) -> bool {
    let w = c.view.world;
    w.orgs[o.id as usize].members.iter().any(|&m| w.actor(m).is_some_and(|a| a.alive) && crate::combat::tasks::can_bombard(w, m))
}

/// `7A2310` -> `7A1A00`: the range of the first bombard ability of the
/// Performer list of the organization's first member that has one (0 without
/// one). A record takes a BOMBARD goal only with the target inside it
/// (`7A25F0` case 0x14, the distance `7C82D0`).
pub(crate) fn bombard_range(c: &Ctx<'_, '_>, o: &super::view::OrgView) -> f32 {
    let w = c.view.world;
    w.orgs[o.id as usize]
        .members
        .iter()
        .filter_map(|&m| w.actor(m).filter(|a| a.alive))
        .find_map(|a| c.def(a.def).weapons.iter().find(|x| !x.defender && x.flags & crate::special_ops::FLAG_BOMBARD != 0 && x.range > 0.0).map(|x| x.range))
        .unwrap_or(0.0)
}

/// The static actors `7BB5E0` admits as region targets: owned by a kingdom
/// the player regards as an enemy (player virtual +140), and either a city
/// (object +D4, the CityComponent, admits it regardless of the other tests)
/// or an actor passing the targetable tests: +8 bit 11 (`invulnerable`)
/// clear, bit 14 (`targetable`) set and the definition +291
/// (BodyComponent `destructible`). The blocks, farms and resource buildings
/// of a city change hands with it and fail these tests. A city nobody owns
/// is taken by presence and is admitted too.
pub(crate) fn attack_target(c: &Ctx<'_, '_>, b: &super::view::BuildingView) -> bool {
    match b.owner {
        None => is_city(c.def(b.def)),
        Some(o) if c.view.hostile[o] => admitted(c.def(b.def)),
        Some(_) => false,
    }
}

fn is_city(def: &crate::data::Def) -> bool {
    def.city && def.city_zone_radius > 0.0
}

/// The flag tests of `7BB5E0` on a hostile actor's definition.
fn admitted(def: &crate::data::Def) -> bool {
    is_city(def) || (!def.invulnerable && def.targetable && def.destructible)
}

/// Where a regiment of a combat goal goes: its own slot around the target;
/// inside the zone of a city target (presence captures, 5647A0).
pub(crate) fn goal_point(c: &Ctx<'_, '_>, target: Vec2, target_actor: Option<ActorId>, slot: usize) -> Vec2 {
    let offset = spread_offset(slot);
    match target_actor.map(|t| c.def(c.view.world.actors[t as usize].def)) {
        Some(d) if d.city && d.city_zone_radius > 0.0 => {
            let max = d.city_zone_radius * 0.6;
            let len = offset.length();
            target + if len > max { offset * (max / len) } else { offset }
        }
        _ => target + offset,
    }
}

pub(crate) fn city_target(c: &Ctx<'_, '_>, target_actor: Option<ActorId>) -> bool {
    target_actor.is_some_and(|t| {
        let d = c.def(c.view.world.actors[t as usize].def);
        d.city && d.city_zone_radius > 0.0
    })
}

/// EXPLORE goals (`7C29F0`): close (0), deep (1) and air recon (2, its
/// actors the air-recon denizens of the airfields, pool category 14 of
/// `7A1680`). Above its floor the base shrinks by the share of island
/// regions already explored.
fn explore_goals(c: &Ctx<'_, '_>, player: &Player, goals: &mut Vec<Goal>) {
    let home = c.view.buildings.iter().find(|b| b.owner == Some(c.view.me)).map(|b| c.region_of(b.pos));
    let mine: Vec<Vec2> = c.view.buildings.iter().filter(|b| b.owner == Some(c.view.me)).map(|b| b.pos).collect();
    let center = (!mine.is_empty()).then(|| mine.iter().copied().sum::<Vec2>() / mine.len() as f32);
    let island: Vec<u32> = (0..c.regions.len() as u32).filter(|&r| home.is_some_and(|h| c.regions.same_island(r, h))).collect();
    let explored = island.iter().filter(|&&r| player.scouted.get(r as usize).is_some_and(|t| t.is_finite())).count() as f32;
    // CloseGoals, DeepGoals and AirReconGoals (ego +464/+528/+5EC sets).
    for kind in 0..3usize {
        let e = &c.ego.exploration[kind];
        for slot in 0..e.num_goals.max(0) {
            let mut base = e.base_priority - slot as f32 * e.repeat_penalty;
            if base > e.min_priority && !island.is_empty() {
                base -= explored / island.len() as f32 * (base - e.min_priority);
            }
            // The travel weight of an explore goal measures the way to the
            // region it explored last, else to the kingdom centre (its
            // region is chosen per explorer when executed).
            let key = Key::Explore(kind as u8, slot as u8);
            let region = player.memory.explore_region.get(&key).copied().or(home).unwrap_or(0);
            let target = player.memory.explore_region.get(&key).map(|&r| c.regions.centers[r as usize]).or(center).unwrap_or(Vec2::ZERO);
            let mut g = Goal::new(key, data::EXPLORE, base, region, target);
            g.explore = Some(kind);
            goals.push(g);
        }
    }
}

/// RECRUIT goals (`7CFEA0`). The candidates are the things the player's
/// recruiters list whose element requirements hold (`7B4380`: `4E65E0`,
/// `5827B0`). In the planner's order:
/// - `7CD330`: every SpecificRecruitRequest makes `number_to_request`
///   requests per matching candidate (`4A6EA0`), priority `7CC640`. Of what
///   an HQ recruits (list `+104`) those not above zero are dropped, of what
///   the other recruiters list (hash `+C4`) none is;
/// - `num_military_goals` placeholders of priority 0 join them, and the list
///   is sorted by priority, highest first (`7CC3B0`);
/// - each request makes a goal (`7CFD30`, `7CF340`) unless the definition has
///   reached its `max_recruited_per_think` (`4A9F10`, the think's count
///   `+C`) or nothing recruits it; a placeholder (`7CFB60`), until one has
///   succeeded, makes goals of the best company (`7CF010`) until their
///   combat value (`7CCD30`) covers its 10. A goal's base is its request
///   priority plus the recruit value (`7CEE50` -> `7CE5E0`);
/// - the goals are sorted by base (`7A30B0`) and at most `max_goals_per_think`
///   of them are looked at: one whose LIMITED_CONSUMED upkeep does not fit
///   the running limits (`7CC300`) counts and is dropped (`7CCF00`: its
///   company replacement is not played), the others are registered.
fn recruit_goals(c: &Ctx<'_, '_>, val: &Val<'_, '_, '_>, data: &AiData, rng: &mut SaiRng, goals: &mut Vec<Goal>) {
    let w = c.view.world;
    let rr = &c.ego.recruiting;
    let queue_max = if c.ego.build_queue_max_items > 0 { c.ego.build_queue_max_items as usize } else { usize::MAX };
    // Recruiting buildings with room in their queue, by recruitable def.
    let mut sites: BTreeMap<DefId, Vec<ActorId>> = BTreeMap::new();
    for b in c.view.buildings.iter().filter(|b| b.owner == Some(c.view.me) && b.recruits && b.queue < queue_max) {
        if w.books.actor(b.id).is_some_and(|bk| bk.deploy.is_some()) {
            continue;
        }
        for &d in &c.def(b.def).recruits {
            if c.def(d).recruitable != 0 || val.econ.synthetic {
                sites.entry(d).or_default().push(b.id);
            }
        }
    }
    if sites.is_empty() {
        return;
    }
    let owned_props: Vec<String> = c
        .view
        .buildings
        .iter()
        .filter(|b| b.owner == Some(c.view.me))
        .flat_map(|b| c.props(b.def).iter().cloned())
        .chain(w.books.kingdom(c.view.kingdom).map(|k| k.technologies.iter().map(|t| t.to_ascii_lowercase()).collect::<Vec<_>>()).unwrap_or_default())
        .collect();
    let owned_defs: Vec<String> = c.view.buildings.iter().filter(|b| b.owner == Some(c.view.me)).map(|b| c.def(b.def).id.to_ascii_lowercase()).collect();
    // ElementComponent required_structures / required_properties of the
    // thing and of every slot of a company.
    let requirements_met = |d: DefId| -> bool {
        let def = c.def(d);
        let mut ids = vec![def.id.to_ascii_lowercase()];
        for (m, _) in &def.slots {
            if let Some(md) = w.data.defs.get(*m as usize) {
                ids.push(md.id.to_ascii_lowercase());
            }
        }
        ids.iter().all(|id| {
            data.requirements.get(id).is_none_or(|r| {
                r.structures.iter().all(|s| owned_defs.contains(s)) && r.properties.iter().all(|p| owned_props.contains(p))
            })
        })
    };
    // An organization recruited by an HQ takes one of its attachments.
    let enroll = |b: ActorId, d: DefId| -> bool {
        if c.def(d).class != Some(DefClass::Organization) {
            return true;
        }
        let Some(o) = crate::production::hq_org(w, b) else { return true };
        let max = c.def(w.orgs[o as usize].def).max_attachments.unwrap_or(u32::MAX);
        let reserved = w.books.org(o).map_or(0, |bk| bk.reserved_attachments);
        crate::production::attachments(w, b) + reserved < max
    };
    let location = |d: DefId, rng: &mut SaiRng| -> Option<ActorId> {
        let list = sites.get(&d)?;
        let mut best = None;
        let mut best_v = f32::MIN;
        for &b in list.iter().filter(|&&b| enroll(b, d)) {
            let pos = w.actors[b as usize].pos.truncate();
            let near = c.view.buildings.iter().filter(|o| o.owner == Some(c.view.me) && o.pos.distance(pos) < c.regions.size).count() as f32;
            let v = rr.base_value + rr.building_bonus * near + rng.unit() * rr.location_fuzziness;
            if v > best_v {
                best_v = v;
                best = Some(b);
            }
        }
        best
    };
    // 7CC640 HQ scan: an HQ of the candidate's type that still takes an
    // attachment (packed or deployed) means no hq_type_not_available_bonus.
    let hq_props = |d: DefId| -> Vec<String> { c.props(d).iter().filter(|p| p.ends_with("hq") && p.as_str() != "hq").cloned().collect() };
    let hq_available = |d: DefId| -> bool {
        let types = hq_props(d);
        for b in c.view.buildings.iter().filter(|b| b.owner == Some(c.view.me) && types.iter().any(|t| has(c.props(b.def), t))) {
            let Some(o) = crate::production::hq_org(w, b.id) else { continue };
            let max = c.def(w.orgs[o as usize].def).max_attachments.unwrap_or(u32::MAX);
            let reserved = w.books.org(o).map_or(0, |bk| bk.reserved_attachments);
            if crate::production::attachments(w, b.id) + reserved < max {
                return true;
            }
        }
        c.view.trucks.iter().any(|t| t.owner == c.view.me && types.iter().any(|ty| has(c.props(t.def), ty)))
            || types.iter().any(|t| val.census.prop(t) > c.view.buildings.iter().filter(|b| b.owner == Some(c.view.me) && has(c.props(b.def), t)).count() as i32)
    };
    // 7B4380: +C4 holds the things a recruiter without an HQ lists, +104 the
    // companies an HQ recruits.
    let hq_list = |d: DefId| -> bool { sites.get(&d).is_some_and(|l| l.iter().all(|&b| crate::production::hq_org(w, b).is_some())) };
    let eligible: Vec<DefId> = sites.keys().copied().filter(|&d| requirements_met(d)).collect();
    // 7CD330 and the placeholders: (priority, definition, request index);
    // None is a military placeholder.
    let mut requests: Vec<(f32, Option<DefId>, u32)> = Vec::new();
    for (index, req) in rr.requests.iter().enumerate() {
        for &d in &eligible {
            let id = c.def(d).id.to_ascii_lowercase();
            let (matched, count) = match (&req.recruit, &req.property) {
                (Some(r), _) => (*r == id, val.census.def(d)),
                (None, Some(p)) => (has(c.props(d), p), val.census.prop(p)),
                _ => (false, 0),
            };
            if !matched {
                continue;
            }
            let mut p = req.base_priority + val.def_value(d, true) * req.economic_value_multiplier - req.repeat_penalty * count as f32;
            if count < 1 {
                p += req.one_time_bonus;
            }
            if c.def(d).max_attachments.is_some() && !hq_available(d) {
                p += req.hq_type_not_available_bonus;
            }
            if p <= 0.0 && hq_list(d) {
                continue;
            }
            for n in 0..req.number_to_request.max(0) {
                requests.push((p, Some(d), index as u32 * 16 + n as u32));
            }
        }
    }
    for n in 0..rr.num_military_goals.max(0) {
        requests.push((0.0, None, u32::MAX - n as u32));
    }
    requests.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    // The think's count per definition (hash +C) and the recruits' goals.
    let mut per_def: HashMap<DefId, i32> = HashMap::new();
    let mut made: Vec<Goal> = Vec::new();
    let room = |d: DefId, per_def: &HashMap<DefId, i32>| -> bool {
        let max = val.max_per_think(d);
        max <= 0 || per_def.get(&d).copied().unwrap_or(0) < max
    };
    let mut military_done = false;
    for (priority, def, index) in requests {
        match def {
            Some(d) => {
                if !room(d, &per_def) {
                    continue;
                }
                let Some(b) = location(d, rng) else { continue };
                let base = priority + val.recruit_value(d, false, true, rng);
                let mut g = Goal::new(Key::Recruit(index, d), data::RECRUIT, base, 0, Vec2::ZERO);
                g.cost = c.quote(d, crate::production::Vector::Cost);
                g.recruit = Some((d, b));
                *per_def.entry(d).or_default() += 1;
                made.push(g);
            }
            None if military_done => {}
            None => {
                // 7CFB60: goals of the best company until their combat value
                // covers the placeholder's 10; a goal without a company or
                // without a recruiter ends it unsuccessfully.
                let mut left = 10.0f32;
                let mut k = index;
                while left > 0.0 {
                    let mut best: Option<(f32, DefId)> = None;
                    for &d in &eligible {
                        let def = c.def(d);
                        if def.class != Some(DefClass::Organization) || def.max_attachments.is_some() || has(&def.properties, "hq") || c.cv(d) <= 0.0 || !room(d, &per_def) {
                            continue;
                        }
                        let v = val.recruit_value(d, true, true, rng);
                        if best.is_none_or(|(bv, _)| v > bv) {
                            best = Some((v, d));
                        }
                    }
                    let Some((_, d)) = best else { break };
                    let Some(b) = location(d, rng) else { break };
                    let base = val.recruit_value(d, true, true, rng);
                    let mut g = Goal::new(Key::Recruit(k, d), data::RECRUIT, base, 0, Vec2::ZERO);
                    g.cost = c.quote(d, crate::production::Vector::Cost);
                    g.recruit = Some((d, b));
                    *per_def.entry(d).or_default() += 1;
                    made.push(g);
                    left -= c.cv(d);
                    k = k.wrapping_sub(1 << 8);
                    if left <= 0.0 {
                        military_done = true;
                    }
                }
            }
        }
    }
    made.sort_by(|a, b| b.raw.partial_cmp(&a.raw).unwrap_or(std::cmp::Ordering::Equal));
    let limit = if rr.max_goals_per_think < 0 { usize::MAX } else { rr.max_goals_per_think as usize };
    let mut limits = RunningLimits::new(c);
    for (looked, g) in made.into_iter().enumerate() {
        if looked >= limit {
            break;
        }
        let (d, _) = g.recruit.expect("recruit goal");
        if limits.take(c, d) {
            goals.push(g);
        }
    }
}

/// The LIMITED_CONSUMED resources of `7CC300`, running over one planning
/// pass: `4CD980` compares a positive upkeep plus the consumed amount with
/// the limit's provider, `4CCD80` adds it to the consumed copy (kingdom
/// `+1A0` / `+1B8`). Synthetic data has only the kingdom's unit limit.
struct RunningLimits {
    provided: Vec<f32>,
    consumed: Vec<f32>,
    unit_room: f32,
}

impl RunningLimits {
    fn new(c: &Ctx<'_, '_>) -> Self {
        let w = c.view.world;
        let books = w.books.kingdom(c.view.kingdom);
        let king = &w.kingdoms[c.view.me];
        RunningLimits {
            provided: books.map(|b| b.provided.clone()).unwrap_or_default(),
            consumed: books.map(|b| b.consumed.clone()).unwrap_or_default(),
            unit_room: king.unit_limit - king.units_used,
        }
    }
    fn take(&mut self, c: &Ctx<'_, '_>, d: DefId) -> bool {
        let w = c.view.world;
        if w.data.resources.is_empty() {
            let need = c.unit_need(d);
            if need > 0.0 && need > self.unit_room + 1e-3 {
                return false;
            }
            self.unit_room -= need;
            return true;
        }
        let need = crate::production::quote(w, d, crate::production::Vector::Upkeep);
        for (i, r) in w.data.resources.iter().enumerate() {
            let amount = need.get(i).copied().unwrap_or(0.0);
            if r.kind == crate::data::ResourceKind::LimitedConsumed
                && amount > 0.0
                && let Some(p) = r.limited_by
                && self.provided.get(p).copied().unwrap_or(0.0) < self.consumed.get(i).copied().unwrap_or(0.0) + amount
            {
                return false;
            }
        }
        for (i, v) in need.iter().enumerate() {
            if let Some(x) = self.consumed.get_mut(i) {
                *x += v;
            }
        }
        true
    }
}

/// RESEARCH goals (`7D0680`): every technology the player's buildings list
/// that it neither owns nor researches, at a building with room in its queue
/// (research shares the recruit queue); base priority the ego TechPriority
/// (`4A9A20`), cost the ResearchCost.
fn research_goals(c: &Ctx<'_, '_>, goals: &mut Vec<Goal>) {
    let w = c.view.world;
    let queue_max = if c.ego.build_queue_max_items > 0 { c.ego.build_queue_max_items as usize } else { usize::MAX };
    let owned = w.books.kingdom(c.view.kingdom).map(|k| k.technologies.clone()).unwrap_or_default();
    let mut queued: Vec<String> = Vec::new();
    for b in c.view.buildings.iter().filter(|b| b.owner == Some(c.view.me)) {
        if let Some(bk) = w.books.actor(b.id) {
            queued.extend(bk.queue.items.iter().filter_map(|i| i.research.clone()));
        }
    }
    let mut seen: Vec<String> = Vec::new();
    for b in c.view.buildings.iter().filter(|b| b.owner == Some(c.view.me) && b.queue < queue_max) {
        if w.books.actor(b.id).is_some_and(|bk| bk.deploy.is_some()) {
            continue;
        }
        for (i, item) in c.def(b.def).research.iter().enumerate() {
            let tech = item.property.to_ascii_lowercase();
            if owned.iter().chain(queued.iter()).chain(seen.iter()).any(|t| t.eq_ignore_ascii_case(&tech)) {
                continue;
            }
            seen.push(tech.clone());
            let mut g = Goal::new(Key::Research(b.id, i as u32), data::RESEARCH, c.ego.tech_priority(&tech), c.region_of(b.pos), b.pos);
            for (slot, name) in RESOURCES.iter().enumerate() {
                if let Some(r) = w.data.resource_index(name) {
                    g.cost[slot] = item.cost.get(r).copied().unwrap_or(0.0);
                }
            }
            g.research = Some((b.id, item.property.clone()));
            goals.push(g);
        }
    }
}

/// ATTACH goals (`7BE260`): a supply-using regiment whose HQ is gone is
/// attached to the first of the player's HQs that accepts it (`574A10`);
/// unless `attach_when_engaged`, not while engaged. Priority: its maximum CV
/// (`4D01E0`) times the Attach `cv_multiplier`, plus its definition value.
fn attach_goals(c: &Ctx<'_, '_>, val: &Val<'_, '_, '_>, goals: &mut Vec<Goal>) {
    let w = c.view.world;
    let mut hqs: Vec<ActorId> = c
        .view
        .buildings
        .iter()
        .filter(|b| b.owner == Some(c.view.me) && crate::production::hq_org(w, b.id).is_some())
        .map(|b| b.id)
        .collect();
    hqs.sort_unstable();
    let mut taken: HashMap<ActorId, u32> = HashMap::new();
    for o in c.view.orgs.iter().filter(|o| o.owner == c.view.me && o.military) {
        let org = &w.orgs[o.id as usize];
        let def = c.def(org.def);
        if def.max_attachments.is_some() || !def.supply_used {
            continue;
        }
        if org.hq.is_some_and(|h| w.actor(h).is_some_and(|a| a.alive && a.kingdom == Some(c.view.kingdom))) {
            continue;
        }
        if !c.ego.attach_when_engaged && (o.engaged || o.routing) {
            continue;
        }
        let Some(hq) = hqs.iter().copied().find(|&h| {
            let ho = crate::production::hq_org(w, h).expect("listed HQ");
            let max = c.def(w.orgs[ho as usize].def).max_attachments.unwrap_or(0);
            let reserved = w.books.org(ho).map_or(0, |b| b.reserved_attachments);
            crate::production::attachments(w, h) + reserved + taken.get(&h).copied().unwrap_or(0) < max
        }) else {
            continue;
        };
        *taken.entry(hq).or_default() += 1;
        let value = c.cv(org.def) * c.ego.attach_cv_multiplier + val.actor_priority(org.def, false);
        let mut g = Goal::new(Key::Attach(o.id), data::ATTACH, value, c.region_of(o.pos), o.pos);
        let factor = w.data.rule_ci("HQAttachCostPercentage").unwrap_or(0.0);
        let cost = c.quote(org.def, crate::production::Vector::Cost);
        g.cost = [cost[0] * factor, cost[1] * factor, cost[2] * factor];
        g.attach = Some((o.id, hq));
        goals.push(g);
    }
}

// ---------------------------------------------------------------------------
// Selection and assignment

/// ActorPreference contributions of one organization to a goal type:
/// additive value and, per preference, the number of matches.
fn preferences(c: &Ctx<'_, '_>, kind: usize, o: &super::view::OrgView) -> (f32, Vec<u32>) {
    let gp = c.ego.goal(kind);
    let w = c.view.world;
    let org = &w.orgs[o.id as usize];
    let mut add = 0.0;
    let mut counts = Vec::with_capacity(gp.preferences.len());
    for pref in &gp.preferences {
        let member_match = |d: DefId| -> bool {
            match (&pref.property, &pref.actor) {
                (Some(p), _) => has(c.props(d), p),
                (None, Some(a)) => c.def(d).id.eq_ignore_ascii_case(a),
                _ => false,
            }
        };
        let members = org.members.iter().filter(|&&m| w.actors.get(m as usize).is_some_and(|a| a.alive && member_match(a.def))).count() as u32;
        let company = member_match(o.def) || members > 0;
        if company {
            add += if pref.count_value_once_per_company { pref.value } else { pref.value * members.max(1) as f32 };
        }
        counts.push(if company { members.max(1) } else { 0 });
    }
    (add, counts)
}

fn pref_mult(c: &Ctx<'_, '_>, kind: usize, counts: &[u32], companies: &[u32]) -> f32 {
    let gp = c.ego.goal(kind);
    let mut m = 1.0;
    for (i, pref) in gp.preferences.iter().enumerate() {
        let n = counts.get(i).copied().unwrap_or(0);
        if n == 0 {
            continue;
        }
        let times = if pref.count_multiplier_once_per_goal {
            1
        } else if pref.count_multiplier_once_per_company {
            companies.get(i).copied().unwrap_or(1)
        } else {
            n
        };
        m *= pref.multiplier.powi(times as i32);
    }
    m
}

/// The CV an organization brings to a goal: builders count at
/// `penalty_to_builder_cv` for attack and defend goals.
pub(super) fn effective_cv(c: &Ctx<'_, '_>, o: &super::view::OrgView, combat: bool) -> f32 {
    if combat && o.builder { o.cv * c.ego.penalty_to_builder_cv } else { o.cv }
}

pub(super) struct Assign<'a, 'b, 'w> {
    c: &'a Ctx<'b, 'w>,
    prefs: HashMap<(usize, usize), (f32, Vec<u32>)>,
}

impl<'a, 'b, 'w> Assign<'a, 'b, 'w> {
    pub(super) fn new(c: &'a Ctx<'b, 'w>) -> Self {
        Assign { c, prefs: HashMap::new() }
    }
    fn pref(&mut self, kind: usize, o: usize) -> (f32, Vec<u32>) {
        let c = self.c;
        self.prefs.entry((kind, o)).or_insert_with(|| preferences(c, kind, &c.view.orgs[o])).clone()
    }
    fn linked(&self, goals: &[Goal], gi: usize) -> f32 {
        let g = &goals[gi];
        let reach = self.c.ego.linked_region_distance.max(0) as u32;
        goals
            .iter()
            .enumerate()
            .filter(|(j, h)| *j != gi && h.base > 0.0 && h.kind == g.kind && self.c.regions.hops(h.region, g.region) <= reach)
            .map(|(_, h)| h.sum_cv)
            .sum()
    }
    /// `7A3B00` for a hypothetical actor set: the class priority (CV curve,
    /// or explore base + actor factors), plus ActorPreference values, times
    /// their multipliers and the travel weight.
    fn priority(&mut self, goals: &[Goal], gi: usize, add: Option<usize>, remove: Option<usize>) -> f32 {
        let c = self.c;
        let g = &goals[gi];
        if g.base <= 0.0 {
            return 0.0;
        }
        let mut cv = g.sum_cv;
        let mut travel = g.sum_travel;
        let mut n = g.actors.len() as f32;
        let mut pref_add = g.pref_add;
        let mut counts = g.pref_counts.clone();
        let mut companies: Vec<u32> = vec![0; counts.len()];
        let combat = g.combat();
        let mut adjust = |o: usize, sign: f32, this: &mut Self| {
            let ov = &c.view.orgs[o];
            cv += sign * effective_cv(c, ov, combat);
            travel += sign * c.travel_time(ov.pos, g.target, ov.speed);
            n += sign;
            let (a, k) = this.pref(g.kind, o);
            pref_add += sign * a;
            if counts.len() < k.len() {
                counts.resize(k.len(), 0);
                companies.resize(k.len(), 0);
            }
            for (i, v) in k.iter().enumerate() {
                if sign > 0.0 {
                    counts[i] += v;
                } else {
                    counts[i] = counts[i].saturating_sub(*v);
                }
            }
        };
        if let Some(o) = add {
            adjust(o, 1.0, self);
        }
        if let Some(o) = remove {
            adjust(o, -1.0, self);
        }
        if n <= 0.0 && g.kind == data::EXPLORE {
            return 0.0;
        }
        for (i, k) in counts.iter().enumerate() {
            companies[i] = (*k > 0) as u32;
        }
        let class = if g.combat() {
            let linked = self.linked(goals, gi);
            cv_curve(c.ego, g.base, cv + linked + g.static_cv, g.target_cv, g.max_cv, g.attack, g.allied_cv)
        } else if g.kind == data::EXPLORE {
            let e = &c.ego.exploration[g.explore.unwrap_or(1)];
            let mut v = g.base;
            for &o in g.actors.iter().chain(add.iter()).filter(|o| Some(**o) != remove) {
                let ov = &c.view.orgs[o];
                v += e.move_rate_mult * ov.speed.max(0.0).powf(e.move_rate_exp) + e.sight_mult * ov.sight.max(0.0).powf(e.sight_exp);
                let cvt = e.cv_mult * ov.cv.max(0.0).powf(e.cv_exp);
                v += if e.subtract_cv { -cvt } else { cvt };
            }
            v
        } else if g.kind == data::BOMBARD {
            // 7C6500 (BOMBARD +7C): +38 times the hypothetical actors'
            // combat values (actor record +1C, 4D0180) over the
            // Bombardement balancing_constant (ego +458).
            let bc = c.ego.bombard_balancing_constant;
            if bc > 0.0 { g.base * cv / bc } else { 0.0 }
        } else {
            g.base
        };
        let avg = if n > 0.0 { travel / n } else { 0.0 };
        (class + pref_add) * pref_mult(c, g.kind, &counts, &companies) * c.travel_weight(avg)
    }
    pub(super) fn add(&mut self, goals: &mut [Goal], gi: usize, o: usize) {
        let c = self.c;
        let (a, k) = self.pref(goals[gi].kind, o);
        let g = &mut goals[gi];
        let ov = &c.view.orgs[o];
        g.sum_cv += effective_cv(c, ov, g.combat());
        g.sum_travel += c.travel_time(ov.pos, g.target, ov.speed);
        g.pref_add += a;
        if g.pref_counts.len() < k.len() {
            g.pref_counts.resize(k.len(), 0);
        }
        for (i, v) in k.iter().enumerate() {
            g.pref_counts[i] += v;
        }
        g.actors.push(o);
    }
    fn remove(&mut self, goals: &mut [Goal], gi: usize, o: usize) {
        let c = self.c;
        let (a, k) = self.pref(goals[gi].kind, o);
        let g = &mut goals[gi];
        let ov = &c.view.orgs[o];
        g.sum_cv -= effective_cv(c, ov, g.combat());
        g.sum_travel -= c.travel_time(ov.pos, g.target, ov.speed);
        g.pref_add -= a;
        for (i, v) in k.iter().enumerate() {
            if let Some(slot) = g.pref_counts.get_mut(i) {
                *slot = slot.saturating_sub(*v);
            }
        }
        g.actors.retain(|&x| x != o);
    }
}

// ---------------------------------------------------------------------------
// Execution

fn execute(c: &Ctx<'_, '_>, data: &AiData, player: &mut Player, rng: &mut SaiRng, goals: &[Goal], log: &mut Vec<String>) {
    let w = c.view.world;
    let me = c.view.kingdom;
    let mut commands = Vec::new();
    let mut stock = c.view.stockpile;
    let mut used_buildings: Vec<ActorId> = Vec::new();
    let mut assigned = BTreeMap::new();
    // 7A59C0: the selected goals run in order of their current priority
    // (comparator 7A3070, goal +0x34). 7A39C0 pays each from the running
    // stockpile (7A3860 without income: the purchase resources, money);
    // a goal it cannot pay now but could within saving_time_limit
    // (7A3860 with income) starts saving, and no later goal with a purchase
    // cost runs this think (7A3980). Goals without a cost always run.
    let mut order: Vec<usize> = (0..goals.len()).filter(|&i| goals[i].selected).collect();
    order.sort_by(|&a, &b| goals[b].current.partial_cmp(&goals[a].current).unwrap_or(std::cmp::Ordering::Equal));
    let saving_rate = c.ego.saving_time_limit / 60.0;
    let mut saving = false;
    let mut pay = |cost: [f32; 3], stock: &mut [f32; 3]| -> bool {
        let money = cost[0];
        if money <= 0.0 {
            return true;
        }
        if saving {
            return false;
        }
        if money <= stock[0] + 1e-3 {
            stock[0] -= money;
            return true;
        }
        if money <= stock[0] + c.view.income[0].max(0.0) * saving_rate + 1e-3 {
            saving = true;
        }
        false
    };
    for gi in order {
        let g = &goals[gi];
        match g.kind {
            data::RECRUIT => {
                let Some((d, b)) = g.recruit else { continue };
                // 7A39C0 pays first; then 7CDA10 sends the recruit order,
                // one per recruiter up to build_queue_max_items.
                if !pay(g.cost, &mut stock) || used_buildings.contains(&b) {
                    continue;
                }
                used_buildings.push(b);
                commands.push(Command::Recruit { kingdom: me, building: b, def: d });
                player.memory.recruited.push((c.now, d));
                log.push(format!("recruit {} at {} (priority {:.0})", c.def(d).id, c.def(w.actors[b as usize].def).id, g.base));
            }
            data::RESEARCH => {
                let Some((b, tech)) = g.research.clone() else { continue };
                // 7D0820: one order per building up to build_queue_max_items.
                if !pay(g.cost, &mut stock) || used_buildings.contains(&b) {
                    continue;
                }
                used_buildings.push(b);
                log.push(format!("research {tech} at {} (priority {:.0})", c.def(w.actors[b as usize].def).id, g.base));
                commands.push(Command::Research { kingdom: me, building: b, property: tech });
            }
            data::ATTACH => {
                let Some((org, hq)) = g.attach else { continue };
                if !pay(g.cost, &mut stock) {
                    continue;
                }
                log.push(format!("attach {} to {} (priority {:.0})", c.def(w.orgs[org as usize].def).id, c.def(w.actors[hq as usize].def).id, g.base));
                commands.push(Command::Attach { kingdom: me, org, hq });
            }
            data::UNPACK => {
                let (Key::Unpack(truck), Some((at, facing, region))) = (g.key, g.site) else { continue };
                commands.push(Command::Deploy { kingdom: me, truck, at, facing });
                let memo = player.memory.unpack.entry(truck).or_default();
                memo.region = region;
                memo.site = at;
                memo.ordered_at = c.now;
                memo.sent_at = 0.0;
                player.memory.deployed.push((c.now, w.actors[truck as usize].def));
                log.push(format!("deploy {} at ({:.0},{:.0}) (priority {:.0})", c.def(w.actors[truck as usize].def).id, at.x, at.y, g.base));
            }
            data::RECOVER => {
                // 7C5740 (RECOVER +58), every think it stays selected: the goal's +5C for its
                // live actor (7C9E20 -> 7C8AF0) goes on the player's queue (7B4FA0).
                for &o in &g.actors {
                    let ov = &c.view.orgs[o];
                    assigned.insert(ov.id, Assignment { key: g.key, target: g.target, target_actor: None, thing: None, region: g.region, since: c.now, combat: false, hold: 0.0 });
                    let order = super::rai::retreat_order(c, player, o, None, rng);
                    if let Some(s) = player.rai.states.get_mut(&ov.id) {
                        s.kind = super::rai::StateKind::Recover;
                    }
                    commands.push(order);
                    log.push(format!("recover {} (health {:.2}, morale {:.2})", c.def(ov.def).id, ov.health, ov.morale));
                }
            }
            data::BOMBARD => {
                // 7C8340 (BOMBARD +58): while the target stands, is known
                // (visible +70 or the kingdom's record 4EA430) and hostile,
                // every assigned actor is sent the goal's +5C order
                // (7C84F0): bombard the target when visible, else the
                // record position (command flag 4).
                let Some(t) = g.target_actor else { continue };
                let target = if crate::fog::actor_visible(w, t, Some(me)) { AbilityTarget::Actor(t) } else { AbilityTarget::Point(g.target) };
                for &o in &g.actors {
                    let ov = &c.view.orgs[o];
                    assigned.insert(ov.id, Assignment { key: g.key, target: g.target, target_actor: Some(t), thing: Some(Unit::Actor(t)), region: g.region, since: c.now, combat: false, hold: 0.0 });
                    let busy = w.combat.org(ov.id).and_then(|s| s.task.as_ref()).is_some_and(|task| *task == crate::combat::tasks::Task::Bombard(target));
                    if busy || player.pending(ov.id, c.now) {
                        continue;
                    }
                    commands.push(Command::Bombard { kingdom: me, units: vec![Unit::Org(ov.id)], target });
                    log.push(format!("bombard {} with {} (priority {:.0})", c.def(w.actors[t as usize].def).id, c.def(ov.def).id, g.current));
                }
            }
            data::REPAIR => {
                // 7A40F0 (REPAIR +58): each live actor not routed that its RECOVER goal does
                // not want (7C9CE0 with the goal's current priority) gets the goal's +5C
                // (7A5630, the repair command 51E870 at the target).
                let Some(t) = g.target_actor else { continue };
                for &o in &g.actors {
                    let ov = &c.view.orgs[o];
                    assigned.insert(ov.id, Assignment { key: g.key, target: g.target, target_actor: Some(t), thing: Some(Unit::Actor(t)), region: g.region, since: c.now, combat: false, hold: 0.0 });
                    if ov.routing || super::rai::recover_wanted(c, player, o, g.current) {
                        continue;
                    }
                    let busy = w.combat.org(ov.id).and_then(|s| s.task.as_ref()).is_some_and(|task| *task == crate::combat::tasks::Task::Repair(t));
                    if !busy && !player.pending(ov.id, c.now) {
                        commands.push(Command::Repair { kingdom: me, units: vec![Unit::Org(ov.id)], target: t });
                        log.push(format!("repair {} with {} (priority {:.0})", c.def(w.actors[t as usize].def).id, c.def(ov.def).id, g.current));
                    }
                }
            }
            data::AIR_ATTACK => {
                // 7C79C0 (AIR_ATTACK +58): the denizen's air_attack order
                // (7C7C30: the target when visible, else its record
                // position), paid with the AirAttackCost; the goal's +40
                // takes the attack time.
                let (Some(t), Some(field)) = (g.target_actor, g.airfield) else { continue };
                if !pay(g.cost, &mut stock) {
                    continue;
                }
                let target = if crate::fog::actor_visible(w, t, Some(me)) { AbilityTarget::Actor(t) } else { AbilityTarget::Point(g.target) };
                commands.push(Command::AirAttack { kingdom: me, building: field, target });
                player.memory.air_attacked.insert(t, c.now);
                log.push(format!("air attack {} from {} (priority {:.0})", c.def(w.actors[t as usize].def).id, c.def(w.actors[field as usize].def).id, g.current));
            }
            data::EXPLORE if g.airfield.is_some() => {
                // An air-recon goal (7C3440 with a recon denizen): the
                // region from the airfield, the air_recon order there, paid
                // with the AirReconCost.
                let Some(field) = g.airfield else { continue };
                if !pay(g.cost, &mut stock) {
                    continue;
                }
                let from = w.actors[field as usize].pos.truncate();
                let region = explore_region(c, player, g, from, rng).unwrap_or_else(|| c.region_of(from));
                let target = c.regions.centers[region as usize];
                player.memory.explore_region.insert(g.key, region);
                commands.push(Command::AirRecon { kingdom: me, building: field, target: AbilityTarget::Point(target) });
                log.push(format!("explore by air from {} to region {region} ({:.0},{:.0})", c.def(w.actors[field as usize].def).id, target.x, target.y));
            }
            data::EXPLORE => {
                let Some(&o) = g.actors.first() else { continue };
                let ov = &c.view.orgs[o];
                // 7C3440: no candidate region -> the actor's own region; none at all:
                // stop (0x40).
                let region = explore_region(c, player, g, ov.pos, rng).unwrap_or_else(|| c.region_of(ov.pos));
                let target = c.regions.centers[region as usize];
                let same = player.memory.explore_region.get(&g.key) == Some(&region) && player.memory.explore_actor.get(&g.key) == Some(&ov.id);
                player.memory.explore_region.insert(g.key, region);
                player.memory.explore_actor.insert(g.key, ov.id);
                assigned.insert(ov.id, Assignment { key: g.key, target, target_actor: None, thing: None, region, since: c.now, combat: false, hold: 0.0 });
                if !(same && matches!(ov.order, Order::Move { .. })) {
                    // explore (9) at the region's centre, 7C29C0: 0x44.
                    commands.push(Command::Order { kingdom: me, units: vec![Unit::Org(ov.id)], order: crate::orders::ActorOrder::Explore { at: target, scalar: None } });
                    log.push(format!("explore with {} to region {region} ({:.0},{:.0})", c.def(ov.def).id, target.x, target.y));
                }
            }
            kind => {
                // The combat goals. What each actor is worth to the goal ([+2C]).
                let mut a = Assign { c, prefs: HashMap::new() };
                let whole = a.priority(goals, gi, None, None);
                let mut hold = HashMap::new();
                for &o in &g.actors {
                    hold.insert(o, whole - a.priority(goals, gi, None, Some(o)));
                }
                for &o in &g.actors {
                    let ov = &c.view.orgs[o];
                    let previous = player.memory.assigned.get(&ov.id);
                    let since = previous.filter(|p| p.key == g.key).map_or(c.now, |p| p.since);
                    assigned.insert(ov.id, Assignment { key: g.key, target: g.target, target_actor: g.target_actor, thing: g.thing, region: g.region, since, combat: true, hold: hold[&o] });
                }
                if kind == data::DEFEND_REGION {
                    // 7CA1D0: an actor its RECOVER goal wants is sent there (7C9CE0), the
                    // others hold the region (7A1070(1)); every actor holding one has
                    // the goal's +5C, guard (7C5C60).
                    for &o in &g.actors {
                        let ov = &c.view.orgs[o];
                        if super::rai::recover_wanted(c, player, o, hold[&o]) {
                            let order = super::rai::retreat_order(c, player, o, Some(g.region), rng);
                            commands.push(order);
                            continue;
                        }
                        player.rai.states.entry(ov.id).or_insert_with(|| super::rai::OrderState { region: Some(g.region), ..super::rai::OrderState::new(super::rai::StateKind::Defend) });
                    }
                    let mut sent = 0;
                    for &o in &g.actors {
                        let ov = &c.view.orgs[o];
                        if player.rai.states.get(&ov.id).is_some_and(|s| s.kind == super::rai::StateKind::Defend) {
                            commands.push(super::rai::guard_order(c, ov.id, g.thing, g.region));
                            sent += 1;
                        }
                    }
                    if sent > 0 {
                        log.push(format!("defend region {} with {} orgs (cv {:.0}/{:.0}, priority {:.0})", g.region, g.actors.len(), g.sum_cv, g.target_cv, g.current));
                    }
                } else {
                    // 7C9710 (ATTACK_REGION), 7C6FC0 (ATTACK_ACTOR): 7A5E10 sends off
                    // those going to recover; the rest come out of any state
                    // (7A1070(0)) and go as one group (7ADB80, 7AF220) at the target
                    // thing (4DDE60, move 0x59), or the region's centre (0x5C).
                    let list = super::rai::retreat_check(c, player, g.region, |o| hold[&o], g.actors.clone(), rng, &mut commands);
                    if list.is_empty() {
                        continue;
                    }
                    let mut units = Vec::with_capacity(list.len());
                    for &o in &list {
                        let ov = &c.view.orgs[o];
                        player.rai.states.insert(ov.id, super::rai::OrderState::new(super::rai::StateKind::Attack));
                        units.push(Unit::Org(ov.id));
                    }
                    let target = g.thing.and_then(|t| crate::ids::target_actor(w, t));
                    commands.push(match target {
                        Some(target) => Command::Follow { kingdom: me, units, target },
                        None => Command::Move { kingdom: me, units, to: c.regions.centers[g.region as usize], attack_move: false },
                    });
                    player.memory.attacks.push((c.now, g.region, list.len()));
                    log.push(format!("attack region {} with {} orgs (cv {:.0}/{:.0}, priority {:.0})", g.region, g.actors.len(), g.sum_cv, g.target_cv, g.current));
                }
            }
        }
    }
    player.memory.assigned = assigned;
    player.memory.air_attacked.retain(|&t, _| w.actor(t).is_some_and(|a| a.alive));
    let profile = &data.profiles[player.profile];
    enqueue(player, data, c.now, c.ego.reaction_time, profile.max_ge_commands, commands);
}

/// Explore region choice of an explore goal with an actor (`7C2C20`, run by
/// the goal's execution `7C3440`). Every region of the actor's island gets
/// the largest of these candidate bonuses:
/// - the actor's region: `actor_region_bonus`;
/// - the region this goal explored last think: `current_region_bonus`;
/// - close goals only: the regions of the player's buildings,
///   `building_region_bonus`;
/// - intuition: each region, with `intuition_probability`, counts the cities
///   (`intuition_count_settlements`) and the hostile buildings outside a city
///   (`intuition_count_other_buildings`) standing there, seen or not; with
///   any, and within `intuition_distance` regions of one of the player's
///   buildings when that is not negative, `intuition_region_bonus` (times
///   count ^ `intuition_region_exponent` when that is positive);
/// - `num_random_candidates` random island regions: `random_candidate_bonus`.
///
/// Regions with a positive bonus are valued by `4A9450` and the best is
/// explored; None when no region qualifies (`7C3440` then uses the actor's
/// own region).
fn explore_region(c: &Ctx<'_, '_>, player: &Player, g: &Goal, from: Vec2, rng: &mut SaiRng) -> Option<u32> {
    let kind = g.explore.unwrap_or(1);
    let e = &c.ego.exploration[kind];
    let w = c.view.world;
    let n = c.regions.len();
    let home = c.region_of(from);
    let island: Vec<u32> = (0..n as u32).filter(|&r| c.regions.same_island(r, home)).collect();
    let mut bonus = vec![0.0f32; n];
    let raise = |bonus: &mut Vec<f32>, r: u32, v: f32| {
        if let Some(b) = bonus.get_mut(r as usize)
            && *b < v
        {
            *b = v;
        }
    };
    raise(&mut bonus, home, e.actor_region_bonus);
    let last = player.memory.explore_region.get(&g.key).copied();
    if let Some(r) = last {
        raise(&mut bonus, r, e.current_region_bonus);
    }
    let mine: Vec<u32> = c.view.buildings.iter().filter(|b| b.owner == Some(c.view.me)).map(|b| c.region_of(b.pos)).collect();
    if kind == 0 {
        for &r in &mine {
            raise(&mut bonus, r, e.building_region_bonus);
        }
    }
    // Intuition reads the region's actors themselves, not what the player
    // has seen (the original walks each region's actor list).
    let mut items = vec![0u32; n];
    for a in w.actors.iter().filter(|a| a.alive && !a.removed) {
        let def = c.def(a.def);
        if def.class != Some(crate::data::DefClass::Building) && !def.city {
            continue;
        }
        let owner = a.kingdom.and_then(|k| super::view::kingdom_index(w, k));
        if owner == Some(c.view.me) || owner.is_some_and(|o| c.view.allied[o]) {
            continue;
        }
        let city = def.city && def.city_zone_radius > 0.0;
        let counted = if city {
            e.intuition_count_settlements
        } else {
            // Other buildings: a hostile kingdom's, not a city's blocks.
            e.intuition_count_other_buildings && owner.is_some_and(|o| c.view.hostile[o] && !w.kingdoms[o].independent)
        };
        if counted {
            items[c.region_of(a.pos.truncate()) as usize] += 1;
        }
    }
    for &r in island.iter().rev() {
        if rng.unit() > e.intuition_probability {
            continue;
        }
        let count = items[r as usize];
        if count == 0 {
            continue;
        }
        if e.intuition_distance >= 0 && !mine.iter().any(|&m| c.regions.hops(m, r) as i32 <= e.intuition_distance) {
            continue;
        }
        let v = if e.intuition_region_exponent > 0.0 { e.intuition_region_bonus * (count as f32).powf(e.intuition_region_exponent) } else { e.intuition_region_bonus };
        raise(&mut bonus, r, v);
    }
    for _ in 0..e.num_random_candidates.max(0) {
        if island.is_empty() {
            break;
        }
        let r = island[rng.below(island.len())];
        raise(&mut bonus, r, e.random_candidate_bonus);
    }
    let center = {
        let pts: Vec<Vec2> = c.view.buildings.iter().filter(|b| b.owner == Some(c.view.me)).map(|b| b.pos).collect();
        if pts.is_empty() { from } else { pts.iter().copied().sum::<Vec2>() / pts.len() as f32 }
    };
    let mut best: Option<(u32, f32)> = None;
    for r in 0..n as u32 {
        let b = bonus[r as usize];
        if b <= 0.0 || !c.regions.same_island(r, home) {
            continue;
        }
        let v = explore_value(c, player, e, b, r, last, from, center, rng);
        if v > best.map_or(0.0, |x| x.1) {
            best = Some((r, v));
        }
    }
    best.map(|b| b.0)
}

/// `4A9450`: the value of exploring region `r` with candidate bonus `bonus`.
/// Fog: within `fog_search_depth` regions the nearest region whose explored
/// share is at most `percent_to_count_region_explored` scales the priority
/// from `black_fog_priority` (the region itself) to `grey_fog_priority`
/// (none in reach; a negative depth searches nothing). Then the
/// kingdom-centre, last-region and actor distance multipliers (a candidate
/// beyond a positive cap is worth 0), the time since the region was last
/// scouted (`min(t, max) ^ exp`, for a region not itself unexplored) and
/// `fuzzy_factor` times a draw of the SAI RNG.
#[allow(clippy::too_many_arguments)]
fn explore_value(c: &Ctx<'_, '_>, player: &Player, e: &super::data::Exploration, bonus: f32, r: u32, last: Option<u32>, from: Vec2, center: Vec2, rng: &mut SaiRng) -> f32 {
    let depth = e.fog_search_depth;
    let mut steps = (depth + 1).max(0);
    let span = if steps < 2 {
        steps = 1;
        1
    } else {
        steps
    };
    if depth >= 0 {
        for q in c.regions.within(r, depth as u32) {
            let hops = c.regions.hops(r, q) as i32;
            if hops < steps && c.regions.same_island(q, r) && explored_share(c, q) <= e.percent_to_count_region_explored {
                steps = hops;
            }
        }
    }
    let mut v = e.black_fog_priority - (e.black_fog_priority - e.grey_fog_priority) * (steps as f32 / span as f32) + bonus;
    let p = c.regions.centers[r as usize];
    let factor = |d: f32, max: f32, close: bool, exp: f32, floor: f32, ceil: f32| -> f32 {
        let mut m = d.min(max) / if max > 0.0 { max } else { 1.0 };
        if close {
            m = 1.0 - m;
        }
        (ceil - floor) * m.max(0.0).powf(exp) + floor
    };
    if e.consider_distance_to_kingdom_center {
        let d = p.distance(center);
        if e.kingdom_center_distance_cap > 0.0 && d > e.kingdom_center_distance_cap {
            return 0.0;
        }
        v *= factor(d, e.kingdom_center_distance_max, e.kingdom_center_distance_prefer_close, e.kingdom_center_distance_exp, e.kingdom_center_distance_floor, e.kingdom_center_distance_ceil);
    }
    if e.consider_distance_to_last_region
        && let Some(l) = last
    {
        let d = c.regions.centers[l as usize].distance(p);
        if e.last_region_distance_cap > 0.0 && d > e.last_region_distance_cap {
            return 0.0;
        }
        v *= factor(d, e.last_region_distance_max, e.last_region_distance_prefer_close, e.last_region_distance_exp, e.last_region_distance_floor, e.last_region_distance_ceil);
    }
    if e.consider_distance_to_actor {
        let d = from.distance(p);
        if e.actor_distance_cap > 0.0 && d > e.actor_distance_cap {
            return 0.0;
        }
        v *= factor(d, e.actor_distance_max, e.actor_distance_prefer_close, e.actor_distance_exp, e.actor_distance_floor, e.actor_distance_ceil);
    }
    if e.use_time_since_last_scouted && steps > 0 {
        let seen = player.scouted.get(r as usize).copied().unwrap_or(f32::NEG_INFINITY);
        let t = if seen.is_finite() { c.now - seen } else { c.now };
        if t > 0.0 {
            v *= t.min(e.time_since_last_scouted_max).powf(e.time_since_last_scouted_exp);
        }
    }
    rng.unit() * e.fuzzy_factor + v
}

/// The share of a region's tiles the player's side has explored (grey or
/// white fog).
fn explored_share(c: &Ctx<'_, '_>, r: u32) -> f32 {
    let w = c.view.world;
    let (mut seen, mut all) = (0u32, 0u32);
    for b in c.regions.rects.get(r as usize).into_iter().flatten() {
        for y in b[1].max(0)..=b[3].min(w.map.height as i32 - 1) {
            for x in b[0].max(0)..=b[2].min(w.map.width as i32 - 1) {
                all += 1;
                if crate::fog::tile_view(w, Some(c.view.kingdom), x, y) != crate::fog::TileView::Unexplored {
                    seen += 1;
                }
            }
        }
    }
    if all == 0 { 1.0 } else { seen as f32 / all as f32 }
}
