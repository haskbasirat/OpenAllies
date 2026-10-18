//! Direct owner call uses a real restored Organization. Present Economy paths
//! are covered by the unchanged saved_economy_refresh adapter's own native and
//! actual-data tests; this test does not fabricate that component on an owner.
use aa_actors::components::ComponentSlot;
use aa_mapworld::restore::{OrganizationAttachContext, RestoredComponent, RestoredRecord};
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    resources::ResourceCatalog,
    sources::{SourceLimits, V14Sources},
    tgi::Limits,
};
use aa_runtime::{
    catalog::Catalog,
    localization::Localization,
    restoration::{Attempt, LoadBindings},
    restore_lifecycle::{ScopedManagedLoad, TaskKind},
    settings::Settings,
};
use serde_json::json;
use std::sync::Arc;

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 assets"]
fn actual_saved_owner_absent_economy_requires_no_owner_word_or_flow_defaults() {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let settings = Settings::load_v14(&mut sources, Limits::default()).unwrap();
    let text = Localization::load_v14(&mut sources, Limits::default()).unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let resources = Arc::new(ResourceCatalog::load_v14(&mut sources).unwrap());
    let inactive_load = ScopedManagedLoad::from_sources(&mut sources, 0)
        .unwrap()
        .begin(TaskKind::Normal)
        .unwrap();
    let map = aa_mapworld::MapArchive::from_bytes(
        std::fs::read(
            root.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP"),
        )
        .unwrap(),
    )
    .unwrap();
    let mut attempt = Attempt::load_with_bindings(
        &map,
        &actors,
        settings,
        text,
        Precision::Double53,
        u64::MAX,
        aa_mapworld::restore::RestoreMode::InspectPartial,
        LoadBindings {
            native_load_mode: None,
            inactive_load: Some(inactive_load),
            resources: Some(resources.clone()),
        },
    )
    .unwrap();
    assert!(!attempt.report.runtime_publication_ready);
    let owners: Vec<_> = attempt
        .registry
        .iter()
        .filter_map(|e| {
            let a = e.value.loaded()?.shell.actor.as_ref()?;
            a.components
                .present(ComponentSlot::Organization)
                .then_some((e.id, a.components.present(ComponentSlot::Economy)))
        })
        .collect();
    let owner = owners
        .iter()
        .find_map(|(id, has)| (!has).then_some(*id))
        .expect("real restored Organization without Economy");
    let economy_snapshot = |registry: &aa_actors::registry::IdentityRegistry<RestoredRecord>| {
        registry
            .iter()
            .filter_map(|entry| {
                let actor = entry.value.loaded()?.shell.actor.as_ref()?;
                let RestoredComponent::Economy(e) = actor.components.get(ComponentSlot::Economy)?
                else {
                    return None;
                };
                Some((
                    entry.id,
                    (&**e as *const aa_actors::ground_states::EconomyState) as usize,
                    json!(e),
                ))
            })
            .collect::<Vec<_>>()
    };
    let before_economies = economy_snapshot(&attempt.registry);
    assert!(
        !before_economies.is_empty(),
        "actual other Economy payloads are present"
    );
    let taken = std::mem::replace(
        attempt.registry.value_mut(owner).unwrap(),
        RestoredRecord::Loading,
    );
    let RestoredRecord::Actor(mut entity) = taken else {
        panic!("Actor")
    };
    let actor = entity.shell.actor.as_mut().unwrap();
    let Some(RestoredComponent::Organization(mut organization)) =
        actor.components.remove(ComponentSlot::Organization)
    else {
        panic!("Organization")
    };
    let before_org = json!(&organization);
    let before_words = actor.words.clone();
    let absent_e8 = actor.words.remove(&0xe8);
    let mut catalog = Catalog::new(&actors);
    catalog.native_load_mode = attempt.native_load_mode;
    catalog.settings = Some(attempt.settings.clone());
    catalog.bind_resource_catalog(resources).unwrap();
    catalog
        .bind_inactive_load(attempt.inactive_load.as_ref().unwrap())
        .unwrap();
    for precision in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        aa_runtime::saved_owner_economy::refresh(
            &catalog,
            OrganizationAttachContext {
                inventory: &attempt.inventory,
                owner,
                owner_actor: actor,
                owner_definition: &entity.definition,
                owner_thing_flags: entity.shell.thing.flags,
                owner_thing_word_c: entity.shell.thing.word_c,
                owner_object_byte_4c: entity.shell.object.as_ref().unwrap().byte_4c,
                organization: &mut organization,
                registry: &mut attempt.registry,
                precision,
            },
        )
        .unwrap();
        assert!(!actor.words.contains_key(&0xe8));
        assert!(!actor.components.present(ComponentSlot::Economy));
        assert_eq!(json!(&organization), before_org);
        assert_eq!(economy_snapshot(&attempt.registry), before_economies);
    }
    if let Some(word) = absent_e8 {
        actor.words.insert(0xe8, word);
    }
    assert_eq!(actor.words, before_words);
    println!(
        "{}",
        json!({"owner":owner.raw(),"definition":entity.definition.proof.id,"organization_owners":owners.iter().map(|(id,has)|json!({"actor":id.raw(),"economy_present":has})).collect::<Vec<_>>(),"other_actual_economies_retained":before_economies.len(),"absent_e8_word_not_read":true,"precision_cases":3,"kingdom_flows_initialized_by_test":false,"publication_ready":false})
    );
    actor.components.insert(
        ComponentSlot::Organization,
        RestoredComponent::Organization(organization),
    );
    *attempt.registry.value_mut(owner).unwrap() = RestoredRecord::Actor(entity);
}
