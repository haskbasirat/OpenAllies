//! The Catalog lookup indexes (`catalog_lookups`) answer exactly like the
//! linear scans they replace, and an indexed Bound's Translate, VFX Find and
//! canonical-node lookups equal the plain Bound's, on the real catalog.
use super::*;
use aa_rules::sources::{SourceLimits, V14Sources};

fn real() -> ActorCatalog {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"));
    let mut files = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits { max_member_bytes: 64 * 1024 * 1024 },
    )
    .unwrap();
    ActorCatalog::load_v14(&mut files).unwrap()
}
/// The original canonical_node loop.
fn canonical_linear(actors: &ActorCatalog, source: &Provenance) -> Result<Option<Vec<Provenance>>> {
    for tag in aa_rules::actors::native_load::THING_TAGS {
        for node in actors.resolved_definitions(tag).map_err(|e| e.to_string())? {
            if node.sources().first() == Some(source) {
                return Ok(Some(node.sources()));
            }
        }
    }
    Ok(None)
}

#[test]
#[ignore = "requires actual AA_GAME_ROOT archive and v14 definitions"]
fn indexed_lookups_equal_linear_scans_on_the_real_catalog() {
    let actors = real();
    let catalog = Catalog::new(&actors);
    let lookups = catalog.lookups();
    assert!(lookups.is_for(&actors));

    let mut paths: Vec<String> = actors.sources.iter().map(|s| s.path.clone()).collect();
    paths.extend(actors.sources.iter().map(|s| s.path.to_ascii_uppercase()));
    paths.extend(["missing/path.tgi".to_string(), String::new()]);
    for path in &paths {
        assert_eq!(
            lookups.source_position(&actors, path),
            actors.sources.iter().position(|s| s.path.eq_ignore_ascii_case(path)),
            "{path}"
        );
    }
    let mut ids: Vec<String> = actors.actors.iter().map(|a| a.id.clone()).collect();
    ids.extend(actors.actors.iter().map(|a| a.id.to_ascii_uppercase()));
    ids.extend(["missing_actor".to_string(), String::new()]);
    for id in &ids {
        assert_eq!(
            lookups.actor_position(&actors, id),
            actors.actors.iter().position(|a| a.id.eq_ignore_ascii_case(id)),
            "{id}"
        );
    }
    // Another catalog never uses these indexes.
    let other = ActorCatalog::from_files(Vec::new(), aa_rules::tgi::Limits::default());
    assert!(!lookups.is_for(&other));
    assert_eq!(lookups.source_position(&other, &actors.sources[0].path), None);

    for actor in actors.actors.iter().step_by(3) {
        let plain = Bound::new(&actors, Precision::Double53, actor.source.clone()).unwrap();
        let fast = Bound::for_catalog(&catalog, Precision::Double53, actor.source.clone()).unwrap();
        let expected = match canonical_linear(&actors, &actor.source) {
            Ok(Some(sources)) => Ok(sources),
            Ok(None) => Err("FX canonical source root is absent".to_string()),
            Err(error) => Err(error),
        };
        assert_eq!(fast.canonical_node(&actor.source).map(|n| n.sources()), expected, "{}", actor.id);
        assert_eq!(
            format!("{:?}", fast.canonical_node(&actor.source).map(|n| n.sources())),
            format!("{:?}", plain.canonical_node(&actor.source).map(|n| n.sources()))
        );
    }

    // Translate queries: every authored (node, ids_old) pair in several
    // spellings and managers, plus misses, from owners early and late in
    // source order and at two precisions.
    let text = |node: &ResolvedNode<'_>, name: &str| match node.field(name, Precision::Double53) {
        Ok(Some(aa_rules::native::TypedValue::String { value, .. })) => value,
        _ => String::new(),
    };
    let translate = actors.resolved_definitions("Translate").unwrap();
    assert!(translate.len() > 100, "{}", translate.len());
    let mut queries: Vec<(String, String)> = Vec::new();
    for node in translate.iter().step_by(4) {
        let kind = text(node, "node");
        let old = text(node, "ids_old");
        queries.push((kind.clone(), old.clone()));
        queries.push((kind.clone(), old.to_ascii_uppercase()));
        queries.push(("VFX".into(), old.clone()));
        queries.push(("Bone".into(), old));
    }
    queries.extend([("VFX".into(), "missing_vfx".into()), ("Texture".into(), "missing".into()), ("Audio".into(), String::new())]);
    let mut owners: Vec<Provenance> = Vec::new();
    for index in [0, actors.actors.len() / 2, actors.actors.len() - 1] {
        owners.push(actors.actors[index].source.clone());
    }
    owners.push(translate[0].sources()[0].clone());
    let mut missing_owner = owners[0].clone();
    missing_owner.path = "missing/owner.tgi".into();
    owners.push(missing_owner);
    let mut compared = 0;
    for pc in [Precision::Double53, Precision::Single24] {
        for owner in &owners {
            let plain = Bound::new(&actors, pc, owner.clone()).unwrap();
            let fast = Bound::for_catalog(&catalog, pc, owner.clone()).unwrap();
            for (manager, ids) in &queries {
                let expected = plain.translated(manager, ids);
                assert_eq!(fast.translated(manager, ids), expected, "{manager} {ids}");
                // The second call is replayed from the lookup.
                assert_eq!(fast.translated(manager, ids), expected, "{manager} {ids}");
                compared += 1;
            }
            let mut plain = plain;
            let mut fast = fast;
            for (_, ids) in queries.iter().step_by(3) {
                assert_eq!(
                    format!("{:?}", fast.find("VFX", ids)),
                    format!("{:?}", plain.find("VFX", ids)),
                    "{ids}"
                );
            }
        }
    }
    assert!(compared > 500, "{compared}");
}
