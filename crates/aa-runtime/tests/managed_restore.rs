//! Explicit stopped-load capabilities; these are record diagnostics, not
//! certification of the still-missing per-record Init/PostCreate/publication.
use aa_numeric::Precision;
use aa_rules::{actors::ActorCatalog, resources::ResourceCatalog,
    sources::{SourceLimits, V14Sources}, tgi::Limits};
use aa_runtime::{catalog::Catalog, localization::Localization,
    restoration::{Attempt, Error, LoadBindings}, restore_lifecycle::{ScopedManagedLoad, TaskKind},
    settings::Settings};
use std::sync::Arc;

fn sources() -> V14Sources {
    let root = root();
    V14Sources::open(root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits { max_member_bytes: 64 * 1024 * 1024 }).unwrap()
}
fn root() -> std::path::PathBuf {
    std::env::var_os("AA_GAME_ROOT").expect("original/v14 source required").into()
}
fn map(name: &str) -> aa_mapworld::MapArchive {
    aa_mapworld::MapArchive::from_bytes(std::fs::read(
        root().join("reconstruction/assets/original/Data/Maps").join(name)).unwrap()).unwrap()
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 source"]
fn managed_owner_and_immutable_sources_move_into_staging_without_activation() {
    let mut sources = sources();
    let settings = Settings::load_v14(&mut sources, Limits::default()).unwrap();
    let text = Localization::load_v14(&mut sources, Limits::default()).unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let resources = Arc::new(ResourceCatalog::load_v14(&mut sources).unwrap());
    // Explicit constructor clock fixture, unrelated to the saved world's RNG.
    let token = ScopedManagedLoad::from_sources(&mut sources, 375).unwrap()
        .begin(TaskKind::Normal).unwrap();
    let address = token.synchronization_owner_address();
    let attempt = Attempt::load_with_bindings(
        &map("DeveloperMaps/(3v3)Firefight.RMP"), &actors, settings, text,
        Precision::Double53, u64::MAX, aa_mapworld::restore::RestoreMode::StopOnError,
        LoadBindings { native_load_mode: None, inactive_load: Some(token),
            resources: Some(Arc::clone(&resources)) }).unwrap();
    assert!(attempt.report.stopped_before.is_none());
    assert!(attempt.report.events.iter().all(|event| event.error.is_none()));
    assert_eq!(attempt.native_load_mode, Some(1));
    assert_eq!(attempt.inactive_load.as_ref().unwrap().synchronization_owner_address(), address);
    assert!(Arc::ptr_eq(attempt.resource_source.as_ref().unwrap(), &resources));
    let mut staged = attempt.into_staged().unwrap();
    assert_eq!(staged.inactive_load.as_ref().unwrap().synchronization_owner_address(), address);
    assert!(Arc::ptr_eq(staged.resource_source.as_ref().unwrap(), &resources));
    staged.post_load(&actors, Precision::Double53).unwrap();
    assert_eq!(staged.rng.state, 3_627_657_861);
    assert!(!staged.report.runtime_publication_ready);
    assert!(staged.init_for_loaded_map().is_err());
    let token = staged.inactive_load.as_mut().unwrap();
    assert!(token.observer_present().unwrap());
    token.notify_observer(0xdeadbeef).unwrap(); // inactive callback never reads it
    assert!(token.request_world_ready().is_err());
    assert!(token.observer_present().is_err());
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 source"]
fn explicit_sources_cache_by_precision_and_cannot_replace_their_bound_owner() {
    let mut sources = sources();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let other_owner = actors.clone();
    let resources = Arc::new(ResourceCatalog::load_v14(&mut sources).unwrap());
    let mut catalog = Catalog::new(&actors);
    assert!(catalog.native_resource_economy(Precision::Double53).is_err());
    catalog.bind_resource_catalog(Arc::clone(&resources)).unwrap();
    let a = catalog.native_resource_economy(Precision::Double53).unwrap();
    let b = catalog.native_resource_economy(Precision::Double53).unwrap();
    assert!(Arc::ptr_eq(&a, &b));
    assert_eq!(a.resource_count(), 7);
    assert!(catalog.bind_resource_catalog(resources).is_err());
    assert!(Arc::ptr_eq(&a, &catalog.native_resource_economy(Precision::Double53).unwrap()));
    let c = catalog.native_recruit_backrefs(Precision::Double53).unwrap();
    assert!(Arc::ptr_eq(&c, &catalog.native_recruit_backrefs(Precision::Double53).unwrap()));
    // Both are real source actors; neither test substitutes the transport field.
    assert_eq!(c.present("usa_infantry").unwrap(), actors.native_recruit_backrefs(Precision::Double53)
        .unwrap().present("usa_infantry").unwrap());
    catalog.actors = &other_owner;
    assert!(catalog.native_resource_economy(Precision::Double53).err().unwrap()
        .contains("source owner changed"));
    assert!(catalog.native_recruit_backrefs(Precision::Double53).unwrap_err()
        .contains("source owner changed"));
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 source"]
fn mismatched_or_invalidated_managed_modes_fail_before_record_loading() {
    let mut sources = sources();
    let settings = Settings::load_v14(&mut sources, Limits::default()).unwrap();
    let text = Localization::load_v14(&mut sources, Limits::default()).unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let data = map("DeveloperMaps/(3v3)Firefight.RMP");
    let token = ScopedManagedLoad::from_sources(&mut sources, 1).unwrap()
        .begin(TaskKind::Editor).unwrap();
    let result = Attempt::load_with_bindings(&data, &actors, settings.clone(), text.clone(),
        Precision::Double53, u64::MAX, aa_mapworld::restore::RestoreMode::StopOnError,
        LoadBindings { native_load_mode: Some(1), inactive_load: Some(token), resources: None });
    assert!(matches!(result, Err(Error::Lifecycle(_))));
    let mut token = ScopedManagedLoad::from_sources(&mut sources, 2).unwrap()
        .begin(TaskKind::Normal).unwrap();
    assert!(token.request_world_ready().is_err());
    let result = Attempt::load_with_bindings(&data, &actors, settings, text,
        Precision::Double53, u64::MAX, aa_mapworld::restore::RestoreMode::StopOnError,
        LoadBindings { native_load_mode: Some(1), inactive_load: Some(token), resources: None });
    assert!(matches!(result, Err(Error::Lifecycle(_))));
}
