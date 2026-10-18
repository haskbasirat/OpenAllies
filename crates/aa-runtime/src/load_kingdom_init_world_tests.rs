use super::*;
use serde_json::{Value, json};

#[test]
fn native_world_registration_and_partial_stores_all_pc() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-load-kingdom-init/native-registration.json"
    ))
    .unwrap();
    assert_eq!(fixture["native_runs"], 96);
    for row in fixture["cases"].as_array().unwrap() {
        let c = &row["input"];
        let slots = |name: &str| {
            c[name]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().map(|v| ActorId::from_raw(v as u32)))
                .collect()
        };
        let mut state = KingdomRegistryState {
            count: c["count"].as_u64().unwrap() as u32,
            all: slots("all"),
            majors: slots("majors"),
            color_counts: serde_json::from_value(c["colors"].clone()).unwrap(),
        };
        let owner = ActorId::from_raw(c["owner"].as_u64().unwrap() as u32);
        let mut derived = SpecialShell::kingdom(owner, 0).unwrap();
        for (field, offset) in [
            ("all_index", 0x188),
            ("major_index", 0x18c),
            ("minor", 0x190),
        ] {
            derived
                .words
                .insert(offset, c[field].as_u64().unwrap() as u32);
        }
        if let Some(color) = c["color"].as_u64() {
            derived.references.insert(
                0x1e8,
                Some(aa_mapworld::restore::NamedDefinition {
                    id: "explicit-native-color-fixture".into(),
                    index: Some(color as u32),
                }),
            );
        }
        let outcome = state.register(owner, &derived);
        let stop = row["output"]["stop"].as_str();
        assert_eq!(outcome.is_ok(), stop.is_none(), "{row}: {outcome:?}");
        if let Err(e) = outcome {
            assert!(
                match stop.unwrap() {
                    "all_occupied" | "major_occupied" => e.contains("occupied"),
                    "all_negative" | "major_negative" => e.contains("negative"),
                    "all_sparse" | "major_sparse" => e.contains("uninitialized"),
                    "color_storage_absent" => e.contains("allocation is unbound"),
                    "color_outside" => e.contains("outside"),
                    other => panic!("unknown native stop {other}"),
                },
                "{row}: {e}"
            );
        }
        let raw = |values: &[Option<ActorId>]| {
            values
                .iter()
                .map(|v| v.map(ActorId::raw))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            json!({"count":state.count,"all":raw(&state.all),"majors":raw(&state.majors),
            "colors":state.color_counts,"stop":stop}),
            row["output"],
            "{row}"
        );
    }
}

#[test]
#[ignore = "requires AA_GAME_ROOT only for an unused serialized container witness"]
fn borrowed_relation_tables_and_native_mask_stores_all_pc() {
    use aa_mapworld::bootstrap::DefinitionProof;
    use aa_mapworld::restore::{
        GroundDefinitions, LoadCompletion, RuntimeDefinition, SourceWorldEntity, WorldEntityRuntime,
    };
    use aa_mapworld::special_state::NativeVectorState;
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let map = aa_mapworld::MapArchive::from_bytes(
        std::fs::read(
            game.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP"),
        )
        .unwrap(),
    )
    .unwrap();
    let inventory = aa_mapworld::WorldInventory::decode(&map, u64::MAX).unwrap();
    let template = inventory
        .kingdoms
        .as_ref()
        .unwrap()
        .slots
        .iter()
        .find_map(|v| v.record.as_ref())
        .unwrap();
    let actors =
        aa_rules::actors::ActorCatalog::from_files(vec![], aa_rules::tgi::Limits::default());
    let catalog = Catalog::new(&actors);
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-load-kingdom-init/native-relations-masks.json"
    ))
    .unwrap();
    let make_definition = |class| RuntimeDefinition {
        proof: DefinitionProof {
            id: "explicit-relation-table-fixture".into(),
            source: aa_rules::resources::Provenance {
                path: "explicit-unread-fixture-source".into(),
                sha256: "fixture".into(),
                span: aa_rules::tgi::Span { start: 0, end: 0 },
            },
            template_chain: vec![],
            category: *aa_rules::native::CATEGORIES
                .iter()
                .find(|v| v.runtime_class == class)
                .unwrap(),
            base_flag_inputs: vec![],
        },
        native_class: class,
        flags: 0,
        object: None,
        component_mask: None,
        ground: GroundDefinitions::default(),
    };
    struct MustNotRun;
    impl crate::load_actor_init_world::Services for MustNotRun {
        fn object_init_null(
            &mut self,
            _: &mut Record<'_>,
            _: Precision,
        ) -> aa_actors::actor_creation::Result<bool> {
            panic!("relation/mask proof must not enter Object Init")
        }
    }
    let number = |v: &Value| v.as_u64().unwrap() as u32;
    let vector = |v: &Value| NativeVectorState {
        values: v.as_array().unwrap().iter().map(number).collect(),
        capacity_hint: v.as_array().unwrap().len() as u32,
    };
    for row in fixture["cases"].as_array().unwrap() {
        let input = &row["input"];
        for expected in row["output"].as_array().unwrap() {
            let owner = ActorId::from_raw(number(&expected["owner"]));
            let mut registry = aa_actors::IdentityRegistry::new();
            for k in input["kingdoms"].as_array().unwrap() {
                let id = ActorId::from_raw(number(&k["id"]));
                let mut special = SpecialShell::kingdom(id, 0).unwrap();
                for (key, offset) in [
                    ("index", 0x188),
                    ("major", 0x18c),
                    ("minor", 0x190),
                    ("word228", 0x228),
                    ("word22c", 0x22c),
                ] {
                    special.words.insert(offset, number(&k[key]));
                }
                special.bytes.remove(&0x22c);
                special.bytes.remove(&0x22d);
                special.identity_references.insert(
                    0x1ec,
                    k["team"].as_u64().map(|v| ActorId::from_raw(v as u32)),
                );
                special.vectors.insert(0x1d0, vector(&k["kingdoms"]));
                special.vectors.insert(0x1dc, vector(&k["teams"]));
                let definition = make_definition(4);
                let runtime = WorldEntityRuntime {
                    base: aa_actors::loading::construct_shell_with_components(
                        aa_actors::loading::LoadKind::Actor,
                        definition.proof.clone(),
                    ),
                    definition,
                    derived: Some(special),
                    actor_tail: None,
                    derived_payload_loaded: false,
                };
                // The serialized container is unused by these native leaves.
                // All table/identity/word inputs are the named synthetic oracle.
                registry
                    .register_saved(
                        id,
                        false,
                        RestoredRecord::Kingdom(Box::new(SourceWorldEntity {
                            source: template.clone(),
                            completion: LoadCompletion::RegisteredLoadShell,
                            runtime: Some(runtime),
                            failure: None,
                        })),
                    )
                    .unwrap();
            }
            for t in input["teams"].as_array().unwrap() {
                let id = ActorId::from_raw(number(&t["id"]));
                let mut special = SpecialShell::team().unwrap();
                special.words.insert(0x38, number(&t["index"]));
                special.vectors.insert(0x4c, vector(&t["kingdoms"]));
                special.vectors.insert(0x58, vector(&t["teams"]));
                let definition = make_definition(5);
                let runtime = WorldEntityRuntime {
                    base: aa_actors::loading::construct_shell_with_components(
                        aa_actors::loading::LoadKind::Thing,
                        definition.proof.clone(),
                    ),
                    definition,
                    derived: Some(special),
                    actor_tail: None,
                    derived_payload_loaded: false,
                };
                registry
                    .register_saved(
                        id,
                        false,
                        RestoredRecord::Team(Box::new(SourceWorldEntity {
                            source: aa_mapworld::kingdoms::TeamRecord {
                                source_span: aa_mapworld::inventory::ByteSpan {
                                    offset: 0,
                                    length: 0,
                                },
                                definition_id_utf16: vec![],
                                serialized_id: id.raw(),
                                thing: template.thing.clone(),
                                version: 0,
                                slot_index: number(&t["index"]),
                                display_key_utf16: vec![],
                                words_4c: vec![],
                                words_58: vec![],
                                string_64_utf16: None,
                            },
                            completion: LoadCompletion::RegisteredLoadShell,
                            runtime: Some(runtime),
                            failure: None,
                        })),
                    )
                    .unwrap();
            }
            let mut held =
                std::mem::replace(registry.value_mut(owner).unwrap(), RestoredRecord::Loading);
            let RestoredRecord::Kingdom(k) = &mut held else {
                unreachable!()
            };
            let runtime = k.runtime.as_mut().unwrap();
            let derived = runtime.derived.as_mut().unwrap();
            let mut record = Record {
                owner,
                shell: &mut runtime.base,
                definition: &runtime.definition,
                registry: &mut registry,
            };
            let mut state = KingdomRegistryState::from_saved_counts(0, 0).unwrap();
            state.majors = input["majors"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().map(|v| ActorId::from_raw(v as u32)))
                .collect();
            let mut version = 27;
            let mut messages = Vec::new();
            let mut services = MustNotRun;
            let mut world = World {
                catalog: &catalog,
                record: &mut record,
                derived,
                context: Context {
                    actor_services: &mut services,
                    world_version: &mut version,
                    registry_state: &mut state,
                    restrictions: None,
                    messages: &mut messages,
                    source_ordinal: 0,
                },
                source_pc: Precision::Double53,
                categories: None,
            };
            for q in expected["queries"].as_array().unwrap() {
                let other = ActorId::from_raw(number(&q["other"]));
                assert_eq!(
                    world.relation(owner, other).unwrap(),
                    number(&q["relation"])
                );
                assert_eq!(
                    world.related_actual(owner, other).unwrap(),
                    q["related"].as_bool().unwrap()
                );
            }
            world.refresh_mask(owner, false).unwrap();
            world.refresh_mask(owner, true).unwrap();
            assert_eq!(
                word(world.derived, 0x228).unwrap(),
                number(&expected["word228"]),
                "{row}"
            );
            assert_eq!(
                word(world.derived, 0x22c).unwrap(),
                number(&expected["word22c"]),
                "{row}"
            );
            drop(world);
            *registry.value_mut(owner).unwrap()=held;
            crate::configure_world_relations::refresh(&mut registry,&state,owner).unwrap();
            let RestoredRecord::Kingdom(k)=&registry.get(owner).unwrap().value else {unreachable!()};
            let actual=k.runtime.as_ref().unwrap().derived.as_ref().unwrap();
            let full:Value=serde_json::from_str(include_str!(
                "../../../research/development10/native-configure-relations.json")).unwrap();
            let matching=full["cases"].as_array().unwrap().iter().find(|f|
                f["input"]==row["input"]&&f["precision"]==row["precision"]).unwrap();
            let expected=matching["output"].as_array().unwrap().iter().find(|v|
                v["owner"].as_u64()==Some(owner.raw() as u64)).unwrap();
            assert_eq!(word(actual,0x228).unwrap(),number(&expected["word228"]));
            assert_eq!(word(actual,0x22c).unwrap(),number(&expected["word22c"]));
        }
    }
}

#[test]
fn native_team_leaves_and_actual_color_constructor_all_pc() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-load-kingdom-init/native-team-leaves.json"
    ))
    .unwrap();
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 72);
    for row in fixture["cases"].as_array().unwrap() {
        match row["operation"].as_str().unwrap() {
            "team_constructor_vtable_store" => assert_eq!(row["output"], "0xa29780"),
            "global_point_copy" => assert_eq!(row["input"], row["output"]),
            "false_leaf" => {
                assert_eq!(row["output"], 0);
                if row["function"] == "0x4c9fa0" {
                    assert_eq!(row["full_eax"], 0);
                }
            }
            "color_count_initialize" => {
                let count = row["count"].as_u64().unwrap() as u32;
                let mut state = KingdomRegistryState::from_saved_counts(0, 0).unwrap();
                state.bind_color_counts(vec![0; count as usize]).unwrap();
                assert_eq!(
                    serde_json::to_value(state.color_counts()).unwrap(),
                    row["output"]
                );
            }
            _ => unreachable!(),
        }
    }
}

#[test]
#[ignore = "requires AA_GAME_ROOT and verified capture03"]
fn actual_first_kingdom_init_uses_same_owned_payload_with_explicit_object_seam() {
    use crate::{
        combat_source_completion as completion,
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
        sources::{SourceLimits, V14Sources, sha256},
        tgi::Limits,
    };
    use std::sync::Arc;
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
    struct ObjectSeam(usize);
    impl crate::load_actor_init_world::Services for ObjectSeam {
        fn object_init_null(
            &mut self,
            _: &mut Record<'_>,
            _: Precision,
        ) -> aa_actors::actor_creation::Result<bool> {
            self.0 += 1;
            Ok(true)
        }
    }
    let mut rows = Vec::new();
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        // Fresh actual serialized load for each live PC. Only base Object Init
        // is a named fixture seam; all Actor/Kingdom fields remain owned.
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
                resources: Some(resources.clone()),
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
        let owner = attempt
            .registry
            .iter()
            .find(|e| matches!(e.value, RestoredRecord::Kingdom(_)))
            .unwrap()
            .id;
        let mut held = std::mem::replace(
            attempt.registry.value_mut(owner).unwrap(),
            RestoredRecord::Loading,
        );
        let RestoredRecord::Kingdom(kingdom) = &mut held else {
            unreachable!()
        };
        let runtime = kingdom.runtime.as_mut().unwrap();
        let derived = runtime.derived.as_mut().unwrap();
        let derived_address = derived as *const _ as usize;
        let initial = json!({"derived_words":derived.words,"flows":derived.vectors.iter()
            .map(|(k,v)|(k.to_string(),json!({"values":v.values,"capacity":v.capacity_hint})))
            .collect::<serde_json::Map<String,Value>>(),"recruit_items":derived.recruit_items,
            "team":derived.identity_references.get(&0x1ec),"color":derived.references.get(&0x1e8)
                .map(|value|value.as_ref().map(|v|json!({"id":v.id,"index":v.index})))});
        let mut catalog = Catalog::new(&actors);
        catalog.settings = Some(settings.clone());
        catalog
            .bind_inactive_load(attempt.inactive_load.as_ref().unwrap())
            .unwrap();
        catalog.bind_resource_catalog(resources.clone()).unwrap();
        *catalog.definition_bank.borrow_mut() = attempt.definitions;
        let source_pc = catalog
            .definition_bank
            .borrow()
            .combat_source()
            .unwrap()
            .source_precision();
        let groups = crate::restriction_groups::LoadedRestrictionGroups::load(
            &map.decode_chunk("RESTRICTION_GROUPS", u64::MAX).unwrap(),
            &catalog,
            aa_mapworld::restriction_groups::LocalizedStorage::KeyOnly,
            source_pc,
        )
        .unwrap();
        let mut registry_state =
            KingdomRegistryState::from_inventory(&catalog, &attempt.inventory).unwrap();
        let registry_initial = serde_json::to_value(&registry_state).unwrap();
        let mut version = attempt.inventory.world.version;
        assert_eq!(version, 27);
        let mut messages = Vec::new();
        let mut seam = ObjectSeam(0);
        let mut record = Record {
            owner,
            shell: &mut runtime.base,
            definition: &runtime.definition,
            registry: &mut attempt.registry,
        };
        let address = record.shell as *const _ as usize;
        let outcome = initialize_kingdom(
            &catalog,
            &mut record,
            derived,
            Context {
                actor_services: &mut seam,
                world_version: &mut version,
                registry_state: &mut registry_state,
                restrictions: Some(&groups),
                messages: &mut messages,
                source_ordinal: 0,
            },
            pc,
        );
        assert_eq!(
            outcome,
            Ok(true),
            "actual first Kingdom must pass the complete owned tail"
        );
        assert_eq!(record.shell as *const _ as usize, address);
        assert_eq!(derived as *const _ as usize, derived_address);
        assert_eq!(seam.0, 1);
        let mut definitions = Vec::new();
        if outcome.is_ok() {
            let mut world = World {
                catalog: &catalog,
                record: &mut record,
                derived,
                context: Context {
                    actor_services: &mut seam,
                    world_version: &mut version,
                    registry_state: &mut registry_state,
                    restrictions: Some(&groups),
                    messages: &mut messages,
                    source_ordinal: 0,
                },
                source_pc,
                categories: None,
            };
            for category in native::RECRUIT_CATEGORIES {
                let mut cursor =
                    native::World::definition_category_head(&mut world, category).unwrap();
                while let Some(node) = cursor {
                    let token = native::World::definition_at_node(&mut world, node).unwrap();
                    let canonical = world.canonical(token).unwrap();
                    let mask = native::World::definition_component_mask(&mut world, token).unwrap();
                    let mut links = [None; 3];
                    let mut allowed = None;
                    if mask & (1 << 8) != 0 {
                        for (i, link) in [
                            native::RecruitLink::Primary,
                            native::RecruitLink::Secondary,
                            native::RecruitLink::Property17c,
                        ]
                        .into_iter()
                        .enumerate()
                        {
                            let present =
                                native::World::recruit_link_nonempty(&mut world, token, link)
                                    .unwrap();
                            links[i] = Some(present);
                            if present {
                                allowed = Some(
                                    native::World::recruit_allowed(&mut world, owner.raw(), token)
                                        .unwrap(),
                                );
                                break;
                            }
                        }
                    }
                    definitions.push(
                        json!({"definition":canonical,"token":token,"category":category,
                        "node":node,"mask":mask,"links":links,"allowed":allowed}),
                    );
                    cursor = native::World::next_definition_node(&mut world, node).unwrap();
                }
            }
        }
        let output = json!({"owner":owner.raw(),"definition":runtime.definition.proof.id,
            "source_precision":source_pc,"live_precision":pc,"initial":initial,
            "outcome":outcome,"world_version":version,"registry_initial":registry_initial,"registry_state":registry_state,
            "definitions":definitions,"resolved_color_index":derived.references.get(&0x1e8)
                .and_then(|v|v.as_ref()).map(|v|current_color_index(&catalog,v).unwrap()),
            "derived_words":derived.words,"derived_bytes":derived.bytes,
            "flows":derived.vectors.iter().map(|(k,v)|(k.to_string(),json!({"values":v.values,"capacity":v.capacity_hint})))
                .collect::<serde_json::Map<String,Value>>(),"recruit_items":derived.recruit_items,
            "messages":messages,"same_held_allocation":true,"object_init_calls":seam.0});
        rows.push(output);
        *attempt.registry.value_mut(owner).unwrap() = held;
    }
    let output = json!({"scope":"actual source-completed tutorial02 first Kingdom; explicit base Object Init seam, no whole-map success claim",
        "map_sha256":sha256(&map_bytes),"rows":rows});
    if let Some(path) = std::env::var_os("AA_KINGDOM_INIT_CAPTURE") {
        let path = std::path::PathBuf::from(path);
        assert!(path.is_absolute());
        serde_json::to_writer_pretty(
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .unwrap(),
            &output,
        )
        .unwrap();
    } else {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../../research/skirmish-20261016-load-kingdom-init/actual-first-kingdom-final.json",
        );
        let expected: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(output, expected);
    }
    println!("{output}");
}
