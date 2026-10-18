//! Original SAI actor travel cost `7A1B30` and travel time `7A1C10`
//! (`Source/Sai`, SAI actor virtuals). The region-route block
//! `7A1BA3..7A1BEC` (a fresh list, `53F320(list, start, goal, 1)` on the
//! persistent solver `[[B36534]+234]`, `53E5B0`, `5C2CD0`) is
//! `aa_navigation::route_cache::route_cost`; [`RouteServices`] binds it.
//!
//! `7A1B30(location)`:
//! 1. actor virtual +C (unavailable) -> binary32 -1.0 ([9FDC48]);
//! 2. a cached cost (+14, binary32) that is not below 0 is returned when the
//!    +10 object's virtual +20 yields this same location;
//! 3. otherwise the route cost from the region word at actor virtual +8 to
//!    the location's region word (+0), stored to binary32 (`7A1BDF`).
//!
//! `7A1C10(location)`: a null location costs +0.0; an unavailable actor
//! -1.0; else the binary32 cost of `7A1B30` divided by actor virtual +10
//! (`fdivr` at the x87 precision), returned unstored. Virtual +C is asked
//! again inside `7A1B30`, as natively.
use crate::{Result, x87::num};
use aa_navigation::region_search::{NATIVE_QUEUE_LIMIT, RegionGraph, Request, State};
use aa_navigation::route_cache::{RouteCache, route_cost};
use aa_numeric::{Binary, Precision};

/// binary32 -1.0 at [9FDC48] (also the empty-route cost of `53E5B0`).
pub const UNAVAILABLE_BITS: u32 = 0xbf80_0000;

/// SAI actor virtuals and fields read by `7A1B30`/`7A1C10`, plus the route
/// query. `actor` and `location` are opaque handles (native pointers).
pub trait TravelWorld {
    /// Actor virtual +C.
    fn unavailable(&mut self, actor: u32) -> Result<bool>;
    /// Actor +14: the cached cost (binary32 bits).
    fn cached_cost(&mut self, actor: u32) -> Result<u32>;
    /// Virtual +20 of the actor's +10 object: the location of the cache.
    fn cached_location(&mut self, actor: u32) -> Result<u32>;
    /// `[actor virtual +8]`: the region the route starts from.
    fn start_region(&mut self, actor: u32) -> Result<u32>;
    /// Location +0: its region.
    fn location_region(&mut self, location: u32) -> Result<u32>;
    /// Actor virtual +10 (ST0, unstored).
    fn speed(&mut self, actor: u32) -> Result<Binary>;
    /// `7A1BA3..7A1BEC`: binary32 bits of the route cost (search status
    /// ignored, as natively).
    fn route_cost(&mut self, start: u32, goal: u32, p: Precision) -> Result<u32>;
}

/// `7A1B30`: binary32 bits of the actor's route cost to `location`.
pub fn travel_cost(
    world: &mut impl TravelWorld,
    actor: u32,
    location: u32,
    p: Precision,
) -> Result<u32> {
    if world.unavailable(actor)? {
        return Ok(UNAVAILABLE_BITS);
    }
    let cached = world.cached_cost(actor)?;
    // fcomp 0; test ah,1: a cache below zero (C0) is skipped.
    let below_zero = f32::from_bits(cached).partial_cmp(&0.0) == Some(std::cmp::Ordering::Less);
    if !below_zero && world.cached_location(actor)? == location {
        return Ok(cached);
    }
    let start = world.start_region(actor)?;
    let goal = world.location_region(location)?;
    world.route_cost(start, goal, p)
}

/// `7A1C10`: the actor's travel time to `location` (unstored ST0).
pub fn travel_time(
    world: &mut impl TravelWorld,
    actor: u32,
    location: u32,
    p: Precision,
) -> Result<Binary> {
    if location == 0 {
        return Ok(Binary::from_i32(0));
    }
    if world.unavailable(actor)? {
        return num(Binary::from_f32_bits(UNAVAILABLE_BITS));
    }
    let cost = num(Binary::from_f32_bits(travel_cost(
        world, actor, location, p,
    )?))?;
    let speed = world.speed(actor)?;
    num(cost.div(&speed, p))
}

/// The persistent SAI route solver state the route query runs on: the
/// region graph and solver of `[[B36534]+234]`, the process-wide busy byte
/// (`B3D43D`, shared with the water flood fill) and the manager's route cache
/// (`[[B3652C]+40]`, `None` exactly when that pointer is null).
///
/// Cache lifecycle (owned by whoever holds the SAI map manager):
/// `RouteCache::build(region count)` where `7A0D40` runs (just before the SAI
/// ready byte `[B36528]+20` is set), `None` wherever `7AC9A0` runs (topology
/// load `7ACB00`, destructor `7ACE50`), `RouteCache::reset` for
/// `ResetPathPlanner` (`5D8560`). Queries mutate the solver epoch, node bytes
/// and cache: run them in native order.
pub struct RouteServices<'a> {
    pub graph: &'a mut RegionGraph,
    pub state: &'a mut State,
    pub cache: Option<&'a mut RouteCache>,
}

impl RouteServices<'_> {
    /// `7A1BA3..7A1BEC` through `aa_navigation::route_cache::route_cost`
    /// (queue limit 0x2000 as built by `53F290`).
    pub fn route_cost(&mut self, start: u32, goal: u32, p: Precision) -> Result<u32> {
        let request = Request {
            start,
            goal,
            precision: p,
            queue_limit: Some(NATIVE_QUEUE_LIMIT),
        };
        let cost = route_cost(self.graph, request, self.state, self.cache.as_deref_mut())
            .map_err(|e| crate::Error::World(format!("SAI route query: {e}")))?;
        Ok(cost.cost_bits)
    }
}
