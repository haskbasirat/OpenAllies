use aa_rules::{
    actors::{ActorCatalog, FieldResolution},
    sources::{Origin, ResolvedFile, sha256},
    tgi::Limits,
};
fn catalog(source: &str) -> ActorCatalog {
    ActorCatalog::from_files(
        vec![ResolvedFile {
            logical_path: "test.tgi".into(),
            origin: Origin::V14Loose,
            sha256: sha256(source.as_bytes()),
            bytes: source.as_bytes().to_vec(),
        }],
        Limits::default(),
    )
}
#[test]
fn derived_fields_win_and_provenance_points_to_authored_value() {
    let source = "[Template Body]\n{\n float health = 100\n float defense = 1\n}\n[Template Base]\n{\n category = UNIT\n [optional BodyComponent Template=Body]\n}\n[Template Derived Inherit=Base]\n{\n [BodyComponent]\n health = 200\n}\n[Thing Template=Derived]\n{\n IDS = sample\n}\n";
    let c = catalog(source);
    let a = c.find("SAMPLE").unwrap();
    assert_eq!(
        a.field("BodyComponent.health").unwrap().raw_text(),
        Some("200")
    );
    assert_eq!(
        a.field("BodyComponent.defense").unwrap().raw_text(),
        Some("1")
    );
    assert_eq!(a.field("category").unwrap().raw_text(), Some("UNIT"));
    let p = a.field("BodyComponent.health").unwrap().source().unwrap();
    assert_eq!(&source[p.span.start..p.span.end], "health = 200");
    assert_eq!(p.sha256, sha256(source.as_bytes()));
}
#[test]
fn absent_optional_component_is_not_created() {
    let c = catalog(
        "[Template Base]{\n [optional BodyComponent]\n health=100\n}\n[Thing Template=Base]{\n IDS=empty\n}\n",
    );
    assert_eq!(
        c.find("empty").unwrap().field("BodyComponent.health"),
        Some(&FieldResolution::Missing)
    );
}
#[test]
fn expressions_stay_raw_and_list_or_duplicate_resolution_is_explicit() {
    let c = catalog(
        "[Template Base]{\n [BodyComponent]\n health = $balance * 2\n [list MoverComponent]\n speed_max=3\n}\n[Thing Template=Base]{\n IDS=sample\n}\n",
    );
    let a = c.find("sample").unwrap();
    assert_eq!(
        a.field("BodyComponent.health").unwrap().raw_text(),
        Some("$balance * 2")
    );
    assert!(matches!(
        a.field("MoverComponent.speed_max"),
        Some(FieldResolution::Unsupported { .. })
    ));
}
#[test]
fn missing_base_cycle_and_duplicate_ids_do_not_abort_catalog() {
    let c = catalog(
        "[Thing Template=Missing]{\n IDS=missing\n}\n[Template Loop Template=Loop]\n[Thing Template=Loop]{\n IDS=cycle\n}\n[Thing]{\n IDS=duplicate\n category=UNIT\n}\n[Thing]{\n IDS=duplicate\n category=BUILDING\n}\n",
    );
    for id in ["missing", "cycle", "duplicate"] {
        assert!(
            matches!(
                c.find(id).unwrap().field("category"),
                Some(FieldResolution::Unsupported { .. })
            ),
            "{id}"
        );
    }
}
#[test]
fn forged_source_digest_is_not_used_for_provenance() {
    let c = ActorCatalog::from_files(
        vec![ResolvedFile {
            logical_path: "bad.tgi".into(),
            origin: Origin::V14Loose,
            sha256: "0".repeat(64),
            bytes: b"[Thing]{\n IDS=fake\n}".to_vec(),
        }],
        Limits::default(),
    );
    assert!(c.find("fake").is_none());
    assert!(c.diagnostics[0].message.contains("SHA256"));
}
