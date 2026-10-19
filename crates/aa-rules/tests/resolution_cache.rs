//! The cached lookups return exactly what the full scans return, on the
//! real installed catalog (these exist only to make loading fast).
use aa_numeric::Precision;
use aa_rules::{
    actors::native_load::{self, THING_TAGS},
    native::TypedValue,
    sources::{SourceLimits, V14Sources},
};
use std::path::PathBuf;

fn open() -> V14Sources {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    V14Sources::open(
        root.join("../Data.rwd"),
        root.join("../reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits { max_member_bytes: 64 * 1024 * 1024 },
    )
    .unwrap()
}

#[test]
#[ignore = "requires locally installed original Data.rwd and accepted v14 runtime"]
fn cached_lookups_equal_full_scans_on_the_real_catalog() {
    let mut sources = open();
    assert_eq!(sources.catalog(), sources.catalog_entries());
    let (catalog, _) = native_load::load(&mut sources).unwrap();
    let pc = Precision::Double53;
    // The first call computes and caches; later calls and a clone replay it.
    let fresh = catalog.clone();
    for tag in THING_TAGS.iter().chain(["Banner", "Biome", "Missing"].iter()) {
        let computed: Vec<_> = fresh.resolved_definitions(tag).unwrap().iter().map(|n| n.sources()).collect();
        let cached: Vec<_> = fresh.resolved_definitions(tag).unwrap().iter().map(|n| n.sources()).collect();
        let cloned = fresh.clone();
        let replayed: Vec<_> = cloned.resolved_definitions(tag).unwrap().iter().map(|n| n.sources()).collect();
        assert_eq!(computed, cached, "{tag}");
        assert_eq!(computed, replayed, "{tag}");
    }
    let mut checked = 0;
    for tag in THING_TAGS {
        let all = catalog.resolved_definitions(tag).unwrap();
        for node in &all {
            let source = &node.sources()[0];
            let full: Vec<_> = all.iter().filter(|n| n.sources().first() == Some(source)).collect();
            let fast = catalog.resolved_definitions_at(tag, source).unwrap();
            assert_eq!(fast.len(), full.len(), "{tag} {source:?}");
            for (a, b) in fast.iter().zip(&full) {
                assert_eq!(a.sources(), b.sources());
            }
            // Second call is served from the cache and still agrees.
            assert_eq!(catalog.resolved_definitions_at(tag, source).unwrap().len(), full.len());
            checked += 1;
        }
    }
    assert!(checked > 1900, "{checked}");
    let mut names = Vec::new();
    for tag in THING_TAGS {
        for node in catalog.resolved_definitions(tag).unwrap() {
            if let Some(TypedValue::String { value, .. }) = node.field("IDS", pc).unwrap() {
                names.push(value);
            }
        }
    }
    for name in names.iter().chain(["missing_ids".to_owned()].iter()) {
        let full = names.iter().filter(|v| v.eq_ignore_ascii_case(name)).count();
        assert_eq!(catalog.thing_ids_declarations(name, pc).unwrap(), full, "{name}");
        assert_eq!(catalog.thing_ids_declarations(&name.to_ascii_uppercase(), pc).unwrap(), full);
    }
    let own = catalog.registered("chaparral_wall").unwrap();
    assert_eq!(own.source.path.replace('\\', "/"), "Doodads/Chaparral/chaparral_wall.tgi");
    assert_eq!(catalog.find("chaparral_wall").unwrap().id, "temperate_walls");
}
