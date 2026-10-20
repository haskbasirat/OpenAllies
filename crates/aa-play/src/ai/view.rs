//! What one computer player looks at during a think: the live organizations,
//! buildings and trucks of every kingdom, with the per-definition tables the
//! SAI valuations read (combat values, property sets). Built from the shared
//! `World` only; nothing here writes the world.
use super::Bound;
use crate::combat;
use crate::data::{DefClass, DefId};
use crate::world::{ActorId, Controller, KingdomId, Order, OrgId, World};
use glam::Vec2;

#[derive(Clone, Debug)]
pub struct OrgView {
    pub id: OrgId,
    /// Index into `World::kingdoms`.
    pub owner: usize,
    pub def: DefId,
    pub pos: Vec2,
    /// `5788B0` current value: members weighted by health.
    pub cv: f32,
    pub health: f32,
    /// Morale / maximum morale (1 when the combat state is not prepared).
    pub morale: f32,
    pub speed: f32,
    pub sight: f32,
    pub engaged: bool,
    pub routing: bool,
    /// HQ organizations (property `hq`) and trucks: not military.
    pub military: bool,
    /// Member truck of an HQ organization (deployable).
    pub truck: Option<ActorId>,
    pub order: Order,
    pub members: usize,
    pub out_of_supply: bool,
    /// A member can build (BuilderComponent): `penalty_to_builder_cv`.
    pub builder: bool,
}

#[derive(Clone, Debug)]
pub struct BuildingView {
    pub id: ActorId,
    pub owner: Option<usize>,
    pub def: DefId,
    pub pos: Vec2,
    pub cv: f32,
    pub health: f32,
    pub city: bool,
    pub recruits: bool,
    pub queue: usize,
    pub supply_range: f32,
    pub radius: f32,
}

#[derive(Clone, Debug)]
pub struct TruckView {
    pub id: ActorId,
    pub owner: usize,
    pub org: Option<OrgId>,
    pub def: DefId,
    pub pos: Vec2,
    pub unpacks_to: DefId,
    pub deploying: bool,
    pub speed: f32,
}

pub struct View<'w> {
    pub world: &'w World,
    pub me: usize,
    pub kingdom: KingdomId,
    pub team: u32,
    /// Per kingdom index: hostile to me (relation ENEMY, the player
    /// virtual +140 of 4E52A0) / allied (SELF or FRIEND, virtual +138).
    /// Neutral independents (farms, city blocks) are neither.
    pub hostile: Vec<bool>,
    pub allied: Vec<bool>,
    pub orgs: Vec<OrgView>,
    pub buildings: Vec<BuildingView>,
    pub trucks: Vec<TruckView>,
    /// Per resource (money, ammo, oil): net income per minute.
    pub income: [f32; 3],
    pub stockpile: [f32; 3],
}

pub fn kingdom_index(world: &World, id: KingdomId) -> Option<usize> {
    world.kingdoms.iter().position(|k| k.id == id)
}

pub fn has(props: &[String], p: &str) -> bool {
    props.iter().any(|q| q.eq_ignore_ascii_case(p))
}

impl<'w> View<'w> {
    pub fn build(world: &'w World, bound: &Bound, me: usize) -> Self {
        let kingdom = world.kingdoms[me].id;
        let team = world.kingdoms[me].team;
        let rel = |k: &crate::world::Kingdom| world.books.relation(&world.kingdoms, Some(kingdom), Some(k.id));
        let hostile = world.kingdoms.iter().map(|k| rel(k) == crate::books::ENEMY && !k.defeated).collect();
        let allied = world.kingdoms.iter().map(|k| matches!(rel(k), crate::books::SELF | crate::books::FRIEND)).collect();
        let data = &world.data;
        let max_health = |id: ActorId| -> f32 {
            let a = &world.actors[id as usize];
            world.combat.actor(id).filter(|c| c.initialized && c.max_health > 0.0).map(|c| c.max_health).unwrap_or(data.def(a.def).health)
        };
        let mut orgs = Vec::new();
        for org in world.orgs.iter().filter(|o| o.alive) {
            let Some(owner) = kingdom_index(world, org.kingdom) else { continue };
            let def = data.def(org.def);
            let mut cv = 0.0;
            let (mut hp, mut hp_max) = (0.0f32, 0.0f32);
            let mut speed = f32::MAX;
            let mut sight = 0.0f32;
            let mut sum = Vec2::ZERO;
            let mut members = 0usize;
            let mut truck = None;
            let mut engaged = false;
            let mut builder = def.has_component("BuilderComponent");
            for &m in &org.members {
                let Some(a) = world.actors.get(m as usize).filter(|a| a.alive && !a.removed) else { continue };
                let md = data.def(a.def);
                builder |= md.has_component("BuilderComponent");
                let maxh = max_health(m).max(1e-3);
                cv += bound.cv.get(a.def as usize).copied().unwrap_or(0.0) * (a.health / maxh).clamp(0.0, 1.0);
                hp += a.health.max(0.0);
                hp_max += maxh;
                if md.speed_max > 0.0 {
                    speed = speed.min(md.speed_max);
                }
                sight = sight.max(md.sight_range).max(md.detection_range);
                sum += a.pos.truncate();
                members += 1;
                if md.class == Some(DefClass::Truck) && md.unpacks_to.is_some() {
                    truck = Some(m);
                }
                if a.combat.target.is_some() {
                    engaged = true;
                }
            }
            if members == 0 {
                continue;
            }
            let state = world.combat.org(org.id);
            let mut pos = sum / members as f32;
            if let Some(s) = state.filter(|s| s.initialized) {
                engaged |= s.engaging() || !s.engaged_by.is_empty();
                if s.position != Vec2::ZERO {
                    pos = s.position;
                }
            }
            let morale = match state {
                Some(s) if s.initialized && s.morale_max > 0.0 => (org.morale / s.morale_max).clamp(0.0, 1.0),
                _ => 1.0,
            };
            let props = &bound.props[org.def as usize];
            let military = !has(&def.properties, "hq") && truck.is_none() && !has(props, "economic_truck") && cv > 0.0;
            orgs.push(OrgView {
                id: org.id,
                owner,
                def: org.def,
                pos,
                cv,
                health: if hp_max > 0.0 { hp / hp_max } else { 0.0 },
                morale,
                speed: if speed == f32::MAX { 0.0 } else { speed },
                sight,
                engaged,
                routing: state.is_some_and(|s| s.routing.is_some()),
                military,
                truck,
                order: org.order.clone(),
                members,
                out_of_supply: world.books.org(org.id).is_some_and(|b| b.out_of_supply),
                builder,
            });
        }
        let mut buildings = Vec::new();
        let mut trucks = Vec::new();
        for a in world.actors.iter().filter(|a| a.alive && !a.removed) {
            let def = data.def(a.def);
            let owner = a.kingdom.and_then(|k| kingdom_index(world, k));
            let city = def.city || has(&def.properties, "property_city");
            match def.class {
                Some(DefClass::Building) | Some(DefClass::Other) if def.class == Some(DefClass::Building) || city => {
                    let queue = a.queue.len().max(world.books.actor(a.id).map_or(0, |b| b.queue.items.len()));
                    let radius = def.blocking_size.map_or(4.0, |s| s[0].max(s[1]) * 0.5);
                    buildings.push(BuildingView {
                        id: a.id,
                        owner,
                        def: a.def,
                        pos: a.pos.truncate(),
                        cv: bound.cv.get(a.def as usize).copied().unwrap_or(0.0) * (a.health / max_health(a.id).max(1e-3)).clamp(0.0, 1.0),
                        health: (a.health / max_health(a.id).max(1e-3)).clamp(0.0, 1.0),
                        city,
                        recruits: !def.recruits.is_empty(),
                        queue,
                        supply_range: def.supply_range,
                        radius,
                    });
                }
                Some(DefClass::Truck) => {
                    let (Some(owner), Some(unpacks_to)) = (owner, def.unpacks_to) else { continue };
                    let deploying = matches!(a.order, Order::Deploy { .. }) || world.books.actor(a.id).is_some_and(|b| b.deploy.is_some());
                    trucks.push(TruckView { id: a.id, owner, org: a.org, def: a.def, pos: a.pos.truncate(), unpacks_to, deploying, speed: def.speed_max });
                }
                _ => {}
            }
        }
        let k = &world.kingdoms[me];
        let mut income = k.income;
        if let Some(books) = world.books.kingdom(kingdom) {
            for (i, name) in ["money", "ammo", "oil"].iter().enumerate() {
                if let Some(r) = data.resource_index(name)
                    && let (Some(p), Some(c)) = (books.provided.get(r), books.consumed.get(r))
                {
                    income[i] = p - c;
                }
            }
        }
        View { world, me, kingdom, team, hostile, allied, orgs, buildings, trucks, income, stockpile: k.stockpile }
    }

    pub fn mine(&self, owner: Option<usize>) -> bool {
        owner == Some(self.me)
    }
    pub fn is_hostile(&self, owner: Option<usize>) -> bool {
        match owner {
            Some(i) => self.hostile.get(i).copied().unwrap_or(false),
            // An unowned side is neutral (the native default relation).
            None => false,
        }
    }
    pub fn computer(&self) -> bool {
        matches!(self.world.kingdoms[self.me].controller, Controller::Computer(_))
    }
}

/// Combat value table (`4618B0` per definition, organizations summed over
/// their slots) and lower-case property sets (own and members') per def.
pub fn tables(data: &crate::data::GameData) -> (Vec<f32>, Vec<Vec<String>>) {
    let rules = combat::Rules::from_data(data);
    let mut cv = combat::combat_values(data, &rules);
    let mut props: Vec<Vec<String>> = data.defs.iter().map(|d| d.properties.iter().map(|p| p.to_ascii_lowercase()).collect()).collect();
    for (i, def) in data.defs.iter().enumerate() {
        if def.class == Some(DefClass::Organization) && !def.slots.is_empty() {
            cv[i] = def.slots.iter().filter(|(d, _)| (*d as usize) < data.defs.len()).map(|(d, _)| cv[*d as usize]).sum();
            for (d, _) in &def.slots {
                if let Some(member) = data.defs.get(*d as usize) {
                    for p in &member.properties {
                        let p = p.to_ascii_lowercase();
                        if !props[i].contains(&p) {
                            props[i].push(p);
                        }
                    }
                }
            }
        }
    }
    (cv, props)
}
