//! The world's things as the collision queries see them: GWorld+0x30, the tile lists
//! of `aa_spatial::index::SpatialIndex` with the descriptor policies of `0x552340`
//! and `0x4D5600`. Census cause M5.
//!
//! [`Space`] follows the actors: registered when they appear, removed when they go,
//! their positions committed by the Mover (`0x4EEB40`) and, for what other systems
//! moved, at the start of the update. The flags are the Thing's +8 words the queries
//! read: bit 10 `blocking`, 18 ethereal, 21 a water mover, 29 no Mover.
//!
//! A query that cannot hit (no tile bit and no thing's shape within a margin of the
//! placement, all in f64) answers without the x87 arithmetic of the port, whose exact
//! test decides everything closer; see [`Space::surely_clear`].
//!
//! A map's objects that are no actors stand in the lists by their records in
//! `Books::scenery`, when their definition blocks: `Space::add_scenery`. A tile a large one
//! covers carries the blocker bit (`aa_spatial::geometry::marks_blocker`).
//!
//! **Rendition:** the trailers (aa-play keeps no trailer things) are not in the lists.
use super::*;
use aa_navigation::low_path::{LowPathSolver, WindowConfig};
use aa_navigation::search::{SearchCell, SearchGrid};
use aa_numeric::Precision;
use aa_spatial::collision::{self, Descriptor, Shape};
use aa_spatial::geometry::Placement;
use aa_spatial::index::{Member, SpatialIndex};
use aa_spatial::overlaps;
use aa_spatial::query::{self, Hit, PreparedQuery, QueryGeometry, QueryPolicy};
use aa_spatial::source_exclusion as se;
use aa_spatial::sweep;

const PC: Precision = PC53;
/// Thing +8 bits the queries read.
const REMOVED: u32 = 1;
const DEPLOYING: u32 = 1 << 2;
const BLOCKING: u32 = 1 << 10;
const ETHEREAL: u32 = 1 << 18;
const WATER: u32 = 1 << 21;
const NO_MOVER: u32 = 1 << 29;

/// `0x554AD0(point, direction, definition of `id`, 3, 0)` -> `0x554850`: the point, or
/// the first clear place on three rings around it. A map whose things are not indexed
/// keeps the point.
pub(super) fn place_near(ms: &mut MovementState, world: &World, id: ActorId, point: &mut [u32; 2], direction: [u32; 2]) {
    let Some(mut s) = ms.space.take() else { return };
    let def = world.data.def(world.actors[id as usize].def);
    let large = mover_class(def) == MoverClass::Water;
    if let Ok(radius_bits) = aa_spatial::placement::wrapper_radius(def.blocking_words[1], world.map.cell_size.to_bits(), PC) {
        let search = aa_spatial::placement::PlacementSearch { shape: shape(def.blocking_words), radius_bits, rings: 3, keep_direction: true, strict: false, large };
        let mut facing = direction;
        let mut policy = Policy { ms: &*ms, world };
        let Space { index, geometry, .. } = &mut *s;
        let _ = aa_spatial::placement::find_free_placement(index, geometry, point, &mut facing, &search, &mut policy);
    }
    ms.space = Some(s);
}

/// The index key and GTID of an actor (key 0 is no one).
pub(super) fn actor_key(id: ActorId) -> u32 {
    id + 1
}
pub(super) fn key_actor(key: u32) -> ActorId {
    key - 1
}
/// Keys of a map's objects that are no actors, the low half of a key above `0x8000` so
/// they name no actor's GTID slot (`SpatialIndex::register`). `thing` here is a key - 1,
/// as [`key_actor`] names an actor's.
const SCENERY_KEY: u32 = 0x4000_8000;
pub(super) fn is_scenery(thing: u32) -> bool {
    thing & 0x8000_0000 == 0 && thing >= SCENERY_KEY - 1
}
fn err(what: impl std::fmt::Display) -> collision::Error {
    collision::Error::World(what.to_string())
}

fn shape(words: [u32; 6]) -> Shape {
    Shape { kind: words[0], radius_bits: words[1], bounds: [words[2], words[3], words[4], words[5]] }
}

#[derive(Clone, Copy, Debug, Default)]
struct Tracked {
    registered: bool,
    position: [u32; 3],
    facing: [u32; 2],
    flags: u32,
}

/// GWorld+0x30 and GWorld+0x23C.
pub(super) struct Space {
    pub index: SpatialIndex,
    pub geometry: QueryGeometry,
    /// The low-path solver's grid view (tile flags, visit stamps).
    pub grid: SearchGrid,
    pub solver: LowPathSolver,
    pub busy: bool,
    map: usize,
    tracked: Vec<Tracked>,
    /// Largest blocking reach of any registered thing, for [`Space::surely_clear`].
    largest: f64,
    /// `map.placed` indices of the objects in the lists, by key - [`SCENERY_KEY`].
    scenery: Vec<usize>,
    cell: f64,
    dims: [i32; 2],
}
impl std::fmt::Debug for Space {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Space")
    }
}

/// The flags of `id` now.
pub(super) fn thing_flags(ms: &MovementState, world: &World, id: ActorId) -> u32 {
    let a = &world.actors[id as usize];
    let def = world.data.def(a.def);
    let mut flags = 0;
    if a.removed || !a.alive {
        flags |= REMOVED;
    }
    let walking = ms.actors.get(id as usize).is_some_and(|m| m.walk.is_some());
    if walking {
        flags |= ETHEREAL | DEPLOYING;
    } else if def.blocking {
        flags |= BLOCKING;
    }
    if def.speed_max <= 0.0 {
        flags |= NO_MOVER;
    }
    if mover_class(def) == MoverClass::Water {
        flags |= WATER;
    }
    flags
}

impl Space {
    pub(super) fn new(world: &World) -> Option<Box<Space>> {
        let map = &world.map;
        if map.width == 0 || map.height == 0 || map.cell_size <= 0.0 {
            return None;
        }
        let cell_size_bits = map.cell_size.to_bits();
        let coordinate_shift = (map.cell_size as u32).trailing_zeros();
        let grid = collision::Grid { dimensions: [map.width as i32, map.height as i32], coordinate_shift, cell_size_bits };
        let flags: Vec<u32> = map.flags.iter().map(|&f| f & !crate::path::OCCUPIED).collect();
        let mut extent = [map.width as f32 * map.cell_size, map.height as f32 * map.cell_size];
        if map.width as usize * map.height as usize != flags.len() {
            return None;
        }
        extent[0] += 0.0;
        let index = SpatialIndex::new(grid, flags.clone(), PC).ok()?;
        let geometry = QueryGeometry::from_native_extents(&index, [extent[0].to_bits(), extent[1].to_bits()], PC).ok()?;
        let cells: Vec<SearchCell> = flags.iter().map(|&flags20| SearchCell { flags20, ..SearchCell::default() }).collect();
        let search = SearchGrid { width: map.width as i32, height: map.height as i32, coordinate_shift, cell_size_bits, epoch: 0, cells };
        let solver = LowPathSolver::new(&search, WindowConfig::NATIVE, PC).ok()?;
        Some(Box::new(Space {
            index,
            geometry,
            grid: search,
            solver,
            busy: false,
            map: std::sync::Arc::as_ptr(&world.map) as usize,
            tracked: Vec::new(),
            largest: 0.0,
            scenery: Vec::new(),
            cell: f64::from(map.cell_size),
            dims: [map.width as i32, map.height as i32],
        }))
    }

    /// [`Space::new`] with the map's objects in the lists.
    pub(super) fn with_scenery(world: &World) -> Option<Box<Space>> {
        let mut s = Space::new(world)?;
        s.add_scenery(world);
        Some(s)
    }

    pub(super) fn fits(&self, world: &World) -> bool {
        self.map == std::sync::Arc::as_ptr(&world.map) as usize
    }

    /// The objects of `Books::scenery` that block, in the lists as the map's Load leaves
    /// them: Thing +8 from the definition (`native_final_flags`), the blocking shape, not of
    /// the actor category (virtual +0xB4, +0xB0: `0x4EE4C0` holds them by their flags).
    /// A doodad a building clears (`ground::doodads_under`) does not block, so none goes.
    fn add_scenery(&mut self, world: &World) {
        for &index in &world.books.scenery {
            let Some(p) = world.map.placed.get(index) else { continue };
            let Some(d) = world.data.find(&p.thing_id) else { continue };
            let def = world.data.def(d);
            if def.thing_flags & BLOCKING == 0 || self.scenery.len() >= 0x7fff {
                continue;
            }
            let key = SCENERY_KEY + self.scenery.len() as u32;
            let blocking = shape(def.blocking_words);
            let soft = shape(def.soft_shape);
            let member = Member {
                key,
                reference: key,
                actor: collision::Actor { definition: d, flags: def.thing_flags & !(1 << 26), position: [p.position[0].to_bits(), p.position[1].to_bits()], facing: [p.facing[0].to_bits(), p.facing[1].to_bits()] },
                height_bits: p.position[2].to_bits(),
                facing_z_bits: 0,
                cached_tile: [0, 0],
                blocking,
                soft: soft.clone(),
                target: soft,
                actor_category: false,
                wall_present: false,
            };
            if self.index.register(member).is_err() || self.index.refresh_membership(key, false).is_err() {
                continue;
            }
            self.scenery.push(index);
        }
    }
    /// Actors created since the last look (in the middle of an update, for a query
    /// that may come before the next [`Space::sync`]).
    pub(super) fn add_new(&mut self, ms: &MovementState, world: &World) {
        let from = self.tracked.len();
        if from >= world.actors.len() {
            return;
        }
        self.tracked.resize(world.actors.len(), Tracked::default());
        for i in from..world.actors.len() {
            let a = &world.actors[i];
            if a.removed || !a.alive {
                continue;
            }
            let id = i as ActorId;
            let position = [a.pos.x.to_bits(), a.pos.y.to_bits(), a.pos.z.to_bits()];
            let f = ms.actors.get(i).map_or_else(|| facing_vector(a.facing), |m| m.mover.facing);
            self.add(id, world, position, [f.x.to_bits(), f.y.to_bits()], thing_flags(ms, world, id));
        }
    }

    /// The actors as they stand now: new ones in, gone ones out, what another system
    /// moved or turned committed.
    pub(super) fn sync(&mut self, ms: &MovementState, world: &World) {
        if self.tracked.len() < world.actors.len() {
            self.tracked.resize(world.actors.len(), Tracked::default());
        }
        for (i, a) in world.actors.iter().enumerate() {
            let id = i as ActorId;
            let gone = a.removed || !a.alive;
            let t = self.tracked[i];
            if gone {
                if t.registered {
                    self.remove(id);
                }
                continue;
            }
            let flags = thing_flags(ms, world, id);
            let position = [a.pos.x.to_bits(), a.pos.y.to_bits(), a.pos.z.to_bits()];
            let facing = ms.actors.get(i).map_or_else(|| facing_vector(a.facing), |m| m.mover.facing);
            let facing = [facing.x.to_bits(), facing.y.to_bits()];
            if !t.registered {
                self.add(id, world, position, facing, flags);
                continue;
            }
            if t.flags != flags {
                self.set_flags(id, flags);
            }
            if t.position != position {
                self.commit(id, position);
            }
            if t.facing != facing {
                self.set_facing(id, facing);
            }
        }
    }

    fn add(&mut self, id: ActorId, world: &World, position: [u32; 3], facing: [u32; 2], flags: u32) {
        let a = &world.actors[id as usize];
        let def = world.data.def(a.def);
        let blocking = shape(def.blocking_words);
        let soft = shape(def.soft_shape);
        let key = actor_key(id);
        let member = Member {
            key,
            reference: key,
            actor: collision::Actor { definition: a.def, flags, position: [position[0], position[1]], facing },
            height_bits: position[2],
            facing_z_bits: 0,
            cached_tile: [0, 0],
            blocking,
            soft: soft.clone(),
            target: soft,
            actor_category: true,
            wall_present: false,
        };
        let reach = f64::from(f32::from_bits(def.blocking_words[1]));
        if self.index.register(member).is_err() || self.index.refresh_membership(key, true).is_err() {
            return;
        }
        if reach > self.largest {
            self.largest = reach;
        }
        self.tracked[id as usize] = Tracked { registered: true, position, facing, flags };
    }

    fn remove(&mut self, id: ActorId) {
        let key = actor_key(id);
        self.tracked[id as usize].registered = false;
        // 0x4F6760: out of the tile lists, its slot free.
        let _ = self.index.or_removal_flags(key, REMOVED);
        let _ = self.index.remove(key);
        let _ = self.index.unregister(key);
    }

    /// `0x4D5EF0`-style vf+0x50 and the Deploy state's words: out of the lists, the
    /// flags, back in (`0x4EE4C0`).
    pub(super) fn set_flags(&mut self, id: ActorId, flags: u32) {
        let key = actor_key(id);
        let Some(t) = self.tracked.get_mut(id as usize) else { return };
        if !t.registered || t.flags == flags {
            return;
        }
        t.flags = flags;
        let _ = self.index.remove(key);
        let _ = self.index.set_thing_flags(key, flags);
        let _ = self.index.refresh_membership(key, true);
    }

    /// `0x4EE5C0`.
    pub(super) fn commit(&mut self, id: ActorId, position: [u32; 3]) {
        let Some(t) = self.tracked.get_mut(id as usize) else { return };
        if !t.registered {
            return;
        }
        t.position = position;
        let _ = self.index.commit_position_fields(actor_key(id), position);
    }

    /// `0x4EEBE0`.
    pub(super) fn set_facing(&mut self, id: ActorId, facing: [u32; 2]) {
        let Some(t) = self.tracked.get_mut(id as usize) else { return };
        if !t.registered {
            return;
        }
        t.facing = facing;
        let _ = self.index.set_facing_fields(actor_key(id), facing);
    }

    /// No hit is possible for `shape` of bounding radius `radius` at `p` for a query
    /// with tile `mask`: every tile under it has clear flags and no thing with bit 10
    /// comes within a margin. Decided in f64 with a margin far above the x87 rounding,
    /// so a clear answer is the port's; anything closer goes to the port.
    pub(super) fn surely_clear(&self, p: [f64; 2], radius: f64, mask: u32, source: u32) -> bool {
        const MARGIN: f64 = 1.0 / 64.0;
        let reach = radius + MARGIN;
        let cell = self.cell;
        let (lo, hi) = ((p[0] - reach) / cell, (p[1] - reach) / cell);
        if lo < 1.0 || hi < 1.0 {
            return false;
        }
        let (x0, y0) = (lo as i32, hi as i32);
        let (x1, y1) = (((p[0] + reach) / cell) as i32, ((p[1] + reach) / cell) as i32);
        if x1 >= self.dims[0] - 1 || y1 >= self.dims[1] - 1 {
            return false;
        }
        for y in y0..=y1 {
            for x in x0..=x1 {
                let Ok(tile) = self.index.tile([x, y]) else { return false };
                if tile.flags20 & mask != 0 {
                    return false;
                }
                let mut cursor = tile.first_node;
                while let Some(n) = cursor {
                    let Ok(node) = self.index.node(n) else { return false };
                    cursor = node.next;
                    if node.actor == source {
                        continue;
                    }
                    let Ok(m) = self.index.member(node.actor) else { return false };
                    if m.actor.flags & BLOCKING == 0 {
                        continue;
                    }
                    let (dx, dy) = (f64::from(f32::from_bits(m.actor.position[0])) - p[0], f64::from(f32::from_bits(m.actor.position[1])) - p[1]);
                    let touch = reach + f64::from(f32::from_bits(m.blocking.radius_bits));
                    if dx * dx + dy * dy <= touch * touch {
                        return false;
                    }
                }
            }
        }
        true
    }
}

/// The descriptor services of the queries over the things as `MovementState` and the
/// world hold them.
pub(super) struct Policy<'a> {
    pub ms: &'a MovementState,
    pub world: &'a World,
}

impl aa_spatial::index_world::CollisionPolicy for Policy<'_> {
    /// The definition's bit 10 (`blocking`) and bit 29 as the flags above.
    fn definition_flags(&self, definition: u32) -> collision::Result<u32> {
        let d = self.world.data.def(definition);
        Ok(if d.blocking { BLOCKING } else { 0 } | if d.speed_max <= 0.0 { NO_MOVER } else { 0 })
    }
    /// A settlement's wall (none is hosted).
    fn definition_flag_1a8(&self, _: u32) -> collision::Result<bool> {
        Ok(false)
    }
    fn custom_filter(&mut self, _: &mut SpatialIndex, _: u32, _: &mut Descriptor) -> collision::Result<bool> {
        Ok(false)
    }
    /// Actor virtual +0x30 `0x4D5600`.
    fn source_excludes(&mut self, index: &mut SpatialIndex, source: u32, actor: u32, _: &mut Descriptor) -> collision::Result<bool> {
        let mut w = Exclusion { ms: self.ms, world: self.world, index };
        se::excludes(source, actor, &mut w)
    }
    fn allegiance(&mut self, _: &mut SpatialIndex, _: u32, _: &mut Descriptor) -> collision::Result<Option<u32>> {
        Ok(None)
    }
    fn hostility_dispatch(&self, _: u32) -> collision::Result<u32> {
        Ok(0)
    }
    fn hostile(&mut self, _: &mut SpatialIndex, _: u32, _: Option<u32>, _: Option<u32>, _: &mut Descriptor) -> collision::Result<bool> {
        Ok(false)
    }
    /// `0x4CEDE0` on the thing: the low path's byte +0x31 passes movers.
    fn moving_exclusion(&mut self, _: &mut SpatialIndex, actor: u32, _: &mut Descriptor) -> collision::Result<bool> {
        Ok(self.ms.actors.get(key_actor(actor) as usize).is_some_and(|a| a.mover.active))
    }
    fn outer_actor(&mut self, _: &mut SpatialIndex, _: u32, _: &mut Descriptor) -> collision::Result<Option<u32>> {
        Ok(None)
    }
}
impl QueryPolicy for Policy<'_> {
    fn definition_has_object_data(&self, _: u32) -> collision::Result<bool> {
        Ok(true)
    }
}

/// `aa_spatial::source_exclusion::World` of one test.
struct Exclusion<'a> {
    ms: &'a MovementState,
    world: &'a World,
    index: &'a SpatialIndex,
}
impl se::World for Exclusion<'_> {
    type Actor = u32;
    type Mind = OrgId;
    type AcceptanceDispatch = ();
    type Owner = Option<u32>;
    fn actor(&self, actor: u32) -> collision::Result<se::Actor> {
        let m = self.index.member(actor).map_err(err)?;
        let has_element = self.world.actors.get(key_actor(actor) as usize).is_some_and(|a| a.org.is_some());
        Ok(se::Actor {
            definition: m.actor.definition,
            reference: m.reference,
            flags: m.actor.flags,
            position: m.actor.position,
            linked_50: 0,
            linked_168: 0,
            has_element,
            has_wall: false,
        })
    }
    fn definition(&self, _: u32) -> collision::Result<se::Definition> {
        Ok(se::Definition { required_object: None, settlement_wall: false, gate: false })
    }
    fn resolve(&self, reference: u32) -> collision::Result<Option<u32>> {
        Ok(self.index.resolve_reference(reference))
    }
    /// Actor +0x110, its overlaps and the things it ignores for a while.
    fn exclusion_head(&self, source: u32) -> collision::Result<Option<u32>> {
        Ok((!self.ms.actors[key_actor(source) as usize].overlaps.is_empty()).then_some(key_actor(source) << 12))
    }
    fn exclusion_node(&self, node: u32) -> collision::Result<se::ExclusionNode> {
        let (actor, at) = (node >> 12, node & 0xfff);
        let list = &self.ms.actors[actor as usize].overlaps;
        let entry = list.get(at as usize).ok_or_else(|| err("exclusion node"))?;
        Ok(se::ExclusionNode { reference: entry.reference, next: (at as usize + 1 < list.len()).then_some(node + 1) })
    }
    fn actor_category(&mut self, actor: u32) -> collision::Result<bool> {
        Ok(self.index.member(actor).map_err(err)?.actor_category)
    }
    fn acceptance_dispatch(&self, _: u32) -> collision::Result<()> {
        Ok(())
    }
    fn owner(&mut self, actor: u32) -> collision::Result<Option<u32>> {
        Ok(self.world.actors.get(key_actor(actor) as usize).and_then(|a| a.kingdom))
    }
    /// `0x4D1EB0`: the same owner, or virtual +0x144 says ally (1).
    fn accepts_owner(&mut self, _: (), actor: u32, owner: Option<u32>) -> collision::Result<bool> {
        let own = self.world.actors.get(key_actor(actor) as usize).and_then(|a| a.kingdom);
        se::ordinary_accepts_owner(own, owner, || Ok(u32::from(kingdom_team(self.world, own) == kingdom_team(self.world, owner))))
    }
    /// Element +0x10.
    fn organization(&self, actor: u32) -> collision::Result<Option<u32>> {
        Ok(self.world.actors.get(key_actor(actor) as usize).and_then(|a| a.org).filter(|&o| self.world.orgs.get(o as usize).is_some_and(|x| x.alive)))
    }
    fn mind(&self, organization: u32) -> collision::Result<Option<OrgId>> {
        Ok(Some(organization))
    }
    /// Mind +0x64 `0x5247B0`: its Move's status is 2.
    fn mind_ready_64(&mut self, o: OrgId) -> collision::Result<bool> {
        Ok(self.ms.orgs.get(o as usize).is_some_and(|x| x.order.active))
    }
    fn wall_allows_source(&mut self, _: u32, _: u32) -> collision::Result<bool> {
        Ok(false)
    }
    fn parent(&mut self, _: u32) -> collision::Result<Option<u32>> {
        Ok(None)
    }
    fn parent_collision_required(&mut self, _: u32, _: Option<u32>) -> collision::Result<bool> {
        Ok(false)
    }
}

/// A query's shape, what [`Space::surely_clear`] needs of it.
fn bounding(query: &PreparedQuery) -> f64 {
    let s = &query.placement.shape;
    let r = f64::from(f32::from_bits(s.radius_bits));
    if s.kind <= 1 {
        r
    } else {
        // Boxes: the corner, whatever the constructor kept in the radius word.
        let hx = f64::from(f32::from_bits(s.bounds[2]));
        let hy = f64::from(f32::from_bits(s.bounds[3]));
        r.max((hx * hx + hy * hy).sqrt())
    }
}
fn placed(p: &Placement) -> [f64; 2] {
    [f64::from(f32::from_bits(p.position[0])), f64::from(f32::from_bits(p.position[1]))]
}

/// `0x553BA0` with the fast answer in front.
pub(super) fn first_hit(space: &mut Space, policy: &mut Policy<'_>, query: &PreparedQuery) -> Hit {
    let source = query.descriptor.source_actor.unwrap_or(0);
    if query.required_actor_flags & BLOCKING != 0 && !query.descriptor.ignore_all && space.surely_clear(placed(&query.placement), bounding(query), query.descriptor.mask, source) {
        return Hit::default();
    }
    let geometry = space.geometry.clone();
    match query::first_hit(&mut space.index, &geometry, query, policy) {
        Ok(hit) => hit,
        Err(_) => Hit::default(),
    }
}

/// `0x553CE0`.
pub(super) fn sweep_to(space: &mut Space, policy: &mut Policy<'_>, query: &mut PreparedQuery, goal: [u32; 2]) -> Hit {
    let source = query.descriptor.source_actor.unwrap_or(0);
    if query.required_actor_flags & BLOCKING != 0 && !query.descriptor.ignore_all {
        // The circle around the whole corridor.
        let (a, b) = (placed(&query.placement), [f64::from(f32::from_bits(goal[0])), f64::from(f32::from_bits(goal[1]))]);
        let half = ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt() * 0.5;
        if space.surely_clear([(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5], half + bounding(query), query.descriptor.mask, source) {
            return Hit::default();
        }
    }
    let geometry = space.geometry.clone();
    match sweep::sweep(&mut space.index, &geometry, query, goal, policy) {
        Ok(hit) => hit,
        Err(_) => Hit::default(),
    }
}

/// FindOpenPositionLineThenRadius `0x554BF0(point, facing, definition, from, radius)`: the
/// line `0x553FA0` back from the point to `from`, static things only, then `0x554B30` around
/// what it found, every blocking thing. No clear place on the rings keeps the line's.
pub(super) fn open_line_then_radius(space: &mut Space, policy: &mut Policy<'_>, def: u32, point: [u32; 2], facing: [u32; 2], from: [u32; 2], radius_bits: u32) -> [u32; 2] {
    let d = policy.world.data.def(def);
    let (shape_of, radius_word, water) = (shape(d.blocking_words), d.blocking_words[1], d.thing_flags & WATER != 0);
    let mut at = open_line_553fa0(space, policy, shape_of.clone(), radius_word, water, point, facing, from);
    let line = at;
    let mut direction = facing;
    let Space { index, geometry, .. } = space;
    match aa_spatial::placement::find_placement_within(index, geometry, &mut at, &mut direction, shape_of, radius_word, radius_bits, false, water, policy) {
        Ok(true) => at,
        _ => line,
    }
}

/// `0x554B30(point, facing, definition of `id`, radius, strict, 0)`: the point, or the first
/// clear place on the rings of radius / step around it from the facing.
pub(super) fn open_radius(space: &mut Space, policy: &mut Policy<'_>, id: ActorId, point: [u32; 2], facing: [u32; 2], radius_bits: u32, strict: bool) -> Option<[u32; 2]> {
    let d = policy.world.data.def(policy.world.actors[id as usize].def);
    let (shape_of, radius_word, water) = (shape(d.blocking_words), d.blocking_words[1], d.thing_flags & WATER != 0);
    let (mut at, mut direction) = (point, facing);
    let Space { index, geometry, .. } = space;
    matches!(aa_spatial::placement::find_placement_within(index, geometry, &mut at, &mut direction, shape_of, radius_word, radius_bits, strict, water, policy), Ok(true)).then_some(at)
}

/// `0x553FA0(point, facing, definition, from, strict 1)`: from the point back to `from` in
/// steps of the definition's radius (at least a quarter tile), each place clamped into the
/// map (`0x552640`); the first that no static thing and no terrain takes. On none the walk
/// has ended at `from`. Away from `from`, the clamped place is the answer.
#[allow(clippy::too_many_arguments)]
fn open_line_553fa0(space: &mut Space, policy: &mut Policy<'_>, shape_of: Shape, radius_word: u32, water: bool, point: [u32; 2], facing: [u32; 2], from: [u32; 2]) -> [u32; 2] {
    let f = f32::from_bits;
    let cell = f(space.index.grid().cell_size_bits);
    let quarter = (f64::from(cell) * 0.25) as f32;
    let step = if f(radius_word) < quarter { quarter } else { f(radius_word) };
    let (mut px, mut py) = (f(point[0]), f(point[1]));
    let (fx, fy) = (f(from[0]), f(from[1]));
    let (mut dx, mut dy) = ((f64::from(px) - f64::from(fx)) as f32, (f64::from(py) - f64::from(fy)) as f32);
    let mut left = (f64::from(dx) * f64::from(dx) + f64::from(dy) * f64::from(dy)).sqrt() as f32;
    if left > 0.0 {
        let scale = f64::from(step) / f64::from(left);
        dx = (f64::from(dx) * scale) as f32;
        dy = (f64::from(dy) * scale) as f32;
    }
    let mut descriptor = Descriptor::default();
    descriptor.mask = if water { 0x600 } else { 0x500 };
    let mut query = PreparedQuery { placement: Placement { shape: shape_of, facing, position: [0, 0] }, descriptor, required_actor_flags: 0x400 | 0x2000_0000 };
    let bounds = aa_navigation::endpoint::Bounds { cell_size_bits: cell.to_bits(), extent_bits: space.geometry.extent_bits };
    let mut found = true;
    let clamped = loop {
        let mut place = [px.to_bits(), py.to_bits()];
        let _ = aa_navigation::endpoint::clamp_to_interior(bounds, &mut place, PC);
        query.placement.position = place;
        if !first_hit(space, policy, &query).blocked() {
            break place;
        }
        if left <= 0.0 {
            found = false;
            break place;
        }
        px = (f64::from(px) - f64::from(dx)) as f32;
        py = (f64::from(py) - f64::from(dy)) as f32;
        left = (f64::from(left) - f64::from(step)) as f32;
        if left <= 0.0 {
            px = fx;
            py = fy;
            left = 0.0;
        }
    };
    let away = (f64::from(px) - f64::from(fx)).abs() + (f64::from(py) - f64::from(fy)).abs();
    if found && away >= f64::from(f32::from_bits(0x3951_b717)) {
        return clamped;
    }
    [px.to_bits(), py.to_bits()]
}

/// `0x4F3640(descriptor, no source, no other thing, ...)` for a definition with a Mover
/// (`0x556070`, `id`'s definition at `at` and `facing`). Bit 29 of a Mover's definition
/// is clear, so the zone blocks (`0x4F1660`, the walls of `0x4F2220`) and the required
/// object stand aside: what is left is outside the map (3) and `0x4F0A70` -> `0x553BA0`
/// with no source: every blocking thing counts, the actor's own regiment and itself too.
pub(super) fn creation_clear(space: &mut Space, policy: &mut Policy<'_>, id: ActorId, at: [u32; 2], facing: [u32; 2]) -> Option<bool> {
    let def = policy.world.data.def(policy.world.actors[id as usize].def);
    let mut q = query_for(space, policy, id, false)?;
    q.descriptor.source_actor = None;
    q.required_actor_flags = 0x400 | if def.blocking { 0 } else { 0x2000_0000 };
    q.descriptor.mask = if mover_class(def) == MoverClass::Water { 0x600 } else { 0x500 };
    q.set_position(&space.index, at, policy).ok()?;
    q.placement.facing = facing;
    if aa_spatial::query::outside(&q.placement, space.geometry.extent_bits, PC).unwrap_or(true) {
        return Some(false);
    }
    Some(!first_hit(space, policy, &q).blocked())
}

/// `0x554350(actor, strict)`.
pub(super) fn query_for(space: &Space, policy: &Policy<'_>, id: ActorId, strict: bool) -> Option<PreparedQuery> {
    aa_spatial::actor_context::prepare(&space.index, actor_key(id), strict, policy).ok()
}

/// `0x4D1820`: entries of things that no longer resolve leave the head of the list; for
/// the first that does, `0x4D04D0`: from the other thing's position, along the direction
/// from it to this one ((1, 0) when they coincide) turned by up to seven eighths of a
/// turn, at its blocking radius plus this one's soft radius, the first place this
/// one's query finds clear. The turn's `fsin` and `fcos` are binary64 here: a difference
/// could only show in a last bit, and only after the first direction is blocked.
pub(super) fn overlap_point(ms: &mut MovementState, world: &World, id: ActorId) -> Option<[u32; 2]> {
    let mut s = ms.space.take()?;
    let other = loop {
        let Some(first) = ms.actors[id as usize].overlaps.first().map(|e| e.reference) else { break None };
        match s.index.resolve_reference(first) {
            Some(key) => break Some(key_actor(key)),
            None => {
                ms.actors[id as usize].overlaps.remove(0);
            }
        }
    };
    // The list holds things a moving actor passed; an object of the map does not get up.
    let other = other.filter(|&t| !is_scenery(t));
    let result = other.and_then(|other| {
        let (me, it) = (pos2(world, id), pos2(world, other));
        let (dx, dy) = (me.x - it.x, me.y - it.y);
        let (x, y) = if dx != 0.0 || dy != 0.0 {
            let (fx, fy) = (f64::from(dx), f64::from(dy));
            let inverse = 1.0 / (fx * fx + fy * fy).sqrt();
            (f64::from((inverse * fx) as f32), f64::from((inverse * fy) as f32))
        } else {
            (1.0, 0.0)
        };
        let radius = f32::from_bits(world.data.def(world.actors[other as usize].def).blocking_words[1]) + f32::from_bits(world.data.def(world.actors[id as usize].def).soft_shape[1]);
        let radius = f64::from(radius);
        let mut policy = Policy { ms: &*ms, world };
        let mut query = query_for(&s, &policy, id, false)?;
        for turn in 0..8 {
            let (tx, ty) = if turn == 0 {
                (x, y)
            } else {
                let angle = f64::from(turn) * f64::from(0.785_398_2f32);
                let (sin, cos) = angle.sin_cos();
                (f64::from((cos * x - sin * y) as f32), f64::from((cos * y + sin * x) as f32))
            };
            let point = [((tx * radius + f64::from(it.x)) as f32).to_bits(), ((f64::from(it.y) + radius * ty) as f32).to_bits()];
            query.set_position(&s.index, point, &policy).ok()?;
            if !first_hit(&mut s, &mut policy, &query).blocked() {
                return Some(point);
            }
        }
        None
    });
    ms.space = Some(s);
    result
}

/// `0x4D67F0` over [`ActorMove::overlaps`].
pub(super) fn refresh_overlaps(space: &mut Space, ms: &mut MovementState, world: &World, id: ActorId) {
    // The list is empty and no thing's shape comes near: the query finds nothing and
    // the port changes nothing, so this answers without its x87 arithmetic.
    if ms.actors[id as usize].overlaps.is_empty() {
        let policy = Policy { ms: &*ms, world };
        if let Some(query) = query_for(space, &policy, id, false)
            && space.surely_clear(placed(&query.placement), bounding(&query), 0, actor_key(id))
        {
            return;
        }
    }
    let mut entries = std::mem::take(&mut ms.actors[id as usize].overlaps);
    let now = world.clock.begin.0;
    let geometry = space.geometry.clone();
    {
        let mut policy = Policy { ms, world };
        let _ = overlaps::refresh_overlaps(&mut space.index, &geometry, actor_key(id), &mut entries, now, &mut |_, _| Ok(true), &mut policy);
    }
    ms.actors[id as usize].overlaps = entries;
}

/// A hit as the 16-byte native result.
pub(super) fn words(hit: &Hit) -> [u32; 4] {
    [hit.kind, hit.reference, hit.cell[0] as u32, hit.cell[1] as u32]
}


