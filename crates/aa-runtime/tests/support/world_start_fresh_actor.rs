//!Fresh constructor/source regression, not a fabricated initialized mission.
use super::*;
use aa_actors::creation::{ActorCreationRecord, CreationRecord, ThingDefinition};
use aa_runtime::{catalog::Catalog, localization::Localization, settings::Settings};
use aa_simulation::NativeRng;
use std::sync::Arc;

fn probe(world: &mut World<'_>, key: aa_runtime::actor_world::ObjectKey, raw: u32, accepted: bool) {
    let before = format!("{:?}", world.actors.entity(key).unwrap());
    let address = std::ptr::from_ref(world.actors.entity(key).unwrap());
    let identities = world.objects().unwrap();
    let started = *world.started;
    let result = world.object_active(raw);
    if accepted {
        assert!(result.unwrap());
    } else {
        assert!(result.is_err(), "poisoned ownership must refuse, not return false");
    }
    assert_eq!(format!("{:?}", world.actors.entity(key).unwrap()), before);
    assert_eq!(std::ptr::from_ref(world.actors.entity(key).unwrap()), address);
    assert_eq!(world.objects().unwrap(), identities);
    assert_eq!(*world.started, started);
    assert!(world.economy.is_empty());
}

#[test]
#[ignore = "requires AA_GAME_ROOT for real source-backed Actor constructor"]
fn actual_source_fresh_actor_world_start_b4_preserves_owner_and_rejects_poison() {
    use aa_rules::{resources::ResourceCatalog as Resources, sources::{SourceLimits, V14Sources}};
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files = V14Sources::open(game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {max_member_bytes: u64::MAX}).unwrap();
    let settings = Settings::load_v14(&mut files, Limits::default()).unwrap();
    let text = Localization::load_v14(&mut files, Limits::default()).unwrap();
    let source = ActorCatalog::load_v14(&mut files).unwrap();
    let resource_source = Arc::new(Resources::load_v14(&mut files).unwrap());
    let definitions = aa_runtime::kingdom_economy::Definitions::bind(
        &resource_source, &source, Precision::Double53).unwrap();
    let formulas = bind_formulas(&source, definitions.catalog(), Precision::Double53).unwrap();
    let mut catalog = Catalog::new(&source);
    catalog.settings = Some(settings);
    catalog.definition_text = Some(text.clone());
    *catalog.localization.borrow_mut() = Some(text);
    catalog.bind_resource_catalog(resource_source).unwrap();
    let binding = catalog.actor_bindings("ger_corps", Precision::Double53).unwrap();
    for live_pc in [Precision::Single24, Precision::Double53, Precision::Extended64] {
        let mut actors = ActorStore::new();
        let mut rng = NativeRng::new(0x12345678);
        let request = ActorCreationRecord {
            base: CreationRecord {definition: ThingDefinition {key: "ger_corps".into(), flags: 0},
                position_bits: [0;3], direction_bits: [0, (-1f32).to_bits()],
                name_utf16: vec![], saved_id: ActorId::NULL},
            variant: 0, owner_word: 0, word_30: 0, request_nonblocking: false,
        };
        //The same real source factory used by map creation. StartupPC24 is an
        //explicit constructor-laboratory input, not a live StartupInputs proof.
        let key = actors.construct_new_bound(&catalog, Arc::clone(&binding), request,
            0, &mut rng, Precision::Single24, live_pc).unwrap();
        let current = actors.initialize_object(key, &mut rng, live_pc).unwrap();
        assert_ne!(current, ActorId::NULL);
        let Source::New {record, ..} = &actors.entity(key).unwrap().source else {panic!()};
        assert_eq!(record.base.saved_id, ActorId::NULL);
        assert!(aa_runtime::post_load_world::object_class(actors.entity(key).unwrap()).is_err());
        let rng_after_factory = rng.state;
        let mut economy = BTreeMap::new();
        //Only the class predicate is called. This supplied sentinel is not a
        //World constructor/mission-ready value and must remain untouched.
        let mut started = true;
        let mut world = World {actors: &mut actors, economy: &mut economy, started: &mut started,
            major_kingdoms: &[], source: &source, resources: definitions.catalog(),
            formulas: &formulas, source_precision: Precision::Double53, precision: live_pc};
        probe(&mut world, key, current.raw(), true);
        probe(&mut world, key, current.raw() ^ 0x10000, false);

        let old_flags = world.actors.staged_placement(key).unwrap().flags;
        world.actors.staged_placement_mut(key).unwrap().flags |= 0x8000_0083;
        probe(&mut world, key, current.raw(), true); //class leaf is not liveness
        world.actors.staged_placement_mut(key).unwrap().flags = old_flags;
        for phase in [aa_runtime::actor_world::NewPhase::IdentityRegistered,
            aa_runtime::actor_world::NewPhase::AwaitingInitCreate] {
            let Source::New {phase: p, failure, ..} = &mut world.actors.entity_mut(key).unwrap().source else {panic!()};
            *p = phase;
            *failure = Some("explicit bookkeeping independence probe".into());
            probe(&mut world, key, current.raw(), true);
        }

        let old = world.actors.object_mut(key).unwrap().kind;
        world.actors.object_mut(key).unwrap().kind = LoadKind::Object;
        probe(&mut world, key, current.raw(), false);
        world.actors.object_mut(key).unwrap().kind = old;
        let actor = world.actors.object_mut(key).unwrap().actor.take();
        probe(&mut world, key, current.raw(), false);
        world.actors.object_mut(key).unwrap().actor = actor;
        let object = world.actors.object_mut(key).unwrap().object.take();
        probe(&mut world, key, current.raw(), false);
        world.actors.object_mut(key).unwrap().object = object;

        world.actors.object_mut(key).unwrap().definition.native_class = 2;
        probe(&mut world, key, current.raw(), false);
        world.actors.object_mut(key).unwrap().definition.native_class = 3;
        let proof = world.actors.object(key).unwrap().definition.proof.clone();
        world.actors.object_mut(key).unwrap().definition.proof.category.runtime_class = 2;
        probe(&mut world, key, current.raw(), false);
        world.actors.object_mut(key).unwrap().definition.proof = proof.clone();
        world.actors.object_mut(key).unwrap().definition.proof.category.native_kind = 1;
        probe(&mut world, key, current.raw(), false);
        world.actors.object_mut(key).unwrap().definition.proof = proof;

        let proof = world.actors.object(key).unwrap().thing.definition.clone();
        world.actors.object_mut(key).unwrap().thing.definition.source.sha256.push('x');
        probe(&mut world, key, current.raw(), false);
        world.actors.object_mut(key).unwrap().thing.definition = proof;
        let Source::New {record, ..} = &mut world.actors.entity_mut(key).unwrap().source else {panic!()};
        let proof = record.base.definition.key.clone();
        record.base.definition.key.id.push_str("-forged");
        probe(&mut world, key, current.raw(), false);
        let Source::New {record, ..} = &mut world.actors.entity_mut(key).unwrap().source else {panic!()};
        record.base.definition.key = proof;

        let retained = world.actors.entity_mut(key).unwrap().bindings.take().unwrap();
        probe(&mut world, key, current.raw(), false);
        let mut poisoned = retained.as_ref().clone();
        poisoned.precision = Precision::Single24;
        world.actors.entity_mut(key).unwrap().bindings = Some(Arc::new(poisoned));
        probe(&mut world, key, current.raw(), false);
        let mut poisoned = retained.as_ref().clone();
        poisoned.definition.proof.id.push_str("-forged");
        world.actors.entity_mut(key).unwrap().bindings = Some(Arc::new(poisoned));
        probe(&mut world, key, current.raw(), false);
        world.actors.entity_mut(key).unwrap().bindings = Some(retained);
        probe(&mut world, key, current.raw(), true);
        assert_eq!(rng.state, rng_after_factory);
    }
}

#[test]
fn world_start_saved_group_b4_stays_true_while_inert_stays_false() {
    let source = empty_source();
    let resources = resource_count(0);
    let formulas = DerivedCatalog {rows: vec![]};
    let mut registry = IdentityRegistry::new();
    actor(&mut registry, id(7), false, 0, None, false, false);
    let def = definition("explicit saved Group predicate fixture", 8);
    let shell = construct_shell_with_components::<_, RestoredComponent>(LoadKind::Actor, def.proof.clone());
    let record = LoadedEntity {
        source: Placement {record_ordinal: 0, source_span: span(), thing_id_utf16: vec![],
            class_code: 8, serialized_id: 9, thing: thing(), object: None, actor: None,
            opaque_tail_span: span(), opaque_tail: vec![]},
        definition: def, shell, completion: LoadCompletion::PostLoaded,
        owner_source_slot: None, saved_components: vec![], actor_tail: None,
        doodad: None, special_object: None, failure: None, active_component: None,
    };
    registry.register_saved(id(9), true, RestoredRecord::Actor(Box::new(record))).unwrap();
    let mut actors = ActorStore::from_restored(registry).unwrap();
    let mut economy = BTreeMap::new();
    let mut started = false;
    let mut world = World {actors: &mut actors, economy: &mut economy, started: &mut started,
        major_kingdoms: &[], source: &source, resources: &resources, formulas: &formulas,
        source_precision: Precision::Double53, precision: Precision::Double53};
    assert!(!world.object_active(7).unwrap());
    assert!(world.object_active(9).unwrap());
}
