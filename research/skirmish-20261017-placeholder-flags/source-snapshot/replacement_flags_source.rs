//! Exact source-bound Placeholder fixed point, not full4BC7F0 completion.
use aa_rules::{
    actors::ActorCatalog,
    sources::{SourceLimits, V14Sources},
    tgi::Limits,
};
use aa_runtime::{
    catalog::Catalog, combat_source_completion as completion,
    combat_source_receipt::VerifiedStartupReceipt, replacement_flags_source::State,
};
use serde_json::{json, Value};
#[test]
#[ignore = "requires AA_GAME_ROOT actual source-completed Bank"]
fn actual_all_nations_preserve_placeholder_masks_and_source_authority() {
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut source = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: u64::MAX,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut source).unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings =
        Some(aa_runtime::settings::Settings::load_v14(&mut source, Limits::default()).unwrap());
    catalog.native_load_mode = Some(1);
    let receipt = VerifiedStartupReceipt::verify_embedded_reference(&catalog, &mut source).unwrap();
    completion::install(
        &catalog,
        &mut source,
        receipt.context(),
        receipt.csv(),
        receipt.globals(),
    )
    .unwrap();
    completion::compute(&catalog).unwrap();
    let mut bank = catalog.definition_bank.borrow_mut();
    let pc = bank.combat_source().unwrap().source_precision();
    let fields = actors.native_source_requirements(pc).unwrap();
    let state = State::bind(&catalog, &mut bank, &fields).unwrap();
    assert_eq!(state.precision(), pc);
    assert_eq!(state.placeholder_count(), 6);
    assert_eq!(state.verified_pairs(), 30);
    let native: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-placeholder-flags/native-placeholder-flags.json"
    ))
    .unwrap();
    let mut comparisons = 0;
    let mut flags = std::collections::BTreeMap::new();
    for row in native["actual"].as_array().unwrap() {
        if row["pc"].as_str().unwrap() != format!("{pc:?}") {
            continue;
        }
        let id = row["input"]["placeholder"].as_str().unwrap();
        let actual = state.flags(&catalog, &bank, id).unwrap();
        assert_eq!(
            actual,
            u32::try_from(row["native"]["flags"].as_u64().unwrap()).unwrap()
        );
        flags.insert(id.to_owned(), actual);
        comparisons += 1;
    }
    assert_eq!(comparisons, 30);
    assert!(state.flags(&catalog, &bank, "gbr_corps").is_err());
    let other_pc = if pc == aa_numeric::Precision::Single24 {
        aa_numeric::Precision::Double53
    } else {
        aa_numeric::Precision::Single24
    };
    let wrong = actors.native_source_requirements(other_pc).unwrap();
    assert!(State::bind(&catalog, &mut bank, &wrong)
        .err()
        .unwrap()
        .contains("PC differs"));
    let output = json!({"source_pc":format!("{pc:?}"),"placeholders":state.placeholder_count(),"verified_pairs":state.verified_pairs(),"flags":flags,"sources":actors.sources});
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261017-placeholder-flags/actual-fixed-point.json");
    if std::env::var_os("AA_PLACEHOLDER_FLAGS_CAPTURE").is_some() {
        std::fs::write(&path, serde_json::to_vec_pretty(&output).unwrap()).unwrap()
    } else {
        let expected: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(output, expected)
    }
    println!("actual same-Bank {} source:6 Placeholder fixed points/30 Native Nation pairs; wrong-PC refused",format!("{pc:?}"));
}
