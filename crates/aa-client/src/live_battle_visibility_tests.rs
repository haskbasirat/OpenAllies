//! Explicit native getter fixtures, not fabricated whole-map startup evidence.
use super::*;
use aa_actors::{
    IdentityRegistry,
    ground_states::{ElementState, LayoutDefinition, OrganizationState},
    loading::construct_shell_with_components,
};
use aa_body::F32;
use aa_mapworld::{
    bootstrap::DefinitionProof,
    inventory::{ByteSpan, Placement, ThingState},
    kingdoms::KingdomRecord,
    restore::{
        LoadCompletion, LoadedEntity, RestoredComponent, RestoredRecord, RuntimeDefinition,
        SourceWorldEntity, WorldEntityRuntime,
    },
};
use aa_rules::{native::CategoryDescriptor, resources::Provenance, tgi::Span};
use aa_spatial::collision::Grid;
use serde_json::{Value, json};
use std::cell::RefCell;

fn id(n: u32) -> ActorId {
    ActorId::from_raw(n)
}
fn number(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn definition(class: u32) -> RuntimeDefinition {
    RuntimeDefinition {
        proof: DefinitionProof {
            id: format!("explicit-native-visibility-fixture-{class}"),
            source: Provenance {
                path: "isolated native getter input".into(),
                sha256: String::new(),
                span: Span { start: 0, end: 0 },
            },
            template_chain: vec![],
            category: CategoryDescriptor {
                name: "native visibility fixture",
                registry_index: 0,
                native_kind: if matches!(class, 3 | 4) { 2 } else { 19 },
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
fn thing() -> ThingState {
    ThingState {
        version: 2,
        name_utf16: vec![],
        word_1c: 0,
        position_bits: [0; 3],
        facing_bits: [0, 0xbf800000, 0],
        serialized_flags: 0,
        legacy_version_one_word: None,
    }
}
fn span() -> ByteSpan {
    ByteSpan {
        offset: 0,
        length: 0,
    }
}
fn fixture_store(c: &Value) -> ActorStore {
    let mut registry = IdentityRegistry::new();
    for (key, value) in c["actors"].as_object().unwrap() {
        let aid = id(key.parse().unwrap());
        let class = number(&value["class"]);
        let def = definition(class);
        let kind = if matches!(class, 3 | 4) {
            LoadKind::Actor
        } else {
            LoadKind::Object
        };
        let mut shell =
            construct_shell_with_components::<_, RestoredComponent>(kind, def.proof.clone());
        shell.thing.flags = number(&value["flags"]);
        shell.thing.facing_bits = [0, 0xbf800000, 0];
        let object = shell.object.as_mut().unwrap();
        object.scale_bits = 1f32.to_bits();
        object.words_50_to_54[1] = number(&value["detection"]);
        if let Some(actor) = shell.actor.as_mut() {
            if let Some(members) = value["members"].as_array() {
                let mut org = OrganizationState::new(
                    aid,
                    LayoutDefinition {
                        definition: "explicit-native-layout".to_owned(),
                        parameters: vec![],
                        slots: members.len(),
                    },
                    F32::ZERO,
                    F32::ZERO,
                    [F32::ZERO; 2],
                )
                .unwrap();
                org.members = members.iter().map(|v| id(number(v))).collect();
                actor.components.insert(
                    ComponentSlot::Organization,
                    Component::Organization(Box::new(org)),
                );
            }
            if let Some(parent) = value["element"].as_u64() {
                let mut element = ElementState::<String>::new(aid).unwrap();
                element.organization = Some(id(parent as u32));
                actor.components.insert(
                    ComponentSlot::Element,
                    Component::Element(Box::new(element)),
                );
            }
        }
        let source = Placement {
            record_ordinal: 0,
            source_span: span(),
            thing_id_utf16: vec![],
            class_code: class,
            serialized_id: aid.raw(),
            thing: thing(),
            object: None,
            actor: None,
            opaque_tail_span: span(),
            opaque_tail: vec![],
        };
        let value = Box::new(LoadedEntity {
            source,
            definition: def,
            shell,
            completion: LoadCompletion::RegisteredLoadShell,
            owner_source_slot: None,
            saved_components: vec![],
            actor_tail: None,
            doodad: None,
            special_object: None,
            failure: None,
            active_component: None,
        });
        registry
            .register_saved(
                aid,
                matches!(class, 3 | 4),
                if matches!(class, 3 | 4) {
                    RestoredRecord::Actor(value)
                } else {
                    RestoredRecord::Object(value)
                },
            )
            .unwrap();
    }
    let kid = id(10);
    let def = definition(4);
    let base =
        construct_shell_with_components::<_, RestoredComponent>(LoadKind::Actor, def.proof.clone());
    let mut derived = SpecialShell::kingdom(kid, 0).unwrap();
    derived.words.insert(0x190, number(&c["nonmajor"]));
    derived.words.insert(0x228, number(&c["shared_mask"]));
    let payload=serde_json::from_value(json!({"version":0,"word_188":0,"word_18c":0,"word_190":number(&c["nonmajor"]),
        "display_key_utf16":[],"color_reference_utf16":[],"words_1d0":{"serialized_count":0,"values":[]},"resources_1c4":{"serialized_count":0,"values":[]}})).unwrap();
    let source = KingdomRecord {
        source_span: span(),
        definition_id_utf16: vec![],
        serialized_id: 10,
        thing: thing(),
        object: None,
        actor: None,
        updatable: None,
        payload,
    };
    registry
        .register_saved(
            kid,
            true,
            RestoredRecord::Kingdom(Box::new(SourceWorldEntity {
                source,
                completion: LoadCompletion::RegisteredLoadShell,
                failure: None,
                runtime: Some(WorldEntityRuntime {
                    definition: def,
                    base,
                    derived: Some(derived),
                    actor_tail: None,
                    derived_payload_loaded: false,
                }),
            })),
        )
        .unwrap();
    ActorStore::from_restored(registry).unwrap()
}
struct View<'a> {
    c: &'a Value,
    reads: RefCell<Vec<&'static str>>,
    fail: Option<&'static str>,
}
impl View<'_> {
    fn read(&self, name: &'static str, value: bool) -> Result<bool> {
        self.reads.borrow_mut().push(name);
        ensure!(self.fail != Some(name), "required view owner {name}");
        Ok(value)
    }
}
impl CurrentView for View<'_> {
    fn kingdom(&self) -> Result<Option<ActorId>> {
        Ok(self.c["kingdom"].as_bool().unwrap().then_some(id(10)))
    }
    fn fog_enabled(&self) -> Result<bool> {
        self.read("fog", number(&self.c["fog"]) != 0)
    }
    fn current_thread_is_simulation(&self) -> Result<bool> {
        self.read("thread", self.c["simulation_thread"].as_bool().unwrap())
    }
    fn application_state(&self) -> Result<u32> {
        self.reads.borrow_mut().push("application");
        Ok(number(&self.c["application_state"]))
    }
    fn editor_option_enabled(&self) -> Result<bool> {
        self.read("editor_option", self.c["editor_option"].as_bool().unwrap())
    }
    fn world_editor_override(&self) -> Result<bool> {
        self.read("editor_world", self.c["editor_world"].as_bool().unwrap())
    }
}
fn index() -> SpatialWorld {
    SpatialWorld::new(
        Grid {
            dimensions: [4, 4],
            coordinate_shift: 2,
            cell_size_bits: 4f32.to_bits(),
        },
        vec![0; 16],
        Precision::Double53,
    )
    .unwrap()
}

#[test]
fn original_v14_all_pc_visibility_reads_actual_component_and_kingdom_owners() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-live-battle-scene/native-visibility.json"
    ))
    .unwrap();
    let cases = data["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 408);
    for case in cases {
        let c = &case["input"];
        let actors = fixture_store(c);
        let spatial = index();
        let view = View {
            c,
            reads: RefCell::new(vec![]),
            fail: None,
        };
        let kernel = NativeVisibility::new(&view);
        let target = c["kingdom"].as_bool().unwrap().then_some(id(10));
        let deep = c["deep"].as_bool().unwrap();
        let result = match c["method"].as_str().unwrap() {
            "visible" => kernel.visible(&actors, &spatial, id(1), deep),
            "visible_to" => kernel.visible_to(&actors, id(1), target, deep),
            "explored_by" => kernel.explored_by(&actors, id(1), target, deep),
            _ => panic!(),
        }
        .unwrap();
        assert_eq!(result, case["output"]["result"].as_bool().unwrap(), "{c}");
        assert_eq!(
            serde_json::to_value(view.reads.borrow().as_slice()).unwrap(),
            case["output"]["view_reads"],
            "{c}"
        );
    }
}

#[test]
fn visibility_reloads_current_words_and_refuses_missing_reached_view_owners() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-live-battle-scene/native-visibility.json"
    ))
    .unwrap();
    let mut c = data["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| {
            r["input"]["method"] == "visible_to"
                && r["input"]["actors"]["1"]["class"] == 3
                && r["input"]["fog"] == 1
                && r["input"]["kingdom"] == true
                && r["input"]["nonmajor"] == 0
                && r["input"]["shared_mask"] == 65537
        })
        .unwrap()["input"]
        .clone();
    c["actors"]["1"]["detection"] = json!(0);
    let mut actors = fixture_store(&c);
    let view = View {
        c: &c,
        reads: RefCell::new(vec![]),
        fail: None,
    };
    let kernel = NativeVisibility::new(&view);
    assert!(
        !kernel
            .visible_to(&actors, id(1), Some(id(10)), false)
            .unwrap()
    );
    actors
        .object_mut(actors.key(id(1)).unwrap())
        .unwrap()
        .object
        .as_mut()
        .unwrap()
        .words_50_to_54[1] = 0x10000;
    assert!(
        kernel
            .visible_to(&actors, id(1), Some(id(10)), false)
            .unwrap()
    );
    let kid = actors.key(id(10)).unwrap();
    let Source::Special(k) = &mut actors.entity_mut(kid).unwrap().source else {
        panic!()
    };
    k.derived.as_mut().unwrap().words.insert(0x228, 0);
    assert!(
        !kernel
            .visible_to(&actors, id(1), Some(id(10)), false)
            .unwrap()
    );
    let view = View {
        c: &c,
        reads: RefCell::new(vec![]),
        fail: Some("fog"),
    };
    assert!(
        NativeVisibility::new(&view)
            .visible_to(&actors, id(1), Some(id(10)), false)
            .unwrap_err()
            .to_string()
            .contains("required view owner")
    );
    assert!(
        NativeVisibility::new(&view)
            .visible_to(&actors, id(1), Some(id(999)), false)
            .is_err()
    );
    let key = actors.key(id(1)).unwrap();
    actors.object_mut(key).unwrap().definition.proof.id = "foreign".into();
    assert!(kernel.visible_to(&actors, id(1), None, false).is_err());
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 source"]
fn actual_source_color_words_match_native_load_and_keep_canonical_order() {
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = aa_rules::sources::V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        aa_rules::sources::SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let catalog = ActorCatalog::load_v14(&mut sources).unwrap();
    let proof: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-live-battle-scene/native-color-load.json"
    ))
    .unwrap();
    assert_eq!(proof["cases"].as_array().unwrap().len(), 21);
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        let colors = NativeColors::from_source(&catalog, pc).unwrap();
        assert!(colors.colors.len() >= 19);
        for (name, bits) in [
            ("blue", [0, 0x3e99999a, 0x3f800000]),
            ("red", [0x3f59999a, 0, 0x3dcccccd]),
        ] {
            let (_, actual) = colors.colors[name];
            assert_eq!(actual, bits);
            let row = proof["cases"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["input"] == json!(bits))
                .unwrap();
            assert_eq!(json!(actual), row["output"]["bits"]);
        }
        let mut c = json!({"actors":{"1":{"class":3,"flags":0,"detection":0,"members":null,"element":null}},"nonmajor":0,"shared_mask":0});
        let mut store = fixture_store(&c);
        let kid = store.key(id(10)).unwrap();
        for (name, expected) in [
            ("blue", [0, 0x3e99999a, 0x3f800000]),
            ("red", [0x3f59999a, 0, 0x3dcccccd]),
        ] {
            let Source::Special(k) = &mut store.entity_mut(kid).unwrap().source else {
                panic!()
            };
            k.derived.as_mut().unwrap().references.insert(
                0x1e8,
                Some(aa_mapworld::restore::NamedDefinition {
                    id: name.into(),
                    index: None,
                }),
            );
            assert_eq!(
                colors
                    .reference_color(&store, id(10))
                    .unwrap()
                    .unwrap()
                    .map(f32::to_bits),
                expected
            );
        }
        let Source::Special(k) = &mut store.entity_mut(kid).unwrap().source else {
            panic!()
        };
        k.derived.as_mut().unwrap().references.insert(0x1e8, None);
        assert!(colors.reference_color(&store, id(10)).unwrap().is_none());
        c["nonmajor"] = json!(1); // This is a source/getter probe, not a World initialization claim.
    }
}
