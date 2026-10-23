//! Income, upkeep and stockpiles per kingdom, per minute like the original
//! (production applies only in supply; upkeep shortfalls). Owner: core.
//!
//! Behaviour-faithful port of the original Kingdom economy:
//! - contributions (570020 refresh) are recomputed every tick from the
//!   actors instead of being added/removed incrementally; the result is the
//!   same arrays: production eligibility 56F9A0 (organization members are
//!   counted through their organization, destroyed and packing/unpacking
//!   actors produce nothing, an out-of-supply actor whose packed form has an
//!   economy produces that form's values 56F960, otherwise supply_required
//!   needs supply), upkeep 56FB10 (an organization pays for every layout slot
//!   definition while recruitable), member production traversal 56FC20;
//! - providers 0x4E3940: limited providers capped at EconomyLimitedResourceMax;
//! - shortage charges 0x4E1F60: an UPKEEP deficit is bought with the
//!   resource's PurchaseCost (ammo 2, oil 3 money each) as money consumption;
//! - GKingdom Tick 0x4E9C40 (aa-economy `tick.rs`): stock += (provided -
//!   consumed) * seconds/60, clamped [0, EconomyStockpileResourceMax];
//!   running out sets the shortage ratio 1 - P/C and the
//!   kingdom_upkeep_shortage / kingdom_upkeep_repaired notices;
//! - shortfall attrition 578660/562D10 (aa-combat `shortfall.rs`): members
//!   of an out-of-supply supply_used organization of a kingdom in shortage
//!   lose dt * max_health * shortfall_damage_rate * (0.75 + u).
use crate::data::{DefId, ResourceKind};
use crate::world::{ActorId, Event, KingdomId, World};

/// Native binary32 1/60 (0x3C888889).
const SECONDS_TO_MINUTES: f32 = f32::from_bits(0x3c88_8889);
/// 0xA2A278: minimum ratio change announced again.
const ANNOUNCE_DELTA: f32 = f32::from_bits(0x3d4c_cccd);

pub fn tick(world: &mut World) {
    refresh(world);
    flow(world, crate::DT);
    shortfall(world, crate::DT);
    for k in 0..world.kingdoms.len() as KingdomId {
        publish(world, k);
    }
}

/// Packing/unpacking actors are in the native state 21 (no production, no supply).
fn deploying(world: &World, a: ActorId) -> bool {
    world.books.actor(a).is_some_and(|b| b.deploy.is_some())
}
/// 4D0C90: in supply unless in state 19 (out_of_supply), and a supplier in
/// state 19 still counts while its relay stockpile (Supply+18) is positive.
fn supplied(world: &World, a: ActorId) -> bool {
    match world.books.actor(a) {
        Some(b) if b.out_of_supply => world.data.def(world.actors[a as usize].def).has_supply && b.supply_stock > 0.0,
        _ => true,
    }
}

/// 56F960/56FC20 production_start for one actor (member or top level).
fn actor_production(world: &World, a: ActorId, out: &mut [f32]) {
    let actor = &world.actors[a as usize];
    let d = world.data.def(actor.def);
    // 56F960: ActorData+548 is the inverse pack link (a building's truck,
    // written by 4605B0); a truck has only the forward +534 unpack link.
    let source: DefId = if !supplied(world, a)
        && let Some(p) = d.packs_to
        && world.data.def(p).has_economy
    {
        p
    } else {
        actor.def
    };
    let sd = world.data.def(source);
    if sd.has_economy {
        for (o, v) in out.iter_mut().zip(&sd.production_all) {
            *o += *v;
        }
        // 5019D0: RESOURCE_PRODUCTION modifiers (technologies such as
        // tech_ammo_production) add to the actor's production.
        if let Some(k) = actor.kingdom {
            for m in crate::production::technology_modifiers(world, k, &world.data.def(actor.def).properties) {
                if m.kind == "RESOURCE_PRODUCTION" {
                    for (o, v) in out.iter_mut().zip(&m.production) {
                        *o += *v;
                    }
                }
            }
        }
    }
}

/// 56F9A0 for an actor that is not an organization member.
fn eligible(world: &World, a: ActorId) -> bool {
    let actor = &world.actors[a as usize];
    if deploying(world, a) {
        return false;
    }
    let d = world.data.def(actor.def);
    if !supplied(world, a)
        && let Some(p) = d.packs_to
        && world.data.def(p).has_economy
    {
        return true;
    }
    !d.supply_required || supplied(world, a)
}

/// Modifier flag NO_UPKEEP (004C1110 index 9): the upkeep vector of an actor
/// whose modifier carries it is cleared (aa-actors `definition_values`). It
/// comes from the actor's own properties (its definition's and the ones the
/// map gave it, e.g. tech_noupkeep on the random map start bunkers) or from
/// a kingdom technology that applies to it.
fn no_upkeep(world: &World, a: ActorId) -> bool {
    let actor = &world.actors[a as usize];
    let d = world.data.def(actor.def);
    if d.upkeep_all.iter().all(|v| *v == 0.0) {
        return false;
    }
    let extra = world.books.actor(a).map_or(&[][..], |b| b.properties.as_slice());
    let has = |r: &String| d.properties.iter().chain(extra).any(|q| q.eq_ignore_ascii_case(r));
    let flag = |m: &crate::data::Modifier| m.kind == "FLAG" && m.flag.as_deref().is_some_and(|f| f.eq_ignore_ascii_case("NO_UPKEEP"));
    d.properties
        .iter()
        .chain(extra)
        .filter_map(|p| world.data.property(p))
        .filter(|p| p.required_properties.iter().all(|r| has(r)))
        .any(|p| p.modifiers.iter().any(flag))
        || actor.kingdom.is_some_and(|k| crate::production::technology_modifiers(world, k, &d.properties).any(flag))
}

/// Recompute every kingdom's provider and consumer arrays.
pub fn refresh(world: &mut World) {
    let n = world.data.resources.len();
    let kingdoms = world.kingdoms.len();
    let mut raw = vec![vec![0.0f32; n]; kingdoms];
    let mut upkeep = vec![vec![0.0f32; n]; kingdoms];
    for a in 0..world.actors.len() {
        let actor = &world.actors[a];
        let Some(k) = actor.kingdom.map(|k| k as usize).filter(|&k| k < kingdoms) else { continue };
        if actor.removed || !actor.alive || actor.org.is_some() {
            continue;
        }
        let d = world.data.def(actor.def);
        if d.has_economy && !no_upkeep(world, a as ActorId) {
            for (o, v) in upkeep[k].iter_mut().zip(&d.upkeep_all) {
                *o += *v;
            }
        }
        if eligible(world, a as ActorId) {
            actor_production(world, a as ActorId, &mut raw[k]);
        }
    }
    for org in &world.orgs {
        let k = org.kingdom as usize;
        if !org.alive || k >= kingdoms {
            continue;
        }
        let d = world.data.def(org.def);
        if d.recruitable == 0 {
            continue;
        }
        let u = crate::production::quote(world, org.def, crate::production::Vector::Upkeep);
        for (o, v) in upkeep[k].iter_mut().zip(&u) {
            *o += *v;
        }
        if d.has_economy {
            for (o, v) in raw[k].iter_mut().zip(&d.production_all) {
                *o += *v;
            }
        }
        for &m in &org.members {
            let ma = &world.actors[m as usize];
            if ma.removed || !ma.alive || deploying(world, m) {
                continue;
            }
            actor_production(world, m, &mut raw[k]);
        }
    }
    let mut reserved = vec![vec![0.0f32; n]; kingdoms];
    for (a, b) in world.books.actors.iter().enumerate() {
        let Some(k) = world.actors.get(a).and_then(|x| x.kingdom).map(|k| k as usize).filter(|&k| k < kingdoms) else { continue };
        for item in &b.queue.items {
            for (o, v) in reserved[k].iter_mut().zip(&item.current_upkeep) {
                *o += *v;
            }
        }
    }
    let cap = world.data.rule_ci("EconomyLimitedResourceMax").unwrap_or(f32::MAX);
    let resources = world.data.resources.clone();
    for k in 0..kingdoms {
        let kb = world.books.kingdom_mut(k as KingdomId, n);
        kb.raw_provided.copy_from_slice(&raw[k]);
        for i in 0..n {
            kb.provided[i] = if resources[i].limited_provider { raw[k][i].min(cap) } else { raw[k][i] };
            kb.upkeep[i] = upkeep[k][i];
            kb.consumed[i] = upkeep[k][i] + reserved[k][i];
        }
        // 0x4E1F60 shortage charges.
        for (i, r) in resources.iter().enumerate() {
            let deficit = kb.consumed[i] - kb.provided[i];
            if r.kind == ResourceKind::Upkeep && deficit > 0.0 {
                for (j, t) in resources.iter().enumerate() {
                    if t.kind == ResourceKind::Stockpiled {
                        kb.consumed[j] += deficit * r.purchase_cost[j];
                    }
                }
            }
        }
    }
}

/// 0x4E9C40 flow and shortage ratio for `seconds` of battle time.
pub fn flow(world: &mut World, seconds: f32) {
    let n = world.data.resources.len();
    let max = world.data.rule_ci("EconomyStockpileResourceMax").unwrap_or(f32::MAX);
    let minutes = seconds * SECONDS_TO_MINUTES;
    let resources = world.data.resources.clone();
    for k in 0..world.kingdoms.len() as KingdomId {
        if world.kingdoms[k as usize].defeated {
            continue;
        }
        let kb = world.books.kingdom_mut(k, n);
        let mut overdrawn = false;
        for i in (0..n).rev() {
            if resources[i].kind != ResourceKind::Stockpiled {
                continue;
            }
            let v = (kb.provided[i] - kb.consumed[i]) * minutes + kb.stockpiled[i];
            kb.stockpiled[i] = if v < 0.0 {
                overdrawn = true;
                0.0
            } else {
                v.min(max)
            };
        }
        let mut notice = None;
        if overdrawn {
            let (mut p, mut c) = (0.0f32, 0.0f32);
            for (i, r) in resources.iter().enumerate() {
                match r.kind {
                    ResourceKind::Stockpiled => {
                        p += kb.provided[i];
                        c += kb.consumed[i];
                    }
                    ResourceKind::Upkeep => {
                        for (j, t) in resources.iter().enumerate() {
                            if t.kind == ResourceKind::Stockpiled {
                                let charge = kb.provided[i].min(kb.consumed[i]) * r.purchase_cost[j];
                                p += charge;
                                c += charge;
                            }
                        }
                    }
                    ResourceKind::LimitedConsumed => {}
                }
            }
            if c > 0.0 {
                let ratio = 1.0 - p / c;
                let delta = if kb.shortage_ratio == 0.0 { 1.0 } else { (ratio - kb.announced_ratio).abs() };
                kb.shortage_ratio = ratio;
                if delta > ANNOUNCE_DELTA && ratio > 0.0 {
                    kb.announced_ratio = ratio;
                    notice = Some(crate::books::Notice::UpkeepShortage { kingdom: k, percent: (ratio * 100.0) as i32 });
                }
            }
        } else {
            if kb.shortage_ratio != 0.0 {
                notice = Some(crate::books::Notice::UpkeepRepaired { kingdom: k });
            }
            kb.shortage_ratio = 0.0;
            kb.announced_ratio = 0.0;
        }
        if let Some(notice) = notice {
            world.books.notices.push(notice);
        }
    }
}

/// 578660 shortfall attrition (the 57C8E0 gate: supply_used, out of
/// supply, kingdom shortage ratio above zero).
fn shortfall(world: &mut World, seconds: f32) {
    for o in 0..world.orgs.len() {
        let org = &world.orgs[o];
        if !org.alive {
            continue;
        }
        let d = world.data.def(org.def);
        if !d.supply_used || !world.books.org(o as u32).is_some_and(|b| b.out_of_supply) {
            continue;
        }
        let ratio = world.books.kingdom(org.kingdom).map_or(0.0, |k| k.shortage_ratio);
        if ratio <= 0.0 {
            continue;
        }
        let rate = d.shortfall_damage_rate;
        let members = org.members.clone();
        for m in members {
            let a = &world.actors[m as usize];
            if !a.alive || a.removed {
                continue;
            }
            let max = world.data.def(a.def).health;
            let base = seconds * max * rate;
            let u = world.random();
            let amount = (u - 0.25) * base + base;
            let a = &mut world.actors[m as usize];
            a.health = (a.health - amount).max(0.0);
            world.events.push(Event::Hit { target: m, damage: amount });
        }
    }
}

fn index(world: &World, id: &str) -> Option<usize> {
    world.data.resource_index(id)
}

/// Copy the books into `Kingdom` for the HUD: money stock, per-minute net
/// rates (provided - consumed, shortage purchases included) of money, ammo
/// and oil, and the unit limit (provided cap) and units used.
pub fn publish(world: &mut World, k: KingdomId) {
    let Some(kb) = world.books.kingdom(k).cloned() else { return };
    let ids = [index(world, "money"), index(world, "ammo"), index(world, "oil")];
    let limit = index(world, "unit_limit_provided");
    let used = index(world, "unit_limit_consumed");
    let Some(kingdom) = world.kingdoms.get_mut(k as usize) else { return };
    for (slot, i) in ids.into_iter().enumerate() {
        if let Some(i) = i {
            kingdom.stockpile[slot] = kb.stockpiled.get(i).copied().unwrap_or(0.0);
            kingdom.income[slot] = kb.provided.get(i).copied().unwrap_or(0.0) - kb.consumed.get(i).copied().unwrap_or(0.0);
        }
    }
    kingdom.unit_limit = limit.and_then(|i| kb.provided.get(i).copied()).unwrap_or(0.0);
    kingdom.units_used = used.and_then(|i| kb.consumed.get(i).copied()).unwrap_or(0.0);
}

/// Per-minute (provided, consumed) of one resource for the HUD.
pub fn rates(world: &World, k: KingdomId, resource: &str) -> Option<(f32, f32)> {
    let i = index(world, resource)?;
    let kb = world.books.kingdom(k)?;
    Some((kb.provided.get(i).copied()?, kb.consumed.get(i).copied()?))
}
