use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    sources::{Origin, ResolvedFile, SourceLimits, V14Sources, sha256},
    tgi::Limits,
};
fn catalog(source: &str) -> ActorCatalog {
    ActorCatalog::from_files(
        vec![ResolvedFile {
            logical_path: "source.tgi".into(),
            origin: Origin::V14Loose,
            sha256: sha256(source.as_bytes()),
            bytes: source.as_bytes().to_vec(),
        }],
        Limits::default(),
    )
}
#[test]
fn ordinary_collision_source_flags_require_complete_negative_reference_evidence() {
    let base = "[Thing]{\nIDS=unit\nstring category=ACTOR\n[WallComponent]{bool gate=true}\n}\n";
    let value = catalog(base)
        .native_collision_definition("UNIT", Precision::Double53)
        .unwrap();
    assert_eq!(value.id, "unit");
    assert!(value.gate);
    assert!(!value.settlement_wall);
    assert!(value.required_object.is_none());
    let src = format!("{base}[Template Unused]{{string gate_placeholder_ids=unit}}\n");
    assert!(
        catalog(&src)
            .native_collision_definition("unit", Precision::Double53)
            .is_err(),
        "even a possible unused-template match needs positive side-effect proof"
    );
    let src = format!("{base}[Template Other]{{[Upgrade]{{ids=elsewhere}}}}\n");
    assert!(
        catalog(&src)
            .native_collision_definition("unit", Precision::Double53)
            .is_err(),
        "unresolved predecessor graph cannot silently become absent"
    );
}
#[test]
#[ignore = "requires original/v14 files via AA_GAME_ROOT"]
fn original_infantry_and_corps_collision_definitions_have_proven_source_flags() {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut source = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let catalog = ActorCatalog::load_v14(&mut source).unwrap();
    let mut rows = Vec::new();
    for id in [
        "usa_infantry",
        "gbr_infantry",
        "ger_infantry",
        "rus_infantry",
        "jpn_infantry",
        "rus_corps",
    ] {
        let input = catalog
            .native_collision_definition(id, Precision::Double53)
            .unwrap();
        assert!(!input.settlement_wall);
        assert!(!input.gate);
        assert!(input.required_object.is_none());
        assert!(input.wall_reference_census.references.is_empty());
        // The original index chain loads 1,647 data files and never reads the
        // other 200 TGI members (research/phase8-definitions/README.txt).
        assert_eq!(input.wall_reference_census.sources.len(), 1647);
        rows.push(serde_json::json!({"id":input.id,"required_object":input.required_object,"gate":input.gate,"settlement_wall":input.settlement_wall,"sources":input.sources,"wall_reference_count":input.wall_reference_census.references.len(),"source_count":input.wall_reference_census.sources.len(),"opaque_sources":input.wall_reference_census.opaque_sources_without_field_names}));
    }
    println!("{}", serde_json::to_string(&rows).unwrap());
}
