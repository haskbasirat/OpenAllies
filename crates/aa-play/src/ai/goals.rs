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
//! - military priority with actors `7A3B00` -> `7C9080` -> `7ACF90` CV curve
//!   (`7C6A70` target/maximum CV, linked goals), ActorPreference modifiers
//!   and the travel weight `4A7130`.
use super::data::{self, AiData, Ego};
use super::regions::{Influence, Regions};
use super::view::{View, has};
use super::{Bound, Player, SaiRng, enqueue};
use crate::data::{DefClass, DefId};
use crate::map::{BLOCKED, NOT_LAND_PASSABLE};
use crate::world::{ActorId, Command, Order, OrgId, Unit};
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
}

#[derive(Clone, Debug)]
pub struct Assignment {
    pub key: Key,
    pub target: Vec2,
    pub target_actor: Option<ActorId>,
    pub region: u32,
    pub since: f32,
    /// Attack or defend goal (combat), recover, explore.
    pub combat: bool,
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
}

pub(crate) struct Ctx<'a, 'w> {
    pub view: &'a View<'w>,
    pub bound: &'a Bound,
    pub ego: &'a Ego,
    pub regions: &'a Regions,
    pub inf: &'a Influence,
    pub now: f32,
}

const RESOURCES: [&str; 3] = ["money", "ammo", "oil"];

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
    /// ResourceManagement need per point of a stockpilable resource.
    pub fn need(&self, r: usize) -> f32 {
        let e = self.ego;
        let income = self.view.income[r];
        let (short, enough, plenty_value, plenty) = match e.resource_needs.iter().find(|n| n.resource == RESOURCES[r]) {
            Some(n) => (n.short_need, n.enough_need, n.plenty_value, n.plenty_need),
            None => (e.stockpilable_short_need, e.stockpilable_enough_need, e.stockpilable_plenty_value, e.stockpilable_plenty_need),
        };
        if income < 0.0 {
            short
        } else if plenty_value != 0.0 && income > plenty_value {
            plenty
        } else {
            enough
        }
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
    /// Per-minute production minus upkeep of a definition and its slots.
    pub fn net(&self, d: DefId) -> [f32; 3] {
        let p = self.quote(d, crate::production::Vector::Production);
        let u = self.quote(d, crate::production::Vector::Upkeep);
        [p[0] - u[0], p[1] - u[1], p[2] - u[2]]
    }
    /// Ego ActorValue of a definition (`7AD2D0` closest reading):
    /// `scaling_mult * (economic + military_cv_mult * cv * military_value_scaling_mult)`.
    pub fn actor_value_cv(&self, d: DefId, cv: f32) -> f32 {
        let e = self.ego;
        let net = self.net(d);
        let economic: f32 = (0..3).map(|r| net[r] * self.need(r)).sum();
        e.scaling_mult * (economic + e.military_cv_mult * cv * e.military_value_scaling_mult)
    }
    pub fn actor_value(&self, d: DefId) -> f32 {
        self.actor_value_cv(d, self.cv(d))
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
            actors: Vec::new(),
            sum_cv: 0.0,
            sum_travel: 0.0,
            pref_add: 0.0,
            pref_counts: Vec::new(),
            current: 0.0,
            selected: false,
        }
    }
    fn combat(&self) -> bool {
        self.kind == data::ATTACK_REGION || self.kind == data::DEFEND_REGION
    }
}

/// Counts of the player's own things (live, queued) by definition and property.
#[derive(Default)]
struct Census {
    by_def: HashMap<DefId, i32>,
    by_prop: HashMap<String, i32>,
}

impl Census {
    fn add(&mut self, d: DefId, props: &[String]) {
        *self.by_def.entry(d).or_default() += 1;
        for p in props {
            *self.by_prop.entry(p.clone()).or_default() += 1;
        }
    }
    fn build(c: &Ctx<'_, '_>) -> Self {
        let mut s = Census::default();
        let w = c.view.world;
        for o in c.view.orgs.iter().filter(|o| o.owner == c.view.me) {
            s.add(o.def, c.props(o.def));
        }
        for b in c.view.buildings.iter().filter(|b| b.owner == Some(c.view.me)) {
            if w.actors[b.id as usize].org.is_none() {
                s.add(b.def, c.props(b.def));
            }
            let queued = w.books.actor(b.id).map(|bk| bk.queue.items.iter().map(|i| i.def).collect::<Vec<_>>()).unwrap_or_default();
            let queued = if queued.is_empty() { w.actors[b.id as usize].queue.iter().map(|(d, _)| *d).collect() } else { queued };
            for d in queued {
                s.add(d, c.props(d));
            }
        }
        for t in c.view.trucks.iter().filter(|t| t.owner == c.view.me && t.org.is_none()) {
            s.add(t.def, c.props(t.def));
        }
        s
    }
    fn def(&self, d: DefId) -> i32 {
        self.by_def.get(&d).copied().unwrap_or(0)
    }
    fn prop(&self, p: &str) -> i32 {
        self.by_prop.get(p).copied().unwrap_or(0)
    }
}

/// `ActorPriority` bonus of recruiting `d` and the per-think maximum.
fn actor_priority(c: &Ctx<'_, '_>, census: &Census, d: DefId) -> (f32, i32) {
    let props = c.props(d);
    let id = c.def(d).id.to_ascii_lowercase();
    let mut value = 0.0;
    let mut max = c.ego.default_max_recruited_per_think;
    for ap in &c.ego.actor_priorities {
        let matched = match (&ap.property, &ap.actor) {
            (Some(p), _) => has(props, p),
            (None, Some(a)) => *a == id,
            _ => false,
        };
        if !matched {
            continue;
        }
        let count = match &ap.property {
            Some(p) if ap.count_all_property_actors => census.prop(p),
            _ => census.def(d),
        };
        value += ap.value - ap.repeat_penalty * count as f32;
        if count == 0 {
            value += ap.one_time_bonus;
        }
        if ap.max_recruited_per_think != 0 {
            max = ap.max_recruited_per_think;
        }
    }
    (value, max)
}

#[allow(clippy::too_many_arguments)]
pub fn think(
    view: &View<'_>,
    bound: &Bound,
    data: &AiData,
    regions: &Regions,
    inf: &Influence,
    player: &mut Player,
    rng: &mut SaiRng,
    log: &mut Vec<String>,
    logging: bool,
) {
    select_ego(view, data, player, rng, log);
    let ego = &data.egos[player.ego];
    let c = Ctx { view, bound, ego, regions, inf, now: view.world.time };
    update_knowledge(&c, player);
    update_flares(&c, player);
    let census = Census::build(&c);
    let mut goals = Vec::new();
    unpack_goals(&c, player, rng, &mut goals);
    recover_goals(&c, &mut goals);
    defend_goals(&c, &mut goals);
    attack_goals(&c, player, &mut goals);
    apply_flares(&c, player, &mut goals);
    explore_goals(&c, player, &mut goals);
    recruit_goals(&c, &census, rng, &mut goals);
    research_goals(&c, &mut goals);
    attach_goals(&c, &mut goals);
    for g in &mut goals {
        finalize(g, ego, &player.memory, rng);
    }
    // 7A93B0: candidates sorted by base priority (comparator 7A30B0).
    goals.sort_by(|a, b| b.base.partial_cmp(&a.base).unwrap_or(std::cmp::Ordering::Equal));
    select(&c, player, rng, &mut goals);
    if logging {
        let mut shown = 0;
        for g in goals.iter().filter(|g| g.base > 0.0) {
            let other = matches!(g.key, Key::Attack(_) | Key::Defend(_));
            if other && shown >= 6 {
                continue;
            }
            shown += other as usize;
            log.push(format!(
                "  goal {} base {:.0} current {:.0} actors {} cv {:.0}/{:.0}{}",
                match g.key {
                    Key::Recruit(_, d) => format!("Recruit {} cost {:?}", c.def(d).id, g.cost),
                    k => format!("{k:?}"),
                },
                g.base,
                g.current,
                g.actors.len(),
                g.sum_cv,
                g.target_cv,
                if g.selected { " selected" } else { "" }
            ));
        }
    }
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

/// Seen records: hostile static actors within sight of the player's actors
/// become known; regions in sight are scouted now.
fn update_knowledge(c: &Ctx<'_, '_>, player: &mut Player) {
    let w = c.view.world;
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
    for b in &c.view.buildings {
        if b.owner == Some(c.view.me) {
            continue;
        }
        if eyes.iter().any(|&(p, r)| p.distance(b.pos) <= r + b.radius) {
            player.known.insert(b.id);
        }
    }
    player.known.retain(|&id| w.actor(id).is_some_and(|a| a.alive));
}

/// Internal flares (`[InternalFlares]`, ego +8C0..+908): a structure that
/// loses health is defended (`defend_attacked_target_flare_*`), one taken by
/// the enemy is counter-attacked (`counter_attack_captured_actor_flare_*`).
fn update_flares(c: &Ctx<'_, '_>, player: &mut Player) {
    let f = &c.ego.flares;
    player.flares.retain(|(_, _, until)| *until > c.now);
    let mut health = BTreeMap::new();
    for b in c.view.buildings.iter().filter(|b| b.owner == Some(c.view.me)) {
        let a = &c.view.world.actors[b.id as usize];
        if player.structure_health.get(&b.id).is_some_and(|&h| a.health < h - 1e-3) {
            player.flares.push((Key::Defend(c.region_of(b.pos)), f.defend_attacked_target_priority, c.now + f.defend_attacked_target_duration));
        }
        health.insert(b.id, a.health);
    }
    for &id in player.structure_health.keys() {
        let a = &c.view.world.actors[id as usize];
        if a.alive && !a.removed && a.kingdom.is_some_and(|k| k != c.view.kingdom) {
            player.flares.push((Key::Attack(c.region_of(a.pos.truncate())), f.counter_attack_captured_actor_priority, c.now + f.counter_attack_captured_actor_duration));
        }
    }
    player.structure_health = health;
}

/// A flare adds its priority to its goal and the goals linked to it.
fn apply_flares(c: &Ctx<'_, '_>, player: &Player, goals: &mut [Goal]) {
    let reach = c.ego.linked_region_distance.max(0) as u32;
    for (key, priority, _) in &player.flares {
        let (kind, region) = match *key {
            Key::Defend(r) => (data::DEFEND_REGION, r),
            Key::Attack(r) => (data::ATTACK_REGION, r),
            _ => continue,
        };
        for g in goals.iter_mut().filter(|g| g.kind == kind && c.regions.hops(g.region, region) <= reach) {
            g.raw += priority;
        }
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
    if let Some(&fuzzy) = memory.selected.get(&g.key) {
        g.fuzzy = fuzzy;
        g.base = p + fuzzy + inertia;
    } else {
        g.fuzzy = rng.unit() * fuzziness;
        g.base = p + g.fuzzy;
    }
}

// ---------------------------------------------------------------------------
// Candidate goals

fn spread_offset(i: usize) -> Vec2 {
    let a = i as f32 * 2.399_963;
    Vec2::new(a.cos(), a.sin()) * (12.0 + 8.0 * (i as f32).sqrt())
}

/// A free, land-passable spot in `region` for a building of `def`.
fn find_site(c: &Ctx<'_, '_>, region: u32, def: DefId, truck: ActorId, reserved: &[(Vec2, f32)]) -> Option<Vec2> {
    let d = c.def(def);
    let radius = d.blocking_size.map_or(4.0, |s| s[0].max(s[1]) * 0.5);
    let pad = c.view.world.data.rule_ci("BuildingPaddingMeters").unwrap_or(2.0);
    let center = c.regions.centers[region as usize];
    let half = c.regions.size * 0.5;
    let (rx, ry) = c.regions.coords(region);
    let lo = Vec2::new(rx as f32, ry as f32) * c.regions.size;
    let step = (radius * 2.0 + pad).max(4.0);
    let rings = (c.regions.size / step).ceil() as i32 + 1;
    for ring in 0..=rings {
        let count = if ring == 0 { 1 } else { 8 * ring };
        for k in 0..count {
            let a = k as f32 / count as f32 * std::f32::consts::TAU;
            let p = center + Vec2::new(a.cos(), a.sin()) * step * ring as f32;
            if p.x < lo.x || p.y < lo.y || p.x > lo.x + 2.0 * half || p.y > lo.y + 2.0 * half {
                continue;
            }
            let corners = [Vec2::new(-radius, -radius), Vec2::new(radius, -radius), Vec2::new(-radius, radius), Vec2::new(radius, radius), Vec2::ZERO];
            if !corners.iter().all(|o| c.land(p + *o)) {
                continue;
            }
            let clear_b = c.view.buildings.iter().all(|b| b.pos.distance(p) >= b.radius + radius + pad);
            let clear_t = c.view.trucks.iter().filter(|t| t.deploying).all(|t| {
                if let Order::Deploy { at, .. } = c.view.world.actors[t.id as usize].order { at.distance(p) >= 2.0 * radius + pad } else { true }
            });
            let clear_r = reserved.iter().all(|(q, r)| q.distance(p) >= r + radius + pad);
            if clear_b && clear_t && clear_r {
                // The deploy owner's own placement and footprint rules.
                let (at, _) = crate::deploy::placement(c.view.world, def, p, 0.0);
                if crate::deploy::spot_clear(c.view.world, def, at, Some(truck)).is_ok()
                    && crate::movement::deploy_spot_problem(c.view.world, truck, at).is_none()
                {
                    return Some(at);
                }
            }
        }
    }
    None
}

/// UNPACK goals (`7CBC40` with the `4A9AE0` value).
fn unpack_goals(c: &Ctx<'_, '_>, player: &mut Player, rng: &mut SaiRng, goals: &mut Vec<Goal>) {
    let u = &c.ego.unpacking;
    let mine: Vec<_> = c.view.buildings.iter().filter(|b| b.owner == Some(c.view.me)).collect();
    let mut reserved: Vec<(Vec2, f32)> = Vec::new();
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
        let mut best: Option<(f32, u32, Vec2)> = None;
        for r in candidates {
            if memo.bad.contains(&r) || !c.regions.same_island(r, home) {
                continue;
            }
            let Some(site) = find_site(c, r, t.unpacks_to, t.id, &reserved) else { continue };
            let center = c.regions.centers[r as usize];
            let mut bonus = 0.0;
            for b in &mine {
                let d = c.regions.centers[c.region_of(b.pos) as usize].distance(center);
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
            if best.is_none_or(|(bv, _, _)| v > bv) {
                best = Some((v, r, site));
            }
        }
        let Some((value, region, site)) = best else { continue };
        let radius = c.def(t.unpacks_to).blocking_size.map_or(4.0, |s| s[0].max(s[1]) * 0.5);
        reserved.push((site, radius));
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
fn recover_goals(c: &Ctx<'_, '_>, goals: &mut Vec<Goal>) {
    let r = &c.ego.recover;
    for (i, o) in c.view.orgs.iter().enumerate() {
        if o.owner != c.view.me || !o.military || o.routing {
            continue;
        }
        let mut need = 0.0f32;
        if o.health < r.max_hp {
            need = (1.0 - o.health.max(0.0).powf(r.exponent_hp)) * r.base_priority_hp;
        }
        if o.morale < r.max_morale {
            need = need.max((1.0 - o.morale.max(0.0).powf(r.exponent_morale)) * r.base_priority_morale);
        }
        if need <= 0.0 {
            continue;
        }
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
fn defend_goals(c: &Ctx<'_, '_>, goals: &mut Vec<Goal>) {
    let dp = &c.ego.defend;
    let mut regions: BTreeMap<u32, (f32, Vec2, f32)> = BTreeMap::new();
    for b in &c.view.buildings {
        let Some(owner) = b.owner else { continue };
        if !c.view.allied[owner] {
            continue;
        }
        let value = c.actor_value(b.def).max(0.0);
        let mut threat = dp.economic_value_multiplier * value.powf(dp.economic_value_exponent);
        if owner != c.view.me {
            threat *= c.ego.defend_friendly_kingdom_actor_multiplier;
        }
        let r = c.region_of(b.pos);
        let e = regions.entry(r).or_insert((0.0, b.pos, 0.0));
        if threat > e.0 {
            e.0 = threat;
            e.1 = b.pos;
        }
        if owner == c.view.me {
            e.2 += b.cv;
        }
    }
    let hostile_orgs: Vec<_> = c.view.orgs.iter().filter(|o| c.view.hostile[o.owner] && o.military).collect();
    for (r, (value, pos, static_cv)) in regions {
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
fn attack_goals(c: &Ctx<'_, '_>, player: &Player, goals: &mut Vec<Goal>) {
    let mut best: BTreeMap<u32, (f32, Vec2, Option<ActorId>)> = BTreeMap::new();
    let mut home_islands: Vec<u32> = c.view.orgs.iter().filter(|o| o.owner == c.view.me).map(|o| c.regions.island[c.region_of(o.pos) as usize]).collect();
    home_islands.extend(c.view.buildings.iter().filter(|b| b.owner == Some(c.view.me)).map(|b| c.regions.island[c.region_of(b.pos) as usize]));
    let reachable = |r: u32| home_islands.contains(&c.regions.island[r as usize]);
    let mut consider = |r: u32, v: f32, p: Vec2, a: Option<ActorId>| {
        let v = v.max(0.001);
        let e = best.entry(r).or_insert((0.0, p, a));
        if v > e.0 {
            *e = (v, p, a);
        }
    };
    for b in &c.view.buildings {
        if b.owner == Some(c.view.me) || !player.known.contains(&b.id) || !attack_target(c, b) {
            continue;
        }
        let r = c.region_of(b.pos);
        if !reachable(r) {
            continue;
        }
        consider(r, c.actor_value_cv(b.def, b.cv), b.pos, Some(b.id));
    }
    let mut eyes: Vec<(Vec2, f32)> = c.view.orgs.iter().filter(|o| o.owner == c.view.me).map(|o| (o.pos, o.sight)).collect();
    for b in c.view.buildings.iter().filter(|b| b.owner == Some(c.view.me)) {
        let d = c.def(b.def);
        eyes.push((b.pos, d.sight_range.max(d.detection_range)));
    }
    for o in c.view.orgs.iter().filter(|o| c.view.hostile[o.owner] && o.cv > 0.0) {
        if !eyes.iter().any(|&(p, s)| p.distance(o.pos) <= s) {
            continue;
        }
        let r = c.region_of(o.pos);
        if reachable(r) {
            consider(r, c.actor_value_cv(o.def, o.cv), o.pos, None);
        }
    }
    let oc = &c.ego.offensive;
    for (r, (value, pos, actor)) in best {
        let mut g = Goal::new(Key::Attack(r), data::ATTACK_REGION, value, r, pos);
        g.attack = true;
        g.target_actor = actor;
        g.target_cv = (c.hostile(r, false) * oc.cv_advantage_target).max(oc.min_cv_to_assign);
        g.max_cv = g.target_cv * oc.cv_advantage_max;
        g.allied_cv = (c.allied(r, true) - c.inf.at(&c.inf.mobile, r, c.view.me)).max(0.0) * c.ego.attack_allied_influence_cv_multiplier;
        goals.push(g);
    }
}

/// The static actors `7BB5E0` admits as region targets: owned by a kingdom
/// the player regards as an enemy, and either a city (object +D4, the
/// CityComponent, admits it regardless of the other tests) or an actor
/// passing the targetable tests (+8 bit 11 clear, bit 14 set, definition
/// +291). Closest reading of those tests here: the buildings of enemy
/// players; an independent's other buildings (city blocks, farms) change
/// hands with their city and are not targets. A city nobody owns is taken
/// by presence and is admitted too.
pub(crate) fn attack_target(c: &Ctx<'_, '_>, b: &super::view::BuildingView) -> bool {
    let def = c.def(b.def);
    let city = def.city && def.city_zone_radius > 0.0;
    match b.owner {
        None => city,
        Some(o) if c.view.hostile[o] => city || !c.view.world.kingdoms[o].independent,
        Some(_) => false,
    }
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

/// EXPLORE goals (`7C29F0`): close (0) and deep (1); air recon needs
/// aircraft. Above its floor the base shrinks by the share of island
/// regions already explored.
fn explore_goals(c: &Ctx<'_, '_>, player: &Player, goals: &mut Vec<Goal>) {
    let home = c.view.buildings.iter().find(|b| b.owner == Some(c.view.me)).map(|b| c.region_of(b.pos));
    let island: Vec<u32> = (0..c.regions.len() as u32).filter(|&r| home.is_some_and(|h| c.regions.same_island(r, h))).collect();
    let explored = island.iter().filter(|&&r| player.scouted.get(r as usize).is_some_and(|t| t.is_finite())).count() as f32;
    for kind in 0..2usize {
        let e = &c.ego.exploration[kind];
        for slot in 0..e.num_goals.max(0) {
            let mut base = e.base_priority - slot as f32 * e.repeat_penalty;
            if base > e.min_priority && !island.is_empty() {
                base -= explored / island.len() as f32 * (base - e.min_priority);
            }
            let mut g = Goal::new(Key::Explore(kind as u8, slot as u8), data::EXPLORE, base, 0, Vec2::ZERO);
            g.explore = Some(kind);
            goals.push(g);
        }
    }
}

/// RECRUIT goals (`7CFEA0`).
fn recruit_goals(c: &Ctx<'_, '_>, census: &Census, rng: &mut SaiRng, goals: &mut Vec<Goal>) {
    let w = c.view.world;
    let rr = &c.ego.recruiting;
    let king = &w.kingdoms[c.view.me];
    let queue_max = if c.ego.build_queue_max_items > 0 { c.ego.build_queue_max_items as usize } else { usize::MAX };
    // Recruiting buildings with room in their queue, by recruitable def.
    let mut sites: HashMap<DefId, Vec<ActorId>> = HashMap::new();
    for b in c.view.buildings.iter().filter(|b| b.owner == Some(c.view.me) && b.recruits && b.queue < queue_max) {
        if w.books.actor(b.id).is_some_and(|bk| bk.deploy.is_some()) {
            continue;
        }
        for &d in &c.def(b.def).recruits {
            sites.entry(d).or_default().push(b.id);
        }
    }
    if sites.is_empty() {
        return;
    }
    let unit_room = king.unit_limit - king.units_used;
    let fits = |d: DefId| -> bool {
        let need = c.unit_need(d);
        need <= 0.0 || need <= unit_room + 1e-3
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
    let mut candidates: Vec<Goal> = Vec::new();
    let mut per_def: HashMap<DefId, i32> = HashMap::new();
    // hq_type_not_available_bonus: no HQ of the type, or every one full.
    let hq_unavailable = |prop: &str, d: DefId| -> bool {
        let capacity = c.def(d).max_attachments.or_else(|| c.def(d).slots.iter().find_map(|(m, _)| w.data.defs.get(*m as usize).and_then(|m| m.max_attachments)));
        let mut seen = 0;
        for b in c.view.buildings.iter().filter(|b| b.owner == Some(c.view.me) && has(c.props(b.def), prop)) {
            seen += 1;
            let attached = crate::production::attachments(w, b.id) + crate::production::hq_org(w, b.id).and_then(|o| w.books.org(o)).map_or(0, |bk| bk.reserved_attachments);
            let cap = crate::production::hq_org(w, b.id).and_then(|o| c.def(w.orgs[o as usize].def).max_attachments).or(capacity);
            if cap.is_none_or(|cap| attached < cap) {
                return false;
            }
        }
        // A packed or queued HQ of the type will offer its attachments.
        census.prop(prop) <= seen
    };
    for (index, req) in rr.requests.iter().enumerate() {
        let mut best: Option<(f32, DefId)> = None;
        for &d in sites.keys() {
            let id = c.def(d).id.to_ascii_lowercase();
            let matched = match (&req.recruit, &req.property) {
                (Some(r), _) => *r == id,
                (None, Some(p)) => has(c.props(d), p),
                _ => false,
            };
            if !matched || !fits(d) {
                continue;
            }
            // Repeats count the requested type: every owned or queued thing
            // with the property (trucks unpack into other definitions).
            let count = match (&req.recruit, &req.property) {
                (None, Some(p)) => census.prop(p),
                _ => census.def(d),
            };
            let (ap, _) = actor_priority(c, census, d);
            let mut p = req.base_priority + c.actor_value(d) * req.economic_value_multiplier - req.repeat_penalty * count as f32 + ap;
            if count == 0 {
                p += req.one_time_bonus;
            }
            let is_hq = c.def(d).max_attachments.is_some() || c.def(d).slots.iter().any(|(m, _)| w.data.defs.get(*m as usize).is_some_and(|m| m.max_attachments.is_some()));
            if is_hq && let Some(prop) = &req.property && hq_unavailable(prop, d) {
                p += req.hq_type_not_available_bonus;
            }
            if best.is_none_or(|(bp, _)| p > bp) {
                best = Some((p, d));
            }
        }
        if let Some((p, d)) = best {
            for n in 0..req.number_to_request.max(1) {
                let mut g = Goal::new(Key::Recruit(index as u32 * 16 + n as u32, d), data::RECRUIT, p, 0, Vec2::ZERO);
                g.cost = c.quote(d, crate::production::Vector::Cost);
                g.recruit = Some((d, 0));
                candidates.push(g);
            }
        }
    }
    // Military requests: the best company to recruit (core CV, ActorPriority,
    // economic value and income penalties, candidate fuzziness).
    let mut extra: HashMap<DefId, i32> = HashMap::new();
    for n in 0..rr.num_military_goals.max(0) {
        let mut best: Option<(f32, DefId)> = None;
        for &d in sites.keys() {
            let def = c.def(d);
            if def.class != Some(DefClass::Organization) || has(&def.properties, "hq") || c.cv(d) <= 0.0 || has(c.props(d), "economic_truck") || !fits(d) {
                continue;
            }
            let cv = c.cv(d);
            let money = c.quote(d, crate::production::Vector::Cost)[0];
            let mut p = cv * rr.core_cv_multiplier;
            if money > 0.0 {
                p += cv / money * rr.core_cv_ratio_multiplier;
            }
            let (ap, _) = actor_priority(c, census, d);
            p += ap - extra.get(&d).copied().unwrap_or(0) as f32 * repeat_of(c, d);
            p += c.actor_value(d) * rr.economic_value_multiplier;
            let net = c.net(d);
            if net.iter().any(|v| *v < 0.0) {
                p -= rr.decreased_income_penalty;
            }
            if (0..3).any(|r| net[r] < 0.0 && c.view.income[r] + net[r] < rr.insufficient_income_level) {
                p -= rr.insufficient_income_penalty;
            }
            p += rng.unit() * rr.candidate_fuzziness;
            if best.is_none_or(|(bp, _)| p > bp) {
                best = Some((p, d));
            }
        }
        if let Some((p, d)) = best {
            *extra.entry(d).or_default() += 1;
            let mut g = Goal::new(Key::Recruit(u32::MAX - n as u32, d), data::RECRUIT, p, 0, Vec2::ZERO);
            g.cost = c.quote(d, crate::production::Vector::Cost);
            g.recruit = Some((d, 0));
            candidates.push(g);
        }
    }
    // max_goals_per_think best goals; at most max_recruited_per_think of a type.
    candidates.sort_by(|a, b| b.raw.partial_cmp(&a.raw).unwrap_or(std::cmp::Ordering::Equal));
    let limit = if rr.max_goals_per_think < 0 { usize::MAX } else { rr.max_goals_per_think as usize };
    let mut taken = 0;
    for mut g in candidates {
        if taken >= limit {
            break;
        }
        let (d, _) = g.recruit.expect("recruit goal");
        let (_, max) = actor_priority(c, census, d);
        let count = per_def.entry(d).or_default();
        if max > 0 && *count >= max {
            continue;
        }
        let Some(b) = location(d, rng) else { continue };
        *count += 1;
        g.recruit = Some((d, b));
        goals.push(g);
        taken += 1;
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
fn attach_goals(c: &Ctx<'_, '_>, goals: &mut Vec<Goal>) {
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
        let value = c.cv(org.def) * c.ego.attach_cv_multiplier + c.actor_value(org.def);
        let mut g = Goal::new(Key::Attach(o.id), data::ATTACH, value, c.region_of(o.pos), o.pos);
        let factor = w.data.rule_ci("HQAttachCostPercentage").unwrap_or(0.0);
        let cost = c.quote(org.def, crate::production::Vector::Cost);
        g.cost = [cost[0] * factor, cost[1] * factor, cost[2] * factor];
        g.attach = Some((o.id, hq));
        goals.push(g);
    }
}

fn repeat_of(c: &Ctx<'_, '_>, d: DefId) -> f32 {
    let props = c.props(d);
    let id = c.def(d).id.to_ascii_lowercase();
    c.ego
        .actor_priorities
        .iter()
        .filter(|ap| match (&ap.property, &ap.actor) {
            (Some(p), _) => has(props, p),
            (None, Some(a)) => *a == id,
            _ => false,
        })
        .map(|ap| ap.repeat_penalty)
        .sum()
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
fn effective_cv(c: &Ctx<'_, '_>, o: &super::view::OrgView, combat: bool) -> f32 {
    if combat && o.builder { o.cv * c.ego.penalty_to_builder_cv } else { o.cv }
}

struct Assign<'a, 'b, 'w> {
    c: &'a Ctx<'b, 'w>,
    prefs: HashMap<(usize, usize), (f32, Vec<u32>)>,
}

impl Assign<'_, '_, '_> {
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
            .filter(|(j, h)| *j != gi && h.selected && h.kind == g.kind && self.c.regions.hops(h.region, g.region) <= reach)
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
        } else {
            g.base
        };
        let avg = if n > 0.0 { travel / n } else { 0.0 };
        (class + pref_add) * pref_mult(c, g.kind, &counts, &companies) * c.travel_weight(avg)
    }
    fn add(&mut self, goals: &mut [Goal], gi: usize, o: usize) {
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

fn select(c: &Ctx<'_, '_>, player: &Player, _rng: &mut SaiRng, goals: &mut [Goal]) {
    let ego = c.ego;
    let mut available = c.view.stockpile;
    let rate = ego.saving_time_limit / 60.0;
    let pool_ok = |o: &super::view::OrgView| o.owner == c.view.me && o.military && !o.routing && !player.rai.retreating.contains_key(&o.id);
    // Holder of every pooled organization: None = free.
    let mut holder: HashMap<usize, usize> = HashMap::new();
    let mut a = Assign { c, prefs: HashMap::new() };
    for gi in 0..goals.len() {
        if goals[gi].base <= 0.0 {
            continue;
        }
        match goals[gi].kind {
            data::RECRUIT | data::RESEARCH | data::ATTACH => {
                // 7A3860 with the running resources, then the reservation.
                let cost = goals[gi].cost;
                let ok = (0..3).all(|r| cost[r] <= available[r] + c.view.income[r].max(0.0) * rate);
                if ok {
                    for r in 0..3 {
                        available[r] -= cost[r];
                    }
                    goals[gi].selected = true;
                    goals[gi].current = goals[gi].base;
                }
            }
            data::UNPACK => {
                goals[gi].selected = true;
                goals[gi].current = goals[gi].base;
            }
            data::RECOVER => {
                // 7A35D0: the required actor is claimed when free, or taken
                // from an owner valuing it below this base priority.
                let Some(o) = goals[gi].required else { continue };
                if let Some(&h) = holder.get(&o) {
                    let loss = a.priority(goals, h, None, None) - a.priority(goals, h, None, Some(o));
                    if loss >= goals[gi].base {
                        continue;
                    }
                    a.remove(goals, h, o);
                    goals[h].current = a.priority(goals, h, None, None);
                }
                a.add(goals, gi, o);
                holder.insert(o, gi);
                goals[gi].selected = true;
                goals[gi].current = goals[gi].base;
            }
            _ => {
                let single = goals[gi].kind == data::EXPLORE;
                // Previous holders first (continuity), then the greedy gather
                // (7A7EB0) and the take-from-higher trade (7A3F30).
                let key = goals[gi].key;
                let previous: Vec<usize> =
                    (0..c.view.orgs.len()).filter(|&o| pool_ok(&c.view.orgs[o]) && !holder.contains_key(&o) && player.memory.assigned.get(&c.view.orgs[o].id).is_some_and(|x| x.key == key)).collect();
                for o in previous {
                    if single && !goals[gi].actors.is_empty() {
                        break;
                    }
                    let gain = a.priority(goals, gi, Some(o), None) - a.priority(goals, gi, None, None);
                    if gain > 0.0 {
                        a.add(goals, gi, o);
                        holder.insert(o, gi);
                    }
                }
                loop {
                    if single && !goals[gi].actors.is_empty() {
                        break;
                    }
                    let now = a.priority(goals, gi, None, None);
                    let mut best: Option<(f32, usize)> = None;
                    for o in 0..c.view.orgs.len() {
                        if !pool_ok(&c.view.orgs[o]) || goals[gi].actors.contains(&o) {
                            continue;
                        }
                        let mut gain = a.priority(goals, gi, Some(o), None) - now;
                        if single && player.memory.explore_actor.get(&key) == Some(&c.view.orgs[o].id) {
                            gain += ego.exploration[goals[gi].explore.unwrap_or(1)].unit_inertia;
                        }
                        if gain <= 0.0 {
                            continue;
                        }
                        if let Some(&h) = holder.get(&o) {
                            if goals[h].kind == data::RECOVER || h == gi {
                                continue;
                            }
                            let loss = a.priority(goals, h, None, None) - a.priority(goals, h, None, Some(o));
                            gain -= loss;
                        }
                        if gain > 0.0 && best.is_none_or(|(bg, _)| gain > bg) {
                            best = Some((gain, o));
                        }
                    }
                    let Some((_, o)) = best else { break };
                    if let Some(&h) = holder.get(&o) {
                        a.remove(goals, h, o);
                        goals[h].current = a.priority(goals, h, None, None);
                        goals[h].selected = goals[h].current > 0.0 && !goals[h].actors.is_empty();
                    }
                    a.add(goals, gi, o);
                    holder.insert(o, gi);
                }
                goals[gi].current = a.priority(goals, gi, None, None);
                goals[gi].selected = goals[gi].current > 0.0 && !goals[gi].actors.is_empty();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Execution

fn ordered_to(order: &Order, to: Vec2, tolerance: f32) -> bool {
    match order {
        Order::Move { to: t, .. } => t.distance(to) <= tolerance,
        _ => false,
    }
}

fn execute(c: &Ctx<'_, '_>, data: &AiData, player: &mut Player, _rng: &mut SaiRng, goals: &[Goal], log: &mut Vec<String>) {
    let w = c.view.world;
    let me = c.view.kingdom;
    let mut commands = Vec::new();
    let mut stock = c.view.stockpile;
    let mut used_buildings: Vec<ActorId> = Vec::new();
    let mut assigned = BTreeMap::new();
    let tol = c.regions.size * 0.5;
    for g in goals.iter().filter(|g| g.selected) {
        match g.kind {
            data::RECRUIT => {
                let Some((d, b)) = g.recruit else { continue };
                // 7CDA10 sends the recruit order of every selected goal (one
                // per recruiter up to build_queue_max_items); production
                // refuses what the stockpile cannot pay now, so an unpaid
                // goal neither queues nor holds back the goals below it.
                if used_buildings.contains(&b) || (0..3).any(|r| g.cost[r] > stock[r] + 1e-3) {
                    continue;
                }
                for (s, cost) in stock.iter_mut().zip(g.cost) {
                    *s -= cost;
                }
                used_buildings.push(b);
                commands.push(Command::Recruit { kingdom: me, building: b, def: d });
                player.memory.recruited.push((c.now, d));
                log.push(format!("recruit {} at {} (priority {:.0})", c.def(d).id, c.def(w.actors[b as usize].def).id, g.base));
            }
            data::RESEARCH => {
                let Some((b, tech)) = g.research.clone() else { continue };
                if used_buildings.contains(&b) || (0..3).any(|r| g.cost[r] > stock[r] + 1e-3) {
                    continue;
                }
                for (s, cost) in stock.iter_mut().zip(g.cost) {
                    *s -= cost;
                }
                used_buildings.push(b);
                log.push(format!("research {tech} at {} (priority {:.0})", c.def(w.actors[b as usize].def).id, g.base));
                commands.push(Command::Research { kingdom: me, building: b, property: tech });
            }
            data::ATTACH => {
                let Some((org, hq)) = g.attach else { continue };
                if (0..3).any(|r| g.cost[r] > stock[r] + 1e-3) {
                    continue;
                }
                for (s, cost) in stock.iter_mut().zip(g.cost) {
                    *s -= cost;
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
                for &o in &g.actors {
                    let ov = &c.view.orgs[o];
                    assigned.insert(ov.id, Assignment { key: g.key, target: g.target, target_actor: None, region: g.region, since: c.now, combat: false });
                    if !ordered_to(&ov.order, g.target, tol) && ov.pos.distance(g.target) > tol {
                        commands.push(Command::Move { kingdom: me, units: vec![Unit::Org(ov.id)], to: g.target, attack_move: false });
                        log.push(format!("recover {} (health {:.2}, morale {:.2})", c.def(ov.def).id, ov.health, ov.morale));
                    }
                }
            }
            data::EXPLORE => {
                let Some(&o) = g.actors.first() else { continue };
                let ov = &c.view.orgs[o];
                let region = explore_region(c, player, g, ov);
                let target = c.regions.centers[region as usize];
                player.memory.explore_region.insert(g.key, region);
                player.memory.explore_actor.insert(g.key, ov.id);
                assigned.insert(ov.id, Assignment { key: g.key, target, target_actor: None, region, since: c.now, combat: false });
                if !ordered_to(&ov.order, target, tol) {
                    commands.push(Command::Move { kingdom: me, units: vec![Unit::Org(ov.id)], to: target, attack_move: false });
                    log.push(format!("explore with {} to region {region} ({:.0},{:.0})", c.def(ov.def).id, target.x, target.y));
                }
            }
            _ => {
                let mut sent = 0;
                for &o in &g.actors {
                    let ov = &c.view.orgs[o];
                    // A stable slot per organization keeps re-sent orders identical.
                    let target = goal_point(c, g.target, g.target_actor, ov.id as usize % 8);
                    let previous = player.memory.assigned.get(&ov.id);
                    let since = previous.filter(|p| p.key == g.key).map_or(c.now, |p| p.since);
                    assigned.insert(ov.id, Assignment { key: g.key, target, target_actor: g.target_actor, region: g.region, since, combat: true });
                    if ov.engaged || ordered_to(&ov.order, target, tol) || player.pending(ov.id, c.now) {
                        continue;
                    }
                    if !g.attack {
                        let hops = c.regions.hops(c.region_of(ov.pos), g.region);
                        if hops <= 1 && matches!(ov.order, Order::Idle) {
                            continue;
                        }
                    }
                    // A static target is attacked directly once close; until then
                    // (and for mobile targets) an attack-move to the region. A
                    // city is taken by standing in its zone, never attacked.
                    let near = g.target_actor.is_some() && !city_target(c, g.target_actor) && ov.pos.distance(g.target) <= c.regions.size;
                    if near && let Some(t) = g.target_actor {
                        if !matches!(ov.order, Order::Attack { target } if target == t) {
                            commands.push(Command::Attack { kingdom: me, units: vec![Unit::Org(ov.id)], target: t });
                        }
                    } else {
                        commands.push(Command::Move { kingdom: me, units: vec![Unit::Org(ov.id)], to: target, attack_move: true });
                    }
                    sent += 1;
                }
                if sent > 0 {
                    if g.attack {
                        player.memory.attacks.push((c.now, g.region, sent));
                    }
                    log.push(format!(
                        "{} region {} with {} orgs (cv {:.0}/{:.0}, priority {:.0})",
                        if g.attack { "attack" } else { "defend" },
                        g.region,
                        g.actors.len(),
                        g.sum_cv,
                        g.target_cv,
                        g.current
                    ));
                }
            }
        }
    }
    player.memory.assigned = assigned;
    let profile = &data.profiles[player.profile];
    enqueue(player, data, c.now, c.ego.reaction_time, profile.max_ge_commands, commands);
}

/// Explore region choice: the current region, regions with unknown hostile
/// buildings (intuition), random island regions; fog, kingdom-centre and
/// actor distances, time since scouted, fuzziness (ExplorationTypeData).
fn explore_region(c: &Ctx<'_, '_>, player: &Player, g: &Goal, ov: &super::view::OrgView) -> u32 {
    let e = &c.ego.exploration[g.explore.unwrap_or(1)];
    let home = c.region_of(ov.pos);
    let mut seed = (c.now.to_bits() as u64) ^ (ov.id as u64 * 0x9e37_79b9);
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed >> 11) as f32 / (1u64 << 53) as f32
    };
    let mut candidates: Vec<(u32, f32)> = Vec::new();
    if let Some(&r) = player.memory.explore_region.get(&g.key) {
        candidates.push((r, e.current_region_bonus));
    }
    candidates.push((home, e.actor_region_bonus));
    let mut intuition: BTreeMap<u32, u32> = BTreeMap::new();
    for b in &c.view.buildings {
        if b.owner == Some(c.view.me) || player.known.contains(&b.id) {
            continue;
        }
        let counted = if b.city { e.intuition_count_settlements } else { e.intuition_count_other_buildings };
        if counted && attack_target(c, b) {
            *intuition.entry(c.region_of(b.pos)).or_default() += 1;
        }
    }
    for (r, n) in intuition {
        if next() <= e.intuition_probability && (e.intuition_distance < 0 || c.regions.hops(home, r) as i32 <= e.intuition_distance) {
            let bonus = if e.intuition_region_exponent <= 0.0 { e.intuition_region_bonus } else { e.intuition_region_bonus * (n as f32).powf(e.intuition_region_exponent) };
            candidates.push((r, bonus));
        }
    }
    let land: Vec<u32> = (0..c.regions.len() as u32).filter(|&r| c.regions.same_island(r, home)).collect();
    for _ in 0..e.num_random_candidates.max(0) {
        if land.is_empty() {
            break;
        }
        let r = land[((next() * land.len() as f32) as usize).min(land.len() - 1)];
        candidates.push((r, e.random_candidate_bonus));
    }
    let mine: Vec<Vec2> = c.view.buildings.iter().filter(|b| b.owner == Some(c.view.me)).map(|b| b.pos).collect();
    let center = if mine.is_empty() { ov.pos } else { mine.iter().copied().sum::<Vec2>() / mine.len() as f32 };
    let factor = |d: f32, cap: f32, max: f32, exp: f32, ceil: f32, floor: f32, close: bool| -> Option<f32> {
        if cap >= 0.0 && d > cap {
            return None;
        }
        let d = d.min(max.max(1e-3));
        let mut m = d / max.max(1e-3);
        if close {
            m = 1.0 - m;
        }
        Some(m.max(0.0).powf(exp) * (ceil - floor) + floor)
    };
    let mut best = (home, f32::MIN);
    for (r, bonus) in candidates {
        if !c.regions.same_island(r, home) {
            continue;
        }
        let p = c.regions.centers[r as usize];
        let seen = player.scouted.get(r as usize).copied().unwrap_or(f32::NEG_INFINITY);
        let fog = if seen.is_finite() { e.grey_fog_priority } else { e.black_fog_priority };
        let mut v = fog + bonus;
        if e.consider_distance_to_kingdom_center {
            match factor(p.distance(center), e.kingdom_center_distance_cap, e.kingdom_center_distance_max, e.kingdom_center_distance_exp, e.kingdom_center_distance_ceil, e.kingdom_center_distance_floor, e.kingdom_center_distance_prefer_close) {
                Some(m) => v *= m,
                None => continue,
            }
        }
        if e.consider_distance_to_actor {
            match factor(p.distance(ov.pos), e.actor_distance_cap, e.actor_distance_max, e.actor_distance_exp, e.actor_distance_ceil, e.actor_distance_floor, e.actor_distance_prefer_close) {
                Some(m) => v *= m,
                None => continue,
            }
        }
        if e.consider_distance_to_last_region && let Some(&last) = player.memory.explore_region.get(&g.key) {
            let lp = c.regions.centers[last as usize];
            match factor(p.distance(lp), e.last_region_distance_cap, e.last_region_distance_max, e.last_region_distance_exp, e.last_region_distance_ceil, e.last_region_distance_floor, e.last_region_distance_prefer_close) {
                Some(m) => v *= m,
                None => continue,
            }
        }
        if e.use_time_since_last_scouted && e.time_since_last_scouted_max > 0.0 {
            let t = if seen.is_finite() { (c.now - seen).max(0.0) } else { e.time_since_last_scouted_max };
            v *= (t / e.time_since_last_scouted_max).min(1.0).powf(e.time_since_last_scouted_exp);
        }
        v += next() * e.fuzzy_factor;
        if v > best.1 {
            best = (r, v);
        }
    }
    best.0
}
