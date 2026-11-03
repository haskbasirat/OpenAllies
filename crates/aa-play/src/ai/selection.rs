//! The goal selection of one think through the original's own passes, the exact ports
//! in `aa_mind::sai`: `7A93B0` ([`engine::select_goals`]), `7A9150` and `7A7E30`
//! ([`assign::evaluate`], [`assign::assign`]) and the base goal virtuals ([`goal_base`]).
//! What this file adds is the seam: the goals, actors and pools of a think as the native
//! lists, and the class virtuals the ports call.
//!
//! - A goal holds its actors from think to think while it stays selected (`goals::Memory::holding`,
//!   native goal +0C); the others are in the player's pools (+54), one per category of the
//!   actor's mask (`7A1150`, tested by `7A1120`).
//! - A goal is selected (status 2) when its current priority (+34, `7A3B00`) is above zero,
//!   with or without actors: an ATTACK_REGION or DEFEND_REGION goal with none still has the
//!   curve `7ACF90` at the CV floor, which is why the original's goal log shows the inertia
//!   on every one of them from the second think on, and why a BOMBARD or AIR_ATTACK goal sees
//!   its region's attack goal as selected (`7C8750`, `7C7E90`). An EXPLORE goal without an
//!   actor is worth nothing and stays a candidate.
//!
//! Renditions, labelled: the class priority `+7C` is `Bind::class_priority` over the
//! goal's fields (the curve `7ACF90` is `goals::cv_curve`); `+6C` (`7C9E90`, the hostile
//! static on the route that gets an attack flare) is not played; travel time is the straight
//! line over the speed where `7A1C10` asks the route; UNPACK takes no actor from pool 1 here,
//! its truck is its key.
use super::data;
use super::goals::{Assign, Ctx, Goal, Key, airfields, bombard_range, bombards, cv_curve, effective_cv};
use super::rai::StateKind;
use super::Player;
use crate::world::OrgId;
use aa_mind::Result;
use aa_mind::sai::assign::{self, ActorId as Id, ActorList, AssignWorld, CATEGORIES, NO_CATEGORY, Resources, Roster, SaiActor, State};
use aa_mind::sai::engine::{self, BUCKETS, Controller, EngineWorld, GoalId, GoalState};
use aa_mind::sai::goal_base::{self, GoalBaseWorld, TravelWeighting};
use aa_numeric::crt_pow::CrtEnvironment;
use aa_numeric::{Binary, Precision};
use std::collections::BTreeMap;

const P: Precision = Precision::Double53;
/// The actors of pools 13 and 14: the air-attack and air-recon denizens of the airfields
/// (`7A1680`), which aa-play keeps as airfields.
const AIR: u32 = 1 << 20;
const RECON: u32 = 2 << 20;

fn b(v: f32) -> Result<Binary> {
    Binary::from_f32_bits(if v.is_finite() { v } else { 0.0 }.to_bits()).map_err(|e| aa_mind::Error::World(format!("x87 arithmetic: {e}")))
}

/// `fadd [esp+8]`, `fstp [esp+8]`.
fn fstp_sum(a: &Binary, v: &Binary) -> Result<Binary> {
    let world = |e: aa_numeric::Error| aa_mind::Error::World(format!("x87 arithmetic: {e}"));
    let bits = a.add(v, P).map_err(world)?.store_f32().map_err(world)?;
    Binary::from_f32_bits(bits).map_err(world)
}

/// Goal `+8`, the pool a goal takes its actors from.
fn category(g: &Goal) -> u32 {
    match g.kind {
        data::ATTACK_REGION | data::DEFEND_REGION => 2,
        data::EXPLORE if g.explore == Some(2) => 15,
        data::EXPLORE => 1,
        data::REPAIR => 3,
        data::BOMBARD => 12,
        data::AIR_ATTACK => 13,
        _ => NO_CATEGORY,
    }
}

/// Everything the ports ask of a think.
struct Bind<'a, 'b, 'w> {
    c: &'a Ctx<'b, 'w>,
    player: &'a Player,
    goals: &'a mut [Goal],
    roster: Roster,
    /// Bombard range per organization (`7A2310`).
    range: Vec<f32>,
    fields: Vec<crate::world::ActorId>,
    recons: Vec<crate::world::ActorId>,
}

impl Bind<'_, '_, '_> {
    fn org(&self, a: Id) -> Option<usize> {
        (a < AIR).then_some(a as usize)
    }
    fn cv_of(&self, gi: usize, a: Id) -> f32 {
        self.org(a).map_or(0.0, |o| effective_cv(self.c, &self.c.view.orgs[o], self.goals[gi].combat()))
    }
    /// The other goals of a combat goal's +50 list: same type, within linked_region_distance
    /// route hops, with a base above zero (`7C5A20`, `7C5870`).
    fn linked_cv(&self, gi: usize) -> f32 {
        let g = &self.goals[gi];
        let reach = self.c.ego.linked_region_distance.max(0) as u32;
        self.goals.iter().enumerate().filter(|(j, h)| *j != gi && h.kind == g.kind && h.base > 0.0 && self.c.regions.hops(h.region, g.region) <= reach).map(|(_, h)| h.sum_cv).sum()
    }
    fn linked(&self, gi: usize) -> Vec<usize> {
        let g = &self.goals[gi];
        let reach = self.c.ego.linked_region_distance.max(0) as u32;
        self.goals.iter().enumerate().filter(|(j, h)| (*j == gi || (h.kind == g.kind && h.base > 0.0 && self.c.regions.hops(h.region, g.region) <= reach))).map(|(j, _)| j).collect()
    }
    /// Class `+7C` with a hypothetical actor change. Combat goals: `7C9080` -> `7ACF90`
    /// with the linked goals' CV; EXPLORE: its base and the actors' factors, nothing without
    /// an actor; BOMBARD `7C6500`: the base times the actors' CV over the balancing constant;
    /// AIR_ATTACK: the base with a plane. Others, the base.
    fn class_priority(&self, s: &State<'_>, gi: usize, add: Option<Id>, remove: Option<Id>) -> f32 {
        let c = self.c;
        let g = &self.goals[gi];
        if g.base <= 0.0 {
            return 0.0;
        }
        let list = s.roster.goal_list(gi as GoalId).map(ActorList::actors).unwrap_or_default();
        let actors = || list.iter().flatten().copied().filter(|a| Some(*a) != remove).chain(add);
        let delta = add.map_or(0.0, |a| self.cv_of(gi, a)) - remove.map_or(0.0, |a| self.cv_of(gi, a));
        match g.kind {
            data::ATTACK_REGION | data::DEFEND_REGION => cv_curve(c.ego, g.base, g.sum_cv + delta + self.linked_cv(gi) + g.static_cv, g.target_cv, g.max_cv, g.attack, g.allied_cv),
            data::EXPLORE => {
                if actors().next().is_none() {
                    return 0.0;
                }
                let e = &c.ego.exploration[g.explore.unwrap_or(1)];
                let mut v = g.base;
                for o in actors().filter_map(|a| self.org(a)) {
                    let ov = &c.view.orgs[o];
                    v += e.move_rate_mult * ov.speed.max(0.0).powf(e.move_rate_exp) + e.sight_mult * ov.sight.max(0.0).powf(e.sight_exp);
                    let cvt = e.cv_mult * ov.cv.max(0.0).powf(e.cv_exp);
                    v += if e.subtract_cv { -cvt } else { cvt };
                }
                v
            }
            data::BOMBARD => {
                let bc = c.ego.bombard_balancing_constant;
                if bc > 0.0 { g.base * (g.sum_cv + delta) / bc } else { 0.0 }
            }
            data::AIR_ATTACK => {
                if actors().next().is_some() {
                    g.base
                } else {
                    0.0
                }
            }
            // 7A3210 -> 7A31A0, with n actors: base * (max - (max - 1) / 2^(n - 1)).
            data::REPAIR => {
                let n = actors().count() as i32;
                if n <= 0 {
                    return 0.0;
                }
                let m = c.ego.construct_max_multiplier;
                g.base * (m - (m - 1.0) / 2f32.powi(n - 1))
            }
            _ => g.base,
        }
    }
    /// Actor `+20` (`7A25F0`): the holder's commit test, the category, the route, then the
    /// goal type's case.
    fn can_serve(&self, s: &State<'_>, a: Id, goal: GoalId, flag: bool) -> bool {
        let g = &self.goals[goal as usize];
        let Some(o) = self.org(a) else { return true };
        let ov = &self.c.view.orgs[o];
        if ov.routing {
            return false;
        }
        if let Ok(actor) = s.roster.actor(Some(a))
            && let Some(h) = actor.owner
            && h != goal
            && self.committed(a, h as usize)
        {
            return false;
        }
        if g.required == Some(o) {
            return true;
        }
        let cat = category(g);
        if cat == NO_CATEGORY || s.roster.actor(Some(a)).is_ok_and(|x| x.categories & (1 << cat) == 0) {
            return false;
        }
        // 53F320, the region route from the actor to the goal's location.
        if !self.c.regions.same_island(self.c.region_of(ov.pos), g.region) {
            return false;
        }
        match g.kind {
            data::EXPLORE if !flag => s.roster.goal_list(goal).is_ok_and(|l| l.actors().iter().flatten().all(|x| *x == a)),
            data::BOMBARD => self.range[o] > 0.0 && ov.pos.distance(g.target) <= self.range[o],
            _ => true,
        }
    }
    /// A goal's `+40`, does it keep this actor. Combat `7C5290`: selected, the record in the
    /// attack state, within attack_commit_distance hops of the goal's region. RECOVER
    /// `7C5470`: no state or the recover state.
    fn committed(&self, a: Id, h: usize) -> bool {
        let Some(o) = self.org(a) else { return false };
        let (ov, g) = (&self.c.view.orgs[o], &self.goals[h]);
        if g.base <= 0.0 {
            return false;
        }
        let state = self.player.rai.states.get(&ov.id).map(|s| s.kind);
        match g.kind {
            data::ATTACK_REGION | data::DEFEND_REGION => state == Some(StateKind::Attack) && self.c.regions.hops(self.c.region_of(ov.pos), g.region) as i32 <= self.c.ego.attack_commit_distance,
            data::RECOVER => state.is_none() || state == Some(StateKind::Recover),
            _ => false,
        }
    }
    fn affordable(&self, gi: usize, res: Resources<'_>) -> bool {
        let g = &self.goals[gi];
        let rate = self.c.ego.saving_time_limit / 60.0;
        let have: [f32; 3] = match res {
            Resources::Null => return true,
            Resources::Running(r) => [r.first().map_or(0.0, |x| f32::from_bits(*x)), r.get(1).map_or(0.0, |x| f32::from_bits(*x)), r.get(2).map_or(0.0, |x| f32::from_bits(*x))],
            Resources::Economy => self.c.view.stockpile,
        };
        (0..3).all(|r| g.cost[r] <= have[r] + self.c.view.income[r].max(0.0) * rate)
    }
    fn adjust_cv(&mut self, gi: usize, a: Id, sign: f32) {
        let v = self.cv_of(gi, a) * sign;
        self.goals[gi].sum_cv += v;
    }
}

impl GoalBaseWorld for Bind<'_, '_, '_> {
    fn goal_type(&mut self, _: &mut State<'_>, goal: GoalId) -> Result<u32> {
        Ok(self.goals[goal as usize].kind as u32)
    }
    fn category(&mut self, _: &mut State<'_>, goal: GoalId) -> Result<u32> {
        Ok(category(&self.goals[goal as usize]))
    }
    fn location(&mut self, _: &mut State<'_>, goal: GoalId) -> Result<u32> {
        Ok(goal)
    }
    fn value(&mut self, s: &mut State<'_>, goal: GoalId, actor: Option<Id>, check: bool) -> Result<Binary> {
        goal_base::value(self, s, goal, actor, check, P)
    }
    fn held_value(&mut self, s: &mut State<'_>, goal: GoalId, actor: Option<Id>) -> Result<Binary> {
        goal_base::held_value(self, s, goal, actor, P)
    }
    fn count(&mut self, s: &mut State<'_>, goal: GoalId) -> Result<i32> {
        Ok(s.roster.goal_list(goal)?.count)
    }
    fn required_actor(&mut self, _: &mut State<'_>, goal: GoalId) -> Result<Option<Id>> {
        Ok(self.goals[goal as usize].required.map(|o| o as Id))
    }
    fn remove_actor(&mut self, s: &mut State<'_>, goal: GoalId, actor: Option<Id>, resources: Resources<'_>, refresh: bool, keep: bool, _extra: u32) -> Result<()> {
        <Self as AssignWorld>::remove_actor(self, s, goal, actor, resources, refresh, keep, false)
    }
    fn priority_hook(&mut self, s: &mut State<'_>, goal: GoalId, add: Option<Id>, remove: Option<Id>) -> Result<Binary> {
        b(self.class_priority(s, goal as usize, add, remove))
    }
    fn deselect(&mut self, s: &mut State<'_>, goal: GoalId) -> Result<()> {
        goal_base::deselect(self, s, goal)
    }
    fn actor_dead(&mut self, _: &mut State<'_>, _actor: Id) -> Result<bool> {
        Ok(false)
    }
    fn actor_flag4(&mut self, _: &mut State<'_>, _actor: Id) -> Result<bool> {
        Ok(false)
    }
    fn actor_can_serve(&mut self, s: &mut State<'_>, actor: Id, goal: GoalId, flag: bool) -> Result<bool> {
        Ok(self.can_serve(s, actor, goal, flag))
    }
    fn unit_check(&mut self, _: &mut State<'_>, actor: Id) -> Result<bool> {
        Ok(actor >= AIR)
    }
    fn travel_time(&mut self, _: &mut State<'_>, actor: Id, location: u32) -> Result<Binary> {
        let t = self.org(actor).map_or(0.0, |o| {
            let ov = &self.c.view.orgs[o];
            self.c.travel_time(ov.pos, self.goals[location as usize].target, ov.speed)
        });
        b(t)
    }
    fn modifiers(&mut self, _: &mut State<'_>, goal_type: u32) -> Result<Option<Vec<u32>>> {
        let n = self.c.ego.goal(goal_type as usize).preferences.len() as u32;
        Ok((n > 0).then(|| (0..n).map(|i| i | (goal_type << 16)).collect()))
    }
    /// `4A7CD0`: the organization, then its members (`4D7EE0`).
    fn apply_modifier(&mut self, _: &mut State<'_>, modifier: u32, actor: Id, sum: &mut u32, product: &mut u32, matched: &mut bool) -> Result<()> {
        let Some(o) = self.org(actor) else { return Ok(()) };
        let c = self.c;
        let gp = c.ego.goal((modifier >> 16) as usize);
        let Some(pref) = gp.preferences.get((modifier & 0xffff) as usize) else { return Ok(()) };
        let matches = |d: crate::data::DefId| match (&pref.property, &pref.actor) {
            (Some(p), _) => super::view::has(c.props(d), p),
            (None, Some(a)) => c.def(d).id.eq_ignore_ascii_case(a),
            _ => false,
        };
        let (mut s, mut m) = (f32::from_bits(*sum), f32::from_bits(*product));
        let mut counted = false;
        if !pref.count_multiplier_once_per_goal {
            *matched = false;
        }
        let ov = &c.view.orgs[o];
        if matches(ov.def) {
            counted = true;
            s += pref.value;
            if !*matched {
                *matched = true;
                m *= pref.multiplier;
            }
        }
        let once = pref.count_multiplier_once_per_goal || pref.count_multiplier_once_per_company;
        let w = c.view.world;
        for &member in &w.orgs[ov.id as usize].members {
            if counted && pref.count_value_once_per_company && *matched && once {
                break;
            }
            let Some(x) = w.actor(member).filter(|x| x.alive) else { continue };
            if matches(x.def) {
                if !counted || !pref.count_value_once_per_company {
                    counted = true;
                    s += pref.value;
                }
                if !*matched || !once {
                    *matched = true;
                    m *= pref.multiplier;
                }
            }
        }
        *sum = s.to_bits();
        *product = m.to_bits();
        Ok(())
    }
    fn travel_weighting(&mut self, _: &mut State<'_>) -> Result<TravelWeighting> {
        // An ego without the field (test data) has no travel weight, as `Ctx::travel_weight` reads it.
        let normaliser = if self.c.ego.max_travel_time > 0.0 { self.c.ego.max_travel_time } else { f32::MAX };
        Ok(TravelWeighting { normaliser: normaliser.to_bits(), exponent: self.c.ego.travel_time_exponent.to_bits(), crt: CrtEnvironment::SSE2_DOUBLE53 })
    }
    fn affordable(&mut self, _: &mut State<'_>, goal: GoalId, resources: Resources<'_>, _relaxed: bool) -> Result<bool> {
        Ok(Bind::affordable(self, goal as usize, resources))
    }
    fn finalize(&mut self, _: &mut State<'_>, _goal: GoalId, _resources: Resources<'_>, _draw: bool) -> Result<()> {
        Ok(())
    }
}

impl AssignWorld for Bind<'_, '_, '_> {
    fn category(&mut self, _: &mut State<'_>, goal: GoalId) -> Result<u32> {
        Ok(category(&self.goals[goal as usize]))
    }
    /// Goal `+C`. Combat `7C5220`: +3C at +40. EXPLORE, BOMBARD, AIR_ATTACK (`7A7A80`,
    /// `7C54D0`): an actor. Without a pool it does not matter.
    fn satisfied(&mut self, s: &mut State<'_>, goal: GoalId) -> Result<bool> {
        let gi = goal as usize;
        Ok(if self.goals[gi].combat() { self.goals[gi].sum_cv + self.linked_cv(gi) >= self.goals[gi].max_cv } else { s.roster.goal_list(goal)?.count > 0 })
    }
    fn expired(&mut self, _: &mut State<'_>, _goal: GoalId) -> Result<bool> {
        Ok(false)
    }
    fn swap_value(&mut self, s: &mut State<'_>, goal: GoalId, gained: Option<Id>, lost: Option<Id>) -> Result<Binary> {
        goal_base::swap_value(self, s, goal, gained, lost, true, P)
    }
    /// Goal `+28`. Combat `7C57F0`: the can-serve test, then `7A5020` over the +50 list.
    fn value(&mut self, s: &mut State<'_>, goal: GoalId, actor: Option<Id>) -> Result<Binary> {
        if !self.goals[goal as usize].combat() {
            return goal_base::value(self, s, goal, actor, true, P);
        }
        let Some(id) = actor else { return Ok(Binary::from_i32(0)) };
        if !self.can_serve(s, id, goal, false) {
            return Ok(Binary::from_i32(0));
        }
        let mut total = Binary::from_i32(0);
        for j in self.linked(goal as usize) {
            let v = goal_base::value(self, s, j as GoalId, actor, false, P)?;
            total = fstp_sum(&total, &v)?;
        }
        Ok(total)
    }
    /// Goal `+2C`. Combat `7C5870`: `7A5080` over the +50 list.
    fn held_value(&mut self, s: &mut State<'_>, goal: GoalId, actor: Option<Id>) -> Result<Binary> {
        if !self.goals[goal as usize].combat() {
            return goal_base::held_value(self, s, goal, actor, P);
        }
        let mut total = Binary::from_i32(0);
        for j in self.linked(goal as usize) {
            let v = goal_base::held_value(self, s, j as GoalId, actor, P)?;
            total = fstp_sum(&total, &v)?;
        }
        Ok(total)
    }
    fn spare(&mut self, s: &mut State<'_>, goal: GoalId) -> Result<i32> {
        Ok(s.roster.goal_list(goal)?.count)
    }
    fn required_actor(&mut self, _: &mut State<'_>, goal: GoalId) -> Result<Option<Id>> {
        Ok(self.goals[goal as usize].required.map(|o| o as Id))
    }
    fn critical(&mut self, _: &mut State<'_>, _goal: GoalId) -> Result<bool> {
        Ok(false)
    }
    /// Goal `+64`. Combat `7C59F0`: the actor's CV (`4A6660`) on +3C, then `7A6930`.
    fn add_actor(&mut self, s: &mut State<'_>, goal: GoalId, actor: Option<Id>, resources: Resources<'_>, a: bool, keep: bool) -> Result<()> {
        if let Some(id) = actor {
            self.adjust_cv(goal as usize, id, 1.0);
        }
        goal_base::add_actor(self, s, goal, actor, resources, a, keep, P)
    }
    /// Goal `+68`. Combat `7C5A60`, as `+64` with the CV off.
    fn remove_actor(&mut self, s: &mut State<'_>, goal: GoalId, actor: Option<Id>, resources: Resources<'_>, a: bool, keep: bool, _c: bool) -> Result<()> {
        if let Some(id) = actor {
            self.adjust_cv(goal as usize, id, -1.0);
        }
        goal_base::remove_actor(self, s, goal, actor, resources, a, keep, P)
    }
    fn excluded(&mut self, _: &mut State<'_>, _goal: GoalId, _actor: Option<Id>) -> Result<bool> {
        Ok(false)
    }
    fn deselect(&mut self, s: &mut State<'_>, goal: GoalId) -> Result<()> {
        goal_base::deselect(self, s, goal)
    }
    fn actor_dead(&mut self, _: &mut State<'_>, _actor: Id) -> Result<bool> {
        Ok(false)
    }
    fn actor_can_serve(&mut self, s: &mut State<'_>, actor: Id, goal: GoalId) -> Result<bool> {
        Ok(self.can_serve(s, actor, goal, true))
    }
    fn affordable(&mut self, _: &mut State<'_>, goal: GoalId, running: &[u32], _relaxed: bool) -> Result<bool> {
        Ok(Bind::affordable(self, goal as usize, Resources::Running(running)))
    }
    fn priority_with_actors(&mut self, s: &mut State<'_>, goal: GoalId) -> Result<Binary> {
        goal_base::priority_with_actors(self, s, goal, None, None, P)
    }
    fn yield_point(&mut self, _: &mut State<'_>) -> Result<()> {
        Ok(())
    }
}

impl EngineWorld for Bind<'_, '_, '_> {
    fn goal_type(&mut self, goal: GoalId, _: &mut Controller) -> Result<u32> {
        Ok(self.goals[goal as usize].kind as u32)
    }
    fn expired(&mut self, _goal: GoalId, _: &mut Controller) -> Result<bool> {
        Ok(false)
    }
    fn destroy(&mut self, _goal: GoalId, _: &mut Controller) -> Result<()> {
        Ok(())
    }
    fn reevaluate(&mut self, _goal: GoalId, _: &mut Controller) -> Result<()> {
        Ok(())
    }
    fn deselect(&mut self, goal: GoalId, ctl: &mut Controller) -> Result<()> {
        let mut roster = std::mem::take(&mut self.roster);
        let r = goal_base::deselect(self, &mut State { ctl, roster: &mut roster }, goal);
        self.roster = roster;
        r
    }
    fn evaluate(&mut self, goal: GoalId, running: &mut [u32], order: &[GoalId], index: usize, ctl: &mut Controller) -> Result<()> {
        let mut roster = std::mem::take(&mut self.roster);
        let r = assign::evaluate(self, &mut State { ctl, roster: &mut roster }, goal, running, order, index);
        self.roster = roster;
        r
    }
    fn assign(&mut self, goal: GoalId, running: &mut [u32], order: &[GoalId], index: usize, ctl: &mut Controller) -> Result<()> {
        let mut roster = std::mem::take(&mut self.roster);
        let r = assign::assign(self, &mut State { ctl, roster: &mut roster }, goal, running, order, index, P);
        self.roster = roster;
        r
    }
    fn yield_point(&mut self, _: &mut Controller) -> Result<()> {
        Ok(())
    }
}

/// The category mask of an actor (`7A1150`, the bits this game's goals ask for): bit 1 what
/// can walk and is not an HQ's, bit 2 with a combat value, bit 12 with a bombard ability, bits
/// 13 and 15 the denizens.
fn mask(c: &Ctx<'_, '_>, o: usize) -> u64 {
    let ov = &c.view.orgs[o];
    let mut m = 0u64;
    if ov.owner == c.view.me && ov.military {
        m |= 1 << 1;
        if ov.cv > 0.0 {
            m |= 1 << 2;
        }
        if bombards(c, ov) {
            m |= 1 << 12;
        }
        let w = c.view.world;
        if w.orgs[ov.id as usize].members.iter().any(|&a| w.actor(a).is_some_and(|x| x.alive) && crate::combat::tasks::heals_something(w, a)) {
            m |= 1 << 3;
        }
    }
    m
}

/// `7A93B0` over the goals of a think: fills `selected`, `current` and `actors` (and a
/// plane's `airfield`, and `sum_*` for the execution's worth), and returns the actors held.
pub(super) fn select(c: &Ctx<'_, '_>, player: &Player, goals: &mut [Goal], air_orders: bool, log: &mut Vec<String>) -> BTreeMap<Key, Vec<OrgId>> {
    let orgs = &c.view.orgs;
    let fields = if air_orders { airfields(c, false) } else { Vec::new() };
    let recons = if air_orders { airfields(c, true) } else { Vec::new() };
    let mut ctl = Controller { buckets: vec![engine::Bucket::default(); BUCKETS], goals: BTreeMap::new() };
    let mut roster = Roster { pools: Vec::with_capacity(CATEGORIES), ..Roster::default() };
    let index: BTreeMap<OrgId, usize> = orgs.iter().enumerate().map(|(i, o)| (o.id, i)).collect();
    let mut owner: BTreeMap<usize, usize> = BTreeMap::new();
    let mut held: Vec<(usize, Vec<usize>)> = Vec::new();
    for (gi, g) in goals.iter_mut().enumerate() {
        g.actors.clear();
        g.sum_cv = 0.0;
        g.sum_travel = 0.0;
        g.pref_add = 0.0;
        g.pref_counts.clear();
        g.current = 0.0;
        g.selected = false;
        let id = gi as GoalId;
        let last = g.kind != data::RECRUIT && player.memory.selected.contains_key(&g.key);
        let status = if g.base <= 0.0 {
            0
        } else if last {
            2
        } else {
            1
        };
        ctl.goals.insert(id, GoalState { status, costs: g.cost.iter().map(|x| x.to_bits()).collect(), current: 0, base: g.base.to_bits(), travel: 0 });
        roster.goal_actors.insert(id, ActorList::default());
        if g.base > 0.0 {
            ctl.buckets[g.kind.min(BUCKETS - 1)].lists[status as usize].push_back(Some(id));
        }
        if last && g.base > 0.0 {
            let mine: Vec<usize> = player.memory.holding.get(&g.key).map_or(Vec::new(), |l| l.iter().filter_map(|o| index.get(o).copied()).collect());
            let mut keep = Vec::new();
            for o in mine {
                if !owner.contains_key(&o) && orgs[o].owner == c.view.me {
                    owner.insert(o, gi);
                    keep.push(o);
                }
            }
            held.push((gi, keep));
        }
    }
    // The pools: the actors no goal holds.
    let mut pools: Vec<Vec<Option<Id>>> = vec![Vec::new(); CATEGORIES];
    for o in 0..orgs.len() {
        let m = mask(c, o);
        if m == 0 {
            continue;
        }
        roster.actors.insert(o as Id, SaiActor { owner: None, categories: m, travel_cache: (-1.0f32).to_bits(), route: None });
        if !owner.contains_key(&o) {
            for (cat, pool) in pools.iter_mut().enumerate() {
                if m & (1 << cat) != 0 {
                    pool.push(Some(o as Id));
                }
            }
        }
    }
    for i in 0..fields.len() {
        let a = AIR + i as Id;
        roster.actors.insert(a, SaiActor { owner: None, categories: 1 << 13, travel_cache: (-1.0f32).to_bits(), route: None });
        pools[13].push(Some(a));
    }
    for i in 0..recons.len() {
        let a = RECON + i as Id;
        roster.actors.insert(a, SaiActor { owner: None, categories: 1 << 15, travel_cache: (-1.0f32).to_bits(), route: None });
        pools[15].push(Some(a));
    }
    roster.pools = pools.into_iter().map(|l| { let n = l.len() as i32; Some(ActorList::from_actors(l, n)) }).collect();
    let range: Vec<f32> = orgs.iter().map(|o| if bombards(c, o) { bombard_range(c, o) } else { 0.0 }).collect();
    let economy: Vec<u32> = c.view.stockpile.iter().map(|x| x.to_bits()).collect();
    let mut bind = Bind { c, player, goals, roster, range, fields, recons };
    let outcome = (|| -> Result<()> {
        let mut roster = std::mem::take(&mut bind.roster);
        {
            let mut s = State { ctl: &mut ctl, roster: &mut roster };
            // 7A7700: the goals keep what they held; 7A67B0 has refreshed +34.
            for (gi, keep) in &held {
                for &o in keep.iter().rev() {
                    <Bind as AssignWorld>::add_actor(&mut bind, &mut s, *gi as GoalId, Some(o as Id), Resources::Null, false, true)?;
                }
            }
            for gi in 0..bind.goals.len() {
                if bind.goals[gi].base > 0.0 {
                    let id = gi as GoalId;
                    let v = if Bind::affordable(&bind, gi, Resources::Economy) { goal_base::priority_with_actors(&mut bind, &mut s, id, None, None, P)?.store_f32().map_err(|e| aa_mind::Error::World(format!("{e}")))? } else { 0 };
                    s.ctl.goal_mut(id)?.current = v;
                }
            }
        }
        bind.roster = roster;
        engine::select_goals(&mut ctl, &economy, &mut bind, P)
    })();
    if let Err(e) = outcome {
        log.push(format!("goal selection: {e}"));
    }
    let Bind { goals, roster, fields, recons, .. } = bind;
    let mut holding = BTreeMap::new();
    for (gi, g) in goals.iter_mut().enumerate() {
        let id = gi as GoalId;
        let Ok(state) = ctl.goal(id) else { continue };
        g.current = f32::from_bits(state.current);
        g.selected = state.status == 2 && g.base > 0.0;
        if !g.selected {
            g.sum_cv = 0.0;
            continue;
        }
        let list = roster.goal_list(id).map(ActorList::actors).unwrap_or_default();
        let mut kept = Vec::new();
        for a in list.into_iter().flatten() {
            if a >= RECON {
                g.airfield = recons.get((a - RECON) as usize).copied();
            } else if a >= AIR {
                g.airfield = fields.get((a - AIR) as usize).copied();
            } else {
                kept.push(a as usize);
            }
        }
        holding.insert(g.key, kept.iter().map(|&o| orgs[o].id).collect());
        g.actors = kept;
    }
    // What the execution reads of a goal's actors (its worth to each, [+2C]).
    let mut a = Assign::new(c);
    for gi in 0..goals.len() {
        let list = std::mem::take(&mut goals[gi].actors);
        goals[gi].sum_cv = 0.0;
        for o in list {
            a.add(goals, gi, o);
        }
    }
    holding
}
