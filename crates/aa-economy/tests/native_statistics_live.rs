use aa_economy::statistics::Statistics;
use aa_economy::statistics_live::*;
use aa_economy::{Catalog, F32Bits, ResourceDefinition, ResourceKind, X87Precision};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-world-start/native-statistics-live.json"
    ))
    .unwrap()
}
fn words(v: &Value) -> Vec<F32Bits> {
    serde_json::from_value(v.clone()).unwrap()
}

#[test]
fn tracking_matches_original_and_v14_at_three_precisions() {
    let doc = fixture();
    let mut count = 0;
    for f in doc["track"].as_array().unwrap() {
        let c = &f["case"];
        let pc: X87Precision = serde_json::from_value(f["precision"].clone()).unwrap();
        let facts: TrackingFacts = serde_json::from_value(c["facts"].clone()).unwrap();
        let mut s = Statistics::new(0);
        s.scores = words(&c["scores"]);
        let mut field = c["field"].as_u64().unwrap() as u32;
        let started = c["started"].as_bool().unwrap();
        let mut calls = vec![];
        if c["op"] == "track" {
            track_actor(&mut s, &mut field, &facts, started, pc, || {
                let source = c["virtual_100"].as_array().unwrap();
                let x = source[calls.len().min(source.len() - 1)].as_bool().unwrap();
                calls.push(x);
                Ok(x)
            })
            .unwrap();
        } else {
            remove_actor(&mut s, &mut field, &facts, started, pc).unwrap();
        }
        assert_eq!(
            json!({"field":field,"scores":s.scores,"virtual_calls":calls}),
            f["expected"],
            "{} {:?}",
            c["name"],
            pc
        );
        count += 1;
    }
    assert_eq!(count, 330);
}

fn resources(c: &Value) -> Catalog {
    Catalog {
        resources: c["resources"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, r)| ResourceDefinition {
                id: format!("r{i}"),
                kind: match r["kind"].as_u64().unwrap() {
                    0 => ResourceKind::Stockpiled,
                    1 => ResourceKind::Upkeep,
                    2 => ResourceKind::LimitedConsumed,
                    _ => unreachable!(),
                },
                limited_provider: r["limited_provider"].as_bool().unwrap(),
                purchase_cost: vec![],
            })
            .collect(),
    }
}

#[test]
fn full_snapshots_match_original_and_v14_at_three_precisions() {
    let doc = fixture();
    let mut count = 0;
    for f in doc["snapshot"].as_array().unwrap() {
        let c = &f["case"];
        let pc: X87Precision = serde_json::from_value(f["precision"].clone()).unwrap();
        let s = Statistics {
            production: words(&c["production"]),
            scores: words(&c["scores"]),
        };
        let derived = DerivedCatalog {
            rows: serde_json::from_value(c["rows"].clone()).unwrap(),
        };
        let actual = snapshot(
            &s,
            &resources(c),
            &words(&c["provided"]),
            &words(&c["consumed"]),
            c["count"].as_i64().unwrap() as i32,
            c["relation_identity"].as_u64().map(|i| i as u32),
            &derived,
            pc,
        )
        .unwrap();
        let v = json!({"production":actual.statistics.production,"scores":actual.statistics.scores,"resource_levels":actual.resource_levels,"kingdom_count":actual.kingdom_count,"derived":actual.derived,"relation_identity":actual.relation_identity});
        assert_eq!(v, f["expected"], "{} {:?}", c["name"], pc);
        count += 1;
    }
    assert_eq!(count, 132);
}

#[test]
fn snapshots_extend_resource_and_derived_storage_without_a_ten_entry_cap() {
    let n = 17;
    let catalog = Catalog {
        resources: (0..n)
            .map(|i| ResourceDefinition {
                id: i.to_string(),
                kind: ResourceKind::Stockpiled,
                limited_provider: false,
                purchase_cost: vec![],
            })
            .collect(),
    };
    let original = Statistics::new(n);
    let rows = (0..19)
        .map(|i| {
            vec![WeightedTerm {
                index: if i == 0 { 62 + n } else { 62 + 2 * n + i - 1 },
                weight: F32Bits::from_f32(2.),
            }]
        })
        .collect();
    let s = snapshot(
        &original,
        &catalog,
        &vec![F32Bits::from_f32(3.); n],
        &vec![F32Bits::from_f32(1.); n],
        1,
        None,
        &DerivedCatalog { rows },
        X87Precision::Double53,
    )
    .unwrap();
    assert_eq!(s.resource_levels.len(), 17);
    assert_eq!(s.derived.len(), 19);
    assert_eq!(s.derived[18], F32Bits::from_f32(1048576.));
    assert_eq!(original, Statistics::new(n));
}

#[test]
fn tracking_retains_earlier_stores_on_missing_component_and_virtual_failure() {
    let mut stats = Statistics::new(0);
    let mut field = 50;
    assert!(
        untrack_actor(
            &mut stats,
            &mut field,
            &TrackingFacts::default(),
            X87Precision::Double53
        )
        .is_err()
    );
    assert_eq!(field, UNTRACKED);
    assert_eq!(stats.scores[50], F32Bits::ZERO);
    let facts = TrackingFacts {
        component_7c: true,
        structure_94: true,
        ..Default::default()
    };
    assert!(
        track_actor(
            &mut stats,
            &mut field,
            &facts,
            true,
            X87Precision::Double53,
            || Err(aa_economy::EconomyError::World(
                "virtual+100 unavailable".into()
            ))
        )
        .is_err()
    );
    assert_eq!(field, 47);
    assert_eq!(stats.scores[47], F32Bits::from_f32(1.));
}
