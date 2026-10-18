use super::*;
use aa_numeric::Binary;
use aa_rules::{sources::SourceLimits, tgi::Limits};
use serde_json::{Value, json};
use std::path::PathBuf;

fn evidence(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-battle-terrain")
        .join(name)
}
#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 maps and source"]
fn all_original_maps_bind_owned_source_ground_surface_and_native_samples() {
    let root = PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: u64::MAX,
        },
    )
    .unwrap();
    let settings = Settings::load_v14(&mut sources, Limits::default()).unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let mut maps = sources
        .catalog()
        .into_iter()
        .filter(|entry| {
            entry.logical_path.to_ascii_lowercase().starts_with("maps/")
                && entry.logical_path.to_ascii_lowercase().ends_with(".rmp")
        })
        .map(|entry| entry.logical_path)
        .collect::<Vec<_>>();
    maps.sort_by_key(|name| name.to_ascii_lowercase());
    assert_eq!(maps.len(), 80);
    let capture = std::env::var_os("AA_BATTLE_TERRAIN_CAPTURE").is_some();
    let oracle: Option<Value> = (!capture).then(|| {
        serde_json::from_slice(&std::fs::read(evidence("native-battle-terrain.json")).unwrap())
            .unwrap()
    });
    let source_oracle: Option<Value> = (!capture).then(|| {
        serde_json::from_slice(&std::fs::read(evidence("source-metadata.json")).unwrap()).unwrap()
    });
    let find_oracle: Option<Value> = (!capture).then(|| {
        serde_json::from_slice(&std::fs::read(evidence("native-source-find.json")).unwrap())
            .unwrap()
    });
    let fields_oracle: Option<Value> = (!capture).then(|| {
        serde_json::from_slice(&std::fs::read(evidence("native-source-fields.json")).unwrap())
            .unwrap()
    });
    let old: Value = serde_json::from_str(include_str!(
        "../../../research/phase6-spatial/native-real-map-terrain.json"
    ))
    .unwrap();
    let heights: Value = serde_json::from_str(include_str!(
        "../../../research/phase6-spatial/native-real-map-heights.json"
    ))
    .unwrap();
    let previous_aliases: Value = serde_json::from_str(include_str!(
        "../../../research/phase6-terrain/terrain-render-metadata.json"
    ))
    .unwrap();
    let mut captures = vec![];
    let mut source_captures = vec![];
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        let source = Source::bind(&mut sources, &actors, &settings, pc).unwrap();
        assert_eq!(source.source_precision(), pc);
        source_captures.push(json!({"precision":format!("{pc:?}"),"terrains":source.terrains,"variations":source.variations,"files":source.files,"biomes":source.biome_initial.iter().map(|(path,index)|json!({"path":path,"variation":index})).collect::<Vec<_>>(),"variation_translations":actors.native_data_registry("Variation").unwrap().translations()}));
        if let Some(oracle) = &source_oracle {
            assert_eq!(
                source_captures.last().unwrap(),
                oracle["precisions"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|row| row["precision"] == format!("{pc:?}"))
                    .unwrap(),
                "current source metadata must equal frozen native inputs"
            );
        }
        if let Some(oracle) = &find_oracle {
            for row in oracle["lookups"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| row["precision"] == format!("{pc:?}"))
            {
                let query = row["query"].as_str().unwrap();
                let result = source
                    .resolve(&query.encode_utf16().collect::<Vec<_>>())
                    .unwrap()
                    .map(|(index, _)| source.variations[index].id.clone());
                assert_eq!(json!(result), row["result"], "native48D500 {pc:?} {query}");
            }
        }
        if let Some(oracle) = &fields_oracle {
            for row in
                oracle["cases"].as_array().unwrap().iter().filter(|row| {
                    row["precision"] == format!("{pc:?}") && row.get("terrain").is_some()
                })
            {
                let id = row["terrain"].as_str().unwrap();
                let terrain = source.terrains.iter().find(|value| value.id == id).unwrap();
                assert_eq!(json!(terrain.passability_bits), row["passability_bits"]);
                assert_eq!(json!(terrain.noise_bits), row["noise_bits"]);
                let index = source
                    .terrains
                    .iter()
                    .position(|value| value.id == id)
                    .unwrap();
                assert_eq!(
                    source
                        .variations
                        .iter()
                        .find(|value| value.terrain == index)
                        .unwrap()
                        .id,
                    row["initial_variation"]
                );
            }
        }
        if capture {
            std::fs::create_dir_all(evidence("")).unwrap();
            std::fs::write(
                evidence("source-metadata.json"),
                serde_json::to_vec_pretty(&json!({"schema":1,"precisions":source_captures}))
                    .unwrap(),
            )
            .unwrap();
        }
        assert_eq!(source.height_resolution, 1);
        assert_eq!(source.terrain_resolution, 1);
        for path in &maps {
            let file = sources.read(path).unwrap();
            let archive = MapArchive::from_bytes(file.bytes).unwrap();
            let loaded =
                BattleTerrain::load(&archive, &source).unwrap_or_else(|e| panic!("{path}: {e}"));
            assert_eq!(loaded.source_precision(), pc);
            let name = path.replace('\\', "/");
            assert_eq!(archive.source().sha256, old["maps"][&name]["map_sha256"]);
            assert_eq!(
                loaded.base.chunk_sha256,
                heights["maps"][&name]["chunks"]["HEIGHT_MAP"]
            );
            assert_eq!(
                loaded.overlay.chunk_sha256,
                heights["maps"][&name]["chunks"]["MOUNTAIN_HEIGHT_MAP"]
            );
            assert_eq!(
                loaded.bound.cells.len(),
                old["maps"][&name]["tile_count"].as_u64().unwrap() as usize
            );
            let changed_aliases = loaded
                .terrain
                .palette_utf16
                .iter()
                .enumerate()
                .filter(|(index, _)| {
                    loaded
                        .terrain
                        .tiles
                        .iter()
                        .any(|tile| usize::from(tile.variation_index()) == *index)
                })
                .filter_map(|(_, id)| {
                    let name = String::from_utf16(id).unwrap();
                    let current = source
                        .resolve(id)
                        .unwrap()
                        .map(|(index, _)| source.variations[index].id.clone());
                    let prior = previous_aliases["variants"][&name]["canonical"]
                        .as_str()
                        .map(str::to_owned);
                    (current != prior)
                        .then(|| json!({"query":name,"current":current,"previous":prior}))
                })
                .collect::<Vec<_>>();
            if changed_aliases.is_empty() {
                assert_eq!(
                    loaded.bound.missing_variation_ordinals.len(),
                    old["maps"][&name]["fallback_count"].as_u64().unwrap() as usize,
                    "{name} fallback count"
                );
            }
            assert_eq!(
                loaded.bound.has_blocked_outer_ring(),
                old["maps"][&name]["blocked_outer_ring"].as_bool().unwrap()
            );
            let flag_bytes = loaded
                .bound
                .cells
                .iter()
                .flat_map(|cell| cell.flags20.to_le_bytes())
                .collect::<Vec<_>>();
            let flags_sha256 = aa_mapworld::archive::sha256(&flag_bytes);
            if changed_aliases.is_empty() {
                assert_eq!(
                    flags_sha256, old["maps"][&name]["flags_sha256"],
                    "{name} static terrain flags"
                );
            }
            let base_address = loaded.base.state.field.height_bits.as_ptr();
            let overlay_address = loaded.overlay.state.field.height_bits.as_ptr();
            assert_ne!(
                base_address, overlay_address,
                "independent actual layer allocations"
            );
            let surface = loaded.surface().unwrap();
            assert!(std::ptr::eq(surface.terrain, &loaded.grid));
            assert!(loaded.passability_bits_at(-1, 0).is_err());
            assert!(loaded.passability_bits_at(loaded.grid.width, 0).is_err());
            let mut cells = vec![
                (0, 0),
                (loaded.grid.width / 2, loaded.grid.height / 2),
                (loaded.grid.width - 1, loaded.grid.height - 1),
            ];
            for kind in [1, 2] {
                if let Some(index) = loaded
                    .grid
                    .cells
                    .iter()
                    .position(|cell| cell.overlay_kind == kind)
                {
                    cells.push((
                        index as i32 % loaded.grid.width,
                        index as i32 / loaded.grid.width,
                    ));
                }
            }
            cells.sort_unstable();
            cells.dedup();
            let mut samples = vec![];
            for (x, y) in cells {
                let definition = loaded.terrain_definition_at(x, y).unwrap();
                assert_eq!(
                    definition.native_type as i32,
                    loaded.grid.cells[(y * loaded.grid.width + x) as usize].overlay_kind
                );
                assert_eq!(
                    loaded.passability_bits_at(x, y).unwrap(),
                    definition.passability_bits
                );
                //An explicit interior fractional location exercises native
                //interpolation and the current tile's mountain-noise branch.
                let position = [x, y].map(|value| {
                    Binary::from_i32(value)
                        .mul(&Binary::from_f32_bits(loaded.grid.cell_size).unwrap(), pc)
                        .unwrap()
                        .add(&Binary::from_f32_bits(0x3f400000).unwrap(), pc)
                        .unwrap()
                        .store_f32()
                        .unwrap()
                });
                for flags in [0x122u32, 0x123] {
                    let value = surface.sample(position, flags, pc).unwrap();
                    let bits = value.store_f32().unwrap();
                    let neighborhood=(-1..=1).flat_map(|dy|(-1..=1).map(move|dx|(x+dx,y+dy))).filter(|(a,b)|*a>=0 && *b>=0 && *a<loaded.grid.width && *b<loaded.grid.height).map(|(a,b)|json!({"xy":[a,b],"cell":loaded.grid.cells[(b*loaded.grid.width+a) as usize]})).collect::<Vec<_>>();
                    let sample = json!({"position":position,"flags":flags,"neighborhood":neighborhood,"height_bits":bits});
                    if let Some(oracle) = &oracle {
                        let rows = oracle["samples"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .filter(|row| {
                                row["source"] == name
                                    && row["precision"] == format!("{pc:?}")
                                    && row["input"] == sample
                            })
                            .collect::<Vec<_>>();
                        assert_eq!(rows.len(), 2, "original/v14 {name} {sample}");
                        for row in rows {
                            assert_eq!(row["output"]["height_bits"], bits);
                        }
                    }
                    samples.push(sample);
                }
            }
            assert_eq!(loaded.base.state.field.height_bits.as_ptr(), base_address);
            assert_eq!(
                loaded.overlay.state.field.height_bits.as_ptr(),
                overlay_address
            );
            captures.push(json!({"source":name,"map_sha256":archive.source().sha256,"precision":format!("{pc:?}"),"base_layout":loaded.base.state.layout,"overlay_layout":loaded.overlay.state.layout,"grid":{"width":loaded.grid.width,"height":loaded.grid.height,"coordinate_shift":loaded.grid.coordinate_shift,"cell_size":loaded.grid.cell_size},"initial_variation":loaded.bound.initial_variation.as_ref().map(|v|String::from_utf16(&v.id_utf16).unwrap()),"palette":loaded.terrain.palette_utf16.iter().map(|id|String::from_utf16(id).unwrap()).collect::<Vec<_>>(),"fallback_count":loaded.bound.missing_variation_ordinals.len(),"flags_sha256":flags_sha256,"previous_fallback_count":old["maps"][&name]["fallback_count"],"previous_flags_sha256":old["maps"][&name]["flags_sha256"],"changed_previous_aliases":changed_aliases,"samples":samples}));
        }
    }
    if capture {
        std::fs::create_dir_all(evidence("")).unwrap();
        std::fs::write(
            evidence("actual-map-inputs.json"),
            serde_json::to_vec_pretty(&json!({"schema":1,"maps":captures})).unwrap(),
        )
        .unwrap();
    }
    assert_eq!(captures.len(), 240);
}
