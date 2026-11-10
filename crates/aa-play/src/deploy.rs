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
            // The order carries the given point: the truck drives to it and the
            // building snaps only when it is made (0x575C80 -> 0x4F3640). The
            // native refuses a spot then, not here (tai_error_unpack).
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

/// `0x530960`'s default: an order with no direction words hands (0, -1) to its state.
const NO_DIRECTION: Vec2 = Vec2::new(0.0, -1.0);
/// The native x87 precision of the simulation thread.
const PC: aa_numeric::Precision = aa_numeric::Precision::Double53;
/// The Thing +8 flags the unpack query requires of a thing (descriptor +0x38): `0x4F3640`
/// sets bit 29 because the placement record names the truck (+0x3C, its GTID) and bit 10
/// always, so flags 10 and 29 whatever the building's own flags.
const UNPACK_REQUIRED: u32 = 0x2000_0400;
/// Thing flag bit 18, `[Flags] ethereal`: with it `0x4F3640` skips the tile and thing query.
const ETHEREAL: u32 = 0x4_0000;

/// A thing of the tile lists (GWorld+0x30) as `0x558280` mode 2 reads it.
#[derive(Clone, Debug)]
struct Listed {
    /// Thing +8.
    flags: u32,
    /// Actor virtual +0xC: the thing's own blocking shape, unpadded.
    blocking: aa_spatial::collision::Shape,
    /// The shape whose tile rectangle lists the thing in its cells (`0x5539B0`).
    soft: aa_spatial::collision::Shape,
    /// Thing +0x20/+0x24.
    position: [u32; 2],
    /// Thing +0x2C/+0x30.
    facing: [u32; 2],
}

/// `0x558280` mode 2 for one thing listed in the cells `0x5534F0` gives the query
/// (`cells`, None without a map): the thing must be listed in one of them, carry the
/// required flags, and `0x46EF30` must find our padded shape over `0x4CF210`'s
/// descriptor of it, its own unpadded shape at its position and facing. `0x4EFB20`
/// (each thing once) and the order of the walk do not change whether one hits. The
/// `0x552340` exclusions are the caller's.
fn listed_in_the_way(placed: &aa_spatial::geometry::Placement, cells: Option<(&aa_spatial::collision::Grid, aa_spatial::geometry::TileRect)>, thing: &Listed) -> bool {
    use aa_spatial::geometry::{tile_rectangle, Placement};
    if thing.flags & UNPACK_REQUIRED != UNPACK_REQUIRED {
        return false;
    }
    if let Some((grid, query)) = cells {
        let listing = Placement { shape: thing.soft.clone(), facing: thing.facing, position: thing.position };
        if let Ok(own) = tile_rectangle(grid, &listing, PC) {
            if (0..2).any(|a| own.max[a] < query.min[a] || own.min[a] > query.max[a]) {
                return false;
            }
        }
    }
    let other = Placement { shape: thing.blocking.clone(), facing: thing.facing, position: thing.position };
    aa_spatial::shape_overlap::overlaps(placed, &other, PC).unwrap_or(true)
}

/// Descriptor +0x24 as `0x4F3640` fills it: the definition's ActorData +0x4E0 when it is
/// non-null; `0x552340` then leaves out every thing of that definition.
fn required_definition(world: &World, def: &crate::data::Def) -> Option<DefId> {
    def.required_object.as_deref().and_then(|id| world.data.find(id))
}

fn shape_of(words: [u32; 6]) -> aa_spatial::collision::Shape {
    aa_spatial::collision::Shape { kind: words[0], radius_bits: words[1], bounds: [words[2], words[3], words[4], words[5]] }
}
/// Definition virtual +0x28 `0x49BD10`: the blocking shape, for a blocking static thing
/// (flags 0x20000400) grown by `BuildingPaddingMeters` (GameData +0x44): a circle's
/// radius, each half extent of a box, rebuilt as a rectangle. The walls `0x49BD10` leaves
/// alone have no truck.
fn padded_shape(world: &World, def: &crate::data::Def) -> aa_spatial::collision::Shape {
    let clearance = f64::from(world.data.rule_ci("BuildingPaddingMeters").unwrap_or(2.0));
    pad_shape(shape_of(def.blocking_words), def.thing_flags, clearance)
}

fn pad_shape(original: aa_spatial::collision::Shape, thing_flags: u32, clearance: f64) -> aa_spatial::collision::Shape {
    if thing_flags & 0x2000_0400 != 0x2000_0400 || original.kind > 3 {
        return original;
    }
    if original.kind <= 1 {
        let radius = (clearance + f64::from(f32::from_bits(original.radius_bits))) as f32;
        return aa_spatial::collision::Shape { kind: 1, radius_bits: radius.to_bits(), bounds: [0; 4] };
    }
    let mut dims = [0u32; 2];
    for a in [1usize, 0] {
        let half = f64::from(f32::from_bits(original.bounds[a + 2])) + clearance;
        dims[a] = ((half + half) as f32).to_bits();
    }
    aa_rules::objects::shape(3, &dims, PC).map_or(original, |s| shape_of(s.words))
}

/// Whether the building's blocking footprint at `at` fits: inside the map,
/// on land-passable unblocked cells and clear of every other building's
/// footprint (`ignore` is the deploying truck itself). Rendition, for a construct item's
/// ghost, a construction and a computer's site; the unpack itself is [`spot_clear_facing`],
/// the Unpack ghost [`ghost_spot_clear`]. Not ported: the original's ghost (`0x693EF0` ->
/// `0x6215D0`) and site search (`0x5553E0`) run `0x4F3640` on a record without a GTID, which
/// takes the padded shape (unless GWorld +0x28 +0xA8 is set), requires only Thing flag 10, and
/// first tests `0x4F1660` (code 5); BuildActor `0x563160` reaches no `0x4F3640` call directly.
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

/// `0x4F3640` as `0x575C80` calls it for the building a truck unpacks into: definition virtual
/// +0x28's shape at the snapped point. Outside the map is 3; then `0x4F0A70` -> `0x553BA0`:
/// a tile under the shape with a flag of mask +0x3C, or the blocking shape of a thing with
/// Thing flags 10 and 29, a structure or an object of the map, that overlaps it. A building
/// that fits here can still be 2 m from water: the padding is what refuses it. Rendition:
/// codes 4 and 7 read a settlement's fields no truck's building has.
///
/// `0x4F3640` runs the `0x4F0A70` query only when the definition's flag bit 18 (ethereal,
/// 0x40000) is clear, the record's byte +0x52 is 0 and its fourth argument is null or lacks
/// Thing flag bit 12. `0x575C80` zeroes +0x52 (its local at record +0x52) and passes 0, so
/// only bit 18 can skip it here: an ethereal building is then refused only outside the map.
pub fn spot_clear_facing(world: &World, building: DefId, at: Vec2, facing: Vec2, ignore: Option<ActorId>) -> Result<(), String> {
    replacing_query(world, building, at, facing, ignore, true)
}

/// `0x4F3640` on the Unpack ghost's record: `0x693EF0` builds it with kind +0x38 = 0 and
/// `0x6215D0` gives it the truck's GTID (+0x3C) before the call. With +0x3C set and a kind
/// other than 3, `0x4F370D` takes the definition's ObjectData +0x38 shape, the blocking shape
/// without `BuildingPaddingMeters`; the rest is the unpack's query ([`spot_clear_facing`]).
/// So the original's ghost can show a spot as legal that the unpack, padded, then refuses.
pub fn ghost_spot_clear(world: &World, building: DefId, at: Vec2, facing: Vec2, truck: Option<ActorId>) -> Result<(), String> {
    replacing_query(world, building, at, facing, truck, false)
}

/// `0x4F3640` for a record that names the actor it replaces (+0x3C), so the query is strict
/// (descriptor flags 0x20000400): `unpack` is the record kind 3 of `0x575C80`.
fn replacing_query(world: &World, building: DefId, at: Vec2, facing: Vec2, ignore: Option<ActorId>, unpack: bool) -> Result<(), String> {
    use aa_spatial::geometry::Placement;
    use aa_spatial::shape_overlap::overlaps;
    let d = world.data.def(building);
    let shape = if unpack { padded_shape(world, d) } else { shape_of(d.blocking_words) };
    let bits = |v: Vec2| [v.x.to_bits(), v.y.to_bits()];
    let placed = Placement { shape, facing: bits(facing), position: bits(at) };
    let map = &world.map;
    let reach = aa_spatial::geometry::world_bounds(&placed, PC).map_or(0.0, |b| {
        let (x, y) = (f32::from_bits(b[2]) - f32::from_bits(b[0]), f32::from_bits(b[3]) - f32::from_bits(b[1]));
        0.5 * x.abs().max(y.abs())
    });
    if map.width > 0 {
        let extent = Vec2::new(map.width as f32, map.height as f32) * map.cell_size;
        if aa_spatial::query::outside(&placed, bits(extent), PC).unwrap_or(true) {
            return Err("the spot is outside the map".into());
        }
    }
    if d.thing_flags & ETHEREAL != 0 {
        return Ok(());
    }
    if map.width > 0 {
        let c = map.cell_size;
        let half = c * 0.5;
        // The cell's own square, GMap +0x30 as `0x556689` makes it (`QueryGeometry`).
        let corner = (f64::from(half) * f64::from(half) * 2.0).sqrt() as f32;
        let tile_shape = aa_spatial::collision::Shape { kind: 2, radius_bits: corner.to_bits(), bounds: [(-half).to_bits(), (-half).to_bits(), half.to_bits(), half.to_bits()] };
        let (lo, hi) = (at - Vec2::splat(reach), at + Vec2::splat(reach));
        let (x0, y0) = ((lo.x / c).floor().max(0.0) as u32, (lo.y / c).floor().max(0.0) as u32);
        let (x1, y1) = (((hi.x / c).floor() as u32).min(map.width - 1), ((hi.y / c).floor() as u32).min(map.height - 1));
        let mask = if d.thing_flags & (1 << 21) != 0 { crate::map::NOT_WATER_PASSABLE } else { crate::map::NOT_LAND_PASSABLE };
        for y in y0..=y1 {
            for x in x0..=x1 {
                if map.flags[(y * map.width + x) as usize] & mask == 0 {
                    continue;
                }
                let tile = Placement { shape: tile_shape.clone(), facing: bits(NO_DIRECTION), position: bits(Vec2::new(x as f32 * c + half, y as f32 * c + half)) };
                if overlaps(&tile, &placed, PC).unwrap_or(true) {
                    return Err("the ground there cannot hold the building".into());
                }
            }
        }
    }
    // The things: `0x558280` mode 2 over the cells under the query's shape. The `0x552340`
    // exclusions of this descriptor: the truck (+0x20); a thing of the building's
    // `required_object_ids` definition (+0x24, ActorData +0x4E0, [`required_definition`]);
    // no source actor (+0x1C) and no filter (+0x34). Its hostility test (+0x2C) only reads
    // things without flag 29, which the required flags already leave out, and its
    // `settlement_wall` test (ActorData +0x1A8) only applies when the building lacks flags
    // 0x20000400, which a building with a truck has (`padded_shape`).
    let required = required_definition(world, d);
    let grid = (map.width > 0 && map.cell_size > 0.0).then(|| aa_spatial::collision::Grid {
        dimensions: [map.width as i32, map.height as i32],
        coordinate_shift: (map.cell_size as u32).trailing_zeros(),
        cell_size_bits: map.cell_size.to_bits(),
    });
    let query = grid.as_ref().and_then(|g| aa_spatial::geometry::tile_rectangle(g, &placed, PC).ok().map(|r| (g, r)));
    // A thing whose shapes reach no cell of the query cannot be listed there (f64, wide).
    let margin = f64::from(reach) + 2.0 * f64::from(map.cell_size.max(1.0)) + 1.0;
    let far = |position: Vec2, a: &[u32; 6], b: &[u32; 6]| {
        let r = f64::from(f32::from_bits(a[1]).abs().max(f32::from_bits(b[1]).abs()));
        let d = (position - at).abs();
        r.is_finite() && f64::from(d.x.max(d.y)) > margin + r
    };
    for a in &world.actors {
        if a.removed || !a.alive || Some(a.id) == ignore || Some(a.def) == required {
            continue;
        }
        let od = world.data.def(a.def);
        let position = a.pos.truncate();
        if far(position, &od.blocking_words, &od.soft_shape) {
            continue;
        }
        // Rendition: Thing +8 as the definition leaves it; a building's facing words from
        // its angle (aa-play keeps no others for a thing without a Mover).
        let thing = Listed {
            flags: od.thing_flags,
            blocking: shape_of(od.blocking_words),
            soft: shape_of(od.soft_shape),
            position: bits(position),
            // an axis_locked thing stands on the exact (0, -1), which its angle
            // cannot hold (ground::facing_words).
            facing: bits(if od.axis_locked { NO_DIRECTION } else { crate::formation::facing_vector(a.facing) }),
        };
        if listed_in_the_way(&placed, query, &thing) {
            return Err("another building is in the way".into());
        }
    }
    for &index in &world.books.scenery {
        let Some(p) = map.placed.get(index) else { continue };
        let position = Vec2::new(p.position[0], p.position[1]);
        let Some(thing) = world.data.find(&p.thing_id) else { continue };
        if Some(thing) == required {
            continue;
        }
        let od = world.data.def(thing);
        if far(position, &od.blocking_words, &od.soft_shape) {
            continue;
        }
        let thing = Listed {
            flags: od.thing_flags,
            blocking: shape_of(od.blocking_words),
            soft: shape_of(od.soft_shape),
            position: bits(position),
            facing: bits(Vec2::new(p.facing[0], p.facing[1])),
        };
        if listed_in_the_way(&placed, query, &thing) {
            return Err("an object of the map is in the way".into());
        }
    }
    Ok(())
}


/// Replace `old` by a new actor of `def` at `pos`/`facing`, keeping its
/// kingdom, organization slot, HQ links and queue owner identity.
fn replace(world: &mut World, old: ActorId, def: DefId, pos: Vec2, facing: f32) -> ActorId {
    let kingdom = world.actors[old as usize].kingdom;
    let (org, slot) = (world.actors[old as usize].org, world.actors[old as usize].slot);
    // The new actor takes the old one's GTID (the film: the unpacked HQ keeps
    // its truck's id); the old one's trailer is destroyed with it.
    world.ids.replace_actor(old);
    let new = crate::production::spawn_single(world, def, kingdom, pos, facing);
    world.ids.drop_request();
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
        // 0x4D6174 -> 0x57D8A0 (movement owner): the new actor is the
        // organization's member (and leader) in the old one's place, and the
        // goal follows it (0x57C800 -> 0x57C680), the regiment's unpack state
        // still current.
        crate::movement::member_replaced(world, o, old, new);
        // (combat) 0x4F6760 destroys the old thing: 0x57CD70's morale loss.
        let old_def = world.actors[old as usize].def;
        crate::combat::member_replaced(world, o, old_def);
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
                    // The deferred end of the unpacking makes the components left
                    // out at creation (0x576319 -> 0x4D21A0(1)): queue 1 for each,
                    // and the Supply's first 0x58AE10.
                    crate::creation::unpack_end(world, a);
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
    }
}

/// `0x575C80` from `KKC_UAIStateUnpack` 0x5372B0: the building is created in
/// place of the truck, snapped there (0x4F3640, [`placement`]); its Supply waits
/// for the end of the unpacking (crate::creation). A spot the building cannot stand on:
/// the truck's state has ended (0x5372B0's +0x1B0 either way). A truck with no organization
/// reports `tai_error_unpack` itself and the order ends. A regiment's unit says nothing: its
/// regiment's Unpack `0x526E60` sees it has not unpacked, sends the action again, thinks
/// in a second, and after two tries reports the error and ends (the original on film:
/// the truck stands from tick 688, the regiment leaves its Unpack at 721).
pub fn unpack_now(world: &mut World, a: ActorId) {
    if !live(world, a) {
        return;
    }
    let now = world.time;
    let actor = &world.actors[a as usize];
    let Order::Deploy { at: _, facing } = actor.order else { return };
    let Some(building) = world.data.def(actor.def).unpacks_to else { return };
    let kingdom = actor.kingdom;
    let org = actor.org;
    // 0x575C80 places the building where the truck stands less its +0x538 (the Unpack
    // state's Enter drove it to the order's point, as near as the map lets it).
    let at = actor.pos.truncate() - Vec2::from(world.data.def(actor.def).packed_position_offset);
    let (at, facing) = placement(world, building, at, facing);
    // An axis_locked building stands at (0, -1), the words of an order with no direction.
    let words = if world.data.def(building).axis_locked { NO_DIRECTION } else { crate::formation::facing_vector(facing) };
    if let Err(reason) = spot_clear_facing(world, building, at, words, Some(a)) {
        if org.is_none() {
            world.actors[a as usize].order = Order::Idle;
            if let Some(kingdom) = kingdom {
                world.events.push(Event::Refused { kingdom, reason });
            }
        }
        return;
    }
    let length = world.data.def(building).unpacking_length;
    world.ids.unpack_scope = true;
    let b = replace(world, a, building, at, facing);
    world.ids.unpack_scope = false;
    world.books.actor_mut(b).deploy = Some(Deployment { kind: DeployKind::Unpacking, into: building, started: now, done_at: now + length });
    world.set_activity(b, Activity::Unpacking);
}

#[cfg(test)]
mod tests {
    use super::*;
    use aa_spatial::collision::{Grid, Shape};
    use aa_spatial::geometry::{tile_rectangle, Placement};

    fn rect(width: f32, height: f32) -> Shape {
        shape_of(aa_rules::objects::shape(3, &[width.to_bits(), height.to_bits()], PC).unwrap().words)
    }
    fn circle(radius: f32) -> Shape {
        shape_of(aa_rules::objects::shape(1, &[radius.to_bits()], PC).unwrap().words)
    }
    fn words(x: f32, y: f32) -> [u32; 2] {
        [x.to_bits(), y.to_bits()]
    }
    fn thing(shape: Shape, x: f32, y: f32, facing: [f32; 2]) -> Listed {
        Listed { flags: UNPACK_REQUIRED, soft: shape.clone(), blocking: shape, position: words(x, y), facing: words(facing[0], facing[1]) }
    }
    /// A 6 m square HQ padded by the shipped 2 m: x and y 45..55, facing (0, -1).
    fn hq() -> Placement {
        Placement { shape: pad_shape(rect(6.0, 6.0), UNPACK_REQUIRED, 2.0), facing: words(0.0, -1.0), position: words(50.0, 50.0) }
    }

    #[test]
    fn hq_needs_the_padding_clear_of_another_building() {
        // The HQ's own edge is at x 53: a 4 m building 1.5 m beyond it reaches 54.5.
        assert!(listed_in_the_way(&hq(), None, &thing(rect(4.0, 4.0), 56.5, 50.0, [0.0, -1.0])));
        // 2.5 m beyond it, from 55.5: clear of the padded 55.
        assert!(!listed_in_the_way(&hq(), None, &thing(rect(4.0, 4.0), 57.5, 50.0, [0.0, -1.0])));
    }

    #[test]
    fn the_ghost_tests_the_unpadded_shape() {
        // `ghost_spot_clear` takes ObjectData +0x38 (x 47..53), the unpack the padded
        // shape (45..55): a 4 m building 1.5 m beyond the HQ's edge, from 54.5.
        let neighbour = thing(rect(4.0, 4.0), 56.5, 50.0, [0.0, -1.0]);
        let ghost = Placement { shape: rect(6.0, 6.0), ..hq() };
        assert!(!listed_in_the_way(&ghost, None, &neighbour));
        assert!(listed_in_the_way(&hq(), None, &neighbour));
    }

    #[test]
    fn a_neighbour_is_tested_by_its_rotated_shape() {
        // 20 m by 2 m centred at y 64.5: along x (facing (0, -1)) it spans y 63.5..65.5,
        // turned by facing (1, 0) it spans y 54.5..74.5 and reaches the padded HQ.
        let long = rect(20.0, 2.0);
        assert!(!listed_in_the_way(&hq(), None, &thing(long.clone(), 50.0, 64.5, [0.0, -1.0])));
        assert!(listed_in_the_way(&hq(), None, &thing(long, 50.0, 64.5, [1.0, 0.0])));
    }

    #[test]
    fn only_things_with_flags_10_and_29_count() {
        let mut unit = thing(rect(4.0, 4.0), 52.0, 50.0, [0.0, -1.0]);
        unit.flags = 0x400;
        assert!(!listed_in_the_way(&hq(), None, &unit));
        unit.flags = 0x2000_0000;
        assert!(!listed_in_the_way(&hq(), None, &unit));
        unit.flags = 0x2000_0400 | 1 << 21;
        assert!(listed_in_the_way(&hq(), None, &unit));
    }

    /// The shipped data the unpack query relies on: each building a truck unpacks into
    /// carries flags 10 and 29 (so `0x552340`'s settlement-wall test never applies) and not
    /// bit 18 (so `0x4F3640` runs its query); no truck carries flag 29 (so the ghost's
    /// ignored thing changes nothing); the final flags load for every definition with a
    /// blocking shape; and which definitions name a `required_object_ids`.
    #[test]
    #[ignore = "needs AA_GAME_ROOT"]
    fn shipped_unpack_buildings() {
        let root = std::env::var("AA_GAME_ROOT").expect("set AA_GAME_ROOT to the folder of the installed game");
        let data = crate::data::GameData::load(std::path::Path::new(&root)).expect("GameData::load");
        let mut buildings = Vec::new();
        for truck in data.defs.iter().filter(|d| d.unpacks_to.is_some()) {
            let b = data.def(truck.unpacks_to.unwrap());
            eprintln!("{} -> {}: flags {:#010x} axis_locked {} required {:?}", truck.id, b.id, b.thing_flags, b.axis_locked, b.required_object);
            assert_eq!(truck.thing_flags & 0x2000_0000, 0, "{} carries flag 29", truck.id);
            buildings.push(b);
        }
        assert!(!buildings.is_empty());
        for b in &buildings {
            assert_eq!(b.thing_flags & UNPACK_REQUIRED, UNPACK_REQUIRED, "{}", b.id);
            assert_eq!(b.thing_flags & ETHEREAL, 0, "{}", b.id);
        }
        let failed: Vec<&String> = data.load_notes.iter().filter(|n| n.contains(": final flags: ")).collect();
        // RANDOM_ACTOR (native kind 19) placeholders stand for a concrete definition chosen
        // when one is made; their flag finalizer is not ported and nothing tests them.
        let blocks = |d: &crate::data::Def| (d.blocking_words != [0; 6] || d.blocking_size.is_some() || d.blocking) && d.category != "RANDOM_ACTOR";
        let placeholders = data.defs.iter().filter(|d| d.category == "RANDOM_ACTOR" && failed.iter().any(|n| n.starts_with(&format!("{}: ", d.id)))).count();
        let blocking_failed: Vec<&&String> = failed.iter().filter(|n| data.defs.iter().any(|d| blocks(d) && n.starts_with(&format!("{}: ", d.id)))).collect();
        eprintln!("final flags failed for {} definitions ({placeholders} RANDOM_ACTOR), {} other with a blocking shape: {blocking_failed:?}", failed.len(), blocking_failed.len());
        assert!(blocking_failed.is_empty());
        let required: Vec<(&str, &str)> = data.defs.iter().filter_map(|d| d.required_object.as_deref().map(|r| (d.id.as_str(), r))).collect();
        eprintln!("required_object_ids: {required:?}");
        for (_, r) in &required {
            assert!(data.find(r).is_some(), "{r} is not a definition");
        }
    }

    #[test]
    fn a_thing_counts_only_where_its_cells_list_it() {
        // 4 m cells: the query's cells are x 11..13. A thing at x 58 listed by a 0.5 m soft
        // circle stands in cell 14 only, though its 8 m blocking box reaches 54.
        let grid = Grid { dimensions: [32, 32], coordinate_shift: 2, cell_size_bits: 4.0f32.to_bits() };
        let placed = hq();
        let cells = tile_rectangle(&grid, &placed, PC).unwrap();
        assert_eq!((cells.min, cells.max), ([11, 11], [13, 13]));
        let mut other = thing(rect(8.0, 8.0), 58.0, 50.0, [0.0, -1.0]);
        assert!(listed_in_the_way(&placed, Some((&grid, cells)), &other));
        other.soft = circle(0.5);
        assert!(!listed_in_the_way(&placed, Some((&grid, cells)), &other));
        assert!(listed_in_the_way(&placed, None, &other));
    }
}
