//! Replays research/phase8-economy/native_completion_statistics.py: original
//! construction (0x564ED0) and upgrade (0x564E10) completion statistics with
//! 0x58EAB0/0x58EE00 classification, 0x58E870 scores and 0x58D8C0.
use aa_economy::statistics::*;
use aa_economy::{F32Bits, X87Precision};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Deserialize)]
struct Document {
    definition: u32,
    executions: usize,
    fixtures: Vec<Fixture>,
}
#[derive(Deserialize)]
struct Fixture {
    name: String,
    precision: X87Precision,
    steps: Vec<Step>,
}
#[derive(Deserialize)]
struct Step {
    op: Value,
    expected: Value,
}

fn flag(facts: &Value, name: &str) -> bool {
    facts[name].as_bool().unwrap()
}

#[test]
fn completion_statistics_match_original_x86_at_all_precisions() {
    let document: Document = serde_json::from_str(include_str!(
        "fixtures/native-completion-statistics-fixtures.json"
    ))
    .unwrap();
    let mut executions = 0;
    for fixture in document.fixtures {
        let mut statistics = Statistics::new(10);
        let mut relations = RelationStatistics::default();
        for (index, step) in fixture.steps.iter().enumerate() {
            let f = &step.op["facts"];
            let label = format!("{} {:?} step {index}", fixture.name, fixture.precision);
            let facts = StructureFacts {
                structure: flag(f, "structure"),
                virtual_100: flag(f, "virtual_100"),
                construction: flag(f, "construction"),
                data_list_43c: flag(f, "data_list_43c"),
                site_class: f["site_class"].as_u64().unwrap() as u32,
                component_88: flag(f, "component_88"),
                component_90: flag(f, "component_90"),
            };
            let owner = flag(f, "owner");
            let stats = (owner && flag(f, "statistics")).then_some(&mut statistics);
            if step.op["kind"] == "construction" {
                let rel = (owner && flag(f, "relations")).then_some(&mut relations);
                construction_completed(stats, rel, &facts, document.definition, fixture.precision)
                    .expect(&label);
            } else {
                upgrade_completed(stats, &facts, fixture.precision).expect(&label);
            }
            let scores: Vec<F32Bits> =
                serde_json::from_value(step.expected["scores"].clone()).unwrap();
            assert_eq!(statistics.scores, scores, "{label}");
            for (slot, expected) in step.expected["relations"]
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
            {
                let entry = &relations.entries[slot];
                let values: BTreeMap<String, u32> = entry
                    .values
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.0))
                    .collect();
                let expected_values: BTreeMap<String, u32> =
                    serde_json::from_value(expected["values"].clone()).unwrap();
                assert_eq!(values, expected_values, "{label} slot {slot}");
                assert_eq!(
                    entry.maximum.0 as u64,
                    expected["maximum"].as_u64().unwrap(),
                    "{label}"
                );
                assert_eq!(
                    entry.leader.map(u64::from),
                    expected["leader"].as_u64(),
                    "{label}"
                );
            }
            executions += 1;
        }
    }
    assert_eq!(executions, document.executions);
}
