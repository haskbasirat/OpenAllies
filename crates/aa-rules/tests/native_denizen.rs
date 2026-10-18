use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    denizen::finalize_guards,
    sources::{Origin, ResolvedFile, SourceLimits, V14Sources, sha256},
    tgi::Limits,
};
#[test]
fn native_denizen_guard_defaults_validation_and_inventory_empty_branch_match() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/phase7-definitions/native-denizen-guards.json"
    ))
    .unwrap();
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 180);
    for row in fixture["cases"].as_array().unwrap() {
        let out = finalize_guards(
            row["guard_bits"].as_u64().unwrap() as u32,
            row["operational_bits"].as_u64().unwrap() as u32,
            row["inventory_present"].as_bool().unwrap(),
            serde_json::from_value(row["precision"].clone()).unwrap(),
        );
        if row["output_bits"].is_null() {
            assert!(out.is_err());
        } else {
            assert_eq!(
                serde_json::to_value(out.unwrap()).unwrap(),
                row["output_bits"]
            );
        }
    }
    let source = "[Thing]{\nIDS=test\n[DenizenComponent]{\nfixed guard_range=10\nfixed guard_operational_range=0\n[Inventory]{\nstring actor_ids=test\nint count=0\n}\n}\n}\n";
    let cat = ActorCatalog::from_files(
        vec![ResolvedFile {
            logical_path: "source.tgi".into(),
            origin: Origin::V14Loose,
            sha256: sha256(source.as_bytes()),
            bytes: source.as_bytes().to_vec(),
        }],
        Limits::default(),
    );
    assert_eq!(
        cat.native_denizen_guards("test", Precision::Double53)
            .unwrap()
            .unwrap()
            .finalized_bits,
        [10.0f32.to_bits(), 12.0f32.to_bits()],
        "Inventory node presence, not the authored actor quantity, controls the native guard reset"
    );
}
#[test]
#[ignore = "requires original/v14 files via AA_GAME_ROOT"]
fn original_rus_corps_detection_uses_proven_empty_inventory_guard_range() {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut source = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let cat = ActorCatalog::load_v14(&mut source).unwrap();
    let definition = cat
        .native_ground_definition("rus_corps", Precision::Double53)
        .unwrap();
    let guards = definition.denizen_guards.as_ref().unwrap();
    assert!(guards.inventory.is_empty());
    assert_eq!(guards.finalized_bits, [0, 12.0f32.to_bits()]);
    let detection = definition.detection.as_ref().unwrap();
    assert!(f32::from_bits(detection.finalized_range_bits) >= 12.0);
    println!("{}", serde_json::to_string(&definition).unwrap());
}
