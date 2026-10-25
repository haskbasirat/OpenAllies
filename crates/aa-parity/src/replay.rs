//! H4 replay driver: a film's decrees into aa-play on the film's own stock
//! map, checked against the film's S checkpoints.
//!
//! * [`plan`] reads the battle the film was recorded in (SessionPlayers,
//!   the creator's kingdom parameters, the world rules) and builds the
//!   matching `aa_play::SkirmishSetup` on the loaded map. Every player keeps
//!   its film kingdom (SessionPlayers Kingdom+0x188 index → the map slot with
//!   that KINGDOMS +0x188), nation, general, colour and team. Computer players
//!   (their kingdom was renamed "<General> AI" by an UpdateKingdomName decree,
//!   the SAI naming) become `Controller::Computer` with the film's difficulty
//!   handicap, but the SAI is never attached: their decisions are the film's
//!   decrees. Everything the plan could not take from the film is listed in
//!   [`ReplayPlan::notes`].
//! * [`run`] steps the world once per film record: record `r` runs right
//!   before update `r + 1` (`World::step_decrees`), each decree first logs its
//!   S entries (class name and descriptor, nothing for SetPlayer) and then
//!   applies the commands [`Mapper`] made of it through [`PlayIds`]. After
//!   step `c` the trace must equal a checkpoint carried by record `c` (S last
//!   index and hash: `c` updates and the decrees of records `0..c`).
//! * At the first checkpoint that differs, [`FirstDifference`] counts the S
//!   entries of both sides in that interval by kind (clock, decree, getter,
//!   other) and, when the getter count agrees, solves the film's interval for
//!   the low 16 bits of the world-RNG state at aa-play's getter placement
//!   ([`crate::chain::solve`]), so a state difference shows as a value.
use crate::chain;
use crate::decree::Decree;
use crate::film::{Film, KingdomInfo};
use crate::mapping::{Mapped, Mapper, PlayIds};
use crate::slog::{self, decree_s_entries};
use aa_play::sim::TracedDecree;
use aa_play::trace::{DECREE_ID_LABEL, SKind};
use aa_play::{Controller, GameData, GameMap, PlayerSetup, SkirmishSetup, World};
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::Arc;

/// One film player as the replay sets it up.
#[derive(Clone, Debug, Serialize)]
pub struct PlayerPlan {
    /// SessionPlayer+0x20 id.
    pub player: u32,
    pub name: String,
    /// Kingdom+0x188 index (SessionPlayers).
    pub kingdom_index: u32,
    /// Map kingdom slot (`GameMap::slots`) and its header id.
    pub slot: usize,
    pub slot_id: String,
    /// Index into `GameMap::start_locations`.
    pub start: usize,
    /// aa-play nation id ("rus") and the film general ("zhukov").
    pub nation: String,
    pub general: String,
    pub team: u32,
    pub team_id: String,
    pub color_id: String,
    pub handicap: String,
    pub computer: bool,
    /// The creator kept the staging "Random" (empty nation / general): the
    /// original draws the pick in WorldParameters 0x50E8B0 at map restore.
    pub random_nation: bool,
    pub random_general: bool,
}

/// The aa-play battle a film is replayed in.
#[derive(Clone, Debug, Serialize)]
pub struct ReplayPlan {
    pub players: Vec<PlayerPlan>,
    #[serde(skip)]
    pub setup: SkirmishSetup,
    /// World rule StartingExperience of the film header, when present.
    pub starting_experience: Option<f32>,
    /// Where the film's battle could not be taken over as it is.
    pub notes: Vec<String>,
}

fn utf16(v: &[u16]) -> String {
    String::from_utf16_lossy(v)
}

fn nation_id(nation: &str) -> String {
    nation.trim_start_matches("nation_").to_ascii_lowercase()
}

/// A film rule value as f32. Type 2 holds binary32 bits (500.0 for
/// ResourceStockpileDefault, 200.0 for StartingExperience in the Berlin
/// film); types 0, 1 and 3 an integer (DifficultyLevel 1, FogOfWar 1).
fn rule_value(film: &Film, key: &str) -> Option<f32> {
    let rule = film
        .header
        .map_header
        .creator
        .rules
        .rules
        .iter()
        .find(|r| utf16(&r.key).eq_ignore_ascii_case(key))?;
    Some(match rule.kind {
        2 => f32::from_bits(rule.value),
        _ => rule.value as i32 as f32,
    })
}

/// The kingdom ids the film renamed to "<General> AI" (the SAI naming of a
/// computer player's kingdom).
fn computer_kingdoms(film: &Film) -> Vec<String> {
    let mut out = Vec::new();
    for t in &film.ticks {
        for d in &t.decrees {
            if let Decree::UpdateKingdomName { kingdom, name, .. } = &d.decree
                && utf16(name).ends_with(" AI")
            {
                out.push(utf16(kingdom));
            }
        }
    }
    out
}

/// The replay battle for `film` on `map` (the film's stock map, loaded with
/// `GameMap::load_with`).
pub fn plan(film: &Film, data: &GameData, map: &GameMap) -> anyhow::Result<ReplayPlan> {
    let mut notes = Vec::new();
    let computers = computer_kingdoms(film);
    let creator: &[KingdomInfo] = &film.header.creator_kingdoms;
    let mut players = Vec::new();
    for p in &film.players {
        let Some(index) = p.kingdom_index else {
            notes.push(format!(
                "player {} {:?} has no kingdom: not set up",
                p.id, p.name
            ));
            continue;
        };
        let slot = map
            .slots
            .iter()
            .position(|s| s.kingdom_index == Some(index))
            .or_else(|| {
                map.slots
                    .iter()
                    .position(|s| s.id.eq_ignore_ascii_case(&p.name))
            })
            .ok_or_else(|| {
                anyhow::anyhow!("player {}: no map slot with kingdom index {index}", p.id)
            })?;
        let s = &map.slots[slot];
        if !s.id.eq_ignore_ascii_case(&p.name) {
            notes.push(format!(
                "player {} {:?}: kingdom index {index} is map slot {:?}",
                p.id, p.name, s.id
            ));
        }
        let start = map
            .player_slots
            .iter()
            .position(|&ps| ps == slot)
            .ok_or_else(|| anyhow::anyhow!("map slot {} has no start location", s.id))?;
        let info = creator.iter().find(|k| k.id.eq_ignore_ascii_case(&s.id));
        let Some(info) = info else {
            anyhow::bail!("film creator has no kingdom {:?}", s.id);
        };
        // The creator keeps the staging input ("Random": empty nation and
        // general); the header's own kingdom parameters carry the picks.
        let picked = film
            .header
            .header_kingdoms
            .iter()
            .find(|k| k.id.eq_ignore_ascii_case(&s.id));
        let pick = |f: fn(&KingdomInfo) -> &String| -> String {
            picked
                .map(f)
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| f(info))
                .clone()
        };
        let (nation_raw, general, handicap) = (
            pick(|k| &k.nation),
            pick(|k| &k.faction),
            pick(|k| &k.handicap),
        );
        let nation = nation_id(&nation_raw);
        if data.nation(&nation).is_none() {
            anyhow::bail!("player {}: unknown nation {nation_raw:?}", p.id);
        }
        if info.nation.is_empty() || info.faction.is_empty() {
            notes.push(format!(
                "{}: staging nation {:?} general {:?} (Random) resolved to {nation_raw:?} {general:?} by the original before the film starts",
                s.id, info.nation, info.faction
            ));
        }
        if let Some(n) = &s.nation
            && *n != nation
        {
            notes.push(format!(
                "{}: map nation {n}, film nation {nation} (film kept)",
                s.id
            ));
        }
        let team = map
            .teams
            .iter()
            .position(|t| t.eq_ignore_ascii_case(&info.team))
            .map_or(s.team, |t| t as u32);
        if !info.team.is_empty() && !info.team.eq_ignore_ascii_case(&s.team_id) {
            notes.push(format!(
                "{}: map team {:?}, film team {:?} (film kept)",
                s.id, s.team_id, info.team
            ));
        }
        if !info.color.eq_ignore_ascii_case(&s.color_id) {
            notes.push(format!(
                "{}: map colour {:?}, film colour {:?} (film kept)",
                s.id, s.color_id, info.color
            ));
        }
        players.push(PlayerPlan {
            player: p.id,
            name: p.name.clone(),
            kingdom_index: index,
            slot,
            slot_id: s.id.clone(),
            start,
            nation,
            general,
            team,
            team_id: info.team.clone(),
            color_id: info.color.clone(),
            handicap,
            computer: computers.iter().any(|c| c.eq_ignore_ascii_case(&s.id)),
            random_nation: info.nation.is_empty(),
            random_general: info.faction.is_empty(),
        });
    }
    // Native kingdom creation follows the creator's kingdom list
    // (WP::CreateKingdomsAndTeams); aa-play numbers its kingdoms in setup
    // order, so keep that order.
    players.sort_by_key(|p| {
        creator
            .iter()
            .position(|k| k.id.eq_ignore_ascii_case(&p.slot_id))
    });
    // World rule DifficultyLevel (the staging difficulty, Game/difficulty_level
    // order 0 easy, 1 medium, 2 hard) for every computer player.
    let difficulty = rule_value(film, "DifficultyLevel");
    if difficulty.is_none() {
        notes.push("no DifficultyLevel rule: computer players get medium (1)".into());
    }
    let level = difficulty.map_or(1, |v| v.max(0.0) as u8);
    let setup_players = players
        .iter()
        .map(|p| {
            let rgb = data
                .kingdom_colors
                .iter()
                .find(|(c, _)| c.eq_ignore_ascii_case(&p.color_id))
                .map_or([1.0, 1.0, 1.0], |c| c.1);
            PlayerSetup {
                name: p.slot_id.clone(),
                nation: p.nation.clone(),
                team: p.team,
                color: rgb.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8),
                controller: if p.computer {
                    Controller::Computer(level)
                } else {
                    Controller::Human
                },
                start: p.start,
            }
        })
        .collect();
    let unused: Vec<&str> = map
        .player_slots
        .iter()
        .filter(|&&s| !players.iter().any(|p| p.slot == s))
        .map(|&s| map.slots[s].id.as_str())
        .collect();
    if !unused.is_empty() {
        notes.push(format!(
            "map slots without a film player (destroyed at setup): {unused:?}"
        ));
    }
    let starting_resources = rule_value(film, "ResourceStockpileDefault");
    Ok(ReplayPlan {
        players,
        setup: SkirmishSetup {
            map_id: map.id.clone(),
            players: setup_players,
            starting_resources,
            seed: u64::from(film.header.seed),
        },
        starting_experience: rule_value(film, "StartingExperience"),
        notes,
    })
}

/// The aa-play world of `plan`, recording its S trace: the custom battle
/// with the original's random picks where the film's creator kept "Random"
/// (`World::new_skirmish_with`, the WorldParameters draws), then the film's
/// generals and starting experience (no draw). The picks must reproduce the
/// film header's; [`pick_mismatches`] reports any that does not.
pub fn build(
    plan: &ReplayPlan,
    game_root: &std::path::Path,
    data: Arc<GameData>,
    map: Arc<GameMap>,
) -> anyhow::Result<World> {
    let picks: Vec<aa_play::creation::StartPick> = plan
        .players
        .iter()
        .map(|p| aa_play::creation::StartPick {
            random_nation: p.random_nation,
            random_general: p.random_general,
        })
        .collect();
    let mut world = World::new_skirmish_with(data.clone(), map, &plan.setup, &picks)?;
    let generals = aa_play::special_ops::GeneralData::load(game_root, &data)?;
    aa_play::special_ops::attach(&mut world, Arc::new(generals));
    if let Some(x) = plan.starting_experience {
        aa_play::special_ops::set_starting_experience(&mut world, x);
    }
    for (k, p) in plan.players.iter().enumerate() {
        aa_play::special_ops::set_general(&mut world, k as u32, Some(&p.general));
    }
    world.trace.record(true);
    Ok(world)
}

/// Players whose aa-play nation or drawn general differs from the film
/// header's pick (empty when the random picks reproduce the film).
pub fn pick_mismatches(plan: &ReplayPlan, world: &World) -> Vec<String> {
    let mut out = Vec::new();
    for (k, p) in plan.players.iter().enumerate() {
        let Some(kingdom) = world.kingdoms.get(k) else {
            out.push(format!("{}: no aa-play kingdom {k}", p.slot_id));
            continue;
        };
        if kingdom.nation != p.nation {
            out.push(format!(
                "{}: nation {} against the film's {}",
                p.slot_id, kingdom.nation, p.nation
            ));
        }
        let drawn = world
            .books
            .kingdom(k as u32)
            .and_then(|b| b.general.clone());
        if p.random_general
            && drawn
                .as_deref()
                .is_some_and(|g| !g.eq_ignore_ascii_case(&p.general))
        {
            out.push(format!(
                "{}: general {drawn:?} against the film's {}",
                p.slot_id, p.general
            ));
        }
    }
    out
}

/// What aa-play has in a native GTID slot at the start (creation-order check
/// against the film's selections).
#[derive(Clone, Debug, Serialize)]
pub struct SlotCheck {
    pub slot: u16,
    /// The film player that selected it first, and its plan kingdom.
    pub player: Option<u32>,
    pub expected_kingdom: Option<u32>,
    /// What aa-play has there: description and owning kingdom.
    pub ours: String,
    pub our_kingdom: Option<u32>,
}

fn describe_slot(world: &World, slot: u16) -> (String, Option<u32>) {
    use aa_play::ids::Thing;
    match world.ids.thing_at_slot(slot) {
        None => ("nothing".into(), None),
        Some(Thing::Actor(a)) => {
            let actor = &world.actors[a as usize];
            (
                format!("actor {a} {}", world.data.def(actor.def).id),
                actor.kingdom,
            )
        }
        Some(Thing::Org(o)) => {
            let org = &world.orgs[o as usize];
            (
                format!("organization {o} {}", world.data.def(org.def).id),
                Some(org.kingdom),
            )
        }
        Some(other) => (format!("{other:?}"), None),
    }
}

/// Every thing slot the film's players select before `ticks`, against what
/// aa-play has in it now (call on the freshly built world).
pub fn slot_checks(film: &Film, plan: &ReplayPlan, world: &World, ticks: u32) -> Vec<SlotCheck> {
    let mut out: Vec<SlotCheck> = Vec::new();
    for t in film.ticks.iter().take(ticks as usize) {
        for d in &t.decrees {
            let slot = match d.decree {
                Decree::SelectionSet { slot } | Decree::SelectionAdd { slot } if slot != 0 => slot,
                _ => continue,
            };
            if out.iter().any(|c| c.slot == slot) {
                continue;
            }
            let expected_kingdom = d
                .player
                .and_then(|p| plan.players.iter().position(|q| q.player == p))
                .map(|k| k as u32);
            let (ours, our_kingdom) = describe_slot(world, slot);
            out.push(SlotCheck {
                slot,
                player: d.player,
                expected_kingdom,
                ours,
                our_kingdom,
            });
        }
    }
    out
}

/// One checkpoint against aa-play: (S last index, hash) of both.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct CheckpointCheck {
    pub tick: u32,
    pub film: (i32, i32),
    pub ours: (i32, i32),
}

/// aa-play's S entries of one checkpoint interval, by kind.
#[derive(Clone, Debug, Default, Serialize)]
pub struct EntryKinds {
    pub clock: u32,
    pub decree: u32,
    pub getters: u32,
    pub other: u32,
    /// (step, low 16 bits of the logged state) of every getter.
    pub getter_states: Vec<(u32, u32)>,
    /// The getters' slots in the film interval's slot list (`chain`).
    pub getter_slots: Vec<String>,
}

/// The first checkpoint that differs and the interval before it.
#[derive(Clone, Debug, Serialize)]
pub struct FirstDifference {
    pub check: CheckpointCheck,
    pub from_tick: u32,
    /// What the film's own model accounts for in the interval.
    pub film: slog::Interval,
    pub ours: EntryKinds,
    /// When both sides log the same number of getters: the low halves of
    /// the first getter's state that would explain the film's hash with the
    /// getters where aa-play logged them (one draw each).
    pub film_states_at_our_placement: Option<Vec<u32>>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct ReplayReport {
    pub steps: u32,
    pub checked: usize,
    pub matched: usize,
    pub first_difference: Option<FirstDifference>,
    /// Decrees by mapping outcome ("commands", "state", "no effect: ...",
    /// "no equivalent: ...", "unresolved: ...").
    pub outcomes: BTreeMap<String, usize>,
    /// (record, ordering player, reason) of the first unresolved decrees.
    pub unresolved: Vec<(u32, Option<u32>, String)>,
    /// (step, kingdom, reason) of commands aa-play refused.
    pub refused: Vec<(u32, u32, String)>,
    pub commands: usize,
}

fn outcome_key(m: &Mapped) -> String {
    match m {
        Mapped::Commands(_) => "commands".into(),
        Mapped::State => "state".into(),
        Mapped::NoEffect(w) => format!("no effect: {w}"),
        Mapped::NoEquivalent(w) => format!("no equivalent: {w}"),
        Mapped::Unresolved(w) => format!("unresolved: {w}"),
    }
}

/// Film thing slots answered by a fixed aa-play unit before `PlayIds` is
/// asked (a diagnostic override for a known id difference).
pub type Aliases = BTreeMap<u16, aa_play::Unit>;

struct Aliased<'a> {
    ids: PlayIds<'a>,
    aliases: &'a Aliases,
}
impl crate::mapping::NativeIds for Aliased<'_> {
    fn kingdom(&self, player: u32) -> Option<aa_play::KingdomId> {
        self.ids.kingdom(player)
    }
    fn thing(&self, slot: u16) -> Option<aa_play::Unit> {
        self.aliases
            .get(&slot)
            .copied()
            .or_else(|| self.ids.thing(slot))
    }
    fn gtid(&self, gtid: u32) -> Option<aa_play::Unit> {
        self.aliases
            .get(&(gtid as u16))
            .copied()
            .or_else(|| self.ids.gtid(gtid))
    }
    fn target_actor(&self, unit: aa_play::Unit) -> Option<aa_play::ActorId> {
        self.ids.target_actor(unit)
    }
    fn definition(&self, object: u32) -> Option<aa_play::DefId> {
        self.ids.definition(object)
    }
    fn formation(&self, org: aa_play::OrgId, formation: u32) -> Option<usize> {
        self.ids.formation(org, formation)
    }
    fn property(&self, property: u32) -> Option<String> {
        self.ids.property(property)
    }
    fn general_ability(&self, kingdom: aa_play::KingdomId, ability: u32) -> Option<u32> {
        self.ids.general_ability(kingdom, ability)
    }
    fn named(&self, selector: &[u16]) -> Option<aa_play::Unit> {
        self.ids.named(selector)
    }
}

/// Replay up to `max_steps` film records into `world` (built by [`build`]).
/// With `stop_at_difference` the replay stops at the first differing
/// checkpoint.
pub fn run(
    film: &Film,
    world: &mut World,
    object_ids: &[String],
    max_steps: Option<u32>,
    stop_at_difference: bool,
) -> ReplayReport {
    run_aliased(
        film,
        world,
        object_ids,
        max_steps,
        stop_at_difference,
        &Aliases::new(),
    )
}

/// [`run`] with `aliases` answering some thing slots first.
pub fn run_aliased(
    film: &Film,
    world: &mut World,
    object_ids: &[String],
    max_steps: Option<u32>,
    stop_at_difference: bool,
    aliases: &Aliases,
) -> ReplayReport {
    run_observed(
        film,
        world,
        object_ids,
        max_steps,
        stop_at_difference,
        aliases,
        &mut |_, _| {},
    )
}

/// SharedPropertyData slot -> property IDS for every replay of this process,
/// set once by the caller from the measured native table (the v14 one is the
/// git-ignored fixture `tests/fixtures/native_shared_property_slots_v14.json`,
/// NativeSampler `REGIDS` of `[0xB3AA2C]`). Unset: research orders stay
/// unresolved.
pub fn set_property_ids(ids: Vec<String>) {
    let _ = PROPERTY_IDS.set(ids);
}
static PROPERTY_IDS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();

/// [`run_aliased`] calling `observe(step, world)` after every step (the
/// world after GWorld update `step`, as the native sampler's 0x76F850
/// snapshot of the same tick).
pub fn run_observed(
    film: &Film,
    world: &mut World,
    object_ids: &[String],
    max_steps: Option<u32>,
    stop_at_difference: bool,
    aliases: &Aliases,
    observe: &mut dyn FnMut(u32, &World),
) -> ReplayReport {
    let mut report = ReplayReport::default();
    let mut mapper = Mapper::new();
    let checkpoints: BTreeMap<u32, (i32, i32)> = film
        .checkpoints
        .iter()
        .map(|c| (c.tick, (c.count, c.hash)))
        .collect();
    let intervals = slog::account(film);
    let prepared = chain::prepare(film);
    let mut previous_tick = 0u32;
    let steps = max_steps.map_or(film.ticks.len() as u32, |m| m.min(film.ticks.len() as u32));
    for record in 0..steps {
        let mut traced = Vec::new();
        {
            let ids = Aliased {
                ids: PlayIds {
                    world: &*world,
                    players: &film.players,
                    object_ids,
                    property_ids: PROPERTY_IDS.get().map_or(&[], |v| v.as_slice()),
                },
                aliases,
            };
            for d in film.decrees_at(record) {
                let mapped = mapper.map(d, &ids);
                *report.outcomes.entry(outcome_key(&mapped)).or_default() += 1;
                if let Mapped::Unresolved(why) = &mapped
                    && report.unresolved.len() < 64
                {
                    report.unresolved.push((record, d.player, why.clone()));
                }
                let sync = decree_s_entries(d.descriptor as usize)
                    .map(|name| (u32::from(d.descriptor), name.encode_utf16().collect()));
                let commands = match mapped {
                    Mapped::Commands(c) => c,
                    _ => Vec::new(),
                };
                report.commands += commands.len();
                traced.push(TracedDecree { sync, commands });
            }
        }
        world.step_decrees(traced);
        let step = record + 1;
        report.steps = step;
        observe(step, world);
        for e in &world.events {
            if let aa_play::Event::Refused { kingdom, reason } = e
                && report.refused.len() < 256
            {
                report.refused.push((step, *kingdom, reason.clone()));
            }
        }
        if let Some(&film_state) = checkpoints.get(&step) {
            report.checked += 1;
            let ours = world.trace.checkpoint();
            if ours == film_state {
                report.matched += 1;
            } else if report.first_difference.is_none() {
                let check = CheckpointCheck {
                    tick: step,
                    film: film_state,
                    ours,
                };
                let film_iv = intervals.iter().find(|iv| iv.to_tick == step).cloned();
                let prepared_iv = prepared.iter().find(|iv| iv.to_tick == step);
                let ours_kinds = kinds(world, previous_tick, step, prepared_iv);
                let film_states = match (&film_iv, prepared_iv) {
                    (Some(f), Some(p)) if f.residual.max(0) as u32 == ours_kinds.getters => {
                        let pick = pick_of(world, previous_tick, step, p);
                        pick.map(|pick| chain::solve(p, &pick, 1))
                    }
                    _ => None,
                };
                if let Some(film) = film_iv {
                    report.first_difference = Some(FirstDifference {
                        check,
                        from_tick: previous_tick,
                        film,
                        ours: ours_kinds,
                        film_states_at_our_placement: film_states,
                    });
                }
                if stop_at_difference {
                    break;
                }
            }
            previous_tick = step;
        }
    }
    report
}

/// aa-play's entries logged in steps `from + 1 ..= to`.
fn interval_entries(world: &World, from: u32, to: u32) -> Vec<(u32, SKind, u32)> {
    world
        .trace
        .entries()
        .filter(|(t, _, _)| *t > from && *t <= to)
        .collect()
}

fn kinds(world: &World, from: u32, to: u32, prepared: Option<&chain::Interval>) -> EntryKinds {
    let mut k = EntryKinds::default();
    for (tick, kind, value) in interval_entries(world, from, to) {
        match kind {
            SKind::Clock => k.clock += 1,
            SKind::DecreeName => k.decree += 1,
            SKind::Int(l) if l == DECREE_ID_LABEL => k.decree += 1,
            SKind::RngGetter => {
                k.getters += 1;
                k.getter_states.push((tick, value & 0xFFFF));
            }
            _ => k.other += 1,
        }
    }
    if let Some(p) = prepared
        && let Some(pick) = pick_of(world, from, to, p)
    {
        k.getter_slots = pick.iter().map(|&i| format!("{:?}", p.slots[i])).collect();
    }
    k
}

/// The slot (index into `prepared.slots`) after which each of aa-play's
/// getters of the interval was logged: the fixed entries (decree pairs and
/// clocks) come in the film's slot order; a getter sits after the last one
/// passed. `None` when the fixed entries do not line up with the film's.
fn pick_of(world: &World, from: u32, to: u32, prepared: &chain::Interval) -> Option<Vec<usize>> {
    let mut passed: Option<usize> = None;
    let mut next = 0usize;
    let mut pick = Vec::new();
    let mut pending_name = false;
    for (_, kind, _) in interval_entries(world, from, to) {
        match kind {
            SKind::DecreeName => pending_name = true,
            SKind::Int(l) if l == DECREE_ID_LABEL && pending_name => {
                pending_name = false;
                if !matches!(prepared.slots.get(next)?, chain::Slot::Decree { .. }) {
                    return None;
                }
                passed = Some(next);
                next += 1;
            }
            SKind::Clock => {
                if !matches!(prepared.slots.get(next)?, chain::Slot::Update { .. }) {
                    return None;
                }
                passed = Some(next);
                next += 1;
            }
            SKind::RngGetter => pick.push(passed?),
            _ => {}
        }
    }
    Some(pick)
}
