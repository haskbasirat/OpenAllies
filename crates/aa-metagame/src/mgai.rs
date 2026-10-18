//! The original metagame AI (`KKC_MGAIPlayer`, MGAIPlayer.cpp of AA.exe):
//! map analysis, goals and actor assignment, purchasing, attack decision,
//! movement orders and the PlayTurn driver, ported from the machine code of
//! the 32-bit original (addresses cited per item).
//!
//! Proven by `tests/native_mgai.rs` against `research/phase8-metagame/
//! native-mgai-c*.json` (the original code under Unicorn on perturbed
//! InitCreate worlds of every shipped campaign, x87 PC53 and some PC24/PC64
//! turns). Seams of the engine are explicit inputs ([`MgaiHost`]).
//!
//! Native objects (32-bit offsets):
//! - `KKC_MGAIPlayer` (0x2C, ctor 963430(controller, MG kingdom), vtable
//!   ACE11C): +0x04 controller (yields 944CE0, command queue 944F50), +0x08
//!   MG kingdom, +0x0C analysis, +0x10 goal list [+0x1C Attack (ctor 9685A0,
//!   vtable ACF620, Reset 969430), +0x20 AttackUnoccupied (9686A0, ACF630,
//!   9697D0), +0x24 Defend (968620, ACF628, 969600)], +0x28 attacked this
//!   turn, +0x29 turn ended.
//! - analysis (0x34, ctor 969CB0): +0x04 MG kingdom, +0x08 MGAI map record
//!   (95FDD0: first record of the MGAI data whose map is the campaign map),
//!   +0x0C territory analyses (one per MG territory, state order), +0x18
//!   border list, +0x24 front list, +0x30 proximity computed.
//! - territory analysis (0x28, ctor 96A780): +0x04 territory AI record,
//!   +0x0C MG territory, +0x10 influence per state kingdom (binary32),
//!   +0x1C importance, +0x20 border, +0x21 front, +0x24 proximity.
//! - goal (0x1C): +0x04 analysis, +0x08 items (kept in descending priority
//!   order by bisection 869D30 with comparator 9682A0), +0x14 threshold.
//! - goal item (0x18): +0x04 territory analysis, +0x08 assigned army actor
//!   handles, +0x14 priority (binary32). Attack items (ctor 969180, vtable
//!   ACF758, priority 969130) serve Attack and AttackUnoccupied goals;
//!   Defend items (ctor 9690C0, vtable ACF750, priority 968930 -> 968720).
//!
//! Seams (all explicit in [`MgaiHost`]): the kingdom actor army list
//! (`+0x284`, engine data, native order), handle resolution, the yield
//! 944CE0 (SAI fiber scheduler 79DFF0; the engine may change the state
//! there), the command queue 944F50 (issued commands are this module's
//! output; they are processed by the controller, not here), the world tick
//! `[B36534]+0xDC`, the army "busy" virtual (`+0x70` vtable `+0x64`), the
//! app/SAI RNG `[[B36528]+0x2C]` and the contents of the uninitialised
//! 96A5D0 block. The timer 59ABA0 and the debug log 404F60/952310 have no
//! effect and are omitted. The controller (944D60, 944F90 one player per MG
//! kingdom, 945090 fiber body driving PlayTurn, queue 944F50 / flush 944E60,
//! yield 944CE0) and the order execution 9505F0 are in [`controller`].
//!
//! Unsupported (explicit errors, native would crash, hang or use invalid
//! data): MG kingdoms without kingdom actor, kingdom without MGAI data or
//! without a record for the campaign map, territory records without
//! territory, empty army paths read as `path[0]`, armies without territory,
//! stale handles where native dereferences without a null test (968940),
//! `no_attack_distance == 1` (968C50 divides by zero), shops whose lists
//! are empty or have no positive cost (963B70 divides by zero / loops
//! forever), NULL shop rows, non-finite x87 results.
//!
//! Numerics: every x87 operation rounds to the explicit precision control
//! (`crate::GAME_PRECISION`, PC53, is the metagame thread's); binary32 stores
//! are reproduced where the native code stores. Comparisons follow the
//! native flag tests (`fcom`/`fucom` with `test ah, ...`): no NaN can arise
//! from the finite inputs used (a NaN or infinity would be an unsupported
//! error of the arithmetic layer, never a silent value).
pub mod controller;

use crate::battle::{
    BattleArmy, BattleContext, BattleKingdom, BattleShop, KingdomActor, KingdomIndex,
    KingdomRelations, TerritoryBattle,
};
use crate::commands::{Command, CommandData, CommandWorld, Validator};
use crate::data::{AiDefinition, MapAi, MapDefinition, MetaGameData, TerritoryAi};
use crate::path::{PathGraph, PathSearch, SEARCH_FOUND, UNLIMITED};
use crate::rng::MetaGameRng;
use crate::state::{ArmyId, MgState};
use crate::{Error, Result};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::VecDeque;

const ZERO_F32: u32 = 0x0000_0000; // 9F32DC
const ONE_F32: u32 = 0x3f80_0000; // 9F32E0
const HALF_F32: u32 = 0x3f00_0000; // 962F92 immediate, 9FBDB8
const TWO_POW_32_F64: u64 = 0x41f0_0000_0000_0000; // 9FDAA0
const INV_U32_MAX_F64: u64 = 0x3df0_0000_0010_0000; // 9FDA98
/// 963030: an order is waited for until more than this many ticks passed.
pub const MOVE_WAIT_TICKS: i32 = 0x1e0;
/// 963FD0 / 962A40 wait at least this many ticks after a resolved battle.
pub const BATTLE_SETTLE_TICKS: i32 = 3;

fn f32b(bits: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}
fn is_zero_f32(bits: u32) -> bool {
    bits & 0x7fff_ffff == 0
}
fn greater(a: &Binary, b: &Binary) -> bool {
    a.compare(b) == Ordering::Greater
}
/// Built-in command (9511E0 with the index; data = registry entry).
fn command(index: u32) -> Command {
    Command { data: Some(CommandData::builtin(index)), ..Command::default() }
}

/// `pow(float, int)` as inlined at 96A3DF/96A439/968B6A and in 4A6280:
/// square-and-multiply on the x87 stack (`1.0` and the binary32 base), every
/// product rounded to `precision`, `1/result` for a negative exponent.
pub fn int_pow(base_bits: u32, exponent: i32, precision: Precision) -> Result<Binary> {
    let mut result = f32b(ONE_F32)?;
    let mut base = f32b(base_bits)?;
    let mut m = exponent.unsigned_abs();
    loop {
        if m & 1 != 0 {
            result = result.mul(&base, precision)?;
        }
        m >>= 1;
        if m == 0 {
            break;
        }
        base = base.mul(&base, precision)?;
    }
    if exponent < 0 {
        result = f32b(ONE_F32)?.div(&result, precision)?;
    }
    Ok(result)
}

// ------------------------------------------------------------------ host seams

/// One entry of a kingdom actor's army list (actor `+0x284`, nodes {actor,
/// next}). Engine data: the order is the native list order. `deleted` is the
/// actor flag `+0x08` bit 0 (read by 9635A0 only). Entries without a
/// MetaGame component (`+0xCC` null) are skipped by every native reader and
/// are not represented.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KingdomArmy {
    pub army: ArmyId,
    pub deleted: bool,
}

/// A stored army actor handle (`actor + 0x14`, resolved through the B364C0
/// table): the army and the handle serial it had when stored. It resolves
/// only while the army lives and keeps that serial: a killed army or an
/// army whose actor was replaced (respawn 955DD0 on a type change) gives a
/// null actor, as the native handle does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArmyHandle {
    pub army: ArmyId,
    pub serial: u32,
}

/// Observation points of PlayTurn 964D30 (after each stage returns).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stage {
    /// After DoMapAnalysis 962B70.
    Analysis,
    /// After the first SetGoalPriorities 9635A0.
    Goals1,
    /// After DoPurchasing 963B70.
    Purchase,
    /// After the second SetGoalPriorities.
    Goals2,
    /// After PossiblyAttack 963FD0.
    Attack,
    /// After 9649A0 (SetGoalPriorities when MGState+0x46 == 1).
    Goals3,
    /// After Move 9649C0.
    Move,
    /// After EndTurn 962A40.
    End,
}

/// Engine side of the MGAI. Every method stands for an explicit native seam.
pub trait MgaiHost {
    /// Kingdom actor army list (`actor + 0x284`), native order.
    fn kingdom_armies(&self, state: &MgState, actor: KingdomActor) -> Result<Vec<KingdomArmy>>;
    /// Current handle serial of a live army's actor (changes when the
    /// engine replaces the actor). Default 0.
    fn army_serial(&self, _state: &MgState, _army: ArmyId) -> u32 {
        0
    }
    /// Handle resolution (`B364C0` table): the army lives and its actor
    /// still has the stored serial.
    fn army_live(&self, state: &MgState, handle: ArmyHandle) -> bool {
        state.armies.get(handle.army.0 as usize).is_some_and(Option::is_some)
            && self.army_serial(state, handle.army) == handle.serial
    }
    /// `FUN_00944CE0(arg)` on the controller: yield to the engine (SAI fiber
    /// scheduler); false aborts the turn. The engine may change the state.
    fn yield_turn(&mut self, state: &mut MgState, arg: u8) -> Result<bool>;
    /// `FUN_00944F50` on the controller: queue an issued command.
    fn issue(&mut self, state: &MgState, command: Command) -> Result<()>;
    /// World tick `[B36534] + 0xDC`.
    fn tick(&self) -> i32;
    /// Army actor `+0x70` object, virtual `+0x64` (still busy moving).
    fn army_busy(&mut self, state: &MgState, army: ArmyId) -> Result<bool>;
    /// Native bug in 96A5D0: the number of nonzero bytes of an
    /// uninitialised `len`-byte heap block (operator new 6F7810) that is
    /// never written. Pure allocator state: an explicit input.
    fn uninitialized_nonzero(&mut self, len: usize) -> Result<usize>;
    /// The app/SAI RNG `[[B36528] + 0x2C]` (85D840 draws), not `MGState+0x18`.
    fn app_rng(&mut self) -> &mut MetaGameRng;
    /// Observation hook after each PlayTurn stage (no native effect).
    fn stage(&mut self, _stage: Stage, _player: &MgaiPlayer, _state: &MgState) {}
}

// ------------------------------------------------------------------ objects

/// The territory AI record a territory analysis uses (96A780).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecordRef {
    /// Map record `+0x14`.
    Default,
    /// `+0x18`: the AI kingdom's capital.
    Capital,
    /// `+0x1C`: capital of a kingdom the AI kingdom passes vtable+0x138 with.
    FriendlyCapital,
    /// `+0x20`: capital of a kingdom the AI kingdom regards as an enemy.
    EnemyCapital,
    /// Index into [`MapAi::territories`].
    Territory(usize),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerritoryAnalysis {
    /// `+0x04`.
    pub record: RecordRef,
    /// `+0x10`: binary32 bits per state kingdom (count = kingdoms at reset).
    pub influence: Vec<u32>,
    /// `+0x1C`.
    pub importance: i32,
    /// `+0x20`: friendly territory next to a hostile one.
    pub border: bool,
    /// `+0x21`: hostile territory next to one the AI kingdom owns.
    pub front: bool,
    /// `+0x24`: accumulated once (96A230).
    pub proximity: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Analysis {
    /// `+0x08`: index into [`AiDefinition::maps`].
    pub map_record: usize,
    /// `+0x0C`: one per MG territory (index = territory index).
    pub territories: Vec<TerritoryAnalysis>,
    /// `+0x18` (territory indices, ascending).
    pub borders: Vec<usize>,
    /// `+0x24`.
    pub fronts: Vec<usize>,
    /// `+0x30`.
    pub proximity_done: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GoalKind {
    Attack,
    AttackUnoccupied,
    Defend,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemKind {
    /// Vtable ACF758, priority 969130.
    Attack,
    /// Vtable ACF750, priority 968930 (-> 968720).
    Defend,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalItem {
    pub kind: ItemKind,
    /// `+0x04` territory analysis (= MG territory index).
    pub territory: usize,
    /// `+0x08` assigned army handles, append order.
    pub armies: Vec<ArmyHandle>,
    /// `+0x14` binary32 bits.
    pub priority_bits: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Goal {
    pub kind: GoalKind,
    /// `+0x08`, descending priority (bisection insertion).
    pub items: Vec<GoalItem>,
    /// `+0x14` binary32 bits (set by 9632E0).
    pub threshold_bits: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MgaiPlayer {
    /// `+0x08`: MG kingdom (state index).
    pub kingdom: usize,
    pub analysis: Analysis,
    /// `+0x1C`, `+0x20`, `+0x24` (also the `+0x10` list order).
    pub goals: [Goal; 3],
    /// `+0x28`.
    pub attacked: bool,
    /// `+0x29`.
    pub turn_ended: bool,
}

/// One order of the movement list (0x10 bytes: player, army handle, MG
/// territory waited for, command).
#[derive(Clone, Debug)]
struct MoveOrder {
    army: ArmyHandle,
    territory: usize,
    command: Command,
}

struct NoWorld;
impl CommandWorld for NoWorld {
    fn controllable(&self, _: ArmyId, _: KingdomActor) -> bool {
        unreachable!("93A320/954FD0 do not query command world seams")
    }
    fn reach(&self, _: &MgState, _: ArmyId, _: usize) -> Result<bool> {
        Err(Error::Unsupported("93A320/954FD0 do not query the path seam".into()))
    }
}

// ------------------------------------------------------------------ read-only view

/// Read-only view of one state for the MGAI computations of one kingdom.
pub struct View<'a, R: KingdomRelations> {
    pub state: &'a MgState,
    pub data: &'a MetaGameData,
    pub relations: &'a R,
    pub precision: Precision,
    pub ai: &'a AiDefinition,
    map: &'a MapDefinition,
    graph: PathGraph<'a>,
    battle_kingdoms: Vec<BattleKingdom>,
    damage: Vec<u32>,
    us: usize,
    us_actor: Option<KingdomActor>,
}

impl<'a, R: KingdomRelations> View<'a, R> {
    pub fn new(state: &'a MgState, data: &'a MetaGameData, relations: &'a R, precision: Precision, kingdom: usize) -> Result<Self> {
        let campaign = data
            .campaigns
            .get(state.campaign)
            .ok_or_else(|| Error::Unsupported(format!("campaign {}", state.campaign)))?;
        let map = campaign
            .map
            .and_then(|m| data.maps.get(m))
            .ok_or_else(|| Error::Unsupported("campaign without map".into()))?;
        let k = state
            .kingdoms
            .get(kingdom)
            .ok_or_else(|| Error::Unsupported(format!("MG kingdom {kingdom}")))?;
        let ai = campaign.kingdoms[k.data]
            .mgai
            .and_then(|i| data.ai.get(i))
            .ok_or_else(|| Error::Unsupported("kingdom without MGAI data (native null +0x14)".into()))?;
        let battle_kingdoms = state
            .kingdoms
            .iter()
            .map(|k| {
                Ok(BattleKingdom {
                    actor: k.actor.ok_or_else(|| {
                        Error::Unsupported("MG kingdom without kingdom actor (native null virtual call)".into())
                    })?,
                    owned_technologies: Vec::new(),
                    shop: BattleShop::default(),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            state,
            data,
            relations,
            precision,
            ai,
            map,
            graph: PathGraph::new(state, data)?,
            battle_kingdoms,
            damage: data.divisions.iter().map(|d| d.damage_bits).collect(),
            us: kingdom,
            us_actor: k.actor,
        })
    }

    fn battle(&self) -> BattleContext<'_, R> {
        BattleContext {
            kingdoms: &self.battle_kingdoms,
            division_damage_bits: &self.damage,
            relations: self.relations,
            technology_bonus_bits: 0,
            attack_win_probability_bits: 0,
            precision: self.precision,
        }
    }
    fn p(&self) -> Precision {
        self.precision
    }
    fn actor_of(&self, k: usize) -> Result<KingdomActor> {
        self.state.kingdoms[k]
            .actor
            .ok_or_else(|| Error::Unsupported("MG kingdom without kingdom actor (native null virtual call)".into()))
    }
    fn us_actor(&self) -> Result<KingdomActor> {
        self.us_actor
            .ok_or_else(|| Error::Unsupported("AI kingdom without kingdom actor (native null virtual call)".into()))
    }
    fn def_of(&self, t: usize) -> usize {
        self.state.territories[t].definition
    }
    /// MG territories of the definition links of `t` (957960 + 939B60), link order.
    fn neighbors(&self, t: usize) -> Result<Vec<usize>> {
        self.map.territories[self.def_of(t)]
            .links
            .iter()
            .map(|l| self.graph.territory_of_definition(l.target))
            .collect()
    }
    /// `FUN_00957910` on `from`'s definition with `to`'s definition: a link exists.
    fn link(&self, from: usize, to: usize) -> bool {
        let target = self.def_of(to);
        self.map.territories[self.def_of(from)].links.iter().any(|l| l.target == target)
    }
    /// `FUN_00954B70(K)`: K null or territory unowned -> true, else the
    /// owner's kingdom actor regards K's actor as an enemy (vtable+0x140).
    fn hostile_mg(&self, t: usize, k: Option<usize>) -> Result<bool> {
        let Some(k) = k else { return Ok(true) };
        let Some(owner) = self.state.territories[t].owner else { return Ok(true) };
        Ok(self.relations.enemy(self.actor_of(owner)?, self.actor_of(k)?))
    }
    /// `FUN_00954BE0(K)` = !954B70.
    fn friendly_mg(&self, t: usize, k: Option<usize>) -> Result<bool> {
        Ok(!self.hostile_mg(t, k)?)
    }
    /// `FUN_00954B30(actor)`.
    fn hostile_actor(&self, t: usize, actor: KingdomActor) -> Result<bool> {
        match self.state.territories[t].owner {
            None => Ok(true),
            Some(o) => Ok(self.relations.enemy(self.actor_of(o)?, actor)),
        }
    }
    /// `FUN_00954C00(actor)`.
    fn friendly_actor(&self, t: usize, actor: KingdomActor) -> Result<bool> {
        match self.state.territories[t].owner {
            None => Ok(false),
            Some(o) => Ok(!self.relations.enemy(self.actor_of(o)?, actor)),
        }
    }
    fn battle_territory(&self, t: usize) -> TerritoryBattle {
        let tt = &self.state.territories[t];
        TerritoryBattle {
            armies: tt.armies.iter().map(|r| self.battle_army(r.army)).collect(),
            enemy_army_count: tt.enemy_army_count,
            owner: tt.owner.map(KingdomIndex),
        }
    }
    fn battle_army(&self, army: ArmyId) -> BattleArmy {
        let a = self.state.army(army);
        BattleArmy {
            owner: a.owner,
            damage_bits: self.data.armies[a.definition].damage_bits,
            divisions: a.divisions.clone(),
        }
    }
    /// `FUN_00954FD0` (binary32 bits).
    pub fn defender_strength(&self, t: usize) -> Result<u32> {
        self.battle().defender_strength(&self.battle_territory(t))
    }
    /// `FUN_0093EBD0` (unrounded x87 value).
    pub fn strength(&self, army: ArmyId) -> Result<Binary> {
        self.battle().army_strength(&self.battle_army(army))
    }
    /// `FUN_0093F030`: remaining movement (93FCC0 budget rules).
    pub fn remaining(&self, army: ArmyId) -> Result<i32> {
        self.graph.army_budget(self.data, self.relations, army)
    }
    /// Army type movement (93EB40 / 93EB30 +0x04).
    fn movement(&self, army: ArmyId) -> i32 {
        self.data.armies[self.state.army(army).definition].movement
    }
    fn territory_of_army(&self, army: ArmyId) -> Result<usize> {
        self.state
            .army(army)
            .territory
            .ok_or_else(|| Error::Unsupported("army without territory (native null dereference)".into()))
    }
    fn path_start(&self, army: ArmyId) -> Result<usize> {
        self.state
            .army(army)
            .path
            .first()
            .copied()
            .ok_or_else(|| Error::Unsupported("army path +0x24 empty (native reads element 0)".into()))
    }
    /// 961550(start, goal, AI kingdom actor (handle resolve, may be null), budget) + 961350.
    fn search(&self, start: usize, goal: usize, budget: i32) -> Result<PathSearch> {
        self.graph.search(self.relations, start, goal, self.us_actor, budget, self.precision)
    }
    /// `FUN_00938AC0(K)`: owned territories of the kingdoms K regards as enemies.
    fn enemy_territory_count(&self, k: usize) -> Result<i32> {
        let me = self.actor_of(k)?;
        let mut sum = 0i32;
        for (i, x) in self.state.kingdoms.iter().enumerate() {
            if self.relations.enemy(me, self.actor_of(i)?) {
                sum = sum.wrapping_add(x.territories.len() as i32);
            }
        }
        Ok(sum)
    }
    /// `FUN_00955090`: binary32 sum of the strengths of the armies in `t`
    /// the owner does not regard as enemies and whose type can move
    /// (zero for an unowned territory).
    fn mobile_defender_strength(&self, t: usize) -> Result<u32> {
        let mut sum = ZERO_F32;
        let Some(owner) = self.state.territories[t].owner else { return Ok(sum) };
        let owner = self.actor_of(owner)?;
        for r in &self.state.territories[t].armies {
            let a = self.state.army(r.army);
            if !self.relations.enemy(owner, a.owner) && self.movement(r.army) > 0 {
                sum = self.strength(r.army)?.add(&f32b(sum)?, self.p())?.store_f32()?;
            }
        }
        Ok(sum)
    }
    /// `FUN_00955650(exclude)` on `t`: some other neighbour hostile to t's
    /// owner holds a positive 955090 strength.
    fn threatened(&self, t: usize, exclude: usize) -> Result<bool> {
        let owner = self.state.territories[t].owner;
        for n in self.neighbors(t)? {
            if self.hostile_mg(n, owner)? && n != exclude && greater(&f32b(self.mobile_defender_strength(n)?)?, &f32b(ZERO_F32)?) {
                return Ok(true);
            }
        }
        Ok(false)
    }
    /// `FUN_0093A320` is some.
    fn pending_battle(&self) -> Result<bool> {
        let v = Validator { state: self.state, data: self.data, relations: self.relations, world: &NoWorld, precision: self.precision };
        Ok(v.pending_battle()?.is_some())
    }
    fn shop(&self) -> &'a crate::data::Shop {
        let k = &self.state.kingdoms[self.us];
        &self.data.campaigns[self.state.campaign].kingdoms[k.data].shop
    }
    fn division_index(&self, ids: &str) -> Result<usize> {
        self.data
            .divisions
            .iter()
            .position(|d| d.ids == ids)
            .ok_or_else(|| Error::Unsupported(format!("shop division {ids} not loaded")))
    }
    /// `FUN_00962DB0`: some army of the kingdom list lacks a shop division (93E9E0).
    pub fn upgrade_possible<H: MgaiHost>(&self, host: &H) -> Result<bool> {
        for e in host.kingdom_armies(self.state, self.us_actor()?)? {
            let have = &self.state.army(e.army).divisions;
            for row in &self.shop().divisions {
                let present = match &row.ids {
                    None => false,
                    Some(ids) => have.contains(&crate::battle::DivisionRef(self.division_index(ids)? as u32)),
                };
                if !present {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}

// ------------------------------------------------------------------ analysis

impl Analysis {
    /// `FUN_00969CB0` (with 96A780 per territory and 96AA90).
    pub fn new<R: KingdomRelations>(view: &View<'_, R>) -> Result<Self> {
        let campaign_map = view.data.campaigns[view.state.campaign].map;
        let map_record = view
            .ai
            .maps
            .iter()
            .position(|m| Some(m.map) == campaign_map)
            .ok_or_else(|| Error::Unsupported("95FDD0: no MGAI map record for the campaign map (native null)".into()))?;
        let rec = &view.ai.maps[map_record];
        let n = view.state.kingdoms.len();
        let mut territories = Vec::with_capacity(view.state.territories.len());
        for t in 0..view.state.territories.len() {
            // 95FF10: first record whose definition IDS equals (wcscmp) ours.
            let ids = &view.map.territories[view.def_of(t)].ids;
            let mut record = RecordRef::Default;
            for (i, r) in rec.territories.iter().enumerate() {
                let def = r
                    .territory
                    .ok_or_else(|| Error::Unsupported("territory AI record without territory (native null)".into()))?;
                if &view.map.territories[def].ids == ids {
                    record = RecordRef::Territory(i);
                    break;
                }
            }
            // later capital matches win
            for (k, kd) in view.state.kingdoms.iter().enumerate() {
                if kd.capital != Some(t) {
                    continue;
                }
                if k == view.us {
                    record = RecordRef::Capital;
                } else {
                    let me = view.us_actor()?;
                    let other = view.actor_of(k)?;
                    if view.relations.relation_138(me, other) {
                        record = RecordRef::FriendlyCapital;
                    } else if view.relations.enemy(me, other) {
                        record = RecordRef::EnemyCapital;
                    }
                }
            }
            territories.push(TerritoryAnalysis {
                record,
                influence: vec![ZERO_F32; n],
                importance: 0,
                border: false,
                front: false,
                proximity: 0,
            });
        }
        Ok(Self { map_record, territories, borders: Vec::new(), fronts: Vec::new(), proximity_done: false })
    }

    /// `FUN_009699E0`: 96AA90 on every territory analysis.
    pub fn reset(&mut self, kingdoms: usize) {
        for ta in &mut self.territories {
            ta.border = false;
            ta.front = false;
            ta.importance = 0;
            ta.influence = vec![ZERO_F32; kingdoms];
        }
    }

    pub fn record<'b>(&self, ai: &'b AiDefinition, record: RecordRef) -> &'b TerritoryAi {
        let m: &MapAi = &ai.maps[self.map_record];
        match record {
            RecordRef::Default => &m.default_territory,
            RecordRef::Capital => &m.capital,
            RecordRef::FriendlyCapital => &m.friendly_capital,
            RecordRef::EnemyCapital => &m.enemy_capital,
            RecordRef::Territory(i) => &m.territories[i],
        }
    }

    /// Breadth-first walk of 96A1A0/96A090 from `start`: the head is
    /// reported while `continue_at(depth)`; advancing marks the head
    /// visited (by territory index), appends every link neighbour with
    /// depth + 1 (visited or not), drops the head and then every visited
    /// head.
    fn bfs<R: KingdomRelations>(
        view: &View<'_, R>,
        start: usize,
        mut continue_at: impl FnMut(i32) -> bool,
        mut visit: impl FnMut(usize, i32) -> Result<()>,
    ) -> Result<()> {
        let mut visited = vec![false; view.state.territories.len() + 1];
        let mut queue = VecDeque::from([(start, 0i32)]);
        while let Some(&(t, d)) = queue.front() {
            if !continue_at(d) {
                break;
            }
            visit(t, d)?;
            visited[t] = true;
            for n in view.neighbors(t)? {
                queue.push_back((n, d.wrapping_add(1)));
            }
            queue.pop_front();
            while let Some(&(h, _)) = queue.front() {
                if visited[h] {
                    queue.pop_front();
                } else {
                    break;
                }
            }
        }
        Ok(())
    }

    /// `FUN_0096A230` (once): every territory adds, to each territory within
    /// its record's `additional_neighbor_proximity_importance_depth` (BFS
    /// depth d, itself d = 0), `trunc(max / (d + 1))` (`fild`, `fdivp` at
    /// PC, `_ftol2` 96CC4C truncation).
    pub fn proximity<R: KingdomRelations>(&mut self, view: &View<'_, R>) -> Result<()> {
        if self.proximity_done {
            return Ok(());
        }
        for i in 0..self.territories.len() {
            let rec = self.record(view.ai, self.territories[i].record);
            let max = rec.additional_neighbor_proximity_importance_max;
            let depth = rec.additional_neighbor_proximity_importance_depth;
            if depth <= 0 {
                continue;
            }
            let mut adds = Vec::new();
            Self::bfs(view, i, |d| d <= depth, |t, d| {
                let q = Binary::from_i32(max).div(&Binary::from_i32(d.wrapping_add(1)), view.p())?;
                adds.push((t, q.truncate_i32()?));
                Ok(())
            })?;
            for (t, v) in adds {
                self.territories[t].proximity = self.territories[t].proximity.wrapping_add(v);
            }
        }
        self.proximity_done = true;
        Ok(())
    }

    /// `FUN_0096A4C0`: for every state kingdom, every army of its kingdom
    /// actor list adds influence (96A330) around the first territory of the
    /// army path.
    pub fn influence<R: KingdomRelations, H: MgaiHost>(&mut self, view: &View<'_, R>, host: &H) -> Result<()> {
        let mult = view.ai.actor_distance_influence_multiplier_bits;
        for k in 0..view.state.kingdoms.len() {
            let actor = view.actor_of(k)?;
            for e in host.kingdom_armies(view.state, actor)? {
                // 96A330
                let s = view.strength(e.army)?.store_f32()?;
                let limit = view.movement(e.army).wrapping_mul(view.ai.max_influence_depth);
                let start = view.path_start(e.army)?;
                let mut adds = Vec::new();
                Self::bfs(view, start, |d| d <= limit, |t, d| {
                    let mut v = int_pow(mult, d, view.p())?.mul(&f32b(s)?, view.p())?.store_f32()?;
                    if k != view.us && d != 0 {
                        v = int_pow(mult, d - 1, view.p())?.mul(&f32b(s)?, view.p())?.store_f32()?;
                    }
                    adds.push((t, v));
                    Ok(())
                })?;
                for (t, v) in adds {
                    // 96A5A0: kingdom index 9389C0, binary32 accumulate
                    let slot = self.territories[t]
                        .influence
                        .get_mut(k)
                        .ok_or_else(|| Error::Unsupported("influence slot beyond the reset count".into()))?;
                    *slot = f32b(v)?.add(&f32b(*slot)?, view.p())?.store_f32()?;
                }
            }
        }
        Ok(())
    }

    /// `FUN_00969E60`: border (friendly next to hostile) and front (hostile
    /// next to owned) flags, then the two lists in territory order.
    pub fn borders<R: KingdomRelations>(&mut self, view: &View<'_, R>) -> Result<()> {
        let us = Some(view.us);
        let n = self.territories.len();
        for i in 0..n {
            if view.hostile_mg(i, us)? {
                for t in view.neighbors(i)? {
                    if view.friendly_mg(t, us)? {
                        self.territories[t].border = true;
                    }
                }
            }
        }
        for i in 0..n {
            if view.state.territories[i].owner == us {
                for t in view.neighbors(i)? {
                    if view.hostile_mg(t, us)? {
                        self.territories[t].front = true;
                    }
                }
            }
        }
        self.borders = (0..n).filter(|&i| self.territories[i].border).collect();
        self.fronts = (0..n).filter(|&i| self.territories[i].front).collect();
        Ok(())
    }

    /// `FUN_009699B0` -> `FUN_0096A980` per territory: 96A5D0 +
    /// production_importance_multiplier * production + proximity.
    pub fn importance<R: KingdomRelations, H: MgaiHost>(&mut self, view: &View<'_, R>, host: &mut H) -> Result<()> {
        for i in 0..self.territories.len() {
            let base = self.importance_base(view, host, i)?;
            let production = view.map.territories[view.def_of(i)].production;
            self.territories[i].importance = base
                .wrapping_add(view.ai.production_importance_multiplier.wrapping_mul(production))
                .wrapping_add(self.territories[i].proximity);
        }
        Ok(())
    }

    /// `FUN_0096A5D0`.
    fn importance_base<R: KingdomRelations, H: MgaiHost>(&mut self, view: &View<'_, R>, host: &mut H, i: usize) -> Result<i32> {
        let ai = view.ai;
        let us = Some(view.us);
        let mut v = self.record(ai, self.territories[i].record).additional_importance;
        let friendly = view.friendly_mg(i, us)?;
        if friendly {
            v = v.wrapping_add(ai.additional_defense_importance);
        } else if is_zero_f32(view.defender_strength(i)?) {
            v = v.wrapping_add(ai.additional_attack_unoccupied_importance);
        } else {
            v = v.wrapping_add(ai.additional_attack_occupied_importance);
        }
        if view.friendly_mg(i, us)? && self.territories[i].border {
            v = v.wrapping_add(ai.additional_border_importance);
        }
        if view.hostile_mg(i, us)? {
            let borders = self.borders.len() as i32;
            let n = self.territories.len();
            // The loop marks territory analyses, never the counted block.
            for j in 0..n {
                if view.hostile_mg(j, us)? && j != i {
                    for t in view.neighbors(j)? {
                        if view.friendly_mg(t, us)? {
                            self.territories[t].border = true;
                        }
                    }
                }
            }
            let count = host.uninitialized_nonzero(n)?;
            let count = i32::try_from(count).map_err(|_| Error::Unsupported("nonzero count beyond i32".into()))?;
            if count <= borders {
                v = v.wrapping_add((borders - count + 1).wrapping_mul(ai.additional_border_reduction_importance));
            }
        }
        Ok(v)
    }

    /// `FUN_0096A9C0`: largest influence (binary32, from 0) of the kingdoms
    /// the AI kingdom regards as enemies.
    fn threat<R: KingdomRelations>(&self, view: &View<'_, R>, t: usize) -> Result<u32> {
        let me = view.us_actor()?;
        let mut best = ZERO_F32;
        for (k, &v) in self.territories[t].influence.iter().enumerate() {
            let other = view.actor_of(k)?;
            if view.relations.enemy(me, other) && greater(&f32b(v)?, &f32b(best)?) {
                best = v;
            }
        }
        Ok(best)
    }
}

// ------------------------------------------------------------------ goal items

impl GoalItem {
    /// `FUN_00968720`: defence need of the item territory given `armies`.
    fn defense_value<R: KingdomRelations, H: MgaiHost>(an: &Analysis, view: &View<'_, R>, host: &H, t: usize, armies: &[ArmyHandle]) -> Result<Binary> {
        let p = view.p();
        let threat = an.threat(view, t)?;
        if is_zero_f32(threat) {
            return f32b(ZERO_F32);
        }
        let mult = view.ai.actor_distance_influence_multiplier_bits;
        let mut d = ZERO_F32;
        for &h in armies {
            if !host.army_live(view.state, h) {
                continue;
            }
            let army = h.army;
            let s = view.strength(army)?.store_f32()?;
            if view.remaining(army)? == 0 {
                if view.state.army(army).territory == Some(t) {
                    d = f32b(s)?.add(&f32b(d)?, p)?.store_f32()?;
                }
            } else {
                let r = view.search(view.path_start(army)?, t, UNLIMITED)?;
                if r.code == SEARCH_FOUND {
                    d = int_pow(mult, r.path.len() as i32, p)?
                        .mul(&f32b(s)?, p)?
                        .add(&f32b(d)?, p)?
                        .store_f32()?;
                }
            }
        }
        let desired = an.record(view.ai, an.territories[t].record).desired_defense_threat_ratio_bits;
        let ratio = if is_zero_f32(d) {
            ONE_F32
        } else {
            f32b(threat)?.div(&f32b(d)?.add(&f32b(threat)?, p)?, p)?.store_f32()?
        };
        let q = f32b(ratio)?
            .sub(&f32b(desired)?, p)?
            .div(&f32b(ONE_F32)?.sub(&f32b(desired)?, p)?, p)?;
        let v = Binary::from_i32(an.territories[t].importance).mul(&q, p)?;
        if v.compare(&f32b(ZERO_F32)?) == Ordering::Less {
            return f32b(ZERO_F32);
        }
        Ok(v)
    }

    /// `FUN_00968A50`: share of the armies' distance-weighted strength.
    fn strength_share<R: KingdomRelations, H: MgaiHost>(view: &View<'_, R>, host: &H, t: usize, armies: &[ArmyHandle]) -> Result<Binary> {
        let p = view.p();
        let defenders = view.defender_strength(t)?;
        let mult = view.ai.actor_distance_strength_multiplier_bits;
        let mut sum = ZERO_F32;
        for &h in armies {
            if !host.army_live(view.state, h) {
                continue;
            }
            let army = h.army;
            let r = view.search(view.path_start(army)?, t, UNLIMITED)?;
            if r.code == SEARCH_FOUND {
                let s = view.strength(army)?;
                sum = int_pow(mult, r.path.len() as i32, p)?.mul(&s, p)?.add(&f32b(sum)?, p)?.store_f32()?;
            }
        }
        if is_zero_f32(sum) {
            return f32b(ZERO_F32);
        }
        if is_zero_f32(defenders) {
            return f32b(ONE_F32);
        }
        Ok(f32b(sum)?.div(&f32b(sum)?.add(&f32b(defenders)?, p)?, p)?)
    }

    /// `FUN_00968C50`: attack value of a hostile territory.
    fn attack_value<R: KingdomRelations, H: MgaiHost>(an: &Analysis, view: &View<'_, R>, host: &H, t: usize, armies: &[ArmyHandle]) -> Result<Binary> {
        let p = view.p();
        if !view.hostile_mg(t, Some(view.us))? {
            return f32b(ZERO_F32);
        }
        let importance = an.territories[t].importance;
        if armies.is_empty() {
            return Ok(Binary::from_i32(importance));
        }
        if !is_zero_f32(view.defender_strength(t)?) {
            let impf = Binary::from_i32(importance).store_f32()?;
            let c = Self::strength_share(view, host, t, armies)?.store_f32()?;
            return Ok(f32b(impf)?.sub(&Binary::from_i32(importance).mul(&f32b(c)?, p)?, p)?);
        }
        let n = view.ai.no_attack_distance;
        let mut min = -1i32;
        for &h in armies {
            if !host.army_live(view.state, h) {
                continue;
            }
            let army = h.army;
            let r = view.search(view.path_start(army)?, t, n)?;
            if r.code == SEARCH_FOUND {
                let len = r.path.len() as i32;
                if len < min || min == -1 {
                    min = len;
                }
            }
        }
        if min != -1 && min < n {
            let den = n.wrapping_sub(1);
            if den == 0 {
                return Err(Error::Unsupported("968C50: no_attack_distance 1 divides by zero".into()));
            }
            let q = Binary::from_i32(min - 1).div(&Binary::from_i32(den), p)?;
            return Binary::from_i32(importance).mul(&q, p).map_err(Into::into);
        }
        Ok(Binary::from_i32(importance))
    }

    /// Item virtual `+0x04` (969130 / 968930), unrounded.
    fn priority_of<R: KingdomRelations, H: MgaiHost>(kind: ItemKind, an: &Analysis, view: &View<'_, R>, host: &H, t: usize, armies: &[ArmyHandle]) -> Result<Binary> {
        match kind {
            ItemKind::Defend => Self::defense_value(an, view, host, t, armies),
            ItemKind::Attack => {
                let p = view.p();
                let a = Self::defense_value(an, view, host, t, armies)?.store_f32()?;
                let f = view.ai.attack_defendability_factor_bits;
                let b = Self::attack_value(an, view, host, t, armies)?;
                let left = b.mul(&f32b(ONE_F32)?.sub(&f32b(f)?, p)?, p)?;
                let right = f32b(f)?.mul(&f32b(a)?, p)?;
                Ok(left.add(&right, p)?)
            }
        }
    }

    /// Item constructors 969180 / 9690C0: no armies, priority stored binary32.
    fn new<R: KingdomRelations, H: MgaiHost>(kind: ItemKind, an: &Analysis, view: &View<'_, R>, host: &H, t: usize) -> Result<Self> {
        let priority_bits = Self::priority_of(kind, an, view, host, t, &[])?.store_f32()?;
        Ok(Self { kind, territory: t, armies: Vec::new(), priority_bits })
    }

    /// `FUN_00969010(actor)`: binary32 `priority - priority(armies + actor)`.
    fn reduction<R: KingdomRelations, H: MgaiHost>(&self, an: &Analysis, view: &View<'_, R>, host: &H, army: ArmyHandle) -> Result<u32> {
        let mut armies = self.armies.clone();
        armies.push(army);
        let with = Self::priority_of(self.kind, an, view, host, self.territory, &armies)?;
        Ok(f32b(self.priority_bits)?.sub(&with, view.p())?.store_f32()?)
    }

    /// `FUN_00968F70(actor, reduction)`.
    fn add_army(&mut self, army: ArmyHandle, reduction: u32, p: Precision) -> Result<()> {
        self.armies.push(army);
        self.priority_bits = f32b(self.priority_bits)?.sub(&f32b(reduction)?, p)?.store_f32()?;
        Ok(())
    }

    /// `FUN_00968940`: strength ratio of `armies` against the defenders.
    fn attack_ratio<R: KingdomRelations, H: MgaiHost>(view: &View<'_, R>, host: &H, t: usize, armies: &[ArmyHandle]) -> Result<Binary> {
        let p = view.p();
        if armies.is_empty() {
            return f32b(ZERO_F32);
        }
        let defenders = view.defender_strength(t)?;
        if is_zero_f32(defenders) {
            return f32b(ONE_F32);
        }
        let mut sum = ZERO_F32;
        for &h in armies {
            if !host.army_live(view.state, h) {
                return Err(Error::Unsupported("968940: stale army handle (native null dereference)".into()));
            }
            sum = view.strength(h.army)?.add(&f32b(sum)?, p)?.store_f32()?;
        }
        if is_zero_f32(sum) {
            return f32b(ZERO_F32);
        }
        Ok(f32b(sum)?.div(&f32b(sum)?.add(&f32b(defenders)?, p)?, p)?)
    }
}

// ------------------------------------------------------------------ goals

impl Goal {
    fn new(kind: GoalKind) -> Self {
        Self { kind, items: Vec::new(), threshold_bits: ZERO_F32 }
    }

    /// Insertion of 969430/969600/9697D0: bisection 869D30 with comparator
    /// 9682A0 (`sign(element - key)` of the binary32 priorities; the x87
    /// difference never rounds a nonzero difference to zero), at the
    /// position of an equal element met by the bisection, else at the
    /// lower bound.
    fn insert(&mut self, item: GoalItem) -> Result<()> {
        let key = f32b(item.priority_bits)?;
        let (mut lo, mut hi) = (0i32, self.items.len() as i32);
        let mut at = None;
        while lo < hi {
            let mid = (lo + hi) / 2;
            match f32b(self.items[mid as usize].priority_bits)?.compare(&key) {
                Ordering::Equal => {
                    at = Some(mid);
                    break;
                }
                Ordering::Less => hi = mid,
                Ordering::Greater => lo = mid + 1,
            }
        }
        let at = at.unwrap_or(lo) as usize;
        self.items.insert(at, item);
        Ok(())
    }

    /// `FUN_009682E0`: largest priority, from 0.0 (strictly greater wins).
    fn max_priority(&self) -> Result<u32> {
        let mut best = ZERO_F32;
        for it in &self.items {
            if greater(&f32b(it.priority_bits)?, &f32b(best)?) {
                best = it.priority_bits;
            }
        }
        Ok(best)
    }

    /// `FUN_00969250`: items whose priority is >= the threshold.
    fn eligible(&self) -> Result<Vec<usize>> {
        let thr = f32b(self.threshold_bits)?;
        let mut out = Vec::new();
        for (i, it) in self.items.iter().enumerate() {
            if f32b(it.priority_bits)?.compare(&thr) != Ordering::Less {
                out.push(i);
            }
        }
        Ok(out)
    }

    /// `FUN_00969380`: best reduction (from 0.0, strictly greater) over eligible items.
    fn best_reduction<R: KingdomRelations, H: MgaiHost>(&self, an: &Analysis, view: &View<'_, R>, host: &H, army: ArmyHandle) -> Result<u32> {
        let mut best = ZERO_F32;
        for i in self.eligible()? {
            let r = self.items[i].reduction(an, view, host, army)?;
            if greater(&f32b(r)?, &f32b(best)?) {
                best = r;
            }
        }
        Ok(best)
    }

    /// `FUN_009692B0`: give the army to the eligible item with the largest
    /// positive reduction (first wins).
    fn assign<R: KingdomRelations, H: MgaiHost>(&mut self, an: &Analysis, view: &View<'_, R>, host: &H, army: ArmyHandle) -> Result<()> {
        let mut best = ZERO_F32;
        let mut pick = None;
        for i in self.eligible()? {
            let r = self.items[i].reduction(an, view, host, army)?;
            if greater(&f32b(r)?, &f32b(best)?) {
                best = r;
                pick = Some(i);
            }
        }
        if let Some(i) = pick {
            self.items[i].add_army(army, best, view.p())?;
        }
        Ok(())
    }

    /// Goal virtual `+0x04` (Reset): 969430 / 9697D0 / 969600.
    fn reset<R: KingdomRelations, H: MgaiHost>(&mut self, an: &Analysis, view: &View<'_, R>, host: &H) -> Result<()> {
        self.items.clear(); // 9683B0(0)
        match self.kind {
            GoalKind::Attack | GoalKind::AttackUnoccupied => {
                for &t in &an.fronts {
                    let s = f32b(view.defender_strength(t)?)?;
                    let take = match self.kind {
                        GoalKind::Attack => greater(&s, &f32b(ZERO_F32)?),
                        _ => s.compare(&f32b(ZERO_F32)?) == Ordering::Equal,
                    };
                    if take {
                        let item = GoalItem::new(ItemKind::Attack, an, view, host, t)?;
                        self.insert(item)?;
                    }
                }
            }
            GoalKind::Defend => {
                for t in 0..an.territories.len() {
                    if view.state.territories[t].owner == Some(view.us) {
                        let item = GoalItem::new(ItemKind::Defend, an, view, host, t)?;
                        self.insert(item)?;
                    }
                }
            }
        }
        Ok(())
    }
}

// ------------------------------------------------------------------ player

impl MgaiPlayer {
    /// `FUN_00963430(controller, MG kingdom)`.
    pub fn new<R: KingdomRelations>(state: &MgState, data: &MetaGameData, relations: &R, precision: Precision, kingdom: usize) -> Result<Self> {
        let view = View::new(state, data, relations, precision, kingdom)?;
        Ok(Self {
            kingdom,
            analysis: Analysis::new(&view)?,
            goals: [Goal::new(GoalKind::Attack), Goal::new(GoalKind::AttackUnoccupied), Goal::new(GoalKind::Defend)],
            attacked: false,
            turn_ended: false,
        })
    }

    fn view<'a, R: KingdomRelations>(&self, state: &'a MgState, data: &'a MetaGameData, relations: &'a R, precision: Precision) -> Result<View<'a, R>> {
        View::new(state, data, relations, precision, self.kingdom)
    }

    /// `FUN_00962B70` DoMapAnalysis (timer 59ABA0 and logs omitted).
    pub fn map_analysis<R: KingdomRelations, H: MgaiHost>(&mut self, state: &mut MgState, data: &MetaGameData, relations: &R, precision: Precision, host: &mut H) -> Result<bool> {
        self.analysis.proximity(&self.view(state, data, relations, precision)?)?;
        if !host.yield_turn(state, 0)? {
            return Ok(false);
        }
        self.analysis.influence(&self.view(state, data, relations, precision)?, host)?;
        if !host.yield_turn(state, 0)? {
            return Ok(false);
        }
        self.analysis.borders(&self.view(state, data, relations, precision)?)?;
        if !host.yield_turn(state, 0)? {
            return Ok(false);
        }
        self.analysis.importance(&self.view(state, data, relations, precision)?, host)?;
        host.yield_turn(state, 0)
    }

    /// `FUN_009632E0(threshold, goals, armies)`.
    fn assign_armies<R: KingdomRelations, H: MgaiHost>(&mut self, view: &View<'_, R>, host: &H, threshold: u32, armies: &mut Vec<ArmyHandle>) -> Result<()> {
        for g in &mut self.goals {
            g.threshold_bits = threshold;
        }
        while !armies.is_empty() {
            let mut best = ZERO_F32;
            let mut pick: Option<(usize, usize)> = None;
            for (gi, g) in self.goals.iter().enumerate() {
                for (ai, &army) in armies.iter().enumerate() {
                    if !host.army_live(view.state, army) {
                        continue;
                    }
                    let r = g.best_reduction(&self.analysis, view, host, army)?;
                    if greater(&f32b(r)?, &f32b(best)?) {
                        best = r;
                        pick = Some((gi, ai));
                    }
                }
            }
            let Some((gi, ai)) = pick else { return Ok(()) };
            let army = armies[ai];
            let an = &self.analysis;
            self.goals[gi].assign(an, view, host, army)?;
            armies.remove(ai);
        }
        Ok(())
    }

    /// `FUN_009635A0` SetGoalPriorities (always true).
    pub fn set_goal_priorities<R: KingdomRelations, H: MgaiHost>(&mut self, state: &MgState, data: &MetaGameData, relations: &R, precision: Precision, host: &mut H) -> Result<bool> {
        let view = self.view(state, data, relations, precision)?;
        for gi in 0..3 {
            let an = &self.analysis;
            self.goals[gi].reset(an, &view, host)?;
        }
        let mut armies = Vec::new();
        for e in host.kingdom_armies(state, view.us_actor()?)? {
            if e.deleted {
                continue;
            }
            // the stored handle is the actor's current one (`actor + 0x14`)
            let handle = ArmyHandle { army: e.army, serial: host.army_serial(state, e.army) };
            if view.remaining(e.army)? <= 0 {
                let an = &self.analysis;
                self.goals[2].assign(an, &view, host, handle)?;
            } else {
                armies.push(handle);
            }
        }
        let mut best = ZERO_F32;
        for g in &self.goals {
            let m = g.max_priority()?;
            if greater(&f32b(m)?, &f32b(best)?) {
                best = m;
            }
        }
        let threshold = f32b(view.ai.goal_cutoff_ratio_bits)?.mul(&f32b(best)?, precision)?.store_f32()?;
        self.assign_armies(&view, host, threshold, &mut armies)?;
        self.assign_armies(&view, host, ZERO_F32, &mut armies)?;
        Ok(true)
    }

    /// `FUN_00962F00`: 0 buy armies, 1 upgrade armies, 2 buy technologies
    /// (one app RNG draw).
    fn purchase_mode<R: KingdomRelations, H: MgaiHost>(&self, view: &View<'_, R>, host: &mut H) -> Result<u32> {
        let p = view.p();
        let mut pa = view.ai.purchase_army_probability_bits;
        let mut pt = view.ai.purchase_tech_probability_bits;
        let pu = f32b(ONE_F32)?.sub(&f32b(pa)?, p)?.sub(&f32b(pt)?, p)?.store_f32()?;
        let draw = host.app_rng().next_u32() as i32;
        let mut roll = Binary::from_i32(draw);
        if draw < 0 {
            roll = roll.add(&Binary::from_f64_bits(TWO_POW_32_F64)?, p)?;
        }
        let roll = roll.mul(&Binary::from_f64_bits(INV_U32_MAX_F64)?, p)?.store_f32()?;
        if view.state.capital_lost(view.us, view.relations)? {
            pa = ZERO_F32;
            let s = f32b(pu)?.add(&f32b(pt)?, p)?;
            pt = if greater(&s, &f32b(ZERO_F32)?) { f32b(pt)?.div(&s, p)?.store_f32()? } else { HALF_F32 };
        }
        let (army, tech) = if view.upgrade_possible(host)? {
            (f32b(pa)?, f32b(pt)?)
        } else {
            let s = f32b(pt)?.add(&f32b(pa)?, p)?;
            if greater(&s, &f32b(ZERO_F32)?) {
                let scale = f32b(ONE_F32)?.div(&s, p)?.store_f32()?;
                (f32b(scale)?.mul(&f32b(pa)?, p)?, f32b(scale)?.mul(&f32b(pt)?, p)?)
            } else {
                (f32b(HALF_F32)?, f32b(HALF_F32)?)
            }
        };
        let sum = tech.add(&army, p)?;
        if !greater(&sum, &f32b(roll)?) {
            return Ok(1);
        }
        Ok(if f32b(roll)?.compare(&army) == Ordering::Less { 0 } else { 2 })
    }

    /// `FUN_00963900(item, army, &budget)`: random affordable missing shop
    /// divisions for an army at or next to the item territory.
    fn upgrade_army<R: KingdomRelations, H: MgaiHost>(view: &View<'_, R>, host: &mut H, t: usize, army: Option<ArmyId>, budget: &mut i32) -> Result<()> {
        let Some(army) = army else { return Ok(()) };
        let cur = view.territory_of_army(army)?;
        if cur != t && !view.link(cur, t) {
            return Ok(());
        }
        let have = &view.state.army(army).divisions;
        let mut rows = Vec::new();
        for row in &view.shop().divisions {
            let Some(ids) = &row.ids else {
                return Err(Error::Unsupported("963900: NULL shop division (93E9E0 on NULL)".into()));
            };
            let d = view.division_index(ids)?;
            if !have.contains(&crate::battle::DivisionRef(d as u32)) {
                rows.push((d, row.cost));
            }
        }
        // 962CB0
        let n = rows.len() as u32;
        let rng = host.app_rng();
        for i in 0..n.saturating_sub(1) {
            let j = i + rng.next_u32() % (n - i);
            rows.swap(i as usize, j as usize);
        }
        let mut chosen = Vec::new();
        for (d, cost) in rows {
            if cost <= *budget {
                chosen.push(d);
                *budget = budget.wrapping_sub(cost);
            }
        }
        if !chosen.is_empty() {
            let mut c = command(9);
            c.army = Some(army);
            c.divisions = Some(chosen);
            host.issue(view.state, c)?;
        }
        Ok(())
    }

    /// `FUN_00963B70` DoPurchasing (always true).
    pub fn purchase<R: KingdomRelations, H: MgaiHost>(&mut self, state: &MgState, data: &MetaGameData, relations: &R, precision: Precision, host: &mut H) -> Result<bool> {
        let view = self.view(state, data, relations, precision)?;
        let mode = self.purchase_mode(&view, host)?;
        let mut budget = state.kingdoms[self.kingdom].resources;
        match mode {
            1 => {
                for item in self.goals[0].items.iter().filter(|i| !i.armies.is_empty()) {
                    if !greater(&f32b(view.defender_strength(item.territory)?)?, &f32b(ZERO_F32)?) {
                        continue;
                    }
                    for &a in &item.armies {
                        let a = host.army_live(state, a).then_some(a.army);
                        Self::upgrade_army(&view, host, item.territory, a, &mut budget)?;
                    }
                }
            }
            0 => {
                if view.enemy_territory_count(self.kingdom)? > 0 {
                    let rows = &view.shop().actors;
                    if rows.is_empty() {
                        return Err(Error::Unsupported("963B70: empty actor shop (native division by zero)".into()));
                    }
                    if rows.iter().all(|r| r.cost <= 0) {
                        return Err(Error::Unsupported("963B70: actor shop without positive cost (native endless loop)".into()));
                    }
                    loop {
                        let i = (host.app_rng().next_u32() % rows.len() as u32) as usize;
                        if rows[i].cost > budget {
                            break;
                        }
                        let ids = rows[i].ids.as_deref().ok_or_else(|| Error::Unsupported("963B70: NULL shop actor".into()))?;
                        let def = data
                            .armies
                            .iter()
                            .position(|a| a.ids == ids)
                            .ok_or_else(|| Error::Unsupported(format!("shop actor {ids} not loaded")))?;
                        let mut c = command(8);
                        c.actor_definition = Some(def);
                        host.issue(state, c)?;
                        budget = budget.wrapping_sub(rows[i].cost);
                    }
                }
            }
            2 => {
                let owned = &state.kingdoms[self.kingdom].technologies;
                let rows: Vec<_> = view
                    .shop()
                    .technologies
                    .iter()
                    .filter(|r| r.ids.as_ref().is_none_or(|ids| !owned.contains(ids)))
                    .collect();
                if !rows.is_empty() && rows.iter().all(|r| r.cost <= 0) {
                    return Err(Error::Unsupported("963B70: technology shop without positive cost (native endless loop)".into()));
                }
                while !rows.is_empty() {
                    let i = (host.app_rng().next_u32() % rows.len() as u32) as usize;
                    if rows[i].cost > budget {
                        break;
                    }
                    let ids = rows[i].ids.clone().ok_or_else(|| Error::Unsupported("963B70: NULL shop technology".into()))?;
                    let mut c = command(7);
                    c.technology = Some(ids);
                    host.issue(state, c)?;
                    budget = budget.wrapping_sub(rows[i].cost);
                }
            }
            _ => unreachable!("962F00 returns 0, 1 or 2"),
        }
        Ok(true)
    }

    /// Battle wait of 963FD0 / 962A40: yield(1) until MGState+0x46, then
    /// yield(1) until 3 ticks passed (at least once).
    fn settle<H: MgaiHost>(state: &mut MgState, host: &mut H) -> Result<bool> {
        while !state.battle_resolved {
            if !host.yield_turn(state, 1)? {
                return Ok(false);
            }
        }
        let t0 = host.tick();
        loop {
            if !host.yield_turn(state, 1)? {
                return Ok(false);
            }
            if host.tick().wrapping_sub(t0) >= BATTLE_SETTLE_TICKS {
                return Ok(true);
            }
        }
    }

    /// `FUN_00963030` DoMovement: issue the orders and wait for arrival.
    fn do_movement<H: MgaiHost>(state: &mut MgState, host: &mut H, orders: Vec<MoveOrder>) -> Result<bool> {
        let arrived = |state: &MgState, host: &mut H, o: &MoveOrder| -> Result<Option<bool>> {
            if !host.army_live(state, o.army) {
                return Ok(None);
            }
            if state.army(o.army.army).territory != Some(o.territory) {
                return Ok(Some(false));
            }
            Ok(Some(!host.army_busy(state, o.army.army)?))
        };
        if state.fast_ai != 0 {
            for o in &orders {
                host.issue(state, o.command.clone())?;
            }
            let t0 = host.tick();
            loop {
                if !host.yield_turn(state, 1)? {
                    return Ok(false);
                }
                let mut all = true;
                for o in &orders {
                    if arrived(state, host, o)? == Some(false) {
                        all = false;
                    }
                }
                if host.tick().wrapping_sub(t0) > MOVE_WAIT_TICKS || all {
                    break;
                }
            }
        } else {
            for o in &orders {
                let t0 = host.tick();
                host.issue(state, o.command.clone())?;
                if !host.yield_turn(state, 1)? {
                    return Ok(false);
                }
                loop {
                    match arrived(state, host, o)? {
                        None | Some(true) => break,
                        Some(false) => {}
                    }
                    if !host.yield_turn(state, 1)? {
                        return Ok(false);
                    }
                    if host.tick().wrapping_sub(t0) > MOVE_WAIT_TICKS {
                        break;
                    }
                }
            }
        }
        Ok(true)
    }

    /// `FUN_00963FD0` PossiblyAttack.
    pub fn possibly_attack<R: KingdomRelations, H: MgaiHost>(&mut self, state: &mut MgState, data: &MetaGameData, relations: &R, precision: Precision, host: &mut H) -> Result<bool> {
        if 1i32.wrapping_sub(state.kingdoms[self.kingdom].battles) <= 0 {
            return Ok(true);
        }
        let pending = self.view(state, data, relations, precision)?.pending_battle()?;
        if pending {
            host.issue(state, command(2))?;
            return Self::settle(state, host);
        }
        let mut attacked = false;
        let n_items = self.goals[0].items.len();
        for ii in 0..n_items {
            if self.goals[0].items[ii].armies.is_empty() || attacked {
                continue;
            }
            let orders = {
                let view = self.view(state, data, relations, precision)?;
                let item = &self.goals[0].items[ii];
                let t = item.territory;
                let mut list = Vec::new();
                for &a in &item.armies {
                    if host.army_live(state, a) && view.link(view.territory_of_army(a.army)?, t) && view.remaining(a.army)? > 0 {
                        list.push(a);
                    }
                }
                for d in self.goals[2].items.iter().filter(|i| !i.armies.is_empty()) {
                    if view.link(d.territory, t) && !view.threatened(d.territory, t)? {
                        for &a in &d.armies {
                            if host.army_live(state, a)
                                && view.state.army(a.army).territory == Some(d.territory)
                                && view.remaining(a.army)? > 0
                            {
                                list.push(a);
                            }
                        }
                    }
                }
                for (oi, o) in self.goals[0].items.iter().enumerate() {
                    if oi == ii || o.armies.is_empty() {
                        continue;
                    }
                    for &a in &o.armies {
                        if host.army_live(state, a) && view.link(view.territory_of_army(a.army)?, t) && view.remaining(a.army)? > 0 {
                            list.push(a);
                        }
                    }
                }
                let required = self
                    .analysis
                    .record(view.ai, self.analysis.territories[t].record)
                    .required_attack_strength_ratio_bits;
                let ratio = GoalItem::attack_ratio(&view, host, t, &list)?.store_f32()?;
                if !greater(&f32b(ratio)?, &f32b(required)?) {
                    None
                } else {
                    let map = data.campaigns[state.campaign].map.expect("checked by View");
                    let mut orders = Vec::new();
                    for &a in &list {
                        if !host.army_live(state, a) {
                            continue;
                        }
                        let mut c = command(1);
                        c.army = Some(a.army);
                        c.set_territory(data, map, state.territories[t].definition);
                        orders.push(MoveOrder { army: a, territory: t, command: c });
                    }
                    Some(orders)
                }
            };
            let Some(orders) = orders else { continue };
            if !Self::do_movement(state, host, orders)? {
                return Ok(false);
            }
            self.attacked = true;
            host.issue(state, command(2))?;
            if !Self::settle(state, host)? {
                return Ok(false);
            }
            attacked = true;
        }
        Ok(true)
    }

    /// `FUN_009649C0` Move: one step toward every item for every assigned
    /// army that can still move.
    pub fn move_armies<R: KingdomRelations, H: MgaiHost>(&mut self, state: &mut MgState, data: &MetaGameData, relations: &R, precision: Precision, host: &mut H) -> Result<bool> {
        let orders = {
            let view = self.view(state, data, relations, precision)?;
            let map = data.campaigns[state.campaign].map.expect("checked by View");
            let mut orders = Vec::new();
            for g in &self.goals {
                for item in &g.items {
                    for &h in &item.armies {
                        if !host.army_live(state, h) {
                            continue;
                        }
                        let a = h.army;
                        let t = item.territory;
                        if view.remaining(a)? <= 0 {
                            continue;
                        }
                        let cur = view.territory_of_army(a)?;
                        if cur == t {
                            continue;
                        }
                        let r = view.search(cur, t, UNLIMITED)?;
                        if r.code != SEARCH_FOUND {
                            continue;
                        }
                        let step = *r
                            .path
                            .first()
                            .ok_or_else(|| Error::Unsupported("9649C0: found path without a step (native reads element 0)".into()))?;
                        let owner = state.army(a).owner;
                        if view.friendly_actor(step, owner)? {
                            let mut c = command(0);
                            c.set_territory(data, map, state.territories[step].definition);
                            c.army = Some(a);
                            orders.push(MoveOrder { army: h, territory: step, command: c });
                        }
                        if view.hostile_actor(step, owner)? && is_zero_f32(view.defender_strength(step)?) {
                            let mut c = command(1);
                            c.set_territory(data, map, state.territories[step].definition);
                            c.army = Some(a);
                            orders.push(MoveOrder { army: h, territory: step, command: c });
                        }
                    }
                }
            }
            orders
        };
        Self::do_movement(state, host, orders)
    }

    /// `FUN_00962A40` EndTurn: wait for a pending battle's resolution, then
    /// issue endturn.
    pub fn end_turn<R: KingdomRelations, H: MgaiHost>(&mut self, state: &mut MgState, data: &MetaGameData, relations: &R, precision: Precision, host: &mut H) -> Result<bool> {
        let pending = self.view(state, data, relations, precision)?.pending_battle()?;
        if pending && !state.battle_resolved {
            loop {
                if !host.yield_turn(state, 1)? {
                    return Ok(false);
                }
                if state.battle_resolved {
                    break;
                }
            }
        }
        host.issue(state, command(6))?;
        Ok(true)
    }

    /// `FUN_00964D30` PlayTurn.
    pub fn play_turn<R: KingdomRelations, H: MgaiHost>(&mut self, state: &mut MgState, data: &MetaGameData, relations: &R, precision: Precision, host: &mut H) -> Result<bool> {
        self.attacked = false;
        self.analysis.reset(state.kingdoms.len());
        if !host.yield_turn(state, 0)? {
            return Ok(false);
        }
        let ok = self.map_analysis(state, data, relations, precision, host)?;
        host.stage(Stage::Analysis, self, state);
        if !ok {
            return Ok(false);
        }
        let ok = self.set_goal_priorities(state, data, relations, precision, host)?;
        host.stage(Stage::Goals1, self, state);
        if !ok || !host.yield_turn(state, 0)? {
            return Ok(false);
        }
        let ok = self.purchase(state, data, relations, precision, host)?;
        host.stage(Stage::Purchase, self, state);
        if !ok || !host.yield_turn(state, 1)? {
            return Ok(false);
        }
        let ok = self.set_goal_priorities(state, data, relations, precision, host)?;
        host.stage(Stage::Goals2, self, state);
        if !ok || !host.yield_turn(state, 0)? {
            return Ok(false);
        }
        let ok = self.possibly_attack(state, data, relations, precision, host)?;
        host.stage(Stage::Attack, self, state);
        if !ok {
            return Ok(false);
        }
        // 9649A0
        let ok = if state.battle_resolved { self.set_goal_priorities(state, data, relations, precision, host)? } else { true };
        host.stage(Stage::Goals3, self, state);
        if !ok || !host.yield_turn(state, 0)? {
            return Ok(false);
        }
        let ok = self.move_armies(state, data, relations, precision, host)?;
        host.stage(Stage::Move, self, state);
        if !ok {
            return Ok(false);
        }
        self.turn_ended = true;
        let ok = self.end_turn(state, data, relations, precision, host)?;
        host.stage(Stage::End, self, state);
        if !ok {
            return Ok(false);
        }
        host.yield_turn(state, 0)
    }
}
