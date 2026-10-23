//! Player orders of the original game interface that the battle systems do
//! not carry themselves: the formation buttons, rally points, selling a
//! structure and detaching from an HQ. Owner: UI (HUD commands).
//!
//! Behaviour-faithful reading of the original actor commands (ids of
//! `0x4DA2F0`):
//! * set_formation (0x21, the formation buttons' `FormationChange <id>`,
//!   `0x629AB0`): the organization takes one of its layout's formations;
//!   its members hold that formation's slots from the next formation turn
//!   (`0x57BEF0` reads the current formation), at once when it stands.
//! * set_rally_point (0x10): every organization recruited from the
//!   building then moves to the point ("Commands all units recruited from
//!   this building to move to target area").
//! * sell (0x14, availability `0x565140`): a structure with a
//!   ConstructionComponent (can_sell) is removed at once and the kingdom
//!   gets SellFactor (SVars 0.5) of its cost, each resource rounded
//!   (`0x565500`); refused while under siege when AllowSellUnderSiege is
//!   false (SVars_AA).
//! * hq_detach (0x20): the organization leaves its HQ.
//! * disband (10): own organizations and single actors leave the battle at
//!   once ("Permanently disbands selection"), without a refund.
use crate::data::{DefClass, DefId};
use crate::world::{ActorId, Command, Event, KingdomId, Order, OrgId, Unit, World};
use glam::Vec2;
use std::collections::BTreeMap;

/// Player order side state (`World::player_orders`).
#[derive(Debug, Default)]
pub struct PlayerOrders {
    /// Chosen formation per organization (index into `Def::formations`).
    formations: BTreeMap<OrgId, usize>,
    /// Rally point per recruiting building.
    rally: BTreeMap<ActorId, Vec2>,
    /// Organizations created so far (recruits newer than this are checked
    /// against the rally points).
    orgs_seen: usize,
}

/// The organization's current formation (index into `Def::formations`):
/// the one chosen with the formation buttons, else the layout default.
pub fn formation(world: &World, org: OrgId) -> Option<usize> {
    let o = world.orgs.get(org as usize)?;
    let def = world.data.def(o.def);
    world.player_orders.formations.get(&org).copied().filter(|&i| i < def.formations.len()).or(def.default_formation)
}

/// The rally point of a building, if one is set.
pub fn rally_point(world: &World, building: ActorId) -> Option<Vec2> {
    world.player_orders.rally.get(&building).copied()
}

/// A structure that can be sold (ConstructionComponent, not a city).
pub fn sellable(world: &World, building: ActorId) -> bool {
    let Some(a) = world.actor(building) else { return false };
    let d = world.data.def(a.def);
    d.class == Some(DefClass::Building) && d.has_component("ConstructionComponent") && !d.city && !d.captureable
}

/// Under siege: an enemy attacks the structure now (the under_siege actor
/// state).
pub fn under_siege(world: &World, building: ActorId) -> bool {
    world.combat.actor(building).is_some_and(|c| c.targeted)
}

/// The sell value per resource (`0x565500`): floor(SellFactor * cost + 0.5).
pub fn sell_value(world: &World, def: DefId) -> Vec<f32> {
    let factor = world.data.rule_ci("SellFactor").unwrap_or(0.5);
    let d = world.data.def(def);
    let n = world.data.resources.len();
    let mut cost = d.cost_all.clone();
    if cost.len() < n {
        cost.resize(n, 0.0);
        for (i, v) in d.cost.iter().enumerate() {
            if cost[i] == 0.0 {
                cost[i] = *v;
            }
        }
    }
    cost.iter().map(|c| (factor * c + 0.5).floor()).collect()
}

/// Whether selling is refused now (the original disables the button).
pub fn sell_refused(world: &World, building: ActorId) -> Option<&'static str> {
    if !sellable(world, building) {
        return Some("this structure cannot be sold");
    }
    if crate::production::busy(world, building) {
        return Some("the structure is busy");
    }
    let allow = world.data.rule_ci("AllowSellUnderSiege").unwrap_or(1.0) != 0.0;
    if !allow && under_siege(world, building) {
        return Some("cannot sell a structure under siege");
    }
    None
}

/// Apply one of this module's commands; Err is the refusal reason.
pub fn command(world: &mut World, command: &Command) -> Result<(), String> {
    match command {
        Command::SetFormation { kingdom, orgs, formation } => {
            let mut any = false;
            for &o in orgs {
                let Some(org) = world.orgs.get(o as usize).filter(|x| x.alive && x.kingdom == *kingdom) else { continue };
                let def = world.data.def(org.def);
                if *formation >= def.formations.len() {
                    continue;
                }
                world.player_orders.formations.insert(o, *formation);
                any = true;
                // a standing regiment turns into the new formation at once
                if org.order == Order::Idle {
                    let to = org.anchor;
                    let _ = crate::movement::command(world, &Command::Move { kingdom: *kingdom, units: vec![Unit::Org(o)], to, attack_move: false });
                }
            }
            if any { Ok(()) } else { Err("no organization takes this formation".into()) }
        }
        Command::SetRallyPoint { kingdom, building, at } => {
            let a = world.actor(*building).filter(|a| a.alive && a.kingdom == Some(*kingdom)).ok_or("not your building")?;
            if world.data.def(a.def).recruits.is_empty() {
                return Err("this building does not recruit".into());
            }
            world.player_orders.rally.insert(*building, *at);
            Ok(())
        }
        Command::Sell { kingdom, building } => sell(world, *kingdom, *building),
        Command::Detach { kingdom, org } => {
            let o = world.orgs.get(*org as usize).filter(|o| o.alive && o.kingdom == *kingdom).ok_or("not your organization")?;
            if o.hq.is_none() {
                return Err("not attached to an HQ".into());
            }
            world.orgs[*org as usize].hq = None;
            Ok(())
        }
        Command::Disband { kingdom, units } => {
            let mut gone = Vec::new();
            for u in units {
                match *u {
                    Unit::Org(o) => {
                        let Some(org) = world.orgs.get(o as usize).filter(|x| x.alive && x.kingdom == *kingdom) else { continue };
                        gone.extend(org.members.iter().copied());
                        world.orgs[o as usize].alive = false;
                    }
                    Unit::Actor(a) => {
                        if world.actor(a).is_some_and(|x| x.alive && x.kingdom == Some(*kingdom) && x.org.is_none()) {
                            gone.push(a);
                        }
                    }
                }
            }
            if gone.is_empty() {
                return Err("nothing of this kingdom to disband".into());
            }
            for id in gone {
                let x = &mut world.actors[id as usize];
                x.alive = false;
                x.removed = true;
                x.order = Order::Idle;
            }
            Ok(())
        }
        other => Err(format!("{other:?} is not a player order")),
    }
}

fn sell(world: &mut World, kingdom: KingdomId, building: ActorId) -> Result<(), String> {
    let a = world.actor(building).filter(|a| a.alive && a.kingdom == Some(kingdom)).ok_or("not your building")?;
    if let Some(r) = sell_refused(world, building) {
        return Err(r.into());
    }
    let def = a.def;
    let value = sell_value(world, def);
    let n = world.data.resources.len();
    let max = world.data.rule_ci("EconomyStockpileResourceMax").unwrap_or(f32::MAX);
    {
        let k = world.books.kingdom_mut(kingdom, n);
        for (s, v) in k.stockpiled.iter_mut().zip(&value) {
            *s = (*s + v).clamp(0.0, max);
        }
    }
    crate::economy::publish(world, kingdom);
    // the structure (and an HQ's organization) is removed at once
    let org = world.actors[building as usize].org;
    let mut gone = vec![building];
    if let Some(o) = org {
        if world.data.def(world.orgs[o as usize].def).max_attachments.is_some() {
            gone.extend(world.orgs[o as usize].members.iter().copied());
            world.orgs[o as usize].alive = false;
        }
    }
    for id in gone {
        let x = &mut world.actors[id as usize];
        x.alive = false;
        x.removed = true;
        x.order = Order::Idle;
        x.queue.clear();
    }
    for r in world.orgs.iter_mut() {
        if r.hq == Some(building) {
            r.hq = None;
        }
    }
    world.player_orders.rally.remove(&building);
    Ok(())
}

/// After the battle tick: organizations recruited from a building with a
/// rally point move to it.
pub fn tick(world: &mut World) {
    let from = world.player_orders.orgs_seen.min(world.orgs.len());
    world.player_orders.orgs_seen = world.orgs.len();
    if world.player_orders.rally.is_empty() {
        return;
    }
    let recruited: Vec<OrgId> = world
        .events
        .iter()
        .filter_map(|e| match e {
            Event::Recruited { org: Some(o), .. } => Some(*o),
            _ => None,
        })
        .filter(|o| (*o as usize) >= from)
        .collect();
    for o in recruited {
        let (kingdom, at) = {
            let org = &world.orgs[o as usize];
            (org.kingdom, org.anchor)
        };
        // the building whose creation point the organization stands on
        let source = world.player_orders.rally.iter().find_map(|(&b, &p)| {
            let a = world.actor(b).filter(|a| a.alive && a.kingdom == Some(kingdom))?;
            let d = world.data.def(a.def);
            let start = a.pos.truncate() + crate::production::rotate(d.creation_offset.unwrap_or([0.0, 0.0]), a.facing);
            (start.distance(at) < 0.01).then_some(p)
        });
        if let Some(to) = source {
            let _ = crate::movement::command(world, &Command::Move { kingdom, units: vec![Unit::Org(o)], to, attack_move: false });
        }
    }
}
