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
    /// Actual Kingdom+1FC/+18 list in source order, including empty entries.
    pub owned_units: Vec<Option<Object>>,
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
fn zeros<T: Clone>(count: usize, value: T) -> Result<Vec<T>> {
    let mut out = Vec::new();
    out.try_reserve_exact(count)?;
    out.resize(count, value);
    Ok(out)
}
fn push_goal(
    goals: &mut Vec<Goal>,
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
pub fn initialize(
    input: &Inputs,
    graph: &RegionGraph,
    region_state: &sai_state::State,
) -> Result<Player> {
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
    let script_words = zeros(input.script_count, 0)?;
    let mut controller = Controller::new(input.controller, input.player)?;
    let mut goals = Vec::new();
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
                input.controller,
                Kind::Kingdom,
                GoalData::Kingdom {
                    target_id: kingdom.native_id,
                    words40_44: [0; 2],
                },
            )?;
        }
    }
    let mut region_kingdoms = Vec::new();
    region_kingdoms.try_reserve_exact(graph.nodes.len())?;
    let mut region_players = Vec::new();
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
        let land = push_goal(&mut goals, input.controller, Kind::LandRegion, data())?;
        let water = push_goal(&mut goals, input.controller, Kind::WaterRegion, data())?;
        push_goal(
            &mut goals,
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
    for category in 0..3 {
        for index in 0..input.ability_counts[category] {
            push_goal(
                &mut goals,
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
    if let Some(recruitment) = &input.recruitment {
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
    for &unit in input.owned_units.iter().rev().flatten() {
        push_goal(
            &mut goals,
            input.controller,
            Kind::UnitPrimary,
            GoalData::Unit {
                unit,
                words40_44: zeros(1, 0)?,
            },
        )?;
        push_goal(
            &mut goals,
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
    let mut retained = Vec::new();
    retained.try_reserve_exact(goals.len())?;
    for (i, goal) in goals.into_iter().enumerate() {
        if let GoalData::Kingdom { target_id, .. } = goal.data
            && !input.registered_ids.contains(&target_id)
        {
            continue;
        }
        index_map[i] = Some(retained.len());
        retained.push(goal);
    }
    let mut groups: Vec<[Vec<usize>; 3]> = Vec::new();
    groups.try_reserve_exact(23)?;
    groups.resize_with(23, || std::array::from_fn(|_| Vec::new()));
    // Reverse pending then prepend to each category restores creation order.
    for (i, goal) in retained.iter().enumerate() {
        let group = &mut groups[goal.kind.bucket()][goal.base.status as usize];
        group.try_reserve(1)?;
        group.push(i);
    }
    for region in &mut region_players {
        region.land =
            index_map[region.land].ok_or(Error::Invalid("land goal removed during startup"))?;
        region.water =
            index_map[region.water].ok_or(Error::Invalid("water goal removed during startup"))?;
    }
    for goal in &mut retained {
        if let GoalData::Region {
            region, neighbors, ..
        } = &mut goal.data
        {
            let order = neighboring_regions(graph, *region, input.neighbor_depth)?;
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
    for (kind, categories) in groups.iter().enumerate() {
        for (category, goals) in categories.iter().enumerate() {
            let target = &mut controller.groups[kind][category];
            target.try_reserve_exact(goals.len())?;
            target.extend(goals.iter().map(|&index| Object(index as u64)));
        }
    }
    Ok(Player {
        ready: true,
        script_words,
        word10: 0,
        word14: 0,
        controller,
        goals: retained,
        region_kingdoms,
        region_players,
    })
}
