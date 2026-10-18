use aa_actors::{
    ActorId,
    components::ComponentSlot,
    creation::{ActorCreationRecord, CreationRecord, ThingDefinition},
};
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    sources::{SourceLimits, V14Sources},
};
use aa_runtime::{
    actor_world::{ActiveComponent, ActorStore, ObjectKey, RuntimeComponent},
    catalog::Catalog,
    relations::Relations,
    spatial_policy::{CollisionDefinitions, PolicyInputs},
    spatial_world::SpatialWorld,
};
use aa_simulation::NativeRng;
use aa_spatial::{
    collision::{Descriptor, Grid},
    index_world::CollisionPolicy,
    query::{PreparedQuery, QueryGeometry, first_hit},
};
use std::collections::BTreeMap;
const PC: Precision = Precision::Double53;
fn source() -> ActorCatalog {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    ActorCatalog::load_v14(&mut files).unwrap()
}
fn make(
    store: &mut ActorStore,
    catalog: &Catalog<'_>,
    rng: &mut NativeRng,
    name: &str,
) -> ObjectKey {
    let record = ActorCreationRecord {
        base: CreationRecord {
            definition: ThingDefinition {
                key: name.into(),
                flags: 0,
            },
            position_bits: [20f32.to_bits(), 20f32.to_bits(), 0],
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
        .construct_new(catalog, record, 0, rng, PC, PC)
        .unwrap();
    store.initialize_object(key, rng, PC).unwrap();
    key
}
fn world() -> SpatialWorld {
    SpatialWorld::new(
        Grid {
            dimensions: [32, 32],
            coordinate_shift: 2,
            cell_size_bits: 4f32.to_bits(),
        },
        vec![0; 1024],
        PC,
    )
    .unwrap()
}
fn relations() -> Relations {
    Relations {
        orphan_relation: 3,
        team_null_relation: 3,
        teams: BTreeMap::new(),
        kingdoms: BTreeMap::new(),
    }
}

#[test]
#[ignore = "requires original source assets via AA_GAME_ROOT"]
fn live_source_actors_exclusions_and_query_have_no_duplicate_runtime_state() {
    let source = source();
    let catalog = Catalog::new(&source);
    let definitions =
        CollisionDefinitions::prepare(&catalog, &["usa_infantry", "regiment_usa_infantry"], PC)
            .unwrap();
    let relations = relations();
    let inputs = PolicyInputs {
        definitions: &definitions,
        relations: &relations,
    };
    let mut store = ActorStore::new();
    let mut spatial = world();
    let mut rng = NativeRng::new(0x12345678);
    let a = make(&mut store, &catalog, &mut rng, "usa_infantry");
    let b = make(&mut store, &catalog, &mut rng, "usa_infantry");
    store
        .construct_ground(a, ComponentSlot::Element, aa_body::F32(0), PC)
        .unwrap();
    store
        .construct_ground(b, ComponentSlot::Element, aa_body::F32(0), PC)
        .unwrap();
    store
        .construct_ground(b, ComponentSlot::Mover, aa_body::F32(0), PC)
        .unwrap();
    let aid = store.id(a).unwrap();
    let bid = store.id(b).unwrap();
    for object in [a, b] {
        spatial.publish_actor(&mut store, &catalog, object).unwrap();
        spatial.refresh_membership(object).unwrap();
    }
    let ak = spatial.key(a).unwrap();
    let bk = spatial.key(b).unwrap();
    let geometry =
        QueryGeometry::from_native_extents(spatial.index(), [128f32.to_bits(); 2], PC).unwrap();
    {
        let (arena, access) = store.split_minds();
        let (index, bindings) = spatial.split_index();
        let mut policy = inputs.bind(&access, arena, bindings);
        assert!(
            policy
                .source_excludes(index, ak, ak, &mut Descriptor::default())
                .unwrap()
        );
        assert!(
            !policy
                .source_excludes(index, ak, bk, &mut Descriptor::default())
                .unwrap()
        );
        assert!(
            !policy
                .moving_exclusion(index, bk, &mut Descriptor::default())
                .unwrap()
        );
        // Explicit query flags include every placed actor; geometry and source
        // policy remain the actual recovered kernels over the published actors.
        let mut query = PreparedQuery::for_actor(index, ak, false, 0, &policy).unwrap();
        query.required_actor_flags = 0;
        query.descriptor.mask = 0;
        let hit = first_hit(index, &geometry, &query, &mut policy).unwrap();
        assert!(hit.blocked());
        assert_eq!(hit.reference, bid.raw());
    }
    store
        .object_mut(a)
        .unwrap()
        .object
        .as_mut()
        .unwrap()
        .words_50_to_54[0] = bid.raw();
    store.object_mut(b).unwrap().active_component = ActiveComponent::Slot(ComponentSlot::Mover);
    {
        let (arena, access) = store.split_minds();
        let (index, bindings) = spatial.split_index();
        let mut policy = inputs.bind(&access, arena, bindings);
        assert!(
            policy
                .source_excludes(index, ak, bk, &mut Descriptor::default())
                .unwrap()
        );
        assert!(
            policy
                .moving_exclusion(index, bk, &mut Descriptor::default())
                .unwrap()
        );
    }
    store
        .object_mut(a)
        .unwrap()
        .object
        .as_mut()
        .unwrap()
        .words_50_to_54[0] = bid.raw() ^ (1 << 16);
    store
        .object_mut(a)
        .unwrap()
        .actor
        .as_mut()
        .unwrap()
        .words
        .insert(0x110, 0x1234);
    {
        let (arena, access) = store.split_minds();
        let (index, bindings) = spatial.split_index();
        let mut policy = inputs.bind(&access, arena, bindings);
        assert!(
            policy
                .source_excludes(index, ak, bk, &mut Descriptor::default())
                .unwrap_err()
                .to_string()
                .contains("exclusion list")
        );
    }
    store
        .object_mut(a)
        .unwrap()
        .actor
        .as_mut()
        .unwrap()
        .words
        .remove(&0x110);
    let overlap_node = store
        .object_mut(a)
        .unwrap()
        .actor
        .as_mut()
        .unwrap()
        .overlap_list_110
        .prepend(aa_actors::overlap_list::Entry {
            reference: bid.raw(),
            expiry_bits: 0,
        })
        .unwrap();
    {
        let (arena, access) = store.split_minds();
        let (index, bindings) = spatial.split_index();
        let mut policy = inputs.bind(&access, arena, bindings);
        assert!(
            policy
                .source_excludes(index, ak, bk, &mut Descriptor::default())
                .unwrap()
        );
    }
    store
        .object_mut(a)
        .unwrap()
        .actor
        .as_mut()
        .unwrap()
        .overlap_list_110
        .remove(overlap_node)
        .unwrap();
    let mut leaders = vec![];
    for _ in 0..2 {
        let key = make(&mut store, &catalog, &mut rng, "regiment_usa_infantry");
        store
            .construct_ground(key, ComponentSlot::Organization, aa_body::F32(0), PC)
            .unwrap();
        store.construct_mind(key, &mut rng, PC).unwrap();
        let id = store.id(key).unwrap();
        let Some(RuntimeComponent::Mind(value)) = store.component(id, ComponentSlot::Mind).unwrap()
        else {
            panic!()
        };
        let state = value.state_bindings[&0x68];
        store.minds.state_mut(state).unwrap().status = 2;
        leaders.push((id, key, state));
    }
    for (unit, leader) in [(aid, leaders[0].0), (bid, leaders[1].0)] {
        let Some(RuntimeComponent::Element(value)) =
            store.component_mut(unit, ComponentSlot::Element).unwrap()
        else {
            panic!()
        };
        value.organization = Some(leader);
    }
    {
        let (arena, access) = store.split_minds();
        let (index, bindings) = spatial.split_index();
        let mut policy = inputs.bind(&access, arena, bindings);
        // Formation leaders need not be spatially published for real Mind64.
        assert!(
            policy
                .source_excludes(index, ak, bk, &mut Descriptor::default())
                .unwrap()
        );
    }
    store.minds.state_mut(leaders[1].2).unwrap().status = 0;
    {
        let (arena, access) = store.split_minds();
        let (index, bindings) = spatial.split_index();
        let mut policy = inputs.bind(&access, arena, bindings);
        assert!(
            !policy
                .source_excludes(index, ak, bk, &mut Descriptor::default())
                .unwrap()
        );
    }
}
