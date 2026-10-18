//!Real archive/source and manager allocation tests. No Resource decoder/Task
//!completion or application readiness is substituted by these class checks.
use super::*;

fn files() -> V14Sources {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        aa_rules::sources::SourceLimits {
            max_member_bytes: u64::MAX,
        },
    )
    .unwrap()
}

#[test]
#[ignore = "requires actual AA_GAME_ROOT source archive and accepted v14 source completion"]
fn actual_sound_class_is_lazy_after_entry_and_reuses_same_manager_allocation() {
    use crate::{
        combat_source_completion as complete, combat_source_receipt::VerifiedStartupReceipt,
    };
    let mut input = files();
    let actors = aa_rules::actors::ActorCatalog::load_v14(&mut input).unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings = Some(
        crate::settings::Settings::load_v14(&mut input, aa_rules::tgi::Limits::default()).unwrap(),
    );
    let receipt = VerifiedStartupReceipt::verify_embedded_reference(&catalog, &mut input).unwrap();
    complete::install(
        &catalog,
        &mut input,
        receipt.context(),
        receipt.csv(),
        receipt.globals(),
    )
    .unwrap();
    complete::compute(&catalog).unwrap();
    let ids = "Audio/Sounds/General/CallOuts/CityBuildingSelect.wav"
        .encode_utf16()
        .collect::<Vec<_>>();
    let key = resource_map_cache::normalized_key(&ids).unwrap();
    //A standalone manager with no current variable owner must retain the real
    //new Sound/Task prefix, not manufacture a default successful class policy.
    let mut missing = Application::from_sources(files());
    let bank = catalog.definition_bank.borrow();
    let error = missing
        .resources()
        .sound_59b4b0(&catalog, &bank, &ids, true)
        .unwrap_err();
    assert!(error.contains("cache-variable policy"));
    {
        let manager = missing.manager.lock().unwrap();
        assert!(manager.audio_class.is_none());
        let entry = manager.resources.get(&key).unwrap();
        assert_eq!(entry.references14, 0);
        assert_eq!(entry.flags8, 0);
        assert_eq!(entry.task.state14().unwrap(), 0);
        assert_eq!(entry.sound.as_ref().unwrap().streaming41, 0);
    }
    drop(bank);
    assert!(
        missing
            .install_fresh_cache_variable_scope(&catalog, NativeOverrides::ControlledNone)
            .is_err()
    );

    let mut application = Application::from_sources(input);
    application
        .install_fresh_cache_variable_scope(&catalog, NativeOverrides::ControlledNone)
        .unwrap();
    assert!(application.manager.lock().unwrap().audio_class.is_none());
    let bank = catalog.definition_bank.take();
    let first = application
        .resources()
        .sound_59b4b0(&catalog, &bank, &ids, true)
        .unwrap();
    let class_allocation;
    {
        let manager = application.manager.lock().unwrap();
        let class = manager.audio_class.as_ref().unwrap();
        assert_eq!(class.name(), "KKC_AudioResource");
        assert_eq!((class.ram_weight(), class.vram_weight()), (10, 10));
        class_allocation = std::ptr::from_ref(class);
        let entry = manager.resources.get(&key).unwrap();
        assert_eq!(entry.references14, 0);
        assert_eq!(entry.flags8, 0);
        assert_eq!(entry.task.state14().unwrap(), 0);
        assert_eq!(entry.sound.as_ref().unwrap().streaming41, 1);
    }
    let uppercase = String::from_utf16(&ids)
        .unwrap()
        .to_ascii_uppercase()
        .replace('/', "\\")
        .encode_utf16()
        .collect::<Vec<_>>();
    let second = application
        .resources()
        .sound_59b4b0(&catalog, &bank, &uppercase, false)
        .unwrap();
    assert!(first.same_identity(&second));
    let manager = application.manager.lock().unwrap();
    assert_eq!(
        class_allocation,
        std::ptr::from_ref(manager.audio_class.as_ref().unwrap())
    );
    let entry = manager.resources.get(&key).unwrap();
    assert_eq!(entry.references14, 1);
    assert_eq!(entry.flags8 & 1, 0);
    assert_eq!(entry.task.state14().unwrap(), 0);
    assert_eq!(entry.sound.as_ref().unwrap().streaming41, 1);
}
