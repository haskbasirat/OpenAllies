//! Computer opponent: plays through the same commands as a human, following
//! the original strategic AI (`Source/Sai`, AA.exe 79D300..7D0A70; exact ports
//! of its pieces live in `aa_mind::sai`). Owner: opponent.
//!
//! Behaviour-faithful, plain `f32`. Structure, as in the original:
//! - `KKC_SAI::Think` (`7A0080`) runs the reactive AI fiber, then the goal
//!   engine fiber; each takes the due head nodes of its queue in turn
//!   (`79E1D0`): `[B36528]+54` (reactive AI) and `+4C` (goal engine, with
//!   the dynamic map analysis `7AC0D0` as id -1). A new AI slot is
//!   prepended to both with time 0 (`79FC40`), so every player thinks at
//!   the start; the goal-engine fiber first analyses the map (`79E900`).
//!   Each think reschedules itself (`79EFF0`): the goal engine after ego
//!   `think_frequency` (30 s in every shipped ego), the reactive AI after
//!   `ReactiveAI/think_frequency` (2 s), the analysis after
//!   `dynamic_map_analysis_think_frequency`; the first reschedule is
//!   staggered by `id / (players + 1)` of the period (the analysis by
//!   `players / (players + 1)`), and the queue keeps its nodes
//!   `GoalEngineThinkFrequencyMinOffset` / `RAIThinkFrequencyMinOffset`
//!   apart. The goal engine also rethinks when one of the player's
//!   constructions finishes while its settlements are at most
//!   `immediate_think_on_build_max_settlements` and the battle is younger
//!   than `immediate_think_on_build_max_time` (`7B2350`/`7AD9A0`, called
//!   from `KKC_ConstructionComponent::OnConstructionFinished`; a truck's
//!   unpacked building is not a construction, so in a skirmish without
//!   engineer constructions the thinks stay on the 30 s schedule, as in the
//!   original's goal CSV).
//! - Decisions leave the AI only as `Command`s (`KKC_SAICommandData::Send`
//!   `7AF220`), spaced by the profile's `command_max_delay` within
//!   `command_max_total_delay` and capped by `max_ge_commands` /
//!   `max_rai_commands`.
//! - The SAI RNG (`85D8A0`/`85D840`) is the AI's own, never the world RNG.
//!
//! Difficulty: every shipped SAI profile is `sai_difficulty_level_HARD`; the
//! original Easy/Normal/Hard choice is the kingdom handicap
//! (`Game/handicaps.tgi`: `property_handicap_easy` RECRUIT_COST and health
//! MultiplyModifier -0.25, `handicap_medium` none, `property_handicap_hard`
//! +0.25, applied through the property system by `SetAIHandicap`
//! `699B70`). It does not change the AI's decisions; [`handicap`] exposes the
//! modifiers for the production and combat owners.
pub mod data;
pub mod goals;
pub mod rai;
pub mod reach;
pub mod regions;
pub mod value;
pub mod view;

pub use data::AiData;

use crate::world::{AbilityTarget, ActorId, Command, Controller, KingdomId, Unit, World};
use glam::Vec2;
use regions::{Influence, Regions, Source, SpreadSettings};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;
use view::View;

/// The SAI RNG object (`[B36528]+2C`): `85D8A0` seeding, `85D840` draws.
/// KKC_SAI's constructor 0x7A0AA0 seeds it once with 0x59ABA0, the
/// milliseconds of a QueryPerformanceCounter timer, so natively it differs
/// from run to run; aa-play takes the seed from [`set_seed`] (0 by default).
#[derive(Clone, Copy, Debug)]
pub struct SaiRng {
    pub state: u32,
}
impl SaiRng {
    /// 0x85D8A0: `((seed << 16) | seed) ^ 0x8E4B12A5`, zero means the
    /// default state.
    pub fn from_seed(seed: u32) -> Self {
        SaiRng { state: aa_simulation::NativeRng::from_native_seed(seed).state }
    }
    pub fn next_u32(&mut self) -> u32 {
        let wide = u64::from(self.state) * 0x41c6_4e6d + 0x3039;
        self.state = wide as u32;
        (wide >> 16) as u32
    }
    /// `45D400`-style unit draw: `r * 1/(2^32-1)` in [0, 1].
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() as f64 / 4_294_967_295.0) as f32
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { self.next_u32() as usize % n }
    }
}

/// One SAI think queue (`[B36528]+4C` goal engine, `+54` reactive AI):
/// `{id, time, next}` nodes, earliest first.
#[derive(Clone, Debug, Default)]
pub struct ThinkQueue {
    pub entries: Vec<(i32, f32)>,
}

impl ThinkQueue {
    /// `79FC40`: a newly selected AI slot is prepended with time 0.
    fn prepend(&mut self, id: i32) {
        self.entries.insert(0, (id, 0.0));
    }
    /// `79E430`: drop the id's node.
    fn remove(&mut self, id: i32) {
        if let Some(i) = self.entries.iter().position(|e| e.0 == id) {
            self.entries.remove(i);
        }
    }
    /// `79EFF0(id, delay, immediate)`: due at now + delay unless already due
    /// no later; the first reschedule is staggered by `id / (count + 1)` of
    /// the delay (the analysis, id -1, by `count / (count + 1)`); inserted
    /// before the first node due later; then every node not due at 0 is
    /// pushed to at least `offset` after its predecessor (the head after
    /// now).
    fn schedule(&mut self, id: i32, delay: f32, immediate: bool, count: usize, now: f32, offset: f32) {
        let mut t = now + delay;
        if let Some(i) = self.entries.iter().position(|e| e.0 == id) {
            if self.entries[i].1 <= t {
                return;
            }
            self.entries.remove(i);
        }
        if immediate {
            let k = if id == -1 { count as f32 } else { id as f32 };
            t -= k / (count as f32 + 1.0) * delay;
        }
        let at = self.entries.iter().position(|e| t < e.1).unwrap_or(self.entries.len());
        self.entries.insert(at, (id, t));
        let mut prev = now;
        for e in self.entries.iter_mut() {
            if e.1 != 0.0 {
                e.1 = e.1.max(prev + offset);
            }
            prev = e.1;
        }
    }
    /// `79E1D0`: pop the head node when it is due.
    fn pop_due(&mut self, now: f32) -> Option<i32> {
        let &(id, t) = self.entries.first()?;
        if t <= now {
            self.entries.remove(0);
            Some(id)
        } else {
            None
        }
    }
    /// The time a node is due (None when not queued).
    pub fn due(&self, id: i32) -> Option<f32> {
        self.entries.iter().find(|e| e.0 == id).map(|e| e.1)
    }
}

/// Per-definition tables built from `GameData` when the AI binds to a world.
#[derive(Clone, Debug, Default)]
pub struct Bound {
    pub cv: Vec<f32>,
    pub props: Vec<Vec<String>>,
}

/// One line of the opponent's decision timeline (tests and debugging).
#[derive(Clone, Debug)]
pub struct LogEntry {
    pub time: f32,
    pub kingdom: KingdomId,
    pub text: String,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Stats {
    pub ticks: u64,
    pub total_ns: u128,
    pub max_ns: u128,
    pub ge_thinks: u64,
    pub rai_thinks: u64,
    pub analyses: u64,
    pub commands: u64,
}

/// One computer kingdom (`KKC_SAIPlayer`).
#[derive(Clone, Debug, Default)]
pub struct Player {
    pub kingdom: KingdomId,
    /// Slot index used by the think staggering.
    pub index: usize,
    pub profile: usize,
    pub ego: usize,
    pub ge_due: f32,
    pub rai_due: f32,
    pub ge_count: u32,
    pub rai_count: u32,
    pub memory: goals::Memory,
    pub rai: rai::Memory,
    /// Commands waiting for their send time.
    pub queue: VecDeque<(f32, Command)>,
    /// Battle time each region was last seen by this player.
    pub scouted: Vec<f32>,
    /// Organizations with a command queued or just sent (the SAI actor
    /// record `+20` pending byte), until the given battle time.
    pub pending: BTreeMap<crate::world::OrgId, f32>,
    /// Internal flares (`[InternalFlares]`): (goal, priority, until).
    pub flares: Vec<(goals::Key, f32, f32)>,
    /// Health of the player's structures at the last goal-engine think.
    pub structure_health: BTreeMap<ActorId, f32>,
    /// The other sides' structures the player knew at the last goal-engine
    /// think (a capture shows as one of them turning the player's).
    pub foreign: BTreeSet<ActorId>,
}

impl Player {
    pub fn pending(&self, o: crate::world::OrgId, now: f32) -> bool {
        self.pending.get(&o).is_some_and(|&t| now < t)
    }
}

/// Opponent state (`World::ai`).
#[derive(Default)]
pub struct AiState {
    pub data: Option<Arc<AiData>>,
    bound_for: usize,
    pub bound: Bound,
    pub regions: Option<Regions>,
    pub influence: Influence,
    pub players: Vec<Player>,
    /// SAI profile ids chosen by the setup screen, by kingdom.
    pub profiles: BTreeMap<KingdomId, String>,
    /// The SAI's one stream (`[B36528]+2C`).
    pub(crate) rng: Option<SaiRng>,
    /// `Command::AirAttack` / `AirRecon` reach an aircraft. Until
    /// `crate::combat::tasks` stops refusing them the air goals take no
    /// airfield and reserve no money (tests of the goals turn it on).
    pub air_orders: bool,
    /// The player's structures under construction, watched for
    /// `OnConstructionFinished` (`565B00` -> `7B2350`).
    constructing: Vec<ActorId>,
    /// `[B36528]+4C`: the goal-engine queue (player ids, and the dynamic map
    /// analysis as id -1); `+54`: the reactive AI queue.
    pub ge_queue: ThinkQueue,
    pub rai_queue: ThinkQueue,
    /// The goal-engine fiber started (`79E900` ran its first analysis).
    ge_started: bool,
    /// `[B28A88]`: the next analysis reschedule is staggered (cleared by
    /// `7AC0D0` after its first use).
    analysis_stagger_used: bool,
    actors_seen: usize,
    pub stats: Stats,
    pub log: Vec<LogEntry>,
    /// Keep the decision timeline (off by default; tests turn it on).
    pub logging: bool,
    /// Land reachability of the unpack site search (rebuilt when the
    /// structures change).
    reach: reach::Reach,
    // ---- appended by the core owner ----
    /// The 0x59ABA0 value KKC_SAI seeds its RNG with ([`set_seed`]).
    pub(crate) seed: u32,
}

impl std::fmt::Debug for AiState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AiState").field("players", &self.players.len()).field("stats", &self.stats).finish()
    }
}

/// Install the original SAI data (from [`AiData::load`]).
pub fn attach(world: &mut World, data: Arc<AiData>) {
    world.ai.data = Some(data);
}

/// The value the SAI RNG is seeded with (0x7A0AA0: 0x85D8A0 of 0x59ABA0,
/// the elapsed milliseconds of a performance-counter timer). Pass a clock
/// reading to vary the AI between runs as the original does; tests keep the
/// default 0. Takes effect if the SAI has not drawn yet.
pub fn set_seed(world: &mut World, seed: u32) {
    world.ai.seed = seed;
    world.ai.rng = None;
}

/// Choose the SAI profile of a kingdom by IDS (e.g. `zhukov_sai`), as the
/// custom-battle setup does when a general is picked.
///
/// A running player keeps its goals, records and think times (79F5D0 -> 744EA0 writes
/// only the player's +18); 7AD180 chooses the ego from the new profile at its next think.
pub fn set_profile(world: &mut World, kingdom: KingdomId, profile: &str) {
    let ids = profile.to_ascii_lowercase();
    let st = &mut world.ai;
    let index = st.data.as_ref().and_then(|d| d.profile(&ids));
    st.profiles.insert(kingdom, ids);
    if let Some(index) = index {
        for p in st.players.iter_mut().filter(|p| p.kingdom == kingdom) {
            p.profile = index;
        }
    }
}

/// The handicap multiply modifiers for an original difficulty index
/// (`Game/difficulty_level.tgi` order: 0 easy, 1 medium, 2 hard), e.g.
/// `[("RECRUIT_COST", -0.25), ("health", -0.25)]` for easy.
pub fn handicap(data: &AiData, level: u8) -> Vec<(String, f32)> {
    data.handicaps.get(level as usize).map(|h| h.modifiers.clone()).unwrap_or_default()
}

fn svar(world: &World, name: &str, shipped: f32) -> f32 {
    world.data.rule_ci(name).filter(|v| v.is_finite()).unwrap_or(shipped)
}

/// The SVars of the static map analysis `7ACB00` and `7AA180`.
fn static_svars(world: &World) -> regions::StaticSvars {
    let d = regions::StaticSvars::default();
    let int = |name: &str, v: i32| svar(world, name, v as f32) as i32;
    regions::StaticSvars {
        starting_square_size: int("StaticMapStartingSquareSize", d.starting_square_size),
        use_fast_square_reduction: svar(world, "StaticMapUseFastSquareReduction", f32::from(u8::from(d.use_fast_square_reduction))) != 0.0,
        fast_square_reduction_mult: int("StaticMapFastSquareReductionMult", d.fast_square_reduction_mult),
        fast_square_reduction_div: int("StaticMapFastSquareReductionDiv", d.fast_square_reduction_div),
        min_region_size: int("StaticMapMinRegionSize", d.min_region_size),
        min_region_width: int("StaticMapMinRegionWidth", d.min_region_width),
        max_region_size_for_expansion: int("StaticMapMaxRegionSizeForExpansion", d.max_region_size_for_expansion),
        max_rectangle_width: int("StaticMapMaxRectangleWidth", d.max_rectangle_width),
        base_map_width: int("StaticMapBaseMapWidth", d.base_map_width),
        max_rectangle_width_exp: svar(world, "StaticMapMaxRectangleWidthExp", d.max_rectangle_width_exp),
    }
}

pub fn think(world: &mut World) -> Vec<Command> {
    if !world.kingdoms.iter().any(|k| matches!(k.controller, Controller::Computer(_)) && !k.defeated) {
        return Vec::new();
    }
    let start = std::time::Instant::now();
    let mut st = std::mem::take(&mut world.ai);
    let out = st.run(world);
    let ns = start.elapsed().as_nanos();
    st.stats.ticks += 1;
    st.stats.total_ns += ns;
    st.stats.max_ns = st.stats.max_ns.max(ns);
    st.stats.commands += out.len() as u64;
    world.ai = st;
    out
}

impl AiState {
    fn run(&mut self, world: &World) -> Vec<Command> {
        // Without the original SAI data (`attach`) the computer stays idle.
        let Some(data) = self.data.clone() else { return Vec::new() };
        // 7A0080 runs the SAI only once the world has ticked ([B36534]+DC
        // >= 1) on the analysed map; by then the battle start has explored
        // the map for the FogOfWar rule (0x4FDCE0 -> 0x551F90), so the
        // first think already knows the neutral cities. Here the think runs
        // inside the update, before the fog: the second update.
        if world.tick < 2 {
            return Vec::new();
        }
        let key = Arc::as_ptr(&world.data) as usize;
        if self.bound_for != key || self.bound.cv.len() != world.data.defs.len() {
            let (cv, props) = view::tables(&world.data);
            self.bound = Bound { cv, props };
            self.bound_for = key;
            self.regions = Some(Regions::build(&world.map, static_svars(world)));
        }
        let seed = self.seed;
        let rng = self.rng.get_or_insert_with(|| SaiRng::from_seed(seed));
        let mut rng = *rng;
        self.ensure_players(world, &data, &mut rng);
        let regions = self.regions.take().unwrap_or_default();
        self.detect_structures(world, &data, &regions);
        self.detect_refused_deploys(world);
        let now = world.time;
        let ge_gap = svar(world, "GoalEngineThinkFrequencyMinOffset", 2.0);
        let rai_gap = svar(world, "RAIThinkFrequencyMinOffset", 0.2);
        let count = self.players.len();
        // 7A0080: the RAI fiber (79E280), then the goal-engine fiber
        // (79E900); each runs its queue's due head nodes in turn (79E1D0).
        // The native fibers stop at wall-clock slice deadlines; here every
        // due node runs in the tick it falls due.
        while let Some(id) = self.rai_queue.pop_due(now) {
            let Some(i) = self.players.iter().position(|p| p.index as i32 == id) else { continue };
            // 7B8AC0: think count, the think while the kingdom stands, then
            // 79EFF0(id, ReactiveAI think_frequency, first think, 1).
            let first = self.players[i].rai_count == 0;
            self.players[i].rai_count += 1;
            if let Some(me) = view::kingdom_index(world, self.players[i].kingdom).filter(|&me| !world.kingdoms[me].defeated && !crate::triggers::sai_disabled(world, world.kingdoms[me].id)) {
                self.stats.rai_thinks += 1;
                let view = sai_view(world, &self.bound, me);
                let mut log = Vec::new();
                rai::think(&view, &self.bound, &data, &regions, &self.influence, &mut self.players[i], &mut rng, &mut log);
                let kingdom = self.players[i].kingdom;
                self.push_log(now, kingdom, log);
            }
            let freq = data.egos[self.players[i].ego].reactive.think_frequency.max(0.1);
            self.rai_queue.schedule(id, freq, first, count, now, rai_gap);
        }
        if !self.ge_started {
            // 79E900: the fiber's first act drops the analysis node (79E430)
            // and analyses the map (7AC0D0), which reschedules itself.
            self.ge_started = true;
            self.ge_queue.remove(-1);
            self.analyse_now(world, &data, &regions, count, ge_gap);
        }
        while let Some(id) = self.ge_queue.pop_due(now) {
            if id == -1 {
                self.analyse_now(world, &data, &regions, count, ge_gap);
                continue;
            }
            let Some(i) = self.players.iter().position(|p| p.index as i32 == id) else { continue };
            // 7B4AD0: think count, the think while the kingdom stands, then
            // 79EFF0(id, ego think_frequency, first think, 0).
            let first = self.players[i].ge_count == 0;
            if let Some(me) = view::kingdom_index(world, self.players[i].kingdom).filter(|&me| !world.kingdoms[me].defeated && !crate::triggers::sai_disabled(world, world.kingdoms[me].id)) {
                self.stats.ge_thinks += 1;
                let view = sai_view(world, &self.bound, me);
                let mut log = Vec::new();
                self.reach.update(world);
                goals::think(&view, &self.bound, &data, &regions, &self.influence, &self.reach, &mut self.players[i], &mut rng, &mut log, self.logging, self.air_orders);
                let kingdom = self.players[i].kingdom;
                self.push_log(now, kingdom, log);
            }
            self.players[i].ge_count += 1;
            let freq = data.egos[self.players[i].ego].think_frequency.max(0.5);
            self.ge_queue.schedule(id, freq, first, count, now, ge_gap);
        }
        for p in &mut self.players {
            p.ge_due = self.ge_queue.due(p.index as i32).unwrap_or(now);
            p.rai_due = self.rai_queue.due(p.index as i32).unwrap_or(now);
        }
        self.regions = Some(regions);
        self.rng = Some(rng);
        // KKC_SAI::Think sends every due command.
        let mut out = Vec::new();
        for p in &mut self.players {
            while p.queue.front().is_some_and(|(due, _)| *due <= now) {
                let (_, command) = p.queue.pop_front().expect("front exists");
                if let Some(c) = still_valid(world, command) {
                    if let Command::Deploy { truck, .. } = c
                        && let Some(memo) = p.memory.unpack.get_mut(&truck)
                    {
                        memo.sent_at = now;
                    }
                    if self.logging && self.log.len() < 20_000 {
                        self.log.push(LogEntry { time: now, kingdom: p.kingdom, text: format!("send {c:?}") });
                    }
                    // move at a hostile thing (4DDE60): as the film's mapping has it,
                    // the move's Kill state (0x527E30, 0x51D920).
                    match c {
                        Command::Follow { kingdom, units, target } if crate::orders::hostile_to(world, kingdom, target) => {
                            out.push(Command::Order { kingdom, units: units.clone(), order: crate::orders::ActorOrder::MoveOnto });
                            out.push(Command::Attack { kingdom, units, target });
                        }
                        c => out.push(c),
                    }
                }
            }
        }
        out
    }

    fn push_log(&mut self, time: f32, kingdom: KingdomId, lines: Vec<String>) {
        if !self.logging {
            return;
        }
        for text in lines {
            if self.log.len() >= 20_000 {
                self.log.remove(0);
            }
            self.log.push(LogEntry { time, kingdom, text });
        }
    }

    fn ensure_players(&mut self, world: &World, data: &AiData, rng: &mut SaiRng) {
        let computers: Vec<KingdomId> = world.kingdoms.iter().filter(|k| matches!(k.controller, Controller::Computer(_))).map(|k| k.id).collect();
        let (ge_queue, rai_queue) = (&mut self.ge_queue, &mut self.rai_queue);
        self.players.retain(|p| {
            let keep = computers.contains(&p.kingdom);
            if !keep {
                ge_queue.remove(p.index as i32);
                rai_queue.remove(p.index as i32);
            }
            keep
        });
        for (index, &id) in computers.iter().enumerate() {
            if self.players.iter().any(|p| p.kingdom == id) {
                continue;
            }
            let k = &world.kingdoms[view::kingdom_index(world, id).expect("listed")];
            let Some(profile) = choose_profile(data, self.profiles.get(&id).map(String::as_str), &k.nation, rng) else { continue };
            let ego = data.profiles[profile].egos.first().copied().unwrap_or(0);
            let regions = self.regions.as_ref().map_or(0, |r| r.len());
            // What the player knows of the other sides comes from the fog
            // (`view`); the seen records start empty.
            self.players.push(Player {
                kingdom: id,
                index,
                profile,
                ego,
                ge_due: world.time,
                rai_due: world.time,
                scouted: vec![f32::NEG_INFINITY; regions],
                ..Default::default()
            });
            // 79FC40: the new AI slot is prepended to both think queues with
            // time 0 (due at once, never spaced).
            self.ge_queue.prepend(index as i32);
            self.rai_queue.prepend(index as i32);
        }
    }

    /// A Deploy refused by the deploy owner (spot blocked or not reachable:
    /// `Event::Refused`, the truck's order reset to Idle) leaves the truck
    /// without a deployment once the command was sent. The AI drops that
    /// region; the truck is planned again by the next regular think (the
    /// original has no rethink on a refused order).
    fn detect_refused_deploys(&mut self, world: &World) {
        let now = world.time;
        let logging = self.logging;
        let mut lines = Vec::new();
        for p in &mut self.players {
            for (&truck, memo) in p.memory.unpack.iter_mut() {
                if memo.sent_at <= 0.0 || memo.ordered_at <= 0.0 || now <= memo.sent_at {
                    continue;
                }
                let Some(a) = world.actor(truck).filter(|a| a.alive) else { continue };
                if view::truck_deploying(world, a.id) {
                    continue;
                }
                memo.bad.push(memo.region);
                memo.ordered_at = 0.0;
                memo.sent_at = 0.0;
                if logging {
                    lines.push((p.kingdom, format!("deploy of {} refused at ({:.0},{:.0}); another site", world.data.def(a.def).id, memo.site.x, memo.site.y)));
                }
            }
        }
        for (k, text) in lines {
            self.push_log(now, k, vec![text]);
        }
    }

    /// The immediate rethink and the new-structure defend flare of `7B2350`.
    /// The original calls it from `KKC_ConstructionComponent::
    /// OnConstructionFinished` (`565B00`) only: a truck's unpacked building
    /// is not a construction (the goal CSV shows no extra think and no +500
    /// defend flare when the supply depot, the Infantry Division HQ or the
    /// Artillery Brigade unpack). Here: a structure placed under construction
    /// is watched until its construction ends.
    fn detect_structures(&mut self, world: &World, data: &AiData, regions: &Regions) {
        let from = self.actors_seen.min(world.actors.len());
        self.actors_seen = world.actors.len();
        for a in &world.actors[from..] {
            if world.books.actor(a.id).is_some_and(|b| b.constructing) {
                self.constructing.push(a.id);
            }
        }
        if self.constructing.is_empty() {
            return;
        }
        let mut finished = Vec::new();
        self.constructing.retain(|&a| {
            let actor = &world.actors[a as usize];
            if !actor.alive || actor.removed {
                return false;
            }
            if world.books.actor(a).is_some_and(|b| b.constructing) {
                return true;
            }
            finished.push(a);
            false
        });
        let count = self.players.len();
        let gap = svar(world, "GoalEngineThinkFrequencyMinOffset", 2.0);
        for a in finished {
            let actor = &world.actors[a as usize];
            let Some(k) = actor.kingdom else { continue };
            for p in self.players.iter_mut().filter(|p| p.kingdom == k) {
                let ego = &data.egos[p.ego];
                // 7B2350 / 7AD9A0: while the player's settlements are at
                // most immediate_think_on_build_max_settlements and the
                // battle is not older than immediate_think_on_build_max_time,
                // 79EFF0(id, 0, 0, 0): due now, then spaced by
                // GoalEngineThinkFrequencyMinOffset.
                let settlements = world.actors.iter().filter(|x| x.alive && !x.removed && x.kingdom == Some(k) && world.data.def(x.def).city).count() as i32;
                if settlements <= ego.immediate_think_on_build_max_settlements && world.time <= ego.immediate_think_on_build_max_time {
                    self.ge_queue.schedule(p.index as i32, 0.0, false, count, world.time, gap);
                }
                // 7B2350: a new structure is defended for a while. A thing
                // without a parent (virtual +100) takes the structure values;
                // the settlement ones are for a building that is the centre
                // of its parent settlement (+158), which aa-play does not
                // keep: the goal CSV shows +500 on a city, not +750.
                let (priority, duration) = (ego.flares.defend_new_structure_priority, ego.flares.defend_new_structure_duration);
                p.flares.push((goals::Key::Defend(regions.region_at(actor.pos.truncate())), priority, world.time + duration));
            }
        }
    }

    /// `7AC0D0` and its reschedule `79EFF0(-1, dynamic_map_analysis_
    /// think_frequency, [B28A88], 0)`; `[B28A88]` is cleared after the
    /// first use, so only the first reschedule is staggered.
    fn analyse_now(&mut self, world: &World, data: &AiData, regions: &Regions, count: usize, gap: f32) {
        self.analyse(world, data, regions);
        let stagger = !self.analysis_stagger_used;
        self.analysis_stagger_used = true;
        let freq = svar(world, "dynamic_map_analysis_think_frequency", 10.0).max(0.5);
        self.ge_queue.schedule(-1, freq, stagger, count, world.time, gap);
    }

    /// `7AC0D0`: spread every live actor's combat value.
    fn analyse(&mut self, world: &World, data: &AiData, regions: &Regions) {
        self.stats.analyses += 1;
        let cap = svar(world, "DynamicMapAnalysisSpeedCap", 10.0);
        let min_factor = svar(world, "min_cv_propogation_factor", 0.2);
        let fudge = svar(world, "settlement_cv_range_fudge_factor", 1.7);
        // Every player's ego shares the map in the original; the first
        // player's ego supplies the spread settings here.
        let ego = self.players.first().map(|p| &data.egos[p.ego]);
        let (travel, exponent, border) = ego.map_or((45.0, 1.5, 300.0), |e| (e.cv_influence_max_travel_time, e.cv_influence_exponent, e.border_influence_range));
        // 4D0180 of standing actors: Performer +60, as the think's view reads them
        // (`view::standing_values`); the definitions' table (bound.cv) is +1E4.
        let standing = view::standing_values(world);
        let standing = |d: crate::data::DefId| standing.get(d as usize).copied().unwrap_or(0.0);
        let mut sources = Vec::new();
        for org in world.orgs.iter().filter(|o| o.alive) {
            let Some(owner) = view::kingdom_index(world, org.kingdom) else { continue };
            let (mut cv, mut sum, mut n, mut speed) = (0.0, Vec2::ZERO, 0, f32::MAX);
            for &m in &org.members {
                let Some(a) = world.actors.get(m as usize).filter(|a| a.alive && !a.removed) else { continue };
                let d = world.data.def(a.def);
                cv += standing(a.def) * (a.health / d.health.max(1e-3)).clamp(0.0, 1.0);
                sum += a.pos.truncate();
                n += 1;
                if d.speed_max > 0.0 {
                    speed = speed.min(d.speed_max);
                }
            }
            if n == 0 || cv <= 0.0 {
                continue;
            }
            let speed = if speed == f32::MAX { 0.0 } else { speed.min(cap) };
            sources.push(Source { owner, pos: sum / n as f32, cv, range: speed * travel, mobile: true });
        }
        for a in world.actors.iter().filter(|a| a.alive && !a.removed && a.org.is_none()) {
            let d = world.data.def(a.def);
            if !matches!(d.class, Some(crate::data::DefClass::Building)) {
                continue;
            }
            let Some(owner) = a.kingdom.and_then(|k| view::kingdom_index(world, k)) else { continue };
            let cv = standing(a.def) * (a.health / d.health.max(1e-3)).clamp(0.0, 1.0);
            let range = d.weapons.iter().map(|w| w.range).fold(d.supply_range, f32::max);
            sources.push(Source { owner, pos: a.pos.truncate(), cv, range, mobile: false });
        }
        self.influence = Influence::compute(regions, world.kingdoms.len(), &sources, SpreadSettings { min_factor, exponent, border_range: border, settlement_fudge: fudge }, world.time);
    }
}

/// `7AD180`/`4A8910` profile choice: the setup's profile, else a random
/// public profile of the kingdom's nation with difficulty HARD ("only hard
/// will be selected on random").
fn choose_profile(data: &AiData, requested: Option<&str>, nation: &str, rng: &mut SaiRng) -> Option<usize> {
    if let Some(ids) = requested
        && let Some(i) = data.profile(ids)
    {
        return Some(i);
    }
    let nation = nation.to_ascii_lowercase();
    let full = if nation.starts_with("nation_") { nation.clone() } else { format!("nation_{nation}") };
    let candidates: Vec<usize> = (0..data.profiles.len())
        .filter(|&i| {
            let p = &data.profiles[i];
            p.public && p.difficulty_level.contains("hard") && (p.nations.is_empty() || p.nations.contains(&full))
        })
        .collect();
    if candidates.is_empty() {
        return (!data.profiles.is_empty()).then_some(0);
    }
    Some(candidates[rng.below(candidates.len())])
}

/// Kingdom `k` regards the owner of an actor as an enemy (relation 3).
pub(crate) fn enemy_of(world: &World, k: KingdomId, owner: Option<KingdomId>) -> bool {
    owner.is_some() && world.books.relation(&world.kingdoms, Some(k), owner) == crate::books::ENEMY
}

/// 572E50's SAI side: a team command flare gives the kingdom's computer team
/// mates the goal (attack or defend the point's region) at the priority for the
/// duration, an internal flare (`[InternalFlares]`, the think reads them).
pub fn team_flare(world: &mut World, kingdom: KingdomId, at: Vec2, attack: bool, priority: f32, duration: f32) {
    let now = world.time;
    let Some(team) = world.kingdoms.get(kingdom as usize).map(|k| k.team) else { return };
    let Some(regions) = world.ai.regions.take() else { return };
    let region = regions.region_at(at);
    let mates: Vec<KingdomId> = world.kingdoms.iter().filter(|k| k.team == team && k.id != kingdom && !k.independent).map(|k| k.id).collect();
    for p in world.ai.players.iter_mut().filter(|p| mates.contains(&p.kingdom)) {
        let key = if attack { goals::Key::Attack(region) } else { goals::Key::Defend(region) };
        p.flares.push((key, priority, now + duration));
    }
    world.ai.regions = Some(regions);
}

/// The think's view: what the SAI may order leaves out its own organizations
/// and trucks a trigger took from it (Object+8 bit 0x20: 7A25F0 refuses to
/// assign such an actor, 7B4FE0 / 7B5210 / 7B6800 skip its record). Other
/// kingdoms' keep their place: a disabled enemy is still seen and valued.
fn sai_view<'w>(world: &'w World, bound: &Bound, me: usize) -> View<'w> {
    let mut v = View::build(world, bound, me);
    v.orgs.retain(|o| o.owner != me || !crate::triggers::unit_sai_disabled(world, Unit::Org(o.id)));
    v.trucks.retain(|t| t.owner != me || !crate::triggers::unit_sai_disabled(world, Unit::Actor(t.id)));
    v
}

/// `7AF220` skips actors that died or are being deleted since the think, and
/// those a trigger took from the SAI since then (Object+8 bits 0 and 5).
fn still_valid(world: &World, command: Command) -> Option<Command> {
    let live = |u: &Unit| {
        !crate::triggers::unit_sai_disabled(world, *u)
            && match *u {
                Unit::Org(o) => world.orgs.get(o as usize).is_some_and(|o| o.alive),
                Unit::Actor(a) => world.actor(a).is_some_and(|a| a.alive),
            }
    };
    match command {
        Command::Move { kingdom, units, to, attack_move } => {
            let units: Vec<Unit> = units.into_iter().filter(live).collect();
            (!units.is_empty()).then_some(Command::Move { kingdom, units, to, attack_move })
        }
        Command::Attack { kingdom, units, target } => {
            // A target that changed hands (capture) since the think is no
            // longer attacked: only an enemy's actor is.
            let units: Vec<Unit> = units.into_iter().filter(live).collect();
            let enemy = world.actor(target).is_some_and(|a| a.alive && enemy_of(world, kingdom, a.kingdom));
            (!units.is_empty() && enemy).then_some(Command::Attack { kingdom, units, target })
        }
        Command::Stop { kingdom, units } => {
            let units: Vec<Unit> = units.into_iter().filter(live).collect();
            (!units.is_empty()).then_some(Command::Stop { kingdom, units })
        }
        Command::Recruit { building, .. } | Command::RecruitWith { building, .. } | Command::Pack { building, .. } | Command::CancelRecruit { building, .. } => {
            world.actor(building).is_some_and(|a| a.alive).then_some(command)
        }
        Command::Deploy { truck, .. } => (world.actor(truck).is_some_and(|a| a.alive) && !crate::triggers::unit_sai_disabled(world, Unit::Actor(truck))).then_some(command),
        Command::Research { building, .. } => world.actor(building).is_some_and(|a| a.alive).then_some(command),
        Command::Attach { org, hq, .. } => {
            (world.orgs.get(org as usize).is_some_and(|o| o.alive) && world.actor(hq).is_some_and(|a| a.alive)).then_some(command)
        }
        // HUD player orders (appended by the UI owner; the AI sends none)
        Command::SetFormation { kingdom, orgs, formation } => {
            let orgs: Vec<_> = orgs.into_iter().filter(|o| live(&Unit::Org(*o))).collect();
            (!orgs.is_empty()).then_some(Command::SetFormation { kingdom, orgs, formation })
        }
        Command::SetRallyPoint { .. } | Command::Sell { .. } | Command::Detach { .. } | Command::Disband { .. } => Some(command),
        // special operations (appended by the combat owner; the SAI general
        // powers apply them directly from `special_ops::sai`)
        Command::UseAbility { .. } => Some(command),
        // a right-click on an actor (appended by the movement owner; the AI
        // sends none)
        Command::Follow { kingdom, units, target } => {
            let units: Vec<Unit> = units.into_iter().filter(live).collect();
            (!units.is_empty() && world.actor(target).is_some_and(|a| !a.removed)).then_some(Command::Follow { kingdom, units, target })
        }
        // the BOMBARD goal's order (7C84F0): its live actors, and a target
        // actor that still stands and is still an enemy's
        Command::Bombard { kingdom, units, target } => {
            let units: Vec<Unit> = units.into_iter().filter(live).collect();
            let ok = match target {
                AbilityTarget::Actor(t) => world.actor(t).is_some_and(|a| a.alive && enemy_of(world, kingdom, a.kingdom)),
                AbilityTarget::Point(_) => true,
            };
            (!units.is_empty() && ok).then_some(Command::Bombard { kingdom, units, target })
        }
        // the AIR_ATTACK goal's order (7C7C30): the airfield still stands,
        // and so does a target actor, still an enemy's
        Command::AirAttack { kingdom, building, target } => {
            let ok = match target {
                AbilityTarget::Actor(t) => world.actor(t).is_some_and(|a| a.alive && enemy_of(world, kingdom, a.kingdom)),
                AbilityTarget::Point(_) => true,
            };
            (ok && world.actor(building).is_some_and(|a| a.alive)).then_some(command)
        }
        // an air-recon EXPLORE goal's order: the airfield still stands
        Command::AirRecon { building, .. } => world.actor(building).is_some_and(|a| a.alive).then_some(command),
        // the HUD orders (appended by the core owner; the AI sends none)
        Command::Repair { .. } | Command::Construct { .. } => Some(command),
        // the actor orders of `crate::orders`: the live actors of the list, and a
        // guarded thing that still stands (7AF220, command flag 1).
        Command::Order { kingdom, units, order } => {
            let units: Vec<Unit> = units.into_iter().filter(live).collect();
            let stale = match &order {
                crate::orders::ActorOrder::Guard { target: Some(t), .. } => crate::ids::target_actor(world, *t).is_none_or(|a| !world.actors[a as usize].alive),
                _ => false,
            };
            (!units.is_empty() && !stale).then_some(Command::Order { kingdom, units, order })
        }
        // a person's selection and hot command (the SAI sends none)
        Command::Select { .. } | Command::Hot { .. } => Some(command),
    }
}

/// Queue a think's commands with the profile's send pacing.
pub(crate) fn enqueue(player: &mut Player, data: &AiData, now: f32, reaction: f32, max: i32, commands: Vec<Command>) {
    let profile = &data.profiles[player.profile];
    let n = commands.len().min(max.max(0) as usize);
    if n == 0 {
        return;
    }
    let spacing = profile.command_max_delay.min(profile.command_max_total_delay / n as f32).max(0.0);
    player.pending.retain(|_, t| *t > now);
    for (i, c) in commands.into_iter().take(n).enumerate() {
        let due = now + reaction + spacing * i as f32;
        // 7B4C50: the record's +20 until 7AF220 has sent the order.
        if let Command::Move { units, .. } | Command::Attack { units, .. } | Command::Stop { units, .. } | Command::Bombard { units, .. } | Command::Follow { units, .. } | Command::Order { units, .. } = &c {
            for u in units {
                if let Unit::Org(o) = u {
                    player.pending.insert(*o, due);
                }
            }
        }
        player.queue.push_back((due, c));
    }
}
