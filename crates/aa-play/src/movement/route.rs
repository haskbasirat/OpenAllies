//! `KKC_PathSolver::Solve` `0x53E1F0` on the exact ports (`aa_navigation::solver`,
//! `region`, `region_search`, `search`, `cost`, `materialize`, `endpoint`), over the
//! tile lists of [`space`] and the SAI regions of the map. Census cause M7.
//!
//! The solver finds the regions from the start to the goal (`0x53DC00`, when
//! `TAIUseRegionPathPlanner` is set and the straight line crosses land that does
//! not pass), then the tile route to the first region it cannot see past, and leaves
//! the hops after it as kind-2 nodes; path +0x18 names that region for `0x52CAE0`.
//!
//! **Renditions:** walls are not hosted, so `0x4F2220` finds no structure, a tile
//! holds no wall thing for `0x53CA30` and `0x53CB70`'s wall cost.
use super::space::{self, Policy, Space};
use super::*;
use aa_navigation::region::{self, RegionRoute};
use aa_navigation::region_search::{self, RegionGraph};
use aa_navigation::search::{self, SearchGrid};
use aa_navigation::topology::{GridLayout, StaticTile, Topology};
use aa_navigation::topology_builder::{StaticGrid, StaticMapParameters, build};
use aa_navigation::{OwnedPath, cost, endpoint, materialize, solver};
use aa_numeric::Precision;
use aa_spatial::gworld_placement as gp;
use aa_spatial::index::{ActorKey, SpatialIndex};
use aa_spatial::index_world::CollisionView;
use aa_spatial::query::{self, Hit, PreparedQuery, QueryGeometry};
use aa_spatial::tile_walk::{TileGrid, TileWalk};

const PC: Precision = PC53;
/// `0x51EA20` and `0x5336B0` set this in the descriptor's +0x38.
const NO_MOVER_FLAG: u32 = 0x2000_0000;

/// `[B3652C]`: the SAI map, as far as the solvers read it. Its adjacency order and
/// distance pairs are what the map saved (`0x7AA9B0`), or `0x7ACB00` for a generated
/// map.
pub(super) struct Regions {
    topology: Topology,
    graph: RegionGraph,
    state: region_search::State,
}
impl std::fmt::Debug for Regions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Regions")
    }
}

impl Regions {
    pub(super) fn of(world: &World) -> Option<Box<Regions>> {
        let topology = match world.map.saved_topology.as_deref() {
            Some(saved) if saved.enabled => saved.clone(),
            _ => static_analysis(world)?,
        };
        let mut graph = topology.prepare_graph().ok()?;
        // `0x7A0D40` set the SAI ready before the battle's first update.
        graph.ready = true;
        Some(Box::new(Regions { topology, graph, state: region_search::State { busy: false, output_linked: false } }))
    }
    /// `0x7AA3D0`, the region's id (-1 for none).
    pub(super) fn region_at(&self, point: [u32; 2]) -> i32 {
        self.topology.region_at(point).ok().flatten().map_or(-1, |r| r as i32)
    }
    /// Region +0x70 / +0x74.
    pub(super) fn neighbors(&self, region: u32) -> &[u32] {
        self.graph.nodes.get(region as usize).map_or(&[], |n| &n.neighbors)
    }
    /// `0x52CAE0`: the point is in path +0x18's region or next to it.
    pub(super) fn path_region_reached(&self, word18: u32, point: [u32; 2]) -> bool {
        let target = word18 as i32;
        if target < 0 {
            return false;
        }
        let here = self.region_at(point);
        here == target || (here >= 0 && self.neighbors(here as u32).contains(&word18))
    }
    /// `0x54ECF0` on the tile: rectangle +8, its island (0 for a null rectangle).
    fn tile_island(&self, tile: usize) -> Option<u32> {
        let rectangle = (*self.topology.tile_rectangles.get(tile)?)?;
        let island = self.topology.rectangles.get(rectangle as usize)?.island;
        (island != 0).then_some(island as u32)
    }
}

/// `0x7ACB00` on the map's tile flags, what `0x5925E0` runs for a generated map.
fn static_analysis(world: &World) -> Option<Topology> {
    let m = &world.map;
    let shift = m.cell_size.log2();
    if m.width == 0 || m.height == 0 || m.flags.len() != (m.width * m.height) as usize || shift.fract() != 0.0 || shift < 0.0 {
        return None;
    }
    let rule = |name: &str, shipped: f32| world.data.rule_ci(name).filter(|v| v.is_finite()).unwrap_or(shipped);
    // 7AA180: trunc(floor(width * pow((W + H) * 0.5 / base, exp) + 0.5)).
    let mean = (f64::from(m.width) + f64::from(m.height)) * 0.5 / f64::from(rule("StaticMapBaseMapWidth", 64.0));
    let max_width = (mean.powf(f64::from(rule("StaticMapMaxRectangleWidthExp", f32::from_bits(0x3eaa_a64c)))) * f64::from(rule("StaticMapMaxRectangleWidth", 64.0)) + 0.5).floor() as i32;
    let params = StaticMapParameters {
        starting_square_size: rule("StaticMapStartingSquareSize", 10.0) as i32,
        use_fast_square_reduction: rule("StaticMapUseFastSquareReduction", 1.0) != 0.0,
        fast_square_reduction_mult: rule("StaticMapFastSquareReductionMult", 3.0) as i32,
        fast_square_reduction_div: rule("StaticMapFastSquareReductionDiv", 4.0) as i32,
        min_region_size: rule("StaticMapMinRegionSize", 9.0) as i32,
        min_region_width: rule("StaticMapMinRegionWidth", 3.0) as i32,
        max_region_size_for_expansion: rule("StaticMapMaxRegionSizeForExpansion", 1000.0) as i32,
        max_rectangle_width: max_width,
    };
    let tiles: Vec<StaticTile> = m.flags.iter().map(|&f| StaticTile { flags20: f & !crate::path::OCCUPIED, definition_type: (f & crate::map::TERRAIN_TYPE_MASK) >> crate::map::TERRAIN_TYPE_SHIFT }).collect();
    let extent = [(m.width as f32 * m.cell_size).to_bits(), (m.height as f32 * m.cell_size).to_bits()];
    let layout = GridLayout { width: m.width as i32, height: m.height as i32, coordinate_shift: shift as u32, extent_bits: extent };
    let grid = StaticGrid { layout, cell_size_bits: m.cell_size.to_bits(), tiles: &tiles };
    build(&grid, &params, PC).ok().map(|a| a.topology)
}

/// `TAIUseRegionPathPlanner` (shipped true).
fn region_planner(world: &World) -> bool {
    world.data.rule_ci("TAIUseRegionPathPlanner").is_none_or(|v| v != 0.0)
}

/// One Solve's world.
struct Host<'a, 'q> {
    index: &'a mut SpatialIndex,
    geometry: &'a QueryGeometry,
    policy: &'a mut Policy<'q>,
    regions: Option<&'a mut Regions>,
    world: &'q World,
    /// Solve's descriptor argument; its position follows `0x552250`.
    query: PreparedQuery,
    map: gp::GMapView,
    tile: TileGrid,
    max_speed_bits: u32,
    penalty_bits: u32,
    walk: Option<TileWalk>,
    /// The `0x553160` / `0x5586D0` probes' +0x6C mask of the line test.
    probe_mask: u32,
    probe: Option<PreparedQuery>,
}

fn end(what: impl std::fmt::Display) -> endpoint::Error {
    endpoint::Error::World(what.to_string())
}
fn se(what: impl std::fmt::Display) -> search::SearchError {
    search::SearchError::World(what.to_string())
}
fn re(what: impl std::fmt::Display) -> region::Error {
    region::Error::World(what.to_string())
}
fn gpe(what: impl std::fmt::Display) -> gp::Error {
    gp::Error::Spatial(aa_spatial::Error::World(what.to_string()))
}

impl<'q> Host<'_, 'q> {
    fn view(&mut self) -> CollisionView<'_, Policy<'q>> {
        CollisionView { index: &mut *self.index, policy: &mut *self.policy }
    }
    /// `0x522DD0`: every tile of the line has none of the probe's mask.
    fn line_clear(&mut self, from: [u32; 2], to: [u32; 2], mask: u32) -> Result<bool, String> {
        let mut walk = TileWalk::new(self.tile, from, to, PC).map_err(|e| e.to_string())?;
        let mut budget = walk.remaining_candidates();
        while let Some(cell) = walk.next_cell_with_budget(&mut budget).map_err(|e| e.to_string())? {
            if self.index.tile(cell).map_err(|e| e.to_string())?.flags20 & mask != 0 {
                return Ok(false);
            }
        }
        Ok(true)
    }
    /// `0x4F3C40(GWorld, source, query)`.
    fn placement(&mut self, query: &PreparedQuery) -> gp::Result<u32> {
        let source = query.descriptor.source_actor;
        let map = self.map;
        Ok(gp::placement_result(&map, source, query, self)?.code())
    }
}

impl gp::CrossingWorld for Host<'_, '_> {
    fn actor_virtual_b4(&mut self, _: ActorKey) -> gp::Result<bool> {
        Ok(true)
    }
    /// `0x4F2220`. Rendition: no wall footprints.
    fn structure_at(&mut self, _: [u32; 2]) -> gp::Result<Option<ActorKey>> {
        Ok(None)
    }
    fn actor_4d36f0(&mut self, _: ActorKey, _: ActorKey) -> gp::Result<bool> {
        Ok(false)
    }
    fn structure_virtual_168(&mut self, _: ActorKey, _: ActorKey) -> gp::Result<bool> {
        Ok(false)
    }
    fn footprint_585280(&mut self, _: ActorKey, _: ActorKey) -> gp::Result<bool> {
        Ok(false)
    }
    fn actor_word_90_present(&mut self, _: ActorKey) -> gp::Result<bool> {
        Ok(false)
    }
    fn actor_virtual_100(&mut self, _: ActorKey) -> gp::Result<Option<ActorKey>> {
        Ok(None)
    }
    fn footprint_585160(&mut self, _: ActorKey) -> gp::Result<bool> {
        Ok(false)
    }
    fn actor_virtual_164(&mut self, _: ActorKey, _: ActorKey) -> gp::Result<bool> {
        Ok(true)
    }
}
impl gp::PlacementWorld for Host<'_, '_> {
    fn actor_position(&mut self, actor: ActorKey) -> gp::Result<[u32; 2]> {
        Ok(self.index.member(actor).map_err(gpe)?.actor.position)
    }
    /// The tile index stands for the tile's rectangle: [`Regions::tile_island`].
    fn tile_region_node(&mut self, tile: usize) -> gp::Result<Option<u32>> {
        Ok(Some(tile as u32))
    }
    fn region_node_region(&mut self, node: u32) -> gp::Result<Option<u32>> {
        Ok(self.regions.as_ref().and_then(|r| r.tile_island(node as usize)))
    }
    fn first_hit(&mut self, query: &PreparedQuery) -> gp::Result<Hit> {
        let geometry = self.geometry;
        query::first_hit(self.index, geometry, query, &mut *self.policy).map_err(gpe)
    }
}
impl gp::DescriptorWorld for Host<'_, '_> {
    fn set_descriptor_position(&mut self, descriptor: &mut PreparedQuery, point: [u32; 2]) -> gp::Result<()> {
        descriptor.set_position(self.index, point, &*self.policy).map_err(gpe)
    }
    fn placement_code(&mut self, _: Option<ActorKey>, descriptor: &PreparedQuery) -> gp::Result<u32> {
        self.placement(descriptor)
    }
}
impl gp::EndpointWorld for Host<'_, '_> {
    fn tile_byte_20(&mut self, tile: usize) -> gp::Result<u8> {
        let width = self.map.width as usize;
        Ok(self.index.tile([(tile % width) as i32, (tile / width) as i32]).map_err(gpe)?.flags20 as u8)
    }
    fn tile_blocked(&mut self, tile: usize, descriptor: &mut PreparedQuery) -> gp::Result<bool> {
        let width = self.map.width as usize;
        self.view().blocked([(tile % width) as i32, (tile / width) as i32], &mut descriptor.descriptor).map_err(gpe)
    }
}
impl endpoint::World for Host<'_, '_> {
    /// `0x53CE40`.
    fn acceptable(&mut self, _: [u32; 2], candidate: [u32; 2]) -> endpoint::Result<bool> {
        let map = self.map;
        let mut query = self.query.clone();
        let accepted = gp::endpoint_acceptable(&map, candidate, &mut query, self).map_err(end)?;
        self.query = query;
        Ok(accepted)
    }
    /// `0x53CA30`. Rendition: no tile holds a wall thing.
    fn displace_blocked(&mut self, _: [u32; 2], _: &mut [u32; 2]) -> endpoint::Result<bool> {
        Ok(false)
    }
}
impl search::SearchWorld for Host<'_, '_> {
    /// `0x54F370`.
    fn collision_blocked(&mut self, _: &SearchGrid, cell: [i32; 2]) -> search::Result<bool> {
        let mut descriptor = self.query.descriptor.clone();
        let blocked = self.view().blocked(cell, &mut descriptor).map_err(se)?;
        self.query.descriptor = descriptor;
        Ok(blocked)
    }
    fn accumulated_cost(&mut self, grid: &SearchGrid, current: search::QueueEntry, next: [i32; 2], p: Precision) -> search::Result<u32> {
        let mut state = cost::State { current, maximum_speed_bits: self.max_speed_bits, collision_owner: self.query.descriptor.collision_owner };
        cost::accumulated_cost(grid.cell_size_bits, next, &mut state, p, self).map_err(se)
    }
}
impl cost::World for Host<'_, '_> {
    /// Rendition: no tile holds a wall thing.
    fn tile_actor(&mut self, _: [i32; 2]) -> cost::Result<Option<u32>> {
        Ok(None)
    }
    fn has_wall_component(&self, _: u32) -> cost::Result<bool> {
        Ok(false)
    }
    fn outer_actor(&mut self, _: u32, _: &mut cost::State) -> cost::Result<Option<u32>> {
        Ok(None)
    }
    fn pathing_wall_cost_bits(&mut self) -> cost::Result<u32> {
        Err(cost::Error::World("no wall things".into()))
    }
    fn body_health_bits(&mut self, _: u32) -> cost::Result<Option<[u32; 2]>> {
        Ok(None)
    }
    fn prepare_terrain_penalty(&mut self) -> cost::Result<()> {
        Ok(())
    }
    /// `0x54ECE0` +0x38.
    fn terrain_speed_bits(&mut self, cell: [i32; 2]) -> cost::Result<u32> {
        let m = &self.world.map;
        let at = cell[1] as usize * m.width as usize + cell[0] as usize;
        Ok(m.speed_limit.get(at).copied().unwrap_or(0.0).to_bits())
    }
    fn terrain_change_penalty_bits(&mut self) -> cost::Result<u32> {
        Ok(self.penalty_bits)
    }
}
impl materialize::World for Host<'_, '_> {
    fn begin_trace(&mut self, from: [u32; 2], to: [u32; 2], _: &mut materialize::State, _: &mut OwnedPath) -> materialize::Result<()> {
        self.walk = Some(TileWalk::new(self.tile, from, to, PC).map_err(|e| materialize::Error::World(e.to_string()))?);
        Ok(())
    }
    fn next_trace_cell(&mut self, _: &mut materialize::State, _: &mut OwnedPath) -> materialize::Result<Option<[i32; 2]>> {
        let Some(walk) = self.walk.as_mut() else {
            return Ok(None);
        };
        let mut budget = walk.remaining_candidates();
        walk.next_cell_with_budget(&mut budget).map_err(|e| materialize::Error::World(e.to_string()))
    }
    /// `0x54F370` with the descriptor's +0x28 as the state holds it (zero in a trace).
    fn cell_blocked(&mut self, cell: [i32; 2], state: &mut materialize::State, _: &mut OwnedPath) -> materialize::Result<bool> {
        let mut descriptor = self.query.descriptor.clone();
        descriptor.collision_owner = (state.collision_word28 != 0).then_some(state.collision_word28);
        let flags = self.index.tile(cell).map_err(|e| materialize::Error::World(e.to_string()))?.flags20;
        if flags & 1 != 0 {
            return Ok(true);
        }
        self.view().blocked(cell, &mut descriptor).map_err(|e| materialize::Error::World(e.to_string()))
    }
}
impl solver::World for Host<'_, '_> {
    fn disable_path_logging(&mut self) {}
    fn use_region_pather(&self) -> bool {
        self.regions.is_some() && region_planner(self.world)
    }
    fn collision_actor_flags(&self) -> Option<u32> {
        let source = self.query.descriptor.source_actor?;
        self.index.member(source).ok().map(|m| m.actor.flags)
    }
    /// `0x553160(0x100, 0)` then `0x5586D0`.
    fn begin_region_probe(&mut self) -> solver::Result<()> {
        self.probe_mask = 0x100;
        Ok(())
    }
    fn region_line_clear(&mut self, start: [u32; 2], goal: [u32; 2]) -> solver::Result<bool> {
        let mask = self.probe_mask;
        let r = self.line_clear(start, goal, mask).map_err(solver::Error::World);
        r
    }
    fn region_path(&mut self, start: [u32; 2], goal: [u32; 2], path: &mut OwnedPath) -> solver::Result<i32> {
        region::prepass(path, start, goal, PC, self).map_err(|e| solver::Error::World(e.to_string()))
    }
    fn collision_actor_id(&self) -> Option<u32> {
        self.query.descriptor.source_actor
    }
    fn collision_target_id(&self) -> Option<u32> {
        self.query.descriptor.ignore_actor
    }
    fn collision_word28(&self) -> u32 {
        self.query.descriptor.collision_owner.unwrap_or(0)
    }
}
impl region::World for Host<'_, '_> {
    fn region_at(&mut self, point: [u32; 2]) -> region::Result<i32> {
        Ok(self.regions.as_ref().map_or(-1, |r| r.region_at(point)))
    }
    /// `0x53F320` with its cached-route flag clear.
    fn plan_regions(&mut self, start: i32, goal: i32) -> region::Result<RegionRoute> {
        let Some(r) = self.regions.as_mut() else {
            return Ok(RegionRoute { status: 0, nodes: Vec::new() });
        };
        if start < 0 || goal < 0 {
            return Ok(RegionRoute { status: 0, nodes: Vec::new() });
        }
        let Regions { graph, state, .. } = &mut **r;
        let request = region_search::Request { start: start as u32, goal: goal as u32, precision: PC, queue_limit: Some(region_search::NATIVE_QUEUE_LIMIT) };
        let r = region_search::region_route(graph, request, state).map_err(re);
        r
    }
    fn region_center(&self, id: i32) -> region::Result<[u32; 2]> {
        self.regions.as_ref().and_then(|r| r.graph.nodes.get(id as usize)).map(|n| n.position_bits).ok_or_else(|| re("no such region"))
    }
    /// `0x5586D0` on Solve's descriptor.
    fn begin_probe(&mut self) -> region::Result<()> {
        self.probe = Some(self.query.clone());
        Ok(())
    }
    fn place_context(&mut self, point: [u32; 2]) -> region::Result<()> {
        let mut q = self.probe.take().ok_or_else(|| re("no probe"))?;
        q.set_position(self.index, point, &*self.policy).map_err(re)?;
        self.probe = Some(q);
        Ok(())
    }
    fn blocked_here(&mut self) -> region::Result<i32> {
        let q = self.probe.take().ok_or_else(|| re("no probe"))?;
        let code = self.placement(&q).map_err(re)?;
        self.probe = Some(q);
        Ok(code as i32)
    }
    /// `0x522DD0` on the probe: its mask is Solve's descriptor's.
    fn line_clear(&mut self, start: [u32; 2], goal: [u32; 2]) -> region::Result<bool> {
        let mask = self.probe.as_ref().map_or(self.query.descriptor.mask, |q| q.descriptor.mask);
        let r = Host::line_clear(self, start, goal, mask).map_err(re);
        r
    }
    /// `0x53D340` with the start as the fallback.
    fn adjust_endpoint(&mut self, start: [u32; 2], goal: &mut [u32; 2]) -> region::Result<bool> {
        let bounds = endpoint::Bounds { cell_size_bits: self.map.cell_size_bits, extent_bits: self.map.extent_bits };
        endpoint::adjust(start, goal, start, bounds, PC, self).map_err(re)
    }
}

/// `0x53E1F0`: 0 no route, 1 reached, 2 the part that could be found. `None` when the
/// map's tile grid cannot take the native search (no blocked border); the caller's
/// tile solver takes those.
pub(super) fn solve(ms: &mut MovementState, world: &World, query: PreparedQuery, start: [u32; 2], goal: [u32; 2], option_word: u32, max_speed: f32, penalty: f32, path: &mut OwnedPath) -> Option<i32> {
    let mut boxed = ms.space.take()?;
    let mut regions = ms.regions.take();
    if regions.is_none() {
        regions = Regions::of(world);
    }
    let status = {
        let Space { index, geometry, grid, .. } = &mut *boxed;
        solve_on(index, geometry, grid, regions.as_deref_mut(), &*ms, world, query, start, goal, option_word, max_speed, penalty, path)
    };
    ms.regions = regions;
    ms.space = Some(boxed);
    status
}

#[allow(clippy::too_many_arguments)]
fn solve_on(index: &mut SpatialIndex, geometry: &QueryGeometry, grid: &mut SearchGrid, regions: Option<&mut Regions>, ms: &MovementState, world: &World, query: PreparedQuery, start: [u32; 2], goal: [u32; 2], option_word: u32, max_speed: f32, penalty: f32, path: &mut OwnedPath) -> Option<i32> {
    let cell_bits = world.map.cell_size.to_bits();
    let extent = geometry.extent_bits;
    let g = index.grid().clone();
    let map = gp::GMapView { width: g.dimensions[0], height: g.dimensions[1], coordinate_shift: g.coordinate_shift, cell_size_bits: g.cell_size_bits, extent_bits: extent };
    let tile = TileGrid { width: g.dimensions[0], height: g.dimensions[1], coordinate_shift: g.coordinate_shift, cell_size_bits: g.cell_size_bits };
    let mut policy = Policy { ms, world };
    let mut host = Host {
        index,
        geometry,
        policy: &mut policy,
        regions,
        world,
        query,
        map,
        tile,
        max_speed_bits: max_speed.to_bits(),
        penalty_bits: penalty.to_bits(),
        walk: None,
        probe_mask: 0x100,
        probe: None,
    };
    let mut state = solver::StrategyState {
        route: materialize::State { start_cell: [0, 0], goal_cell: [0, 0], start_bits: [0, 0], goal_bits: [0, 0], path_linked: false, collision_linked: false, collision_word28: 0 },
        actor_id: 0,
        target_id: 0,
        option_word: 0,
    };
    let options = solver::Options { bounds: endpoint::Bounds { cell_size_bits: cell_bits, extent_bits: extent }, precision: PC, queue_limit: Some(search::NATIVE_QUEUE_LIMIT) };
    let request = solver::Request { start_bits: start, goal_bits: goal, option_word };
    let result = solver::solve(grid, request, &mut state, path, options, &mut host);
    match result {
        Ok(status) => Some(status),
        Err(_) => None,
    }
}

impl gp::NearestPlacementWorld for Host<'_, '_> {
    /// `actor` is the thing itself: `0x4F46E0`'s step from an organization to its
    /// leader is the caller's.
    fn actor_virtual_b4(&mut self, _: ActorKey) -> gp::Result<bool> {
        Ok(false)
    }
    fn actor_word_7c(&mut self, _: ActorKey) -> gp::Result<Option<u32>> {
        Ok(None)
    }
    fn object_word_40(&mut self, _: u32) -> gp::Result<Option<ActorKey>> {
        Ok(None)
    }
    fn prepare_descriptor(&mut self, _: ActorKey) -> gp::Result<PreparedQuery> {
        Ok(self.query.clone())
    }
    fn actor_facing(&mut self, actor: ActorKey) -> gp::Result<[u32; 2]> {
        Ok(self.index.member(actor).map_err(gpe)?.actor.facing)
    }
}

/// `0x4F46E0(actor, &point, radius)` with `actor` the organization's leader. `None`
/// when the map's things are not indexed.
pub(super) fn nearest_placement(ms: &mut MovementState, world: &World, actor: ActorId, point: &mut [u32; 2], radius_bits: u32) -> Option<bool> {
    let query = query_of(ms, world, actor, None, 0)?;
    let mut boxed = ms.space.take()?;
    let mut regions = ms.regions.take();
    if regions.is_none() {
        regions = Regions::of(world);
    }
    let result = {
        let Space { index, geometry, .. } = &mut *boxed;
        let extent = geometry.extent_bits;
        let g = index.grid().clone();
        let map = gp::GMapView { width: g.dimensions[0], height: g.dimensions[1], coordinate_shift: g.coordinate_shift, cell_size_bits: g.cell_size_bits, extent_bits: extent };
        let tile = TileGrid { width: g.dimensions[0], height: g.dimensions[1], coordinate_shift: g.coordinate_shift, cell_size_bits: g.cell_size_bits };
        let mut policy = Policy { ms: &*ms, world };
        let mut host = Host {
            index,
            geometry,
            policy: &mut policy,
            regions: regions.as_deref_mut(),
            world,
            query,
            map,
            tile,
            max_speed_bits: 0,
            penalty_bits: 0,
            walk: None,
            probe_mask: 0x100,
            probe: None,
        };
        gp::nearest_placement(&map, Some(space::actor_key(actor)), point, radius_bits, PC, &mut host)
    };
    ms.regions = regions;
    ms.space = Some(boxed);
    result.ok()
}

/// The descriptor `0x51EA20` and `0x5336B0` hand to Solve: `0x554350(actor, 0)`.
pub(super) fn query_of(ms: &mut MovementState, world: &World, actor: ActorId, ignore: Option<ActorKey>, word28: u32) -> Option<PreparedQuery> {
    let s = ms.space.take()?;
    let policy = Policy { ms: &*ms, world };
    let query = space::query_for(&s, &policy, actor, false);
    ms.space = Some(s);
    let mut query = query?;
    query.required_actor_flags |= NO_MOVER_FLAG;
    query.descriptor.ignore_actor = ignore;
    query.descriptor.collision_owner = (word28 != 0).then_some(word28);
    Some(query)
}
