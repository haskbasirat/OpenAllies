use super::*;

#[test]
#[ignore = "requires actual source and genuine stopped immediate-load prefix; not full mission"]
fn actual_adoption_keeps_one_memory_owner_and_reads_current_published_fields() {
    use crate::{
        localization::Localization,
        restoration::{Attempt, LoadBindings},
        restore_lifecycle::{ScopedManagedLoad, TaskKind},
        settings::Settings,
    };
    use aa_rules::{
        actors::ActorCatalog,
        resources::ResourceCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: u64::MAX,
        },
    )
    .unwrap();
    let settings = Settings::load_v14(&mut sources, Limits::default()).unwrap();
    let text = Localization::load_v14(&mut sources, Limits::default()).unwrap();
    let actor_source = ActorCatalog::load_v14(&mut sources).unwrap();
    let resources = std::sync::Arc::new(ResourceCatalog::load_v14(&mut sources).unwrap());
    let map = aa_mapworld::MapArchive::from_bytes(
        std::fs::read(
            game.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP"),
        )
        .unwrap(),
    )
    .unwrap();
    let session = aa_netcode::lobby::Session::default();
    struct SessionRef<'a>(&'a aa_netcode::lobby::Session);
    impl crate::load_world_globals::SessionPhase for SessionRef<'_> {
        fn current_phase(&self) -> std::result::Result<u32, String> {
            Ok(self.0.field_f8)
        }
        fn application_state(&self) -> std::result::Result<u32, String> {
            Ok(self.0.state)
        }
    }
    let session = SessionRef(&session);
    let inactive = ScopedManagedLoad::from_sources(&mut sources, 0)
        .unwrap()
        .begin(TaskKind::Normal)
        .unwrap();
    let mut attempt = Attempt::load_immediate(
        &map,
        &actor_source,
        settings.clone(),
        text,
        aa_numeric::Precision::Double53,
        u64::MAX,
        aa_mapworld::restore::RestoreMode::StopOnError,
        LoadBindings {
            native_load_mode: Some(1),
            inactive_load: Some(inactive),
            resources: Some(resources),
        },
        &mut sources,
        &session,
        false,
    )
    .unwrap();
    let mut world = attempt.load_world.take().unwrap();
    //The same genuine stopped load prefix is converted/adopted only for this
    //ownership regression. No PostLoad or naturally started mission is claimed.
    let mut actors =
        ActorStore::from_restored_with_abilities(attempt.registry, attempt.abilities).unwrap();
    actors.property_allocations = world.property_allocations.take().unwrap();
    let bank = attempt.definitions;
    let mut catalog = Catalog::new(&actor_source);
    catalog.settings = Some(settings);
    catalog.load_clock = Some(&attempt.clock);
    catalog.definition_text = Some(attempt.definition_text);
    catalog
        .bind_inactive_load(attempt.inactive_load.as_ref().unwrap())
        .unwrap();
    assert!(catalog.definition_bank.borrow().combat_source().is_err());
    let actor = ActorId::from_raw(28);
    let key = actors.key(actor).unwrap();
    assert_eq!(
        actors.object(key).unwrap().definition.proof.id,
        "temperate_pine_trees"
    );
    let memory_address =
        std::ptr::from_ref(&actors.object(key).unwrap().object.as_ref().unwrap().memory);
    //Explicit controlled visibility probe over this actual object/Kingdom
    //allocation: explored by all major Kingdoms, visible to none. This does not
    //assert these are naturally initialized mission fog values.
    actors
        .object_mut(key)
        .unwrap()
        .object
        .as_mut()
        .unwrap()
        .words_50_to_54[1] = 0xffff;
    let crate::load_world::SpatialOwner::Loading(Some(loaded)) = &world.spatial else {
        panic!("actual load-time spatial owner absent")
    };
    let wrong = crate::spatial_world::SpatialWorld::new(
        loaded.index().grid().clone(),
        world
            .terrain
            .bound
            .cells
            .iter()
            .map(|c| c.flags20)
            .collect(),
        loaded.index().precision(),
    )
    .unwrap();
    assert!(
        crate::load_store_memory_refresh::PlacementRead::Published(&wrong)
            .read(&actors, actor)
            .unwrap_err()
            .to_string()
            .contains("actual published owner")
    );
    //Explicit portable binding guard probe: phase5 would otherwise make the
    //native eligibility path return before any Kingdom/placement read. It
    //cannot make a still-Staged record an admitted published allocation.
    let mut skipped_session = aa_netcode::lobby::Session::default();
    skipped_session.field_f8 = 5;
    let skipped_phase = SessionRef(&skipped_session);
    let skipped_services =
        load_zone_services::Context::with_kingdoms(&world.point, &skipped_phase, &world.kingdoms)
            .with_source_fields(&catalog, &world.source_fields);
    let binding_error = refresh_memory_published(
        &catalog,
        &bank,
        &mut actors,
        &wrong,
        &skipped_services,
        &world.kingdoms,
        actor,
    )
    .unwrap_err();
    assert!(binding_error.to_string().contains("actual published owner"));
    drop(skipped_services);
    world.adopt_spatial(&mut actors).unwrap();
    assert!(actors.staged_placement(key).is_err());
    //The global Fog forwarding uses the SAME adopted member for every read
    //and temporary bit30 write. This controlled probe restores the original
    //flags before the pre-existing memory regression continues.
    {
        let crate::load_world::SpatialOwner::Adopted(adoption) = &mut world.spatial else {
            unreachable!()
        };
        let original = actors.placement(key, &adoption.world).unwrap();
        let member = adoption.world.key(key).unwrap();
        let address = std::ptr::from_ref(adoption.world.index().member(member).unwrap());
        let mut loan = FogSpatial::Published(&mut adoption.world);
        assert_eq!(loan.placement(&actors, actor).unwrap(), original);
        for flags in [original.flags & !(1 << 30), original.flags | (1 << 30), original.flags] {
            loan.set_flags(&mut actors, actor, flags).unwrap();
            let actual = loan.placement(&actors, actor).unwrap();
            assert_eq!(actual.flags, flags);
            assert_eq!(actual.position_bits, original.position_bits);
            assert_eq!(actual.facing_bits, original.facing_bits);
            assert_eq!(actual.cached_tile, original.cached_tile);
            assert_eq!(std::ptr::from_ref(loan.index().member(member).unwrap()), address);
            assert_eq!(loan.index().resolve_reference(actor.raw()), Some(member));
        }
        assert!(loan.set_flags(&mut actors, actor, original.flags ^ 2).is_err());
        assert!(loan.placement(&actors, ActorId::from_raw(actor.raw() ^ 0x10000)).is_err());
        assert_eq!(loan.placement(&actors, actor).unwrap(), original);
    }
    let spatial = world.live_spatial().unwrap();
    let services =
        load_zone_services::Context::with_kingdoms(&world.point, &session, &world.kingdoms)
            .with_source_fields(&catalog, &world.source_fields);
    refresh_memory_published(
        &catalog,
        &bank,
        &mut actors,
        spatial,
        &services,
        &world.kingdoms,
        actor,
    )
    .unwrap();
    let memory = &actors.object(key).unwrap().object.as_ref().unwrap().memory;
    assert_eq!(std::ptr::from_ref(memory), memory_address);
    let ids = (0..memory.count())
        .filter_map(|i| memory.slot(i as u32).unwrap())
        .collect::<Vec<_>>();
    assert!(!ids.is_empty());
    let expected = actors.placement(key, spatial).unwrap();
    let initial_flags = expected.flags;
    for id in &ids {
        let r = memory.record(*id).unwrap();
        assert_eq!(r.position_bits, expected.position_bits[..2]);
        assert_eq!(r.flags, expected.flags);
        assert_eq!(r.timestamp_bits, attempt.clock.begin.0);
        assert_eq!(r.definition.as_deref(), Some("temperate_pine_trees"));
    }
    let allocated = memory.allocated_records();
    refresh_memory_published(
        &catalog,
        &bank,
        &mut actors,
        spatial,
        &services,
        &world.kingdoms,
        actor,
    )
    .unwrap();
    assert_eq!(
        actors
            .object(key)
            .unwrap()
            .object
            .as_ref()
            .unwrap()
            .memory
            .allocated_records(),
        allocated
    );
    //The unchanged native4EC4A0 kernel must read the current spatial flags,
    //rather than any historical staged field copy, on every refresh.
    drop(services);
    let crate::load_world::SpatialOwner::Adopted(adoption) = &mut world.spatial else {
        unreachable!()
    };
    adoption.world.or_removal_flags(key, 2).unwrap();
    let spatial = &adoption.world;
    crate::load_store_memory_refresh::refresh_with_placement(
        &catalog,
        &bank,
        &mut actors,
        actor,
        ids[0],
        crate::load_store_memory_refresh::PlacementRead::Published(spatial),
    )
    .unwrap();
    let expected = actors.placement(key, spatial).unwrap();
    assert_ne!(expected.flags, initial_flags);
    assert_eq!(
        actors
            .object(key)
            .unwrap()
            .object
            .as_ref()
            .unwrap()
            .memory
            .record(ids[0])
            .unwrap()
            .flags,
        expected.flags
    );
    //A different world has no matching published generation. The error occurs
    //at position after the earlier record time/name writes, with no rollback.
    actors
        .object_mut(key)
        .unwrap()
        .object
        .as_mut()
        .unwrap()
        .memory
        .record_mut(ids[0])
        .unwrap()
        .timestamp_bits = 0xa5a5a5a5;
    let result = crate::load_store_memory_refresh::refresh_with_placement(
        &catalog,
        &bank,
        &mut actors,
        actor,
        ids[0],
        crate::load_store_memory_refresh::PlacementRead::Published(&wrong),
    );
    assert!(result.is_err());
    assert_eq!(
        actors
            .object(key)
            .unwrap()
            .object
            .as_ref()
            .unwrap()
            .memory
            .record(ids[0])
            .unwrap()
            .timestamp_bits,
        attempt.clock.begin.0
    );
    //Now genuinely execute the existing native delete/clear branches on these
    //same records after making the controlled visibility word visible.
    actors
        .object_mut(key)
        .unwrap()
        .object
        .as_mut()
        .unwrap()
        .words_50_to_54[1] = u32::MAX;
    let services =
        load_zone_services::Context::with_kingdoms(&world.point, &session, &world.kingdoms)
            .with_source_fields(&catalog, &world.source_fields);
    refresh_memory_published(
        &catalog,
        &bank,
        &mut actors,
        spatial,
        &services,
        &world.kingdoms,
        actor,
    )
    .unwrap();
    let memory = &actors.object(key).unwrap().object.as_ref().unwrap().memory;
    assert_eq!(std::ptr::from_ref(memory), memory_address);
    assert_eq!(memory.count(), 0);
    assert!(!memory.allocation_present());
    assert_eq!(memory.allocated_records(), 0);
    eprintln!(
        "published memory same owner: actor28; created {allocated} records; sourcePC {:?}; livePC {:?}; retained error prefix; deleted same records",
        bank.combat_source().unwrap().source_precision(),
        spatial.index().precision()
    );
}
