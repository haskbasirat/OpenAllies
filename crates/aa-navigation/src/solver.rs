//! Original Solve sequencing around the concrete search and path materializer.
use crate::{OwnedPath, endpoint, materialize, search};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error(transparent)]
    Endpoint(#[from] endpoint::Error),
    #[error(transparent)]
    Search(#[from] search::SearchError),
    #[error(transparent)]
    Materialize(#[from] materialize::Error),
    #[error("enabled native region prepass requires the collision actor")]
    MissingRegionActor,
    #[error("global bounds and search grid describe different cell sizes")]
    MismatchedGrid,
    #[error("solver world boundary failed: {0}")]
    World(String),
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Request {
    pub start_bits: [u32; 2],
    pub goal_bits: [u32; 2],
    pub option_word: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct Options {
    pub bounds: endpoint::Bounds,
    pub precision: Precision,
    pub queue_limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyState {
    pub route: materialize::State,
    /// Native strategy+38/+3c, copied from descriptor actors' +14 identities.
    pub actor_id: u32,
    pub target_id: u32,
    pub option_word: u32,
}

/// Original region prepass and geometry/cost dependencies. Endpoint validation,
/// concrete native queue/search policy, and materialization are implemented by
/// this crate. Region topology and object-dependent world predicates are required.
pub trait World: endpoint::World + search::SearchWorld + materialize::World {
    /// Original PathLog flag is reset to false at Solve entry.
    fn disable_path_logging(&mut self);
    /// Original SVar cache+5c, read after accepted-endpoint path clearing.
    fn use_region_pather(&self) -> bool;
    fn collision_actor_flags(&self) -> Option<u32>;
    /// 553160/5586d0 temporary region probe, with flags0x100 and second arg0.
    fn begin_region_probe(&mut self) -> Result<()>;
    /// Original522dd0; false invokes the region-route prepass below.
    fn region_line_clear(&mut self, start: [u32; 2], goal: [u32; 2]) -> Result<bool>;
    /// Entire53dc00; may prepend region waypoints and update path.word18.
    fn region_path(&mut self, start: [u32; 2], goal: [u32; 2], path: &mut OwnedPath)
    -> Result<i32>;
    fn collision_actor_id(&self) -> Option<u32>;
    fn collision_target_id(&self) -> Option<u32>;
    fn collision_word28(&self) -> u32;
}

/// Complete52cd90 path reset, distinct from phase5 clear_nodes which preserves
/// metadata. Also used by the concrete search's native failure cleanup.
pub fn reset_path(path: &mut OwnedPath) {
    path.clear_nodes();
    path.kind1_count = 0;
    path.kind2_count = 0;
    path.word18 = u32::MAX;
    path.dirty = true;
}

/// Native53e1f0 flow with its concrete53d8d0 search and53d500 materialization.
/// The caller supplies stable map data and an explicit queue policy. Use the
/// native queue limit for exact legacy bounded-search behavior; changing that
/// policy can change partial-route results and is an explicit caller choice.
pub fn solve<W: World>(
    grid: &mut search::SearchGrid,
    request: Request,
    state: &mut StrategyState,
    path: &mut OwnedPath,
    options: Options,
    world: &mut W,
) -> Result<i32> {
    let Options {
        bounds,
        precision: p,
        queue_limit,
    } = options;
    if bounds.cell_size_bits != grid.cell_size_bits {
        return Err(Error::MismatchedGrid);
    }
    world.disable_path_logging();
    let mut goal = request.goal_bits;
    if !endpoint::adjust(
        request.start_bits,
        &mut goal,
        request.start_bits,
        bounds,
        p,
        world,
    )? {
        return Ok(0);
    }
    let x = Binary::from_f32_bits(
        Binary::from_f32_bits(goal[0])?
            .sub(&Binary::from_f32_bits(request.start_bits[0])?, p)?
            .store_f32()?,
    )?;
    let y = Binary::from_f32_bits(
        Binary::from_f32_bits(goal[1])?
            .sub(&Binary::from_f32_bits(request.start_bits[1])?, p)?
            .store_f32()?,
    )?;
    //409230 reports zero when the stored delta's norm is <=0.0001.
    let norm = x.mul(&x, p)?.add(&y.mul(&y, p)?, p)?.sqrt(p)?;
    if norm.compare(&Binary::from_f32_bits(0x38d1b717)?) != Ordering::Greater {
        return Ok(0);
    }
    reset_path(path);
    if world.use_region_pather() {
        let flags = world
            .collision_actor_flags()
            .ok_or(Error::MissingRegionActor)?;
        if flags & (1 << 18) == 0 {
            world.begin_region_probe()?;
            if !world.region_line_clear(request.start_bits, goal)? {
                let result = world.region_path(request.start_bits, goal, path)?;
                if result != 1 {
                    return Ok(result);
                }
                if let Some(first) = path.nodes.first() {
                    goal = [first.words[0], first.words[1]];
                }
            }
        }
    }
    let shift = grid.coordinate_shift & 31;
    let cells = |bits: [u32; 2]| -> Result<[i32; 2]> {
        Ok([
            Binary::from_f32_bits(bits[0])?.truncate_i32()? >> shift,
            Binary::from_f32_bits(bits[1])?.truncate_i32()? >> shift,
        ])
    };
    state.route = materialize::State {
        start_cell: cells(request.start_bits)?,
        goal_cell: cells(goal)?,
        start_bits: request.start_bits,
        goal_bits: goal,
        path_linked: true,
        collision_linked: true,
        collision_word28: world.collision_word28(),
    };
    state.option_word = request.option_word;
    state.actor_id = world.collision_actor_id().unwrap_or(0);
    state.target_id = world.collision_target_id().unwrap_or(0);
    let input = search::SearchInput {
        start_cell: state.route.start_cell,
        goal_cell: state.route.goal_cell,
        goal_world_bits: goal,
        precision: p,
        queue_limit,
    };
    let outcome = search::search(grid, input, world)?;
    if let Some(end) = outcome.end_cell {
        materialize::materialize(&mut state.route, path, end, grid, p, world)?;
    } else {
        // Concrete strategy53c450, reached by native4f0590 failure cleanup.
        reset_path(path);
        state.route.path_linked = false;
        state.route.collision_linked = false;
    }
    Ok(outcome.status as i32)
}
