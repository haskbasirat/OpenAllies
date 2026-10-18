use aa_orders::net::sync::{Synchronizer,EntryKind};
#[test]
fn scope_marker_matches_original_v14_guards_hash_and_retained_window() {
    let fixture:serde_json::Value=serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-mission-start/native-sync-marker.json")).unwrap();
    let rows=fixture["cases"].as_array().unwrap();assert_eq!(rows.len(),63);
    for row in rows {
        let c=&row["input"];
        let mut sync=Synchronizer::new(c["max"].as_i64().unwrap() as i32);
        sync.reset(-1,c["hash"].as_i64().unwrap() as i32);
        sync.enabled=c["enabled"]==1;sync.suspend=c["suspend"].as_i64().unwrap() as i32;
        let gate=c["thread"]==1 && (c["null_app"]==1 || (c["flag5c"]==0 && c["flag5d"]==0));
        for expected in row["output"].as_array().unwrap() {
            sync.log_marker(gate,"MergeKingdoms");
            let entries:Vec<_>=(sync.begin()..sync.end()).map(|i| {
                let entry=sync.entry(i).unwrap();assert_eq!(entry.label,"MergeKingdoms");assert_eq!(entry.kind,EntryKind::Empty);
                serde_json::json!({"kind":entry.kind as u32,"value":entry.value,"hash":entry.hash})
            }).collect();
            assert_eq!(serde_json::json!({"hash":sync.hash,"begin":sync.begin(),"end":sync.end(),"entries":entries}),*expected,"{row}");
        }
    }
}
