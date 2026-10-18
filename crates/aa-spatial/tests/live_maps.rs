//! Composition checks, not a claim that full actors/matches are initialized.
use aa_heightfield::binding::{GridLayout, LoadedHeightfield};
use aa_mapworld::{
    MapArchive, SourceIdentity,
    archive::sha256,
    header::MapHeader,
    terrain::{ResolvedVariation, TerrainChunk, TerrainGridLayout},
};
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    sources::{SourceLimits, V14Sources},
};
use aa_spatial::{
    Cell, TerrainGrid,
    collision::{Actor, Grid, Shape},
    geometry::tile_rectangle,
    height_layers::HeightLayers,
    index::{ActorKey, Member, SpatialIndex, TransformCallbacks},
    index_world::GroundSurface,
};
use serde_json::Value;
use std::path::PathBuf;
/// Read-only original game folder (the one holding AA.exe and `reconstruction/`).
fn game_root() -> PathBuf {
    PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("set AA_GAME_ROOT"))
}

struct ExplicitFixtureCallbacks {
    positions: usize,
    facings: usize,
}
impl TransformCallbacks for ExplicitFixtureCallbacks {
    fn position_changed(&mut self, _: &mut SpatialIndex, _: ActorKey) -> aa_spatial::Result<()> {
        self.positions += 1;
        Ok(())
    }
    fn world_phase(&self) -> u32 {
        6
    }
    fn presentation_available(&self) -> bool {
        false
    }
    fn presentation_position_changed(
        &mut self,
        _: &mut SpatialIndex,
        _: ActorKey,
    ) -> aa_spatial::Result<()> {
        panic!("phase6 skips presentation")
    }
    fn presentation_facing_changed(
        &mut self,
        _: &mut SpatialIndex,
        _: ActorKey,
    ) -> aa_spatial::Result<()> {
        self.facings += 1;
        panic!("phase6 skips presentation")
    }
}
fn shape(words: [u32; 6]) -> Shape {
    Shape {
        kind: words[0],
        radius_bits: words[1],
        bounds: words[2..].try_into().unwrap(),
    }
}

#[test]
#[ignore = "requires original local maps, TGI files and Data.rwd; run with AA_GAME_ROOT and --include-ignored"]
fn source_maps_and_ground_unit_shapes_support_live_membership_and_height_moves() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let game = game_root();
    let data = game.join("reconstruction/assets/original/Data");
    let pin: Value = serde_json::from_str(include_str!(
        "../../../research/phase6-spatial/native-real-map-terrain.json"
    ))
    .unwrap();
    let types_bytes =
        std::fs::read(root.join("research/engine-baseline/terrain-variation-metadata.json"))
            .unwrap();
    let metadata_bytes =
        std::fs::read(root.join("research/phase6-terrain/terrain-render-metadata.json")).unwrap();
    assert_eq!(sha256(&types_bytes), pin["types_metadata_sha256"]);
    assert_eq!(sha256(&metadata_bytes), pin["passability_metadata_sha256"]);
    let types: Value = serde_json::from_slice(&types_bytes).unwrap();
    let metadata: Value = serde_json::from_slice(&metadata_bytes).unwrap();
    let svar = std::fs::read(data.join("Game/SVars.tgi")).unwrap();
    assert_eq!(sha256(&svar), pin["resolution_source"]["sha256"]);
    assert!(
        svar.windows(b"int HeightmapResolutionBits = 1".len())
            .any(|s| s == b"int HeightmapResolutionBits = 1")
    );
    let mut source = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let catalog = ActorCatalog::load_v14(&mut source).unwrap();
    let ids = [
        "usa_infantry",
        "ger_infantry",
        "gbr_infantry",
        "jpn_infantry",
        "rus_infantry",
        "usa_tank_medium",
        "regiment_usa_infantry",
    ];
    let definitions = ids.map(|id| {
        let object = catalog
            .native_object_definition(id, Precision::Double53)
            .unwrap();
        assert_eq!(
            catalog.native_component_mask(id, Some(0)).unwrap().bits & (1 << 12),
            0,
            "fixture requires no Wall"
        );
        (
            object,
            catalog.native_final_flags(id, Precision::Double53).unwrap(),
        )
    });
    let resolve = |id: &str| -> Option<ResolvedVariation> {
        let r = metadata["variants"].get(id)?;
        let native = &types["variations"][r["canonical"].as_str().unwrap()];
        let source =
            SourceIdentity::of(&std::fs::read(data.join(r["path"].as_str().unwrap())).unwrap());
        assert_eq!(source.sha256, r["sha256"]);
        assert_eq!(source.sha256, native["source_sha256"]);
        Some(ResolvedVariation {
            id_utf16: id.encode_utf16().collect(),
            native_type: native["native_type"].as_u64().unwrap() as u32,
            passability_bits: u8::from(r["land"].as_bool().unwrap())
                | u8::from(r["water"].as_bool().unwrap()) << 1,
            source,
        })
    };
    let mut moves = 0;
    for (relative, row) in pin["maps"].as_object().unwrap() {
        let archive = MapArchive::from_bytes(std::fs::read(data.join(relative)).unwrap()).unwrap();
        assert_eq!(archive.source().sha256, row["map_sha256"]);
        let header =
            MapHeader::decode(&archive.decode_chunk("MAP_HEADER", u64::MAX).unwrap()).unwrap();
        for r in header.rules.rules.iter().chain(&header.creator.rules.rules) {
            assert!(
                !["MapGTileResolutionBits", "HeightmapResolutionBits"]
                    .iter()
                    .any(|id| String::from_utf16_lossy(&r.key).eq_ignore_ascii_case(id))
            );
        }
        let chunk =
            TerrainChunk::decode(&archive.decode_chunk("TERRAIN", u64::MAX).unwrap()).unwrap();
        let layout =
            TerrainGridLayout::from_world_extents(header.creator.world_extents_bits, 1).unwrap();
        let palette: Vec<_> = chunk
            .palette_utf16
            .iter()
            .map(|s| resolve(&String::from_utf16(s).unwrap()))
            .collect();
        let biome = String::from_utf16(&chunk.biome_utf16).unwrap();
        let initial = metadata["biome_defaults"][&biome]["variation_id"]
            .as_str()
            .unwrap();
        let bound = chunk
            .bind_with_native_fallback(layout, &palette, resolve(initial).unwrap())
            .unwrap();
        let terrain = TerrainGrid {
            width: layout.width,
            height: layout.height,
            coordinate_shift: layout.coordinate_shift,
            cell_size: layout.cell_size_bits,
            cells: bound
                .cells
                .iter()
                .map(|cell| {
                    let id = cell
                        .resolved_palette_index
                        .map(|i| String::from_utf16(&chunk.palette_utf16[i as usize]).unwrap())
                        .unwrap_or_else(|| initial.into());
                    let r = &metadata["variants"][&id];
                    let n = &types["variations"][r["canonical"].as_str().unwrap()];
                    let kind = n["native_type"].as_i64().unwrap() as i32;
                    Cell {
                        overlay_kind: kind,
                        offset_min: if kind == 1 {
                            (n["noise_min"].as_f64().unwrap() as f32).to_bits()
                        } else {
                            0
                        },
                        offset_max: if kind == 1 {
                            (n["noise_max"].as_f64().unwrap() as f32).to_bits()
                        } else {
                            0
                        },
                    }
                })
                .collect(),
        };
        let height_layout =
            GridLayout::from_world_extents(header.creator.world_extents_bits, 1).unwrap();
        let base = LoadedHeightfield::decode(
            height_layout,
            &archive.decode_chunk("HEIGHT_MAP", u64::MAX).unwrap().bytes,
        )
        .unwrap();
        let overlay = LoadedHeightfield::decode(
            height_layout,
            &archive
                .decode_chunk("MOUNTAIN_HEIGHT_MAP", u64::MAX)
                .unwrap()
                .bytes,
        )
        .unwrap();
        let surface = GroundSurface {
            terrain: &terrain,
            heights: HeightLayers::new(&base.state.field, &overlay.state.field).unwrap(),
        };
        let mut index = SpatialIndex::new(
            Grid {
                dimensions: [layout.width, layout.height],
                coordinate_shift: layout.coordinate_shift,
                cell_size_bits: layout.cell_size_bits,
            },
            bound.cells.iter().map(|c| c.flags20).collect(),
            Precision::Double53,
        )
        .unwrap();
        let mut callbacks = ExplicitFixtureCallbacks {
            positions: 0,
            facings: 0,
        };
        for (i, (definition, flags)) in definitions.iter().enumerate() {
            let key = i as u32 + 1;
            // Prepared fixture poses, not saved actors or a replacement for startup.
            let start = [20.25 + i as f32 * 4., 20.75].map(f32::to_bits);
            index
                .register(Member {
                    key,
                    reference: (1 << 16) | key,
                    actor: Actor {
                        definition: key,
                        flags: *flags & !(1 << 26),
                        position: start,
                        facing: [0, (-1_f32).to_bits()],
                    },
                    height_bits: surface
                        .sample(start, 0x122, Precision::Double53)
                        .unwrap()
                        .store_f32()
                        .unwrap(),
                    facing_z_bits: 0,
                    cached_tile: [0, 0],
                    blocking: shape(definition.blocking.words),
                    soft: shape(definition.soft.words),
                    target: shape(definition.target.words),
                    actor_category: true,
                    wall_present: false,
                })
                .unwrap();
            index.insert(key).unwrap();
        }
        for (i, (_, flags)) in definitions.iter().enumerate() {
            let key = i as u32 + 1;
            let before = index.member(key).unwrap().actor.position;
            let expected_height = surface
                .sample(
                    before,
                    if flags & (1 << 21) != 0 { 0x123 } else { 0x122 },
                    Precision::Double53,
                )
                .unwrap()
                .store_f32()
                .unwrap();
            let candidate = [40.25 + i as f32 * 4., 30.75].map(f32::to_bits);
            index
                .move_ground(key, candidate, &surface, &mut callbacks)
                .unwrap();
            assert_eq!(index.member(key).unwrap().height_bits, expected_height);
            index
                .set_facing(key, [3_f32.to_bits(), 4_f32.to_bits()], &mut callbacks)
                .unwrap();
            let rect = tile_rectangle(
                index.grid(),
                &index.member(key).unwrap().placement(false),
                Precision::Double53,
            )
            .unwrap();
            for y in 0..layout.height {
                for x in 0..layout.width {
                    let occurrences = index
                        .actors_at([x, y])
                        .unwrap()
                        .into_iter()
                        .filter(|k| *k == key)
                        .count();
                    assert_eq!(
                        occurrences,
                        usize::from(rect.contains([x, y])),
                        "{relative}: actor{key} cell{x},{y}"
                    );
                }
            }
            index.remove(key).unwrap();
            index.unregister(key).unwrap();
            moves += 1;
        }
        assert_eq!(callbacks.positions, 7);
        assert_eq!(callbacks.facings, 0);
        assert!(index.members().next().is_none());
    }
    assert_eq!(moves, 560);
}
