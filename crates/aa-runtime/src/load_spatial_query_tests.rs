use super::*;

#[test]
#[ignore = "requires AA_GAME_ROOT and verified capture03"]
fn readonly_query_commits_current_and_prior_flags_on_error_and_retires_all_field_copies() {
    use crate::{
        battle_terrain, combat_source_completion as completion,
        combat_source_receipt::{TargetFiles, VerifiedStartupReceipt},
        localization::Localization,
        restoration::{Attempt, LoadBindings},
        restore_lifecycle::{ScopedManagedLoad, TaskKind},
        settings::Settings,
    };
    use aa_mapworld::restore::RestoreMode;
    use aa_rules::{
        actors::ActorCatalog,
        resources::ResourceCatalog,
        sources::{SourceLimits, V14Sources},
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
    let map = aa_mapworld::MapArchive::from_bytes(
        std::fs::read(
            game.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP"),
        )
        .unwrap(),
    )
    .unwrap();
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
    let inactive = ScopedManagedLoad::from_sources(&mut sources, 0)
        .unwrap()
        .begin(TaskKind::Normal)
        .unwrap();
    let mut attempt = Attempt::load_with_source_setup(
        &map,
        &actors,
        settings.clone(),
        text,
        Precision::Double53,
        u64::MAX,
        RestoreMode::InspectPartial,
        LoadBindings {
            native_load_mode: Some(1),
            inactive_load: Some(inactive),
            resources: Some(resources),
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
    let ids = attempt
        .registry
        .iter()
        .filter(|e| matches!(e.value, RestoredRecord::Kingdom(_)))
        .take(2)
        .map(|e| e.id)
        .collect::<Vec<_>>();
    assert_eq!(ids.len(), 2);
    let terrain_source =
        battle_terrain::Source::bind(&mut sources, &actors, &settings, Precision::Double53)
            .unwrap();
    let terrain = battle_terrain::BattleTerrain::load(&map, &terrain_source).unwrap();
    let mut spatial = LoadSpatial::new(&terrain.bound, Precision::Extended64).unwrap();
    let catalog = Catalog::new(&actors);
    *catalog.definition_bank.borrow_mut() = attempt.definitions;
    let mut prior = std::mem::replace(
        attempt.registry.value_mut(ids[0]).unwrap(),
        RestoredRecord::Loading,
    );
    {
        let RestoredRecord::Kingdom(value) = &mut prior else {
            unreachable!()
        };
        let runtime = value.runtime.as_mut().unwrap();
        spatial
            .refresh_membership(
                &catalog,
                &mut Record {
                    owner: ids[0],
                    shell: &mut runtime.base,
                    definition: &runtime.definition,
                    registry: &mut attempt.registry,
                },
            )
            .unwrap();
    }
    *attempt.registry.value_mut(ids[0]).unwrap() = prior;
    let mut held = std::mem::replace(
        attempt.registry.value_mut(ids[1]).unwrap(),
        RestoredRecord::Loading,
    );
    let RestoredRecord::Kingdom(value) = &mut held else {
        unreachable!()
    };
    let runtime = value.runtime.as_mut().unwrap();
    let mut record = Record {
        owner: ids[1],
        shell: &mut runtime.base,
        definition: &runtime.definition,
        registry: &mut attempt.registry,
    };
    // A collision query may identify this current actor BEFORE Object Init
    // inserts membership. Logical key creation must not fabricate tile nodes.
    let prior_tiles = (0..spatial.index().grid().dimensions[1])
        .flat_map(|y| (0..spatial.index().grid().dimensions[0]).map(move |x| [x, y]))
        .map(|cell| spatial.index().actors_at(cell).unwrap())
        .collect::<Vec<_>>();
    let original_flags = record.shell.thing.flags;
    let current_key = spatial
        .with_readonly_query(&catalog, &mut record, |index, bindings, record| {
            let key = bindings.key(record.owner).unwrap();
            assert_eq!(index.member(key)?.actor.flags, original_flags);
            Ok(key)
        })
        .unwrap();
    assert!(spatial.index().member(current_key).is_err());
    let after_tiles = (0..spatial.index().grid().dimensions[1])
        .flat_map(|y| (0..spatial.index().grid().dimensions[0]).map(move |x| [x, y]))
        .map(|cell| spatial.index().actors_at(cell).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(prior_tiles, after_tiles);
    assert_eq!(record.shell.thing.flags, original_flags);
    spatial.refresh_membership(&catalog, &mut record).unwrap();
    let keys = [spatial.key(ids[0]).unwrap(), spatial.key(ids[1]).unwrap()];
    let result: Result<()> =
        spatial.with_readonly_query(&catalog, &mut record, |index, bindings, record| {
            assert_eq!(bindings.key(record.owner).unwrap(), keys[1]);
            assert_eq!(bindings.id(keys[0]).unwrap(), ids[0]);
            for key in keys {
                assert_eq!(bindings.native_class(key).unwrap(), 4);
                let member = index.member(key)?;
                assert_eq!(bindings.proof(key).unwrap().id, "kingdom");
                assert_eq!(
                    bindings
                        .source_geometry(member.actor.definition)
                        .unwrap()
                        .source
                        .id,
                    "kingdom"
                );
                assert_eq!(member.actor.flags & 8, 0);
                index.set_ordinary_membership_request(key, true)?;
            }
            Err(aa_spatial::Error::World(
                "injected error after actual prior/current flag stores".into(),
            ))
        });
    assert!(result.unwrap_err().to_string().contains("injected error"));
    assert_ne!(record.shell.thing.flags & 8, 0);
    let RestoredRecord::Kingdom(prior) = &record.registry.get(ids[0]).unwrap().value else {
        unreachable!()
    };
    assert_ne!(prior.runtime.as_ref().unwrap().base.thing.flags & 8, 0);
    for key in keys {
        assert!(spatial.index().member(key).is_err());
    }
    let count = spatial
        .with_readonly_query(&catalog, &mut record, |index, bindings, _| {
            for binding in bindings.bindings() {
                assert_ne!(index.member(binding.key)?.actor.flags & 8, 0);
            }
            Ok(bindings.bindings().len())
        })
        .unwrap();
    assert_eq!(count, 2);
    for key in keys {
        assert!(spatial.index().member(key).is_err());
    }
    //4EB6E0 may refresh a child while its original source remains held. Both
    // real allocations must be available to the same index and receive flag
    // writes even if the child query subsequently fails.
    let mut other_held = std::mem::replace(
        record.registry.value_mut(ids[0]).unwrap(),
        RestoredRecord::Loading,
    );
    let RestoredRecord::Kingdom(other) = &mut other_held else {
        unreachable!()
    };
    let other = other.runtime.as_mut().unwrap();
    let other_pointer = &other.base as *const _ as usize;
    let mut extra = HeldShell {
        owner: ids[0],
        shell: &mut other.base,
        definition: &other.definition,
    };
    let result: Result<()> = spatial.with_readonly_query_extra(
        &catalog,
        &mut record,
        &mut extra,
        |index, bindings, current, extra| {
            assert_ne!(current.owner, extra.owner);
            assert_eq!(extra.shell as *const _ as usize, other_pointer);
            assert!(matches!(
                current.registry.get(extra.owner).unwrap().value,
                RestoredRecord::Loading
            ));
            assert!(matches!(
                current.registry.get(current.owner).unwrap().value,
                RestoredRecord::Loading
            ));
            for id in [current.owner, extra.owner] {
                index.set_ordinary_membership_request(bindings.key(id).unwrap(), false)?;
            }
            Err(aa_spatial::Error::World(
                "error after both actual held flag writes".into(),
            ))
        },
    );
    assert!(result.unwrap_err().to_string().contains("both actual held"));
    assert_eq!(record.shell.thing.flags & 8, 0);
    assert_eq!(extra.shell.thing.flags & 8, 0);
    for key in keys {
        assert!(spatial.index().member(key).is_err());
    }
    // A second view of the current logical identity is forbidden before any
    // operation. It must never become an alternate mutable source authority.
    extra.owner = record.owner;
    let result: Result<()> =
        spatial.with_readonly_query_extra(&catalog, &mut record, &mut extra, |_, _, _, _| {
            panic!("duplicate held identity must fail before query")
        });
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("duplicates current"));
    extra.owner = ids[0];
    *record.registry.value_mut(ids[0]).unwrap() = other_held;
    *attempt.registry.value_mut(ids[1]).unwrap() = held;
}
