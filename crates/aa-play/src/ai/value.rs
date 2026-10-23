//! The ego valuations of the original SAI, f32 renditions of:
//! - `4A77A0` need value of a change of one resource's income (the ego
//!   ResourceNeed record, else stockpilable for purchase resources and
//!   "other" for upkeep/limited ones: short below 0, enough up to
//!   plenty_value, plenty above it);
//! - `4AAA30` economy value of an economy vector against a kingdom's income
//!   (limited providers capped at EconomyLimitedResourceMax, upkeep
//!   shortfalls valued at their PurchaseCost in money);
//! - `4AC240` actor/definition value (the larger of a truck's and its
//!   unpacked building's economy, the military value `4A7440`, the distance
//!   multiplier, `scaling_mult`, organization slots added);
//! - `4A9DB0` ActorPriority value (the actor-IDS entry, then the largest
//!   matching property entry);
//! - `7CE5E0` recruit value (CV times core/support multiplier, economy times
//!   Recruiting economic_value_multiplier, ActorPriority, the money income
//!   penalties, the builder term `7CE3B0`, candidate fuzziness);
//! - `7CC640` SpecificRecruitRequest priority.
use super::goals::Ctx;
use super::view::has;
use crate::data::{DefClass, DefId, ResourceKind};
use std::collections::HashMap;

/// Counts of the player's own things (live and queued) by definition and
/// property: the SAI player's `7AF140`/`7AF180` lookups.
#[derive(Default)]
pub(crate) struct Census {
    pub by_def: HashMap<DefId, i32>,
    pub by_prop: HashMap<String, i32>,
}

impl Census {
    fn add(&mut self, d: DefId, props: &[String]) {
        *self.by_def.entry(d).or_default() += 1;
        for p in props {
            *self.by_prop.entry(p.clone()).or_default() += 1;
        }
    }
    pub fn build(c: &Ctx<'_, '_>) -> Self {
        let mut s = Census::default();
        let w = c.view.world;
        for o in c.view.orgs.iter().filter(|o| o.owner == c.view.me) {
            s.add(o.def, c.props(o.def));
        }
        for b in c.view.buildings.iter().filter(|b| b.owner == Some(c.view.me)) {
            if w.actors[b.id as usize].org.is_none() {
                s.add(b.def, c.props(b.def));
            }
            let queued = w.books.actor(b.id).map(|bk| bk.queue.items.iter().filter(|i| i.research.is_none()).map(|i| i.def).collect::<Vec<_>>()).unwrap_or_default();
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
    pub fn def(&self, d: DefId) -> i32 {
        self.by_def.get(&d).copied().unwrap_or(0)
    }
    pub fn prop(&self, p: &str) -> i32 {
        self.by_prop.get(p).copied().unwrap_or(0)
    }
}

/// The resource registry as the valuations read it.
pub(crate) struct Econ {
    pub names: Vec<String>,
    /// Native resource +18: 0 purchase (stockpiled), 1 upkeep, 2 limited.
    pub kinds: Vec<u8>,
    /// Resource +2C: provider capped by EconomyLimitedResourceMax.
    pub limited_provider: Vec<bool>,
    /// Resource +1C: PurchaseCost per resource.
    pub purchase: Vec<Vec<f32>>,
    pub limited_max: f32,
    /// Synthetic data without a registry: money/ammo/oil only.
    pub synthetic: bool,
}

impl Econ {
    pub fn new(c: &Ctx<'_, '_>) -> Self {
        let data = &c.view.world.data;
        if data.resources.is_empty() {
            return Econ {
                names: vec!["money".into(), "ammo".into(), "oil".into()],
                kinds: vec![0, 1, 1],
                limited_provider: vec![false; 3],
                purchase: vec![vec![0.0; 3]; 3],
                limited_max: f32::MAX,
                synthetic: true,
            };
        }
        Econ {
            names: data.resources.iter().map(|r| r.id.to_ascii_lowercase()).collect(),
            kinds: data
                .resources
                .iter()
                .map(|r| match r.kind {
                    ResourceKind::Stockpiled => 0,
                    ResourceKind::Upkeep => 1,
                    ResourceKind::LimitedConsumed => 2,
                })
                .collect(),
            limited_provider: data.resources.iter().map(|r| r.limited_provider).collect(),
            purchase: data.resources.iter().map(|r| r.purchase_cost.clone()).collect(),
            limited_max: data.rule_ci("EconomyLimitedResourceMax").unwrap_or(f32::MAX),
            synthetic: false,
        }
    }
    pub fn len(&self) -> usize {
        self.names.len()
    }
}

/// The valuation context of one think.
pub(crate) struct Val<'a, 'b, 'w> {
    pub c: &'a Ctx<'b, 'w>,
    pub census: &'a Census,
    pub econ: Econ,
    /// The player's per-minute income per resource (provided - consumed).
    pub income: Vec<f32>,
    /// Kingdom centre (average of the player's buildings) and the map
    /// diagonal, for the `4AC240` distance multiplier.
    pub center: Option<glam::Vec2>,
    pub diagonal: f32,
}

impl<'a, 'b, 'w> Val<'a, 'b, 'w> {
    pub fn new(c: &'a Ctx<'b, 'w>, census: &'a Census) -> Self {
        let econ = Econ::new(c);
        let income = income_of(c, &econ, c.view.kingdom);
        let mine: Vec<glam::Vec2> = c.view.buildings.iter().filter(|b| b.owner == Some(c.view.me)).map(|b| b.pos).collect();
        let center = (!mine.is_empty()).then(|| mine.iter().copied().sum::<glam::Vec2>() / mine.len() as f32);
        let m = &c.view.world.map;
        let diagonal = (m.width as f32 * m.cell_size).hypot(m.height as f32 * m.cell_size);
        Val { c, census, econ, income, center, diagonal }
    }

    /// Production minus upkeep of a definition (`45F700`, no actor). The
    /// LimitedConsumed resources (unit/building limit consumed) carry no
    /// entry: the goal CSV of the original fits only with the building
    /// limit a truck or depot takes contributing nothing (Oil Depot Truck
    /// 304.5 / 431.7, Artillery Brigade Truck 895.0, Infantry Division HQ
    /// 1070.7).
    pub fn vector(&self, d: DefId) -> Vec<f32> {
        let def = self.c.def(d);
        if self.econ.synthetic {
            return (0..3).map(|r| def.production[r] - def.upkeep[r]).collect();
        }
        if !def.has_economy {
            return vec![0.0; self.econ.len()];
        }
        (0..self.econ.len())
            .map(|i| if self.econ.kinds[i] == 2 { 0.0 } else { def.production_all.get(i).copied().unwrap_or(0.0) - def.upkeep_all.get(i).copied().unwrap_or(0.0) })
            .collect()
    }

    fn need_record(&self, i: usize) -> (f32, f32, f32, f32) {
        let e = self.c.ego;
        if let Some(n) = e.resource_needs.iter().find(|n| n.resource == self.econ.names[i]).filter(|n| n.short_need >= 0.0) {
            return (n.short_need, n.enough_need, n.plenty_value, n.plenty_need);
        }
        if self.econ.kinds[i] == 0 {
            (e.stockpilable_short_need, e.stockpilable_enough_need, e.stockpilable_plenty_value, e.stockpilable_plenty_need)
        } else {
            (e.other_short_need, e.other_enough_need, e.other_plenty_value, e.other_plenty_need)
        }
    }

    /// `4A77A0`: the need value of changing resource `i`'s income by `delta`.
    pub fn need_value(&self, i: usize, income: f32, delta: f32) -> f32 {
        let (short, enough, pv, plenty) = self.need_record(i);
        let mut v = 0.0;
        if delta > 0.0 {
            let (mut d, mut inc) = (delta, income);
            if inc < 0.0 {
                let part = d.min(-inc);
                v += part * short;
                d -= part;
                inc = 0.0;
            }
            if pv > 0.0 {
                if d <= 0.0 {
                    return v;
                }
                if inc >= pv {
                    return v + d * plenty;
                }
            }
            let part = d.min(pv - inc);
            v += part * enough;
            d -= part;
            if d <= 0.0 {
                return v;
            }
            v + d * plenty
        } else {
            let (mut amount, mut inc) = (-delta, income);
            if pv > 0.0 && pv < inc {
                let part = amount.min(inc - pv);
                v -= part * plenty;
                amount -= part;
                inc = pv;
            }
            if amount <= 0.0 {
                return v;
            }
            if inc > 0.0 {
                let part = amount.min(inc);
                v -= part * enough;
                amount -= part;
            }
            if amount <= 0.0 {
                return v;
            }
            v - amount * short
        }
    }

    /// `4AAA30`: economy value of `delta` against `income` (the kingdom's
    /// income without the valued actor).
    pub fn econ_value(&self, delta: &[f32], income: &[f32]) -> f32 {
        let mut value = 0.0;
        for i in (0..self.econ.len().min(delta.len())).rev() {
            let inc = income.get(i).copied().unwrap_or(0.0);
            let mut d = delta[i];
            if d == 0.0 {
                continue;
            }
            if self.econ.limited_provider[i] {
                d = d.min(self.econ.limited_max - inc);
                if d == 0.0 {
                    continue;
                }
            }
            value += self.need_value(i, inc, d);
            if self.econ.kinds[i] == 1 {
                // An upkeep shortfall is bought with money (PurchaseCost).
                let amount = if d > 0.0 && inc < 0.0 {
                    d.min(-inc)
                } else if d < 0.0 && inc + d < 0.0 {
                    (inc + d).max(d)
                } else {
                    0.0
                };
                if amount != 0.0 {
                    for j in (0..self.econ.len()).rev() {
                        let pc = self.econ.purchase[i].get(j).copied().unwrap_or(0.0);
                        if self.econ.kinds[j] == 0 && pc != 0.0 {
                            value += self.need_value(j, income.get(j).copied().unwrap_or(0.0), pc * amount);
                        }
                    }
                }
            }
        }
        value
    }

    /// `4AC240` for a definition (no actor, off the map): the economy (the
    /// larger of a truck's and its unpacked building's), plus, when
    /// `military`, `military_value_scaling_mult * military_cv_mult * CV`
    /// (`4A7440`'s definition branch), times `scaling_mult`; an
    /// organization adds its slots.
    pub fn def_value(&self, d: DefId, military: bool) -> f32 {
        let def = self.c.def(d);
        let mut v = self.econ_value(&self.vector(d), &self.income);
        if let Some(u) = def.unpacks_to {
            let unpacked = self.econ_value(&self.vector(u), &self.income);
            if unpacked >= v {
                v = unpacked;
            }
        }
        if military && def.class != Some(DefClass::Organization) {
            v += self.c.ego.military_value_scaling_mult * self.c.ego.military_cv_mult * self.c.cv(d);
        }
        v *= self.c.ego.scaling_mult;
        if def.class == Some(DefClass::Organization) {
            for (m, _) in &def.slots {
                if (*m as usize) < self.c.view.world.data.defs.len() {
                    v += self.def_value(*m, military);
                }
            }
        }
        v
    }

    /// `4AC240` for a standing actor (`7AD2D0` attack value without the
    /// ActorPriority): its economy against its owner's income without it,
    /// the military value (static actors: 0 without enemy influence, the
    /// closest reading of `4A7440`'s radius terms; mobile ones CV-based),
    /// the distance multiplier from the player's kingdom centre.
    pub fn actor_value(&self, d: DefId, owner: Option<crate::world::KingdomId>, cv: f32, mobile: bool, pos: glam::Vec2, distance: bool) -> f32 {
        let income = match owner {
            Some(k) => income_of(self.c, &self.econ, k),
            None => vec![0.0; self.econ.len()],
        };
        let delta = self.vector(d);
        let without: Vec<f32> = income.iter().zip(delta.iter().chain(std::iter::repeat(&0.0))).map(|(i, x)| i - x).collect();
        let mut v = self.econ_value(&delta, &without);
        if mobile {
            v += self.c.ego.military_value_scaling_mult * self.c.ego.military_cv_mult * cv;
        }
        if distance
            && let Some(center) = self.center
            && self.diagonal > 0.0
        {
            let e = self.c.ego;
            let f = 1.0 - center.distance(pos) / self.diagonal;
            v *= (e.distance_mult_max - e.distance_mult_min) * f + e.distance_mult_min;
        }
        v * self.c.ego.scaling_mult
    }

    /// `4A9DB0`: the ActorPriority of a definition: its actor-IDS entry, then
    /// the largest of its matching property entries. `flag` false counts one
    /// fewer of the owned things and gives no one-time bonus. The
    /// definition's own properties count, not its members' (an HQ regiment
    /// is not an `infhq`; its truck is): the CSV's Infantry Division HQ
    /// value holds the 550 of `infhq` once.
    pub fn actor_priority(&self, d: DefId, flag: bool) -> f32 {
        let def = self.c.def(d);
        let id = def.id.to_ascii_lowercase();
        let own: Vec<String> = def.properties.iter().map(|p| p.to_ascii_lowercase()).collect();
        let props = own.as_slice();
        let mut best: Option<f32> = None;
        for ap in self.c.ego.actor_priorities.iter().filter(|ap| ap.property.is_none() && ap.actor.as_deref() == Some(id.as_str())) {
            let count = self.census.def(d);
            let mut v = ap.value - count as f32 * ap.repeat_penalty;
            if count < 1 {
                v += ap.one_time_bonus;
            }
            best = Some(v);
        }
        for ap in self.c.ego.actor_priorities.iter().filter(|ap| ap.property.as_deref().is_some_and(|p| has(props, p))) {
            let mut count = if ap.count_all_property_actors { self.census.prop(ap.property.as_deref().unwrap_or_default()) } else { self.census.def(d) };
            if !flag && count > 0 {
                count -= 1;
            }
            let mut v = ap.value - count as f32 * ap.repeat_penalty;
            if flag && count < 1 {
                v += ap.one_time_bonus;
            }
            if best.is_none_or(|b| b < v) {
                best = Some(v);
            }
        }
        best.unwrap_or(0.0)
    }

    /// The per-think maximum of a definition (ActorPriority
    /// max_recruited_per_think, else the ego default).
    pub fn max_per_think(&self, d: DefId) -> i32 {
        let props = self.c.props(d);
        let id = self.c.def(d).id.to_ascii_lowercase();
        self.c
            .ego
            .actor_priorities
            .iter()
            .filter(|ap| ap.max_recruited_per_think != 0)
            .find(|ap| match (&ap.property, &ap.actor) {
                (Some(p), _) => has(props, p),
                (None, Some(a)) => *a == id,
                _ => false,
            })
            .map_or(self.c.ego.default_max_recruited_per_think, |ap| ap.max_recruited_per_think)
    }

    /// The money penalties of `7CE5E0`: the money change of the recruit (its
    /// upkeep shortfalls bought at PurchaseCost) against
    /// insufficient_income_level / a decrease.
    fn income_penalty(&self, d: DefId) -> f32 {
        let rr = &self.c.ego.recruiting;
        let delta = self.vector(d);
        let mut decreased = false;
        for i in 0..self.econ.len() {
            if self.econ.kinds[i] != 0 {
                continue;
            }
            let mut change = delta.get(i).copied().unwrap_or(0.0);
            for (j, &dj) in delta.iter().enumerate() {
                if self.econ.kinds[j] == 0 || dj == 0.0 {
                    continue;
                }
                let pc = self.econ.purchase[j].get(i).copied().unwrap_or(0.0);
                let inc = self.income.get(j).copied().unwrap_or(0.0);
                if inc < 0.0 {
                    if dj <= 0.0 || dj < -inc {
                        change += dj * pc;
                    } else {
                        change += -inc * pc;
                    }
                } else if inc + dj < 0.0 {
                    change += (inc + dj) * pc;
                }
            }
            if self.income.get(i).copied().unwrap_or(0.0) + change < rr.insufficient_income_level {
                return rr.insufficient_income_penalty;
            }
            if change < 0.0 {
                decreased = true;
            }
        }
        if decreased { rr.decreased_income_penalty } else { 0.0 }
    }

    /// `7CE3B0`: a builder or a mobile building recruited by a military
    /// request costs mismatching_builder_penalty; a specific request gets
    /// matching_builder_bonus only for a builder a construct goal needs (none
    /// are played). An organization asks its slots.
    fn builder_term(&self, d: DefId, military: bool) -> f32 {
        let def = self.c.def(d);
        if def.has_component("BuilderComponent") || def.unpacks_to.is_some() {
            return if military { -self.c.ego.recruiting.mismatching_builder_penalty } else { 0.0 };
        }
        if def.class == Some(DefClass::Organization) {
            for (m, _) in &def.slots {
                if (*m as usize) < self.c.view.world.data.defs.len() {
                    let v = self.builder_term(*m, military);
                    if v != 0.0 {
                        return v;
                    }
                }
            }
        }
        0.0
    }

    /// `7CE5E0`: the recruit value of a definition. `military` selects the
    /// core multipliers and the military builder term; `top` adds the money
    /// penalties, the builder term and the candidate fuzziness.
    pub fn recruit_value(&self, d: DefId, military: bool, top: bool, rng: &mut super::SaiRng) -> f32 {
        let def = self.c.def(d);
        let rr = &self.c.ego.recruiting;
        let mut v = 0.0;
        if def.class == Some(DefClass::Organization) {
            for (m, _) in &def.slots {
                if (*m as usize) < self.c.view.world.data.defs.len() {
                    v += self.recruit_value(*m, military, false, rng);
                }
            }
        } else {
            let cv = self.c.cv(d);
            if cv > 0.0 {
                let cost: f32 = def.cost_all.iter().sum::<f32>().max(def.cost.iter().sum::<f32>());
                let cost = if cost < 1.0 { 1.0 } else { cost };
                v = if military {
                    cv / cost * rr.core_cv_ratio_multiplier + rr.core_cv_multiplier * cv
                } else {
                    cv / cost * rr.support_cv_ratio_multiplier + rr.support_cv_multiplier * cv
                };
            }
            v += self.def_value(d, false) * rr.economic_value_multiplier;
        }
        v += self.actor_priority(d, true);
        if top {
            v -= self.income_penalty(d);
            v += self.builder_term(d, military);
            if v > 0.0 {
                v += rng.unit() * rr.candidate_fuzziness;
            }
        }
        v
    }
}

/// A kingdom's per-minute income per resource (the books' provided minus
/// consumed; the HUD rates for synthetic data).
pub(crate) fn income_of(c: &Ctx<'_, '_>, econ: &Econ, k: crate::world::KingdomId) -> Vec<f32> {
    let w = c.view.world;
    if econ.synthetic {
        return w.kingdoms.iter().find(|x| x.id == k).map_or(vec![0.0; 3], |x| x.income.to_vec());
    }
    match w.books.kingdom(k) {
        Some(b) => (0..econ.len()).map(|i| b.provided.get(i).copied().unwrap_or(0.0) - b.consumed.get(i).copied().unwrap_or(0.0)).collect(),
        None => vec![0.0; econ.len()],
    }
}
