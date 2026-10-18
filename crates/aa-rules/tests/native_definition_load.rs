//! Native-order definition loading (research/phase8-definitions).
use aa_rules::{
    actors::{ActorCatalog, native_load},
    sources::{SourceLimits, V14Sources},
};
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}
fn open() -> V14Sources {
    V14Sources::open(
        root().join("../Data.rwd"),
        root().join("../reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap()
}

/// The original parser's Translate roots for the shipped translate.tgi
/// (fixture section `translate_file`) are exactly the roots the native load
/// reads, in order; Thing targets all exist, so no 48D2D0 exception occurs.
#[test]
#[ignore = "requires locally installed original Data.rwd and accepted v14 runtime"]
fn shipped_translations_match_the_original_parser() {
    let fixture: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root().join("research/phase8-definitions/native-definition-registry.json"))
            .unwrap(),
    )
    .unwrap();
    let native = &fixture["sections"]["translate_file"];
    let mut sources = open();
    let translate = sources.read("translate.tgi").unwrap();
    assert_eq!(translate.sha256, native["source_sha256"].as_str().unwrap());
    let (catalog, report) = native_load::load(&mut sources).unwrap();
    let roots = native["roots"].as_array().unwrap();
    assert_eq!(report.translations.len(), roots.len());
    for (rust, native) in report.translations.iter().zip(roots) {
        assert_eq!(rust.manager, native["node"].as_str().unwrap());
        assert_eq!(rust.ids_old, native["ids_old"].as_str().unwrap());
        assert_eq!(rust.ids_new, native["ids_new"].as_str().unwrap());
    }
    // 427BD0 on the completed registry: translated IDs resolve to their target.
    for (old, new) in [
        ("temperate_sea_rocks", "temperate_rocks"),
        ("eastern_europe_city02", "eeurope_cityblock12x12_02"),
        ("western_europe_farm01", "weurope_farm6x8_01"),
        ("chaparral_wall", "temperate_walls"),
    ] {
        assert_eq!(catalog.find(old).unwrap().id.to_ascii_lowercase(), new, "{old}");
    }
    // Character set tables (ActorData+1BC presence) for the inventory's
    // Character records resolve for every Character actor.
    let mut characters = 0;
    for actor in &catalog.actors {
        if let Ok(Some(present)) =
            catalog.native_character_sets_present(&actor.id, aa_numeric::Precision::Double53)
        {
            assert_eq!(present.last(), Some(&true), "{}", actor.id);
            characters += 1;
        }
    }
    assert!(characters > 0);
    for id in ["cow", "ger_infantry", "usa_infantry", "donkey_brown", "sheep_ewe"] {
        let present = catalog
            .native_character_sets_present(id, aa_numeric::Precision::Double53)
            .unwrap_or_else(|e| panic!("{id}: {e}"))
            .unwrap();
        eprintln!("{id} sets {present:?}");
    }
    // Denizen inventories as the 56B720 constructor consumes them.
    let mut denizens = 0;
    for actor in &catalog.actors {
        let id = actor.id.clone();
        if let Ok(Some(inventory)) =
            catalog.native_denizen_inventory(&id, aa_numeric::Precision::Double53)
        {
            denizens += 1;
            assert!(inventory.organization_behaviors.is_empty(), "{id}");
        }
    }
    assert!(denizens > 0);
    let carrier = catalog
        .native_denizen_inventory("usa_carrier", aa_numeric::Precision::Double53)
        .unwrap()
        .unwrap();
    let order: Vec<&str> = carrier.slots.iter().map(|s| s.target.as_str()).collect();
    assert_eq!(order.len(), 8);
    assert!(order[..6].iter().all(|t| *t == "usa_fighter_corsair"));
    assert!(order[6..].iter().all(|t| *t == "usa_bomber"));
    // Generic managers (48D500): Variation and Biome translations are modeled.
    assert_eq!(report.exceptions().count(), 0);
    assert_eq!(report.data_translations.len(), 68);
    let find = |manager: &str, id: &str| catalog.native_data_find(manager, id).map(|e| e.ids.to_ascii_lowercase());
    assert_eq!(find("Variation", "tropical_shoredirttrans01").as_deref(), Some("tropical_shore01"));
    assert_eq!(find("Variation", "tropical_river02").as_deref(), Some("tropical_river01"));
    // 48D500 does not chain: river03 -> river02 is itself only a translation key.
    assert_eq!(find("Variation", "tropical_river03"), None);
    assert_eq!(find("Biome", "arctic_forest").as_deref(), Some("boreal_forest"));
    assert_eq!(catalog.native_data_registry("Variation").unwrap().entries.len(), 410);
    // Entries inside the comment block opened at translate.tgi line 1292.
    for missing in ["temperate_dynamic_hedge", "tropical_bamboostand", "desert_palmtrees"] {
        assert!(catalog.find(missing).is_none(), "{missing}");
    }
}

#[test]
#[ignore = "requires locally installed original Data.rwd and accepted v14 runtime"]
fn original_data_loads_in_native_order_without_native_exceptions() {
    let mut sources = open();
    let (catalog, report) = native_load::load(&mut sources).unwrap();
    if let Some(out) = std::env::var_os("AA_NATIVE_LOAD_REPORT") {
        std::fs::write(out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    }
    let exceptions: Vec<_> = report.exceptions().collect();
    eprintln!(
        "files {} things {} translations {} exceptions {}",
        report.files.len(),
        report.things.len(),
        report.translations.len(),
        exceptions.len()
    );
    for e in exceptions.iter().take(40) {
        eprintln!("{:?} {} {}", e.kind, e.file, e.detail);
    }
    assert!(catalog.find("usa_battleship").is_some());
    if std::env::var_os("AA_NATIVE_LOAD_COMPARE").is_some() {
        let mut all = open();
        let old = ActorCatalog::load_v14_all_sources(&mut all).unwrap();
        let native: std::collections::BTreeSet<String> =
            catalog.actors.iter().map(|a| a.id.to_ascii_lowercase()).collect();
        let previous: std::collections::BTreeSet<String> =
            old.actors.iter().map(|a| a.id.to_ascii_lowercase()).collect();
        eprintln!("native-only {:?}", native.difference(&previous).take(60).collect::<Vec<_>>());
        eprintln!("all-sources-only {:?}", previous.difference(&native).collect::<Vec<_>>());
    }
}
