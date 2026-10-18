use super::*;

#[test]
#[ignore = "requires actual AA_GAME_ROOT source archive and accepted v14 source completion"]
fn actual_current_audio_class_source_ten_and_same_moved_bank() {
    use crate::{
        combat_source_completion as complete, combat_source_receipt::VerifiedStartupReceipt,
        settings::Settings,
    };
    use aa_rules::{actors::ActorCatalog, sources::SourceLimits, tgi::Limits};
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: u64::MAX,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut files).unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings = Some(Settings::load_v14(&mut files, Limits::default()).unwrap());
    let receipt = VerifiedStartupReceipt::verify_embedded_reference(&catalog, &mut files).unwrap();
    complete::install(
        &catalog,
        &mut files,
        receipt.context(),
        receipt.csv(),
        receipt.globals(),
    )
    .unwrap();
    assert!(
        FreshCacheVariableScope::from_sources(&catalog, &mut files, NativeOverrides::Unbound)
            .is_err()
    );
    //An installed diagnostic/incomplete Bank is not the actual source context.
    assert!(
        FreshCacheVariableScope::from_sources(
            &catalog,
            &mut files,
            NativeOverrides::ControlledNone
        )
        .is_err()
    );
    complete::compute(&catalog).unwrap();
    let scope = FreshCacheVariableScope::from_sources(
        &catalog,
        &mut files,
        NativeOverrides::ControlledNone,
    )
    .unwrap();
    let bank = catalog.definition_bank.take();
    assert!(catalog.definition_bank.borrow().combat_source().is_err());
    //The same source State lives in the moved Bank, not Catalog's fresh empty
    //placeholder. Production Resources passes this actual Staged/World Bank.
    let class = AudioClass::from_current(&catalog, &bank, &mut files, &scope).unwrap();
    assert_eq!(class.name(), "KKC_AudioResource");
    assert_eq!((class.ram_weight(), class.vram_weight()), (10, 10));
    assert_eq!(class.declarations().len(), 1);
    let declaration = &class.declarations()[0];
    assert_eq!(declaration.path, "UVars.tgi");
    let source = files.read(&declaration.path).unwrap();
    assert_eq!(source.sha256, declaration.sha256);
    assert!(
        std::str::from_utf8(&source.bytes)
            .unwrap()
            .contains("int ResourceCacheAudioRAMWeight = 10")
    );
    assert!(!class.source_files().is_empty());
    assert!(AudioClass::from_current(&catalog, &Bank::default(), &mut files, &scope).is_err());
}
