use aa_actors::{
    ActorId,
    components::ComponentSlot,
    creation::{ActorCreationRecord, CreationRecord, ThingDefinition},
};
use aa_body::F32;
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    sources::{SourceLimits, V14Sources},
};
use aa_runtime::{
    actor_world::{ActorStore, PlacementState, RuntimeComponent, Spatial},
    catalog::Catalog,
};
use aa_simulation::NativeRng;
use std::collections::BTreeMap;
fn record(id: &str) -> ActorCreationRecord<String> {
    ActorCreationRecord {
        base: CreationRecord {
            definition: ThingDefinition {
                key: id.into(),
                flags: 0xdeadbeef,
            },
            position_bits: [10f32.to_bits(), 12f32.to_bits(), 0],
            direction_bits: [0, (-1f32).to_bits()],
            name_utf16: vec![65],
            saved_id: ActorId::NULL,
        },
        variant: -1,
        owner_word: 0,
        word_30: 0,
        request_nonblocking: false,
    }
}
struct SpatialStore(BTreeMap<ActorId, PlacementState>);
impl Spatial for SpatialStore {
    fn placement(&self, id: ActorId) -> aa_runtime::actor_world::Result<PlacementState> {
        self.0
            .get(&id)
            .copied()
            .ok_or(aa_runtime::actor_world::Error::Missing("spatial identity"))
    }
    fn publish(&mut self, id: ActorId, p: PlacementState) -> aa_runtime::actor_world::Result<()> {
        assert!(!self.0.contains_key(&id));
        self.0.insert(id, p);
        Ok(())
    }
}
#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 sources"]
fn original_source_ground_components_share_store_rng_and_single_spatial_pose() {
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let settings =
        aa_runtime::settings::Settings::load_v14(&mut sources, aa_rules::tgi::Limits::default())
            .unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let catalog = Catalog::new(&actors);
    let pc = Precision::Double53;
    for name in [
        "usa_infantry",
        "gbr_infantry",
        "ger_infantry",
        "jpn_infantry",
        "rus_infantry",
        "regiment_usa_infantry",
    ] {
        let mut store = ActorStore::new();
        let mut rng = NativeRng::new(0x12345678);
        let binding = catalog.actor_bindings(name, pc).unwrap();
        let key = store
            .construct_new(&catalog, record(name), 0, &mut rng, pc, pc)
            .unwrap();
        assert!(store.id(key).is_none());
        let id = store.initialize_object(key, &mut rng, pc).unwrap();
        assert_eq!(store.key(id), Some(key));
        assert_eq!(store.identities().len(), 1);
        let mask = binding.definition.component_mask.unwrap();
        struct ObservedAssembly(Vec<ComponentSlot>);
        impl aa_runtime::actor_world::AssemblyEffects for ObservedAssembly {
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
        let before_mind = rng;
        let mut effects = ObservedAssembly(vec![]);
        let mut owners = aa_runtime::scheduler_owners::Owners::new();
        let mut definitions = aa_runtime::definitions::Bank::default();
        store
            .assemble_ground(
                key,
                &catalog,
                F32::ZERO,
                &mut rng,
                &mut owners,
                &mut definitions,
                pc,
                &mut effects,
            )
            .unwrap();
        assert_eq!(effects.0.last(), Some(&ComponentSlot::Mind));
        let mut expected = before_mind;
        if mask & (1 << 8) != 0 {
            expected.next_u32();
        }
        assert_eq!(rng, expected);
        let object = store.object(key).unwrap();
        let actual_mask = object
            .components()
            .unwrap()
            .iter()
            .fold(0u32, |mask, (slot, _)| mask | (1 << slot.index()));
        assert_eq!(actual_mask, mask | (1 << 4));
        if let Some(RuntimeComponent::Performer(p)) =
            object.components().unwrap().get(ComponentSlot::Performer)
        {
            let RuntimeComponent::Mind(mind) = object
                .components()
                .unwrap()
                .get(ComponentSlot::Mind)
                .unwrap()
            else {
                panic!()
            };
            assert_eq!(store.minds.mind(mind.mind_id).unwrap().actor, id.raw());
            for handle in &p.state.abilities {
                assert_eq!(store.abilities[handle].fuzzy_range.0, mind.words[&0x1b8]);
            }
        }
        if let Some(RuntimeComponent::Character(c)) =
            object.components().unwrap().get(ComponentSlot::Character)
        {
            let key = c.animation.as_ref().unwrap();
            let definition = store.animations.get(key).unwrap();
            assert_eq!(definition.actor_id, name);
            assert!(!definition.original_name.is_empty());
        }
        let (rate, _) = settings.unique_fixed("GameTicksPerGameSecond", pc).unwrap();
        let plan = aa_world::initialization::ClockPlan::from_tick_rate(rate, pc).unwrap();
        let mut queues = plan.initialize_queues(plan.new_clock(), pc).unwrap();
        let before_support = rng;
        for slot in [
            ComponentSlot::Character,
            ComponentSlot::Mover,
            ComponentSlot::Properties,
            ComponentSlot::Organization,
            ComponentSlot::Element,
            ComponentSlot::Economy,
            ComponentSlot::Experience,
            ComponentSlot::Detection,
        ] {
            if store.component(id, slot).unwrap().is_some() {
                store
                    .initialize_ground_support(key, slot, &owners, &mut queues, &mut rng)
                    .unwrap();
            }
        }
        let mut expected_support = before_support;
        for _ in 0..3 {
            expected_support.next_u32();
        }
        assert_eq!(rng, expected_support);
        for slot in [ComponentSlot::Properties, ComponentSlot::Detection] {
            let owner = owners
                .get(aa_runtime::scheduler_owners::Target::Component {
                    object: key,
                    slot: slot as u8,
                })
                .unwrap();
            assert!(queues.has_owner(owner));
        }
        let object = store.object(key).unwrap();
        let component_before = object
            .components()
            .unwrap()
            .get(ComponentSlot::Properties)
            .unwrap() as *const RuntimeComponent;
        let mut spatial = SpatialStore(BTreeMap::new());
        let before = store.placement(key, &spatial).unwrap();
        assert_ne!(before.flags, 0xdeadbeef);
        store.publish_placement(key, &mut spatial).unwrap();
        assert!(store.staged_placement_mut(key).is_err());
        spatial.0.get_mut(&id).unwrap().position_bits[0] = 22f32.to_bits();
        assert_eq!(
            store.placement(key, &spatial).unwrap().position_bits[0],
            22f32.to_bits()
        );
        assert_eq!(
            component_before,
            store
                .object(key)
                .unwrap()
                .components()
                .unwrap()
                .get(ComponentSlot::Properties)
                .unwrap() as *const RuntimeComponent
        );
        println!(
            "{}",
            serde_json::json!({"actor":name,"native_id":id.raw(),"component_mask":actual_mask,"abilities":store.abilities.len(),"mind_count":store.minds.minds.len(),"state_count":store.minds.states.len(),"rng":rng.state,"published_pose":store.placement(key,&spatial).unwrap().position_bits})
        );
    }
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 sources"]
fn restored_registry_and_common_boxes_move_without_reinitialization() {
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let settings =
        aa_runtime::settings::Settings::load_v14(&mut sources, aa_rules::tgi::Limits::default())
            .unwrap();
    let text = aa_runtime::localization::Localization::load_v14(
        &mut sources,
        aa_rules::tgi::Limits::default(),
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let map = aa_mapworld::MapArchive::from_bytes(
        std::fs::read(
            game.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial01.RMP"),
        )
        .unwrap(),
    )
    .unwrap();
    let attempt = aa_runtime::restoration::Attempt::load(
        &map,
        &actors,
        settings,
        text,
        Precision::Double53,
        u64::MAX,
    )
    .unwrap();
    let next = attempt.registry.next_slot();
    let highest = attempt.registry.highest_slot();
    let words = attempt.registry.occupied_words().to_vec();
    let generations: Vec<_> = (0..=u16::MAX)
        .map(|slot| attempt.registry.generation_at(slot))
        .collect();
    let mut boxes = BTreeMap::new();
    for entry in attempt.registry.iter() {
        let shell = match &entry.value {
            aa_mapworld::restore::RestoredRecord::Team(v) => v.runtime.as_ref().map(|r| &r.base),
            aa_mapworld::restore::RestoredRecord::Kingdom(v) => v.runtime.as_ref().map(|r| &r.base),
            other => other.loaded().map(|v| &v.shell),
        };
        if let Some(actor) = shell.and_then(|s| s.actor.as_ref()) {
            for (slot, c) in actor.components.iter() {
                let pointer = match c {
                    aa_actors::owned::Component::Body(v) => &**v as *const _ as usize,
                    aa_actors::owned::Component::Mover(v) => &**v as *const _ as usize,
                    aa_actors::owned::Component::Properties(v) => &**v as *const _ as usize,
                    _ => continue,
                };
                boxes.insert((entry.id, slot), pointer);
            }
        }
    }
    assert!(!boxes.is_empty());
    let store = ActorStore::from_restored(attempt.registry).unwrap();
    assert_eq!(store.identities().next_slot(), next);
    assert_eq!(store.identities().highest_slot(), highest);
    assert_eq!(store.identities().occupied_words(), words);
    assert_eq!(
        (0..=u16::MAX)
            .map(|slot| store.identities().generation_at(slot))
            .collect::<Vec<_>>(),
        generations
    );
    for ((id, slot), expected) in &boxes {
        let pointer = match store.component(*id, *slot).unwrap().unwrap() {
            RuntimeComponent::Body(v) => &**v as *const _ as usize,
            RuntimeComponent::Mover(v) => &**v as *const _ as usize,
            RuntimeComponent::Properties(v) => &**v as *const _ as usize,
            _ => panic!("payloadchanged"),
        };
        assert_eq!(pointer, *expected);
    }
    println!(
        "{}",
        serde_json::json!({"imported_entries":store.identities().len(),"preserved_common_boxes":boxes.len(),"native_next":next,"native_highest":highest,"source_completion":attempt.report.runtime_publication_ready})
    );
}
#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 sources"]
fn economy_focus_reborrows_parent_without_extracting_or_copying_components() {
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
    let mut store = ActorStore::new();
    let mut rng = NativeRng::new(7);
    let mut objects = Vec::new();
    for name in ["usa_infantry", "regiment_usa_infantry"] {
        let key = store
            .construct_new(&catalog, record(name), 0, &mut rng, pc, pc)
            .unwrap();
        let id = store.initialize_object(key, &mut rng, pc).unwrap();
        for slot in [ComponentSlot::Properties, ComponentSlot::Economy] {
            store.construct_ground(key, slot, F32::ZERO, pc).unwrap();
        }
        let Some(RuntimeComponent::Economy(economy)) =
            store.component(id, ComponentSlot::Economy).unwrap()
        else {
            panic!()
        };
        objects.push((key, id, &**economy as *const _ as usize));
    }
    let [
        (child, child_id, child_pointer),
        (parent, parent_id, parent_pointer),
    ] = objects[..]
    else {
        panic!()
    };
    let mut spatial = SpatialStore(BTreeMap::new());
    store.publish_placement(child, &mut spatial).unwrap();
    let pose = store.placement(child, &spatial).unwrap();
    {
        let mut access = store.economy_access().unwrap();
        let current = access.take_focus(child).unwrap();
        assert_eq!(current as *mut _ as usize, child_pointer);
        current.flag_10 = true;
        current.resources[0].values[0] = F32(7f32.to_bits());
        assert!(access.economy(child).is_err());
        assert!(
            access
                .object(child)
                .unwrap()
                .components()
                .unwrap()
                .present(ComponentSlot::Economy)
        );
        assert!(access.take_focus(child).is_err());
        assert_eq!(access.placement(child, &spatial).unwrap(), pose);
        {
            let mut nested = access.reborrow_with(child, current).unwrap();
            let parent_current = nested.take_focus(parent).unwrap();
            assert_eq!(parent_current as *mut _ as usize, parent_pointer);
            assert!(nested.economy(child).unwrap().flag_10);
            assert_eq!(
                nested.economy(child).unwrap().resources[0].values[0],
                F32(7f32.to_bits())
            );
            parent_current.flag_11 = true;
            parent_current.resources[2].values[0] =
                nested.economy(child).unwrap().resources[0].values[0];
            let Some(RuntimeComponent::Properties(properties)) = nested
                .component_mut(child_id, ComponentSlot::Properties)
                .unwrap()
            else {
                panic!()
            };
            properties.modifier.flags = 0xabc;
            // Reentrant parent -> child reads both exact live allocations.
            let mut deeper = nested.reborrow_with(parent, parent_current).unwrap();
            assert!(deeper.economy(parent).unwrap().flag_11);
            let same_child = deeper.take_focus(child).unwrap();
            assert_eq!(same_child as *mut _ as usize, child_pointer);
            same_child.resources[1].values[0] = F32(9f32.to_bits());
        }
        assert_eq!(current.resources[1].values[0], F32(9f32.to_bits()));
        assert!(access.economy(parent).unwrap().flag_11);
    }
    for (key, id, pointer) in [
        (child, child_id, child_pointer),
        (parent, parent_id, parent_pointer),
    ] {
        let Some(RuntimeComponent::Economy(value)) =
            store.component(id, ComponentSlot::Economy).unwrap()
        else {
            panic!()
        };
        assert_eq!(&**value as *const _ as usize, pointer);
        assert_eq!(store.id(key), Some(id));
    }
    let Some(RuntimeComponent::Properties(properties)) = store
        .component(child_id, ComponentSlot::Properties)
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(properties.modifier.flags, 0xabc);
    assert_eq!(store.placement(child, &spatial).unwrap(), pose);
}
