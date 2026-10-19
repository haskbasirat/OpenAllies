//! The memoized per-definition queries return exactly what the uncached code
//! returns, for every registered definition of the real installed catalog,
//! whichever spelling or lookup filled the memo first. Errors are compared
//! too (they are never cached, so their text keeps the caller's spelling).
use super::*;
use crate::{
    native::{COMPONENTS, THING_FLAGS},
    query_memo::BYPASS,
    sources::{SourceLimits, V14Sources},
};
use aa_numeric::Precision;
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
fn uncached<T>(f: impl FnOnce() -> T) -> T {
    BYPASS.with(|b| b.set(true));
    let value = f();
    BYPASS.with(|b| b.set(false));
    value
}
fn layers(node: crate::native::Result<ResolvedNode<'_>>) -> String {
    match node {
        Ok(node) => format!(
            "{:?}",
            node.layers.iter().map(|l| *l as *const Definition as usize).collect::<Vec<_>>()
        ),
        Err(error) => format!("{error:?}"),
    }
}

#[test]
#[ignore = "requires locally installed original Data.rwd and accepted v14 runtime"]
fn memoized_queries_equal_uncached_queries_on_the_real_catalog() {
    let mut sources = open();
    let (catalog, _) = native_load::load(&mut sources).unwrap();

    // The source index lists exactly the roots a linear scan would find.
    let mut grouped: BTreeMap<(String, String, usize, usize), Vec<usize>> = BTreeMap::new();
    for (index, node) in catalog.resolver.roots.iter().enumerate() {
        let s = &node.source;
        grouped.entry((s.path.clone(), s.sha256.clone(), s.span.start, s.span.end)).or_default().push(index);
    }
    for node in &catalog.resolver.roots {
        let s = &node.source;
        let expected = &grouped[&(s.path.clone(), s.sha256.clone(), s.span.start, s.span.end)];
        assert_eq!(catalog.roots_with_source(s), expected.as_slice());
        assert_eq!(catalog.root_with_source(s), expected.first().copied());
        assert_eq!(
            catalog.root_with_source(s),
            catalog.resolver.roots.iter().position(|n| n.source == *s)
        );
    }
    let absent = Provenance { path: "absent.tgi".into(), sha256: String::new(), span: crate::tgi::Span { start: 0, end: 0 } };
    assert!(catalog.roots_with_source(&absent).is_empty());

    let mut fields: Vec<String> = ["category", "IDS", "name", "Size.height", "Size.blocking_shape",
        "Size.soft_shape", "MoverComponent.speed_max", "BodyComponent.health",
        "DetectionComponent.remote_detection_range", "View.resource_prefix", "Missing.field", "missing"]
        .iter().map(|s| s.to_string()).collect();
    fields.extend(THING_FLAGS.iter().map(|(name, _)| format!("Flags.{name}")));
    let mut nodes: Vec<Vec<&str>> = COMPONENTS.iter().map(|(name, _)| vec![*name]).collect();
    nodes.extend([vec!["View"], vec!["View", "AspectNormal"], vec!["Missing"], vec![]]);
    let mut ids: Vec<String> = catalog.actors.iter().map(|a| a.id.clone()).collect();
    ids.push("missing_actor_id".into());
    let pcs = [Precision::Single24, Precision::Double53, Precision::Extended64];
    let mut checked = 0usize;
    // Pass 0 fills the memo through upper-case Find; pass 1 then reads it
    // through every spelling and lookup. Both passes compare every result.
    for pass in 0..2 {
        for id in &ids {
            let spellings = if pass == 0 { vec![id.to_ascii_uppercase()] } else { vec![id.clone(), id.to_ascii_uppercase()] };
            for spelling in &spellings {
                for lookup in [ThingLookup::Find, ThingLookup::Registered] {
                    let thing = ThingRef { ids: spelling, lookup };
                    let same = |label: &str, cached: String, fresh: String| {
                        assert_eq!(cached, fresh, "{label} {spelling} {lookup:?}");
                    };
                    same("category", format!("{:?}", catalog.native_category(thing)),
                        uncached(|| format!("{:?}", catalog.native_category(thing))));
                    same("actor", layers(catalog.resolved_actor(thing)), uncached(|| layers(catalog.resolved_actor(thing))));
                    for pc in pcs {
                        same("flags", format!("{:?}", catalog.native_final_flags(thing, pc)),
                            uncached(|| format!("{:?}", catalog.native_final_flags(thing, pc))));
                        same("object", format!("{:?}", catalog.native_object_definition(thing, pc)),
                            uncached(|| format!("{:?}", catalog.native_object_definition(thing, pc))));
                        same("creation", format!("{:?}", catalog.native_creation_definition(thing, pc)),
                            uncached(|| format!("{:?}", catalog.native_creation_definition(thing, pc))));
                        for is_actor in [false, true] {
                            same("base", format!("{:?}", catalog.native_base_flags(thing, is_actor, pc)),
                                uncached(|| format!("{:?}", catalog.native_base_flags(thing, is_actor, pc))));
                        }
                        for path in &fields {
                            same(path, format!("{:?}", catalog.native_field(thing, path, pc)),
                                uncached(|| format!("{:?}", catalog.native_field(thing, path, pc))));
                        }
                    }
                    for path in &nodes {
                        same("node", format!("{:?}", catalog.native_node_sources(thing, path)),
                            uncached(|| format!("{:?}", catalog.native_node_sources(thing, path))));
                    }
                    checked += 1;
                }
            }
        }
    }
    assert!(checked > 6000, "{checked}");
    // ResolvedNode::field on roots and their View/component children.
    let names = ["IDS", "category", "name", "height", "resource_prefix", "aspects_as_variations",
        "speed_max", "health", "cutscene", "missing_field", "blocking_shape", "model"];
    let mut nodes_checked = 0usize;
    for pass in 0..2 {
        for tag in crate::actors::native_load::THING_TAGS.iter().chain(["Translate", "Audio", "SVars"].iter()) {
            for root in catalog.resolved_definitions(tag).unwrap_or_default() {
                let mut nodes = vec![root.clone()];
                for child in ["View", "Size", "MoverComponent", "BodyComponent"] {
                    if let Ok(Some(node)) = root.child(child) {
                        nodes.push(node);
                    }
                }
                for node in &nodes {
                    for name in names {
                        for pc in pcs {
                            let name = if pass == 0 { name.to_string() } else { name.to_ascii_lowercase() };
                            assert_eq!(
                                format!("{:?}", node.field(&name, pc)),
                                uncached(|| format!("{:?}", node.field(&name, pc))),
                                "{tag} {name}"
                            );
                            nodes_checked += 1;
                        }
                    }
                }
            }
        }
    }
    assert!(nodes_checked > 100_000, "{nodes_checked}");
    // A clone starts with an empty memo and still agrees.
    let clone = catalog.clone();
    for id in ids.iter().take(200) {
        for pc in pcs {
            assert_eq!(format!("{:?}", clone.native_final_flags(id.as_str(), pc)),
                format!("{:?}", catalog.native_final_flags(id.as_str(), pc)));
        }
    }
}
