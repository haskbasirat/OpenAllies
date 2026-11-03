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

/// A static actor the player remembers that no longer stands (the region
/// kingdom record list `+18` read by `7BA1F0`: a target without an actor,
/// "Attack Region (no target -> region #n)" in the original's goal CSV).
#[derive(Clone, Debug)]
pub struct GhostView {
    pub def: DefId,
    pub owner: Option<usize>,
    pub pos: Vec2,
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
    /// Remembered static actors of the other sides that are gone.
    pub ghosts: Vec<GhostView>,
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
        let allied: Vec<bool> = world.kingdoms.iter().map(|k| matches!(rel(k), crate::books::SELF | crate::books::FRIEND)).collect();
        let data = &world.data;
        let max_health = |id: ActorId| -> f32 {
            let a = &world.actors[id as usize];
            world.combat.actor(id).filter(|c| c.initialized && c.max_health > 0.0).map(|c| c.max_health).unwrap_or(data.def(a.def).health)
        };
        // 4D0180 of standing actors: Performer +60 (57E430), Defender abilities
        // included; the table of definitions (bound.cv) is +1E4.
        let standing_table = standing_values(world);
        let standing = |d: DefId| standing_table.get(d as usize).copied().unwrap_or(0.0);
        let mut orgs = Vec::new();
        for org in world.orgs.iter().filter(|o| o.alive) {
            let Some(owner) = kingdom_index(world, org.kingdom) else { continue };
            let def = data.def(org.def);
            let mut cv = 0.0;
            let mut hp = 0.0f32;
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
                cv += standing(a.def) * (a.health / maxh).clamp(0.0, 1.0);
                hp += (a.health / maxh).clamp(0.0, 1.0);
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
                // 4D0230(1): the members' health fractions over the layout's slots, so a
                // regiment still filling up (4 of 9 when it is recruited) counts as hurt: the
                // goal log's Recover 456.1 = (1 - (4/9)^3) * 500 on every fresh regiment.
                health: hp / def.slots.iter().filter(|(s, _)| *s != u32::MAX).count().max(members) as f32,
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
                        cv: standing(a.def) * (a.health / max_health(a.id).max(1e-3)).clamp(0.0, 1.0),
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
                    let deploying = truck_deploying(world, a.id);
                    trucks.push(TruckView { id: a.id, owner, org: a.org, def: a.def, pos: a.pos.truncate(), unpacks_to, deploying, speed: def.speed_max });
                }
                _ => {}
            }
        }
        // Fog of war (UI/fog owner hook): a computer player knows the other
        // sides' actors as its kingdom sees them. The region target scan
        // `7BB5E0` asks a mobile target (`79E380` of its definition) its
        // visibility (virtual +5C -> +70 `4D3B80`) and a static one virtual
        // +58(player, 1) (`4EE3C0`): visible, else +60 (`4EAB20`): seen by
        // the kingdom's side (+74) and either a static map object
        // (`4EA350`) or remembered (the kingdom's record, `4EA430`). The
        // EXPLORED start (`0x551F90` -> `0x54F400`) explores the map's
        // neutral static objects, so every city is known from the first
        // think while another side's base is known only once seen (the
        // original's goal CSV: five city attack goals at 0.19 s, the
        // enemy Corps HQ first at 270 s). A target it does not see is
        // ordered at its kingdom's record position (`0x7C7C30`/`0x4EA430`).
        let mut ghosts = Vec::new();
        if crate::fog::enabled(world) {
            let others = |owner: Option<usize>| owner.is_some_and(|o| o != me && !allied.get(o).copied().unwrap_or(false));
            orgs.retain(|o| !others(Some(o.owner)) || crate::fog::org_visible(world, o.id, Some(kingdom)));
            buildings.retain_mut(|b| {
                if !others(b.owner) {
                    return true;
                }
                if let crate::fog::Sighting::Remembered(m) = crate::fog::sighting(world, b.id, Some(kingdom)) {
                    b.pos = m.pos.truncate();
                }
                knows(world, b.id, kingdom)
            });
            trucks.retain(|t| !others(Some(t.owner)) || crate::fog::actor_visible(world, t.id, Some(kingdom)));
            for (id, m) in crate::fog::memories(world, Some(kingdom)) {
                if world.actors.get(id as usize).is_some_and(|a| a.alive && !a.removed) {
                    continue;
                }
                let owner = m.kingdom.and_then(|k| kingdom_index(world, k));
                if others(owner) {
                    ghosts.push(GhostView { def: m.def, owner, pos: m.pos.truncate() });
                }
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
        View { world, me, kingdom, team, hostile, allied, orgs, buildings, trucks, ghosts, income, stockpile: k.stockpile }
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

/// Actor virtual +58 (player, 1): visible, or for a static actor (`4EE3C0`)
/// +60 `4EAB20`: seen by the kingdom's side and a static map object or
/// remembered.
pub(crate) fn knows(world: &World, actor: ActorId, kingdom: crate::world::KingdomId) -> bool {
    match crate::fog::sighting(world, actor, Some(kingdom)) {
        crate::fog::Sighting::Visible | crate::fog::Sighting::Remembered(_) => true,
        crate::fog::Sighting::Hidden => {
            let def = world.data.def(world.actors[actor as usize].def);
            crate::fog::actor_seen(world, actor, Some(kingdom)) && matches!(def.class, Some(DefClass::Building)) && crate::zones::def_detection(def).keeps_memory
        }
    }
}

/// A truck is on its way to unpack or unpacking. An HQ regiment's truck
/// drives under its organization's order (the movement owner resets the
/// organization to Idle when the deploy fails, the truck's own order may
/// still read Deploy).
pub(crate) fn truck_deploying(world: &World, truck: crate::world::ActorId) -> bool {
    let Some(a) = world.actor(truck) else { return false };
    if world.books.actor(truck).is_some_and(|b| b.deploy.is_some()) {
        return true;
    }
    match a.org {
        Some(o) => matches!(world.orgs[o as usize].order, Order::Deploy { .. }),
        None => matches!(a.order, Order::Deploy { .. }),
    }
}

/// The value of a standing actor per definition (`4D0180` reads Performer
/// `+60`, `57E430`: [`combat::performer_values`]), from the combat state once
/// that is prepared.
pub fn standing_values(world: &World) -> Vec<f32> {
    match world.combat.extras.as_ref() {
        Some(e) => e.defs.iter().map(|d| d.performer_value).collect(),
        None => combat::performer_values(&world.data, &combat::Rules::from_data(&world.data)),
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
