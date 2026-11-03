//! What a created thing does to the ground (`aa_play::ground`, 0x4EB500) against the
//! original: the FNV-1a hash of the whole HEIGHT_MAP, word by word, after every height
//! brush of a film of (2v2)Kursk, as a read-only sampler took it from the running game
//! at each brush's entry and after the update. Four starting buildings before the session
//! starts, then the unpacks of the film. Reads the original data; set AA_GAME_ROOT.
use aa_play::GameData;
use aa_play::ground::flatten_terrain;
use aa_play::map::GameMap;
use aa_spatial::collision::Shape;
use aa_spatial::geometry::Placement;
use std::path::PathBuf;

fn hash(words: &[f32]) -> u64 {
    let mut h = 14695981039346656037u64;
    for v in words {
        h ^= u64::from(v.to_bits());
        h = h.wrapping_mul(1099511628211);
    }
    h
}

#[test]
#[ignore = "reads the original data; set AA_GAME_ROOT"]
fn kursk_base_map_follows_the_original_brush_by_brush() {
    let root = PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"));
    let data = GameData::load(&root).unwrap();
    let mut map = GameMap::load_with(&root, "(2v2)Kursk", &data).unwrap();
    let map_placed = map.placed.clone();
    let reference = GameMap { native_terrain: None, ..map.clone() };
    let terrain = map.native_terrain.as_mut().expect("height layers");
    assert_eq!(hash(&terrain.base), 0x0ebb_29e3_299c_52ad, "the map as it loads");
    // Thing, position, whether the session has started, the base map's hash after it, the
    // doodads it cleared (their GTIDs).
    let chain: [(&str, [f32; 2], bool, u64, &[u32]); 10] = [
        ("rus_corps", [385.0, 395.0], false, 0x7988_433d_27b9_5c37, &[]),
        ("ger_corps", [299.0, 430.0], false, 0x6058_ba2a_eaf3_6816, &[]),
        ("usa_corps", [19.0, 132.0], false, 0xd58c_261c_640f_f22e, &[]),
        ("jpn_corps", [87.0, 37.0], false, 0x0ecc_681f_35d4_4798, &[]),
        ("rus_hq_infantry", [377.0, 420.0], true, 0xcf9a_9d9f_9bcf_06b8, &[0x261b, 0x261c, 0x301e]),
        ("jpn_hq_infantry", [80.0, 13.0], true, 0x4f89_7afb_0d90_a2bd, &[0x2a66, 0x2a5e, 0x2ff2, 0x2a6b, 0x2a5f, 0x2a60, 0x2a63, 0x2a61]),
        ("usa_hq_infantry", [38.0, 109.0], true, 0x4f89_7afb_0d90_a2bd, &[0x3740, 0x3741, 0x3742, 0x3745, 0x3746, 0x37cb, 0x3747, 0x3748]),
        ("rus_depot_ammo", [372.0, 431.0], true, 0x4fd6_2a8e_6a06_fdac, &[]),
        ("jpn_hq_airborne", [92.0, 67.0], true, 0xf8e6_0223_8529_556f, &[0x3566, 0x3564, 0x3578, 0x3565, 0x3579]),
        ("usa_hq_airborne", [15.0, 119.0], true, 0x870b_4b6b_2dbe_ef15, &[]),
    ];
    let scenery: Vec<usize> = (0..map_placed.len()).collect();
    let slope = data.rule_ci("ElevationSlopeMax").expect("ElevationSlopeMax");
    assert_eq!(slope, 0.5);
    for (id, pos, started, want, cleared) in chain {
        let def = data.def(data.find(id).unwrap_or_else(|| panic!("{id}")));
        assert!(def.thing_flags & 0x2000_2400 == 0x2000_2400, "{id}: flags {:x}", def.thing_flags);
        let w = def.blocking_words;
        let place = Placement {
            shape: Shape { kind: w[0], radius_bits: w[1], bounds: [w[2], w[3], w[4], w[5]] },
            facing: [0, 0xbf80_0000],
            position: [pos[0].to_bits(), pos[1].to_bits()],
        };
        flatten_terrain(terrain, &place, slope, started);
        assert_eq!(hash(&terrain.base), want, "after {id} at {pos:?}");
        let mut got: Vec<u32> = aa_play::ground::doodads_under(&reference, &data, &scenery, &place).iter().map(|&i| map_placed[i].serialized_id).collect();
        let mut expected = cleared.to_vec();
        got.sort_unstable();
        expected.sort_unstable();
        assert_eq!(got, expected, "the doodads {id} at {pos:?} cleared");
    }
    // The drawn terrain is told which samples changed, per brush that changed any.
    assert!(terrain.changes.iter().all(|r| r[0] <= r[2] && r[1] <= r[3]));
}
