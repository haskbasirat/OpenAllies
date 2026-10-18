use aa_actors::{
    ActorId,
    creation::{ActorCreationRecord, CreationRecord, ThingDefinition},
};
use aa_navigation::startup::Object;
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    sources::{SourceLimits, V14Sources},
    tgi::Limits,
};
use aa_runtime::{
    actor_world::{ActorStore, Source, SpecialSource},
    catalog::Catalog,
    localization::Localization,
    names::Table,
    restoration::Attempt,
    settings::Settings,
    trigger_world,
};
use aa_simulation::NativeRng;

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 source assets"]
fn source_names_and_victory_borrow_canonical_runtime_objects() {
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut source = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let settings = Settings::load_v14(&mut source, Limits::default()).unwrap();
    let text = Localization::load_v14(&mut source, Limits::default()).unwrap();
    let actors = ActorCatalog::load_v14(&mut source).unwrap();
    let map = aa_mapworld::MapArchive::from_bytes(
        std::fs::read(
            game.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial01.RMP"),
        )
        .unwrap(),
    )
    .unwrap();
    let attempt =
        Attempt::load(&map, &actors, settings, text, Precision::Double53, u64::MAX).unwrap();
    let mut store = ActorStore::from_restored(attempt.registry).unwrap();
    let kingdoms=store.identities().iter().filter_map(|e| {
        matches!(&store.entity(e.value).unwrap().source,Source::Special(s) if matches!(s.source,SpecialSource::Kingdom(_))).then_some((e.id,e.value))
    }).collect::<Vec<_>>();
    assert_eq!(kingdoms.len(), 3);
    let mut names = Table::default();
    //Explicit ordered publication for this composition test. This is not a
    //production scan that guesses native PostLoad/PostExisting registration.
    for (id, key) in &kingdoms {
        let object = Object(key.raw());
        let name = store.object(*key).unwrap().thing.name_utf16.clone();
        assert!(!name.is_empty());
        assert_eq!(trigger_world::resolve_id(&store, id.raw()), Some(object));
        assert_eq!(
            trigger_world::body_actor_identity(&store, object)
                .unwrap()
                .0,
            id.raw()
        );
        let before = store.staged_placement_mut(*key).unwrap().flags;
        assert!(trigger_world::publish_staged_name(&mut store, &mut names, object).unwrap());
        assert_eq!(
            store.staged_placement_mut(*key).unwrap().flags,
            before & 0x7fffffff
        );
        let uppercase = String::from_utf16(&name)
            .unwrap()
            .to_ascii_uppercase()
            .encode_utf16()
            .collect::<Vec<_>>();
        assert_eq!(
            trigger_world::named_kingdom(&store, &names, &uppercase).unwrap(),
            Some(object)
        );
        //Kingdom inherits actual virtual+B4=true, so named actor lookup includes it.
        assert_eq!(
            trigger_world::named_actor(&store, &names, &name).unwrap(),
            Some(object)
        );
        let original = trigger_world::default_victory(&mut store, object).unwrap();
        let Source::Special(s) = &store.entity(*key).unwrap().source else {
            panic!()
        };
        let old_word = s.derived.as_ref().unwrap().words[&0x238];
        trigger_world::set_default_victory(&mut store, object, 0).unwrap();
        let Source::Special(s) = &store.entity(*key).unwrap().source else {
            panic!()
        };
        assert_eq!(
            s.derived.as_ref().unwrap().words[&0x238],
            old_word & 0xffff_ff00
        );
        let SpecialSource::Kingdom(saved) = &s.source else {
            panic!()
        };
        assert_eq!(saved.payload.byte_238, Some(original));
    }
    assert!(trigger_world::resolve_id(&store, 0xffff_ffff).is_none());
    assert!(trigger_world::body_actor_identity(&store, Object(u64::MAX)).is_err());
    let catalog = Catalog::new(&actors);
    let mut rng = NativeRng::new(0x12345678);
    let request = ActorCreationRecord {
        base: CreationRecord {
            definition: ThingDefinition {
                key: "usa_infantry".into(),
                flags: 0,
            },
            position_bits: [0; 3],
            direction_bits: [0, (-1f32).to_bits()],
            name_utf16: "new_unit".encode_utf16().collect(),
            saved_id: ActorId::NULL,
        },
        variant: -1,
        owner_word: 0,
        word_30: 0,
        request_nonblocking: false,
    };
    let key = store
        .construct_new(
            &catalog,
            request,
            0,
            &mut rng,
            Precision::Double53,
            Precision::Double53,
        )
        .unwrap();
    let object = Object(key.raw());
    assert!(trigger_world::body_actor_identity(&store, object).is_err());
    assert!(trigger_world::publish_staged_name(&mut store, &mut names, object).is_err());
    let id = store
        .initialize_object(key, &mut rng, Precision::Double53)
        .unwrap();
    assert_eq!(
        trigger_world::body_actor_identity(&store, object)
            .unwrap()
            .0,
        id.raw()
    );
    assert!(trigger_world::publish_staged_name(&mut store, &mut names, object).unwrap());
    let alias = "alias".encode_utf16().collect::<Vec<_>>();
    assert!(trigger_world::register_alias(&store, &mut names, object, &alias).unwrap());
    assert_eq!(
        trigger_world::named_actor(&store, &names, &alias).unwrap(),
        Some(object)
    );
    assert!(
        trigger_world::named_kingdom(&store, &names, &alias)
            .unwrap()
            .is_none()
    );
    assert!(trigger_world::set_default_victory(&mut store, object, 0).is_err());
}
