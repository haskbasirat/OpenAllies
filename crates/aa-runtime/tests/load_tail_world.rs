//! Actual ActorStore binding against the existing original-x86 load-tail
//! oracle. These are synthetic native fixture states, not a real-map startup.
use aa_actors::{
    ActorId, IdentityRegistry,
    components::ComponentSlot,
    ground_states::{ElementState, LayoutDefinition, OrganizationState},
    loading::{LoadKind, construct_shell_with_components},
    owned::Component,
    post_load::Error,
    world_load_tail::LoadTailWorld,
};
use aa_body::F32;
use aa_mapworld::{
    ThingState,
    bootstrap::DefinitionProof,
    inventory::{ByteSpan, Placement},
    kingdoms::{KingdomPayload, KingdomRecord},
    restore::{
        LoadCompletion, LoadedEntity, RestoredComponent, RestoredRecord, RuntimeDefinition,
        SourceWorldEntity, WorldEntityRuntime,
    },
    special_state::SpecialShell,
};
use aa_numeric::Precision;
use aa_rules::{native::CategoryDescriptor, resources::Provenance, tgi::Span};
use aa_runtime::{
    actor_world::{ActorStore, Source},
    load_tail_world::{CameraState, LocalPlayer, SessionFacts, World, WorldFacts},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

const FIXTURE: &str =
    include_str!("../../../research/phase8-saved-components/native-load-tail.json");

fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn text(v: &Value) -> Vec<u16> {
    v.as_str().unwrap().encode_utf16().collect()
}
fn span() -> ByteSpan {
    ByteSpan {
        offset: 0,
        length: 0,
    }
}

fn definition(label: &str, class: u32) -> RuntimeDefinition {
    RuntimeDefinition {
        proof: DefinitionProof {
            id: label.into(),
            source: Provenance {
                path: "native-load-tail.json synthetic fixture".into(),
                sha256: String::new(),
                span: Span { start: 0, end: 0 },
            },
            template_chain: vec![],
            category: CategoryDescriptor {
                name: "fixture",
                registry_index: 0,
                native_kind: 2,
                runtime_class: class,
                view_type: 0,
            },
            base_flag_inputs: vec![],
        },
        native_class: class,
        flags: 0,
        object: None,
        component_mask: None,
        ground: Default::default(),
    }
}

fn thing(name: Vec<u16>, position: [u32; 3], flags: u32) -> ThingState {
    ThingState {
        version: 2,
        name_utf16: name,
        word_1c: 0,
        position_bits: position,
        facing_bits: [0, 0, 0],
        serialized_flags: flags,
        legacy_version_one_word: None,
    }
}

struct Fixture {
    store: ActorStore,
    ids: BTreeMap<String, ActorId>,
    session: SessionFacts,
    slots: Vec<Option<ActorId>>,
    members: BTreeMap<ActorId, Vec<ActorId>>,
    priorities: BTreeMap<String, i32>,
}

impl Fixture {
    fn new(c: &Value) -> Self {
        let mut ids = BTreeMap::new();
        for (i, o) in c["kingdoms"]
            .as_array()
            .unwrap()
            .iter()
            .chain(c["actors"].as_array().unwrap())
            .enumerate()
        {
            ids.insert(
                o["label"].as_str().unwrap().to_owned(),
                ActorId::from_raw(i as u32 + 1),
            );
        }
        let mut registry = IdentityRegistry::new();
        let mut members = BTreeMap::new();
        let mut priorities = BTreeMap::new();
        for k in c["kingdoms"].as_array().unwrap() {
            let label = k["label"].as_str().unwrap();
            let id = ids[label];
            let def = definition(label, 4);
            let mut shell = construct_shell_with_components::<_, RestoredComponent>(
                LoadKind::Actor,
                def.proof.clone(),
            );
            shell.thing.name_utf16 = text(&k["name"]);
            let mut derived = SpecialShell::kingdom(id, 0).unwrap();
            if let Some(capital) = k["f8"].as_str() {
                derived
                    .identity_references
                    .insert(0x1f8, Some(ids[capital]));
                derived.words.remove(&0x1f8);
            }
            // Only source metadata is synthetic here; the actual runtime fields
            // being tested are stored in WorldEntityRuntime / SpecialShell.
            let payload: KingdomPayload = serde_json::from_value(json!({
                "version": 0, "word_188": 0, "word_18c": 0, "word_190": 1,
                "display_key_utf16": [], "color_reference_utf16": [],
                "words_1d0": {"serialized_count":0,"values":[]},
                "resources_1c4": {"serialized_count":0,"values":[]}
            }))
            .unwrap();
            let source = KingdomRecord {
                source_span: span(),
                definition_id_utf16: label.encode_utf16().collect(),
                serialized_id: id.raw(),
                thing: thing(text(&k["name"]), [0; 3], 0),
                object: None,
                actor: None,
                updatable: None,
                payload,
            };
            registry
                .register_saved(
                    id,
                    true,
                    RestoredRecord::Kingdom(Box::new(SourceWorldEntity {
                        source,
                        completion: LoadCompletion::PostLoaded,
                        failure: None,
                        runtime: Some(WorldEntityRuntime {
                            definition: def,
                            base: shell,
                            derived: Some(derived),
                            actor_tail: None,
                            derived_payload_loaded: true,
                        }),
                    })),
                )
                .unwrap();
            members.insert(
                id,
                k["actors"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|a| ids[a.as_str().unwrap()])
                    .collect(),
            );
        }
        for a in c["actors"].as_array().unwrap() {
            let label = a["label"].as_str().unwrap();
            let id = ids[label];
            let def = definition(label, 3);
            let mut shell = construct_shell_with_components::<_, RestoredComponent>(
                LoadKind::Actor,
                def.proof.clone(),
            );
            let position = [u(&a["xy"][0]), u(&a["xy"][1]), 0];
            let flags = a.get("flags").map_or(0, u);
            shell.thing.position_bits = position;
            shell.thing.flags = flags;
            if a.get("organization_component").and_then(Value::as_bool) == Some(true) {
                let state = OrganizationState::new(
                    id,
                    LayoutDefinition {
                        definition: "fixture-layout".into(),
                        parameters: vec![],
                        slots: 0,
                    },
                    F32::ZERO,
                    F32::ZERO,
                    [F32(position[0]), F32(position[1])],
                )
                .unwrap();
                shell.actor.as_mut().unwrap().components.insert(
                    ComponentSlot::Organization,
                    Component::Organization(Box::new(state)),
                );
            }
            if let Some(org) = a.get("element_org").and_then(Value::as_str) {
                let mut element = ElementState::new(id).unwrap();
                element.organization = Some(ids[org]);
                shell.actor.as_mut().unwrap().components.insert(
                    ComponentSlot::Element,
                    Component::Element(Box::new(element)),
                );
            }
            // Settlement has no owned Component variant yet. The two fixture
            // cases that would read a true Settlement presence are excluded
            // below; no fake payload is installed in its slot.
            priorities.insert(
                label.into(),
                a.get("p").map_or(0, |v| v.as_i64().unwrap() as i32),
            );
            let source = Placement {
                record_ordinal: 0,
                source_span: span(),
                thing_id_utf16: label.encode_utf16().collect(),
                class_code: 3,
                serialized_id: id.raw(),
                thing: thing(vec![], position, flags),
                object: None,
                actor: None,
                opaque_tail_span: span(),
                opaque_tail: vec![],
            };
            registry
                .register_saved(
                    id,
                    true,
                    RestoredRecord::Actor(Box::new(LoadedEntity {
                        source,
                        definition: def,
                        shell,
                        completion: LoadCompletion::PostLoaded,
                        owner_source_slot: None,
                        saved_components: vec![],
                        actor_tail: None,
                        doodad: None,
                        special_object: None,
                        failure: None,
                        active_component: None,
                    })),
                )
                .unwrap();
        }
        let player = &c["player"];
        Self {
            store: ActorStore::from_restored(registry).unwrap(),
            session: SessionFacts {
                application_state: u(&c["app_state"]),
                local_player: (!player.is_null()).then(|| LocalPlayer {
                    name_utf16: text(&player["name"]),
                    kingdom: player
                        .get("kingdom")
                        .and_then(Value::as_str)
                        .map(|k| ids[k]),
                }),
            },
            slots: c["world_kingdoms"]
                .as_array()
                .unwrap()
                .iter()
                .map(|k| k.as_str().map(|k| ids[k]))
                .collect(),
            ids,
            members,
            priorities,
        }
    }

    fn world<'a>(&'a mut self, c: &Value, camera: &'a mut CameraState) -> World<'a> {
        World {
            actors: &mut self.store,
            world_start: None,
            session: &self.session,
            facts: WorldFacts {
                map_settings_mode: u(&c["mode"]),
                world_start_flag: Some(c["flag_59e520"].as_bool().unwrap()),
                world_word_48: Some(u(&c["world_48"])),
                map_size_bits: Some([u(&c["map"][0]), u(&c["map"][1])]),
                kingdom_slots: Some(&self.slots),
                kingdom_actors: &self.members,
                definition_priorities: &self.priorities,
            },
            camera,
            precision: match c.get("precision").and_then(Value::as_str) {
                Some("Single24") => Precision::Single24,
                Some("Extended64") => Precision::Extended64,
                _ => Precision::Double53,
            },
        }
    }
}

fn camera() -> CameraState {
    CameraState {
        look_at: None,
        messages: vec![],
    }
}
fn input(name: &str) -> Value {
    let document: Value = serde_json::from_str(FIXTURE).unwrap();
    document["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["input"]["name"] == name)
        .unwrap()["input"]
        .clone()
}

#[test]
fn canonical_store_binding_matches_supported_original_load_tail_cases() {
    let document: Value = serde_json::from_str(FIXTURE).unwrap();
    assert_eq!(document["passed"], true);
    let mut matched = 0;
    let mut blocked_world_start = 0;
    let mut unavailable_settlement = 0;
    for case in document["cases"].as_array().unwrap() {
        let c = &case["input"];
        let name = c["name"].as_str().unwrap();
        if matches!(name, "by_name_settlement_wins" | "player_kingdom") {
            unavailable_settlement += 1;
            continue;
        }
        let mut fixture = Fixture::new(c);
        let mut state = camera();
        let original = state.clone();
        let result = fixture.world(c, &mut state).after_post_load();
        if matches!(name, "world_start_mode1" | "world_start_mode4") {
            assert!(
                matches!(result, Err(Error::Unsupported(message)) if message.contains("58F010") && message.contains("58F530")),
                "{name}"
            );
            assert_eq!(
                state, original,
                "{name}: unsupported world start must not initialize the camera"
            );
            blocked_world_start += 1;
            continue;
        }
        if case["native"]["outcome"]["kind"] == "NullDereference" {
            assert!(
                matches!(result, Err(Error::NullDereference(_))),
                "{name}: {result:?}"
            );
            assert_eq!(state, original);
        } else {
            result.unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(
                json!(state.look_at.unwrap()),
                case["native"]["look_at"],
                "{name}"
            );
            let native_log = case["native"]["calls"]
                .as_array()
                .unwrap()
                .iter()
                .find(|call| call["seam"] == "404F60")
                .unwrap();
            assert_eq!(state.messages.len(), 1, "{name}");
            let log = &state.messages[0];
            assert_eq!(log.line, u(&native_log["line"]), "{name}");
            assert_eq!(log.function, native_log["function"].as_str().unwrap());
            assert_eq!(log.format, native_log["format"].as_str().unwrap());
            assert_eq!(
                format!("{:016x}", log.x_bits),
                native_log["x"].as_str().unwrap(),
                "{name}"
            );
            assert_eq!(
                format!("{:016x}", log.y_bits),
                native_log["y"].as_str().unwrap(),
                "{name}"
            );
        }
        matched += 1;
    }
    assert_eq!(
        (matched, blocked_world_start, unavailable_settlement),
        (20, 2, 2)
    );
}

#[test]
fn actual_store_name_flags_pose_and_element_identity_are_authoritative() {
    let c = input("by_name_organization");
    let mut f = Fixture::new(&c);
    let mut state = camera();
    f.world(&c, &mut state).after_post_load().unwrap();
    let org = f.ids["org"];
    let key = f.store.key(org).unwrap();
    let position = [3.5f32.to_bits(), (-9.0f32).to_bits()];
    f.store.staged_placement_mut(key).unwrap().position_bits[..2].copy_from_slice(&position);
    let mut state = camera();
    f.world(&c, &mut state).after_post_load().unwrap();
    assert_eq!(state.look_at, Some(position));

    let member = f.ids["m1"];
    let Component::Element(element) = f
        .store
        .component_mut(member, ComponentSlot::Element)
        .unwrap()
        .unwrap()
    else {
        panic!()
    };
    element.organization = Some(ActorId::from_raw(org.raw() | 0x10000));
    let mut state = camera();
    assert!(f.world(&c, &mut state).after_post_load().is_err());
    assert!(state.look_at.is_none() && state.messages.is_empty());

    let kingdom = f.store.key(f.ids["allies"]).unwrap();
    f.store.object_mut(kingdom).unwrap().thing.name_utf16 = "renamed".encode_utf16().collect();
    // No matching local Kingdom means the stale Element above is never read.
    f.world(&c, &mut state).after_post_load().unwrap();
    assert_eq!(state.look_at, Some([1157627904, 1149239296]));
}

#[test]
fn membership_order_and_live_visibility_flags_preserve_the_first_priority_tie() {
    let c = input("by_name_flagged_wins");
    let mut f = Fixture::new(&c);
    let mut state = camera();
    f.world(&c, &mut state).after_post_load().unwrap();
    assert_eq!(state.look_at, Some([1106247680, 1109393408]));
    f.members.get_mut(&f.ids["allies"]).unwrap().swap(0, 1);
    f.world(&c, &mut state).after_post_load().unwrap();
    assert_eq!(state.look_at, Some([1106771968, 1109655552]));
    let f2 = f.store.key(f.ids["f2"]).unwrap();
    f.store.staged_placement_mut(f2).unwrap().flags |= 1 << 22;
    f.world(&c, &mut state).after_post_load().unwrap();
    assert_eq!(state.look_at, Some([1106247680, 1109393408]));
}

#[test]
fn missing_host_facts_are_errors_only_on_the_reached_native_path() {
    let c = input("app_state_3");
    let mut f = Fixture::new(&c);
    let mut state = camera();
    let mut world = f.world(&c, &mut state);
    world.facts.world_word_48 = None;
    world.facts.world_start_flag = None;
    world.facts.kingdom_slots = None;
    world.after_post_load().unwrap();
    assert!(state.look_at.is_some());

    let mut state = camera();
    let mut world = f.world(&c, &mut state);
    world.facts.map_size_bits = None;
    assert!(matches!(
        world.after_post_load(),
        Err(Error::Unsupported(_))
    ));
    assert_eq!(state, camera());

    let c = input("by_name_flagged_wins");
    let mut f = Fixture::new(&c);
    for missing in 0..4 {
        let absent = BTreeMap::new();
        let absent_priorities = BTreeMap::new();
        let mut state = camera();
        let mut world = f.world(&c, &mut state);
        match missing {
            0 => world.facts.world_word_48 = None,
            1 => world.facts.kingdom_slots = None,
            2 => world.facts.kingdom_actors = &absent,
            3 => world.facts.definition_priorities = &absent_priorities,
            _ => unreachable!(),
        }
        assert!(world.after_post_load().is_err());
        assert_eq!(state, camera());
    }
    let mut state = camera();
    let mut world = f.world(&c, &mut state);
    world.facts.map_settings_mode = 4;
    world.facts.world_start_flag = None;
    assert!(matches!(
        world.after_post_load(),
        Err(Error::Unsupported(_))
    ));
    assert_eq!(state, camera());
}

#[test]
fn staged_completion_and_unresolved_capital_words_cannot_be_promoted() {
    let c = input("by_name_f8");
    let mut f = Fixture::new(&c);
    let kingdom = f.store.key(f.ids["allies"]).unwrap();
    let Source::Special(s) = &mut f.store.entity_mut(kingdom).unwrap().source else {
        panic!()
    };
    s.completion = LoadCompletion::AwaitingPostLoad;
    let mut state = camera();
    assert!(f.world(&c, &mut state).after_post_load().is_err());
    assert_eq!(state, camera());

    let Source::Special(s) = &mut f.store.entity_mut(kingdom).unwrap().source else {
        panic!()
    };
    s.completion = LoadCompletion::PostLoaded;
    s.failure = Some("earlier stage failed".into());
    assert!(f.world(&c, &mut state).after_post_load().is_err());
    assert_eq!(state, camera());
    let Source::Special(s) = &mut f.store.entity_mut(kingdom).unwrap().source else {
        panic!()
    };
    s.failure = None;
    let shell = s.derived.as_mut().unwrap();
    shell.identity_references.remove(&0x1f8);
    shell.words.insert(0x1f8, 0x12345678); // unresolved pointer is not a GTID
    assert!(matches!(
        f.world(&c, &mut state).after_post_load(),
        Err(Error::Unsupported(_))
    ));
    assert_eq!(state, camera());
}

#[test]
fn camera_store_precedes_log_and_unreadable_objects_do_not_change_it() {
    let c = input("by_name_flagged_wins");
    let mut f = Fixture::new(&c);
    let mut state = camera();
    let mut world = f.world(&c, &mut state);
    let xy = [(-0.0f32).to_bits(), 1.0f32.to_bits()];
    world.set_look_at(xy).unwrap();
    assert_eq!(world.camera.look_at, Some(xy));
    assert!(world.camera.messages.is_empty());
    assert!(world.object_position(0x10001).is_err());
    assert_eq!(world.camera.look_at, Some(xy));
    world.log_look_at(xy).unwrap();
    assert_eq!(world.camera.messages[0].x_bits, (-0.0f64).to_bits());
}

#[test]
fn explicit_world_start_binding_reaches_the_native_camera_tail() {
    // This original oracle case has no objects/kingdoms. It tests the real
    // startup gate and call plumbing, not an invented playable empty map.
    let c = input("world_start_mode4");
    let mut fixture = Fixture::new(&c);
    let source = aa_rules::actors::native_load::load_files(
        Vec::new(), aa_rules::tgi::Limits::default(),
        &mut aa_rules::actors::native_load::NativeLoadReport::default(),
    );
    let resources = aa_economy::Catalog { resources: Vec::new() };
    let formulas = aa_economy::statistics_live::DerivedCatalog { rows: Vec::new() };
    let mut economy = BTreeMap::new();
    let mut started = false;
    let mut state = camera();
    {
        let mut world = fixture.world(&c, &mut state);
        world.world_start = Some(aa_runtime::load_tail_world::WorldStartFacts {
            economy: &mut economy, started: &mut started, major_kingdoms: &[],
            source: &source, resources: &resources, formulas: &formulas, source_precision: Precision::Double53,
        });
        world.after_post_load().unwrap();
    }
    assert!(started);
    let document: Value = serde_json::from_str(FIXTURE).unwrap();
    let native = document["cases"].as_array().unwrap().iter()
        .find(|row| row["input"]["name"] == "world_start_mode4").unwrap();
    assert_eq!(json!(state.look_at.unwrap()), native["native"]["look_at"]);
    assert_eq!(state.messages.len(), 1);
}
