//! Trucks driving to a spot and unpacking into their building; packing
//! buildings back into trucks. Owner: core.
//!
//! Behaviour-faithful: a truck's MobileDeploymentComponent names the
//! building it unpacks into (unpack_to_actor_ids, ActorData+534) and the
//! building names its truck through the inverse link written by 4605B0
//! (ActorData+548). Command::Deploy gives the truck (or its HQ organization)
//! Order::Deploy; the movement owner drives it there. Once it stands at the
//! spot the truck is replaced by the building, which plays its unpacking for
//! the building's unpacking_length and is disabled meanwhile (no production,
//! no supply zone, paused queue). Command::Pack runs packing_length and
//! replaces the building by its truck at packed_position_offset turned by
//! the building facing, facing packed_orientation (turned the same way).
//! Packing needs pack_health_min of the building's health.
use crate::books::{DeployKind, Deployment, Notice};
use crate::data::DefId;
use crate::world::{Activity, ActorId, Command, Event, Order, World};
use glam::Vec2;

fn live(world: &World, a: ActorId) -> bool {
    world.actors.get(a as usize).is_some_and(|x| x.alive && !x.removed)
}

/// Apply a player or computer command; Err is the refusal reason.
pub fn command(world: &mut World, command: &Command) -> Result<(), String> {
    match *command {
        Command::Deploy { kingdom, truck, at, facing } => {
            if !live(world, truck) || world.actors[truck as usize].kingdom != Some(kingdom) {
                return Err("not your truck".into());
            }
            if world.data.def(world.actors[truck as usize].def).unpacks_to.is_none() {
                return Err("this unit cannot deploy".into());
            }
            if world.books.actor(truck).is_some_and(|b| b.deploy.is_some()) {
                return Err("already deploying".into());
            }
            let building = world.data.def(world.actors[truck as usize].def).unpacks_to.expect("checked");
            let (at, facing) = placement(world, building, at, facing);
            spot_clear(world, building, at, Some(truck))?;
            // The truck must be able to stand there and reach it (movement's check).
            if let Some(problem) = crate::movement::deploy_spot_problem(world, truck, at) {
                return Err(problem);
            }
            let order = Order::Deploy { at, facing };
            world.actors[truck as usize].order = order.clone();
            if let Some(o) = world.actors[truck as usize].org {
                world.orgs[o as usize].order = order;
            }
            Ok(())
        }
        Command::Pack { kingdom, building } => {
            if !live(world, building) || world.actors[building as usize].kingdom != Some(kingdom) {
                return Err("not your building".into());
            }
            let def = world.data.def(world.actors[building as usize].def);
            let Some(truck) = def.packs_to else {
                return Err("this building cannot pack".into());
            };
            if world.books.actor(building).is_some_and(|b| b.deploy.is_some()) {
                return Err("already packing or unpacking".into());
            }
            // 5763F0: the health ratio must be strictly above pack_health_min
            // (mobile_deployment_error_pack_health).
            if def.health > 0.0 && world.actors[building as usize].health / def.health <= def.pack_health_min {
                return Err("too damaged to pack".into());
            }
            let length = def.packing_length;
            crate::production::cancel_all(world, building);
            let now = world.time;
            world.books.actor_mut(building).deploy = Some(Deployment { kind: DeployKind::Packing, into: truck, started: now, done_at: now + length });
            world.set_activity(building, Activity::Packing);
            Ok(())
        }
        ref other => Err(format!("{other:?} is not a deployment command")),
    }
}

/// 0x6939B0 placement adjust for the building a truck unpacks into: a
/// tile_locked building snaps to the half-cell grid (0x46EE00:
/// floor(x / (cell * 0.5) + 0.5) * cell * 0.5), an axis_locked one faces
/// native -Y.
pub fn placement(world: &World, building: DefId, at: Vec2, facing: f32) -> (Vec2, f32) {
    let d = world.data.def(building);
    let mut at = at;
    if d.tile_locked && d.blocking_size.is_some() {
        let half = world.map.cell_size * 0.5;
        at = Vec2::new((at.x / half + 0.5).floor() * half, (at.y / half + 0.5).floor() * half);
    }
    let facing = if d.axis_locked { -std::f32::consts::FRAC_PI_2 } else { facing };
    (at, facing)
}

/// Whether the building's blocking footprint at `at` fits: inside the map,
/// on land-passable unblocked cells and clear of every other building's
/// footprint (`ignore` is the deploying truck itself).
pub fn spot_clear(world: &World, building: DefId, at: Vec2, ignore: Option<ActorId>) -> Result<(), String> {
    let d = world.data.def(building);
    let half = d.blocking_size.map_or(Vec2::ZERO, |s| Vec2::new(s[0], s[1]) * 0.5);
    let (lo, hi) = (at - half, at + half);
    let map = &world.map;
    if map.width > 0 {
        let extent = Vec2::new(map.width as f32, map.height as f32) * map.cell_size;
        if lo.x < 0.0 || lo.y < 0.0 || hi.x > extent.x || hi.y > extent.y {
            return Err("the spot is outside the map".into());
        }
        let c = map.cell_size;
        let (x0, y0) = ((lo.x / c).floor() as u32, (lo.y / c).floor() as u32);
        let (x1, y1) = (((hi.x / c).ceil() as u32).max(x0 + 1).min(map.width), ((hi.y / c).ceil() as u32).max(y0 + 1).min(map.height));
        for y in y0..y1 {
            for x in x0..x1 {
                let f = map.flags[(y * map.width + x) as usize];
                if f & (crate::map::BLOCKED | crate::map::NOT_LAND_PASSABLE) != 0 {
                    return Err("the ground there cannot hold the building".into());
                }
            }
        }
    }
    for a in &world.actors {
        if a.removed || !a.alive || Some(a.id) == ignore {
            continue;
        }
        let od = world.data.def(a.def);
        if od.class != Some(crate::data::DefClass::Building) {
            continue;
        }
        let Some(s) = od.blocking_size else { continue };
        let oh = Vec2::new(s[0], s[1]) * 0.5;
        let p = a.pos.truncate();
        if lo.x < p.x + oh.x && hi.x > p.x - oh.x && lo.y < p.y + oh.y && hi.y > p.y - oh.y {
            return Err("another building is in the way".into());
        }
    }
    Ok(())
}

/// The truck stands at its deploy spot: the movement owner's arrival test
/// (0.5 m, facing reached) or, for an HQ organization's truck, stopped within
/// one map cell of the spot.
fn arrived(world: &World, truck: ActorId) -> bool {
    let a = &world.actors[truck as usize];
    let Order::Deploy { at, .. } = a.order else { return false };
    if crate::movement::deploy_ready(world, truck) {
        return true;
    }
    let near = (a.pos.truncate() - at).length() <= world.map.cell_size.max(1.0);
    near && a.motion.path.is_empty() && !crate::movement::actor_moving(world, truck)
}

/// Replace `old` by a new actor of `def` at `pos`/`facing`, keeping its
/// kingdom, organization slot, HQ links and queue owner identity.
fn replace(world: &mut World, old: ActorId, def: DefId, pos: Vec2, facing: f32) -> ActorId {
    let kingdom = world.actors[old as usize].kingdom;
    let (org, slot) = (world.actors[old as usize].org, world.actors[old as usize].slot);
    let new = crate::production::spawn_single(world, def, kingdom, pos, facing);
    {
        let n = &mut world.actors[new as usize];
        n.org = org;
        n.slot = slot;
    }
    if let Some(o) = org {
        for m in world.orgs[o as usize].members.iter_mut() {
            if *m == old {
                *m = new;
            }
        }
        world.orgs[o as usize].order = Order::Idle;
    }
    for r in world.orgs.iter_mut() {
        if r.hq == Some(old) {
            r.hq = Some(new);
        }
    }
    let placed = world.books.actor(old).and_then(|b| b.placed);
    world.books.actor_mut(new).placed = placed;
    let o = &mut world.actors[old as usize];
    o.removed = true;
    o.order = Order::Idle;
    world.books.notices.push(Notice::Replaced { old, new });
    new
}

pub fn tick(world: &mut World) {
    let now = world.time;
    let n = world.actors.len();
    for a in 0..n as ActorId {
        if !live(world, a) {
            continue;
        }
        // Finish a running pack/unpack.
        if let Some(d) = world.books.actor(a).and_then(|b| b.deploy.clone()) {
            if now < d.done_at {
                continue;
            }
            match d.kind {
                DeployKind::Unpacking => {
                    world.books.actor_mut(a).deploy = None;
                    world.set_activity(a, Activity::Idle);
                    world.events.push(Event::Deployed { building: a });
                }
                DeployKind::Packing => {
                    let b = &world.actors[a as usize];
                    let bdef = world.data.def(b.def);
                    let truck_def = world.data.def(d.into);
                    let (p, f) = (b.pos.truncate(), b.facing);
                    let pos = p + crate::production::rotate(truck_def.packed_position_offset, f);
                    let dir = crate::production::rotate(truck_def.packed_orientation, f);
                    let facing = if dir.length_squared() > 1e-8 { dir.y.atan2(dir.x) } else { f };
                    let _ = bdef;
                    world.books.actor_mut(a).deploy = None;
                    let truck = replace(world, a, d.into, pos, facing);
                    world.books.notices.push(Notice::Packed { truck });
                }
            }
            continue;
        }
        // Start unpacking a truck that reached its spot.
        let actor = &world.actors[a as usize];
        let Order::Deploy { at, facing } = actor.order else { continue };
        let Some(building) = world.data.def(actor.def).unpacks_to else { continue };
        if !arrived(world, a) {
            continue;
        }
        let length = world.data.def(building).unpacking_length;
        let b = replace(world, a, building, at, facing);
        world.books.actor_mut(b).deploy = Some(Deployment { kind: DeployKind::Unpacking, into: building, started: now, done_at: now + length });
        world.set_activity(b, Activity::Unpacking);
    }
}
