//! Replays the native 50B220 / 50B1C0 runs of
//! research/phase8-saved-components/native-post-load.json
//! (`updatable_has_50b220`, `updatable_remove_50b1c0`) against
//! `Queues::has_from_owner_head` / `Queues::remove_matching`. Each recorded
//! link topology (bucket chains head first, owner lists most recent first)
//! is rebuilt with deferred `add_at` inserts in a compatible order.
use aa_numeric::Precision;
use aa_world::queues::{Clock, Kind, Queues};
use serde_json::Value;
use std::collections::BTreeMap;

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../research/phase8-saved-components/native-post-load.json"
);

fn owner_id(label: &str) -> u64 {
    u64::from(label.as_bytes()[0])
}

/// Builds the queues; returns them and, per entry key, (queue, bucket).
fn build(row: &Value) -> (Queues, BTreeMap<String, (u8, usize)>) {
    let entries = row["entries"].as_object().unwrap();
    let buckets: Vec<Vec<String>> = row["buckets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b.as_array().unwrap().iter().map(|k| k.as_str().unwrap().to_owned()).collect())
        .collect();
    let owners: Vec<Vec<String>> = row["owners"]
        .as_object()
        .unwrap()
        .values()
        .map(|o| o.as_array().unwrap().iter().map(|k| k.as_str().unwrap().to_owned()).collect())
        .collect();
    // An entry is inserted after every entry that follows it in its bucket
    // chain or owner list (inserts go to both heads).
    let mut after: BTreeMap<String, Vec<String>> = entries.keys().map(|k| (k.clone(), vec![])).collect();
    for chain in buckets.iter().chain(owners.iter()) {
        for w in chain.windows(2) {
            after.get_mut(&w[0]).unwrap().push(w[1].clone());
        }
    }
    let mut order = Vec::new();
    while order.len() < entries.len() {
        let next = after
            .iter()
            .find(|(k, deps)| !order.contains(*k) && deps.iter().all(|d| order.contains(d)))
            .map(|(k, _)| k.clone())
            .expect("consistent topology");
        order.push(next);
    }
    let mut placement = BTreeMap::new();
    let mut bucket_of_queue: BTreeMap<u8, usize> = BTreeMap::new();
    for chain in &buckets {
        let queue = entries[&chain[0]]["queue"].as_u64().unwrap() as u8;
        let index = bucket_of_queue.entry(queue).or_insert(0);
        for k in chain {
            assert_eq!(entries[k]["queue"].as_u64().unwrap() as u8, queue);
            placement.insert(k.clone(), (queue, *index));
        }
        *index += 1;
    }
    let mut q = Queues::new(
        Clock {
            begin_bits: 0,
            end_bits: 0,
            step_bits: 1.0f32.to_bits(),
        },
        Precision::Double53,
    )
    .unwrap();
    for queue in bucket_of_queue.keys() {
        q.initialize(*queue, Kind::Deferred, 8).unwrap();
    }
    for k in &order {
        let (queue, bucket) = placement[k];
        let e = &entries[k];
        q.add_at(
            queue,
            owner_id(e["owner"].as_str().unwrap()),
            e["data"].as_u64().unwrap() as u32,
            (bucket as f32).to_bits(),
        )
        .unwrap();
    }
    (q, placement)
}

#[test]
fn has_from_owner_head_matches_native_50b220() {
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(FIXTURE).unwrap()).unwrap();
    let rows = doc["updatable_has_50b220"].as_array().unwrap();
    assert_eq!(rows.len(), 8);
    for row in rows {
        let name = row["name"].as_str().unwrap();
        let (q, _) = build(row);
        let who = owner_id(row["updatable"].as_str().unwrap());
        let r = q.has_from_owner_head(who, row["queue"].as_u64().unwrap() as u8, row["data"].as_u64().unwrap() as u32);
        assert_eq!(r, row["native_result"].as_bool().unwrap(), "{name}");
    }
}

#[test]
fn remove_matching_matches_native_50b1c0() {
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(FIXTURE).unwrap()).unwrap();
    let rows = doc["updatable_remove_50b1c0"].as_array().unwrap();
    assert_eq!(rows.len(), 7);
    for row in rows {
        let name = row["name"].as_str().unwrap();
        let (mut q, placement) = build(row);
        let who = owner_id(row["updatable"].as_str().unwrap());
        let n = q
            .remove_matching(who, row["queue"].as_u64().unwrap() as u8, row["data"].as_u64().unwrap() as u32)
            .unwrap();
        assert_eq!(u64::from(n), row["native_removed"].as_u64().unwrap(), "{name}");
        let snapshot = q.snapshot();
        let entries = row["entries"].as_object().unwrap();
        for (chain, after) in row["buckets"].as_array().unwrap().iter().zip(row["buckets_after"].as_array().unwrap()) {
            let first = chain[0].as_str().unwrap();
            let (queue, bucket) = placement[first];
            let got = &snapshot.iter().find(|s| s.id == queue).unwrap().buckets[bucket];
            let want: Vec<(u64, u32)> = after
                .as_array()
                .unwrap()
                .iter()
                .map(|k| {
                    let e = &entries[k.as_str().unwrap()];
                    (owner_id(e["owner"].as_str().unwrap()), e["data"].as_u64().unwrap() as u32)
                })
                .collect();
            assert_eq!(got, &want, "{name}: bucket of {first}");
        }
        for (owner, list) in row["owners_after"].as_object().unwrap() {
            assert_eq!(q.has_owner(owner_id(owner)), !list.as_array().unwrap().is_empty(), "{name}: {owner}");
        }
    }
}
