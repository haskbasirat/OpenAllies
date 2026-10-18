use aa_mapworld::restore::RestoreDefinitions;
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    sources::{SourceLimits, V14Sources},
    tgi::Limits,
};
use aa_runtime::{
    catalog::Catalog, localization::Localization, restoration::Attempt, settings::Settings,
};
#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 assets"]
fn real_infantry_definitions_and_tutorial_restore_share_owned_state() {
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
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
    let catalog = Catalog::new(&actors);
    for id in [
        "usa_infantry",
        "gbr_infantry",
        "ger_infantry",
        "jpn_infantry",
        "rus_infantry",
    ] {
        let bound = catalog
            .actor_bindings(&id.to_ascii_uppercase(), Precision::Double53)
            .unwrap();
        assert_eq!(bound.definition.proof.id, id);
        assert_eq!(bound.character.as_ref().unwrap().actor_id, id);
        assert_eq!(
            bound.performer.as_ref().unwrap().completed_ability_count(),
            2
        );
        assert_eq!(bound.definition.component_mask.unwrap(), 0x40036d);
        assert!(bound.definition.ground.body.is_some());
        assert!(bound.definition.ground.mover.is_some());
        assert_eq!(bound.definition.ground.properties_domain_count, Some(2));
        assert_eq!(bound.definition.ground.resource_domain_count, Some(7));
        assert!(std::sync::Arc::ptr_eq(
            &bound,
            &catalog
                .actor_bindings(&id.to_ascii_uppercase(), Precision::Double53)
                .unwrap()
        ));
    }
    let resource = catalog
        .resolve_named_definition(
            "Resource",
            &"money".encode_utf16().collect::<Vec<_>>(),
            Precision::Double53,
        )
        .unwrap()
        .unwrap();
    assert!(resource.index.is_some());
    let regiment = catalog
        .actor_bindings("regiment_usa_infantry", Precision::Double53)
        .unwrap();
    let org = regiment.definition.ground.organization.as_ref().unwrap();
    assert_eq!(org.layout.parameters.len(), 6);
    assert_eq!(org.layout.slots, 9);
    let commander = catalog
        .commander_constructor("zhukov_tutorial", Precision::Double53)
        .unwrap();
    assert_eq!(commander.abilities.len(), 1);
    assert!(
        catalog
            .definition_bank
            .borrow()
            .require_usable(commander.abilities[0].definition)
            .is_err()
    );
    let again = catalog
        .commander_constructor("ZHUKOV_TUTORIAL", Precision::Double53)
        .unwrap();
    assert_eq!(again.id, commander.id);
    assert_eq!(
        again.abilities[0].definition,
        commander.abilities[0].definition
    );
    for name in ["tutorial01", "tutorial02"] {
        let map = aa_mapworld::MapArchive::from_bytes(
            std::fs::read(game.join(format!(
                "reconstruction/assets/original/Data/Maps/Campaign/{name}.RMP"
            )))
            .unwrap(),
        )
        .unwrap();
        let attempt = Attempt::load(
            &map,
            &actors,
            settings.clone(),
            text.clone(),
            Precision::Double53,
            u64::MAX,
        )
        .unwrap();
        println!(
            "{}",
            serde_json::json!({"map":name,"clock":attempt.clock,"rng":attempt.rng,"registered":attempt.registry.len(),"table_count":attempt.updatables.len(),"bound_updatables":attempt.updatables.slots().iter().filter(|s|s.is_some()).count(),"report":attempt.report})
        );
        assert!(!attempt.registry.is_empty());
        // Map load decodes TRIGGERS with the LanguageIDS AVar (AVars_Locale.tgi).
        assert_eq!(settings.language_ids().unwrap().0, "English");
        assert!(!attempt.triggers.as_ref().unwrap().triggers.is_empty());
        // Every restored Kingdom has its GKingdom::Load economy part.
        let kingdoms = attempt
            .registry
            .iter()
            .filter(|e| matches!(e.value, aa_mapworld::restore::RestoredRecord::Kingdom(_)))
            .count();
        assert_eq!(attempt.kingdom_economy.len(), kingdoms);
        assert!(kingdoms > 0);
        if let Some(index) = attempt.report.events.iter().position(|e| e.error.is_some()) {
            assert_eq!(
                index + 1,
                attempt.report.events.len(),
                "must stop at first Load failure"
            );
            assert!(!attempt.report.runtime_publication_ready);
        }
        let expected = attempt
            .updatables
            .slots()
            .iter()
            .filter(|s| s.is_some())
            .count();
        let clock = attempt.clock;
        let rng = attempt.rng.state;
        let staged = attempt.into_staged().unwrap();
        assert_eq!(staged.clock, clock);
        assert_eq!(staged.rng.state, rng);
        assert_eq!(
            staged
                .updatables
                .slots()
                .iter()
                .filter(|s| s.is_some())
                .count(),
            expected
        );
        for owner in staged.updatables.slots().iter().flatten() {
            assert!(staged.owners.target(*owner).is_ok());
            assert!(
                !staged.queues.has_owner(*owner),
                "saved scheduling follows completed object Load"
            );
        }
        // The queue Load and PostLoad pass require every record section.
        let stopped = staged.report.stopped_before.is_some();
        let mut staged = staged;
        match staged.post_load(&actors, Precision::Double53) {
            Ok(report) => {
                assert!(!stopped);
                assert_eq!(report.objects, staged.actors.identities().len());
                assert!(matches!(
                    staged.init_for_loaded_map(),
                    Err(aa_runtime::restoration::Error::Unported(_))
                ));
            }
            Err(aa_runtime::restoration::Error::PostLoadState(_)) => assert!(stopped),
            Err(e) => panic!("{name}: {e}"),
        }
    }
}
