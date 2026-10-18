use super::*;
use serde_json::{Value, json};

#[test]
fn original_v14_native_removal_and_unindexed_descriptor_skip() {
    let actual: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-load-spatial/actual-first-kingdom.json"
    ))
    .unwrap();
    let native: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-load-spatial-teardown/native-teardown.json"
    ))
    .unwrap();
    let mut compared = 0;
    for row in native["rows"].as_array().unwrap() {
        let input = actual["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["precision"] == row["precision"])
            .unwrap();
        let pc: Precision = serde_json::from_value(row["precision"].clone()).unwrap();
        let mut member: Member = serde_json::from_value(input["member"].clone()).unwrap();
        member.actor_category = row["native_class"] != 2;
        let key = member.key;
        let reference = member.reference;
        let mut index = SpatialIndex::new(
            serde_json::from_value(input["grid"].clone()).unwrap(),
            serde_json::from_value(input["initial_flags"].clone()).unwrap(),
            pc,
        )
        .unwrap();
        index.register_load_reference(key, reference).unwrap();
        index.set_event_capture(true);
        if row["member_present"] == true {
            let mut lease = index.lease_load_fields(vec![member.clone()]).unwrap();
            lease.index_mut().refresh_membership(key, true).unwrap();
            member.actor.flags = lease.index().member(key).unwrap().actor.flags;
        }
        index.take_events();
        let flags = {
            let mut lease = index.lease_load_fields(vec![member]).unwrap();
            if lease.index().member(key).unwrap().actor.flags & (1 << 26) != 0 {
                lease.index_mut().remove(key).unwrap();
            }
            let flags = lease.index().member(key).unwrap().actor.flags;
            lease.index_mut().unregister(key).unwrap();
            flags
        };
        assert_eq!(
            json!({"tiles":snapshot(&index),"member_flags":{key.to_string():flags},"trace":index.take_events()}),
            row["expected"]["after"]
        );
        assert!(index.resolve_reference(reference).is_none());
        assert!(index.member(key).is_err());
        assert_eq!(row["expected"]["global_identity_still_present"], true);
        compared += 1;
    }
    assert_eq!(compared, 18);
}

fn snapshot(index: &SpatialIndex) -> Value {
    let mut tiles = Vec::new();
    let mut flag_bytes = Vec::new();
    for y in 0..index.grid().dimensions[1] {
        for x in 0..index.grid().dimensions[0] {
            let tile = index.tile([x, y]).unwrap();
            let mut cursor = tile.first_node;
            let mut last = None;
            while let Some(node) = cursor {
                last = cursor;
                cursor = index.node(node).unwrap().next;
            }
            assert_eq!(last, tile.last_node);
            flag_bytes.extend_from_slice(&tile.flags20.to_le_bytes());
            let actors = index.actors_at([x, y]).unwrap();
            if !actors.is_empty() || tile.primary_reference != 0 {
                tiles.push(json!({"cell":[x,y],"actors":actors,
                    "flags20":tile.flags20,"primary_reference":tile.primary_reference}));
            }
        }
    }
    json!({"occupied":tiles,"tile_count":flag_bytes.len()/4,
        "flags20_le_sha256":aa_rules::sources::sha256(&flag_bytes)})
}

#[test]
#[ignore = "requires actual source/capture03; bounded registered-shell teardown"]
fn actual_registered_kingdom_spatial_detach_precedes_global_identity_retirement() {
    use crate::{
        actor_world::ActorStore,
        battle_terrain, combat_source_completion as completion,
        combat_source_receipt::{TargetFiles, VerifiedStartupReceipt},
        localization::Localization,
        restoration::{Attempt, LoadBindings},
        restore_lifecycle::{ScopedManagedLoad, TaskKind},
        settings::Settings,
        spatial_world::SpatialWorld,
    };
    use aa_mapworld::restore::RestoreMode;
    use aa_rules::{
        actors::ActorCatalog,
        resources::ResourceCatalog,
        sources::{SourceLimits, V14Sources, sha256},
        tgi::Limits,
    };
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let native = game.join("reconstruction/native-game/runtime-expanded-v14");
    let capture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/development10/native-source-capture-03");
    let exe = std::fs::read(native.join("AA.exe")).unwrap();
    let dll = std::fs::read(native.join("D3D9.dll")).unwrap();
    let config = std::fs::read(native.join("dgVoodoo.conf")).unwrap();
    let receipt = std::fs::read(capture.join("capture.json")).unwrap();
    let manifest = std::fs::read(capture.join("runtime-files.json")).unwrap();
    let map_bytes = std::fs::read(
        game.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP"),
    )
    .unwrap();
    let map = aa_mapworld::MapArchive::from_bytes(map_bytes.clone()).unwrap();
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        &native,
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let settings = Settings::load_v14(&mut sources, Limits::default()).unwrap();
    let text = Localization::load_v14(&mut sources, Limits::default()).unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let resources = Arc::new(ResourceCatalog::load_v14(&mut sources).unwrap());
    let native: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-load-spatial-teardown/native-teardown.json"
    ))
    .unwrap();
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        // The captured startup receipt is source authority for the stopped
        // record diagnostic. This is a membership test, not completed map Init.
        let inactive = ScopedManagedLoad::from_sources(&mut sources, 0)
            .unwrap()
            .begin(TaskKind::Normal)
            .unwrap();
        let mut attempt = Attempt::load_with_source_setup(
            &map,
            &actors,
            settings.clone(),
            text.clone(),
            Precision::Double53,
            u64::MAX,
            RestoreMode::InspectPartial,
            LoadBindings {
                native_load_mode: Some(1),
                inactive_load: Some(inactive),
                resources: Some(Arc::clone(&resources)),
            },
            |catalog| {
                let proof = VerifiedStartupReceipt::verify(
                    catalog,
                    &mut sources,
                    TargetFiles {
                        native_exe: &exe,
                        renderer_dll: &dll,
                        renderer_config: &config,
                        capture_receipt: &receipt,
                        file_manifest_receipt: &manifest,
                    },
                )?;
                completion::install(
                    catalog,
                    &mut sources,
                    proof.context(),
                    proof.csv(),
                    proof.globals(),
                )?;
                completion::compute(catalog)
            },
        )
        .unwrap();
        let id = attempt
            .registry
            .iter()
            .find(|e| matches!(e.value, RestoredRecord::Kingdom(_)))
            .unwrap()
            .id;
        let mut held = std::mem::replace(
            attempt.registry.value_mut(id).unwrap(),
            RestoredRecord::Loading,
        );
        let RestoredRecord::Kingdom(kingdom) = &mut held else {
            unreachable!()
        };
        let runtime = kingdom.runtime.as_mut().unwrap();
        assert_eq!(runtime.definition.native_class, 4);
        assert_eq!(runtime.base.thing.flags, 0xA0C46900);
        assert_eq!(runtime.base.thing.flags & (1 << 26), 0);
        let terrain_source =
            battle_terrain::Source::bind(&mut sources, &actors, &settings, pc).unwrap();
        let terrain = battle_terrain::BattleTerrain::load(&map, &terrain_source).unwrap();
        let mut spatial = LoadSpatial::new(&terrain.bound, pc).unwrap();
        spatial.index.set_event_capture(true);
        let catalog = Catalog::new(&actors);
        *catalog.definition_bank.borrow_mut() = attempt.definitions;
        let mut record = Record {
            owner: id,
            shell: &mut runtime.base,
            definition: &runtime.definition,
            registry: &mut attempt.registry,
        };

        spatial.refresh_membership(&catalog, &mut record).unwrap();
        let key = spatial.key(id).unwrap();
        drop(record);
        *attempt.registry.value_mut(id).unwrap() = held;
        spatial.index.take_events();
        let before = snapshot(&spatial.index);
        let flags;
        {
            //Actual allocation stays in its registered entry during teardown.
            let RestoredRecord::Kingdom(kingdom) = attempt.registry.value_mut(id).unwrap() else {
                unreachable!()
            };
            let runtime = kingdom.runtime.as_mut().unwrap();
            let address = std::ptr::from_ref(&runtime.base);
            let components = runtime
                .base
                .actor
                .as_ref()
                .unwrap()
                .components
                .iter()
                .map(|(s, c)| (s, std::ptr::from_ref(c)))
                .collect::<Vec<_>>();
            let wrong_pc = if pc == Precision::Single24 {
                Precision::Extended64
            } else {
                Precision::Single24
            };
            assert!(
                spatial
                    .detach_for_destruction(
                        &catalog,
                        id,
                        &mut runtime.base,
                        &runtime.definition,
                        wrong_pc
                    )
                    .is_err()
            );
            assert_eq!(snapshot(&spatial.index), before);
            let original_pc = spatial.bindings[0].source_precision;
            spatial.bindings[0].source_precision = Precision::Single24;
            assert!(
                spatial
                    .detach_for_destruction(
                        &catalog,
                        id,
                        &mut runtime.base,
                        &runtime.definition,
                        pc
                    )
                    .is_err()
            );
            spatial.bindings[0].source_precision = original_pc;
            assert_eq!(snapshot(&spatial.index), before);
            spatial
                .detach_for_destruction(&catalog, id, &mut runtime.base, &runtime.definition, pc)
                .unwrap();
            flags = runtime.base.thing.flags;
            assert_eq!(flags & (1 << 26), 0);
            assert_eq!(std::ptr::from_ref(&runtime.base), address);
            assert_eq!(
                runtime
                    .base
                    .actor
                    .as_ref()
                    .unwrap()
                    .components
                    .iter()
                    .map(|(s, c)| (s, std::ptr::from_ref(c)))
                    .collect::<Vec<_>>(),
                components
            );
        }
        assert!(
            attempt.registry.get(id).is_some(),
            "globalGTID retirement is a later caller action"
        );
        assert!(spatial.key(id).is_err());
        assert!(spatial.index.member(key).is_err());
        assert!(spatial.index.resolve_reference(id.raw()).is_none());
        let expected = native["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| {
                r["precision"] == json!(pc) && r["native_class"] == 4 && r["member_present"] == true
            })
            .unwrap();
        assert_eq!(
            json!({"tiles":snapshot(&spatial.index),"member_flags":{key.to_string():flags},"trace":spatial.index.take_events()}),
            expected["expected"]["after"]
        );

        //A second ACTUAL Kingdom is bound for a query, but never inserted.
        //Native4EE560 then skips both descriptor getters and membership writes.
        let second = attempt
            .registry
            .iter()
            .find(|e| e.id != id && matches!(e.value, RestoredRecord::Kingdom(_)))
            .unwrap()
            .id;
        let mut held = std::mem::replace(
            attempt.registry.value_mut(second).unwrap(),
            RestoredRecord::Loading,
        );
        let RestoredRecord::Kingdom(kingdom) = &mut held else {
            unreachable!()
        };
        let runtime = kingdom.runtime.as_mut().unwrap();
        assert_eq!(runtime.base.thing.flags & (1 << 26), 0);
        spatial
            .prepare(
                &catalog,
                &Record {
                    owner: second,
                    shell: &mut runtime.base,
                    definition: &runtime.definition,
                    registry: &mut attempt.registry,
                },
            )
            .unwrap();
        *attempt.registry.value_mut(second).unwrap() = held;
        {
            let RestoredRecord::Kingdom(kingdom) = attempt.registry.value_mut(second).unwrap()
            else {
                unreachable!()
            };
            let runtime = kingdom.runtime.as_mut().unwrap();
            let flags = runtime.base.thing.flags;
            spatial
                .detach_for_destruction(
                    &catalog,
                    second,
                    &mut runtime.base,
                    &runtime.definition,
                    pc,
                )
                .unwrap();
            assert_eq!(runtime.base.thing.flags, flags);
        }
        assert!(spatial.bindings.is_empty());
        assert!(spatial.index.take_events().is_empty());
        assert!(attempt.registry.get(second).is_some());
        eprintln!(
            "actual registered Kingdom teardown pc{pc:?}: member removed, query-only detached, both globalIDs and shell/components retained"
        );
    }
}
