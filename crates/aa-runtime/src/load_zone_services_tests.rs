use super::*;
use serde_json::Value;

#[test]
fn native_initial_tile_and_world_mode_gate_rows() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-load-zone-services/native-initial-zones.json"
    ))
    .unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 6);
    let registration = fixture["registration"].as_array().unwrap();
    assert_eq!(registration.len(), 7);
    assert_eq!(registration[0]["name"], "playable");
    assert_eq!(registration[0]["payload"], serde_json::json!([0, 0]));
    let mut rows = 0;
    for case in cases {
        for row in case["mode_rows"].as_array().unwrap() {
            let mode = row["mode"].as_u64().unwrap() as usize;
            let gate = registration[mode]["payload"][0].as_u64().unwrap();
            assert_eq!(row["gate_al"], gate);
            let initial = row["initial_fog"].as_u64().unwrap() as u32;
            let saved = row["saved_explored"].as_u64().unwrap() as u16;
            let expected = if row["terrain_version"].as_u64().unwrap() >= 4 && gate != 0 {
                (initial & 0xffff0000) | u32::from(saved)
            } else {
                initial
            };
            assert_eq!(row["fog_after"], expected);
            rows += 1;
        }
        for row in case["zero_rows"].as_array().unwrap() {
            assert_eq!(row["all_48_byte_words_zero"], true);
            assert_eq!(row["zone_set"], 0);
            assert_eq!(row["fog"], 0);
            assert_eq!(row["supply"], 0);
            assert_eq!(row["after_actual_54FDC0_binding_zone_words_zero"], true);
        }
    }
    assert_eq!(rows, 126);
}

#[test]
fn native_first_kingdom_footprint_and_ineligible_memory_rows() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-load-zone-services/native-current-services.json"
    ))
    .unwrap();
    let rows = fixture["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 30);
    for row in rows {
        let p: Precision = serde_json::from_value(row["precision"].clone()).unwrap();
        let member: aa_spatial::index::Member =
            serde_json::from_value(row["input_member"].clone()).unwrap();
        let grid: aa_spatial::collision::Grid =
            serde_json::from_value(row["grid"].clone()).unwrap();
        let point = aa_spatial::collision::Shape {
            kind: 0,
            radius_bits: 0,
            bounds: [0; 4],
        };
        let placement = aa_spatial::geometry::Placement {
            shape: member.blocking,
            position: member.actor.position,
            facing: member.actor.facing,
        };
        let extents = grid
            .dimensions
            .map(|n| ((n << grid.coordinate_shift) as f32).to_bits());
        let cells =
            aa_spatial::creation_placement::occupancy_tiles(&grid, extents, &point, &placement, p)
                .unwrap();
        let indices = cells
            .into_iter()
            .map(|[x, y]| y as usize * grid.dimensions[0] as usize + x as usize)
            .collect::<Vec<_>>();
        assert_eq!(
            serde_json::json!(indices),
            row["output"]["occupancy_indices"]
        );
        assert_eq!(row["output"]["eligible_al"], 0);
        assert_eq!(row["output"]["memory_count"], 0);
        // Fixture shape is explicitly supplied; actual PointShape construction
        // remains the separate startup producer's responsibility.
        assert_eq!(row["explicit_fixture_point"], serde_json::json!(vec![0; 6]));
    }
    assert!(require_layout(4, LoadKind::Object, true, false).is_err());
    assert!(require_layout(3, LoadKind::Actor, true, false).is_err());
    assert!(require_layout(2, LoadKind::Object, true, false).is_ok());
    for class in [6, 7, 9] {
        assert!(require_layout(class, LoadKind::Object, true, false).is_ok());
        assert!(require_layout(class, LoadKind::Actor, true, true).is_err());
        assert!(require_layout(class, LoadKind::Object, true, true).is_err());
    }
}

#[test]
fn actual_special_object_vtable_null_owner_rows() {
    let f: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-load-placement-children/native-object-dispatch.json"
    ))
    .unwrap();
    let rows = f["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 60);
    let mut count = 0;
    for row in rows {
        if row["input"]["slot"] == 0x108 {
            let class = row["input"]["class"].as_u64().unwrap() as u32;
            assert!(require_layout(class, LoadKind::Object, true, false).is_ok());
            assert_eq!(row["output"], 0);
            count += 1;
        }
    }
    assert_eq!(count, 12);
}

#[test]
#[ignore = "requires original/v14 source assets in AA_GAME_ROOT"]
fn actual_tutorial_map_initial_zone_owner_all_precisions() {
    use crate::{
        battle_terrain::{BattleTerrain, Source as TerrainSource},
        settings::Settings,
    };
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let native = game.join("reconstruction/native-game/runtime-expanded-v14");
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        native,
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let settings = Settings::load_v14(&mut sources, Limits::default()).unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let path = game.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP");
    let archive = aa_mapworld::MapArchive::from_bytes(std::fs::read(path).unwrap()).unwrap();
    let header = MapHeader::decode(&archive.decode_chunk("MAP_HEADER", u64::MAX).unwrap()).unwrap();
    assert_eq!(header.rules.mode, 0);
    for p in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        let source = TerrainSource::bind(&mut sources, &actors, &settings, p).unwrap();
        let terrain = BattleTerrain::load(&archive, &source).unwrap();
        let spatial = LoadSpatial::new(&terrain.bound, p).unwrap();
        let zones = before_first_record(&terrain.bound, spatial.index(), &header).unwrap();
        assert_eq!(zones.tiles.len(), terrain.bound.cells.len());
        assert!(
            zones
                .tiles
                .iter()
                .all(|t| t.set.is_none() && t.fog == 0 && t.supply == 0)
        );
        assert_eq!(
            zones.geometry.extent_bits,
            terrain
                .bound
                .layout
                .extent_i32
                .map(|n| (n as f32).to_bits())
        );
        // Isolated header-domain probe: the same constructed integer grid
        // can come from a fractional creator extent. Native GMap stores ceil.
        let mut fractional = header.clone();
        fractional.creator.world_extents_bits = terrain
            .bound
            .layout
            .extent_i32
            .map(|n| ((n as f32) - 0.25).to_bits());
        assert_eq!(
            before_first_record(&terrain.bound, spatial.index(), &fractional)
                .unwrap()
                .geometry
                .extent_bits,
            zones.geometry.extent_bits
        );
        // Explicit mode0 restoration probe: a retained saved explored mask is
        // not current fog. Only this isolated bound-data copy is modified.
        let mut probe = terrain.bound.clone();
        probe.cells[0].saved_explored_mask = 0xa55a;
        assert_eq!(
            before_first_record(&probe, spatial.index(), &header)
                .unwrap()
                .tiles[0]
                .fog,
            0
        );
        probe.cells[0].flags20 ^= 1;
        assert!(before_first_record(&probe, spatial.index(), &header).is_err());
        let mut wrong_mode = header.clone();
        wrong_mode.rules.mode = 2;
        assert!(before_first_record(&terrain.bound, spatial.index(), &wrong_mode).is_err());
        let mut wrong_grid = header.clone();
        wrong_grid.creator.world_extents_bits[0] = 4f32.to_bits();
        assert!(before_first_record(&terrain.bound, spatial.index(), &wrong_grid).is_err());
        eprintln!(
            "{p:?}: actual tutorial02 {}tile zone factory, mode0 gate, saved-mask and owner/grid guards",
            zones.tiles.len()
        );
    }
}
