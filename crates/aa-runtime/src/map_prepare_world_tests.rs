use super::*;
use native::World as _;
use serde_json::{Value, json};

#[test]
fn actual_gtid_iterator_and_typed_definition_virtuals_match_native_leaves() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-map-prepare-world/native-world-leaves.json"
    ))
    .unwrap();
    let mut iterators = 0;
    let mut virtuals = 0;
    let mut drains = 0;
    for row in fixture["cases"].as_array().unwrap() {
        let input = &row["input"];
        match input["kind"].as_str().unwrap() {
            "iterator" => {
                let mut registry = IdentityRegistry::new();
                if input["entries"].as_array().unwrap().is_empty()
                    && input["highest"].as_u64().unwrap() > 0
                {
                    let highest = input["highest"].as_u64().unwrap() as u32;
                    registry = IdentityRegistry::from_load_metadata(
                        1,
                        highest,
                        &vec![0; highest as usize],
                    )
                    .unwrap();
                }
                for e in input["entries"].as_array().unwrap() {
                    let id = ActorId::from_raw(e["id"].as_u64().unwrap() as u32);
                    registry.seed_saved_generation(id).unwrap();
                    registry
                        .register_saved(id, e["actor"].as_bool().unwrap(), ())
                        .unwrap();
                }
                let mut cursor = input["cursor"].as_u64().unwrap() as u32;
                let answers = (0..input["calls"].as_u64().unwrap())
                    .map(|_| {
                        let id = next(&registry, &mut cursor);
                        json!({"id":id.map(ActorId::raw),"cursor":cursor})
                    })
                    .collect::<Vec<_>>();
                assert_eq!(json!(answers), row["output"]["answers"], "{row}");
                assert_eq!(json!(registry.len()), row["output"]["active_count"]);
                iterators += 1;
            }
            "virtual38" => {
                let kind = input["native_kind"].as_u64().unwrap() as u32;
                assert_eq!(
                    definition_virtual_38(kind).unwrap(),
                    row["output"]["eax"] != 0
                );
                virtuals += 1;
            }
            "drain" => {
                let empty = input["objects"] == 0 && input["actors"] == 0;
                assert_eq!(empty, row["output"]["stopped"].is_null());
                assert_eq!(row["output"]["queue_unchanged"], true);
                drains += 1;
            }
            _ => unreachable!(),
        }
    }
    assert_eq!((iterators, virtuals, drains), (48, 18, 9));
    for kind in [0, 3, 4, 5, 17, 22, 23, 24, u32::MAX] {
        assert!(definition_virtual_38(kind).is_err())
    }
}

#[test]
fn immediate_foreign_property_warning_retains_native_stop_prefix() {
    use capture::{DefinitionRef, PropertyInput, Snapshot};
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-map-prepare-world/native-warning-prefix.json"
    ))
    .unwrap();
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 9);
    for row in fixture["cases"].as_array().unwrap() {
        let properties = row["input"]["properties"].as_array().unwrap();
        let mut scanned = vec![];
        let mut warnings = vec![];
        let mut queue = Queue::default();
        let definitions = |id| DefinitionRef {
            token: id,
            canonical_ids: format!("property{id}"),
        };
        let inputs = properties.iter().enumerate().map(|(index, p)| {
            scanned.push(index);
            Ok(PropertyInput {
                kind: p["kind"].as_u64().unwrap() as u32,
                source: ActorId::from_raw(p["source"].as_u64().unwrap() as u32),
                definition: Some(900 + index as u32),
            })
        });
        let result = capture::actor_payload_with_warning(
            Snapshot {
                id: ActorId::from_raw(7),
                position_bits: [0; 3],
                direction_bits: [0; 2],
            },
            Some(ActorId::from_raw(777)),
            100,
            Some(DefinitionRef {
                token: 200,
                canonical_ids: "replacement".into(),
            }),
            inputs,
            &mut |token| Ok(definitions(token)),
            |_| panic!("native does not reach required-object read before warning"),
            &mut |warning| {
                warnings.push([warning.actor.raw(), warning.property.token]);
                Err("required404F60".into())
            },
        );
        if let Ok(request) = result.as_ref() {
            queue.append(capture::Request::Actor(request.clone()))
        }
        assert_eq!(result.unwrap_err(), "required404F60");
        assert_eq!(
            scanned,
            (0..=row["input"]["warning_index"].as_u64().unwrap() as usize).collect::<Vec<_>>()
        );
        assert_eq!(json!(warnings[0]), row["output"]["warning"]);
        assert!(queue.objects().is_empty() && queue.actors().is_empty());
    }
}

#[test]
fn random_logging_uses_each_actual_owner_and_lazy_gate() {
    use aa_orders::net::sync::{EntryKind, Synchronizer};
    let mut owner = Synchronizer::new(16);
    World::log(
        &mut SyncLog {
            owner: &mut owner,
            gate: &mut || panic!("disabled native log must not ask thread gate"),
        },
        "scope",
        None,
    )
    .unwrap();
    owner.enabled = true;
    owner.suspend = 1;
    World::log(
        &mut SyncLog {
            owner: &mut owner,
            gate: &mut || panic!("suspended native log must not ask thread gate"),
        },
        "seed",
        Some(9),
    )
    .unwrap();
    owner.suspend = 0;
    assert!(
        World::log(
            &mut SyncLog {
                owner: &mut owner,
                gate: &mut || Err("actual gate missing".into())
            },
            "scope",
            None
        )
        .is_err()
    );
    assert_eq!(owner.end(), i32::MIN);
    //Enable a retained native owner through its actual Host/join reset API.
    //Do not invent a writable fresh-owner log window by toggling enabled.
    owner.reset(-1, -1);
    World::log(
        &mut SyncLog {
            owner: &mut owner,
            gate: &mut || Ok(true),
        },
        "scope",
        None,
    )
    .unwrap();
    World::log(
        &mut SyncLog {
            owner: &mut owner,
            gate: &mut || Ok(true),
        },
        "seed",
        Some(0x80000002),
    )
    .unwrap();
    let marker = owner.entry(0).unwrap();
    assert_eq!(
        (&*marker.label, marker.kind, marker.value),
        ("scope", EntryKind::Empty, 0)
    );
    let seed = owner.entry(1).unwrap();
    assert_eq!(
        (&*seed.label, seed.kind, seed.value),
        ("seed", EntryKind::Int, 0x80000002)
    );
}

#[test]
#[ignore = "requires actual AA_GAME_ROOT data and verified capture03; bounded diagnostic staged input only"]
fn actual_firefight_map_preparation_borrows_same_gtid_bank_and_rng() {
    use crate::{
        combat_source_completion as completion,
        combat_source_receipt::VerifiedStartupReceipt,
        localization::Localization,
        restoration::{Attempt, LoadBindings},
        settings::Settings,
    };
    use aa_orders::net::sync::Synchronizer;
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: u64::MAX,
        },
    )
    .unwrap();
    let settings = Settings::load_v14(&mut sources, Limits::default()).unwrap();
    let text = Localization::load_v14(&mut sources, Limits::default()).unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let map = aa_mapworld::MapArchive::from_bytes(
        std::fs::read(
            game.join("reconstruction/assets/original/Data/Maps/DeveloperMaps/(3v3)Firefight.RMP"),
        )
        .unwrap(),
    )
    .unwrap();
    let attempt = Attempt::load_with_source_setup(
        &map,
        &actors,
        settings,
        text,
        Precision::Double53,
        u64::MAX,
        aa_mapworld::restore::RestoreMode::StopOnError,
        LoadBindings {
            native_load_mode: Some(1),
            ..LoadBindings::default()
        },
        |catalog| {
            let proof = VerifiedStartupReceipt::verify_embedded_reference(catalog, &mut sources)?;
            completion::install(
                catalog,
                &mut sources,
                proof.context(),
                proof.csv(),
                proof.globals(),
            )?;
            completion::compute(catalog)?;
            Ok(())
        },
    )
    .unwrap();
    assert!(
        attempt
            .report
            .events
            .iter()
            .all(|event| event.error.is_none())
    );
    //This codec-only Attempt deliberately omits immediate lifecycle. Moving it
    //into Staged supplies an actual source/owner integration fixture only.
    let mut staged = attempt.into_staged().unwrap();
    let catalog = Catalog::new(&actors);
    let before = staged
        .actors
        .identities()
        .iter()
        .map(|e| (e.id, e.is_actor, e.value))
        .collect::<Vec<_>>();
    let source_pc = staged
        .definitions
        .combat_source()
        .unwrap()
        .source_precision();
    let addresses = before
        .iter()
        .map(|(_, _, key)| staged.actors.object(*key).unwrap() as *const _ as usize)
        .collect::<Vec<_>>();
    let mut a = Synchronizer::new(16);
    let mut b = Synchronizer::new(16);
    let mut log_a = SyncLog {
        owner: &mut a,
        gate: &mut || panic!("disabled actual SyncA"),
    };
    let mut log_b = SyncLog {
        owner: &mut b,
        gate: &mut || panic!("disabled actual SyncB"),
    };
    let seed = staged.rng.state;
    let mut world = World {
        catalog: &catalog,
        staged: &mut staged,
        source_pc,
        precision: Precision::Double53,
        sync: &mut log_a,
        sim_sync: &mut log_b,
        definitions: None,
        startup: None,
    };
    let mut cursor = 0;
    let mut seen = vec![];
    while let Some(key) = world.next_object(&mut cursor).unwrap() {
        let id = world.staged.actors.id(key).unwrap();
        let token = world.definition(key).unwrap();
        let category = world.category(token).unwrap();
        seen.push((id, key, category));
    }
    assert_eq!(
        seen.iter().map(|(id, _, _)| *id).collect::<Vec<_>>(),
        before.iter().map(|(id, _, _)| *id).collect::<Vec<_>>()
    );
    assert!(
        seen.iter().any(|(_, _, c)| *c == 31),
        "real map includes actual placeholder sources"
    );
    //The codec-only input genuinely has no initialized/assigned Nation. Retain
    //that required boundary. A separate EXPLICIT canonical replacement fixture
    //supplies the real old-Actor arguments to positive removal work; it is not
    //claimed to be a native Nation selection or an initialized map.
    let mut first_capture = None;
    for &(id, key, category) in &seen {
        if category != 31 {
            continue;
        }
        let Some(owner) = world.owner(key).unwrap() else {
            continue;
        };
        if world.kingdom_word_190(owner).unwrap() != 0 {
            continue;
        }
        let definition = world.definition(key).unwrap();
        let nation = world.nation(owner).unwrap();
        assert!(
            nation.is_none(),
            "codec-only saved Kingdom has no assigned Nation"
        );
        let native_selection_error = world
            .substitution(nation, definition)
            .unwrap_err()
            .to_string();
        assert!(native_selection_error.contains("473370 dereferences actual null Nation"));
        world.bind_definitions().unwrap();
        let replacement = world
            .staged
            .definitions
            .token("ActorDefinition", "gbr_corps")
            .expect("explicit canonical real-source fixture target");
        let kind = if world.replacement_is_actor(replacement).unwrap() {
            RandomKind::Actor
        } else {
            RandomKind::Object
        };
        let mut queue = Queue::default();
        world
            .queue_replacement(&mut queue, key, Some(replacement), kind)
            .unwrap();
        first_capture = Some(
            json!({"id":id.raw(),"owner":owner.raw(),"old_definition":world.token(definition).unwrap(),
            "objects":queue.objects(),"actors":queue.actors(),"diagnostic_staged_input_only":true,
            "explicit_replacement_fixture":true,"native_nation_selection_not_claimed":true,"native_selection_error":native_selection_error}),
        );
        break;
    }
    let first_capture = first_capture.expect("actual eligible starting placeholder");
    if let Some(path) = std::env::var_os("AA_MAP_PREPARE_FIRST_QUEUE") {
        use std::io::Write;
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap()
            .write_all(
                serde_json::to_string_pretty(&first_capture)
                    .unwrap()
                    .as_bytes(),
            )
            .unwrap();
    }
    let mut rows = vec![];
    for live_pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        world.precision = live_pc;
        let outcome = native::resolve_placeholders_and_random_things(&mut world);
        let error = outcome.unwrap_err().to_string();
        assert!(
            error.contains("473370 dereferences actual null Nation"),
            "{error}"
        );
        rows.push(json!({"live_precision":live_pc,"outcome":error}));
    }
    assert_eq!(
        world.staged.rng.state, seed,
        "this actual map reaches no random draw"
    );
    assert_eq!(
        world
            .staged
            .actors
            .identities()
            .iter()
            .map(|e| (e.id, e.is_actor, e.value))
            .collect::<Vec<_>>(),
        before
    );
    assert_eq!(
        before
            .iter()
            .map(|(_, _, key)| world.staged.actors.object(*key).unwrap() as *const _ as usize)
            .collect::<Vec<_>>(),
        addresses
    );
    let output = json!({"map":"(3v3)Firefight","source_precision":source_pc,"objects":seen.len(),"rng_before":seed,"rng_after":world.staged.rng.state,
        "rows":rows,"first_capture":first_capture,"same_actual_allocation":true,"diagnostic_input_only":true});
    if let Some(path) = std::env::var_os("AA_MAP_PREPARE_CAPTURE") {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        file.write_all(serde_json::to_string_pretty(&output).unwrap().as_bytes())
            .unwrap();
    }
}
