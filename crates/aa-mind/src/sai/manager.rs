//! The SAI manager objects and their lifecycle (Source/Sai/SAI.cpp,
//! SAIMap.cpp), with the host design that replaces the native fibers.
//!
//! Native (ported here):
//! - `KKC_SAI` constructor `7A0AA0` ([`Sai::new`]): fields, the singleton
//!   `[B36528]`, and the SAI RNG `+2C` seeded (`85D8A0`) from the
//!   millisecond timer `59ABA0` (QueryPerformanceCounter): [`SaiRng`]
//!   (`85D840` draws, the same LCG as the world RNG but its own state).
//! - SAI reset `79FF10` ([`Sai::reset`]): every AI player's `7B1760`, the
//!   goal-engine and RAI fibers deleted, the command queue `+78` cleared
//!   (`79F9B0`), the general-power list `+80` released, ready byte cleared.
//! - SAI map region finalisation `7ABC30` ([`SaiMapLifecycle::finalize`]):
//!   dynamic flag `+5`, region areas `+C`, bottlenecks (aa_navigation
//!   `preparation::prepare`) and a fresh route cache `+40`
//!   (`RouteCache::build`); it runs inside the SAI startup `7A0D40`
//!   (`aa_navigation::startup::initialize`, `World::prepare_topology`).
//! - `KKC_SAIMap::ClearStaticMapAnalysis` `7AC9A0`
//!   ([`SaiMapLifecycle::clear`]): SAI reset when the dynamic map exists,
//!   islands/regions/rectangles released, the route cache dropped, `+4` and
//!   `+5` cleared (reached from topology load `7ACB00` and the destructor
//!   `7ACE50`).
//! - `ResetPathPlanner` `5D8560` ([`SaiMapLifecycle::reset_path_planner`]).
//! - `KKC_SAI::Think` `7A0080` ([`think`]): the gating, the per-call
//!   millisecond budgets and their split, which phases run, and the send of
//!   due commands (`79F9F0` pop, `7AF220`, `7ADC60`).
//! - The white-fog timer `7B8E70` and its zone hook `7A9A60`
//!   ([`white_fog_tile_hidden`]).
//!
//! Host design (not native; recorded in research/phase8-sai/README.txt):
//! native Think switches to three Win32 fibers (goal engine `79F660`, RAI
//! `79EF60`, MGAI `79E830`) that run until a yield point (`79DF70`) sees the
//! millisecond timer pass the slice deadline. Rust replaces each fiber by a
//! [`ThinkHost`] phase runner given the same deadline; a runner may keep its
//! own continuation (a dedicated thread handed a baton, or a resumable
//! state machine) and must stop at the same yield points. Because the SAI
//! output is only orders (`7AF220`) and the SAI RNG is not the world RNG,
//! the whole SAI may run on a host-local worker thread: the simulation sees
//! its decisions only as order requests entering the normal command path,
//! exactly as native AI orders do. Wall-clock budgets make SAI decisions
//! host-timing dependent in the original too; they are explicit inputs here.
use super::command::SaiCommandData;
use crate::{Error, Result};
use aa_navigation::route_cache::{self, RouteCache};
use aa_numeric::{Binary, Precision};
use std::collections::VecDeque;

/// `85D8A0` default / zero-seed replacement state.
const DEFAULT_RNG_STATE: u32 = 0x8e4b_12a5;

/// The SAI RNG object `[B36528]+2C` (4 bytes): `85D8A0` seeding and
/// `85D840` draws. Never the world RNG.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaiRng {
    pub state: u32,
}

impl SaiRng {
    /// `85D8A0(seed)`: `(seed << 16 | seed) ^ 0x8E4B12A5`, the default when
    /// that is zero.
    pub fn from_seed(seed: u32) -> Self {
        let mixed = ((seed << 16) | seed) ^ DEFAULT_RNG_STATE;
        SaiRng {
            state: if mixed == 0 { DEFAULT_RNG_STATE } else { mixed },
        }
    }
    /// `85D840`: bits 16..48 of `state * 0x41C64E6D + 0x3039`.
    pub fn next_u32(&mut self) -> u32 {
        let wide = u64::from(self.state) * 0x41c6_4e6d + 0x3039;
        self.state = wide as u32;
        (wide >> 16) as u32
    }
}

/// `KKC_SAI` (vtable A9C644), the singleton at `[B36528]`.
#[derive(Clone, Debug, PartialEq)]
pub struct Sai {
    /// `+20` ready byte (set by the startup `7A0D40`).
    pub ready: bool,
    /// `+24` the world revision `[B3CC94]` the fibers were created for.
    /// The constructor leaves it uninitialized (`None`): the first Think
    /// then behaves as for a different revision, which only clears fibers
    /// and an MGAI object that do not exist yet.
    pub revision: Option<u32>,
    /// `+28` the MGAI object exists (owned by the metagame AI).
    pub metagame: bool,
    /// `+2C` the SAI RNG.
    pub rng: SaiRng,
    /// `+30`/`+34`/`+38` goal-engine, RAI and MGAI fibers exist; `+3C` the
    /// thread converted to a fiber.
    pub goal_fiber: bool,
    pub rai_fiber: bool,
    pub metagame_fiber: bool,
    pub main_fiber: bool,
    /// `+40` goal-engine slice deadline, `+44` RAI slice deadline, `+48`
    /// MGAI deadline (milliseconds of `59ABA0`; the constructor leaves `+48`
    /// uninitialized, Think writes it before use).
    pub goal_deadline: u32,
    pub rai_deadline: u32,
    pub metagame_deadline: u32,
    /// Native+4C/+50 and+54/+58, distinct player work lists.7A0AA0 constructs
    /// both empty;79FC40 prepends a newly selected AI slot to both lists.
    pub players_4c: VecDeque<u32>,
    pub players_54: VecDeque<u32>,
    /// `+5C..+5E` bytes (`+5C` cleared while switched to the main fiber).
    pub bytes_5c: [u8; 3],
    /// `+60/+64/+68` AI players (null slots kept); the ctor reserves 0x10.
    pub players: Vec<Option<u32>>,
    /// `+6C`/`+70` (-1 from the ctor and again at the end of `7A0D40`).
    pub word_6c: u32,
    pub word_70: u32,
    pub bytes_74: [u8; 2],
    /// `+78/+7C` command queue (due millisecond, data), front first.
    pub commands: VecDeque<(u32, SaiCommandData)>,
    /// `+80/+84` active general powers (`base_priority::GeneralPowerInstance`
    /// records are produced from it), `+88/+8C` a second list.
    pub general_powers: Vec<u32>,
    pub list_88: Vec<u32>,
    /// `+90/+94/+98` a vector reserved for 10 entries.
    pub vector_90: Vec<u32>,
}

impl Sai {
    /// Complete `7A0AA0` with the timer value `ticks` (`59ABA0`, the
    /// host's millisecond performance counter). The singleton registration
    /// (`[B36528]`, `[B40D4C]`, `[B40D48]`), the critical section `+4`
    /// (`857D60`) and the static `7A5BF0` are host concerns.
    pub fn new(ticks: u32) -> Self {
        Sai {
            ready: false,
            revision: None,
            metagame: false,
            rng: SaiRng::from_seed(ticks),
            goal_fiber: false,
            rai_fiber: false,
            metagame_fiber: false,
            main_fiber: false,
            goal_deadline: 0,
            rai_deadline: 0,
            metagame_deadline: 0,
            players_4c: VecDeque::new(),
            players_54: VecDeque::new(),
            bytes_5c: [0; 3],
            players: Vec::new(),
            word_6c: u32::MAX,
            word_70: u32::MAX,
            bytes_74: [0; 2],
            commands: VecDeque::new(),
            general_powers: Vec::new(),
            list_88: Vec::new(),
            vector_90: Vec::new(),
        }
    }

    /// `79FF10`: `reset_player` (`7B1760`) for every non-null AI player in
    /// slot order, the goal-engine and RAI fibers deleted, the command queue
    /// and general powers released, not ready.
    pub fn reset(&mut self, mut reset_player: impl FnMut(u32) -> Result<()>) -> Result<()> {
        for player in self.players.clone().into_iter().flatten() {
            reset_player(player)?;
        }
        self.goal_fiber = false;
        self.rai_fiber = false;
        self.commands.clear();
        self.general_powers.clear();
        self.ready = false;
        Ok(())
    }
}

/// The SAI map (`[B3652C]`, `KKC_SAIMap`) fields of its lifecycle.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SaiMapLifecycle {
    /// `+4` the static analysis exists.
    pub analysed: bool,
    /// `+5` the dynamic map is finalised (`7ABC30`).
    pub dynamic: bool,
    /// Region `+C` areas written by `7ABC30` (`7B9900`).
    pub areas: Vec<i32>,
    /// Region `7BC8B0` bottleneck flags.
    pub bottlenecks: Vec<bool>,
    /// `+40` the route cache.
    pub route_cache: Option<RouteCache>,
}

impl SaiMapLifecycle {
    /// `7ABC30` on a prepared topology: dynamic flag, areas, bottlenecks and
    /// a fresh route cache built from the region count.
    pub fn finalize(&mut self, prepared: &aa_navigation::preparation::Prepared) -> Result<()> {
        self.dynamic = true;
        self.areas = prepared.areas.clone();
        self.bottlenecks = prepared.bottlenecks.clone();
        self.route_cache = Some(
            RouteCache::build(prepared.areas.len())
                .map_err(|e| Error::World(format!("route cache 53F0A0: {e}")))?,
        );
        Ok(())
    }

    /// `7AC9A0` ClearStaticMapAnalysis: `reset_sai` (`79FF10` on `[B36528]`)
    /// when the dynamic map exists, then everything is released and the
    /// route cache dropped. `5D85C0(0)` on `[[B3974C]+18C]` and the tile
    /// rectangle links (`[B36534]+30` tiles `+10`) are the world owners'.
    pub fn clear(&mut self, reset_sai: impl FnOnce() -> Result<()>) -> Result<()> {
        if self.dynamic {
            reset_sai()?;
        }
        self.areas.clear();
        self.bottlenecks.clear();
        self.route_cache = None;
        self.analysed = false;
        self.dynamic = false;
        Ok(())
    }

    /// `5D8560` `ResetPathPlanner` with the current region count (the
    /// native dereferences a null cache).
    pub fn reset_path_planner(&mut self, region_count: usize) -> Result<()> {
        route_cache::reset_path_planner(self.route_cache.as_mut(), region_count)
            .map_err(|e| Error::World(format!("ResetPathPlanner 5D8560: {e}")))
    }
}

/// Global state `7A0080` reads (all plain fields except the session mode).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThinkGlobals {
    /// `[[B39940]+284]` byte (consulted only in session mode 7).
    pub mode7_enabled: bool,
    /// `[B36534]` exists, its `+DC` (signed) and `+275` byte.
    pub world: Option<(i32, bool)>,
    /// `[B3652C]` exists with `+4` (static) and `+5` (dynamic).
    pub map: Option<(bool, bool)>,
    /// `[B36550]+E4` (2 battle, 5 metagame, 4 skips).
    pub session_e4: u32,
    /// `[B3CC94]` world revision.
    pub revision: u32,
    /// `[B36554]+29` paused byte.
    pub paused: bool,
}

/// Which native fiber a step concerns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fiber {
    GoalEngine,
    Rai,
    Metagame,
}

/// Services of `7A0080`, in native order.
pub trait ThinkHost {
    /// `76AC90` on `[B36550]`: the session mode.
    fn session_mode(&mut self) -> Result<u32>;
    /// Critical section `+4` virtual `+4` (enter) / `+8` (leave).
    fn lock(&mut self) -> Result<()>;
    fn unlock(&mut self) -> Result<()>;
    /// `59ABA0` on `[B396E0]`: milliseconds.
    fn now(&mut self) -> Result<u32>;
    /// `79DF00`/`79DF20`/`79DF40` (the latter also destroys the MGAI object
    /// `+28`): drop a fiber's continuation.
    fn delete_fiber(&mut self, fiber: Fiber) -> Result<()>;
    /// `79F660`/`79EF60`/`79E830`: create a fiber's continuation.
    fn create_fiber(&mut self, fiber: Fiber) -> Result<()>;
    /// `ConvertThreadToFiber(0)`; false when it fails.
    fn convert_thread(&mut self) -> Result<bool>;
    /// `404EA0` "Error converting thread to fiber: %x" with GetLastError
    /// (SAI.cpp line 0x36C); Think continues.
    fn log_convert_failure(&mut self) -> Result<()>;
    /// `SwitchToFiber`: run the phase until its yield point passes
    /// `deadline` (the host's continuation of that fiber).
    fn run(&mut self, fiber: Fiber, deadline: u32) -> Result<()>;
    /// `7AF220` for one due command (`sai::command::send`).
    fn send(&mut self, data: SaiCommandData) -> Result<()>;
    /// `944E60` on the MGAI object after its slice.
    fn metagame_post(&mut self) -> Result<()>;
}

/// `_ftol2` (`96CC4C`) of a binary32 value: truncation to 64 bits, the low
/// word; out of range (or NaN) yields the integer indefinite (low word 0).
pub fn ftol2_low(bits: u32) -> u32 {
    let v = f64::from(f32::from_bits(bits)).trunc();
    if v.is_nan() || v < -9.223_372_036_854_776e18 || v >= 9.223_372_036_854_776e18 {
        0
    } else {
        (v as i64) as u32
    }
}

/// Complete `7A0080(budget)`; `budget` is the binary32 argument (seconds).
pub fn think(
    sai: &mut Sai,
    budget: u32,
    g: &ThinkGlobals,
    host: &mut impl ThinkHost,
    p: Precision,
) -> Result<()> {
    let thousand = Binary::from_f32_bits(0x447a_0000).map_err(num_err)?;
    let budget = Binary::from_f32_bits(budget)
        .and_then(|b| b.mul(&thousand, p))
        .and_then(|b| b.store_f32())
        .map_err(num_err)?;
    if host.session_mode()? == 7 && !g.mode7_enabled {
        return Ok(());
    }
    if host.session_mode()? != 7 && host.session_mode()? != 4 && host.session_mode()? != 0xe {
        return Ok(());
    }
    let Some((world_dc, world_275)) = g.world else {
        return Ok(());
    };
    if world_dc < 1 || world_275 {
        return Ok(());
    }
    host.lock()?;
    let Some((analysed, dynamic)) = g.map else {
        return host.unlock();
    };
    if !analysed || g.session_e4 == 4 {
        return host.unlock();
    }
    if !sai.ready || !dynamic {
        return host.unlock();
    }
    if sai.revision != Some(g.revision) {
        sai.revision = Some(g.revision);
        host.delete_fiber(Fiber::GoalEngine)?;
        sai.goal_fiber = false;
        host.delete_fiber(Fiber::Rai)?;
        sai.rai_fiber = false;
        host.delete_fiber(Fiber::Metagame)?;
        sai.metagame_fiber = false;
        sai.metagame = false;
    }
    if !sai.rai_fiber {
        host.create_fiber(Fiber::Rai)?;
        sai.rai_fiber = true;
    }
    if !sai.goal_fiber {
        host.create_fiber(Fiber::GoalEngine)?;
        sai.goal_fiber = true;
    }
    if !sai.main_fiber {
        sai.main_fiber = host.convert_thread()?;
        if !sai.main_fiber {
            host.log_convert_failure()?;
        }
    }
    if g.paused {
        return host.unlock();
    }
    match g.session_e4 {
        2 => {
            let total = ftol2_low(budget);
            let half = total >> 1;
            let first = total.wrapping_sub(half);
            sai.rai_deadline = host.now()?.wrapping_add(first);
            if !sai.players.is_empty() {
                host.run(Fiber::Rai, sai.rai_deadline)?;
            }
            let now = host.now()?;
            sai.goal_deadline = now.max(sai.rai_deadline).wrapping_add(half);
            host.run(Fiber::GoalEngine, sai.goal_deadline)?;
            host.unlock()?;
            while let Some((due, _)) = sai.commands.front() {
                let due = *due;
                if due > host.now()? {
                    break;
                }
                let (_, data) = sai.commands.pop_front().expect("front exists");
                host.send(data)?;
            }
            Ok(())
        }
        5 => {
            if !sai.metagame_fiber {
                host.create_fiber(Fiber::Metagame)?;
                sai.metagame_fiber = true;
            }
            let now = host.now()?;
            sai.metagame_deadline = now.wrapping_add(ftol2_low(budget));
            host.run(Fiber::Metagame, sai.metagame_deadline)?;
            host.unlock()?;
            host.metagame_post()
        }
        _ => host.unlock(),
    }
}

fn num_err(e: aa_numeric::Error) -> Error {
    Error::World(format!("x87 arithmetic: {e}"))
}

/// Complete `7B8E70` on a region-kingdom record (`+0` region, `+10`
/// visible-tile count, `+14` white-fog time): the count drops by one and,
/// when the visible fraction `count / region area` (x87 `fild`/`fidiv`) is
/// (ordered) below the SVar `ResetWhiteFogTimerPecent`, `+14` takes the game
/// time `[B36534]+E0`. A zero area divides by zero with the exception
/// masked (signed infinity, or NaN for 0/0), as natively.
pub fn white_fog_tick(
    record: &mut aa_navigation::sai_state::RegionKingdom,
    area: i32,
    reset_percent: u32,
    game_time: u32,
    p: Precision,
) -> Result<()> {
    record.visible = record.visible.wrapping_sub(1);
    let count = record.visible as i32;
    let threshold = Binary::from_f32_bits(reset_percent).map_err(num_err)?;
    let below = if area == 0 {
        // +-inf compares by sign; 0/0 is NaN (unordered: no reset).
        count < 0
    } else {
        let fraction = Binary::from_i32(count)
            .div(&Binary::from_i32(area), p)
            .map_err(num_err)?;
        fraction.compare(&threshold) == std::cmp::Ordering::Less
    };
    if below {
        record.word14 = game_time;
    }
    Ok(())
}

/// Complete `7A9A60(tile, kingdom)`, the zone hook when a tile stops being
/// visible to a kingdom: the tile's rectangle (`+10`, may be null) gives its
/// region (`7B9300`: none for a negative region id), the region its record
/// for the kingdom (`7B8FF0`: `+188`... index below the record count), and
/// that record ticks (`7B8E70`).
#[allow(clippy::too_many_arguments)]
pub fn white_fog_tile_hidden(
    state: &mut aa_navigation::sai_state::State,
    tile_region: Option<i32>,
    kingdom: aa_navigation::sai_state::Kingdom,
    areas: &[i32],
    reset_percent: u32,
    game_time: u32,
    p: Precision,
) -> Result<()> {
    let Some(region) = tile_region else {
        return Ok(());
    };
    if region < 0 {
        return Ok(());
    }
    let index = region as usize;
    let regions = state.regions.len();
    let entry = state.regions.get_mut(index).ok_or_else(|| {
        Error::World(format!(
            "7B9300 reads region {index} beyond the {regions} SAI regions"
        ))
    })?;
    let slot = usize::try_from(kingdom.major_index)
        .map_err(|_| Error::World("7B8FF0 with a negative kingdom index".into()))?;
    let Some(Some(record)) = entry.kingdoms.get_mut(slot) else {
        return Ok(());
    };
    let area = *areas
        .get(index)
        .ok_or_else(|| Error::World(format!("region {index} has no +C area")))?;
    white_fog_tick(record, area, reset_percent, game_time, p)
}
