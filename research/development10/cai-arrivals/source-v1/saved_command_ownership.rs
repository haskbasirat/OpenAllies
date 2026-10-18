//! Saved-command ownership transfer into the live Mind arena. Constructor
//! states are produced by the existing native Regiment constructor/install;
//! only the saved ownership graph is synthetic. No gameplay orders are run.
use aa_actors::{
    ActorId, IdentityRegistry,
    components::ComponentSlot,
    loading::{LoadKind, construct_shell_with_components},
    owned::Component,
};
use aa_mapworld::{
    ThingState,
    bootstrap::DefinitionProof,
    inventory::{ByteSpan, Placement},
    kingdoms::TeamRecord,
    restore::{
        LoadCompletion, LoadedEntity, RestoredComponent, RestoredRecord, RuntimeDefinition,
        SourceWorldEntity, WorldEntityRuntime,
    },
    restore_components::RestoredMind,
};
use aa_mind::{
    MindId, StateId,
    construct::{self, Kind},
};
use aa_numeric::Precision;
use aa_rules::{native::CategoryDescriptor, resources::Provenance, tgi::Span};
use aa_runtime::{
    actor_world::ActorStore,
    catalog::RestoredMindParts,
    mind_world::{PlainCommand, Runtime},
};

struct ConstructorLeaves;
impl construct::World for ConstructorLeaves {
    fn random_word(&mut self) -> aa_mind::Result<u32> {
        panic!("Regiment constructor must not draw RNG")
    }
    fn performer_ability_count(&self, _: u32) -> aa_mind::Result<usize> {
        panic!("Regiment constructor must not query abilities")
    }
    fn set_ability_jitter(&mut self, _: u32, _: usize, _: u32) -> aa_mind::Result<()> {
        panic!("Regiment constructor must not set ability jitter")
    }
}

fn command() -> PlainCommand {
    PlainCommand {
        definition_id: 0,
        definition: None,
        flags: 0x87654321,
        word10: 0xabcdef01,
        object_definition: Some(8),
        position_bits: [0x7fc12345, 0x80000000],
        auxiliary_bits: [0x3f800000, 0xff800000],
        scalar_bits: 0x7fa12345,
        parameter_definition: Some(13),
        extra_definition: Some(21),
        boolean: 0xfe,
        trailing: 0x80,
    }
}
fn parts(actor: u32, pending: &[u32], current: Option<u32>, stored: &[u32]) -> RestoredMindParts {
    let built = construct::construct(
        Kind::Regiment,
        actor,
        &mut ConstructorLeaves,
        Precision::Double53,
    )
    .unwrap();
    let bindings = built
        .states
        .iter()
        .enumerate()
        .map(|(i, s)| (s.offset, StateId(actor * 100 + i as u32 + 1)))
        .collect();
    let mut arena = aa_mind::Arena::default();
    let installed = built.install(MindId(actor), &bindings, &mut arena).unwrap();
    let mut mind = arena.minds.remove(&MindId(actor)).unwrap();
    mind.pending_commands.extend(pending.iter().copied());
    mind.current_command = current;
    RestoredMindParts {
        installed,
        mind: Some(mind),
        states: arena.states,
        element_path: None,
        regiment_owned: None,
        precision: Precision::Double53,
        commands: stored.iter().map(|id| (*id, command())).collect(),
    }
}
fn span() -> ByteSpan {
    ByteSpan {
        offset: 0,
        length: 0,
    }
}
fn thing() -> ThingState {
    ThingState {
        version: 2,
        name_utf16: vec![],
        word_1c: 0,
        position_bits: [0; 3],
        facing_bits: [0; 3],
        serialized_flags: 0,
        legacy_version_one_word: None,
    }
}
fn record(actor: u32, parts: RestoredMindParts, special: bool) -> RestoredRecord {
    let def = RuntimeDefinition {
        proof: DefinitionProof {
            id: "saved-command ownership fixture".into(),
            source: Provenance {
                path: "saved_command_ownership.rs synthetic ownership graph".into(),
                sha256: String::new(),
                span: Span { start: 0, end: 0 },
            },
            template_chain: vec![],
            category: CategoryDescriptor {
                name: "fixture",
                registry_index: 0,
                native_kind: 2,
                runtime_class: 3,
                view_type: 0,
            },
            base_flag_inputs: vec![],
        },
        native_class: 3,
        flags: 0,
        object: None,
        component_mask: None,
        ground: Default::default(),
    };
    let mut shell =
        construct_shell_with_components::<_, RestoredComponent>(LoadKind::Actor, def.proof.clone());
    shell.actor.as_mut().unwrap().components.insert(
        ComponentSlot::Mind,
        Component::Mind(RestoredMind {
            required: ComponentSlot::Mind,
            state: Box::new(parts),
        }),
    );
    if special {
        return RestoredRecord::Team(Box::new(SourceWorldEntity {
            source: TeamRecord {
                source_span: span(),
                definition_id_utf16: vec![],
                serialized_id: actor,
                thing: thing(),
                version: 1,
                slot_index: 0,
                display_key_utf16: vec![],
                words_4c: vec![],
                words_58: vec![],
                string_64_utf16: Some(vec![]),
            },
            completion: LoadCompletion::PostLoaded,
            failure: None,
            runtime: Some(WorldEntityRuntime {
                definition: def,
                base: shell,
                derived: None,
                actor_tail: None,
                derived_payload_loaded: false,
            }),
        }));
    }
    RestoredRecord::Actor(Box::new(LoadedEntity {
        source: Placement {
            record_ordinal: 0,
            source_span: span(),
            thing_id_utf16: vec![],
            class_code: 3,
            serialized_id: actor,
            thing: thing(),
            object: None,
            actor: None,
            opaque_tail_span: span(),
            opaque_tail: vec![],
        },
        definition: def,
        shell,
        completion: LoadCompletion::PostLoaded,
        owner_source_slot: None,
        saved_components: vec![],
        actor_tail: None,
        doodad: None,
        special_object: None,
        failure: None,
        active_component: Some(ComponentSlot::Mind),
    }))
}
fn registry(records: Vec<RestoredMindParts>, special: bool) -> IdentityRegistry<RestoredRecord> {
    let mut registry = IdentityRegistry::new();
    for p in records {
        let id = p.mind.as_ref().unwrap().actor;
        registry
            .register_saved(ActorId::from_raw(id), true, record(id, p, special))
            .unwrap();
    }
    registry
}

#[test]
fn restored_pending_current_and_special_commands_move_once_without_normalization() {
    for special in [false, true] {
        let source = parts(1, &[19, 7], Some(128), &[7, 128, 19]);
        let pending_pointer = source.mind.as_ref().unwrap().pending_commands.as_slices().0.as_ptr();
        let expected_states = source.states.clone();
        let mut actors = ActorStore::from_restored(registry(vec![source], special)).unwrap();
        assert_eq!(
            actors.minds.minds[&MindId(1)]
                .pending_commands
                .as_slices()
                .0
                .as_ptr(),
            pending_pointer
        );
        assert_eq!(actors.minds.states, expected_states);
        let mut runtime = Runtime::default();
        runtime.bind_restored_commands(&mut actors).unwrap();
        for id in [7, 19, 128] {
            assert_eq!(runtime.commands.value(id).unwrap(), &command());
            assert_eq!(runtime.commands.references(id).unwrap(), 1);
        }
        assert_eq!(actors.minds.minds[&MindId(1)].pending_commands, [19, 7]);
        assert_eq!(actors.minds.minds[&MindId(1)].current_command, Some(128));
        assert_eq!(runtime.commands.allocate(command()).unwrap(), 129);
        assert!(runtime.bind_restored_commands(&mut actors).is_err());
        assert!(
            Runtime::default()
                .bind_restored_commands(&mut actors)
                .is_err()
        );
    }
}

#[test]
fn saved_command_prevalidation_rejects_zero_duplicate_missing_and_orphan_ownership() {
    let invalid = [
        parts(1, &[0], None, &[0]),
        parts(1, &[4, 4], None, &[4]),
        parts(1, &[4], Some(4), &[4]),
        parts(1, &[4], None, &[4, 4]),
        parts(1, &[4], None, &[5]),
        parts(1, &[], None, &[5]),
        parts(1, &[], Some(0), &[]),
    ];
    for source in invalid {
        assert!(ActorStore::from_restored(registry(vec![source], false)).is_err());
    }
    // Individually valid references cannot share one owned allocation across Minds.
    assert!(
        ActorStore::from_restored(registry(
            vec![parts(1, &[4], None, &[4]), parts(2, &[], Some(4), &[4])],
            false
        ))
        .is_err()
    );
    // Swapping each Mind's payload list is not accepted merely because the
    // global union of referenced and stored IDs still agrees.
    assert!(
        ActorStore::from_restored(registry(
            vec![parts(1, &[4], None, &[5]), parts(2, &[5], None, &[4])],
            false
        ))
        .is_err()
    );
}

#[test]
fn dirty_target_refusal_preserves_source_even_after_its_last_live_command_is_released() {
    for release_first in [false, true] {
        let mut actors =
            ActorStore::from_restored(registry(vec![parts(1, &[7], None, &[7])], false)).unwrap();
        let mut occupied = Runtime::default();
        let allocated = occupied.commands.allocate(command()).unwrap();
        if release_first {
            occupied.commands.release(allocated).unwrap();
        }
        assert!(occupied.bind_restored_commands(&mut actors).is_err());
        let mut fresh = Runtime::default();
        fresh.bind_restored_commands(&mut actors).unwrap();
        assert_eq!(fresh.commands.value(7).unwrap(), &command());
        assert_eq!(fresh.commands.references(7).unwrap(), 1);
        assert_eq!(fresh.commands.allocate(command()).unwrap(), 8);
    }
}

#[test]
fn empty_restored_registry_has_one_transfer_but_new_store_has_none() {
    let mut restored = ActorStore::from_restored(IdentityRegistry::new()).unwrap();
    let mut runtime = Runtime::default();
    runtime.bind_restored_commands(&mut restored).unwrap();
    assert!(runtime.bind_restored_commands(&mut restored).is_err());
    assert!(
        Runtime::default()
            .bind_restored_commands(&mut ActorStore::new())
            .is_err()
    );
}

fn native_regiment_payload(version: u32, rich: bool) -> Vec<u8> {
    let doc: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../research/skirmish-20261016-regiment-load/native-regiment.json"
    )).unwrap();
    let case = doc["cases"].as_array().unwrap().iter().find(|c| {
        c["version"] == version && c["precision"] == "Double53"
            && !c["native"]["pending"].as_array().unwrap().is_empty() == rich
    }).unwrap();
    hex(case["payload"].as_str().unwrap())
}
fn hex(text: &str) -> Vec<u8> {
    (0..text.len()).step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap()).collect()
}

#[test]
fn saved_load_moves_the_actual_core_and_restores_it_on_context_error() {
    use aa_mapworld::{restore::RestoreDefinitions, restore_components::MindLoadContext};
    let actors = aa_rules::actors::native_load::load_files(
        Vec::new(), aa_rules::tgi::Limits::default(),
        &mut aa_rules::actors::native_load::NativeLoadReport::default(),
    );
    let catalog = aa_runtime::catalog::Catalog::new(&actors);
    let mut source = parts(1, &[], None, &[]);
    let core = source.mind.as_mut().unwrap();
    // These source fixtures have base version0, so native Load must preserve
    // the existing state stack and empty command allocation capacities.
    core.stack.push_back(Some(source.installed.state_bindings[&0x34]));
    core.pending_commands.reserve(19);
    let stack_pointer = core.stack.as_slices().0.as_ptr();
    let command_pointer = core.pending_commands.as_slices().0.as_ptr();
    let state_pointer = source.states.values().next().unwrap() as *const aa_mind::State;
    let mut restored = RestoredMind { required: ComponentSlot::Mind, state: Box::new(source) };
    for version in [0, 1] {
        let bytes = if version == 0 { 0u32.to_le_bytes().to_vec() }
            else { native_regiment_payload(version, false) };
        catalog.load_saved_mind(&mut restored, &bytes).unwrap();
    }
    let bytes = native_regiment_payload(21, false);
    assert!(catalog.load_saved_mind(&mut restored, &bytes).unwrap_err().contains("requires"));
    assert!(catalog.load_saved_mind_with_context(&mut restored, &bytes, MindLoadContext {
        owner: ActorId::from_raw(2), world_time_bits: 0x427b0000, organization: None,
    }).unwrap_err().contains("different actor"));
    let p = restored.state.as_any().downcast_ref::<RestoredMindParts>().unwrap();
    assert_eq!(p.mind.as_ref().unwrap().stack.as_slices().0.as_ptr(), stack_pointer);
    assert_eq!(p.mind.as_ref().unwrap().pending_commands.as_slices().0.as_ptr(), command_pointer);
    assert_eq!(p.states.values().next().unwrap() as *const aa_mind::State, state_pointer);
    assert!(p.regiment_owned.is_none());
    catalog.load_saved_mind_with_context(&mut restored, &bytes, MindLoadContext {
        owner: ActorId::from_raw(1), world_time_bits: 0x427b0000, organization: None,
    }).unwrap();
    let p = restored.state.as_any_mut().downcast_mut::<RestoredMindParts>().unwrap();
    assert_eq!(p.mind.as_ref().unwrap().stack.as_slices().0.as_ptr(), stack_pointer);
    assert_eq!(p.mind.as_ref().unwrap().pending_commands.as_slices().0.as_ptr(), command_pointer);
    assert_eq!(p.states.values().next().unwrap() as *const aa_mind::State, state_pointer);
    assert_eq!(p.installed.words[&0xbc], 0x427b0000);
    assert!(p.regiment_owned.is_some());

    let mut absent = parts(1, &[], None, &[]);
    absent.mind = None;
    let mut restored = RestoredMind { required: ComponentSlot::Mind, state: Box::new(absent) };
    assert!(catalog.load_saved_mind(&mut restored, &0u32.to_le_bytes()).unwrap_err().contains("core is absent"));
    let absent = *restored.state.into_any().downcast::<RestoredMindParts>().unwrap();
    let mut reg = IdentityRegistry::new();
    reg.register_saved(ActorId::from_raw(1), true, record(1, absent, false)).unwrap();
    assert!(ActorStore::from_restored(reg).err().unwrap().to_string().contains("core is absent"));
}

#[test]
fn nonempty_regiment_containers_move_once_and_duplicate_or_missing_owners_fail() {
    let bytes = native_regiment_payload(21, true);
    let aa_mind::saved_load::Saved::Regiment(saved) =
        aa_mind::saved_load::decode(Kind::Regiment, &bytes).unwrap().0 else { panic!("Regiment") };
    let pending: Vec<_> = (1..=saved.base.as_ref().unwrap().commands.len() as u32).collect();
    let bindings = aa_mind::saved_load::CommandBindings { pending: pending.clone(), current: None };
    let mut source = parts(1, &[], None, &[]);
    let mut arena = aa_mind::Arena {
        minds: [(MindId(1), source.mind.take().unwrap())].into(),
        states: std::mem::take(&mut source.states),
    };
    let applied = aa_mind::regiment_load::apply_with_context(saved, &mut source.installed,
        &mut arena, &bindings, aa_mind::regiment_load::Context {
            world_time_bits: 0x427b0000, formations: Some(&[11, 13, 17]), construct_definition: Some(23),
        }).unwrap();
    source.mind = arena.minds.remove(&MindId(1));
    source.states = arena.states;
    source.commands = pending.iter().map(|&id| (id, command())).collect();
    source.regiment_owned = Some(applied.owned);
    let owned = source.regiment_owned.as_ref().unwrap();
    let pointers = (owned.tracked_words.iter().next().map(std::ptr::from_ref), owned.path.nodes.as_ptr(),
        owned.join_words.as_ptr(), owned.bombard_text.as_ptr());
    assert!(!owned.tracked_words.is_empty() && !owned.path.nodes.is_empty()
        && !owned.join_words.is_empty() && !owned.bombard_text.is_empty());
    let expected = owned.clone(); // observation only, never supplied to live storage
    let mut missing = source.clone();
    missing.regiment_owned = None;
    assert!(ActorStore::from_restored(registry(vec![missing], false)).is_err());
    let mut duplicate = source.clone();
    duplicate.installed.words.insert(0xb8, 17);
    assert!(ActorStore::from_restored(registry(vec![duplicate], false)).is_err());
    let mut store = ActorStore::from_restored(registry(vec![source], false)).unwrap();
    let owned = &store.regiment_owned[&MindId(1)];
    assert_eq!(owned, &expected);
    assert_eq!((owned.tracked_words.iter().next().map(std::ptr::from_ref), owned.path.nodes.as_ptr(),
        owned.join_words.as_ptr(), owned.bombard_text.as_ptr()), pointers);
    let mut runtime = Runtime::default();
    runtime.bind_restored_commands(&mut store).unwrap();
    assert!(runtime.bind_restored_commands(&mut store).is_err());
    for id in pending { assert_eq!(runtime.commands.references(id).unwrap(), 1); }
}

#[test]
#[ignore = "requires original AA_GAME_ROOT assets"]
fn actual_source_regiment_context_component_load_and_failure_keep_owned_allocations() {
    use aa_mapworld::{restore::{MindOwner, RestoreDefinitions}, restore_components::{
        ComponentLoadAction, ComponentOwner, GroundDefinitions, PerformerServices,
        apply_component_stage_with_services,
    }, saved_components::{MindPayload, SavedComponent}};
    use aa_rules::{actors::ActorCatalog, sources::{SourceLimits, V14Sources}, tgi::Limits};
    use aa_runtime::{catalog::Catalog, localization::Localization, settings::Settings};
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits { max_member_bytes: 64 * 1024 * 1024 }).unwrap();
    let settings = Settings::load_v14(&mut sources, Limits::default()).unwrap();
    let text = Localization::load_v14(&mut sources, Limits::default()).unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings = Some(settings);
    catalog.definition_text = Some(text.clone());
    catalog.replace_current_text(text);
    let source: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../research/skirmish-20261016-regiment-context/native-source-context.json"
    )).unwrap();
    for pc in [Precision::Single24, Precision::Double53, Precision::Extended64] {
        let case = source["cases"].as_array().unwrap().iter()
            .find(|c| c["precision"] == format!("{pc:?}")).unwrap();
        let binding = catalog.actor_bindings("regiment_usa_infantry", pc).unwrap();
        let ground = &binding.definition.ground;
        let bytes = hex(case["payload"].as_str().unwrap());
        let world_time_bits = case["world_time"].as_u64().unwrap() as u32;
        for injected_error in [false, true] {
            let id = ActorId::from_raw(1);
            let mut components = aa_actors::components::Components::new(Default::default());
            components.insert(ComponentSlot::Organization, aa_actors::owned::construct(
                ComponentSlot::Organization, ground, aa_actors::owned::Context {
                    owner: id, world_time: aa_body::F32(world_time_bits),
                    position: [aa_body::F32::ZERO; 3], precision: pc,
                }).unwrap());
            let Some(Component::Organization(org)) = components.get(ComponentSlot::Organization) else { panic!("Organization") };
            let organization_pointer = org.as_ref() as *const _ as usize;
            let org_before = format!("{org:?}");
            let expected_formation = catalog.organization_layout_info(&org.layout.definition, pc).unwrap().formations[2];
            let observation = std::cell::RefCell::new(None);
            let mut rng = aa_simulation::NativeRng::new(0x12345678);
            let mut effects = Vec::new();
            let mut registrations = Vec::new();
            let mut flags = 0;
            let result = apply_component_stage_with_services(&mut components,
                ComponentOwner { id, owner: None, owner_effects: &mut effects, flags: &mut flags,
                    world_time_bits, position_bits: [0; 3], updatable_registrations: &mut registrations,
                    register_updatable: &|_, _| Ok(()) }, ComponentSlot::Mind,
                ComponentLoadAction::Saved { remove_after_load: false },
                Some(&SavedComponent::Mind(MindPayload { updatable: None, bytes: bytes.clone() })),
                &GroundDefinitions::default(), pc, Some(PerformerServices {
                    construct: &mut |_, _| panic!("not a Performer stage"),
                    apply_ability: &|_, _| panic!("not a Performer stage"),
                    construct_defender: &mut || panic!("not a Defender stage"),
                    ability_word14: &|_| panic!("not an Ability stage"),
                    construct_mind: &mut |components| catalog.construct_mind(MindOwner {
                        id, definition_mask: binding.definition.component_mask.unwrap(), components,
                        rng: &mut rng, precision: pc,
                    }).map_err(aa_mapworld::Error::Format),
                    load_mind: &|mind, bytes, context| {
                        assert_eq!(context.organization.unwrap() as *const _ as usize, organization_pointer);
                        // A real apply rejection after Catalog takes the core
                        // into its Arena must return the exact allocations.
                        let p = mind.state.as_any_mut().downcast_mut::<RestoredMindParts>().unwrap();
                        let core = p.mind.as_mut().unwrap();
                        core.stack.reserve(64);
                        core.pending_commands.reserve(64);
                        let before = (format!("{p:?}"),
                            p.mind.as_ref().unwrap().stack.as_slices().0.as_ptr() as usize,
                            p.mind.as_ref().unwrap().pending_commands.as_slices().0.as_ptr() as usize,
                            p.states.values().next().unwrap() as *const _ as usize);
                        let mut invalid = native_regiment_payload(21, false);
                        let offsets: Vec<_> = invalid.windows(4).enumerate()
                            .filter_map(|(i, w)| (w == [255; 4]).then_some(i)).collect();
                        assert_eq!(offsets.len(), 1);
                        invalid[offsets[0]..offsets[0] + 4].copy_from_slice(&i32::MAX.to_le_bytes());
                        let error = catalog.load_saved_mind_with_context(mind, &invalid,
                            aa_mapworld::restore_components::MindLoadContext {
                                owner: context.owner, world_time_bits: context.world_time_bits,
                                organization: context.organization,
                            }).unwrap_err();
                        assert!(error.contains("Bad desired formation index"), "{error}");
                        let p = mind.state.as_any().downcast_ref::<RestoredMindParts>().unwrap();
                        assert_eq!((format!("{p:?}"),
                            p.mind.as_ref().unwrap().stack.as_slices().0.as_ptr() as usize,
                            p.mind.as_ref().unwrap().pending_commands.as_slices().0.as_ptr() as usize,
                            p.states.values().next().unwrap() as *const _ as usize), before);
                        catalog.load_saved_mind_with_context(mind, bytes, context)?;
                        let p = mind.state.as_any().downcast_ref::<RestoredMindParts>().unwrap();
                        let core = p.mind.as_ref().unwrap();
                        let owned = p.regiment_owned.as_ref().unwrap();
                        *observation.borrow_mut() = Some((p as *const _ as usize,
                            core.stack.as_slices().0.as_ptr() as usize,
                            core.pending_commands.as_slices().0.as_ptr() as usize,
                            owned.path.nodes.as_ptr() as usize, p.commands.as_ptr() as usize));
                        if injected_error { Err("injected after successful owned Load".into()) } else { Ok(()) }
                    },
                    load_transport: &|_, _| panic!("not a Transport stage"),
                    character_animation: &|_, _| panic!("not a Character stage"),
                }));
            assert_eq!(result.is_err(), injected_error);
            let Component::Mind(mind) = components.remove(ComponentSlot::Mind).unwrap() else { panic!("Mind returned on error") };
            let p = mind.state.as_any().downcast_ref::<RestoredMindParts>().unwrap();
            let core = p.mind.as_ref().unwrap();
            let owned = p.regiment_owned.as_ref().unwrap();
            assert_eq!(Some((p as *const _ as usize, core.stack.as_slices().0.as_ptr() as usize,
                core.pending_commands.as_slices().0.as_ptr() as usize, owned.path.nodes.as_ptr() as usize,
                p.commands.as_ptr() as usize)), *observation.borrow());
            assert_eq!(p.installed.words[&0xbc], world_time_bits);
            assert_eq!(owned.desired_formation, Some(expected_formation));
            assert_eq!(catalog.definition_bank.borrow().entry(owned.construct_definition.unwrap()).unwrap().key.name, "usa_infantry");
            assert_eq!(owned.path.nodes.len(), 3);
            assert_eq!(owned.tracked_words.len(), 3);
            assert_eq!(owned.join_words.len(), 4);
            assert_eq!(p.commands.len(), 3);
            assert_eq!(core.pending_commands.len(), 2);
            assert!(core.current_command.is_some());
            let native_commands = case["native"]["pending"].as_array().unwrap().iter()
                .chain(std::iter::once(&case["native"]["current"]));
            for ((_, actual), expected) in p.commands.iter().zip(native_commands) {
                assert_eq!(u64::from(actual.boolean), expected["boolean"].as_u64().unwrap());
                assert_eq!(u64::from(actual.trailing), expected["trailing"].as_u64().unwrap());
            }
            let Some(Component::Organization(org)) = components.get(ComponentSlot::Organization) else { panic!("Organization retained") };
            assert_eq!(format!("{org:?}"), org_before);
            assert_eq!(org.as_ref() as *const _ as usize, organization_pointer);
            let p = *mind.state.into_any().downcast::<RestoredMindParts>().unwrap();
            let mind_id = p.installed.mind_id;
            let path_pointer = p.regiment_owned.as_ref().unwrap().path.nodes.as_ptr();
            let mut store = ActorStore::from_restored(registry(vec![p], false)).unwrap();
            assert_eq!(store.regiment_owned[&mind_id].path.nodes.as_ptr(), path_pointer);
            let mut runtime = Runtime::default();
            runtime.bind_restored_commands(&mut store).unwrap();
            assert!(runtime.bind_restored_commands(&mut store).is_err());
        }
    }
}
