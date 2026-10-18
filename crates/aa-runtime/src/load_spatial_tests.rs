use super::*;
use serde_json::{json, Value};

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
#[ignore = "requires AA_GAME_ROOT and the verified capture03 assets"]
fn actual_first_kingdom_membership_and_same_index_adoption() {
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
        sources::{sha256, SourceLimits, V14Sources},
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
    let mut cases = Vec::new();
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
        // Source authority is independent of the live membership PC. The same
        // real Bank also binds the immediate Object Init source helper.
        let init_source =
            crate::live_object_init::Source::bind(&catalog, record.definition).unwrap();
        let source_pc = catalog
            .definition_bank
            .borrow()
            .combat_source()
            .unwrap()
            .source_precision();
        assert_eq!(init_source.source_precision(), source_pc);
        assert!(spatial.prepare(&Catalog::new(&actors), &record).is_err());
        assert!(spatial.bindings.is_empty());
        for guard in ["flags", "mask", "constructor", "category"] {
            // Explicit corrupt-definition guard probe, never native save data.
            let mut corrupt = record.definition.clone();
            let mut shell = record.shell.clone();
            if guard == "flags" {
                corrupt.flags ^= 1;
            } else if guard == "mask" {
                corrupt.component_mask = corrupt.component_mask.map(|v| v ^ 1);
            } else if guard == "constructor" {
                shell.kind = LoadKind::Object;
            } else {
                corrupt.proof.category.registry_index ^= 1;
                shell.thing.definition = corrupt.proof.clone();
            }
            let rejected = Record {
                owner: id,
                shell: &mut shell,
                definition: &corrupt,
                registry: &mut *record.registry,
            };
            assert!(spatial.prepare(&catalog, &rejected).is_err(), "{guard}");
            assert!(spatial.bindings.is_empty());
        }
        let key = spatial.prepare(&catalog, &record).unwrap();
        let mut changed = record.definition.clone();
        changed.proof.category.registry_index ^= 1;
        let mut changed_shell = record.shell.clone();
        changed_shell.thing.definition = changed.proof.clone();
        assert!(
            spatial
                .fields(&spatial.bindings[0], &changed_shell, &changed)
                .is_err(),
            "two changed mutable copies do not replace immutable source authority"
        );
        let input = spatial
            .fields(&spatial.bindings[0], record.shell, record.definition)
            .unwrap();
        let grid = spatial.index.grid().clone();
        let initial_flags = terrain
            .bound
            .cells
            .iter()
            .map(|c| c.flags20)
            .collect::<Vec<_>>();
        spatial.refresh_membership(&catalog, &mut record).unwrap();
        assert_eq!(record.shell.thing.flags, 0xA4C46900);
        let after = snapshot(&spatial.index);
        let trace = spatial.index.take_events();
        assert!(
            spatial.index.member(key).is_err(),
            "no persistent copied mutable fields"
        );
        assert_eq!(spatial.index.resolve_reference(id.raw()), Some(key));
        assert!(!trace.is_empty());
        // Native present-bit refresh is a no-op and must not duplicate nodes.
        spatial.refresh_membership(&catalog, &mut record).unwrap();
        assert!(spatial.index.take_events().is_empty());
        assert_eq!(snapshot(&spatial.index), after);
        let flags = record.shell.thing.flags;
        *attempt.registry.value_mut(id).unwrap() = held;
        let second = attempt
            .registry
            .iter()
            .find(|e| e.id != id && matches!(e.value, RestoredRecord::Kingdom(_)))
            .unwrap()
            .id;
        let mut store = ActorStore::from_restored(attempt.registry).unwrap();
        let object = store.key(id).unwrap();
        let staged = store.staged_placement(object).unwrap();
        assert_eq!(staged.flags, flags);
        let second_object = store.key(second).unwrap();
        let second_staged = store.staged_placement(second_object).unwrap();
        let mut parts = spatial.into_parts();
        let partial_probe = pc == Precision::Single24;
        if partial_probe {
            // Explicit storage corruption probe: another ACTUAL staged Kingdom
            // with a stale generation in the transfer metadata. No second
            // native membership or source geometry is invented.
            let mut stale = parts.bindings[0].clone();
            stale.id = ActorId::from_raw(second.raw() ^ 0x10000);
            stale.key = key + 1;
            parts.bindings.push(stale);
        }
        let mut adoption = SpatialWorld::adopt_loaded(parts, &mut store);
        assert_eq!(
            adoption.error.is_some(),
            partial_probe,
            "{:?}",
            adoption.error
        );
        assert_eq!(adoption.completed, vec![id]);
        assert_eq!(adoption.pending.len(), usize::from(partial_probe));
        assert_eq!(
            store.staged_placement(second_object).unwrap(),
            second_staged
        );
        assert_eq!(snapshot(adoption.world.index()), after);
        assert!(
            adoption.world.index_mut().take_events().is_empty(),
            "adoption never reinserts"
        );
        assert!(
            store.staged_placement(object).is_err(),
            "sole ownership moved to Spatial"
        );
        assert_eq!(store.placement(object, &adoption.world).unwrap(), staged);
        cases.push(json!({"precision":pc,"map":"Campaign/tutorial02","map_sha256":sha256(&map_bytes),
            "record_restoration_precision":Precision::Double53,"source_precision":source_pc,"member":input,"native_class":4,
            "definition_source":runtime_source(&catalog,&input),"grid":grid,"initial_flags":initial_flags,
            "rust":{"tiles":after,"member_flags":{key.to_string():flags},"trace":trace},
            "same_index_adopted":true,"full_map_init_complete":false}));
    }
    let result = json!({"scope":"actual stopped tutorial02 first Kingdom membership and storage adoption; source-completed diagnostic, not complete Init", "cases":cases});
    if let Some(path) = std::env::var_os("AA_LOAD_SPATIAL_CAPTURE") {
        serde_json::to_writer_pretty(
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .unwrap(),
            &result,
        )
        .unwrap();
    } else {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../research/skirmish-20261016-load-spatial/actual-first-kingdom.json");
        let expected: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(
            result, expected,
            "fresh actual owned-source capture changed"
        );
    }
}

#[test]
fn original_v14_native_membership_rows_and_load_leases() {
    let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-load-spatial");
    let actual: Value =
        serde_json::from_slice(&std::fs::read(folder.join("actual-first-kingdom.json")).unwrap())
            .unwrap();
    let fixture: Value =
        serde_json::from_slice(&std::fs::read(folder.join("native-load-spatial.json")).unwrap())
            .unwrap();
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 9);
    let mut rows = 0;
    for row in fixture["cases"].as_array().unwrap() {
        let input = actual["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["precision"] == row["precision"])
            .unwrap();
        let pc: Precision = serde_json::from_value(row["precision"].clone()).unwrap();
        let mut member: Member = serde_json::from_value(input["member"].clone()).unwrap();
        member.actor_category = row["native_class"].as_u64().unwrap() != 2;
        let key = member.key;
        let mut index = SpatialIndex::new(
            serde_json::from_value(input["grid"].clone()).unwrap(),
            serde_json::from_value(input["initial_flags"].clone()).unwrap(),
            pc,
        )
        .unwrap();
        index.set_event_capture(true);
        index
            .register_load_reference(key, member.reference)
            .unwrap();
        for name in ["first", "second"] {
            let expected = &row["expected"][name];
            let flags = {
                let mut lease = index.lease_load_fields(vec![member.clone()]).unwrap();
                lease.index_mut().refresh_membership(key, true).unwrap();
                lease.index().member(key).unwrap().actor.flags
            };
            member.actor.flags = flags;
            assert!(index.member(key).is_err());
            assert_eq!(index.reference_for_key(key).unwrap(), member.reference);
            assert_eq!(
                json!({"tiles":snapshot(&index),"member_flags":{key.to_string():flags},"trace":index.take_events()}),
                *expected
            );
        }
        index.retire_reference(member.reference);
        assert!(index.reference_for_key(key).is_err());
        assert!(index.resolve_reference(member.reference).is_none());
        rows += 1;
    }
    assert_eq!(rows, 9);
}

fn runtime_source(catalog: &Catalog<'_>, member: &Member) -> Value {
    let bank = catalog.definition_bank.borrow();
    let id = &bank.entry(member.actor.definition).unwrap().key.name;
    let definition = catalog
        .actors
        .actors
        .iter()
        .find(|d| d.id.eq_ignore_ascii_case(id))
        .unwrap();
    json!({"id":definition.id,"source":definition.source,"template_chain":definition.template_chain})
}

#[test]
fn load_field_lease_retains_cache_but_exposes_no_stale_pose_after_error() {
    // Explicit owned-storage probe; actual captured membership has its native
    // fixture replay below. This probe checks lease/error ownership, not map Init.
    let mut index = SpatialIndex::new(
        Grid {
            dimensions: [4, 4],
            coordinate_shift: 1,
            cell_size_bits: 2f32.to_bits(),
        },
        vec![0x210; 16],
        Precision::Double53,
    )
    .unwrap();
    let reference = 0x20001;
    let key = 17;
    index.register_load_reference(key, reference).unwrap();
    assert!(index.member(key).is_err());
    let point = Shape {
        kind: 0,
        radius_bits: 0,
        bounds: [0; 4],
    };
    let fields = Member {
        key,
        reference,
        actor: Actor {
            definition: 1,
            flags: 0x200,
            position: [2f32.to_bits(); 2],
            facing: [0, (-1f32).to_bits()],
        },
        height_bits: 0,
        facing_z_bits: 0,
        cached_tile: [0; 2],
        blocking: point.clone(),
        soft: point.clone(),
        target: point,
        actor_category: true,
        wall_present: false,
    };
    let (actual_flags, simulated_error) = {
        let mut lease = index.lease_load_fields(vec![fields]).unwrap();
        lease.index_mut().refresh_membership(key, true).unwrap();
        let flags = lease.index().member(key).unwrap().actor.flags;
        (
            flags,
            Err::<(), &str>("explicit test-only error after native-equivalent stores"),
        )
    };
    assert!(simulated_error.is_err());
    assert_eq!(actual_flags, 0x04000200);
    assert!(index.member(key).is_err());
    assert_eq!(index.resolve_reference(reference), Some(key));
    assert_eq!(index.reference_for_key(key).unwrap(), reference);
    assert!(index
        .register_load_reference(key, reference + 0x10000)
        .is_err());
    assert!(index.reference_for_key(key + 1).is_err());
    assert_eq!(index.actors_at([1, 1]).unwrap(), vec![key]);
}
