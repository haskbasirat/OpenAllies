//! Read-only audit of authored visibility against all recovered map placements.
use aa_rules::{
    actors::{ActorCatalog, FieldResolution},
    sources::{SourceLimits, V14Sources, sha256},
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut source = V14Sources::open(
        root.join("../Data.rwd"),
        root.join("../reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )?;
    let catalog = ActorCatalog::load_v14(&mut source)?;
    let index: Value = serde_json::from_slice(&std::fs::read(
        root.join("../reconstruction/game/data/maps/index.json"),
    )?)?;
    let mut maps = BTreeMap::new();
    let mut hidden_ids = BTreeMap::new();
    for row in index.as_array().ok_or("map index is not an array")? {
        let id = row["id"].as_str().ok_or("map id missing")?;
        let bytes =
            std::fs::read(root.join(format!("../reconstruction/game/data/map-actors/{id}.json")))?;
        let placements: Value = serde_json::from_slice(&bytes)?;
        let mut hidden = 0usize;
        let mut unresolved = BTreeMap::<String, usize>::new();
        let mut hidden_types = BTreeMap::<String, usize>::new();
        let actors = placements["actors"].as_array().ok_or("actors missing")?;
        for instance in actors {
            let thing = instance["thingId"].as_str().ok_or("Thing ID missing")?;
            match catalog.find(thing).and_then(|a| a.field("Flags.invisible")) {
                Some(FieldResolution::Value { raw, source }) if raw == b"true" => {
                    hidden += 1;
                    *hidden_types.entry(thing.into()).or_default() += 1;
                    hidden_ids.insert(
                        thing.to_owned(),
                        json!({"field_source":source, "definition":catalog.find(thing)}),
                    );
                }
                Some(FieldResolution::Value { raw, .. }) if raw == b"false" => {}
                _ => {
                    *unresolved.entry(thing.into()).or_default() += 1;
                }
            }
        }
        maps.insert(id.to_owned(), json!({"placements_sha256":sha256(&bytes), "source_records":actors.len(), "authored_hidden":hidden, "hidden_types":hidden_types,"unresolved":unresolved}));
    }
    let values = catalog
        .actors
        .iter()
        .flat_map(|a| a.fields.values())
        .filter(|f| matches!(f, FieldResolution::Value { .. }))
        .count();
    let resolved_visibility = catalog
        .actors
        .iter()
        .filter(|a| {
            a.field("Flags.invisible")
                .is_some_and(|f| matches!(f.raw_text(), Some("true" | "false")))
        })
        .count();
    let out = json!({"schema":1,"actor_definitions":catalog.actors.len(),"resolved_field_values":values,"resolved_visibility_fields":resolved_visibility,"source_diagnostics":catalog.diagnostics,"hidden_definitions":hidden_ids,"maps":maps,
        "scope":"Source-bound initial authored Flags.invisible only. No runtime visibility/fog/controller evaluation; unresolved values preserve geometry."});
    let bytes = serde_json::to_vec_pretty(&out)?;
    let destination = root.join("research/phase5-scenegraph/visibility-audit.json");
    std::fs::write(&destination, &bytes)?;
    println!(
        "{}",
        json!({"maps":out["maps"].as_object().unwrap().len(),"actor_definitions":catalog.actors.len(),"resolved_fields":values,"visibility_fields":resolved_visibility,"report_sha256":sha256(&bytes),"output":destination})
    );
    Ok(())
}
