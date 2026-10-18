use super::*;
use serde_json::Value;
#[test]
fn original_v14_containment_masks_and_required_prefixes() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-placeholder-flags/native-placeholder-flags.json"
    ))
    .unwrap();
    let mut count = 0;
    for row in fixture["rows"]
        .as_array()
        .unwrap()
        .iter()
        .chain(fixture["actual"].as_array().unwrap())
    {
        let c = &row["input"];
        let n = &row["native"];
        let words = |key: &str| -> [u32; 6] {
            c[key]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as u32)
                .collect::<Vec<_>>()
                .try_into()
                .unwrap()
        };
        let original = c["subject_flags"].as_u64().unwrap() as u32;
        let result = fixed_pair(
            original,
            c["target_flags"].as_u64().unwrap() as u32,
            &words("subject_shape"),
            &words("target_shape"),
            c["virtual_true"].as_bool().unwrap(),
        );
        if n["outcome"] == "complete" && n["events"].as_array().unwrap().is_empty() {
            assert_eq!(result.unwrap(), n["flags"].as_u64().unwrap() as u32);
        } else if n["outcome"] == "containment" {
            assert!(result.unwrap_err().contains("containment"));
            assert_eq!(n["flags"].as_u64().unwrap() as u32, original);
        } else {
            assert!(result.unwrap_err().contains("mutable"));
            if c["stop_warning"].as_bool().unwrap() {
                assert_eq!(n["flags"].as_u64().unwrap() as u32, original)
            }
            //Full native write rows show WHY mutation cannot be certified by
            //the unchanged-only factory. They do not assert missing Rust writes.
        }
        count += 1;
    }
    assert_eq!(count, 294);
}

#[test]
#[ignore = "requires actual AA_GAME_ROOT and source-completed Bank"]
fn actual_catalog_cache_checks_warm_foreign_pc_and_bank_loans() {
    use crate::{
        combat_source_completion as completion, combat_source_receipt::VerifiedStartupReceipt,
    };
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: u64::MAX,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut files).unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings =
        Some(crate::settings::Settings::load_v14(&mut files, Limits::default()).unwrap());
    catalog.native_load_mode = Some(1);
    let proof = VerifiedStartupReceipt::verify_embedded_reference(&catalog, &mut files).unwrap();
    completion::install(
        &catalog,
        &mut files,
        proof.context(),
        proof.csv(),
        proof.globals(),
    )
    .unwrap();
    completion::compute(&catalog).unwrap();
    let pc = catalog
        .definition_bank
        .borrow()
        .combat_source()
        .unwrap()
        .source_precision();
    let fields = actors.native_source_requirements(pc).unwrap();
    let native: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-placeholder-flags/native-placeholder-flags.json"
    ))
    .unwrap();
    for row in native["actual"].as_array().unwrap() {
        if row["pc"].as_str().unwrap() != format!("{pc:?}") {
            continue;
        }
        let id = row["input"]["placeholder"].as_str().unwrap();
        assert_eq!(
            catalog.verified_placeholder_flags(id, &fields).unwrap(),
            row["native"]["flags"].as_u64().unwrap() as u32
        );
    }
    let other = if pc == Precision::Single24 {
        Precision::Double53
    } else {
        Precision::Single24
    };
    let wrong = actors.native_source_requirements(other).unwrap();
    assert!(catalog
        .verified_placeholder_flags("placeholder_airfield", &wrong)
        .is_err());
    let bank = catalog.definition_bank.borrow_mut();
    assert!(catalog
        .verified_placeholder_flags("placeholder_airfield", &fields)
        .is_err());
    drop(bank);
    assert_eq!(
        catalog
            .verified_placeholder_flags("placeholder_airfield", &fields)
            .unwrap(),
        0x20816500
    );
    assert!(catalog
        .verified_placeholder_flags("gbr_corps", &fields)
        .is_err());
    //Staging moves the ONE Bank and its immutable source cache together.
    //Catalog's old slot stays empty; no reinstall or clone occurs.
    let moved = std::mem::take(&mut *catalog.definition_bank.borrow_mut());
    assert!(catalog.definition_bank.borrow().combat_source().is_err());
    assert!(catalog
        .verified_placeholder_flags("placeholder_airfield", &fields)
        .is_err());
    assert_eq!(
        catalog
            .verified_placeholder_flags_in_bank(&moved, "placeholder_airfield", &fields)
            .unwrap(),
        0x20816500
    );
    assert!(catalog
        .verified_placeholder_flags_in_bank(&moved, "placeholder_airfield", &wrong)
        .is_err());
    assert!(catalog
        .verified_placeholder_flags_in_bank(&moved, "gbr_corps", &fields)
        .is_err());
}
