//! SaiMap construction from the region topology: the bucket-order
//! reconstruction is checked against the native hash insert (`7BA3C0` +
//! `606750` as `NativeHash::insert`, oracle-proven in sai_ego_native), and a
//! topology built by `aa_navigation::topology_builder` converts completely.
use aa_mind::sai::map_build::{
    distance_hash_buckets, from_loaded_topology, from_static_analysis, hash_from_bucket_order,
    region_terrain,
};
use aa_mind::sai::native_hash::NativeHash;
use aa_navigation::topology::{GridLayout, StaticTile};
use aa_navigation::topology_builder::{StaticGrid, StaticMapParameters, build};
use aa_numeric::Precision;

fn lcg(state: &mut u64) -> u32 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    (*state >> 33) as u32
}

#[test]
fn bucket_order_round_trips_native_hash() {
    let mut s = 0x7bc5_90u64;
    for len in 0..300usize {
        let mut hash = NativeHash::new(11);
        let mut keys = std::collections::BTreeSet::new();
        while keys.len() < len {
            let k = lcg(&mut s) % 5000;
            if keys.insert(k) {
                hash.insert(k, lcg(&mut s)).unwrap();
            }
        }
        assert_eq!(hash.buckets.len() as u32, distance_hash_buckets(len), "len {len}");
        let flat: Vec<[u32; 2]> = hash
            .buckets
            .iter()
            .flatten()
            .map(|&(k, v)| [k, v])
            .collect();
        let rebuilt = hash_from_bucket_order(&flat).unwrap();
        assert_eq!(rebuilt, hash, "len {len}");
    }
}

#[test]
fn builder_topology_converts() {
    let (w, h, shift) = (48, 40, 5u32);
    let mut tiles = Vec::new();
    for y in 0..h {
        for x in 0..w {
            // A lake and a ridge give several islands and terrain types.
            let water = (x - 30) * (x - 30) + (y - 12) * (y - 12) < 40;
            let ridge = x == 15 && y > 5;
            tiles.push(StaticTile {
                flags20: if ridge { 0x100 } else { 0 },
                definition_type: if water { 1 } else { 0 },
            });
        }
    }
    let layout = GridLayout {
        width: w,
        height: h,
        coordinate_shift: shift,
        extent_bits: [((w << shift) as f32).to_bits(), ((h << shift) as f32).to_bits()],
    };
    let grid = StaticGrid {
        layout,
        cell_size_bits: (32.0f32).to_bits(),
        tiles: &tiles,
    };
    let params = StaticMapParameters {
        starting_square_size: 8,
        use_fast_square_reduction: false,
        fast_square_reduction_mult: 1,
        fast_square_reduction_div: 2,
        min_region_size: 6,
        min_region_width: 2,
        max_region_size_for_expansion: 80,
        max_rectangle_width: 12,
    };
    let analysis = build(&grid, &params, Precision::Double53).unwrap();
    let map = from_static_analysis(&analysis).unwrap();
    let n = analysis.topology.regions.len();
    assert!(n > 4, "{n} regions");
    assert_eq!(map.regions.len(), n);
    let mut neighbours = 0;
    for (i, region) in map.regions.iter().enumerate() {
        assert_eq!(region.index as usize, i);
        let pairs = &analysis.topology.regions[i].distance_pairs;
        let flat: Vec<[u32; 2]> = region
            .neighbors
            .buckets
            .iter()
            .flatten()
            .map(|&(k, v)| [k, v])
            .collect();
        assert_eq!(&flat, pairs);
        assert_eq!(region.neighbors.buckets.len() as u32, distance_hash_buckets(pairs.len()));
        for &[k, _] in pairs {
            assert!((k as usize) < n && k as usize != i);
        }
        neighbours += pairs.len();
        let t = region_terrain(&analysis.topology, &tiles, i as u32).unwrap();
        assert!(t <= 2);
    }
    assert!(neighbours > 0);
    // The loaded form inserts the same pairs in stream order.
    let loaded = from_loaded_topology(&analysis.topology).unwrap();
    for (a, b) in loaded.regions.iter().zip(&map.regions) {
        let mut x: Vec<_> = a.neighbors.buckets.iter().flatten().copied().collect();
        let mut y: Vec<_> = b.neighbors.buckets.iter().flatten().copied().collect();
        x.sort_unstable();
        y.sort_unstable();
        assert_eq!(x, y);
    }
}
