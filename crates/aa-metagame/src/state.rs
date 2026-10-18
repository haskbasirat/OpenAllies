//! Runtime metagame state (`KKC_MGState`, MG kingdoms, MG territories and the
//! metagame side of army actors), mirroring the native object graph with
//! typed indices instead of pointers/handles.
//!
//! Native layouts (offsets of the 32-bit original):
//! - `KKC_MGState`: +0x08 campaign data, +0x0C faction data, +0x10 current MG
//!   kingdom, +0x14 round, +0x18 RNG state, +0x1C/+0x20 MG kingdoms,
//!   +0x28/+0x2C MG territories, +0x34/+0x38 territory hash (lookup speed
//!   only), +0x44..+0x46 battle flags, +0x48 RTS winner name, +0x4C last
//!   purchased army, +0x50/+0x51 flags, +0x54/+0x58 event states, +0x60 battle
//!   records.
//! - MG kingdom (ctor 95A100, 0x40 bytes): +0x04 state back-pointer, +0x08
//!   campaign kingdom data, +0x0C kingdom actor handle, +0x10 capital
//!   territory, +0x14 owned territories, +0x20 owned technologies, +0x2C help
//!   popups, +0x38 resources (starting_resources), +0x3C battles counter (967530).
//! - MG territory (ctors 954A90 owned / 954AE0 unowned, 0x28 bytes): +0x04
//!   index, +0x08 campaign territory record or none, +0x0C map territory,
//!   +0x10 army references kept sorted by slot (956140), +0x1C armies the owner
//!   regards as enemies, +0x20 territory visual handle, +0x24 owner.
//! - army reference (9548E0, 0x10 bytes, vtable ACBDCC): +0x04 army actor
//!   handle, +0x08 slot, +0x0C army snapshot (0x28 bytes, 9405E0 -> 9402E0).
//! - event state (95AD60 / 95AD90, 0x14 bytes, vtable ACC4DC): +0x04 campaign,
//!   +0x08 campaign event, +0x0C triggered byte, +0x10 campaign kingdom.
//! - army actor: MetaGame component at +0xCC (+0x04 actor, +0x14 live
//!   divisions, +0x10 battles fought this turn, +0x20 current territory,
//!   +0x24 path of territories visited this turn, +0x30 waypoint queue).
use crate::battle::{DivisionRef, KingdomActor};
use crate::rng::MetaGameRng;
use serde::{Deserialize, Serialize};

/// Index of an army in [`MgState::armies`] (stable; native uses actor handles).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ArmyId(pub u32);

/// Army reference held in a territory list (0x10-byte ref object, 9548E0):
/// the army, its display slot (+0x08), kept in ascending slot order, and the
/// snapshot (+0x0C) 9548E0 takes when the reference is created.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArmyRef {
    pub army: ArmyId,
    pub slot: i32,
    pub snapshot: ArmySnapshot,
}

/// Army snapshot (93F590 layout, 0x28 bytes) of a territory army reference.
/// 9402E0 fills it when 956140 creates the reference (inside 940100: after
/// the component territory is set, before the path update 93F9F0); nothing
/// refreshes it afterwards. The strategic save writes it (93F1A0) and the
/// loader rebuilds it (9409E0).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArmySnapshot {
    /// `+0x00`: component `+0x10` (battles fought) at capture.
    pub fought: i32,
    /// `+0x04`: map territory definition of the entered territory (a loaded
    /// snapshot keeps NULL when 95D900 finds no territory).
    pub territory: Option<usize>,
    /// `+0x08`: map territory definitions of the component path at capture.
    pub path: Vec<Option<usize>>,
    /// `+0x14`: the component division list at capture (NULL entries only
    /// from loaded empty IDS).
    pub divisions: Vec<Option<DivisionRef>>,
    /// `+0x20`: engine name (4BEDF0) of the owning kingdom actor. `None` only
    /// when the engine did not supply the name ([`MgState::kingdom_names`]);
    /// the save writer refuses it.
    pub owner_name: Option<String>,
    /// `+0x24`: the army Thing (index into `MetaGameData::armies`); NULL when a
    /// loaded IDS is not a registered actor Thing (4BDBA0).
    pub definition: Option<usize>,
}

/// Event state (95AD60 / 95AD90, 0x14 bytes): one per campaign event, created
/// by InitCreate (939B90: 95BE70 + 95AD60, in campaign event order).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventState {
    /// `+0x08`: the campaign event (index into `Campaign::events`).
    pub event: usize,
    /// `+0x0C`: triggered byte (0 from both constructors; 95ADD0 sets it when
    /// the event fires this round).
    pub triggered: bool,
    /// `+0x10`: campaign kingdom the event applies to (index into
    /// `Campaign::kingdoms`; NULL from both constructors and when not triggered).
    pub kingdom: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MgArmy {
    /// Army type (index into `MetaGameData::armies`).
    pub definition: usize,
    /// Owning kingdom actor (army actor vtable+0x108).
    pub owner: KingdomActor,
    /// Live division list (component +0x14), native order.
    pub divisions: Vec<DivisionRef>,
    /// Battles fought this turn (component +0x10; 93E8F0 increments, 93FE10
    /// resets at turn start). Moving and attacking are refused while > 0.
    pub fought: i32,
    /// Current territory (component +0x20).
    pub territory: Option<usize>,
    /// Visited territories (component +0x24); 93F9F0 truncates after a
    /// revisited territory, otherwise appends.
    pub path: Vec<usize>,
    /// Pending move orders (component +0x30/+0x34 list of territories), fed
    /// by 93F4B0 (move/attack) and 93F550 (retreat), consumed by the deferred
    /// per-tick mover 940020.
    pub waypoints: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MgKingdom {
    /// Campaign kingdom index (+0x08).
    pub data: usize,
    /// Kingdom actor (+0x0C handle); `None` when the world had none.
    pub actor: Option<KingdomActor>,
    /// Capital territory (+0x10).
    pub capital: Option<usize>,
    /// Owned territories (+0x14), in the order 955620 adds them.
    pub territories: Vec<usize>,
    /// Owned technologies (+0x20), by technology (shared property) IDS, in
    /// acquisition order (95A770 appends when absent). `""` stands for a NULL
    /// entry (only a loaded empty IDS produces one; 95AAC0 appends NULL).
    pub technologies: Vec<String>,
    /// Help popups (+0x2C, `KKC_MGHelpPopupData` IDS); `""` stands for a NULL
    /// entry (written empty by 95A470, skipped by 95AAC0).
    pub help_popups: Vec<String>,
    /// Resources (+0x38), initialised from `starting_resources`.
    pub resources: i32,
    /// Counter +0x3C (incremented per resolved battle by 967530).
    pub battles: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MgTerritory {
    /// Campaign territory record (campaign kingdom, record) for owned starts.
    pub record: Option<(usize, usize)>,
    /// Map territory index (+0x0C).
    pub definition: usize,
    /// Army references (+0x10), ascending slot order.
    pub armies: Vec<ArmyRef>,
    /// +0x1C: armies whose kingdom the owner regards as enemies.
    pub enemy_army_count: i32,
    /// Owner (+0x24).
    pub owner: Option<usize>,
    /// +0x20: GTID handle of the territory visual (955A40 / 4F9C10, engine).
    /// `None` when the engine did not supply it; the save writer refuses it.
    pub visual: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MgState {
    /// Campaign index (+0x08).
    pub campaign: usize,
    /// Current kingdom (+0x10).
    pub current: Option<usize>,
    /// RNG (+0x18).
    pub rng: MetaGameRng,
    pub kingdoms: Vec<MgKingdom>,
    pub territories: Vec<MgTerritory>,
    /// Army arena; removed armies stay `None` so ids remain stable.
    pub armies: Vec<Option<MgArmy>>,
    /// Round counter (+0x14), 1 at construction (9397E0).
    pub round: i32,
    /// +0x44 pending battle started (954440), +0x45 RTS play requested
    /// (950E40), +0x46 battle resolved this turn (954130), +0x50 game over
    /// (938E10), +0x51 fast-AI decree byte (not written by 9397E0).
    pub battle_started: bool,
    pub battle_play_requested: bool,
    pub battle_resolved: bool,
    pub game_over: bool,
    pub fast_ai: u8,
    /// +0x4C: last purchased army (938A30).
    pub last_purchased: Option<ArmyId>,
    /// +0x60: battle results of the current turn (95EFD0 entries; cleared by
    /// every kingdom advance 938A80).
    pub results: Vec<BattleRecord>,
    /// +0x0C: faction data (`KKC_FactionData` IDS) given to the constructor
    /// 9397E0 (`None` for NULL); sent with `SetKingdomFaction` by the RTS
    /// launch and saved by 938EE0.
    pub faction: Option<String>,
    /// +0x48: name of the RTS battle winner. 60A140 (game type 4, RTS game
    /// over) assigns the 4BEDF0 name of the last world kingdom whose `+0x19C`
    /// is 1 (empty when none); 93A990 compares it with the battle territory's
    /// `rmc_defender_ids`. Both constructors set it empty.
    pub rts_winner: String,
    /// +0x54: event states.
    pub events: Vec<EventState>,
    /// Engine input, not a native field: the 4BEDF0 name of each kingdom
    /// actor, read by 9402E0 when an army snapshot is captured.
    pub kingdom_names: Vec<(KingdomActor, String)>,
}

/// World seams InitCreate (939B90) calls, supplied by the engine.
pub struct InitEngine<'a> {
    /// 4F48E0: the kingdom actor of campaign kingdom `i` (`None` when the
    /// world has none: `MGKingdom+0x0C` stays 0).
    pub kingdom_actor: &'a dyn Fn(usize) -> Option<KingdomActor>,
    /// 4BEDF0: the engine name of a kingdom actor (army snapshots `+0x20`).
    pub kingdom_name: &'a dyn Fn(KingdomActor) -> Option<String>,
    /// 955A40 / 4F9C10: GTID handle of the visual created for MG territory `t`.
    pub territory_visual: &'a dyn Fn(usize) -> Option<u32>,
    /// The 9397E0 faction argument (`MGState+0x0C`), by IDS.
    pub faction: Option<String>,
}

/// Battle-result entry 95EFD0 (0x14 bytes): territory, attacker MG kingdom,
/// defender (owner) MG kingdom, attacker won.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleRecord {
    pub territory: usize,
    pub attacker: Option<usize>,
    pub defender: Option<usize>,
    pub attacker_won: bool,
}

use crate::battle::KingdomRelations;
use crate::data::MetaGameData;
use crate::{Error, Result};

/// RNG word written by the `KKC_MGState` constructors 9397E0/939690 (`FUN_0085D870`).
pub const INITIAL_RNG_STATE: u32 = 0x8e4b_12a5;

impl MgState {
    /// Army by id (panics on a removed id: callers keep ids valid).
    pub fn army(&self, id: ArmyId) -> &MgArmy {
        self.armies[id.0 as usize].as_ref().expect("live army")
    }
    fn army_mut(&mut self, id: ArmyId) -> &mut MgArmy {
        self.armies[id.0 as usize].as_mut().expect("live army")
    }
    /// Kingdom actor of a territory owner.
    pub fn owner_actor(&self, territory: usize) -> Option<KingdomActor> {
        self.territories[territory]
            .owner
            .and_then(|k| self.kingdoms[k].actor)
    }

    /// `FUN_00954CF0`: recount `+0x1C` (every army when unowned).
    pub fn recount_enemies<R: KingdomRelations>(&mut self, territory: usize, relations: &R) {
        let owner = self.owner_actor(territory);
        let count = self.territories[territory]
            .armies
            .iter()
            .filter(|r| match owner {
                None => true,
                Some(o) => relations.enemy(o, self.army(r.army).owner),
            })
            .count();
        self.territories[territory].enemy_army_count = count as i32;
    }

    /// `FUN_00955620` (set owner): leave the old owner list (959F20), take
    /// the new owner, append to its list (95A850), recount (954CF0). The
    /// native function has no NULL owner case: 955620(NULL) dereferences NULL
    /// in 95A850, so the owner is required here (see [`Self::clear_owner`]).
    pub fn set_owner<R: KingdomRelations>(&mut self, territory: usize, owner: usize, relations: &R) {
        self.leave_owner_list(territory);
        self.territories[territory].owner = Some(owner);
        self.kingdoms[owner].territories.push(territory);
        self.recount_enemies(territory, relations);
    }

    /// NOT a native operation: the unowned state that InitCreate (939B90)
    /// leaves on unowned map territories, built from an owned territory for
    /// oracle coverage (959F20 removal, owner NULL, 954CF0 recount). No
    /// native transition makes an owned territory unowned (955620 with NULL
    /// crashes in 95A850); the oracle harnesses use the same synthetic setup.
    pub fn clear_owner<R: KingdomRelations>(&mut self, territory: usize, relations: &R) {
        self.leave_owner_list(territory);
        self.territories[territory].owner = None;
        self.recount_enemies(territory, relations);
    }

    /// `FUN_00959F20` on the current owner: removal of the LAST matching
    /// entry from its owned-territory list.
    fn leave_owner_list(&mut self, territory: usize) {
        if let Some(old) = self.territories[territory].owner {
            let list = &mut self.kingdoms[old].territories;
            if let Some(p) = list.iter().rposition(|&t| t == territory) {
                list.remove(p);
            }
        }
    }

    /// `FUN_00956280` (thiscall on the territory): drop the army reference
    /// (later references shift down, slots unchanged) and decrement `+0x1C`
    /// when there is no owner or the owner regards the army kingdom as an
    /// enemy. Returns false when the army was not in the list.
    pub fn remove_from_territory<R: KingdomRelations>(&mut self, territory: usize, army: ArmyId, relations: &R) -> bool {
        let Some(index) = self.territories[territory].armies.iter().position(|r| r.army == army) else {
            return false;
        };
        self.territories[territory].armies.remove(index);
        let counts = match self.owner_actor(territory) {
            None => true,
            Some(o) => relations.enemy(o, self.army(army).owner),
        };
        if counts {
            self.territories[territory].enemy_army_count -= 1;
        }
        true
    }

    /// Engine name (4BEDF0) of a kingdom actor, from [`Self::kingdom_names`].
    pub fn kingdom_name(&self, actor: KingdomActor) -> Option<&str> {
        self.kingdom_names.iter().find(|(a, _)| *a == actor).map(|(_, n)| n.as_str())
    }

    /// `FUN_009402E0` (called by 9405E0 from 9548E0 when 956140 creates a
    /// reference): the snapshot of `army` as it enters `territory`. 940100
    /// has already set the component territory; the path is still the one
    /// before 93F9F0.
    pub fn capture_snapshot(&self, army: ArmyId, territory: usize) -> ArmySnapshot {
        let a = self.army(army);
        ArmySnapshot {
            fought: a.fought,
            territory: Some(self.territories[territory].definition),
            path: a.path.iter().map(|&t| Some(self.territories[t].definition)).collect(),
            divisions: a.divisions.iter().map(|&d| Some(d)).collect(),
            owner_name: self.kingdom_name(a.owner).map(str::to_owned),
            definition: Some(a.definition),
        }
    }

    /// `FUN_00940100` (`956280` on the previous territory, `956140`,
    /// `93F9F0`): put an army into a territory (first free display slot, list
    /// kept in slot order; `+0x1C` counts it when the owner regards it as an
    /// enemy or there is no owner; the new reference captures the army
    /// snapshot, 9402E0) and record the territory in the army path.
    pub fn enter_territory<R: KingdomRelations>(&mut self, army: ArmyId, territory: usize, relations: &R) -> Result<i32> {
        if let Some(previous) = self.army(army).territory {
            self.remove_from_territory(previous, army, relations);
        }
        self.army_mut(army).territory = Some(territory);
        // 956140: first slot number not used by the slot-ordered prefix.
        let refs = &self.territories[territory].armies;
        let mut slot = 0i32;
        let mut position = 0usize;
        while position < refs.len() && refs[position].slot == slot {
            slot += 1;
            position += 1;
        }
        // 9548E0 -> 9405E0 -> 9402E0
        let snapshot = self.capture_snapshot(army, territory);
        self.territories[territory]
            .armies
            .insert(position, ArmyRef { army, slot, snapshot });
        let counts = match self.owner_actor(territory) {
            None => true,
            Some(o) => relations.enemy(o, self.army(army).owner),
        };
        if counts {
            self.territories[territory].enemy_army_count += 1;
        }
        // 93F9F0: truncate after a revisited territory, else append.
        let path = &mut self.army_mut(army).path;
        match path.iter().position(|&t| t == territory) {
            Some(i) => path.truncate(i + 1),
            None => path.push(territory),
        }
        Ok(slot)
    }

    /// [`Self::init_create_with`] with only the kingdom actor lookup supplied:
    /// no engine names (army snapshots get no owner name), no territory visual
    /// handles and a NULL faction. Such a state cannot be saved (938EE0 needs
    /// all three); everything else is the same.
    pub fn init_create<R: KingdomRelations>(
        data: &MetaGameData,
        campaign: usize,
        kingdom_actor: impl Fn(usize) -> Option<KingdomActor>,
        relations: &R,
    ) -> Result<Self> {
        let engine = InitEngine {
            kingdom_actor: &kingdom_actor,
            kingdom_name: &|_| None,
            territory_visual: &|_| None,
            faction: None,
        };
        Self::init_create_with(data, campaign, &engine, relations)
    }

    /// `KKC_MGState::InitCreate` 939B90 for campaign `campaign` on a state
    /// from the constructor 9397E0 (round 1, RNG 0x8E4B12A5, faction).
    ///
    /// `engine.kingdom_actor(i)` stands for the world lookup 4F48E0 of
    /// campaign kingdom `i` IDS, `engine.territory_visual(t)` for the visual
    /// 955A40 creates for MG territory `t`, `engine.kingdom_name` for 4BEDF0
    /// (army snapshots). Army actor creation 4FA030 is an engine seam: the
    /// created army gets the territory owner kingdom actor and its type
    /// division list (ASSUMPTION until the component creation is ported).
    /// Display positions (TerritoryPositionSolver 959420, trig tables) are
    /// presentation and not produced here; map table props (939230) are not
    /// ported (no state effect identified).
    pub fn init_create_with<R: KingdomRelations>(
        data: &MetaGameData,
        campaign: usize,
        engine: &InitEngine,
        relations: &R,
    ) -> Result<Self> {
        let kingdom_actor = engine.kingdom_actor;
        let c = data
            .campaigns
            .get(campaign)
            .ok_or_else(|| Error::Unsupported(format!("campaign {campaign}")))?;
        let map = c
            .map
            .ok_or_else(|| Error::Unsupported("campaign without map".into()))?;
        let mut state = MgState {
            campaign,
            current: None,
            rng: MetaGameRng::new(INITIAL_RNG_STATE),
            kingdoms: Vec::new(),
            territories: Vec::new(),
            armies: Vec::new(),
            round: 1,
            battle_started: false,
            battle_play_requested: false,
            battle_resolved: false,
            game_over: false,
            fast_ai: 0,
            last_purchased: None,
            results: Vec::new(),
            faction: engine.faction.clone(),
            rts_winner: String::new(),
            events: Vec::new(),
            kingdom_names: Vec::new(),
        };
        // 1. one MG kingdom per campaign kingdom (95A100: resources = starting_resources)
        for (i, k) in c.kingdoms.iter().enumerate() {
            let actor = kingdom_actor(i);
            if let Some(a) = actor
                && let Some(name) = (engine.kingdom_name)(a)
                && state.kingdom_name(a).is_none()
            {
                state.kingdom_names.push((a, name));
            }
            state.kingdoms.push(MgKingdom {
                data: i,
                actor,
                capital: None,
                territories: Vec::new(),
                technologies: Vec::new(),
                help_popups: Vec::new(),
                resources: k.starting_resources,
                battles: 0,
            });
        }
        // 2. owned territories, kingdom by kingdom, record by record
        for (i, k) in c.kingdoms.iter().enumerate() {
            for (j, rec) in k.territories.iter().enumerate() {
                let t = state.territories.len();
                state.territories.push(MgTerritory {
                    record: Some((i, j)),
                    definition: rec.territory,
                    armies: Vec::new(),
                    enemy_army_count: 0,
                    owner: None,
                    visual: (engine.territory_visual)(t),
                });
                state.set_owner(t, i, relations);
                // 955D50 -> 955B70 per authored actor
                for ids in &rec.actors {
                    let definition = data
                        .armies
                        .iter()
                        .position(|a| a.ids.eq_ignore_ascii_case(ids))
                        .ok_or_else(|| Error::Unsupported(format!("army type {ids} not loaded")))?;
                    let owner = state.owner_actor(t).ok_or_else(|| {
                        Error::Unsupported("army created for a kingdom without actor".into())
                    })?;
                    let id = ArmyId(state.armies.len() as u32);
                    state.armies.push(Some(MgArmy {
                        definition,
                        owner,
                        divisions: data.armies[definition]
                            .divisions
                            .iter()
                            .map(|&d| DivisionRef(d as u32))
                            .collect(),
                        fought: 0,
                        territory: None,
                        path: Vec::new(),
                        waypoints: Vec::new(),
                    }));
                    state.enter_territory(id, t, relations)?;
                }
                if rec.territory == k.capital {
                    state.kingdoms[i].capital = Some(t);
                }
            }
        }
        // 3. map territories no kingdom owns (95A690 over every kingdom)
        for d in 0..data.maps[map].territories.len() {
            let owned = state
                .kingdoms
                .iter()
                .any(|k| k.territories.iter().any(|&t| state.territories[t].definition == d));
            if !owned {
                let t = state.territories.len();
                state.territories.push(MgTerritory {
                    record: None,
                    definition: d,
                    armies: Vec::new(),
                    enemy_army_count: 0,
                    owner: None,
                    visual: (engine.territory_visual)(t),
                });
            }
        }
        // 4. capitals still unset: hash lookup of the capital IDS
        for i in 0..state.kingdoms.len() {
            if state.kingdoms[i].capital.is_none() {
                let capital = c.kingdoms[state.kingdoms[i].data].capital;
                state.kingdoms[i].capital = state
                    .territories
                    .iter()
                    .position(|t| t.definition == capital);
            }
        }
        // 5. one event state per campaign event (95BE70 + 95AD60: triggered 0,
        //    no kingdom); map table props (939230) are not ported.
        state.events = (0..c.events.len())
            .map(|event| EventState { event, triggered: false, kingdom: None })
            .collect();
        state.current = (!state.kingdoms.is_empty()).then_some(0);
        Ok(state)
    }
}

impl MgState {
    /// `FUN_0095A350`: kingdom `k` lost its capital (the capital is unowned, or
    /// its owner is a kingdom `k` regards as an enemy). Native dereferences the
    /// capital and `k` actor unconditionally; missing ones are unsupported here.
    pub fn capital_lost<R: KingdomRelations>(&self, k: usize, relations: &R) -> Result<bool> {
        let capital = self.kingdoms[k]
            .capital
            .ok_or_else(|| Error::Unsupported("95A350 on a kingdom without capital (native crash)".into()))?;
        let Some(owner) = self.territories[capital].owner else {
            return Ok(true);
        };
        let me = self.kingdoms[k]
            .actor
            .ok_or_else(|| Error::Unsupported("95A350 on a kingdom without actor".into()))?;
        let other = self.kingdoms[owner]
            .actor
            .ok_or_else(|| Error::Unsupported("capital owner without actor".into()))?;
        Ok(relations.enemy(me, other))
    }

    /// `FUN_0095A6B0`: kingdom `k` has won when the number of OTHER kingdoms
    /// it regards as enemies and whose capital is lost (95A350, evaluated only
    /// for enemies) reaches its objective `capital_captures` (signed compare).
    pub fn has_won<R: KingdomRelations>(&self, data: &MetaGameData, k: usize, relations: &R) -> Result<bool> {
        let me = self.kingdoms[k]
            .actor
            .ok_or_else(|| Error::Unsupported("95A6B0 on a kingdom without actor".into()))?;
        let mut count = 0i32;
        for (i, x) in self.kingdoms.iter().enumerate() {
            if i == k {
                continue;
            }
            let other = x
                .actor
                .ok_or_else(|| Error::Unsupported("kingdom without actor".into()))?;
            if relations.enemy(me, other) && self.capital_lost(i, relations)? {
                count += 1;
            }
        }
        let captures = data.campaigns[self.campaign].kingdoms[self.kingdoms[k].data].capital_captures;
        Ok(captures <= count)
    }
}
