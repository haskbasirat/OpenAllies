use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    native::{TypedValue, scalar_literal},
    sources::{Origin, ResolvedFile, SourceLimits, V14Sources, sha256},
    tgi::Limits,
};
use serde_json::{Value, json};

fn catalog(kind: &str, value: &str) -> ActorCatalog {
    let bytes = format!("[PointFixture]\n{{\n {kind} value = {value}\n}}\n").into_bytes();
    ActorCatalog::from_files(
        vec![ResolvedFile {
            logical_path: "point-prefix-fixture.tgi".into(),
            origin: Origin::V14Loose,
            sha256: sha256(&bytes),
            bytes,
        }],
        Limits::default(),
    )
}

#[test]
fn point2_decimal_prefix_matches_whole_native_parser_and_getter() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-member-deployment/native-point2-decimal-prefix.json"
    ))
    .unwrap();
    let rows = fixture["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 63);
    for row in rows {
        let pc: Precision = serde_json::from_value(row["precision"].clone()).unwrap();
        let input = row["input"]["text"].as_str().unwrap();
        let source = catalog("point2", input);
        let roots = source.resolved_definitions("PointFixture").unwrap();
        let actual = roots[0].field("value", pc);
        if row["input"]["supported"] == true {
            let Some(TypedValue::Point { bits, .. }) = actual.unwrap() else {
                panic!("missing typed point2: {row}");
            };
            assert_eq!(json!(bits), row["output"]["point"], "{row}");
        } else {
            assert!(actual.is_err(), "outside the bounded point2 subset: {row}");
        }
    }
}

#[test]
fn scalar_and_point3_stay_strict_and_point2_rejects_other_extensions() {
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        assert!(scalar_literal("1.5.0", pc).is_err());
        for kind in ["float", "fixed", "point3"] {
            let input = if kind == "point3" {
                "1, 1.5.0, 2"
            } else {
                "1.5.0"
            };
            let source = catalog(kind, input);
            assert!(
                source.resolved_definitions("PointFixture").unwrap()[0]
                    .field("value", pc)
                    .is_err()
            );
        }
        for input in [
            "1.5x,2",
            "1e3,2",
            "foo,1",
            "1 + 2,3",
            "..5,1",
            "1.5.0%,2",
            "1.5.0f,2",
            "1,2,3",
            "NaN,1",
            "Infinity,1",
        ] {
            let source = catalog("point2", input);
            assert!(
                source.resolved_definitions("PointFixture").unwrap()[0]
                    .field("value", pc)
                    .is_err(),
                "{input}"
            );
        }
    }
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 data"]
fn actual_usa_hq_infantry_keeps_authored_point2_decimal_prefix() {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: u64::MAX,
        },
    )
    .unwrap();
    let catalog = ActorCatalog::load_v14(&mut sources).unwrap();
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        let Some(TypedValue::Point { bits, .. }) = catalog
            .native_field(
                "usa_hq_infantry",
                "DenizenComponent.DeploymentInfo.creation_offset",
                pc,
            )
            .unwrap()
        else {
            panic!("actual source point2 absent");
        };
        assert_eq!(bits, [0x4020_0000, 0x3fc0_0000]);
    }
}
