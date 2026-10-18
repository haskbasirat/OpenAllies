use aa_rules::{
    actors::{ActorCatalog, FieldResolution},
    sources::{SourceLimits, V14Sources},
    tgi::{Document, Limits},
};
use std::path::PathBuf;

#[test]
#[ignore = "requires the hash-pinned local original Data.rwd and accepted v14 loose files"]
fn original_corpus_and_authored_actor_values() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut source = V14Sources::open(
        root.join("../Data.rwd"),
        root.join("../reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let mut parsed = 0;
    let mut errors = Vec::new();
    for row in source
        .catalog()
        .into_iter()
        .filter(|r| r.logical_path.to_ascii_lowercase().ends_with(".tgi"))
    {
        let f = source.read(&row.logical_path).unwrap();
        match Document::parse_native(f.bytes.clone(), Limits::default()) {
            Ok(d) => {
                assert_eq!(d.original_bytes(), f.bytes);
                parsed += 1;
            }
            Err(_) => errors.push(row.logical_path),
        }
    }
    // Phase8: the native delayed-body and value-quote rules (5BD080 oracle,
    // native_tgi_parser.rs) accept the two files the original game loads.
    assert_eq!(parsed, 1847);
    assert!(errors.is_empty(), "{errors:?}");
    let actors = ActorCatalog::load_v14(&mut source).unwrap();
    // Native data index chain (48DB60 -> 5AB590, research/phase8-definitions/
    // README.txt): 1,647 loaded files and the native Thing registry.
    assert_eq!(actors.actors.len(), 1959);
    for (id, health, defense, speed) in [
        ("usa_infantry", "200", "1", "2.5"),
        ("usa_tank_medium", "400", "6", "3.5"),
    ] {
        let a = actors.find(id).unwrap();
        for (key, expected) in [
            ("category", "UNIT"),
            ("BodyComponent.health", health),
            ("BodyComponent.defense", defense),
            ("MoverComponent.speed_max", speed),
        ] {
            let field = a.field(key).unwrap();
            assert_eq!(field.raw_text(), Some(expected), "{id} {key}");
            let p = field.source().unwrap();
            let f = source.read(&p.path).unwrap();
            assert_eq!(p.sha256, f.sha256);
            assert!(f.bytes[p.span.start..p.span.end].ends_with(expected.as_bytes()));
        }
    }
    let corps = actors.find("gbr_corps").unwrap();
    assert_eq!(
        corps.field("BodyComponent.health").unwrap().raw_text(),
        Some("3500")
    );
    assert_eq!(
        corps.field("View.resource_prefix").unwrap().raw_text(),
        Some("GBRCorps/GBRCorps")
    );
    assert_eq!(
        corps.field("MoverComponent.speed_max"),
        Some(&FieldResolution::Missing)
    );
    // The former "ambiguous" eastern_europe_city_center02_large is resolved by
    // the native first-declaration-wins Thing registry (4BCA00), see
    // research/phase8-definitions/README.txt.
}
