use aa_actors::{
    ActorId,
    components::ComponentSlot,
    creation::{ActorCreationRecord, CreationRecord, ThingDefinition},
    economy_values::{GateWorld, VectorKind},
};
use aa_body::F32;
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    sources::{SourceLimits, V14Sources},
};
use aa_runtime::{
    actor_world::{ActorStore, AssemblyEffects, PlacementState, RuntimeComponent, Spatial},
    catalog::Catalog,
    economy_values_world::View,
};
use aa_simulation::NativeRng;
use std::collections::BTreeMap;
struct Unpublished;
impl Spatial for Unpublished {
    fn placement(&self, _: ActorId) -> aa_runtime::actor_world::Result<PlacementState> {
        Err(aa_runtime::actor_world::Error::Missing(
            "test actor has not published its pose",
        ))
    }
    fn publish(&mut self, _: ActorId, _: PlacementState) -> aa_runtime::actor_world::Result<()> {
        panic!("not used by resource getters")
    }
}
#[derive(Default)]
struct Observed(Vec<ComponentSlot>);
impl AssemblyEffects for Observed {
    fn completed(
        &mut self,
        _: &mut ActorStore,
        _: ActorId,
        created: &[ComponentSlot],
    ) -> aa_runtime::actor_world::Result<()> {
        self.0.extend_from_slice(created);
        Ok(())
    }
}

fn layout_oracle() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../research/phase8-layouts/native-layout-data.json"
    ))
    .unwrap()
}

fn layout_case<'a>(doc: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
    doc["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == name)
        .unwrap()
}

/// Existing 4C3610/4C34A0 layout fixtures supply the slot maps. Added
/// translations exercise the separately proven one-hop 48D500 lookup rule;
/// this is a diagnostic identity regression, not a new native golden output.
fn chained_identity_catalog(doc: &serde_json::Value) -> ActorCatalog {
    use aa_rules::{
        actors::native_load::{NativeLoadReport, load_files},
        sources::{Origin, ResolvedFile, sha256},
        tgi::Limits,
    };
    use serde_json::json;
    let basic = layout_case(doc, "basic");
    let many = layout_case(doc, "many_slots_and_positions");
    let mut files = doc["templates"].as_array().unwrap().clone();
    files.extend(basic["files"].as_array().unwrap().iter().cloned());
    let text = basic["files"].as_array().unwrap().last().unwrap()["text"]
        .as_str()
        .unwrap();
    files.push(json!({"path":"Fixture/Layouts/l2.tgi","text":text.replace("L1","L2")}));
    let text = many["files"].as_array().unwrap().last().unwrap()["text"]
        .as_str()
        .unwrap();
    files.push(json!({"path":"Fixture/Layouts/l3.tgi","text":text.replace("L1","L3")}));
    files.push(json!({"path":"Templates/template_translate.tgi","text":
        "[template Translate]\n{\nstring node\nstring ids_old\nstring ids_new\n}\n"}));
    files.push(json!({"path":"Fixture/translations.tgi","text":concat!(
        "[Translate template=Translate]\n{\nnode=OrganizationLayout\nids_old=L1\nids_new=L2\n}\n",
        "[Translate template=Translate]\n{\nnode=OrganizationLayout\nids_old=L2\nids_new=L3\n}\n",
        "[Thing]\n{\nIDS=alpha\ncategory=ACTOR\n}\n",
        "[Thing]\n{\nIDS=beta\ncategory=ACTOR\n}\n",
        "[Translate template=Translate]\n{\nnode=Thing\nids_old=old1\nids_new=alpha\n}\n",
        "[Translate template=Translate]\n{\nnode=Thing\nids_old=alpha\nids_new=beta\n}\n"
    )}));
    let files = files
        .iter()
        .map(|file| {
            let bytes: Vec<u8> = file["text"]
                .as_str()
                .unwrap()
                .chars()
                .map(|c| u8::try_from(u32::from(c)).unwrap())
                .collect();
            ResolvedFile {
                logical_path: file["path"].as_str().unwrap().into(),
                origin: Origin::V14Loose,
                sha256: sha256(&bytes),
                bytes,
            }
        })
        .collect();
    load_files(files, Limits::default(), &mut NativeLoadReport::default())
}

#[test]
fn canonical_actor_definition_and_packed_getters_refuse_a_second_translation() {
    // native-definition-registry.json proves Find(old1)->alpha and
    // Find(alpha)->beta. A resolved pointer to alpha is not a new Find call.
    let doc = layout_oracle();
    let actors = chained_identity_catalog(&doc);
    assert_eq!(actors.find("old1").unwrap().id, "alpha");
    assert_eq!(actors.find("alpha").unwrap().id, "beta");
    let catalog = Catalog::new(&actors);
    let mut store = ActorStore::new();
    let packed = BTreeMap::from([
        ("alpha".into(), None),
        ("beta".into(), Some("wrong translated packed actor".into())),
    ]);
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        let view = View {
            actors: &mut store,
            spatial: &Unpublished,
            catalog: &catalog,
            precision: pc,
            packed_definitions: &packed,
        };
        for identity in ["alpha", "ALPHA"] {
            for error in [
                GateWorld::definition(&view, &identity.into())
                    .err()
                    .unwrap(),
                GateWorld::packed_definition(&view, &identity.into())
                    .err()
                    .unwrap(),
            ] {
                let message = error.to_string();
                assert!(
                    message
                        .to_ascii_lowercase()
                        .contains("canonical actordata alpha is shadowed"),
                    "{message}"
                );
                assert!(message.contains("beta"), "{message}");
            }
        }
    }
}

/// Explicit storage fixture only. Native layout slots come from the oracle;
/// these two current parameters are the authoritative Organization payload.
fn live_organization(layout: String, slots: usize) -> (ActorStore, ActorId) {
    use aa_actors::{
        IdentityRegistry,
        ground_states::{LayoutDefinition, OrganizationState},
        loading::{LoadKind, construct_shell_with_components},
    };
    use aa_mapworld::{
        ThingState,
        bootstrap::DefinitionProof,
        inventory::{ByteSpan, Placement},
        restore::{
            LoadCompletion, LoadedEntity, RestoredComponent, RestoredRecord, RuntimeDefinition,
        },
    };
    use aa_rules::{native::CategoryDescriptor, resources::Provenance, tgi::Span};
    let id = ActorId::from_raw(1);
    let definition = RuntimeDefinition {
        proof: DefinitionProof {
            id: "explicit live Organization storage fixture".into(),
            source: Provenance {
                path: "native layout fixture adapter test".into(),
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
    let mut shell = construct_shell_with_components::<_, RestoredComponent>(
        LoadKind::Actor,
        definition.proof.clone(),
    );
    let organization = OrganizationState::new(
        id,
        LayoutDefinition {
            definition: layout,
            parameters: vec![Some("alpha".into()), None],
            slots,
        },
        F32::ZERO,
        F32::ZERO,
        [F32::ZERO; 2],
    )
    .unwrap();
    shell.actor.as_mut().unwrap().components.insert(
        ComponentSlot::Organization,
        RestoredComponent::Organization(Box::new(organization)),
    );
    let source = Placement {
        record_ordinal: 0,
        source_span: ByteSpan {
            offset: 0,
            length: 0,
        },
        thing_id_utf16: vec![],
        class_code: 3,
        serialized_id: id.raw(),
        thing: ThingState {
            version: 2,
            name_utf16: vec![],
            word_1c: 0,
            position_bits: [0; 3],
            facing_bits: [0; 3],
            serialized_flags: 0,
            legacy_version_one_word: None,
        },
        object: None,
        actor: None,
        opaque_tail_span: ByteSpan {
            offset: 0,
            length: 0,
        },
        opaque_tail: vec![],
    };
    let mut registry = IdentityRegistry::new();
    registry
        .register_saved(
            id,
            true,
            RestoredRecord::Actor(Box::new(LoadedEntity {
                source,
                definition,
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
    (ActorStore::from_restored(registry).unwrap(), id)
}

#[test]
fn live_layout_slots_use_canonical_row_and_current_organization_parameters() {
    use aa_actors::economy_values::World;
    use aa_mapworld::restore::RestoreDefinitions;
    let doc = layout_oracle();
    let actors = chained_identity_catalog(&doc);
    let catalog = Catalog::new(&actors);
    let expected: Vec<u32> = serde_json::from_value(
        layout_case(&doc, "basic")["result"]["layouts"][0]["slot_groups"].clone(),
    )
    .unwrap();
    let translated: Vec<u32> = serde_json::from_value(
        layout_case(&doc, "many_slots_and_positions")["result"]["layouts"][0]["slot_groups"]
            .clone(),
    )
    .unwrap();
    assert_ne!(
        expected, translated,
        "fixture must distinguish canonical and retranslated rows"
    );
    let packed = BTreeMap::new();
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        let canonical = catalog
            .organization_layout_data(&"L1".encode_utf16().collect::<Vec<_>>(), pc)
            .unwrap()
            .unwrap();
        assert_eq!(canonical, "L2");
        assert_eq!(
            catalog
                .organization_layout_data(&"L2".encode_utf16().collect::<Vec<_>>(), pc)
                .unwrap()
                .unwrap(),
            "L3"
        );
        let (mut store, id) = live_organization(canonical.to_ascii_lowercase(), expected.len());
        let view = View {
            actors: &mut store,
            spatial: &Unpublished,
            catalog: &catalog,
            precision: pc,
            packed_definitions: &packed,
        };
        assert_eq!(view.layout_slot_count(id.raw()).unwrap(), expected.len());
        for (slot, group) in expected.iter().enumerate() {
            assert_eq!(
                view.layout_slot_definition(id.raw(), slot).unwrap(),
                [Some("alpha".to_owned()), None][*group as usize]
            );
        }
        assert!(
            view.layout_slot_definition(id.raw(), expected.len())
                .is_err(),
            "no fallthrough into translated L3 slots"
        );
        let Some(RuntimeComponent::Organization(org)) = view
            .actors
            .component_mut(id, ComponentSlot::Organization)
            .unwrap()
        else {
            panic!()
        };
        let allocation = &**org as *const _;
        assert_eq!(org.layout.definition, "l2");
        assert_eq!(org.layout.parameters, vec![Some("alpha".into()), None]);
        let members = org.members.clone();
        org.layout.parameters[0] = Some("beta".into());
        org.layout.parameters[1] = Some("alpha".into());
        for (slot, group) in expected.iter().enumerate() {
            assert_eq!(
                view.layout_slot_definition(id.raw(), slot).unwrap(),
                [Some("beta".to_owned()), Some("alpha".to_owned())][*group as usize],
                "read the current parameter; do not translate alpha to beta"
            );
        }
        let Some(RuntimeComponent::Organization(org)) = view
            .actors
            .component(id, ComponentSlot::Organization)
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(&**org as *const _, allocation);
        assert_eq!(org.members, members);
        assert_eq!(org.layout.definition, "l2");
        assert_eq!(
            org.layout.parameters,
            vec![Some("beta".into()), Some("alpha".into())]
        );
    }
}
#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 source"]
fn original_source_costs_read_live_properties_and_lazy_inverse_binding() {
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let catalog = Catalog::new(&actors);
    let pc = Precision::Double53;
    let packed = BTreeMap::new();
    for name in [
        "usa_infantry",
        "gbr_infantry",
        "ger_infantry",
        "jpn_infantry",
        "rus_infantry",
        "regiment_usa_infantry",
    ] {
        let mut store = ActorStore::new();
        let mut rng = NativeRng::new(19);
        let record = ActorCreationRecord {
            base: CreationRecord {
                definition: ThingDefinition {
                    key: name.to_owned(),
                    flags: 0,
                },
                position_bits: [0; 3],
                direction_bits: [0, (-1f32).to_bits()],
                name_utf16: vec![],
                saved_id: ActorId::NULL,
            },
            variant: -1,
            owner_word: 0,
            word_30: 0,
            request_nonblocking: false,
        };
        let key = store
            .construct_new(&catalog, record, 0, &mut rng, pc, pc)
            .unwrap();
        let id = store.initialize_object(key, &mut rng, pc).unwrap();
        let mut owners = aa_runtime::scheduler_owners::Owners::new();
        let mut definitions = aa_runtime::definitions::Bank::default();
        let mut observed = Observed::default();
        store
            .assemble_ground(
                key,
                &catalog,
                F32::ZERO,
                &mut rng,
                &mut owners,
                &mut definitions,
                pc,
                &mut observed,
            )
            .unwrap();
        assert_eq!(observed.0.last(), Some(&ComponentSlot::Mind));
        let mut view = View {
            actors: &mut store,
            spatial: &Unpublished,
            catalog: &catalog,
            precision: pc,
            packed_definitions: &packed,
        };
        let source = catalog
            .actors
            .native_economy_inputs(name, pc)
            .unwrap()
            .resources
            .unwrap();
        let cost = view.cost_or_upkeep(id, VectorKind::Cost).unwrap();
        let upkeep = view.cost_or_upkeep(id, VectorKind::Upkeep).unwrap();
        assert_eq!(cost.len(), 7);
        assert_eq!(upkeep.len(), 7);
        if name != "regiment_usa_infantry" {
            assert_eq!(
                cost.iter().map(|v| v.0).collect::<Vec<_>>(),
                source.cost_bits
            );
            assert_eq!(
                upkeep.iter().map(|v| v.0).collect::<Vec<_>>(),
                source.upkeep_bits
            );
        }
        let _ = view.production(id, false).unwrap();
        // The snapshot's legacy packed field cannot bypass the real unresolved
        // inverse callback. Supplied paths above did not need to read it.
        assert!(view.packed_definition(&name.to_owned()).is_err());
        if name == "usa_infantry" || name == "regiment_usa_infantry" {
            let Some(RuntimeComponent::Properties(props)) = view
                .actors
                .component_mut(id, ComponentSlot::Properties)
                .unwrap()
            else {
                panic!()
            };
            props.modifier.scalars[(0x60 / 4) - 1] = F32(1.5f32.to_bits());
            let changed = view.cost_or_upkeep(id, VectorKind::Cost).unwrap();
            let current = aa_actors::economy_values::World::modifier(&view, id.raw()).unwrap();
            assert_eq!(
                current.factor60,
                F32(1.5f32.to_bits()),
                "getter must observe the actual Properties box"
            );
            if name == "usa_infantry" {
                assert!(source.cost_bits.iter().all(|v| *v == 0));
                assert_eq!(changed, cost, "individual infantry source cost is zero");
            } else {
                assert_eq!(cost[0], F32(50.0f32.to_bits()));
                assert_eq!(
                    changed[0],
                    F32(75.0f32.to_bits()),
                    "recruitable regiment cost must observe its live modifier"
                );
            }
            view.actors
                .object_mut(key)
                .unwrap()
                .components_mut()
                .unwrap()
                .state
                .low |= 1 << 19;
            assert!(
                view.production(id, false).is_err(),
                "unsupplied production must not treat unresolved inverse as null"
            );
            assert!(
                view.cost_or_upkeep(id, VectorKind::Cost).is_ok(),
                "cost does not read packed inverse"
            );
        }
    }
}
