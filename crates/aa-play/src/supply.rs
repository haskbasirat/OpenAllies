//! Supply zones of HQs, depots and cities: healing and member
//! replacement for attached regiments inside a zone. Owner: core.
//!
//! Behaviour-faithful port of the original supply (aa-economy `supply.rs`):
//! - SupplyComponent Tick 0x58AE10: a relay (division HQ, depot) searches
//!   the supply zones of its kingdom it stands in for a supplier that reaches
//!   a primary (non-relay) source 0x58ADB0; a relay without one is out of
//!   supply, drains its stockpile one second per second and keeps providing
//!   supply while it lasts (0x4D0C90); a connected relay recharges at
//!   SupplyStockpileRechargeRate up to stockpile_max; the zone (centre,
//!   supply_range) is active while 0x58ACE0 provides_supply holds (owned,
//!   not packing/unpacking, relay stock, radius >= 0);
//! - actor in supply 0x4D0A80 and state 19 update 0x4D7CE0: suppliers that
//!   are primary sources always, members of a supply_used organization with
//!   their organization, an organization only while attached to a living HQ
//!   (unless in_supply_detached) and standing in a zone of its kingdom,
//!   everything else when its tile is in a zone of its kingdom;
//! - regeneration 0x57B5A0 behind the 0x579110 gate (supply_used, in supply,
//!   not engaged): one slot per tick, a damaged primary first, then the first
//!   damaged member, then an absent primary, then the first empty slot with a
//!   Body; members heal resupply_rate * max_health * (1 - shortage) * dt, an
//!   empty slot builds up the same amount until it reaches the slot
//!   definition's health and the member is created (0x57AD30).
use crate::books::SupplyZone;
use crate::world::{ActorId, KingdomId, OrgId, World};
use glam::Vec2;

fn pos2(world: &World, a: ActorId) -> Vec2 {
    world.actors[a as usize].pos.truncate()
}

fn live(world: &World, a: ActorId) -> bool {
    world.actors.get(a as usize).is_some_and(|x| x.alive && !x.removed)
}

/// True when `p` lies in an active supply zone of `kingdom`.
pub fn supplied_at(world: &World, kingdom: KingdomId, p: Vec2) -> bool {
    world.books.zones.iter().any(|z| z.kingdom == kingdom && (p - z.centre).length_squared() <= z.radius * z.radius)
}

/// 0x58ADB0: follow relay links until a primary source.
fn reaches_source(world: &World, start: ActorId) -> bool {
    let mut seen = Vec::new();
    let mut a = start;
    loop {
        if !live(world, a) || !world.data.def(world.actors[a as usize].def).has_supply {
            return false;
        }
        if !world.data.def(world.actors[a as usize].def).supply_relay {
            return true;
        }
        if seen.contains(&a) {
            return false;
        }
        seen.push(a);
        match world.books.actor(a).and_then(|b| b.supply_relay) {
            Some(next) => a = next,
            None => return false,
        }
    }
}

/// Half extents of the axis-aligned rectangle around a structure's blocking
/// shape (the actor's tile bounds +0x20..+0x30), zero without one. Same
/// convention as the movement footprint: the shape's width lies across the
/// facing, its height along it; a CIRCLE's size is its radius.
fn footprint_half(def: &crate::data::Def, facing: f32) -> Vec2 {
    let Some(size) = def.blocking_size else { return Vec2::ZERO };
    if def.blocking_shape.as_deref().is_some_and(|s| s.eq_ignore_ascii_case("CIRCLE")) {
        return Vec2::splat(size[0].max(0.0));
    }
    let (hw, hh) = (size[0].max(0.0) * 0.5, size[1].max(0.0) * 0.5);
    let (s, c) = facing.sin_cos();
    Vec2::new(hw * s.abs() + hh * c.abs(), hw * c.abs() + hh * s.abs())
}

fn deploying(world: &World, a: ActorId) -> bool {
    world.books.actor(a).is_some_and(|b| b.deploy.is_some())
}

pub fn tick(world: &mut World) {
    let dt = crate::DT;
    let recharge = world.data.rule_ci("SupplyStockpileRechargeRate").unwrap_or(1.0);
    let providers: Vec<ActorId> = (0..world.actors.len() as ActorId)
        .filter(|&a| live(world, a) && world.data.def(world.actors[a as usize].def).has_supply)
        .collect();
    // 0x58AE10 per supplier.
    for &a in &providers {
        let def = world.data.def(world.actors[a as usize].def);
        let (relay, max) = (def.supply_relay, def.supply_stockpile_max);
        let kingdom = world.actors[a as usize].kingdom;
        let p = pos2(world, a);
        if relay {
            let mut link = None;
            if let Some(k) = kingdom {
                let half = footprint_half(def, world.actors[a as usize].facing);
                for z in &world.books.zones {
                    // 004CF210 + 00554CC0: the zones over the tiles of the
                    // relay's footprint rectangle, not only its centre.
                    let d = ((z.centre - p).abs() - half).max(Vec2::ZERO);
                    if z.kingdom != k || z.provider == a || d.length_squared() > z.radius * z.radius {
                        continue;
                    }
                    if world.books.out_of_supply(z.provider) {
                        continue;
                    }
                    if reaches_source(world, z.provider) {
                        link = Some(z.provider);
                        break;
                    }
                }
            }
            world.books.actor_mut(a).supply_relay = link;
        }
        let in_supply = kingdom.is_some() && (!relay || world.books.actor(a).and_then(|b| b.supply_relay).is_some_and(|r| live(world, r)));
        let b = world.books.actor_mut(a);
        b.out_of_supply = !in_supply;
        if relay {
            if !in_supply {
                b.supply_stock = (b.supply_stock - dt).max(0.0);
            } else if b.supply_stock < max {
                b.supply_stock = (recharge * dt + b.supply_stock).min(max);
            }
        }
    }
    // Zones for this tick (0x58ACE0 provides_supply).
    let mut zones = Vec::new();
    for &a in &providers {
        let actor = &world.actors[a as usize];
        let Some(k) = actor.kingdom else { continue };
        let def = world.data.def(actor.def);
        let b = world.books.actor(a);
        let stock_ok = !def.supply_relay || b.is_some_and(|b| !b.out_of_supply || b.supply_stock > 0.0);
        if deploying(world, a) || !stock_ok || def.supply_range < 0.0 {
            if let Some(b) = world.books.actors.get_mut(a as usize) {
                b.zone_active = false;
            }
            continue;
        }
        zones.push(SupplyZone { provider: a, kingdom: k, centre: actor.pos.truncate(), radius: def.supply_range });
        world.books.actor_mut(a).zone_active = true;
    }
    world.books.zones = zones;
    // 0x4D0A80 for organizations, their members and lone actors.
    for o in 0..world.orgs.len() as OrgId {
        let org = &world.orgs[o as usize];
        if !org.alive {
            continue;
        }
        let def = world.data.def(org.def);
        if !def.supply_used {
            continue;
        }
        let attached = def.in_supply_detached || org.hq.is_some_and(|h| live(world, h));
        let at = org_position(world, o);
        // (combat owner) a Resupply special operation's SUPPLIED flag
        // keeps the organization in supply while it lasts
        let in_supply = (attached && supplied_at(world, org.kingdom, at)) || crate::special_ops::org_has_flag(world, o, "SUPPLIED");
        world.books.org_mut(o).out_of_supply = !in_supply;
        for m in world.orgs[o as usize].members.clone() {
            world.books.actor_mut(m).out_of_supply = !in_supply;
        }
    }
    for a in 0..world.actors.len() as ActorId {
        let actor = &world.actors[a as usize];
        if !actor.alive || actor.removed || world.data.def(actor.def).has_supply {
            continue;
        }
        if let Some(o) = actor.org
            && world.data.def(world.orgs[o as usize].def).supply_used
        {
            continue;
        }
        let in_supply = actor.kingdom.is_some_and(|k| supplied_at(world, k, actor.pos.truncate()));
        world.books.actor_mut(a).out_of_supply = !in_supply;
    }
    regenerate(world, dt);
    cities(world);
}

/// One city zone candidate: a lone actor or an organization (as the native
/// organization actor at its own position).
struct CityCandidate {
    kingdom: KingdomId,
    pos: Vec2,
    /// 4D0180: the current value the city tick weighs.
    current: f32,
}

/// Performer +60 (57E430): the combat value of a definition with a
/// PerformerComponent, None without one.
fn performer_value(world: &World, def: crate::data::DefId) -> Option<f32> {
    let d = world.data.def(def);
    if !d.has_component("PerformerComponent") {
        return None;
    }
    Some(world.combat.extras.as_ref().map_or(0.0, |e| e.def(def).combat_value))
}

/// 57DC40: the Body health ratio (Body +10 / +14), 1 without a Body.
fn health_ratio(world: &World, a: ActorId) -> f32 {
    let actor = &world.actors[a as usize];
    let d = world.data.def(actor.def);
    if !d.has_component("BodyComponent") {
        return 1.0;
    }
    let max = world
        .combat
        .actors
        .get(a as usize)
        .filter(|c| c.initialized && c.max_health > 0.0)
        .map_or_else(|| crate::production::max_health(world, actor.kingdom, actor.def), |c| c.max_health);
    if max > 0.0 { actor.health / max } else { 1.0 }
}

/// 5640A0 kingdom gate: owned by a major (non-independent) kingdom
/// (kingdom +400 == 0).
fn major(world: &World, k: Option<KingdomId>) -> Option<KingdomId> {
    k.filter(|&k| world.kingdoms.get(k as usize).is_some_and(|x| !x.independent))
}

/// City candidate admission 0x5640A0 for every actor and organization:
/// not dead, owned by a major kingdom, not an organization member (4549C0:
/// the organization stands for it), thing flag bit 12 (flying, 0x1000)
/// clear, actor value 4D01E0(0) > 0, and without a Body or captureable or
/// destructible. 4D01E0 is Organization +6C (5788B0 maximum: the members'
/// Performer +60 plus the empty layout slots' ActorData +1E4) + Settlement
/// +34 + Performer +60 + Denizen +14; 4D0180, the weight, uses Organization
/// +68 (the members' Performer +60 times their health ratio), Settlement
/// +30, 57DC40 (Performer +60 times the health ratio) and Denizen +10.
/// aa-play has no Settlement or Denizen occupants: those terms are 0.
fn city_candidates(world: &World) -> Vec<CityCandidate> {
    let mut out = Vec::new();
    for a in 0..world.actors.len() as ActorId {
        let actor = &world.actors[a as usize];
        if !actor.alive || actor.removed || actor.org.is_some() {
            continue;
        }
        let Some(kingdom) = major(world, actor.kingdom) else { continue };
        let d = world.data.def(actor.def);
        if d.flying {
            continue;
        }
        if d.has_component("BodyComponent") && !d.captureable && !d.destructible {
            continue;
        }
        let Some(value) = performer_value(world, actor.def) else { continue };
        if value <= 0.0 {
            continue;
        }
        out.push(CityCandidate { kingdom, pos: actor.pos.truncate(), current: value * health_ratio(world, a) });
    }
    for o in 0..world.orgs.len() as OrgId {
        let org = &world.orgs[o as usize];
        if !org.alive || !org.members.iter().any(|&m| live(world, m)) {
            continue;
        }
        let Some(kingdom) = major(world, Some(org.kingdom)) else { continue };
        let d = world.data.def(org.def);
        if d.flying {
            continue;
        }
        if d.has_component("BodyComponent") && !d.captureable && !d.destructible {
            continue;
        }
        let (mut maximum, mut current) = (0.0f32, 0.0f32);
        for (slot, (slot_def, _)) in d.slots.iter().enumerate() {
            let member = org.members.iter().copied().find(|&m| live(world, m) && world.actors[m as usize].slot == Some(slot as u16));
            match member {
                Some(m) => {
                    if let Some(v) = performer_value(world, world.actors[m as usize].def) {
                        current += health_ratio(world, m) * v;
                        maximum += v;
                    }
                }
                None => {
                    if *slot_def != crate::data::NO_DEF
                        && let Some(v) = performer_value(world, *slot_def)
                    {
                        maximum += v;
                    }
                }
            }
        }
        if maximum <= 0.0 {
            continue;
        }
        out.push(CityCandidate { kingdom, pos: org_position(world, o), current });
    }
    out
}

/// City tick 0x5647A0. Against the current owner: the value of its friends
/// in the zone defends, the value of its enemies attacks. Attackers with no
/// defender (or no owner at all) let the zone decide: the first kingdom in
/// kingdom order that has value there, no enemy value against it, and no
/// friend with more value or under threat takes the city (0x4D5230 owner
/// change, reason 10). The neutral buildings of the old owner around the
/// city change hands with it.
fn cities(world: &mut World) {
    use crate::books::{ENEMY, FRIEND, SELF};
    let n = world.kingdoms.len();
    let candidates = city_candidates(world);
    for c in 0..world.actors.len() as ActorId {
        let city = &world.actors[c as usize];
        if !city.alive || city.removed {
            continue;
        }
        let def = world.data.def(city.def);
        if !def.city || def.city_zone_radius <= 0.0 {
            continue;
        }
        let (centre, radius, owner) = (city.pos.truncate(), def.city_zone_radius, city.kingdom);
        // The candidates in the zone (the city itself never is one: it has no
        // Performer and belongs to its owner).
        let mut present = vec![0.0f32; n];
        let mut any = false;
        for cand in &candidates {
            if cand.pos.distance_squared(centre) <= radius * radius && (cand.kingdom as usize) < n {
                present[cand.kingdom as usize] += cand.current;
                any = true;
            }
        }
        if !any {
            continue;
        }
        let rel = |a: usize, b: usize| world.books.relation(&world.kingdoms, Some(a as u32), Some(b as u32));
        let decide = match owner {
            None => true,
            Some(o) => {
                let (mut defend, mut attack) = (0.0f32, 0.0f32);
                for k in 0..n {
                    match rel(k, o as usize) {
                        SELF | FRIEND => defend += present[k],
                        ENEMY => attack += present[k],
                        _ => {}
                    }
                }
                attack > 0.0 && defend <= 0.0
            }
        };
        if !decide {
            continue;
        }
        let threat: Vec<f32> = (0..n).map(|k| (0..n).filter(|&e| e != k && rel(e, k) == ENEMY).map(|e| present[e]).sum()).collect();
        let winner = (0..n).find(|&k| {
            threat[k] <= 0.0
                && present[k] > 0.0
                && !(0..n).any(|f| f != k && rel(k, f) == FRIEND && (present[k] < present[f] || threat[f] > 0.0))
        });
        let Some(w) = winner.map(|k| k as u32) else { continue };
        if Some(w) == owner {
            continue;
        }
        world.actors[c as usize].kingdom = Some(w);
        world.events.push(crate::world::Event::Captured { actor: c, kingdom: w });
        // The city's neutral buildings (city blocks) go with it.
        if let Some(o) = owner {
            for b in 0..world.actors.len() {
                let a = &world.actors[b];
                if a.kingdom == Some(o)
                    && !a.removed
                    && b as ActorId != c
                    && world.data.def(a.def).city_zone_radius <= 0.0
                    && world.data.def(a.def).class == Some(crate::data::DefClass::Building)
                    && a.pos.truncate().distance_squared(centre) <= radius * radius
                {
                    world.actors[b].kingdom = Some(w);
                }
            }
        }
    }
}

/// The organization's own position: it follows its leader (57C800), so the
/// primary member when alive, else the formation anchor.
pub fn org_position(world: &World, o: OrgId) -> Vec2 {
    let org = &world.orgs[o as usize];
    let leader = world.data.def(org.def).leader_slot;
    org.members
        .iter()
        .copied()
        .find(|&m| live(world, m) && world.actors[m as usize].slot == leader && leader.is_some())
        .or_else(|| org.members.iter().copied().find(|&m| live(world, m)))
        .map_or(org.anchor, |m| pos2(world, m))
}

/// 0x579110 gate inputs owned by combat: engaged (an engage target) or
/// routing (mind_routing has resupply = false); a member shooting counts too.
fn engaged(world: &World, o: OrgId) -> bool {
    if crate::combat::org_state(world, o).is_some_and(|c| c.engaging() || c.routing.is_some()) {
        return true;
    }
    world.orgs[o as usize].members.iter().any(|&m| live(world, m) && world.actors[m as usize].combat.target.is_some())
}

fn regenerate(world: &mut World, dt: f32) {
    for o in 0..world.orgs.len() as OrgId {
        let org = &world.orgs[o as usize];
        if !org.alive {
            continue;
        }
        let def = world.data.def(org.def);
        if !def.supply_used || world.books.org(o).is_none_or(|b| b.out_of_supply) || engaged(world, o) {
            continue;
        }
        let shortage = world.books.kingdom(org.kingdom).map_or(0.0, |k| k.shortage_ratio);
        let factor = if shortage > 0.0 { 1.0 - shortage } else { 1.0 };
        let slots = def.slots.len();
        let member_at = |slot: usize| {
            org.members.iter().copied().find(|&m| live(world, m) && world.actors[m as usize].slot == Some(slot as u16))
        };
        let damaged = |m: ActorId| {
            let a = &world.actors[m as usize];
            a.health < world.data.def(a.def).health
        };
        let leader = def.leader_slot.map(usize::from).filter(|&l| l < slots);
        let choice = leader
            .filter(|&l| member_at(l).is_some_and(damaged))
            .or_else(|| (0..slots).find(|&s| member_at(s).is_some_and(damaged)))
            .or_else(|| leader.filter(|&l| member_at(l).is_none() && def.slots[l].0 != u32::MAX))
            .or_else(|| (0..slots).find(|&s| member_at(s).is_none() && def.slots[s].0 != u32::MAX && world.data.def(def.slots[s].0).has_component("BodyComponent")));
        let Some(slot) = choice else { continue };
        if let Some(m) = member_at(slot) {
            let a = &world.actors[m as usize];
            let md = world.data.def(a.def);
            let amount = md.resupply_rate * md.health * factor * dt;
            let a = &mut world.actors[m as usize];
            a.health = (a.health + amount).min(md.health);
            continue;
        }
        let slot_def = def.slots[slot].0;
        let sd = world.data.def(slot_def);
        // 0x5014E0: the organization modifier context multiplies +2CC
        // (technologies such as tech_plentiful_ammunition: RESUPPLY_RATE).
        let rate = sd.resupply_rate * crate::production::multiply_factor(world, org.kingdom, &def.properties, "RESUPPLY_RATE");
        // (combat owner) the enchantments' RESUPPLY_RATE (Resupply special
        // operations) in the same 502A70 sum
        let rate = rate + sd.resupply_rate * (crate::special_ops::org_resupply_multiplier(world, o) - 1.0);
        let amount = sd.health * rate * factor * dt;
        let (anchor, heading, kingdom) = (org_position(world, o), org.heading, org.kingdom);
        let offset = def.slots[slot].1;
        let leader_offset = leader.map_or([0.0, 0.0], |l| def.slots[l].1);
        let b = world.books.org_mut(o);
        if b.progress.len() < slots {
            b.progress.resize(slots, 0.0);
        }
        b.progress[slot] += amount;
        if b.progress[slot] >= sd.health {
            b.progress[slot] = 0.0;
            let local = [offset[0] - leader_offset[0], offset[1] - leader_offset[1]];
            let pos = anchor + crate::production::rotate(local, heading);
            let a = crate::production::spawn_single(world, slot_def, Some(kingdom), pos, heading);
            world.actors[a as usize].org = Some(o);
            world.actors[a as usize].slot = Some(slot as u16);
            world.orgs[o as usize].members.push(a);
        }
    }
}
