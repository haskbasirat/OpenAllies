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
//! * [`run`] steps the world once per film record through [`FilmDriver`], the
//!   decree loop the game client plays through too (`crate::playback`), so the
//!   gate and the library cannot drift: record `r` runs right before update
//!   `r + 1`, each decree first logs its S entries (class name and descriptor,
//!   nothing for SetPlayer) and then applies the commands [`Mapper`] made of it
//!   through [`PlayIds`]. A 311 carried by record `c` is checked by the native
//!   rule 0x77D050 against S's entry at its index (`c` updates and the decrees
//!   of records `0..c`).
//! * At the first checkpoint that differs, [`FirstDifference`] counts the S
//!   entries of both sides in that interval by kind (clock, decree, getter,
//!   other) and, when the getter count agrees, solves the film's interval for
//!   the low 16 bits of the world-RNG state at aa-play's getter placement
//!   ([`crate::chain::solve`]), so a state difference shows as a value.
//! * The replay does not stop there: [`ReplayReport::intervals`] keeps every
//!   checkpoint's S counts and [`ReplayReport::orders`] every film order's
//!   outcome, to the end of the film.
use crate::chain;
use crate::decree::{Decree, id};
use crate::film::{Film, FilmDecree, KingdomInfo, SessionPlayer};
use crate::mapping::{Mapped, Mapper, PlayIds};
use crate::slog::{self, decree_s_entries};
use aa_orders::net::sync::{DEFAULT_MAX_ENTRIES, Synchronizer};
use aa_orders::payload::Field;
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
    /// The creator's sides without a player, whose picks draw all the same.
    #[serde(skip)]
    pub closed: Vec<aa_play::creation::ClosedSide>,
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
pub fn computer_kingdoms(film: &Film) -> Vec<String> {
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
        // Two session players on one kingdom: both order for it (0x76E1D0 binds
        // each to GWorld+0x148[index]), the world has one.
        if players.iter().any(|q: &PlayerPlan| q.slot == slot) {
            notes.push(format!("player {} shares kingdom {index} with another player", p.id));
            continue;
        }
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
        // A kingdom with no team stands alone (a team number past the map's,
        // as the staging's no-team team).
        let team = if info.team.is_empty() {
            (map.teams.len() + start) as u32
        } else {
            map.teams
                .iter()
                .position(|t| t.eq_ignore_ascii_case(&info.team))
                .map_or(s.team, |t| t as u32)
        };
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
                    // A kingdom's own handicap, else the rule's level.
                    let own = data.handicaps.iter().position(|h| !p.handicap.is_empty() && h.0.eq_ignore_ascii_case(&p.handicap));
                    Controller::Computer(own.map_or(level, |i| i as u8))
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
    // 0x50E8B0 resolves every kingdom of the creator, with a player or not.
    let closed = creator
        .iter()
        .filter(|k| !k.independent)
        .filter_map(|k| {
            let slot = map.slots.iter().position(|s| s.id.eq_ignore_ascii_case(&k.id))?;
            let taken = players.iter().any(|p| p.slot == slot);
            (!taken).then(|| aa_play::creation::ClosedSide {
                slot,
                nation: nation_id(&k.nation),
                pick: aa_play::creation::StartPick { random_nation: k.nation.is_empty(), random_general: k.faction.is_empty() },
            })
        })
        .collect();
    Ok(ReplayPlan {
        closed,
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
    let generals = aa_play::special_ops::GeneralData::load(game_root, &data)?;
    build_with(plan, Arc::new(generals), data, map)
}

/// [`build`] with the generals' data already loaded.
pub fn build_with(
    plan: &ReplayPlan,
    generals: Arc<aa_play::special_ops::GeneralData>,
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
    let mut world = World::new_skirmish_closed(data.clone(), map, &plan.setup, &picks, &plan.closed)?;
    aa_play::special_ops::attach(&mut world, generals);
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

/// One checkpoint's S counts: entries of the film and of aa-play since the
/// previous checkpoint.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct IntervalCheck {
    pub check: CheckpointCheck,
    pub film_entries: i32,
    pub our_entries: i32,
}

/// One film command decree (86, 87, 65, 66, 67, 70) and what became of it.
#[derive(Clone, Debug, Serialize)]
pub struct OrderRow {
    pub record: u32,
    pub player: Option<u32>,
    /// The ActorCommand name, or the decree name.
    pub name: String,
    /// The selection it applied to (thing slots).
    pub slots: Vec<u16>,
    /// `outcome_key` of the mapping.
    pub outcome: String,
    /// What the mapper made of it, in order.
    pub commands: Vec<String>,
    /// aa-play's refusals of those commands.
    pub refused: Vec<String>,
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
    /// (record, ordering player, reason) of the unresolved decrees.
    pub unresolved: Vec<(u32, Option<u32>, String)>,
    /// (step, kingdom, reason) of commands aa-play refused.
    pub refused: Vec<(u32, u32, String)>,
    pub commands: usize,
    /// Every checkpoint, to the end of the film.
    pub intervals: Vec<IntervalCheck>,
    /// Every film order.
    pub orders: Vec<OrderRow>,
    /// Order fields aa-play's `Command` cannot carry, by name.
    pub dropped: BTreeMap<&'static str, u32>,
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

/// The things created during the battle, native GTID to aa-play GTID: the
/// k-th native thing of a definition with the k-th aa-play thing of it. The
/// native ids come from the sampler's names (`NativeTrace::names`) in ascending
/// slot, which is creation order. A census of the whole film needs this: one
/// slot shift (the first one is the unpack's next-fit id, 0x4D5EA0 -> 0x4EFB90)
/// would otherwise send every later film order and comparison to another
/// thing. Where the two ids differ is a cause of its own ([`Pairing::shifted`]).
/// Things of the map keep their saved ids on both sides.
#[derive(Clone, Debug, Default)]
pub struct Pairing {
    first_slot: u16,
    scan: u32,
    pending: BTreeMap<String, std::collections::VecDeque<u32>>,
    native_to_ours: BTreeMap<u16, u32>,
    ours_to_native: BTreeMap<u16, u32>,
}

impl Pairing {
    /// `first_slot`: the id cursor at battle start. `names`: GTID -> definition
    /// IDS of the native things.
    pub fn new(first_slot: u16, names: &BTreeMap<u32, String>) -> Self {
        let mut natives: Vec<(u16, u32, &String)> = names
            .iter()
            .filter(|(g, _)| (**g & 0xFFFF) as u16 >= first_slot)
            .map(|(g, n)| ((*g & 0xFFFF) as u16, *g, n))
            .collect();
        natives.sort_by_key(|n| n.0);
        let mut pending: BTreeMap<String, std::collections::VecDeque<u32>> = BTreeMap::new();
        for (_, g, n) in natives {
            pending
                .entry(n.to_ascii_lowercase())
                .or_default()
                .push_back(g);
        }
        Self {
            first_slot,
            scan: u32::from(first_slot),
            pending,
            ..Default::default()
        }
    }
    /// Pair the things aa-play created since the last call.
    pub fn update(&mut self, world: &World) {
        let cursor = u32::from(world.ids.next_slot());
        while self.scan < cursor {
            let slot = self.scan as u16;
            self.scan += 1;
            let Some(gtid) = world.ids.gtid_at_slot(slot) else {
                continue;
            };
            let Some(thing) = world.ids.thing_at_slot(slot) else {
                continue;
            };
            let name = thing_definition(world, thing).to_ascii_lowercase();
            if let Some(native) = self.pending.get_mut(&name).and_then(|q| q.pop_front()) {
                self.native_to_ours
                    .insert((native & 0xFFFF) as u16, gtid.raw());
                self.ours_to_native.insert(slot, native);
            }
        }
    }
    /// aa-play's GTID for a native thing, `None` when it has no counterpart
    /// yet. Map things are themselves.
    pub fn ours(&self, native: u32) -> Option<u32> {
        let slot = (native & 0xFFFF) as u16;
        if slot < self.first_slot {
            return Some(native);
        }
        self.native_to_ours.get(&slot).copied()
    }
    /// The native GTID of aa-play's thing, `None` when it has no counterpart.
    pub fn native(&self, ours: u32) -> Option<u32> {
        let slot = (ours & 0xFFFF) as u16;
        if slot < self.first_slot {
            return Some(ours);
        }
        self.ours_to_native.get(&slot).copied()
    }
    /// (native, aa-play) GTIDs of the pairs whose slots differ.
    pub fn shifted(&self) -> Vec<(u32, u32)> {
        self.ours_to_native
            .iter()
            .filter(|(o, n)| **o != (**n & 0xFFFF) as u16)
            .map(|(o, n)| (*n, u32::from(*o)))
            .collect()
    }
}

/// The definition IDS of an aa-play thing.
pub fn thing_definition(world: &World, thing: aa_play::ids::Thing) -> String {
    use aa_play::ids::Thing;
    let id_of = |d: aa_play::DefId| world.data.def(d).id.clone();
    match thing {
        Thing::Actor(a) => world
            .actors
            .get(a as usize)
            .map_or(String::new(), |x| id_of(x.def)),
        Thing::Org(o) => world
            .orgs
            .get(o as usize)
            .map_or(String::new(), |x| id_of(x.def)),
        Thing::Trailer(a) => world
            .actors
            .get(a as usize)
            .and_then(|x| world.data.def(x.def).trailer)
            .map_or(String::new(), id_of),
        _ => String::new(),
    }
}

struct Aliased<'a> {
    ids: PlayIds<'a>,
    aliases: &'a Aliases,
    pairing: Option<&'a Pairing>,
}
impl crate::mapping::NativeIds for Aliased<'_> {
    fn kingdom(&self, player: u32) -> Option<aa_play::KingdomId> {
        self.ids.kingdom(player)
    }
    fn thing(&self, slot: u16) -> Option<aa_play::Unit> {
        if let Some(u) = self.aliases.get(&slot) {
            return Some(*u);
        }
        match self.pairing {
            // A film slot names the native thing that has it now.
            Some(p) => {
                let ours = p.ours(u32::from(slot))?;
                aa_play::ids::unit(self.ids.world.ids.thing_at_slot(ours as u16)?)
            }
            None => self.ids.thing(slot),
        }
    }
    fn gtid(&self, gtid: u32) -> Option<aa_play::Unit> {
        if let Some(u) = self.aliases.get(&(gtid as u16)) {
            return Some(*u);
        }
        match self.pairing {
            // The generations are the native's, the slot is what pairs.
            Some(p) if (gtid & 0xFFFF) as u16 >= p.first_slot => {
                let ours = p.ours(gtid)?;
                aa_play::ids::unit(self.ids.world.ids.thing_at_slot(ours as u16)?)
            }
            _ => self.ids.gtid(gtid),
        }
    }
    fn target_actor(&self, unit: aa_play::Unit) -> Option<aa_play::ActorId> {
        self.ids.target_actor(unit)
    }
    fn definition(&self, object: u32) -> Option<aa_play::DefId> {
        self.ids.definition(object)
    }
    fn layout(&self, layout: u32) -> Option<String> {
        self.ids.layout(layout)
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
    fn guess(
        &self,
        unit: aa_play::Unit,
        flags: u16,
        word: u32,
        non_group_excluded: bool,
    ) -> Option<u32> {
        // A target word names the native thing (flag bit 0).
        let word = match self.pairing {
            Some(p) if flags & 1 != 0 => p.ours(word).unwrap_or(word),
            _ => word,
        };
        self.ids.guess(unit, flags, word, non_group_excluded)
    }
    fn accepts(&self, kingdom: aa_play::KingdomId, unit: aa_play::Unit, index: u32) -> bool {
        self.ids.accepts(kingdom, unit, index)
    }
    fn selectable(&self, slot: u16) -> bool {
        !matches!(self.thing(slot), Some(aa_play::Unit::Actor(a)) if self.ids.world.actors.get(a as usize).is_some_and(|x| x.org.is_some()))
    }
    fn bombards(&self, unit: aa_play::Unit) -> bool {
        self.ids.bombards(unit)
    }
    fn hostile(&self, kingdom: aa_play::KingdomId, target: aa_play::ActorId) -> bool {
        self.ids.hostile(kingdom, target)
    }
    fn thing_kingdom(&self, slot: u16) -> Option<aa_play::KingdomId> {
        self.ids.thing_kingdom(slot)
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
        None,
        &mut |_, _, _| {},
    )
}

/// SharedPropertyData slot -> property IDS for every replay of this process,
/// set once by the caller (`Registries::property_ids`, computed from the data
/// and proven against the original's 194 live IDS in `tests/registry_counts.rs`).
/// Unset: research orders stay unresolved. The library does not use this: a
/// [`FilmDriver`] is handed the tables of its own data set ([`IdTables`]).
pub fn set_property_ids(ids: Vec<String>) {
    let _ = PROPERTY_IDS.set(ids);
}
static PROPERTY_IDS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();

/// FormationData slot -> formation IDS for every replay of this process, set
/// once by the caller (`Registries::formation_ids`). Unset: set_formation
/// orders stay unresolved.
pub fn set_formation_ids(ids: Vec<String>) {
    let _ = FORMATION_IDS.set(ids);
}
static FORMATION_IDS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();

/// The native id tables of the data set a film plays on (`Registries`).
#[derive(Clone, Copy, Debug)]
pub struct IdTables<'a> {
    /// Thing DataManager slot -> definition IDS.
    pub object_ids: &'a [String],
    /// SharedPropertyData slot -> property IDS.
    pub property_ids: &'a [String],
    /// FormationData slot -> formation IDS.
    pub formation_ids: &'a [String],
    /// AbilityDefinition slot -> IDS (empty: `mapping::set_ability_ids`).
    pub ability_ids: &'a [String],
    /// OrganizationLayout slot -> IDS (empty: `mapping::set_layout_ids`).
    pub layout_ids: &'a [String],
}

/// S (`[0xB36560]`) as the film check 0x77D050 sees it: the native
/// `Synchronizer` fed from what the world logged to `World::trace` (which keeps
/// only the running hash, not the hash after every entry).
#[derive(Clone, Debug)]
pub struct SimSync {
    sync: Synchronizer,
    /// Entries of `World::trace`'s stream already taken.
    fed: usize,
}

impl SimSync {
    /// S as `world` has it now. Its trace must be recording.
    pub fn new(world: &World) -> Self {
        let (index, hash) = world.trace.checkpoint();
        let mut sync = Synchronizer::new(DEFAULT_MAX_ENTRIES);
        sync.reset(index, hash);
        Self {
            sync,
            fed: world.trace.log.as_ref().map_or(0, Vec::len),
        }
    }
    /// Take what the world logged since the last call.
    pub fn feed(&mut self, world: &World) {
        let Some(log) = &world.trace.log else {
            return;
        };
        for entry in &log[self.fed.min(log.len())..] {
            let label = entry.kind.label();
            match entry.kind {
                SKind::Clock | SKind::Float(_) => {
                    self.sync.log_float_bits(true, label, entry.value)
                }
                SKind::RngGetter(_) | SKind::Int(_) => self.sync.log_int(true, label, entry.value),
                SKind::Bool(_) => self.sync.log_bool(true, label, entry.value != 0),
                SKind::Marker(_) => self.sync.log_marker(true, label),
                SKind::DecreeName => {
                    let units = world.trace.decree_name(entry.value).unwrap_or(&[]);
                    self.sync.log_text(true, label, units);
                }
            }
        }
        self.fed = log.len();
    }
    /// [`SimSync::feed`], then drop the world's stream and digests: a player
    /// runs for hours, and S itself retains what the check needs.
    pub fn feed_and_forget(&mut self, world: &mut World) {
        self.feed(world);
        if let Some(log) = &mut world.trace.log {
            log.clear();
        }
        world.trace.digests.clear();
        self.fed = 0;
    }
    /// The check of a 311 (0x77D050, which `Synchronizer::validate` ports) on
    /// the entries taken so far.
    ///
    /// `index == end` is the native accessor's quirk: 0x77D010 appends an empty
    /// entry first, and S's numbering moves by one. `native_append` keeps it for
    /// the native stop, where the film ends here unless the hash happens to be
    /// 0; a film that plays on would have every later check misnumbered, so
    /// without it the case only compares with the empty entry's hash.
    pub fn validate(&mut self, index: i32, hash: i32, native_append: bool) -> bool {
        if !native_append && index == self.sync.end() {
            return hash == 0;
        }
        self.sync.validate(index, hash)
    }
    /// The hash stored after entry `index`, when S still has it.
    pub fn hash_at(&self, index: i32) -> Option<i32> {
        self.sync.entry(index).map(|e| e.hash)
    }
}

/// What a record's decree does beyond its world effect, in decree order.
#[derive(Debug)]
pub enum DecreeSignal<'a> {
    /// What a decree became, before its commands apply. `selection` is the
    /// ordering player's selection before it.
    Mapped {
        decree: &'a FilmDecree,
        selection: &'a [u16],
        mapped: &'a Mapped,
    },
    /// A 311 was checked (0x752F60): its S index, its hash and the result.
    Checkpoint { index: i32, hash: i32, passed: bool },
    /// A 298 named a player the session does not have (0x75D710).
    NoOrderingPlayer { player: u32 },
    /// A 328 renamed a kingdom.
    KingdomRenamed {
        kingdom: aa_play::KingdomId,
        name: String,
    },
}

/// What the receiver of [`DecreeSignal`]s wants next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecreeFlow {
    Continue,
    /// End the record's loop here (0x74D710 with Admin+0x2E5 set).
    Stop,
}

/// How a record's decree loop ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordEnd {
    Finished,
    Stopped,
}

/// The film's side of a playback: the mapper's selections, the film session's
/// players and S. One record of it is the native 0x751A40 and decree loop
/// 0x74D710: every decree in order logs its S entries and takes effect before
/// the next one is even mapped, and only then the world updates (0x752AE0).
#[derive(Clone, Debug)]
pub struct FilmDriver {
    pub mapper: Mapper,
    pub sim: SimSync,
    /// The film players: SessionPlayers (0x76E1D0), and those a 329 created
    /// (0x776870 in film state calls CreateFilmPlayer 0x76CD00 too).
    pub roster: Vec<SessionPlayer>,
}

/// The world about to run a tick's decrees (the head of `World::step_decrees`).
fn begin_tick(world: &mut World) {
    world.events.clear();
    world.trace.tick = world.clock.tick.wrapping_add(1);
}

/// The world update after a tick's decrees. `World::step_decrees` clears the
/// events, so the decrees' own (refusals, say) are put back in front.
pub fn update_world(world: &mut World) {
    let decrees = std::mem::take(&mut world.events);
    world.step_decrees(Vec::new());
    if !decrees.is_empty() {
        world.events.splice(0..0, decrees);
    }
}

impl FilmDriver {
    pub fn new(film: &Film, world: &World) -> Self {
        Self {
            mapper: Mapper::new(),
            sim: SimSync::new(world),
            roster: film.players.clone(),
        }
    }

    /// One record's decrees, in order (0x74D710). The caller then updates the
    /// world ([`update_world`]) and takes S ([`SimSync::feed`]).
    #[allow(clippy::too_many_arguments)]
    pub fn run_record(
        &mut self,
        decrees: &[FilmDecree],
        world: &mut World,
        tables: IdTables<'_>,
        aliases: &Aliases,
        pairing: Option<&Pairing>,
        native_append: bool,
        sink: &mut dyn FnMut(DecreeSignal<'_>, &mut World) -> DecreeFlow,
    ) -> RecordEnd {
        begin_tick(world);
        for d in decrees {
            // The handlers log the class name and the id before Process.
            if let Some(name) = decree_s_entries(d.descriptor as usize) {
                let units: Vec<u16> = name.encode_utf16().collect();
                world.trace.log_decree(u32::from(d.descriptor), &units);
            }
            let flow = match &d.decree {
                // 0x75D710: 0x76B8F0(id, film flag) finds only a film player.
                Decree::SetPlayer { player } if !self.roster.iter().any(|p| p.id == *player) => {
                    sink(DecreeSignal::NoOrderingPlayer { player: *player }, world)
                }
                // 0x75D7F0 -> 0x752F60: every film player has the film flag.
                Decree::SimSyncValue { index, hash } => {
                    self.sim.feed(world);
                    let passed = self.sim.validate(*index, *hash, native_append);
                    sink(
                        DecreeSignal::Checkpoint {
                            index: *index,
                            hash: *hash,
                            passed,
                        },
                        world,
                    )
                }
                Decree::UpdateKingdomName { kingdom, name, .. } => {
                    rename_kingdom(kingdom, name, world, sink)
                }
                Decree::Other { fields } if d.descriptor as usize == id::MIDGAME_CREATE_AIS => {
                    self.create_film_player(fields, world);
                    DecreeFlow::Continue
                }
                _ => DecreeFlow::Continue,
            };
            if flow == DecreeFlow::Stop {
                return RecordEnd::Stopped;
            }
            let selection = self
                .mapper
                .selections
                .get(&d.player.unwrap_or(0))
                .cloned()
                .unwrap_or_default();
            let mapped = {
                let ids = Aliased {
                    ids: PlayIds {
                        world: &*world,
                        players: &self.roster,
                        object_ids: tables.object_ids,
                        property_ids: tables.property_ids,
                        formation_ids: tables.formation_ids,
                        ability_ids: tables.ability_ids,
                        layout_ids: tables.layout_ids,
                    },
                    aliases,
                    pairing,
                };
                self.mapper.map(d, &ids)
            };
            sink(
                DecreeSignal::Mapped {
                    decree: d,
                    selection: &selection,
                    mapped: &mapped,
                },
                world,
            );
            if let Mapped::Commands(commands) = mapped {
                for command in commands {
                    world.apply(command);
                }
            }
        }
        RecordEnd::Finished
    }

    /// 329 in film state (0x75DE50 -> 0x776870): a kingdom found by the name
    /// (0x4F8F00) gets a film player with the decree's id (0x76CD00). Without
    /// the kingdom nothing happens.
    fn create_film_player(&mut self, fields: &[Field], world: &World) {
        let (Some(Field::Text(name)), Some(Field::Uint { value: player, .. })) =
            (fields.first(), fields.get(1))
        else {
            return;
        };
        let name = utf16(name);
        let Some(slot) = world
            .map
            .slots
            .iter()
            .find(|s| s.id.eq_ignore_ascii_case(&name))
        else {
            return;
        };
        self.roster.push(SessionPlayer {
            id: *player,
            name,
            kingdom_index: slot.kingdom_index,
        });
    }
}

/// 328 KKC_UpdateKingdomNameOrderDecree: the name the Scoring screen and the HUD
/// show (`Rommel AI`).
fn rename_kingdom(
    kingdom: &[u16],
    name: &[u16],
    world: &mut World,
    sink: &mut dyn FnMut(DecreeSignal<'_>, &mut World) -> DecreeFlow,
) -> DecreeFlow {
    let id = utf16(kingdom);
    let name = utf16(name);
    let Some(slot) = world
        .map
        .slots
        .iter()
        .position(|s| s.id.eq_ignore_ascii_case(&id))
    else {
        return DecreeFlow::Continue;
    };
    let Some(k) = (0..world.kingdoms.len() as aa_play::KingdomId)
        .find(|&k| world.ids.kingdom_slot(k) == Some(slot))
    else {
        return DecreeFlow::Continue;
    };
    world.kingdoms[k as usize].name.clone_from(&name);
    sink(DecreeSignal::KingdomRenamed { kingdom: k, name }, world)
}

/// [`run_aliased`] calling `observe(step, world)` after every step (the
/// world after GWorld update `step`, as the native sampler's 0x76F850
/// snapshot of the same tick). The hook may change the world
/// (`state_compare::force`). With a [`Pairing`], film thing references go
/// through it, and it is updated before every record and handed to the hook.
#[allow(clippy::too_many_arguments)]
pub fn run_observed(
    film: &Film,
    world: &mut World,
    object_ids: &[String],
    max_steps: Option<u32>,
    stop_at_difference: bool,
    aliases: &Aliases,
    mut pairing: Option<&mut Pairing>,
    observe: &mut dyn FnMut(u32, &mut World, Option<&Pairing>),
) -> ReplayReport {
    let tables = IdTables {
        object_ids,
        property_ids: PROPERTY_IDS.get().map_or(&[], |v| v.as_slice()),
        formation_ids: FORMATION_IDS.get().map_or(&[], |v| v.as_slice()),
        ability_ids: &[],
        layout_ids: &[],
    };
    let mut report = ReplayReport::default();
    let mut driver = FilmDriver::new(film, world);
    let intervals = slog::account(film);
    let prepared = chain::prepare(film);
    let mut previous_tick = 0u32;
    let mut previous_counts = (0i32, 0i32);
    let steps = max_steps.map_or(film.ticks.len() as u32, |m| m.min(film.ticks.len() as u32));
    let mut stopped = false;
    for record in 0..steps {
        if let Some(p) = pairing.as_deref_mut() {
            p.update(world);
        }
        // S as this record's checkpoint holds it: `record` updates and the
        // decrees of the records before.
        let at_start = world.trace.checkpoint();
        let end = driver.run_record(
            film.decrees_at(record),
            world,
            tables,
            aliases,
            pairing.as_deref(),
            false,
            &mut |signal, world| {
                if let DecreeSignal::Mapped {
                    decree: d,
                    selection,
                    mapped,
                } = &signal
                {
                    report_mapped(&mut report, record, d, selection, mapped);
                }
                let DecreeSignal::Checkpoint {
                    index,
                    hash,
                    passed,
                } = signal
                else {
                    return DecreeFlow::Continue;
                };
                // The native check is on entry `index`; the report keeps both
                // sides' (S last index, hash) at the record's start.
                let film_state = (index, hash);
                report.checked += 1;
                let check = CheckpointCheck {
                    tick: record,
                    film: film_state,
                    ours: at_start,
                };
                report.intervals.push(IntervalCheck {
                    check,
                    film_entries: film_state.0 - previous_counts.0,
                    our_entries: at_start.0 - previous_counts.1,
                });
                previous_counts = (film_state.0, at_start.0);
                if passed {
                    report.matched += 1;
                } else if report.first_difference.is_none() {
                    let film_iv = intervals.iter().find(|iv| iv.to_tick == record).cloned();
                    let prepared_iv = prepared.iter().find(|iv| iv.to_tick == record);
                    let ours_kinds = kinds(world, previous_tick, record, prepared_iv);
                    let film_states = match (&film_iv, prepared_iv) {
                        (Some(f), Some(p)) if f.residual.max(0) as u32 == ours_kinds.getters => {
                            let pick = pick_of(world, previous_tick, record, p);
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
                        return DecreeFlow::Stop;
                    }
                }
                previous_tick = record;
                DecreeFlow::Continue
            },
        );
        if end == RecordEnd::Stopped {
            stopped = true;
            break;
        }
        update_world(world);
        driver.sim.feed(world);
        let step = record + 1;
        report.steps = step;
        for e in &world.events {
            if let aa_play::Event::Refused { kingdom, reason } = e {
                report.refused.push((step, *kingdom, reason.clone()));
                for row in report.orders.iter_mut().rev() {
                    if row.record != record {
                        break;
                    }
                    if !row.commands.is_empty() {
                        row.refused.push(reason.clone());
                        break;
                    }
                }
            }
        }
        if let Some(p) = pairing.as_deref_mut() {
            p.update(world);
        }
        observe(step, world, pairing.as_deref());
    }
    // A run that ends early still answers the checkpoint its last update
    // reached, carried by the record it does not run.
    if !stopped && (steps as usize) < film.ticks.len() {
        for c in film.checkpoints.iter().filter(|c| c.tick == steps) {
            driver.sim.feed(world);
            report.checked += 1;
            let ours = world.trace.checkpoint();
            let passed = driver.sim.validate(c.count, c.hash, false);
            report.intervals.push(IntervalCheck {
                check: CheckpointCheck {
                    tick: steps,
                    film: (c.count, c.hash),
                    ours,
                },
                film_entries: c.count - previous_counts.0,
                our_entries: ours.0 - previous_counts.1,
            });
            report.matched += usize::from(passed);
        }
    }
    report.dropped = driver.mapper.dropped.clone();
    report
}

/// The accounting of one mapped decree.
fn report_mapped(
    report: &mut ReplayReport,
    record: u32,
    d: &FilmDecree,
    selection: &[u16],
    mapped: &Mapped,
) {
    *report.outcomes.entry(outcome_key(mapped)).or_default() += 1;
    if let Mapped::Unresolved(why) = mapped {
        report.unresolved.push((record, d.player, why.clone()));
    }
    if let Mapped::Commands(c) = mapped {
        report.commands += c.len();
    }
    if crate::classify::decree_class(&d.decree) == "command" {
        let name = crate::classify::actor_command(&d.decree)
            .map_or_else(|| d.name.to_owned(), |(_, n, _)| n.to_owned());
        report.orders.push(OrderRow {
            record,
            player: d.player,
            name,
            slots: selection.to_vec(),
            outcome: outcome_key(mapped),
            commands: match mapped {
                Mapped::Commands(c) => c.iter().map(|c| format!("{c:?}")).collect(),
                _ => Vec::new(),
            },
            refused: Vec::new(),
        });
    }
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
            SKind::RngGetter(_) => {
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
            SKind::RngGetter(_) => pick.push(passed?),
            _ => {}
        }
    }
    Some(pick)
}
