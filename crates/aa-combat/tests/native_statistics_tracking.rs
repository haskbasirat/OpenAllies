//! 0x58F010/0x58EFE0/0x58EA80 against
//! research/phase8-combat/native-statistics-tracking-fixtures.json, on
//! aa-economy's Statistics object.
use aa_combat::statistics_tracking::{self, TrackFacts};
use aa_economy::{
    F32Bits,
    statistics::{Statistics, StructureFacts},
};
use aa_numeric::Precision;
use serde_json::Value;

fn b(v: &Value) -> bool {
    v.as_bool().unwrap()
}

#[test]
fn statistics_tracking_matches_native() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-statistics-tracking-fixtures.json"
    ))
    .unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 1320);
    let (mut tracked, mut changed) = (0, 0);
    for case in cases {
        let f = &case["facts"];
        if case["entry"] == "predicate" {
            let got = statistics_tracking::classified(
                b(&f["component_80"]),
                b(&f["component_d8"]),
                b(&f["component_a4"]),
            );
            assert_eq!(u64::from(got), case["result"].as_u64().unwrap(), "{case}");
            continue;
        }
        let p: Precision = serde_json::from_value(case["precision"].clone()).unwrap();
        let facts = TrackFacts {
            component_7c: b(&f["component_7c"]),
            component_d8: b(&f["component_d8"]),
            structure: StructureFacts {
                structure: b(&f["structure"]),
                virtual_100: b(&f["virtual_100"]),
                construction: b(&f["construction"]),
                data_list_43c: b(&f["data_list_43c"]),
                site_class: f["site_class"].as_u64().unwrap() as u32,
                component_88: b(&f["component_88"]),
                component_90: b(&f["component_90"]),
            },
            definition_4d0: b(&f["definition_4d0"]),
            component_98: f["component_98"].as_i64().map(|v| v as i32),
            component_d4: b(&f["component_d4"]),
        };
        let mut stats = Statistics::new(10);
        for (index, bits) in case["scores"].as_object().unwrap() {
            stats.scores[index.parse::<usize>().unwrap()] = F32Bits(bits.as_u64().unwrap() as u32);
        }
        let before = stats.scores.clone();
        let mut class = case["initial_class"].as_u64().unwrap() as u32;
        let enabled = b(&case["enabled"]);
        if case["entry"] == "track" {
            tracked += 1;
            statistics_tracking::track(&mut stats, &mut class, &facts, enabled, p).unwrap();
        } else {
            statistics_tracking::untrack(&mut stats, &mut class, facts.component_98, enabled, p)
                .unwrap();
        }
        assert_eq!(
            u64::from(class),
            case["final_class"].as_u64().unwrap(),
            "{case}"
        );
        for (index, bits) in case["final_scores"].as_object().unwrap() {
            let i = index.parse::<usize>().unwrap();
            assert_eq!(
                stats.scores[i].0 as u64,
                bits.as_u64().unwrap(),
                "score {i} {case}"
            );
        }
        changed += usize::from(stats.scores != before);
    }
    assert!(tracked == 900 && changed > 500, "{tracked} {changed}");
}
