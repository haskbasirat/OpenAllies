use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    sources::{Origin, ResolvedFile, SourceLimits, V14Sources, sha256},
    tgi::Limits,
};
fn file(text: &str) -> ResolvedFile {
    ResolvedFile {
        logical_path: "source.tgi".into(),
        origin: Origin::V14Loose,
        sha256: sha256(text.as_bytes()),
        bytes: text.as_bytes().to_vec(),
    }
}
#[test]
fn construction_upgrade_census_includes_unresolved_templates_and_rejects_omissions() {
    let cat = ActorCatalog::from_files(
        vec![file("[Template Base]{[list Upgrade]{string ids}}")],
        Limits::default(),
    );
    let census = cat.construction_upgrade_census().unwrap();
    assert_eq!(census.declarations.len(), 1);
    assert!(census.explicit_instances.is_empty());
    let cat = ActorCatalog::from_files(
        vec![file(
            "[Template Base template=Unresolved]{[Upgrade]{ids=anything}}",
        )],
        Limits::default(),
    );
    assert_eq!(
        cat.construction_upgrade_census()
            .unwrap()
            .explicit_instances
            .len(),
        1
    );
    let mut bad = file("[Upgrade]{ids=anything}");
    bad.sha256 = "00".repeat(32);
    let cat = ActorCatalog::from_files(vec![bad], Limits::default());
    assert!(cat.construction_upgrade_census().is_err());
    let cat = ActorCatalog::from_files(vec![file("value=\"unfinished")], Limits::default());
    assert_eq!(
        cat.construction_upgrade_census()
            .unwrap()
            .opaque_sources_without_upgrade_bytes
            .len(),
        1
    );
    let cat = ActorCatalog::from_files(vec![file("value=\"unfinished Upgrade")], Limits::default());
    assert!(cat.construction_upgrade_census().is_err());
    let cat = ActorCatalog::from_files(vec![file("\0value=none")], Limits::default());
    assert!(cat.construction_upgrade_census().is_err());
}
#[test]
#[ignore = "requires original/v14 files via AA_GAME_ROOT"]
fn original_construction_costs_have_source_proven_empty_predecessor_graph() {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let catalog = ActorCatalog::load_v14(&mut sources).unwrap();
    let census = catalog.construction_upgrade_census().unwrap();
    assert!(census.explicit_instances.is_empty());
    assert!(!census.declarations.is_empty());
    let mut rows = Vec::new();
    for id in ["rus_corps", "usa_corps", "ger_corps"] {
        let input = catalog
            .native_construction_inputs(id, Precision::Double53)
            .unwrap()
            .unwrap();
        assert!(input.has_structure);
        let resources = input.resources.as_ref().unwrap();
        assert_eq!(resources.resource_ids.len(), 7);
        assert_eq!(resources.cost_bits.len(), 7);
        rows.push(serde_json::json!({"id":id,"cost_bits":resources.cost_bits,"resource_ids":resources.resource_ids,"sources":input.sources}));
    }
    println!("{}", serde_json::json!({"census":census,"inputs":rows}));
}
