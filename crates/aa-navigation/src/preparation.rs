//! Native7ABC30 graph preparation for a newly loaded topology.
//! This prepares graph data;7A0D40's world readiness remains a separate lifecycle.
use crate::{region_search::RegionGraph, topology::Topology};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Topology(#[from] crate::topology::Error),
    #[error(transparent)]
    Allocation(#[from] std::collections::TryReserveError),
    #[error("invalid native region graph")]
    Graph,
}
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prepared {
    /// Manager+5, distinct from the SAI engine's ready byte+20.
    pub prepared: bool,
    pub graph: RegionGraph,
    pub areas: Vec<i32>,
    pub bottlenecks: Vec<bool>,
    ///53F0A0 initializes row i with N-i-1 FF bytes. These are unknown cached
    ///routes, not successful path results. Flag1 route computation is separate.
    pub route_cache: Vec<Vec<u8>>,
}

///7BB1C0 finite breadth traversal. Native visits neighbors in reverse adjacency
///order, marks on enqueue, and does not expand the excluded region or a foreign
///island. The result records membership; native linked-list ownership is local.
fn reachable(graph: &RegionGraph, start: usize, excluded: usize, depth: i32) -> Result<Vec<bool>> {
    let mut distance = Vec::new();
    distance.try_reserve_exact(graph.nodes.len())?;
    distance.resize(graph.nodes.len(), -1i32);
    let mut included = Vec::new();
    included.try_reserve_exact(graph.nodes.len())?;
    included.resize(graph.nodes.len(), false);
    let mut queue = VecDeque::new();
    queue.try_reserve(1)?;
    queue.push_back(start);
    distance[start] = 0;
    while let Some(id) = queue.pop_front() {
        if id == excluded || graph.nodes[id].category != graph.nodes[start].category {
            continue;
        }
        included[id] = true;
        if distance[id] < depth {
            for &neighbor in graph.nodes[id].neighbors.iter().rev() {
                let other = neighbor as usize;
                if distance[other] < 0 {
                    distance[other] = distance[id].wrapping_add(1);
                    queue.try_reserve(1)?;
                    queue.push_back(other);
                }
            }
        }
    }
    Ok(included)
}

///Native7BC8B0 sets the flag when two same-island neighbors are disconnected
///within the supplied detection depth with self removed. Existing flags stay set.
pub fn mark_bottlenecks(graph: &RegionGraph, flags: &mut [bool], depth: i32) -> Result<()> {
    if flags.len() != graph.nodes.len()
        || graph.nodes.iter().any(|n| {
            n.neighbors.len() > 255 || n.neighbors.iter().any(|&i| i as usize >= graph.nodes.len())
        })
    {
        return Err(Error::Graph);
    }
    for region in (0..graph.nodes.len()).rev() {
        let node = &graph.nodes[region];
        'neighbor: for i in (0..node.neighbors.len()).rev() {
            let neighbor = node.neighbors[i] as usize;
            if graph.nodes[neighbor].category != node.category {
                continue;
            }
            let included = reachable(graph, neighbor, region, depth)?;
            for &other in node.neighbors[..i].iter().rev() {
                if graph.nodes[other as usize].category == node.category
                    && !included[other as usize]
                {
                    flags[region] = true;
                    break 'neighbor;
                }
            }
        }
    }
    Ok(())
}

///Fresh loaded-world form of7ABC30: adjacency, areas, bottlenecks and empty
///triangular route cache. Caller resolves StaticMapBottleneckDetectionDepth.
///Returns an unpublished owned candidate; it does not clear an existing world's
///native pointers or authorize gameplay after a failed allocation.
pub fn prepare(topology: &Topology, bottleneck_depth: i32) -> Result<Prepared> {
    let graph = topology.prepare_graph()?;
    let count = graph.nodes.len();
    let mut areas = Vec::new();
    areas.try_reserve_exact(count)?;
    for id in 0..count {
        areas.push(topology.region_area(id)?);
    }
    let mut bottlenecks = Vec::new();
    bottlenecks.try_reserve_exact(count)?;
    bottlenecks.resize(count, false);
    mark_bottlenecks(&graph, &mut bottlenecks, bottleneck_depth)?;
    let mut route_cache = Vec::new();
    route_cache.try_reserve_exact(count.saturating_sub(1))?;
    for i in 0..count.saturating_sub(1) {
        let mut row = Vec::new();
        row.try_reserve_exact(count - i - 1)?;
        row.resize(count - i - 1, 255);
        route_cache.push(row);
    }
    Ok(Prepared {
        prepared: true,
        graph,
        areas,
        bottlenecks,
        route_cache,
    })
}
