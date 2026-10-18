//! First authored skirmish candidate, fail-fast restoration only. Placeholder
//! armies remain placeholders until real mission preparation is integrated.
use aa_actors::ActorId;
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    sources::{SourceLimits, V14Sources, sha256},
    tgi::Limits,
};
use aa_runtime::{
    actor_world::Source, localization::Localization, restoration::Attempt, settings::Settings,
};
use std::collections::BTreeMap;

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 assets"]
fn original_firefight_restores_fail_fast_with_exact_ownership_and_postload_rng() {
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let bytes = std::fs::read(
        game.join("reconstruction/assets/original/Data/Maps/DeveloperMaps/(3v3)Firefight.RMP"),
    )
    .unwrap();
    assert_eq!(
        sha256(&bytes),
        "c15fd62a581cd36bb8e6131a4ffacba903a62259e0b3c2ae86afee611251e74b"
    );
    let map = aa_mapworld::MapArchive::from_bytes(bytes).unwrap();
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let settings = Settings::load_v14(&mut sources, Limits::default()).unwrap();
    let text = Localization::load_v14(&mut sources, Limits::default()).unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    // This entry is StopOnError. No native load mode is invented: no reached
    // component in this template requires the mode-dependent Mind callback.
    let attempt =
        Attempt::load(&map, &actors, settings, text, Precision::Double53, u64::MAX).unwrap();
    assert!(attempt.report.stopped_before.is_none());
    assert!(
        attempt
            .report
            .events
            .iter()
            .all(|event| event.error.is_none())
    );
    assert!(attempt.report.converted.is_empty() && attempt.report.skipped.is_empty());
    assert_eq!(attempt.inventory.dynamic_things.placements.len(), 2377);
    assert_eq!(attempt.registry.len(), 2386);
    assert_eq!(attempt.rng.state, 3724158463);
    let clock = attempt.clock;
    let mut staged = attempt.into_staged().unwrap();
    let mut owner_counts = BTreeMap::new();
    let mut actor_kinds = BTreeMap::new();
    for entry in staged.actors.identities().iter() {
        let entity = staged.actors.entity(entry.value).unwrap();
        if let Source::Loaded(source) = &entity.source
            && source.placement.class_code == 3
        {
            let owner = staged.actors.ordinary_owner(entry.id).unwrap().unwrap();
            *owner_counts.entry(owner.raw()).or_insert(0usize) += 1;
            let id = &staged
                .actors
                .object(entry.value)
                .unwrap()
                .definition
                .proof
                .id;
            *actor_kinds.entry(id.clone()).or_insert(0usize) += 1;
        }
    }
    assert_eq!(
        owner_counts,
        [
            (1676, 4),
            (1677, 4),
            (1678, 4),
            (1679, 4),
            (1680, 4),
            (1681, 4),
            (1694, 5)
        ]
        .into()
    );
    for id in owner_counts.keys() {
        assert!(staged.actors.key(ActorId::from_raw(*id)).is_some());
    }
    assert_eq!(actor_kinds.get("start_loc"), Some(&6));
    assert_eq!(actor_kinds.get("placeholder_corpsunpacked"), Some(&6));
    assert_eq!(actor_kinds.get("placeholder_bunker"), Some(&12));
    // Original translate.tgi resolves the saved resource_ammo/resource_oil
    // names to these registered ActorData identities.
    assert_eq!(actor_kinds.get("ammo_bunker"), Some(&3));
    assert_eq!(actor_kinds.get("oil_refinery"), Some(&2));
    let report = staged.post_load(&actors, Precision::Double53).unwrap();
    assert_eq!(report.objects, 2386);
    assert_eq!(report.discarded_updates, 0);
    assert_eq!(staged.rng.state, 3627657861);
    assert_eq!(staged.clock, clock);
    for owner in staged.updatables.slots().iter().flatten() {
        assert!(staged.owners.target(*owner).is_ok());
    }
    assert!(!staged.report.runtime_publication_ready);
    assert!(staged.init_for_loaded_map().is_err());
    println!(
        "{}",
        serde_json::json!({"map":"DeveloperMaps/(3v3)Firefight.RMP", "mode":"StopOnError", "registered":2386, "owner_counts":owner_counts, "actor_kinds":actor_kinds, "post_load_objects":report.objects, "rng_after_post_load":staged.rng.state, "battle_ready":false})
    );
}
