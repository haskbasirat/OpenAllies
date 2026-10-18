//! Decodes the TRIGGERS chunk of every original map (research/phase8-triggers/
//! chunks, SHA256-pinned to the RMP archives by extract_chunks.py) and checks
//! the element census, native validity (541900) and dispatch coverage.
use aa_triggers::decode::{Localization, SavedManager};
use std::collections::BTreeMap;

const BUDGET: usize = 64 << 20;

fn chunk(file: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/phase8-triggers/chunks")
        .join(file);
    std::fs::read(path).unwrap()
}

#[test]
fn every_original_mission_trigger_decodes_and_dispatch_is_explicit() {
    let index: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/phase8-triggers/chunks/index.json"
    ))
    .unwrap();
    let mut usage: BTreeMap<u32, (u64, u64)> = BTreeMap::new();
    let mut campaign = 0;
    let mut v4 = 0;
    let mut invalid = vec![];
    for entry in index.as_array().unwrap() {
        let bytes = chunk(entry["file"].as_str().unwrap());
        assert_eq!(bytes.len() as u64, entry["length"].as_u64().unwrap());
        let decoded = SavedManager::decode(&bytes, Localization::External, BUDGET);
        if entry["manager_version"] == 4 {
            // Developer maps: native 54D3D0 version-4 branch (native-triggers.json).
            v4 += 1;
        }
        let saved = decoded.unwrap_or_else(|e| panic!("{}: {e}", entry["map"]));
        assert_eq!(saved.triggers.len() as u64, entry["triggers"].as_u64().unwrap());
        let mut counts: BTreeMap<String, u64> = BTreeMap::new();
        for t in &saved.triggers {
            for (label, list) in [("events", &t.events), ("conditions", &t.conditions), ("actions", &t.actions)] {
                for e in list {
                    *counts.entry(format!("{label}/{}", e.name)).or_default() += 1;
                    let (kind, _) = aa_triggers::element_schema(e.opcode).unwrap();
                    assert_eq!(
                        kind,
                        match label { "events" => 0, "conditions" => 1, _ => 2 },
                        "{} trigger {} element {} in {label}",
                        entry["map"], t.id, e.name
                    );
                    if !aa_triggers::program::valid(e) {
                        invalid.push(format!("{} trigger {} {}", entry["map"], t.id, e.name));
                    }
                    if entry["campaign"] == true {
                        let u = usage.entry(e.opcode).or_default();
                        u.1 += 1;
                    }
                }
            }
        }
        let census: BTreeMap<String, u64> = entry["elements"]
            .as_object()
            .unwrap()
            .iter()
            .flat_map(|(label, m)| {
                m.as_object()
                    .unwrap()
                    .iter()
                    .map(move |(n, c)| (format!("{label}/{n}"), c.as_u64().unwrap()))
            })
            .collect();
        assert_eq!(counts, census, "{}", entry["map"]);
        if entry["campaign"] == true {
            campaign += 1;
            let mut seen: Vec<u32> = saved
                .triggers
                .iter()
                .flat_map(|t| t.events.iter().chain(&t.conditions).chain(&t.actions))
                .map(|e| e.opcode)
                .collect();
            seen.sort();
            seen.dedup();
            for op in seen {
                usage.entry(op).or_default().0 += 1;
            }
        }
    }
    assert_eq!(campaign, 26);
    assert_eq!(v4, 32);
    // Every element instance in the shipped data passes native 541900 validity.
    assert!(invalid.is_empty(), "{invalid:?}");
    let mut unsupported = vec![];
    let (mut covered, mut total) = (0u64, 0u64);
    for (op, (missions, instances)) in &usage {
        total += instances;
        if aa_triggers::native_module(*op).is_some() {
            covered += instances;
        } else {
            unsupported.push(format!("{op}: {missions} missions, {instances} uses"));
        }
    }
    println!("campaign element instances dispatched by ported modules: {covered}/{total}");
    println!("unsupported (explicit error): {unsupported:#?}");
    // Every element instance of the 26 campaign missions dispatches to a ported module.
    assert_eq!(covered, total, "{unsupported:?}");
    // coverage.json (research/phase8-triggers/coverage.py) agrees with the dispatch tables.
    let coverage: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/phase8-triggers/coverage.json"
    ))
    .unwrap();
    for row in coverage["elements"].as_array().unwrap() {
        let op = row["opcode"].as_u64().unwrap() as u32;
        assert_eq!(
            row["rust_module"].as_str(),
            aa_triggers::native_module(op),
            "coverage.json is stale for opcode {op}"
        );
        if row["status"] == "proven" {
            assert!(row["oracle_cases"].as_u64().unwrap() > 0);
        }
    }
}
