use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    native::finalize_detection_component_mask,
    sources::{Origin, ResolvedFile, SourceLimits, V14Sources, sha256},
    tgi::Limits,
};
use serde_json::Value;

const DETECTION: u32 = 1 << 22;
const ALL_PC: [Precision; 3] = [
    Precision::Single24,
    Precision::Double53,
    Precision::Extended64,
];

fn catalog(text: &str) -> ActorCatalog {
    ActorCatalog::from_files(
        vec![ResolvedFile {
            logical_path: "detection-mask.tgi".into(),
            origin: Origin::V14Loose,
            sha256: sha256(text.as_bytes()),
            bytes: text.as_bytes().to_vec(),
        }],
        Limits::default(),
    )
}

#[test]
fn original_detection_mask_fragment_matches_all_2331_native_probes() {
    let proof: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-detection-mask/native-detection-mask.json"
    ))
    .unwrap();
    let rows = proof["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 2331);
    let mut absent = 0;
    for row in rows {
        let before = row["mask_before"].as_u64().unwrap() as u32;
        let got = if row["component_present"].as_bool().unwrap() {
            assert_eq!(row["getter_calls"], 2);
            finalize_detection_component_mask(
                before | DETECTION,
                row["control_bits"].as_u64().unwrap() as u32,
                row["remote_bits"].as_u64().unwrap() as u32,
            )
        } else {
            absent += 1;
            assert_eq!(row["getter_calls"], 0);
            before
        };
        assert_eq!(got as u64, row["mask_after"], "{row}");
    }
    assert_eq!(absent, 27);
}

#[test]
fn fresh_detection_mask_includes_equality_before_later_defender_control_range() {
    for remote in ["-10", "-0", "0", "0.00001", "10"] {
        let actors = catalog(&format!(
            "[Thing]\n{{\nIDS=probe\nstring category=ACTOR\n[DetectionComponent]\n{{\nfixed detection_range=7\nfixed remote_detection_range={remote}\n}}\n[DefenderComponent]\n{{\nfixed control_range=5\n}}\n}}\n"
        ));
        for pc in ALL_PC {
            let got = actors.native_component_mask_at("probe", None, pc).unwrap();
            let expected = (1 << 20) | if remote.starts_with('-') || remote == "0" { 0 } else { DETECTION };
            assert_eq!(got.bits, expected, "remote {remote}, {pc:?}");
            assert!(!got.component_sources["DetectionComponent"].is_empty());
            assert_eq!(
                actors.native_field("probe", "DefenderComponent.control_range", pc)
                    .unwrap().unwrap().fixed_bits().unwrap(),
                5.0f32.to_bits()
            );
        }
        assert_eq!(
            actors.native_component_mask("probe", None).unwrap().bits,
            actors.native_component_mask_at("probe", None, Precision::Double53).unwrap().bits
        );
    }
}

#[test]
fn absent_detection_skips_remote_read_but_present_invalid_fixed_values_are_errors() {
    let absent = catalog("[Thing]\n{\nIDS=plain\nstring category=ACTOR\n}\n");
    for pc in ALL_PC {
        let got = absent.native_component_mask_at("plain", None, pc).unwrap();
        assert_eq!(got.bits, 0);
        assert!(!got.component_sources.contains_key("DetectionComponent"));
    }
    for remote in ["", "fixed remote_detection_range", "float remote_detection_range=0", "string remote_detection_range=0"] {
        let actors = catalog(&format!(
            "[Thing]\n{{\nIDS=bad\nstring category=ACTOR\n[DetectionComponent]\n{{\nfixed detection_range=7\n{remote}\n}}\n}}\n"
        ));
        for pc in ALL_PC {
            assert!(actors.native_component_mask_at("bad", None, pc).is_err(), "{remote}");
        }
    }
}

#[test]
#[ignore = "requires original Data.rwd and runtime-expanded-v14 via AA_GAME_ROOT"]
fn actual_mine_masks_match_observed_windows_source_return_at_all_pc() {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut source = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits { max_member_bytes: 64 * 1024 * 1024 },
    ).unwrap();
    let actors = ActorCatalog::load_v14(&mut source).unwrap();
    for id in ["minefield", "beach_mine_pole"] {
        for pc in ALL_PC {
            let got = actors.native_component_mask_at(id, None, pc).unwrap();
            // Actual Windows/v14 capture03 source return: flags+174, PC53.
            assert_eq!(got.bits, 1_090_145, "{id}, {pc:?}");
            assert_eq!(got.bits & DETECTION, 0);
            assert!(!got.component_sources["DetectionComponent"].is_empty());
            assert_eq!(
                actors.native_field(id, "DetectionComponent.remote_detection_range", pc)
                    .unwrap().unwrap().fixed_bits().unwrap(), 0
            );
            assert_eq!(
                actors.native_field(id, "DefenderComponent.control_range", pc)
                    .unwrap().unwrap().fixed_bits().unwrap(), 5.0f32.to_bits()
            );
        }
    }
}
