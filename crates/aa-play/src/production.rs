//! Recruiting: queues on buildings/HQs, cost, recruit time, spawning the
//! organization's members in their layout slots. Owner: core.
//!
//! Behaviour-faithful port of the original structure production queue
//! (aa-production `request_recruit` 0x5836E0/0x582AE0/0x589E00, `duration`
//! 0x588D60, `start_head` 0x589420, `process` 0x589F90, `remove` 0x5894C0,
//! `cancel` 0x589BD0) and the recruit spawn of KKC_GWorld::CreateOrganization
//! (0x4FA2C4: leader slot first, then every slot with a definition) with the
//! Denizen DeploymentInfo placement of 0x4FACA0 (creation point, walk to the
//! deployment point, both turned by the building facing).
use crate::books::{PauseReason, Queue, QueueItem};
use crate::data::{DefClass, DefId, ResourceKind};
use crate::world::{ActorId, Command, Event, KingdomId, Order, OrgId, Organization, World};
use glam::Vec2;

/// Native 0x4FAD28 / 0x578250 rotation of a local offset (X right, Y forward)
/// by a facing angle (0 = native +X).
pub fn rotate(offset: [f32; 2], facing: f32) -> Vec2 {
    let (s, c) = facing.sin_cos();
    Vec2::new(s * offset[0] + c * offset[1], s * offset[1] - c * offset[0])
}

fn angle_of(v: Vec2) -> Option<f32> {
    (v.length_squared() > 1e-8).then(|| v.y.atan2(v.x))
}

/// Create one actor standing on the ground at `pos` (books entry included).
pub fn spawn_single(world: &mut World, def: DefId, kingdom: Option<KingdomId>, pos: Vec2, facing: f32) -> ActorId {
    let ground = world.ground(pos);
    let id = world.spawn_actor(def, kingdom, ground, facing);
    let now = world.time;
    world.books.actor_mut(id).created_at = now;
    // Handicap / technology HEALTH multipliers apply from creation.
    let health = max_health(world, kingdom, def);
    world.actors[id as usize].health = health;
    id
}

/// Create organization `def` at `anchor` facing `heading` with a member in
/// every slot that has a definition, the leader (primary) slot first like
/// 0x4FA2C4, each standing at its default-formation offset.
pub fn spawn_organization(world: &mut World, def: DefId, kingdom: KingdomId, anchor: Vec2, heading: f32, hq: Option<ActorId>) -> OrgId {
    let data = world.data.clone();
    let d = data.def(def);
    let id = new_organization(world, def, kingdom, anchor, heading, hq);
    let mut order: Vec<usize> = Vec::with_capacity(d.slots.len());
    if let Some(l) = d.leader_slot.map(usize::from).filter(|&l| l < d.slots.len()) {
        order.push(l);
    }
    order.extend((0..d.slots.len()).filter(|i| Some(*i as u16) != d.leader_slot));
    let mut morale = 0.0;
    for slot in order {
        let (member, offset) = d.slots[slot];
        if member == u32::MAX {
            continue;
        }
        let pos = anchor + rotate(offset, heading);
        let a = spawn_single(world, member, Some(kingdom), pos, heading);
        world.actors[a as usize].org = Some(id);
        world.actors[a as usize].slot = Some(slot as u16);
        world.orgs[id as usize].members.push(a);
        morale += data.def(member).morale_value;
    }
    world.orgs[id as usize].morale = morale;
    let slots = d.slots.len();
    world.books.org_mut(id).progress = vec![0.0; slots];
    id
}

/// The organization actor alone (no members yet), standing at `anchor`.
fn new_organization(world: &mut World, def: DefId, kingdom: KingdomId, anchor: Vec2, heading: f32, hq: Option<ActorId>) -> OrgId {
    let id = world.orgs.len() as OrgId;
    world.orgs.push(Organization {
        id,
        def,
        kingdom,
        members: Vec::new(),
        hq,
        order: Order::Idle,
        anchor,
        heading,
        morale: 0.0,
        experience: 0.0,
        alive: true,
    });
    // CreateOrganization 0x4FA150: the organization's id before its members'.
    world.ids.created_org(id);
    let slots = world.data.def(def).slots.len();
    world.books.org_mut(id).progress = vec![0.0; slots];
    id
}

/// 57AD30's occupied test: a living member holds `slot`.
fn slot_member(world: &World, o: OrgId, slot: usize) -> Option<ActorId> {
    world.orgs[o as usize]
        .members
        .iter()
        .copied()
        .find(|&m| world.actors.get(m as usize).is_some_and(|a| a.alive && !a.removed && a.slot == Some(slot as u16)))
}

/// 4FBF40 structure search: the nearest friendly structure with a
/// DeploymentInfo (ActorData +3EC) whose definition is in the member
/// definition's +17C list, within the organization's control_range
/// (Organization +AC = ActorData +2FC) of the organization.
fn member_structure(world: &World, o: OrgId, member_def: DefId) -> Option<ActorId> {
    let org = &world.orgs[o as usize];
    let range = world.data.def(org.def).control_range;
    let at = crate::supply::org_position(world, o);
    let md = world.data.def(member_def);
    let mut best: Option<(f32, ActorId)> = None;
    for a in &world.actors {
        if !a.alive || a.removed || world.data.def(a.def).creation_offset.is_none() || !md.recruited_at.contains(&a.def) {
            continue;
        }
        let rel = world.books.relation(&world.kingdoms, a.kingdom, Some(org.kingdom));
        if rel != crate::books::SELF && rel != crate::books::FRIEND {
            continue;
        }
        let d = a.pos.truncate().distance_squared(at);
        if d > range * range {
            continue;
        }
        if best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, a.id));
        }
    }
    best.map(|(_, a)| a)
}

/// The radius a definition's shape blocks with: a CIRCLE's radius, half the
/// smaller side of a SQUARE / RECT (the movement occupancy reading).
fn shape_radius(d: &crate::data::Def) -> f32 {
    let Some(s) = d.blocking_size else { return 0.0 };
    match d.blocking_shape.as_deref().map(str::to_ascii_uppercase).as_deref() {
        Some("CIRCLE") => s[0].max(0.0),
        Some(_) => 0.5 * s[0].min(s[1]).max(0.0),
        None => 0.0,
    }
}

/// 553BA0 for the shape of `def` standing at `p`: inside the map, on ground
/// it can stand on, outside every building footprint and, with `movers`,
/// clear of every other blocking actor's shape (553FA0 queries with flag
/// 0x20000000 set, which leaves the movers out).
fn spot_open(world: &World, def: DefId, p: Vec2, movers: bool) -> bool {
    let d = world.data.def(def);
    let r = shape_radius(d);
    let map = &world.map;
    if map.width > 0 {
        let extent = Vec2::new(map.width as f32, map.height as f32) * map.cell_size;
        if p.x - r < 0.0 || p.y - r < 0.0 || p.x + r > extent.x || p.y + r > extent.y {
            return false;
        }
        let c = map.cell_size;
        let cell = |x: f32, y: f32| ((y / c) as u32).min(map.height - 1) * map.width + ((x / c) as u32).min(map.width - 1);
        let mask = if d.water_based { crate::map::BLOCKED } else { crate::map::BLOCKED | crate::map::NOT_LAND_PASSABLE };
        for (x, y) in [(p.x - r, p.y - r), (p.x + r, p.y - r), (p.x - r, p.y + r), (p.x + r, p.y + r), (p.x, p.y)] {
            if map.flags.get(cell(x, y) as usize).is_some_and(|f| f & mask != 0) {
                return false;
            }
        }
    }
    for a in &world.actors {
        if a.removed || !a.alive {
            continue;
        }
        let od = world.data.def(a.def);
        let q = a.pos.truncate();
        if od.class == Some(DefClass::Building) {
            let Some(s) = od.blocking_size else { continue };
            let half = if od.blocking_shape.as_deref().is_some_and(|x| x.eq_ignore_ascii_case("CIRCLE")) { Vec2::splat(s[0]) } else { Vec2::new(s[0], s[1]) * 0.5 };
            let d = ((p - q).abs() - half).max(Vec2::ZERO);
            if d.length_squared() < r * r || (p - q).abs().cmplt(half).all() {
                return false;
            }
        } else if movers && od.blocking && od.speed_max > 0.0 {
            let rr = r + shape_radius(od);
            if (p - q).length_squared() < rr * rr {
                return false;
            }
        }
    }
    true
}

/// FindOpenPositionLineThenRadius 0x554BF0 for the shape of `def`: walk
/// from `point` back toward `from` in steps of the shape radius (at least a
/// quarter cell, 0x553FA0) to the first spot free of static obstacles, then
/// FindOpenPositionNear, movers included
/// (0x554B30): that spot, else rings of eight directions within `radius`,
/// starting along `facing` and turning by -pi/4 (0x554850). Keeps the line
/// result when the rings find nothing.
fn open_position(world: &World, def: DefId, point: Vec2, from: Vec2, facing: Vec2, radius: f32) -> Vec2 {
    let step = shape_radius(world.data.def(def)).max(world.map.cell_size * 0.25).max(0.05);
    let delta = from - point;
    let length = delta.length();
    let dir = if length > 0.0 { delta / length } else { Vec2::ZERO };
    let mut base = point;
    let mut t = 0.0f32;
    loop {
        let p = point + dir * t.min(length);
        if spot_open(world, def, p, false) {
            base = p;
            break;
        }
        if t >= length {
            break;
        }
        t += step;
    }
    if spot_open(world, def, base, true) {
        return base;
    }
    let rings = ((radius / step) as i32).clamp(1, 20);
    let ring_step = radius / rings as f32;
    let facing = if facing.length_squared() > 0.0 { facing.normalize() } else { Vec2::X };
    for k in 1..=rings {
        let mut offset = facing * (k as f32 * ring_step);
        for _ in 0..8 {
            if spot_open(world, def, base + offset, true) {
                return base + offset;
            }
            let (s, c) = (-std::f32::consts::FRAC_PI_4).sin_cos();
            offset = Vec2::new(offset.x * c - offset.y * s, offset.x * s + offset.y * c);
        }
    }
    base
}

/// 4FACA0's mind order (vf198 = 52F320, KKC_UAIStateDeploy): the new member
/// walks from the door to the structure's deployment point on its own, then
/// rejoins its organization (movement::walk_member).
fn walk_out(world: &mut World, member: ActorId, to: Vec2) {
    crate::movement::walk_member(world, member, to);
}

/// Organization member creation 57AD30 -> 4FBF40 for an empty `slot`: at
/// the door of the nearest structure that recruits it (DeploymentInfo
/// creation_offset turned by the structure facing, facing its deployment
/// point, which it then walks to; 4FACA0 + 57D860), else at the slot's
/// formation position around the organization (578290 + 57D950). None when
/// the slot is occupied or has no definition.
pub fn spawn_member(world: &mut World, o: OrgId, slot: usize) -> Option<ActorId> {
    let org = world.orgs.get(o as usize).filter(|x| x.alive)?;
    let def = world.data.def(org.def);
    let (member_def, offset) = *def.slots.get(slot)?;
    if member_def == crate::data::NO_DEF || slot_member(world, o, slot).is_some() {
        return None;
    }
    let (kingdom, heading) = (org.kingdom, org.heading);
    let (pos, facing, walk_to) = match member_structure(world, o, member_def) {
        Some(b) => {
            let s = &world.actors[b as usize];
            let sd = world.data.def(s.def);
            let door = s.pos.truncate() + rotate(sd.creation_offset.unwrap_or([0.0, 0.0]), s.facing);
            let to = s.pos.truncate() + rotate(sd.deployment_offset.unwrap_or([0.0, 0.0]), s.facing);
            (door, angle_of(to - door).unwrap_or(s.facing), (to != door).then_some(to))
        }
        None => {
            let leader = def.leader_slot.map(usize::from).filter(|&l| l < def.slots.len());
            let leader_offset = leader.map_or([0.0, 0.0], |l| def.slots[l].1);
            let local = [offset[0] - leader_offset[0], offset[1] - leader_offset[1]];
            (crate::supply::org_position(world, o) + rotate(local, heading), heading, None)
        }
    };
    let a = spawn_single(world, member_def, Some(kingdom), pos, facing);
    world.actors[a as usize].org = Some(o);
    world.actors[a as usize].slot = Some(slot as u16);
    world.orgs[o as usize].members.push(a);
    if let Some(to) = walk_to {
        walk_out(world, a, to);
    }
    Some(a)
}

/// Which kind of resource vector a quote reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Vector {
    Cost,
    Upkeep,
    Production,
}

/// 56F850/56F580/56F660 for a definition: its own vector plus, for an
/// organization, the vector of every expanded layout slot's definition.
pub fn quote(world: &World, def: DefId, kind: Vector) -> Vec<f32> {
    let data = &world.data;
    let n = data.resources.len();
    let mut out = vec![0.0; n];
    let add = |out: &mut Vec<f32>, d: DefId| {
        let d = data.def(d);
        if !d.has_economy {
            return;
        }
        let v = match kind {
            Vector::Cost => &d.cost_all,
            Vector::Upkeep => &d.upkeep_all,
            Vector::Production => &d.production_all,
        };
        for (o, x) in out.iter_mut().zip(v) {
            *o += *x;
        }
    };
    add(&mut out, def);
    for &(slot_def, _) in &data.def(def).slots {
        if slot_def != u32::MAX {
            add(&mut out, slot_def);
        }
    }
    out
}

/// Technology modifiers of `kingdom` that apply to an actor with
/// `properties` (the technology's required_properties must all be present),
/// as the kingdom property records apply them to every kingdom actor
/// (0x4E3AD0 -> 0x4CEF10).
pub fn technology_modifiers<'a>(world: &'a World, kingdom: KingdomId, properties: &'a [String]) -> impl Iterator<Item = &'a crate::data::Modifier> + 'a {
    let books = world.books.kingdom(kingdom);
    let techs: &'a [String] = books.map_or(&[], |k| k.technologies.as_slice());
    techs
        .iter()
        .chain(books.and_then(|k| k.handicap.as_ref()))
        .filter_map(move |t| world.data.property(t))
        .filter(move |p| p.required_properties.iter().all(|r| properties.iter().any(|q| q.eq_ignore_ascii_case(r))))
        .flat_map(|p| p.modifiers.iter())
}

/// 502A70 MULTIPLY aggregation for one attribute: 1 + sum (cost attributes
/// STRUCTURE_COST/RECRUIT_COST negate the amounts, 4FED70), floored at
/// ModifierAttributeClampMultiply.
pub fn multiply_factor(world: &World, kingdom: KingdomId, properties: &[String], attribute: &str) -> f32 {
    let negate = attribute.eq_ignore_ascii_case("RECRUIT_COST") || attribute.eq_ignore_ascii_case("STRUCTURE_COST");
    let sum: f32 = technology_modifiers(world, kingdom, properties)
        .filter(|m| m.kind == "MULTIPLY" && m.attribute.as_deref().is_some_and(|a| a.eq_ignore_ascii_case(attribute)))
        .map(|m| if negate { -m.amount } else { m.amount })
        .sum();
    let clamp = world.data.rule_ci("ModifierAttributeClampMultiply").unwrap_or(0.05);
    (1.0 + sum).max(clamp)
}

/// Sum of the HEALTH multiply amounts of the kingdom's handicap and
/// technologies for an actor of `def` (502A70 adds them to 1 together with
/// any experience level amount, floored at ModifierAttributeClampMultiply).
pub fn health_amount(world: &World, kingdom: Option<KingdomId>, def: DefId) -> f32 {
    let Some(k) = kingdom else { return 0.0 };
    technology_modifiers(world, k, &world.data.def(def).properties)
        .filter(|m| m.kind == "MULTIPLY" && m.attribute.as_deref().is_some_and(|a| a.eq_ignore_ascii_case("HEALTH")))
        .map(|m| m.amount)
        .sum()
}

/// Maximum health of an actor of `def` owned by `kingdom` (handicap and
/// technology HEALTH modifiers; experience levels are added by morale).
pub fn max_health(world: &World, kingdom: Option<KingdomId>, def: DefId) -> f32 {
    let clamp = world.data.rule_ci("ModifierAttributeClampMultiply").unwrap_or(0.05);
    world.data.def(def).health * (1.0 + health_amount(world, kingdom, def)).max(clamp)
}

/// Recruit price at `building` (501730): the definition quote, times the
/// building's RECRUIT_COST factor for a recruitable definition.
pub fn recruit_cost(world: &World, building: ActorId, def: DefId) -> Vec<f32> {
    let mut cost = quote(world, def, Vector::Cost);
    if let Some(a) = world.actor(building)
        && let Some(k) = a.kingdom
        && world.data.def(def).recruitable != 0
    {
        let f = multiply_factor(world, k, &world.data.def(a.def).properties, "RECRUIT_COST");
        for c in cost.iter_mut() {
            *c *= f;
        }
    }
    cost
}

/// The LIMITED_CONSUMED part of a vector (what a queue item reserves).
pub fn limited_part(world: &World, v: &[f32]) -> Vec<f32> {
    world
        .data
        .resources
        .iter()
        .zip(v)
        .map(|(r, x)| if r.kind == ResourceKind::LimitedConsumed { *x } else { 0.0 })
        .collect()
}

/// The HQ organization a building belongs to when that organization takes
/// attachments (HQComponent max_attachments).
pub fn hq_org(world: &World, building: ActorId) -> Option<OrgId> {
    let o = world.actor(building)?.org?;
    let org = world.orgs.get(o as usize)?;
    (org.alive && world.data.def(org.def).max_attachments.is_some()).then_some(o)
}

/// Organizations attached to the HQ of `building` (its HQ organization's members).
pub fn attachments(world: &World, building: ActorId) -> u32 {
    let Some(o) = hq_org(world, building) else { return 0 };
    let members = &world.orgs[o as usize].members;
    world.orgs.iter().filter(|r| r.alive && r.hq.is_some_and(|h| h == building || members.contains(&h))).count() as u32
}

fn can_enroll(world: &World, building: ActorId, def: DefId) -> bool {
    if world.data.def(def).class != Some(DefClass::Organization) {
        return true;
    }
    let Some(o) = hq_org(world, building) else { return true };
    let max = world.data.def(world.orgs[o as usize].def).max_attachments.unwrap_or(u32::MAX);
    let reserved = world.books.org(o).map_or(0, |b| b.reserved_attachments);
    attachments(world, building) + reserved < max
}

fn can_afford(world: &World, kingdom: KingdomId, cost: &[f32]) -> bool {
    let Some(k) = world.books.kingdom(kingdom) else { return false };
    world
        .data
        .resources
        .iter()
        .enumerate()
        .all(|(i, r)| r.kind != ResourceKind::Stockpiled || k.stockpiled.get(i).copied().unwrap_or(0.0) >= cost.get(i).copied().unwrap_or(0.0))
}

fn credit(world: &mut World, kingdom: KingdomId, values: &[f32], sign: f32) {
    let n = world.data.resources.len();
    let max = world.data.rule_ci("EconomyStockpileResourceMax").unwrap_or(f32::MAX);
    let k = world.books.kingdom_mut(kingdom, n);
    for (s, v) in k.stockpiled.iter_mut().zip(values) {
        // Debits (0x4E1690) do not clamp; credits (0x4E39D0) clamp to [0, max].
        *s += sign * *v;
        if sign > 0.0 {
            *s = s.clamp(0.0, max);
        }
    }
    crate::economy::publish(world, kingdom);
}

/// True while a building is packing or unpacking (native disabled state).
pub fn busy(world: &World, building: ActorId) -> bool {
    world.books.actor(building).is_some_and(|b| b.deploy.is_some())
}

/// Apply a player or computer command; Err is the refusal reason.
pub fn command(world: &mut World, command: &Command) -> Result<(), String> {
    match *command {
        Command::Recruit { kingdom, building, def } => recruit(world, kingdom, building, def),
        Command::CancelRecruit { kingdom, building, index } => {
            let a = world.actor(building).ok_or("no such building")?;
            if a.kingdom != Some(kingdom) {
                return Err("not your building".into());
            }
            cancel(world, building, index).ok_or_else(|| "no such queue entry".to_string())
        }
        ref other => Err(format!("{other:?} is not a production command")),
    }
}

fn recruit(world: &mut World, kingdom: KingdomId, building: ActorId, def: DefId) -> Result<(), String> {
    let a = world.actor(building).ok_or("no such building")?;
    if !a.alive || a.kingdom != Some(kingdom) {
        return Err("not your building".into());
    }
    if (def as usize) >= world.data.defs.len() || !world.data.def(a.def).recruits.contains(&def) {
        return Err(format!("{} cannot recruit {}", world.data.def(a.def).id, world.data.defs.get(def as usize).map_or("?", |d| d.id.as_str())));
    }
    if busy(world, building) {
        return Err("the building is packing or unpacking".into());
    }
    if !can_enroll(world, building, def) {
        return Err("the HQ has no free attachment".into());
    }
    if world.data.def(def).recruitable == 0 {
        return Err("not recruitable".into());
    }
    let cost = recruit_cost(world, building, def);
    if !can_afford(world, kingdom, &cost) {
        world.books.notices.push(crate::books::Notice::Unaffordable { kingdom, building });
        return Err("not enough resources".into());
    }
    credit(world, kingdom, &cost, -1.0);
    let upkeep = quote(world, def, Vector::Upkeep);
    let requested_upkeep = limited_part(world, &upkeep);
    let attaches = world.data.def(def).class == Some(DefClass::Organization) && hq_org(world, building).is_some();
    if attaches && let Some(o) = hq_org(world, building) {
        world.books.org_mut(o).reserved_attachments += 1;
    }
    world.books.next_item += 1;
    let item = QueueItem {
        id: world.books.next_item,
        def,
        paid: cost,
        current_upkeep: vec![0.0; requested_upkeep.len()],
        requested_upkeep,
        attaches,
        research: None,
    };
    let queue = &mut world.books.actor_mut(building).queue;
    queue.items.push(item);
    if queue.items.len() == 1 {
        start_head(world, building);
    }
    mirror(world, building);
    Ok(())
}

/// Research request (0x584790): the building lists the item, the kingdom
/// neither owns nor already researches the technology (0x5844A0), the cost
/// is affordable (0x5845F0) and paid; the item joins the building's queue
/// (0x589F00) and takes research_time. Callable by the UI and the AI until a
/// `Command` variant routes it (see the core report).
pub fn research(world: &mut World, kingdom: KingdomId, building: ActorId, property: &str) -> Result<(), String> {
    let a = world.actor(building).ok_or("no such building")?;
    if !a.alive || a.kingdom != Some(kingdom) {
        return Err("not your building".into());
    }
    let Some(item) = world.data.def(a.def).research.iter().find(|r| r.property.eq_ignore_ascii_case(property)).cloned() else {
        return Err(format!("{} cannot research {property}", world.data.def(a.def).id));
    };
    if busy(world, building) {
        return Err("the building is packing or unpacking".into());
    }
    if world.books.kingdom(kingdom).is_some_and(|k| k.technologies.iter().any(|t| t.eq_ignore_ascii_case(property))) {
        return Err("already researched".into());
    }
    let queued = world.books.actors.iter().enumerate().any(|(i, b)| {
        world.actors.get(i).is_some_and(|x| x.kingdom == Some(kingdom) && !x.removed)
            && b.queue.items.iter().any(|it| it.research.as_deref().is_some_and(|r| r.eq_ignore_ascii_case(property)))
    });
    if queued {
        return Err("already being researched".into());
    }
    if !can_afford(world, kingdom, &item.cost) {
        world.books.notices.push(crate::books::Notice::Unaffordable { kingdom, building });
        return Err("not enough resources".into());
    }
    credit(world, kingdom, &item.cost, -1.0);
    world.books.next_item += 1;
    let n = world.data.resources.len();
    let it = QueueItem {
        id: world.books.next_item,
        def: world.actors[building as usize].def,
        paid: item.cost.clone(),
        requested_upkeep: vec![0.0; n],
        current_upkeep: vec![0.0; n],
        attaches: false,
        research: Some(item.property.clone()),
    };
    let queue = &mut world.books.actor_mut(building).queue;
    queue.items.push(it);
    if queue.items.len() == 1 {
        start_head(world, building);
    }
    mirror(world, building);
    Ok(())
}

/// HQ attach (0x574A10 gate, 0x574C10 attach, 0x5735D0 cost): attach an own
/// supply-using organization to the HQ of `hq` when that HQ has a free
/// attachment; the price is the organization cost times
/// HQAttachCostPercentage (0 in the shipped SVars).
pub fn attach(world: &mut World, kingdom: KingdomId, org: OrgId, hq: ActorId) -> Result<(), String> {
    let o = world.orgs.get(org as usize).filter(|o| o.alive && o.kingdom == kingdom).ok_or("not your organization")?;
    let od = world.data.def(o.def);
    if od.max_attachments.is_some() || !od.supply_used {
        return Err("this organization cannot attach to an HQ".into());
    }
    if o.hq == Some(hq) {
        return Ok(());
    }
    let h = world.actor(hq).ok_or("no such HQ")?;
    if !h.alive || h.kingdom != Some(kingdom) {
        return Err("not your HQ".into());
    }
    let Some(hq_org) = hq_org(world, hq) else {
        return Err("not an HQ".into());
    };
    let max = world.data.def(world.orgs[hq_org as usize].def).max_attachments.unwrap_or(0);
    let reserved = world.books.org(hq_org).map_or(0, |b| b.reserved_attachments);
    if attachments(world, hq) + reserved >= max {
        return Err("the HQ has no free attachment".into());
    }
    let factor = world.data.rule_ci("HQAttachCostPercentage").unwrap_or(0.0);
    let cost: Vec<f32> = quote(world, o.def, Vector::Cost).iter().map(|c| c * factor).collect();
    if !can_afford(world, kingdom, &cost) {
        return Err("not enough resources".into());
    }
    if cost.iter().any(|c| *c != 0.0) {
        credit(world, kingdom, &cost, -1.0);
    }
    world.orgs[org as usize].hq = Some(hq);
    world.books.notices.push(crate::books::Notice::Attached { org, hq });
    Ok(())
}

/// 0x588D60: organization recruit_time, else 1 / ElementComponent
/// resupply_rate (+2CC), else half a tick (instant).
pub fn duration(world: &World, def: DefId) -> f32 {
    let d = world.data.def(def);
    if d.class == Some(DefClass::Organization) {
        d.recruit_time
    } else if d.resupply_rate > 0.0 {
        1.0 / d.resupply_rate
    } else {
        crate::DT * 0.5
    }
}

/// 0x588D60 for a queue item: research_time for research, else `duration`.
pub fn item_duration(world: &World, building: ActorId, item: &QueueItem) -> f32 {
    match &item.research {
        Some(p) => world
            .actor(building)
            .and_then(|a| world.data.def(a.def).research.iter().find(|r| r.property.eq_ignore_ascii_case(p)))
            .map_or(crate::DT * 0.5, |r| r.research_time),
        None => duration(world, item.def),
    }
}

fn start_head(world: &mut World, building: ActorId) {
    let now = world.time;
    let head = world.books.actor(building).and_then(|b| b.queue.items.first().cloned());
    let duration = head.as_ref().map_or(0.0, |i| item_duration(world, building, i));
    let q = &mut world.books.actor_mut(building).queue;
    q.paused_at = None;
    q.pause_reason = None;
    if head.is_none() {
        q.deadline = 0.0;
        q.duration = 0.0;
        // cancel_schedule.
        q.scheduled = false;
        return;
    }
    q.duration = duration;
    q.deadline = now + duration;
    process(world, building);
    // 0x589420 tail: a queue not yet scheduled registers on the periodic
    // queue 0 (0x50AC00(0) -> 0x50B2C0), whose bucket is one world-RNG draw
    // (GetRNG 0x4F02B0 at 0x50B2DF, draw % 1 bucket).
    if world.books.actor(building).is_some_and(|b| !b.queue.scheduled) {
        world.random_u32();
        world.books.actor_mut(building).queue.scheduled = true;
    }
}

/// 0x5894C0: take an item out, releasing its reserved upkeep and HQ attachment.
fn remove(world: &mut World, building: ActorId, index: usize) -> Option<QueueItem> {
    let q = &mut world.books.actor_mut(building).queue;
    if index >= q.items.len() {
        return None;
    }
    let mut item = q.items.remove(index);
    if index == 0 {
        q.paused_at = None;
        q.pause_reason = None;
        // cancel_schedule: the next head registers (and draws) again.
        q.scheduled = false;
    }
    // adjust_upkeep(current, false) (0x4E48C0, also for a zero vector).
    if let Some(k) = world.actors.get(building as usize).and_then(|a| a.kingdom) {
        crate::economy::adjust_consumption(world, k, &item.current_upkeep, false);
    }
    item.current_upkeep.iter_mut().for_each(|v| *v = 0.0);
    if item.attaches
        && let Some(o) = hq_org(world, building)
    {
        let b = world.books.org_mut(o);
        b.reserved_attachments = b.reserved_attachments.saturating_sub(1);
    }
    Some(item)
}

/// 0x589BD0 cancel with refund, then the caller restarts the head.
pub fn cancel(world: &mut World, building: ActorId, index: usize) -> Option<()> {
    let item = remove(world, building, index)?;
    if let Some(k) = world.actors[building as usize].kingdom {
        credit(world, k, &item.paid, 1.0);
    }
    if index == 0 {
        start_head(world, building);
    }
    mirror(world, building);
    Some(())
}

/// Cancel (refund) every queued item of a building.
pub fn cancel_all(world: &mut World, building: ActorId) {
    while world.books.actor(building).is_some_and(|b| !b.queue.items.is_empty()) {
        let n = world.books.actor(building).map_or(0, |b| b.queue.items.len());
        if cancel(world, building, n - 1).is_none() {
            break;
        }
    }
}

fn can_reserve(world: &World, kingdom: KingdomId, item: &QueueItem) -> bool {
    let Some(k) = world.books.kingdom(kingdom) else { return false };
    world.data.resources.iter().enumerate().all(|(i, r)| {
        let need = item.requested_upkeep.get(i).copied().unwrap_or(0.0) - item.current_upkeep.get(i).copied().unwrap_or(0.0);
        if r.kind != ResourceKind::LimitedConsumed || need <= 0.0 {
            return true;
        }
        let Some(p) = r.limited_by else { return true };
        k.consumed.get(i).copied().unwrap_or(0.0) + need <= k.provided.get(p).copied().unwrap_or(0.0)
    })
}

/// 0x589F90: completion first, then the reservation, disabled and supply
/// pauses; a paused head's deadline moves on with the paused time.
fn process(world: &mut World, building: ActorId) {
    let now = world.time;
    let Some(item) = world.books.actor(building).and_then(|b| b.queue.items.first().cloned()) else {
        return;
    };
    let deadline = world.books.actor(building).map_or(0.0, |b| b.queue.deadline);
    let kingdom = world.actors[building as usize].kingdom;
    if now >= deadline {
        let item = remove(world, building, 0).expect("head");
        if !fulfill(world, building, &item)
            && let Some(k) = kingdom
        {
            credit(world, k, &item.paid, 1.0);
        }
        start_head(world, building);
        return;
    }
    let mut reason = None;
    let notify_first = world.books.actor(building).is_some_and(|b| b.queue.paused_at.is_none());
    if item.current_upkeep != item.requested_upkeep {
        match kingdom {
            Some(k) if can_reserve(world, k, &item) => {
                // 0x589F90: current = the limited upkeep, which adjust_upkeep
                // adds whole (0x4E48C0).
                let head = &mut world.books.actor_mut(building).queue.items[0];
                head.current_upkeep = head.requested_upkeep.clone();
                let reserved = head.current_upkeep.clone();
                crate::economy::adjust_consumption(world, k, &reserved, true);
                crate::economy::publish(world, k);
            }
            _ => reason = Some(PauseReason::LimitedResource),
        }
    }
    if reason.is_none() {
        let def = world.data.def(world.actors[building as usize].def);
        if busy(world, building) {
            reason = Some(PauseReason::Disabled);
        } else if def.supply_required && world.books.out_of_supply(building) {
            reason = Some(PauseReason::Supply);
        } else {
            let q = &mut world.books.actor_mut(building).queue;
            q.paused_at = None;
            q.pause_reason = None;
            return;
        }
    }
    let q = &mut world.books.actor_mut(building).queue;
    let changed = q.pause_reason != reason;
    if let Some(at) = q.paused_at {
        q.deadline += now - at;
    }
    q.paused_at = Some(now);
    q.pause_reason = reason;
    if changed && (notify_first || reason == Some(PauseReason::LimitedResource)) {
        world.books.notices.push(crate::books::Notice::ProductionPaused { building, reason: reason.unwrap() });
    }
}

/// Recruit completion: spawn at the DeploymentInfo creation point turned by
/// the building facing and walk to the deployment point (0x4FACA0).
/// Research completion (0x584120): the kingdom gains the technology.
fn fulfill(world: &mut World, building: ActorId, item: &QueueItem) -> bool {
    let b = &world.actors[building as usize];
    let Some(kingdom) = b.kingdom else { return false };
    if let Some(property) = &item.research {
        let n = world.data.resources.len();
        let k = world.books.kingdom_mut(kingdom, n);
        if !k.technologies.iter().any(|t| t.eq_ignore_ascii_case(property)) {
            k.technologies.push(property.clone());
        }
        world.books.notices.push(crate::books::Notice::Researched { kingdom, property: property.clone() });
        return true;
    }
    let bdef = world.data.def(b.def);
    let origin = b.pos.truncate();
    let facing = b.facing;
    let creation = bdef.creation_offset.unwrap_or([0.0, 0.0]);
    let deployment = bdef.deployment_offset.unwrap_or(creation);
    let start = origin + rotate(creation, facing);
    let end = origin + rotate(deployment, facing);
    let heading = angle_of(end - start).unwrap_or(facing);
    let def = item.def;
    let rally = crate::player_orders::rally_point(world, building);
    let (org, actor) = if world.data.def(def).class == Some(DefClass::Organization) {
        // 582E80 organization branch: the organization actor stands at the
        // structure's deployment point (ActorData +3E4/+3E8 turned by its
        // facing) facing the structure's way; its members come through
        // 4FBF40, i.e. out of this structure's door. A supply_used
        // organization without recruit_full gets only its leader slot now
        // (ActorData +2F4 / +31C); regeneration 57B5A0 rebuilds the rest one
        // at a time.
        let hq = item.attaches.then_some(building);
        let d = world.data.def(def);
        let all = !d.supply_used || d.recruit_full;
        let leader = d.leader_slot.map(usize::from).filter(|&l| l < d.slots.len());
        // 554BF0 for the leader slot's shape: from the deployment point back
        // toward the structure to the first open spot, then rings around it
        // (radius: the structure's shape plus the organization's), so a new
        // organization does not stand on the ones already there.
        let leader_def = leader.map(|l| d.slots[l].0).filter(|&x| x != crate::data::NO_DEF).or_else(|| d.slots.iter().map(|s| s.0).find(|&x| x != crate::data::NO_DEF));
        let anchor = match leader_def {
            Some(ld) => {
                let radius = shape_radius(world.data.def(world.actors[building as usize].def)) + shape_radius(d);
                open_position(world, ld, end, origin, Vec2::new(facing.cos(), facing.sin()), radius)
            }
            None => end,
        };
        let o = new_organization(world, def, kingdom, anchor, facing, hq);
        let d = world.data.def(def);
        let slots: Vec<usize> = (0..d.slots.len()).filter(|&s| d.slots[s].0 != crate::data::NO_DEF && (all || Some(s) == leader)).collect();
        // The layout's morale (as a full organization) from the start.
        let morale: f32 = d.slots.iter().filter(|s| s.0 != crate::data::NO_DEF).map(|s| world.data.def(s.0).morale_value).sum();
        world.orgs[o as usize].morale = morale;
        for s in slots {
            spawn_member(world, o, s);
        }
        if world.orgs[o as usize].members.is_empty() {
            // 57A730 == 0: "failed to create any elements - destroying".
            world.orgs[o as usize].alive = false;
            return false;
        }
        // The members walk out of the door to the deployment point, where the
        // organization stands (4FACA0 mind order, walk_member), then take
        // their slots. A rally point (4D3D20 after a 554500 spot) gives the
        // organization a move that replaces those walks.
        if let Some(to) = rally {
            let _ = crate::movement::command(world, &Command::Move { kingdom, units: vec![crate::world::Unit::Org(o)], to, attack_move: false });
        }
        (Some(o), None)
    } else {
        // 4FACA0: created at the creation point facing the deployment point,
        // the mind walks it there; then the rally point, if any.
        let a = spawn_single(world, def, Some(kingdom), start, heading);
        if let Some(to) = rally.or((end != start).then_some(end)) {
            let _ = crate::movement::command(world, &Command::Move { kingdom, units: vec![crate::world::Unit::Actor(a)], to, attack_move: false });
        }
        (None, Some(a))
    };
    world.events.push(Event::Recruited { kingdom, org, actor });
    true
}

/// Keep `Actor::queue` (def, seconds left) in step with the books for the HUD.
fn mirror(world: &mut World, building: ActorId) {
    let now = world.time;
    let Some(b) = world.books.actor(building) else { return };
    let q: &Queue = &b.queue;
    let list: Vec<(DefId, f32)> = q
        .items
        .iter()
        .enumerate()
        .map(|(i, it)| (it.def, if i == 0 { (q.deadline - q.paused_at.unwrap_or(now)).max(0.0) } else { item_duration(world, building, it) }))
        .collect();
    if let Some(a) = world.actors.get_mut(building as usize) {
        a.queue = list;
    }
}

pub fn tick(world: &mut World) {
    let n = world.books.actors.len().min(world.actors.len());
    for id in 0..n as ActorId {
        if world.books.actors[id as usize].queue.items.is_empty() {
            continue;
        }
        let a = &world.actors[id as usize];
        if a.removed || !a.alive || a.kingdom.is_none() {
            cancel_all(world, id);
            continue;
        }
        process(world, id);
        mirror(world, id);
    }
}
