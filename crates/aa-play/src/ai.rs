//! Computer opponent: plays through the same commands as a human, following
//! the original strategic AI (`Source/Sai`, AA.exe 79D300..7D0A70; exact ports
//! of its pieces live in `aa_mind::sai`). Owner: opponent.
//!
//! Behaviour-faithful, plain `f32`. Structure, as in the original:
//! - `KKC_SAI::Think` (`7A0080`) runs every AI player's goal engine and
//!   reactive AI on their own schedules (`79EFF0`): the goal engine every ego
//!   `think_frequency` seconds (30 in every shipped ego), the reactive AI
//!   every `ReactiveAI/think_frequency` (2); the first reschedule staggers the
//!   players by `index / (players + 1) * frequency`, and two thinks of any
//!   players are at least `GoalEngineThinkFrequencyMinOffset` /
//!   `RAIThinkFrequencyMinOffset` seconds apart. The goal engine also
//!   rethinks at once when one of the player's constructions finishes while
//!   its settlements are at most `immediate_think_on_build_max_settlements`
//!   and the battle is younger than `immediate_think_on_build_max_time`
//!   (`7B2350`/`7AD9A0`, called from
//!   `KKC_ConstructionComponent::OnConstructionFinished`; a truck's unpacked
//!   building is not a construction, so in a skirmish without engineer
//!   constructions the thinks stay on the 30 s schedule, as in the
//!   original's goal CSV).
//! - The dynamic map analysis (`7AC0D0`) every
//!   `dynamic_map_analysis_think_frequency` seconds, shared by the players.
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

use crate::world::{ActorId, Command, Controller, KingdomId, Unit, World};
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
    /// Hostile static actors this player has seen (or were explored at the start).
    pub known: BTreeSet<ActorId>,
    /// Battle time each region was last seen by this player.
    pub scouted: Vec<f32>,
    /// Structures owned at the last check (immediate rethink).
    pub structures: BTreeSet<ActorId>,
    /// Organizations with a command queued or just sent (the SAI actor
    /// record `+20` pending byte), until the given battle time.
    pub pending: BTreeMap<crate::world::OrgId, f32>,
    /// Internal flares (`[InternalFlares]`): (goal, priority, until).
    pub flares: Vec<(goals::Key, f32, f32)>,
    /// Health of the player's structures at the last goal-engine think.
    pub structure_health: BTreeMap<ActorId, f32>,
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
    next_analysis: f32,
    pub players: Vec<Player>,
    /// SAI profile ids chosen by the setup screen, by kingdom.
    pub profiles: BTreeMap<KingdomId, String>,
    rng: Option<SaiRng>,
    last_ge: f32,
    last_rai: f32,
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
    seed: u32,
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
pub fn set_profile(world: &mut World, kingdom: KingdomId, profile: &str) {
    world.ai.profiles.insert(kingdom, profile.to_ascii_lowercase());
    world.ai.players.retain(|p| p.kingdom != kingdom);
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
        let key = Arc::as_ptr(&world.data) as usize;
        if self.bound_for != key || self.bound.cv.len() != world.data.defs.len() {
            let (cv, props) = view::tables(&world.data);
            self.bound = Bound { cv, props };
            self.bound_for = key;
            let tiles = svar(world, "StaticMapStartingSquareSize", 10.0).max(1.0) as u32;
            self.regions = Some(Regions::build(&world.map, tiles));
        }
        let seed = self.seed;
        let rng = self.rng.get_or_insert_with(|| SaiRng::from_seed(seed));
        let mut rng = *rng;
        self.ensure_players(world, &data, &mut rng);
        let regions = self.regions.take().unwrap_or_default();
        if world.time >= self.next_analysis {
            self.analyse(world, &data, &regions);
            self.next_analysis = world.time + svar(world, "dynamic_map_analysis_think_frequency", 10.0).max(0.5);
        }
        self.detect_structures(world, &data);
        self.detect_refused_deploys(world, &data);
        let now = world.time;
        let ge_gap = svar(world, "GoalEngineThinkFrequencyMinOffset", 2.0);
        let rai_gap = svar(world, "RAIThinkFrequencyMinOffset", 0.2);
        let count = self.players.len();
        let mut ge_done = false;
        let mut rai_done = false;
        for i in 0..self.players.len() {
            let Some(me) = view::kingdom_index(world, self.players[i].kingdom) else { continue };
            if world.kingdoms[me].defeated {
                continue;
            }
            let ego = &data.egos[self.players[i].ego];
            let ge_due = !ge_done && now >= self.players[i].ge_due && (self.stats.ge_thinks == 0 || now >= self.last_ge + ge_gap);
            let rai_due = !rai_done && now >= self.players[i].rai_due && (self.stats.rai_thinks == 0 || now >= self.last_rai + rai_gap);
            if !ge_due && !rai_due {
                continue;
            }
            let view = View::build(world, &self.bound, me);
            if ge_due {
                ge_done = true;
                self.last_ge = now;
                self.stats.ge_thinks += 1;
                let mut log = Vec::new();
                let first = self.players[i].ge_count == 0;
                self.reach.update(world);
                goals::think(&view, &self.bound, &data, &regions, &self.influence, &self.reach, &mut self.players[i], &mut rng, &mut log, self.logging);
                let p = &mut self.players[i];
                let ego = &data.egos[p.ego];
                let freq = ego.think_frequency.max(0.5);
                p.ge_due = now + freq - if first { p.index as f32 / (count as f32 + 1.0) * freq } else { 0.0 };
                p.ge_count += 1;
                let kingdom = p.kingdom;
                self.push_log(now, kingdom, log);
            }
            if rai_due {
                rai_done = true;
                self.last_rai = now;
                self.stats.rai_thinks += 1;
                let mut log = Vec::new();
                let first = self.players[i].rai_count == 0;
                rai::think(&view, &self.bound, &data, &regions, &self.influence, &mut self.players[i], &mut rng, &mut log);
                let p = &mut self.players[i];
                let freq = ego.reactive.think_frequency.max(0.1);
                p.rai_due = now + freq - if first { p.index as f32 / (count as f32 + 1.0) * freq } else { 0.0 };
                p.rai_count += 1;
                let kingdom = p.kingdom;
                self.push_log(now, kingdom, log);
            }
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
                    out.push(c);
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
        self.players.retain(|p| computers.contains(&p.kingdom));
        for (index, &id) in computers.iter().enumerate() {
            if self.players.iter().any(|p| p.kingdom == id) {
                continue;
            }
            let k = &world.kingdoms[view::kingdom_index(world, id).expect("listed")];
            let Some(profile) = choose_profile(data, self.profiles.get(&id).map(String::as_str), &k.nation, rng) else { continue };
            let ego = data.profiles[profile].egos.first().copied().unwrap_or(0);
            let mut known = BTreeSet::new();
            // FogOfWar EXPLORED (the world_rules_aa default): the static
            // actors placed at the start are explored for every kingdom.
            if world.data.rule_ci("FogOfWar").unwrap_or(1.0) >= 1.0 {
                for a in world.actors.iter().filter(|a| a.alive && !a.removed && a.kingdom != Some(id)) {
                    if matches!(world.data.def(a.def).class, Some(crate::data::DefClass::Building)) || world.data.def(a.def).city {
                        known.insert(a.id);
                    }
                }
            }
            let regions = self.regions.as_ref().map_or(0, |r| r.len());
            self.players.push(Player {
                kingdom: id,
                index,
                profile,
                ego,
                ge_due: world.time,
                rai_due: world.time,
                known,
                scouted: vec![f32::NEG_INFINITY; regions],
                ..Default::default()
            });
        }
    }

    /// A Deploy refused by the deploy owner (spot blocked or not reachable:
    /// `Event::Refused`, the truck's order reset to Idle) leaves the truck
    /// without a deployment once the command was sent. The AI drops that
    /// region; a truck is planned again at once at most once per goal-engine
    /// period, later refusals wait for the next regular think (no retry
    /// loop when the truck itself cannot move).
    fn detect_refused_deploys(&mut self, world: &World, data: &AiData) {
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
                let period = data.egos[p.ego].think_frequency.max(0.5);
                if memo.replanned_at <= 0.0 || now - memo.replanned_at >= period {
                    memo.replanned_at = now;
                    p.ge_due = p.ge_due.min(now);
                }
                if logging {
                    lines.push((p.kingdom, format!("deploy of {} refused at ({:.0},{:.0}); another site", world.data.def(a.def).id, memo.site.x, memo.site.y)));
                }
            }
        }
        for (k, text) in lines {
            self.push_log(now, k, vec![text]);
        }
    }

    /// Structures created since the last tick trigger the immediate rethink
    /// and the new-structure defend flare of `7B2350`. The original calls it
    /// from `KKC_ConstructionComponent::OnConstructionFinished` only: a
    /// truck's unpacked building is not a construction (the goal CSV shows
    /// no extra think and no +500 defend flare when the supply depot, the
    /// Infantry Division HQ or the Artillery Brigade unpack).
    fn detect_structures(&mut self, world: &World, data: &AiData) {
        if world.actors.len() == self.actors_seen {
            return;
        }
        let from = self.actors_seen;
        self.actors_seen = world.actors.len();
        for a in &world.actors[from.min(world.actors.len())..] {
            if !a.alive || a.removed || !matches!(world.data.def(a.def).class, Some(crate::data::DefClass::Building)) {
                continue;
            }
            if world.data.defs.iter().any(|d| d.unpacks_to == Some(a.def)) {
                continue;
            }
            let Some(k) = a.kingdom else { continue };
            for p in self.players.iter_mut().filter(|p| p.kingdom == k) {
                if !p.structures.insert(a.id) || p.ge_count == 0 {
                    continue;
                }
                let ego = &data.egos[p.ego];
                if 0 <= ego.immediate_think_on_build_max_settlements && world.time < ego.immediate_think_on_build_max_time {
                    p.ge_due = p.ge_due.min(world.time);
                }
                // 7B2350: a new structure is defended for a while.
                if let Some(r) = self.regions.as_ref().map(|r| r.region_at(a.pos.truncate())) {
                    p.flares.push((goals::Key::Defend(r), ego.flares.defend_new_structure_priority, world.time + ego.flares.defend_new_structure_duration));
                }
            }
        }
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
        let mut sources = Vec::new();
        for org in world.orgs.iter().filter(|o| o.alive) {
            let Some(owner) = view::kingdom_index(world, org.kingdom) else { continue };
            let (mut cv, mut sum, mut n, mut speed) = (0.0, Vec2::ZERO, 0, f32::MAX);
            for &m in &org.members {
                let Some(a) = world.actors.get(m as usize).filter(|a| a.alive && !a.removed) else { continue };
                let d = world.data.def(a.def);
                cv += self.bound.cv.get(a.def as usize).copied().unwrap_or(0.0) * (a.health / d.health.max(1e-3)).clamp(0.0, 1.0);
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
            let cv = self.bound.cv.get(a.def as usize).copied().unwrap_or(0.0) * (a.health / d.health.max(1e-3)).clamp(0.0, 1.0);
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

/// `7AF220` skips actors that died or are being deleted since the think.
fn still_valid(world: &World, command: Command) -> Option<Command> {
    let live = |u: &Unit| match *u {
        Unit::Org(o) => world.orgs.get(o as usize).is_some_and(|o| o.alive),
        Unit::Actor(a) => world.actor(a).is_some_and(|a| a.alive),
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
        Command::Recruit { building, .. } | Command::Pack { building, .. } | Command::CancelRecruit { building, .. } => {
            world.actor(building).is_some_and(|a| a.alive).then_some(command)
        }
        Command::Deploy { truck, .. } => world.actor(truck).is_some_and(|a| a.alive).then_some(command),
        Command::Research { building, .. } => world.actor(building).is_some_and(|a| a.alive).then_some(command),
        Command::Attach { org, hq, .. } => {
            (world.orgs.get(org as usize).is_some_and(|o| o.alive) && world.actor(hq).is_some_and(|a| a.alive)).then_some(command)
        }
        // HUD player orders (appended by the UI owner; the AI sends none)
        Command::SetFormation { .. } | Command::SetRallyPoint { .. } | Command::Sell { .. } | Command::Detach { .. } | Command::Disband { .. } => Some(command),
        // special operations (appended by the combat owner; the SAI general
        // powers apply them directly from `special_ops::sai`)
        Command::UseAbility { .. } => Some(command),
        // a right-click on an actor (appended by the movement owner; the AI
        // sends none)
        Command::Follow { kingdom, units, target } => {
            let units: Vec<Unit> = units.into_iter().filter(live).collect();
            (!units.is_empty() && world.actor(target).is_some_and(|a| !a.removed)).then_some(Command::Follow { kingdom, units, target })
        }
        // the HUD orders (appended by the core owner; the AI sends none)
        Command::Bombard { .. } | Command::Repair { .. } | Command::Construct { .. } | Command::AirAttack { .. } | Command::AirRecon { .. } => Some(command),
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
        if let Command::Move { units, .. } | Command::Attack { units, .. } | Command::Stop { units, .. } = &c {
            for u in units {
                if let Unit::Org(o) = u {
                    // The order shows on the organization a tick after it is sent.
                    player.pending.insert(*o, due + 0.5);
                }
            }
        }
        player.queue.push_back((due, c));
    }
}
