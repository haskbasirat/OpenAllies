//! `map_analysis::SaiMap` from the movement agent's region topology
//! (`aa_navigation::topology::Topology`).
//!
//! Each SAI map region (`7BA4F0`, 0x78 bytes) keeps its neighbour distances
//! in a native hash at `+5C` (11 buckets, auto-grow): `7BA3C0` pushes at the
//! chain head and, before inserting at `count >= 2n`, rehashes (`606750`).
//! - A freshly analysed map (`7ACB00`, `topology_builder::build`): `7BC590`
//!   inserts each neighbour once; the builder's `distance_pairs` are that
//!   hash flattened in bucket order. The bucket count after `k` inserts is
//!   fixed by the growth rule ([`distance_hash_buckets`]), so the buckets are
//!   recovered exactly by grouping the pairs by `key % n` in order
//!   ([`from_static_analysis`]).
//! - A loaded map (`7AA9B0` -> region loader `7BBAF0`): every stored pair is
//!   inserted in stream order with the same rule, duplicates included
//!   ([`from_loaded_topology`]).
//!
//! The terrain of a region (`7B9320` on its first rectangle, `7A9AC0`) is the
//! terrain type of the rectangle's (min X, min Y) tile ([`region_terrain`]).
//! Region player entries (`+1C/+20`) and the actor lists (`+28`, `+34`) start
//! empty; SAI players add their entries and the simulation fills the lists.
use super::map_analysis::{MapRegion, SaiMap};
use super::native_hash::NativeHash;
use crate::{Error, Result};
use aa_navigation::topology::{StaticTile, Topology};
use aa_navigation::topology_builder::{StaticAnalysis, terrain_type};

/// Initial bucket count of a region distance hash (`7BA4F0` `+60`).
pub const REGION_HASH_BUCKETS: u32 = 11;

/// `606750` bucket growth: `2n + 1`, then `+2` while a multiple of 3 or 5
/// (at most twice).
fn grow(n: u32) -> u32 {
    let skip = |m: u32| m.is_multiple_of(3) || m.is_multiple_of(5);
    let mut m = n.wrapping_mul(2).wrapping_add(1);
    if skip(m) {
        m = m.wrapping_add(2);
        if skip(m) {
            m = m.wrapping_add(2);
        }
    }
    m
}

/// Bucket count of a region distance hash after `inserts` `7BA3C0` inserts.
pub fn distance_hash_buckets(inserts: usize) -> u32 {
    let mut n = REGION_HASH_BUCKETS;
    for count in 0..inserts {
        if count as u64 >= 2 * u64::from(n) {
            n = grow(n);
        }
    }
    n
}

/// The hash of `pairs` flattened in bucket order (as `topology_builder`
/// reports them), rebuilt with its native bucket count.
pub fn hash_from_bucket_order(pairs: &[[u32; 2]]) -> Result<NativeHash<u32>> {
    let n = distance_hash_buckets(pairs.len());
    let mut hash = NativeHash::new(n);
    let mut last = 0_usize;
    for &[key, value] in pairs {
        let b = (key % n) as usize;
        if b < last {
            return Err(Error::World(
                "region distance pairs are not in native bucket order".into(),
            ));
        }
        last = b;
        hash.buckets[b].push((key, value));
        hash.count += 1;
    }
    Ok(hash)
}

/// The hash a loaded region builds from its stored pairs (`7BBAF0`).
pub fn hash_from_stream(pairs: &[[u32; 2]]) -> Result<NativeHash<u32>> {
    let mut hash = NativeHash::new(REGION_HASH_BUCKETS);
    for &[key, value] in pairs {
        hash.insert(key, value)?;
    }
    Ok(hash)
}

fn regions(topology: &Topology, neighbors: Vec<NativeHash<u32>>) -> SaiMap {
    SaiMap {
        analysed: topology.enabled,
        regions: neighbors
            .into_iter()
            .enumerate()
            .map(|(index, neighbors)| MapRegion {
                index: index as u32,
                neighbors,
                entries: Vec::new(),
                static_lists: Vec::new(),
                mobile_lists: Vec::new(),
            })
            .collect(),
    }
}

/// The SAI map of a freshly analysed topology (`7ACB00`).
pub fn from_static_analysis(analysis: &StaticAnalysis) -> Result<SaiMap> {
    let topology = &analysis.topology;
    let hashes = topology
        .regions
        .iter()
        .map(|r| hash_from_bucket_order(&r.distance_pairs))
        .collect::<Result<Vec<_>>>()?;
    Ok(regions(topology, hashes))
}

/// The SAI map of a loaded topology (`7AA9B0`).
pub fn from_loaded_topology(topology: &Topology) -> Result<SaiMap> {
    let hashes = topology
        .regions
        .iter()
        .map(|r| hash_from_stream(&r.distance_pairs))
        .collect::<Result<Vec<_>>>()?;
    Ok(regions(topology, hashes))
}

/// `7B9320` on the region's first rectangle: `7A9AC0` of the tile at the
/// rectangle's (min X `+12`, min Y `+C`).
pub fn region_terrain(topology: &Topology, tiles: &[StaticTile], region: u32) -> Result<u32> {
    let first = topology
        .region_rectangles
        .get(region as usize)
        .and_then(|list| list.first())
        .ok_or_else(|| Error::World(format!("region {region} has no first rectangle")))?;
    let rect = topology
        .rectangles
        .get(*first as usize)
        .ok_or_else(|| Error::World(format!("rectangle {first} missing")))?;
    let (x, y) = (i32::from(rect.bounds[3]), i32::from(rect.bounds[0]));
    let index = topology
        .layout
        .width
        .wrapping_mul(y)
        .wrapping_add(x);
    let tile = usize::try_from(index)
        .ok()
        .and_then(|i| tiles.get(i))
        .ok_or_else(|| Error::World(format!("tile ({x},{y}) outside the grid")))?;
    Ok(terrain_type(tile))
}
