//! Replays research/phase8-metagame/native-qsort.json (original VS2003 CRT
//! _qsort 96CE20 + shop comparator 961790 under Unicorn) against msvc_qsort.
use aa_metagame::qsort::msvc_qsort;
use serde_json::Value;

#[test]
fn msvc_qsort_matches_native_96ce20() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-metagame/native-qsort.json"
    );
    let f: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let mut checked = 0;
    let mut cases: Vec<(Vec<i64>, Vec<usize>)> = f["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            (
                c["costs"].as_array().unwrap().iter().map(|v| v.as_i64().unwrap()).collect(),
                c["order"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as usize).collect(),
            )
        })
        .collect();
    for shop in f["shipped_shops"].as_array().unwrap() {
        for kind in ["Technology", "Actor", "Division"] {
            let l = &shop["lists"][kind];
            cases.push((
                l["rows"].as_array().unwrap().iter().map(|r| r["cost"].as_i64().unwrap()).collect(),
                l["order"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as usize).collect(),
            ));
        }
        let l = &shop["lists"]["all"];
        cases.push((
            l["costs"].as_array().unwrap().iter().map(|v| v.as_i64().unwrap()).collect(),
            l["order"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as usize).collect(),
        ));
    }
    let mut unstable = 0;
    for (costs, want) in &cases {
        let mut items: Vec<(i32, usize)> = costs.iter().enumerate().map(|(i, &c)| (c as i32, i)).collect();
        msvc_qsort(&mut items, |a, b| a.0.cmp(&b.0));
        let got: Vec<usize> = items.iter().map(|x| x.1).collect();
        assert_eq!(&got, want, "costs {costs:?}");
        let mut stable = (0..costs.len()).collect::<Vec<_>>();
        stable.sort_by_key(|&i| costs[i]);
        unstable += usize::from(stable != *want);
        checked += 1;
    }
    assert_eq!(checked, 315 + 20);
    // the fixture must exercise the instability (equal costs reordered)
    assert!(unstable > 50, "{unstable}");
}
