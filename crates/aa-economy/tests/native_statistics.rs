//! Replays research/phase8-economy/native_statistics.py: original ordinary
//! (0x58E870) and relation (0x58D950 / 0x58D8C0 + 0x7BA3C0/0x606750) statistics.
use aa_economy::statistics::{RelationStatistics, Statistics};
use aa_economy::{F32Bits, X87Precision};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
struct Document {
    executions: usize,
    fixtures: Vec<Fixture>,
}
#[derive(Deserialize)]
struct Fixture {
    name: String,
    precision: X87Precision,
    ops: Vec<Op>,
    expected: Expected,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Op {
    Score {
        index: usize,
        amount: F32Bits,
    },
    Relation {
        index: usize,
        other: u32,
        amount: F32Bits,
    },
    DefinitionRelation {
        index: usize,
        other: u32,
        amount: F32Bits,
    },
}
#[derive(Deserialize)]
struct Expected {
    production: Vec<F32Bits>,
    scores: Vec<F32Bits>,
    relations: Vec<ExpectedRelation>,
}
#[derive(Deserialize)]
struct ExpectedRelation {
    values: BTreeMap<String, F32Bits>,
    maximum: F32Bits,
    leader: Option<u32>,
}

#[test]
fn statistics_match_original_x86_at_all_precisions() {
    let document: Document =
        serde_json::from_str(include_str!("fixtures/native-statistics-fixtures.json")).unwrap();
    let mut executions = 0;
    for fixture in document.fixtures {
        let mut statistics = Statistics::new(10);
        let mut relations = RelationStatistics::default();
        for op in &fixture.ops {
            match *op {
                Op::Score { index, amount } => statistics
                    .add_score(index, amount, fixture.precision)
                    .unwrap(),
                Op::Relation {
                    index,
                    other,
                    amount,
                } => relations
                    .add(
                        index,
                        (other != 0).then_some(other),
                        amount,
                        fixture.precision,
                    )
                    .unwrap(),
                Op::DefinitionRelation {
                    index,
                    other,
                    amount,
                } => relations
                    .add_by_definition(
                        index,
                        (other != 0).then_some(other),
                        amount,
                        fixture.precision,
                    )
                    .unwrap(),
            }
            executions += 1;
        }
        let label = format!("{} {:?}", fixture.name, fixture.precision);
        assert_eq!(
            statistics.production, fixture.expected.production,
            "{label}"
        );
        assert_eq!(statistics.scores, fixture.expected.scores, "{label}");
        for (entry, expected) in relations.entries.iter().zip(&fixture.expected.relations) {
            let values: BTreeMap<String, F32Bits> = entry
                .values
                .iter()
                .map(|(k, v)| (k.to_string(), *v))
                .collect();
            assert_eq!(values, expected.values, "{label}");
            assert_eq!(entry.maximum, expected.maximum, "{label}");
            assert_eq!(entry.leader, expected.leader, "{label}");
        }
    }
    assert_eq!(executions, document.executions);
}
