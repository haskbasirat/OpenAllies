use aa_actors::{
    ActorId,
    creation::{ActorCreationRecord, CreationRecord, ThingDefinition},
};
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    sources::{SourceLimits, V14Sources},
};
use aa_runtime::{
    actor_world::{ActorStore, ObjectKey, Spatial},
    catalog::Catalog,
    spatial_world::SpatialWorld,
};
use aa_simulation::NativeRng;
use aa_spatial::{
    collision::Grid,
    index::{SpatialIndex, TransformCallbacks},
    soft_range,
};

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
fn actor(
    store: &mut ActorStore,
    catalog: &Catalog<'_>,
    name: &str,
    x: f32,
    rng: &mut NativeRng,
) -> ObjectKey {
    let record = ActorCreationRecord {
        base: CreationRecord {
            definition: ThingDefinition {
                key: name.into(),
                flags: 0,
            },
            position_bits: [x.to_bits(), 12f32.to_bits(), 0],
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
        .construct_new(
            catalog,
            record,
            0,
            rng,
            Precision::Double53,
            Precision::Double53,
        )
        .unwrap();
    store
        .initialize_object(key, rng, Precision::Double53)
        .unwrap();
    key
}
fn spatial() -> SpatialWorld {
    SpatialWorld::new(
        Grid {
            dimensions: [64, 64],
            coordinate_shift: 2,
            cell_size_bits: 4f32.to_bits(),
        },
        vec![0; 64 * 64],
        Precision::Double53,
    )
    .unwrap()
}
struct FailAfterCommit {
    observed: Option<[u32; 3]>,
}
impl TransformCallbacks for FailAfterCommit {
    fn position_changed(&mut self, index: &mut SpatialIndex, key: u32) -> aa_spatial::Result<()> {
        let m = index.member(key)?;
        self.observed = Some([m.actor.position[0], m.actor.position[1], m.height_bits]);
        Err(aa_spatial::Error::World(
            "deliberate integration callback failure after actual commit".into(),
        ))
    }
    fn world_phase(&self) -> u32 {
        panic!("must stop on failed position effect")
    }
    fn presentation_available(&self) -> bool {
        panic!("must stop on failed position effect")
    }
    fn presentation_position_changed(
        &mut self,
        _: &mut SpatialIndex,
        _: u32,
    ) -> aa_spatial::Result<()> {
        panic!("must stop on failed position effect")
    }
    fn presentation_facing_changed(
        &mut self,
        _: &mut SpatialIndex,
        _: u32,
    ) -> aa_spatial::Result<()> {
        panic!("not used")
    }
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 definitions"]
fn source_actor_publication_membership_and_soft_geometry_use_one_pose() {
    let definitions = source();
    let catalog = Catalog::new(&definitions);
    let mut store = ActorStore::new();
    let mut world = spatial();
    let mut rng = NativeRng::new(0x12345678);
    let mut report = vec![];
    for name in [
        "usa_infantry",
        "ger_infantry",
        "gbr_infantry",
        "jpn_infantry",
        "rus_infantry",
        "regiment_usa_infantry",
    ] {
        let a = actor(&mut store, &catalog, name, 10., &mut rng);
        let b = actor(&mut store, &catalog, name, 30., &mut rng);
        // Renderer scale never changes native soft/blocking/target descriptors.
        store
            .object_mut(b)
            .unwrap()
            .object
            .as_mut()
            .unwrap()
            .scale_bits = 8f32.to_bits();
        let before_a = store.placement(a, &world).unwrap();
        let before_b = store.placement(b, &world).unwrap();
        world.publish_actor(&mut store, &catalog, a).unwrap();
        world.publish_actor(&mut store, &catalog, b).unwrap();
        assert!(store.staged_placement_mut(a).is_err());
        assert_eq!(store.placement(a, &world).unwrap(), before_a);
        assert_eq!(store.placement(b, &world).unwrap(), before_b);
        let ak = world.key(a).unwrap();
        let bk = world.key(b).unwrap();
        assert_eq!(world.object(ak).unwrap(), a);
        let definition = world.index().member(ak).unwrap().actor.definition;
        assert_eq!(
            world.index().member(bk).unwrap().actor.definition,
            definition
        );
        let source = catalog
            .actors
            .native_object_definition(name, Precision::Double53)
            .unwrap();
        let member = world.index().member(bk).unwrap();
        assert_eq!(member.soft.kind, source.soft.words[0]);
        assert_eq!(member.soft.radius_bits, source.soft.words[1]);
        assert_eq!(member.blocking.radius_bits, source.blocking.words[1]);
        world.refresh_membership(a).unwrap();
        world.refresh_membership(b).unwrap();
        assert_ne!(store.placement(a, &world).unwrap().flags & (1 << 26), 0);
        assert_eq!(
            world.index().resolve_reference(store.id(a).unwrap().raw()),
            Some(ak)
        );
        assert!(
            !soft_range::source_first_in_range(
                a,
                b,
                0x3dcccccd,
                Precision::Double53,
                &mut world.soft_view(&store)
            )
            .unwrap()
        );
        let committed = [10f32.to_bits(), 12f32.to_bits(), 3f32.to_bits()];
        let mut callback = FailAfterCommit { observed: None };
        assert!(world.commit_position(b, committed, &mut callback).is_err());
        assert_eq!(callback.observed, Some(committed));
        assert_eq!(store.placement(b, &world).unwrap().position_bits, committed);
        assert_eq!(
            world.placement(store.id(b).unwrap()).unwrap().position_bits,
            committed
        );
        assert!(
            soft_range::source_first_in_range(
                a,
                b,
                0x3dcccccd,
                Precision::Double53,
                &mut world.soft_view(&store)
            )
            .unwrap()
        );
        assert!(world.index().actors_at([2, 3]).unwrap().contains(&bk));
        let a_id = store.id(a).unwrap();
        let b_id = store.id(b).unwrap();
        {
            let (_, access) = store.split_minds();
            assert!(
                soft_range::source_receiver_in_range(
                    a,
                    b,
                    0x3dcccccd,
                    Precision::Double53,
                    &mut world.soft_view(&access)
                )
                .unwrap()
            );
            assert!(
                aa_spatial::range::actors_in_range(
                    a_id.raw(),
                    b_id.raw(),
                    0x3dcccccd,
                    false,
                    Precision::Double53,
                    &mut world.target_view(&access)
                )
                .unwrap()
            );
        }
        {
            let (index, bindings) = world.split_index();
            assert_eq!(bindings.object(ak).unwrap(), a);
            assert_eq!(bindings.key(b).unwrap(), bk);
            assert_eq!(
                bindings
                    .source_geometry(definition)
                    .unwrap()
                    .source
                    .soft
                    .words,
                source.soft.words
            );
            assert_eq!(index.member(bk).unwrap().actor.position, committed[..2]);
        }
        world.set_membership_request(b, true).unwrap();
        world.set_membership_request(b, false).unwrap();
        assert_eq!(store.placement(b, &world).unwrap().flags & 8, 0);
        assert_ne!(store.placement(b, &world).unwrap().flags & (1 << 26), 0);
        report.push(serde_json::json!({"source":name,"source_shape_words":source.soft.words,"actor_gtids":[store.id(a).unwrap().raw(),store.id(b).unwrap().raw()],"spatial_tokens":[ak,bk],"definition_token":definition,"committed_pose":committed}));
    }
    assert_eq!(world.index().members().count(), 12);
    if let Some(file) = std::env::var_os("AA_SPATIAL_SOURCE_OUTPUT") {
        std::fs::write(file, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
}
#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 definitions"]
fn rejected_publication_keeps_staged_authority_and_rejects_derived_shapes() {
    let definitions = source();
    let catalog = Catalog::new(&definitions);
    let mut store = ActorStore::new();
    let mut world = spatial();
    let mut rng = NativeRng::new(17);
    let a = actor(&mut store, &catalog, "usa_infantry", 10., &mut rng);
    store.staged_placement_mut(a).unwrap().flags |= 1 << 26;
    assert!(world.publish_actor(&mut store, &catalog, a).is_err());
    assert_eq!(world.index().members().count(), 0);
    assert!(store.staged_placement_mut(a).is_ok());
    store.staged_placement_mut(a).unwrap().flags &= !(1 << 26);
    store.staged_placement_mut(a).unwrap().position_bits[2] = f32::NAN.to_bits();
    assert!(world.publish_actor(&mut store, &catalog, a).is_err());
    assert_eq!(world.index().members().count(), 0);
    assert!(store.staged_placement_mut(a).is_ok());
    store.staged_placement_mut(a).unwrap().position_bits[2] = 0;
    store.object_mut(a).unwrap().definition.native_class = 4;
    assert!(world.publish_actor(&mut store, &catalog, a).is_err());
    assert_eq!(world.index().members().count(), 0);
    assert!(store.staged_placement_mut(a).is_ok());
    store.object_mut(a).unwrap().definition.native_class = 3;
    world.publish_actor(&mut store, &catalog, a).unwrap();
    assert!(world.publish_actor(&mut store, &catalog, a).is_err());
    assert_eq!(world.index().members().count(), 1);
    let id = store.id(a).unwrap();
    world.index_mut().retire_reference(id.raw());
    assert!(store.placement(a, &world).is_err());
    assert!(
        soft_range::source_first_in_range(
            a,
            a,
            0,
            Precision::Double53,
            &mut world.soft_view(&store)
        )
        .is_err()
    );
    assert!(
        aa_spatial::range::actors_in_range(
            id.raw(),
            id.raw(),
            0,
            false,
            Precision::Double53,
            &mut world.target_view(&store)
        )
        .is_err()
    );
}
