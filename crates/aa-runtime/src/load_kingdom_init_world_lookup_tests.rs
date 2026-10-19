//! The indexed Kingdom Init source lookups equal the original linear scans
//! for every registered definition of the real catalog.
use super::*;
use aa_rules::sources::{SourceLimits, V14Sources};

fn real() -> aa_rules::actors::ActorCatalog {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"));
    let mut files = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits { max_member_bytes: 64 * 1024 * 1024 },
    )
    .unwrap();
    aa_rules::actors::ActorCatalog::load_v14(&mut files).unwrap()
}
/// The original source_node body.
fn source_node_linear(catalog: &Catalog<'_>, canonical: &str) -> Result<Vec<aa_rules::resources::Provenance>> {
    catalog.require_source_owner()?;
    let entry = catalog
        .actors
        .actors
        .iter()
        .find(|a| a.id.eq_ignore_ascii_case(canonical))
        .ok_or("canonical Kingdom recruit definition is missing")?;
    let mut found = None;
    for tag in aa_rules::actors::native_load::THING_TAGS {
        for root in catalog.actors.resolved_definitions(tag).map_err(|e| e.to_string())? {
            if root.sources().first() == Some(&entry.source) {
                if found.is_some() {
                    return Err("canonical recruit source root is ambiguous".into());
                }
                found = Some(root)
            }
        }
    }
    found
        .map(|n| n.sources())
        .ok_or("canonical recruit source provenance is unavailable".into())
}

#[test]
#[ignore = "requires actual AA_GAME_ROOT archive and v14 definitions"]
fn indexed_source_node_equals_linear_scan_on_the_real_catalog() {
    let actors = real();
    let catalog = Catalog::new(&actors);
    let mut ids: Vec<String> = actors.actors.iter().map(|a| a.id.clone()).collect();
    ids.extend(actors.actors.iter().step_by(7).map(|a| a.id.to_ascii_uppercase()));
    ids.push("missing_definition".into());
    for id in &ids {
        assert_eq!(source_node(&catalog, id).map(|n| n.sources()), source_node_linear(&catalog, id), "{id}");
    }
    // first_source is sources().first(), borrowed.
    for tag in aa_rules::actors::native_load::THING_TAGS {
        for root in actors.resolved_definitions(tag).unwrap() {
            assert_eq!(root.first_source(), root.sources().first());
        }
    }
}
