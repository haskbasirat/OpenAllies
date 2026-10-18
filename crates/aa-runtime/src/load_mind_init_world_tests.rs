use crate::catalog::RestoredMindParts;
use aa_actors::components::ComponentSlot;
use aa_mapworld::restore::{RestoredComponent, RestoredRecord};
use aa_numeric::Precision;
use serde_json::{Value, json};

#[test]
fn split_state_owner_replays_all4044_native_state_prefixes() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-saved-actor-state/native-saved-actor-state.json"
    ))
    .unwrap();
    let rows = fixture["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 4044);
    for row in rows {
        let input = &row["input"];
        let expected = &row["output"];
        let bits = input["bits"].as_u64().unwrap();
        let mut state = aa_actors::components::StateBits {
            low: bits as u32,
            high: (bits >> 32) as u32,
        };
        let mut flags = input["flags"].as_u64().unwrap() as u32;
        let result = crate::saved_actor_state::set_fields(
            aa_actors::ActorId::from_raw(4099),
            &mut state,
            &mut flags,
            input["index"].as_u64().unwrap() as u8,
            input["enabled"].as_bool().unwrap(),
        );
        assert_eq!(
            u64::from(state.low) | (u64::from(state.high) << 32),
            expected["bits"].as_u64().unwrap()
        );
        assert_eq!(json!(flags), expected["flags"]);
        if expected["completed"].as_bool().unwrap() {
            assert!(result.is_ok());
        } else {
            assert!(
                result
                    .unwrap_err()
                    .contains(expected["first_required_effect"].as_str().unwrap())
            );
        }
    }
}

#[test]
#[ignore = "requires AA_GAME_ROOT and verified capture03"]
fn capture_actual_first_mind_storage_before_init() {
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
    let map_bytes =
        std::fs::read(game.join("reconstruction/assets/original/Data/Maps/(2v2)Kursk.RMP"))
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
    let inactive = ScopedManagedLoad::from_sources(&mut sources, 0)
        .unwrap()
        .begin(TaskKind::Normal)
        .unwrap();
    let attempt = Attempt::load_with_source_setup(
        &map,
        &actors,
        settings,
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

    let mut rows = Vec::new();
    for e in attempt.registry.iter() {
        let RestoredRecord::Actor(v) = &e.value else {
            continue;
        };
        let Some(a) = v.shell.actor.as_ref() else {
            continue;
        };
        let Some(RestoredComponent::Mind(m)) = a.components.get(ComponentSlot::Mind) else {
            continue;
        };
        let p = m
            .state
            .as_any()
            .downcast_ref::<RestoredMindParts>()
            .expect("actual Mind owner differs");
        let core = p.mind.as_ref().expect("actual Mind core absent");
        let states:Vec<_>=core.stack.iter().map(|s| s.map(|id|json!({"id":id,"offset":p.installed.state_bindings.iter().find_map(|(off,v)|(*v==id).then_some(*off)),"state":p.states.get(&id)}))).collect();
        let state_statuses: std::collections::BTreeMap<_, _> = p
            .installed
            .state_bindings
            .iter()
            .map(|(offset, id)| (*offset, p.states.get(id).unwrap().status))
            .collect();
        rows.push(json!({"owner":e.id.raw(),"definition":v.definition.proof,"definition_mask":v.definition.component_mask,"mind_kind":format!("{:?}",p.installed.kind),"mind_id":p.installed.mind_id,"required_slot":m.required,"core":core,"stack_states":states,"state_low":a.components.state.low,"state_high":a.components.state.high,"thing_flags":v.shell.thing.flags,"installed_words":p.installed.words,"installed_bytes":p.installed.bytes,"command_count":p.commands.len(),"organization_present":a.components.present(ComponentSlot::Organization),"actor_word118":a.words.get(&0x118),"state_statuses":state_statuses}));
        if rows.len() == 4 {
            break;
        }
    }
    assert!(!rows.is_empty(), "actual map has no restored Mind");
    let result = json!({"scope":"Read-only actual stopped Kursk first four Mind owners before Init; no native load/init success claim", "map_sha256":sha256(&map_bytes),"rows":rows});
    if let Some(p) = std::env::var_os("AA_MIND_INIT_CAPTURE") {
        serde_json::to_writer_pretty(
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(p)
                .unwrap(),
            &result,
        )
        .unwrap();
    } else {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../research/skirmish-20261016-load-mind-init/actual-before-init-v2.json");
        let expected: Value = serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap();
        assert_eq!(result, expected);
    }
}

#[test]
#[ignore = "requires AA_GAME_ROOT and verified capture03"]
fn actual_four_minds_sync_same_owners_at_all_live_precisions() {
    use super::{RequiredEffects, after_init, init_existing, parts};
    use crate::{catalog::Catalog, live_object_init::Record};
    let mut captures = Vec::new();
    for live_pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
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
            sources::{SourceLimits, V14Sources},
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
        let map_bytes =
            std::fs::read(game.join("reconstruction/assets/original/Data/Maps/(2v2)Kursk.RMP"))
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
        let inactive = ScopedManagedLoad::from_sources(&mut sources, 0)
            .unwrap()
            .begin(TaskKind::Normal)
            .unwrap();
        let mut attempt = Attempt::load_with_source_setup(
            &map,
            &actors,
            settings,
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

        let ids: Vec<_> = attempt
            .registry
            .iter()
            .filter_map(|e| {
                let RestoredRecord::Actor(v) = &e.value else {
                    return None;
                };
                v.shell
                    .actor
                    .as_ref()?
                    .components
                    .get(ComponentSlot::Mind)
                    .map(|_| e.id)
            })
            .take(4)
            .collect();
        assert_eq!(ids.len(), 4);
        let mut catalog = Catalog::new(&actors);
        catalog.settings = Some(attempt.settings.clone());
        catalog.native_load_mode = attempt.native_load_mode;
        catalog.definition_text = Some(attempt.definition_text.clone());
        *catalog.definition_bank.borrow_mut() = attempt.definitions;
        *catalog.ability_arena.borrow_mut() = attempt.abilities;
        if let Some(inactive) = attempt.inactive_load.as_ref() {
            catalog.bind_inactive_load(inactive).unwrap();
        }
        let mut replayed_kinds = std::collections::BTreeSet::new();
        for id in ids {
            let mut held = std::mem::replace(
                attempt.registry.value_mut(id).unwrap(),
                RestoredRecord::Loading,
            );
            let RestoredRecord::Actor(v) = &mut held else {
                unreachable!()
            };
            let mut record = Record {
                owner: id,
                shell: &mut v.shell,
                definition: &v.definition,
                registry: &mut attempt.registry,
            };
            let pointer = parts(&record).unwrap() as *const _ as usize;
            let before = format!("{:?}", parts(&record).unwrap());
            let state_before = record.shell.actor.as_ref().unwrap().components.state;
            let flags_before = record.shell.thing.flags;
            let result = init_existing(
                &catalog,
                &attempt.inventory,
                &mut record,
                live_pc,
                &mut RequiredEffects,
            )
            .map_err(|e| e.to_string());
            assert_eq!(
                result,
                Ok(true),
                "the four actual restored animal sync paths must complete"
            );
            assert_eq!(
                parts(&record).unwrap() as *const _ as usize,
                pointer,
                "same allocated Mind must return on every outcome"
            );
            assert!(parts(&record).unwrap().mind.is_some());
            assert_eq!(
                format!("{:?}", parts(&record).unwrap()),
                before,
                "admitted pure sync/action masks must not change Mind/commands"
            );
            if matches!(result, Ok(true)) {
                after_init(&catalog, &mut record).unwrap();
            }
            captures.push(json!({"precision":live_pc,"owner":id.raw(),"definition":record.definition.proof.id,"kind":format!("{:?}",parts(&record).unwrap().installed.kind),"state_before":state_before,"flags_before":flags_before,"result":result,"state_after":record.shell.actor.as_ref().unwrap().components.state,"flags_after":record.shell.thing.flags,"actions":[record.shell.actor.as_ref().unwrap().words.get(&0xf0),record.shell.actor.as_ref().unwrap().words.get(&0xf4)],"same_mind_allocation_core_states_commands":true}));
            if replayed_kinds.insert(format!("{:?}", parts(&record).unwrap().installed.kind)) {
                replay_native_prefixes(&catalog, &attempt.inventory, &mut record, live_pc);
            }
            *attempt.registry.value_mut(id).unwrap() = held;
        }
    }
    let result = json!({"scope":"Actual stopped Kursk first four Mind sync calls with real held Actor action rebuilding; diagnostic restore does not certify whole record/world/battle. Source/construction Double53; each PC uses a fresh actual restore.","rows":captures});
    if let Some(p) = std::env::var_os("AA_MIND_SYNC_CAPTURE") {
        serde_json::to_writer_pretty(
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(p)
                .unwrap(),
            &result,
        )
        .unwrap();
    } else {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../research/skirmish-20261016-load-mind-init/actual-four-sync-v2.json");
        let expected: Value = serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap();
        assert_eq!(result, expected);
    }
}

/// Native boundary replay on existing typed allocations. Only labelled input
/// fields/stack references are changed in this test; no component is constructed
/// and each original owner is restored before the real registry receives it.
fn replay_native_prefixes(
    catalog: &crate::catalog::Catalog<'_>,
    inventory: &aa_mapworld::WorldInventory,
    record: &mut crate::live_object_init::Record<'_>,
    pc: Precision,
) {
    use super::{Effects, init_existing, parts, parts_mut};
    use aa_actors::{actor_creation, state};
    use aa_mind::construct::Kind;
    struct StopAtChild;
    fn stop(label: &str) -> actor_creation::Result<()> {
        Err(actor_creation::Error::World(label.to_owned()))
    }
    impl Effects for StopAtChild {
        fn rebuild_actions(
            &mut self,
            _: &crate::catalog::Catalog<'_>,
            _: &aa_mapworld::WorldInventory,
            r: &mut crate::live_object_init::Record<'_>,
            _: Precision,
        ) -> actor_creation::Result<()> {
            assert!(
                parts(r).unwrap().mind.is_some(),
                "callback sees the actual returned Mind core"
            );
            stop("required4D1350")
        }
        fn enter(
            &mut self,
            _: &crate::catalog::Catalog<'_>,
            r: &mut crate::live_object_init::Record<'_>,
            _: &'static state::Descriptor,
            _: Option<&'static state::Descriptor>,
            _: Precision,
        ) -> actor_creation::Result<()> {
            assert!(parts(r).unwrap().mind.is_some());
            stop("required4D4A90")
        }
        fn leave(
            &mut self,
            _: &crate::catalog::Catalog<'_>,
            r: &mut crate::live_object_init::Record<'_>,
            _: &'static state::Descriptor,
            _: Precision,
        ) -> actor_creation::Result<()> {
            assert!(parts(r).unwrap().mind.is_some());
            stop("required4D16B0")
        }
        fn reset_default(
            &mut self,
            _: &crate::catalog::Catalog<'_>,
            r: &mut crate::live_object_init::Record<'_>,
            kind: Kind,
            _: Precision,
        ) -> actor_creation::Result<()> {
            assert!(parts(r).unwrap().mind.is_some());
            stop(if kind == Kind::Element {
                "requiredResetElement"
            } else {
                "requiredResetRegiment"
            })
        }
    }
    let fixture: Value = serde_json::from_slice(
        &std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../research/skirmish-20261016-load-mind-init/native-mind-init.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(fixture["rows"].as_array().unwrap().len(), 903);
    let p = parts(record).unwrap();
    let kind = p.installed.kind;
    let pointer = p as *const _ as usize;
    let original_stack = p.mind.as_ref().unwrap().stack.clone();
    let original_statuses: Vec<_> = p.states.iter().map(|(id, s)| (*id, s.status)).collect();
    let original_bits = record.shell.actor.as_ref().unwrap().components.state;
    let original_flags = record.shell.thing.flags;
    let original_word118 = record
        .shell
        .actor
        .as_ref()
        .unwrap()
        .words
        .get(&0x118)
        .copied();
    let mut organization = record
        .shell
        .actor
        .as_mut()
        .unwrap()
        .components
        .remove(ComponentSlot::Organization);
    let mut count = 0;
    for row in fixture["rows"].as_array().unwrap() {
        let c = &row["input"];
        if row["precision"] != json!(pc)
            || c["kind"].as_str().unwrap()
                != if kind == Kind::Element {
                    "element"
                } else {
                    "regiment"
                }
        {
            continue;
        }
        let desired_org = c["organization"].as_bool().unwrap() && kind == Kind::Regiment;
        let a = record.shell.actor.as_mut().unwrap();
        if desired_org && !a.components.present(ComponentSlot::Organization) {
            a.components.insert(
                ComponentSlot::Organization,
                organization
                    .take()
                    .expect("native Regiment probes use the actual saved Organization allocation"),
            );
        } else if !desired_org && a.components.present(ComponentSlot::Organization) {
            organization = a.components.remove(ComponentSlot::Organization);
        }
        let bits = c["bits"].as_u64().unwrap();
        a.components.state.low = bits as u32;
        a.components.state.high = (bits >> 32) as u32;
        if let Some(word) = c["word118"].as_u64() {
            a.words.insert(0x118, word as u32);
        } else {
            a.words.remove(&0x118);
        }
        record.shell.thing.flags = c["flags"].as_u64().unwrap() as u32;
        let p = parts_mut(record).unwrap();
        for (id, status) in &original_statuses {
            p.states.get_mut(id).unwrap().status = *status;
        }
        if let Some(statuses) = c["state_statuses"].as_object() {
            for (offset, status) in statuses {
                let id = p.installed.state_bindings[&offset.parse::<u32>().unwrap()];
                p.states.get_mut(&id).unwrap().status = status.as_u64().unwrap() as u32;
            }
        } else {
            let disable = p.installed.state_bindings[&0x34];
            p.states.get_mut(&disable).unwrap().status =
                c["disable_status"].as_u64().unwrap() as u32;
        }
        let core = p.mind.as_mut().unwrap();
        core.stack.clear();
        if let Some(offset) = c["offset"].as_u64() {
            core.stack
                .push_back(Some(p.installed.state_bindings[&(offset as u32)]));
        }
        let storage_before = format!("{:?}", p);
        let result = init_existing(catalog, inventory, record, pc, &mut StopAtChild)
            .map_err(|e| e.to_string());
        let expected = &row["native"];
        let outcome = expected["outcome"].as_str().unwrap();
        if outcome == "complete" {
            assert_eq!(result, Ok(true), "{row}");
        } else {
            let error = result.unwrap_err();
            assert!(
                error.contains(if outcome == "requiredStateVirtual20" {
                    "actual state virtual20 is required"
                } else {
                    outcome
                }),
                "{row}: {error}"
            );
        }
        let actual = record.shell.actor.as_ref().unwrap().components.state;
        assert_eq!(
            u64::from(actual.low) | (u64::from(actual.high) << 32),
            expected["bits"].as_u64().unwrap(),
            "{row}"
        );
        assert_eq!(json!(record.shell.thing.flags), expected["flags"], "{row}");
        assert_eq!(parts(record).unwrap() as *const _ as usize, pointer);
        assert_eq!(
            format!("{:?}", parts(record).unwrap()),
            storage_before,
            "{row}"
        );
        count += 1;
    }
    assert!(count > 0);
    let a = record.shell.actor.as_mut().unwrap();
    if let Some(org) = organization.take() {
        assert!(
            a.components
                .insert(ComponentSlot::Organization, org)
                .is_none()
        );
    }
    a.components.state = original_bits;
    if let Some(word) = original_word118 {
        a.words.insert(0x118, word);
    } else {
        a.words.remove(&0x118);
    }
    record.shell.thing.flags = original_flags;
    let p = parts_mut(record).unwrap();
    p.mind.as_mut().unwrap().stack = original_stack;
    for (id, status) in original_statuses {
        p.states.get_mut(&id).unwrap().status = status;
    }
}
