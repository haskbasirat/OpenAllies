//! Fresh native7B4830 AIPlayer construction and concrete goal publication.
//! Inputs are resolved live profile/Kingdom data, never inferred from the map UI.
use crate::{
    region_search::RegionGraph,
    sai_state::{self, Controller, GoalBase, Kingdom},
    startup::Object,
};
pub use sai_state::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecruitmentOption {
    pub object: Object,
    /// A present option can still have no definition at native+4.
    pub definition_type: Option<usize>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Recruitment {
    pub options: Vec<Option<RecruitmentOption>>,
    /// Actual profile+884 table, indexed by option definition+C.
    pub enabled_types: Vec<u32>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Inputs {
    pub controller: Object,
    pub player: Object,
    pub player_index: i32,
    pub kingdoms: Vec<Option<Kingdom>>,
    pub own_slot: usize,
    /// Actual registry occupancy and generation-bearing IDs at initialization.
    pub registered_ids: Vec<u32>,
    /// Original profile+464/+528/+5EC.
    pub ability_counts: [i32; 3],
    /// Evaluated game definition+570/+28 count.
    pub script_count: usize,
    pub neighbor_depth: i32,
    /// Actual Kingdom+230; None means the original pointer is null.
    pub recruitment: Option<Recruitment>,
    /// Actual Kingdom+1FC/+18 RecruitSingle allocation list, in source order.
    /// These object identities are recruitment items, not Actor GTIDs.
    pub owned_units: Vec<Option<Object>>,
}
/// Stable allocation/registry identities read by the fresh initializer.
/// Source-owned scalar and container getters remain separate so an unbound
/// later child cannot erase the actual controller/goal publication prefix.
pub struct Identities {
    pub controller:Object,
    pub player:Object,
    pub player_index:i32,
    pub kingdoms:Vec<Option<Kingdom>>,
    pub own_slot:usize,
    pub registered_ids:Vec<u32>,
}
pub trait StartupData {
    fn identities(&mut self)->Result<Identities>;
    fn script_count(&mut self)->Result<usize>;
    fn ability_counts(&mut self)->Result<[i32;3]>;
    fn recruitment(&mut self)->Result<Option<Recruitment>>;
    fn owned_units(&mut self)->Result<Vec<Option<Object>>>;
    fn neighbor_depth(&mut self)->Result<i32>;
}
impl StartupData for Inputs {
    fn identities(&mut self)->Result<Identities> {Ok(Identities {controller:self.controller,player:self.player,
        player_index:self.player_index,kingdoms:self.kingdoms.clone(),own_slot:self.own_slot,registered_ids:self.registered_ids.clone()})}
    fn script_count(&mut self)->Result<usize> {Ok(self.script_count)}
    fn ability_counts(&mut self)->Result<[i32;3]> {Ok(self.ability_counts)}
    fn recruitment(&mut self)->Result<Option<Recruitment>> {Ok(self.recruitment.clone())}
    fn owned_units(&mut self)->Result<Vec<Option<Object>>> {Ok(self.owned_units.clone())}
    fn neighbor_depth(&mut self)->Result<i32> {Ok(self.neighbor_depth)}
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Kingdom,
    LandRegion,
    WaterRegion,
    RegionAction,
    Ability,
    Recruitment,
    UnitPrimary,
    UnitSecondary,
}
impl Kind {
    fn bucket(self) -> usize {
        match self {
            Self::Kingdom => 14,
            Self::LandRegion => 0,
            Self::WaterRegion => 2,
            Self::RegionAction => 9,
            Self::Ability => 7,
            Self::Recruitment => 21,
            Self::UnitPrimary => 17,
            Self::UnitSecondary => 18,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GoalData {
    Kingdom {
        target_id: u32,
        words40_44: [u32; 2],
    },
    Region {
        words3c_44: [u32; 3],
        region: usize,
        target_id: u32,
        neighbors: Vec<usize>,
        hash_buckets: usize,
        hash_count: u32,
        hash_flag: bool,
        byte6c: bool,
    },
    RegionAction {
        word3c: u32,
        region: usize,
        word44: u32,
        scalars48_4c: [u32; 2],
        byte50: bool,
    },
    Ability {
        category: u32,
        index: u32,
        words44_54: [u32; 5],
    },
    Recruitment {
        options: Vec<Object>,
        selected: Vec<u8>,
        weights: Vec<[u32; 2]>,
        assignments: Vec<u32>,
    },
    Unit {
        /// Native RecruitSingle allocation referenced by7A47C0.
        unit: Object,
        words40_44: Vec<u32>,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Goal {
    pub base: GoalBase,
    pub kind: Kind,
    pub data: GoalData,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionKingdomRef {
    pub region: usize,
    pub kingdom: Object,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionPlayer {
    pub region: usize,
    pub player: Object,
    pub land: usize,
    pub water: usize,
    pub word34: u32,
    /// Native arrays+8/+14/+20/+38, stored counts are zero after7BA130.
    pub arrays: [Vec<Object>; 4],
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Player {
    pub ready: bool,
    pub script_words: Vec<u32>,
    pub word10: u32,
    pub word14: u32,
    /// Its group Object keys are local indices into goals (not global actors).
    pub controller: Controller,
    /// Local goal indices are stable owned identities within this controller.
    pub goals: Vec<Goal>,
    pub region_kingdoms: Vec<Option<RegionKingdomRef>>,
    pub region_players: Vec<RegionPlayer>,
}
/// Sole mutable7B4830 destination, created by the native7B1080 stage.
/// Controller/goals become visible here at their actual publication points;
/// an error never discards them or copies a success-only snapshot back.
#[derive(Debug)]
pub struct Initializing {
    pub ready: bool,
    pub script_words: Vec<u32>,
    pub word10: u32,
    pub word14: u32,
    pub controller: Option<Controller>,
    pub goals: Vec<Goal>,
    /// Generic native pending-goal links, head first. Payloads live in goals.
    /// Controller.pending_kingdom_goals remains the legacy narrow projection;
    /// this full initializer does not use that second payload representation.
    pub pending: Vec<usize>,
    pub region_kingdoms: Vec<Option<RegionKingdomRef>>,
    pub region_players: Vec<RegionPlayer>,
    attempted: bool,
}
impl Initializing {
    pub fn from_native_constructor() -> Result<Self> {
        let mut script_words=Vec::new();script_words.try_reserve_exact(1)?;
        let mut region_kingdoms=Vec::new();region_kingdoms.try_reserve_exact(4096)?;
        Ok(Self {ready:false,script_words,word10:0,word14:0,controller:None,
            goals:Vec::new(),pending:Vec::new(),region_kingdoms,region_players:Vec::new(),attempted:false})
    }
    pub fn attempted(&self)->bool {self.attempted}
    fn into_player(self)->Result<Player> {
        if !self.ready {return Err(Error::Invalid("AI startup is incomplete"));}
        Ok(Player {ready:self.ready,script_words:self.script_words,word10:self.word10,word14:self.word14,
            controller:self.controller.ok_or(Error::Invalid("AI controller is absent"))?,
            goals:self.goals,region_kingdoms:self.region_kingdoms,region_players:self.region_players})
    }
}
fn zeros<T: Clone>(count: usize, value: T) -> Result<Vec<T>> {
    let mut out = Vec::new();
    out.try_reserve_exact(count)?;
    out.resize(count, value);
    Ok(out)
}
fn push_goal(
    goals: &mut Vec<Goal>,
    pending: &mut Vec<usize>,
    controller: Object,
    kind: Kind,
    data: GoalData,
) -> Result<usize> {
    goals.try_reserve(1)?;
    let index = goals.len();
    goals.push(Goal {
        base: GoalBase::new(controller),
        kind,
        data,
    });
    pending.try_reserve(1)?;
    pending.insert(0,index);
    Ok(index)
}
/// Exact ordered7BB1C0 output, then7C8F70's head insertion reverses it.
fn neighboring_regions(graph: &RegionGraph, start: usize, depth: i32) -> Result<Vec<usize>> {
    let mut distance = zeros(graph.nodes.len(), -1i32)?;
    distance[start] = 0;
    let mut queue = VecDeque::new();
    queue.try_reserve(1)?;
    queue.push_back(start);
    let mut out = Vec::new();
    while let Some(id) = queue.pop_front() {
        if graph.nodes[id].category != graph.nodes[start].category {
            continue;
        }
        out.try_reserve(1)?;
        out.push(id);
        if distance[id] < depth {
            for &neighbor in graph.nodes[id].neighbors.iter().rev() {
                let other = neighbor as usize;
                if other >= distance.len() {
                    return Err(Error::Invalid("AI neighbor identity missing"));
                }
                if distance[other] < 0 {
                    distance[other] = distance[id].wrapping_add(1);
                    queue.try_reserve(1)?;
                    queue.push_back(other);
                }
            }
        }
    }
    out.reverse();
    Ok(out)
}
/// Fresh unpublished AIPlayer candidate. Actual7B4830 does not have an early
/// ready guard: this API must not be used to replace an existing live player.
/// It performs every nondiagnostic initialization step for supplied live data,
/// including concrete7A4F00 goal filtering/ordering and7C8F70 neighbor links.
pub fn initialize(input:&Inputs,graph:&RegionGraph,region_state:&sai_state::State)->Result<Player> {
    let mut destination=Initializing::from_native_constructor()?;
    initialize_in_place(input,graph,region_state,&mut destination)?;
    destination.into_player()
}
/// Same algorithm on the actual constructor destination; no rollback on error.
pub fn initialize_in_place(
    input: &Inputs,
    graph: &RegionGraph,
    region_state: &sai_state::State,
    state: &mut Initializing,
) -> Result<()> {
    initialize_from_source(&mut input.clone(),graph,region_state,state)
}
/// Same common algorithm with fallible getters at the reached native stages.
pub fn initialize_from_source(
    source:&mut impl StartupData,
    graph:&RegionGraph,
    region_state:&sai_state::State,
    state:&mut Initializing,
)->Result<()> {
    if state.attempted || state.controller.is_some() || state.ready {
        return Err(Error::Invalid("AI startup destination was already attempted"));
    }
    state.attempted=true;
    let input=source.identities()?;
    if input.player_index < 0 {
        return Err(Error::Invalid("negative AI player slot"));
    }
    // Fresh region-goal constructors pass GTID0. Its native registry slot must
    // be empty; treating an actual live slot0 as null would change7C6920.
    if input.registered_ids.iter().any(|&id| id & 0xffff == 0) {
        return Err(Error::Invalid("native registry null slot is occupied"));
    }
    let own = input
        .kingdoms
        .get(input.own_slot)
        .copied()
        .flatten()
        .ok_or(Error::Invalid("AI player has no live Kingdom"))?;
    if region_state.regions.len() != graph.nodes.len() {
        return Err(Error::Invalid("region ownership count mismatch"));
    }
    let script_count=source.script_count()?;
    state.script_words.try_reserve(script_count.saturating_sub(state.script_words.len()))?;
    state.script_words.resize(script_count,0);
    state.controller=Some(Controller::new(input.controller,input.player)?);
    state.word10=0;state.word14=0;
    let Initializing {goals,pending,region_kingdoms,region_players,controller:Some(controller),..}=state else {unreachable!()};
    let mut goals=goals;let mut pending=pending;
    // Native7B4972 intentionally excludes the final major-list element.
    for kingdom in input
        .kingdoms
        .iter()
        .take(input.kingdoms.len().saturating_sub(1))
        .flatten()
    {
        if kingdom.category == 0 && kingdom.object != own.object {
            push_goal(
                &mut goals,
                &mut pending,
                input.controller,
                Kind::Kingdom,
                GoalData::Kingdom {
                    target_id: kingdom.native_id,
                    words40_44: [0; 2],
                },
            )?;
        }
    }
    region_kingdoms.try_reserve(graph.nodes.len().saturating_sub(region_kingdoms.len()))?;
    region_players.try_reserve_exact(graph.nodes.len())?;
    for region in 0..graph.nodes.len() {
        region_kingdoms.push(region_state.regions[region].kingdom(own)?.map(|r| {
            RegionKingdomRef {
                region,
                kingdom: r.kingdom,
            }
        }));
        let data = || GoalData::Region {
            words3c_44: [0; 3],
            region,
            target_id: 0,
            neighbors: Vec::new(),
            hash_buckets: 37,
            hash_count: 0,
            hash_flag: true,
            byte6c: false,
        };
        let land = push_goal(&mut goals, &mut pending, input.controller, Kind::LandRegion, data())?;
        let water = push_goal(&mut goals, &mut pending, input.controller, Kind::WaterRegion, data())?;
        push_goal(
            &mut goals,
            &mut pending,
            input.controller,
            Kind::RegionAction,
            GoalData::RegionAction {
                word3c: 0,
                region,
                word44: 0,
                scalars48_4c: [0xbf80_0000; 2],
                byte50: false,
            },
        )?;
        region_players.push(RegionPlayer {
            region,
            player: input.player,
            land,
            water,
            word34: 0,
            arrays: std::array::from_fn(|_| Vec::new()),
        });
    }
    // Fresh7A4970 has no previously active category7 goals to invalidate.
    let ability_counts=source.ability_counts()?;
    for category in 0..3 {
        for index in 0..ability_counts[category] {
            push_goal(
                &mut goals,
                &mut pending,
                input.controller,
                Kind::Ability,
                GoalData::Ability {
                    category: category as u32,
                    index: index as u32,
                    words44_54: [0; 5],
                },
            )?;
        }
    }
    let mut options = Vec::new();
    if let Some(recruitment) = &source.recruitment()? {
        for option in recruitment.options.iter().flatten() {
            if let Some(kind) = option.definition_type {
                let enabled = *recruitment
                    .enabled_types
                    .get(kind)
                    .ok_or(Error::Invalid("recruitment type exceeds profile table"))?;
                if enabled != 0 {
                    options.try_reserve(1)?;
                    options.push(option.object);
                }
            }
        }
    }
    let count = options.len();
    let mut selected = zeros(count, 0)?;
    if count > 1 {
        selected[0] = 1;
    }
    push_goal(
        &mut goals,
        &mut pending,
        input.controller,
        Kind::Recruitment,
        GoalData::Recruitment {
            options,
            selected,
            weights: zeros(count, [0xbf80_0000; 2])?,
            assignments: zeros(count, 0)?,
        },
    )?;
    // Actual4E13F0 iterates the Kingdom list backwards, retaining null holes.
    for &unit in source.owned_units()?.iter().rev().flatten() {
        push_goal(
            &mut goals,
            &mut pending,
            input.controller,
            Kind::UnitPrimary,
            GoalData::Unit {
                unit,
                words40_44: zeros(1, 0)?,
            },
        )?;
        push_goal(
            &mut goals,
            &mut pending,
            input.controller,
            Kind::UnitSecondary,
            GoalData::Unit {
                unit,
                words40_44: zeros(2, 0)?,
            },
        )?;
    }
    //7A4F00 consumes the head-inserted pending list, checking actual Kingdom
    //generation-bearing IDs. Constructors leave all other delete flags clear.
    let mut index_map = zeros(goals.len(), None)?;
    let mut original_index=0;let mut retained_index=0;
    goals.retain(|goal| {
        let keep=match &goal.data {GoalData::Kingdom {target_id,..}=>input.registered_ids.contains(target_id),_=>true};
        if keep {index_map[original_index]=Some(retained_index);retained_index+=1;}
        original_index+=1;keep
    });
    pending.retain_mut(|index|match index_map[*index] {Some(mapped)=>{*index=mapped;true},None=>false});
    for region in region_players.iter_mut() {
        region.land=index_map[region.land].ok_or(Error::Invalid("land goal removed during startup"))?;
        region.water=index_map[region.water].ok_or(Error::Invalid("water goal removed during startup"))?;
    }
    // Consume the actual pending head order and prepend each group link.
    // Payloads stay in the destination even if a later allocation/query fails.
    while let Some(&index)=pending.first() {
        let goal=&goals[index];
        let target=&mut controller.groups[goal.kind.bucket()][goal.base.status as usize];
        target.try_reserve(1)?;target.insert(0,Object(index as u64));pending.remove(0);
    }
    let mut neighbor_depth=None;
    for goal in goals.iter_mut() {
        if let GoalData::Region {
            region, neighbors, ..
        } = &mut goal.data
        {
            let depth=match neighbor_depth {Some(value)=>value,None=>{let value=source.neighbor_depth()?;neighbor_depth=Some(value);value}};
            let order = neighboring_regions(graph, *region, depth)?;
            neighbors.try_reserve_exact(order.len())?;
            for id in order {
                neighbors.push(if goal.kind == Kind::LandRegion {
                    region_players[id].land
                } else {
                    region_players[id].water
                });
            }
        }
    }
    state.ready=true;
    Ok(())
}
