//! Replays research/phase8-ai/native-saidata-ego.json (original constructor
//! 4AB9A0 + KKC_SAIEgoData::LoadData 4B0560 with all its sub-loaders under
//! Unicorn, on every shipped [Ego] at three precisions plus answer-patch
//! mutations) against aa_mind::sai::ego.
mod common;
use aa_mind::sai::ego::{self, ActorRequirements, Ego, GeneralPower, RefField};
use aa_mind::sai::native_hash::NativeHash;
use aa_numeric::Precision;
use common::data_host::{AnswerHost, apply_patches, digest};
use serde_json::{Value, json};

fn read(name: &str) -> Value {
    let path = format!(
        "{}/../../research/phase8-ai/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

const EGO_POINTERS: [(usize, usize); 14] = [
    (0x8, 4),
    (0x10, 4),
    (0x2a4, 12),
    (0x2b0, 12),
    (0x2c0, 12),
    (0x374, 16),
    (0x384, 16),
    (0x3fc, 12),
    (0x704, 16),
    (0x73c, 16),
    (0x830, 16),
    (0x884, 12),
    (0x910, 12),
    (0x92c, 16),
];
const POWER_MASK: [(usize, usize); 6] = [
    (0x18, 0x18),
    (0x4b, 1),
    (0x4c, 8),
    (0x63, 1),
    (0x64, 8),
    (0xb4, 12),
];

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn table<V>(t: &NativeHash<V>, f: impl Fn(&V) -> Value) -> Value {
    json!({
        "count": t.count,
        "grow": u8::from(t.grow),
        "buckets": t.buckets.iter().map(|chain| {
            chain.iter().map(|(k, v)| json!([k, f(v)])).collect::<Vec<_>>()
        }).collect::<Vec<_>>(),
    })
}

fn requirements(r: &ActorRequirements) -> Value {
    json!({"max": r.max_percent_health, "flags": r.flags, "properties": r.properties,
           "multiplier": r.multiplier})
}

fn power(g: &GeneralPower) -> Value {
    let mut image = g.image.clone();
    for (off, n) in POWER_MASK {
        image[off..off + n].fill(0);
    }
    json!({
        "image": hex(&image),
        "location": requirements(&g.location_requirements),
        "cv": g.cv_categories.iter().map(requirements).collect::<Vec<_>>(),
        "value": g.value_categories.iter().map(requirements).collect::<Vec<_>>(),
        "resources": g.resource_amounts,
    })
}

/// The decoded form native_saidata_ego.py writes.
fn decode(e: &Ego, uninitialized: &[usize]) -> Value {
    let mut image = e.image.clone();
    for &off in uninitialized {
        image[off] = 0;
    }
    for (off, n) in EGO_POINTERS {
        image[off..off + n].fill(0);
    }
    for t in 0..ego::GOAL_SLOTS {
        let off = 0x4c + 0x18 * t + 0x14;
        image[off..off + 4].fill(0);
    }
    let priority = |p: &ego::ThingPriority| {
        json!([
            p.count_all_property_actors,
            p.value,
            p.repeat_penalty,
            p.max_recruited_per_think,
            p.one_time_bonus
        ])
    };
    json!({
        "name": e.name,
        "image": hex(&image),
        "goal_preferences": e.goal_preferences.iter().map(|slot| slot.as_ref().map(|list| {
            list.iter().map(|a| json!([a.property, a.actor, a.value, a.value_once_per_company,
                a.multiplier, a.multiplier_once_per_goal, a.multiplier_once_per_company]))
                .collect::<Vec<_>>()
        })).collect::<Vec<_>>(),
        "resource_needs": e.resource_needs,
        "economic_excess": e.economic_excess,
        "component_values": e.component_values.iter().map(|(v, b)| json!([v, b])).collect::<Vec<_>>(),
        "actor_priorities": table(&e.actor_priorities, priority),
        "property_priorities": table(&e.property_priorities, priority),
        "wall_strength": e.wall_strength,
        "recruit_preferences": table(&e.recruit_preferences, |list| {
            json!(list.iter().map(|m| json!([m.thing, m.priority])).collect::<Vec<_>>())
        }),
        "building_bonuses": table(&e.building_bonuses, |v| json!(v)),
        "table_830": table(&e.table_830, |v| json!(v)),
        "general_powers": e.general_powers.iter().map(|g| g.as_ref().map(power)).collect::<Vec<_>>(),
        "flare_priorities": e.flare_priorities,
        "goal_formations": table(&e.goal_formations, |v| json!(v)),
        "refs": e.refs.iter().map(|r| json!({
            "vtable": r.vtable,
            "ids": r.ids,
            "fields": r.fields.iter().map(|f| match f {
                RefField::Ego(off) => json!({"ego": off}),
                RefField::Word(v) => json!({"word": v}),
                RefField::Byte(b) => json!({"byte": b}),
            }).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    })
}

fn first_difference(a: &Value, b: &Value, path: &str) -> Option<String> {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for k in x.keys().chain(y.keys()) {
                let (va, vb) = (
                    x.get(k).unwrap_or(&Value::Null),
                    y.get(k).unwrap_or(&Value::Null),
                );
                if let Some(d) = first_difference(va, vb, &format!("{path}.{k}")) {
                    return Some(d);
                }
            }
            None
        }
        (Value::Array(x), Value::Array(y)) => {
            for i in 0..x.len().max(y.len()) {
                let (va, vb) = (
                    x.get(i).unwrap_or(&Value::Null),
                    y.get(i).unwrap_or(&Value::Null),
                );
                if let Some(d) = first_difference(va, vb, &format!("{path}[{i}]")) {
                    return Some(d);
                }
            }
            None
        }
        (Value::String(x), Value::String(y)) if x != y && x.len() == y.len() && x.len() > 64 => {
            let i = x.bytes().zip(y.bytes()).position(|(p, q)| p != q).unwrap();
            Some(format!(
                "{path} @char {i} (byte {:#x}): {} vs {}",
                i / 2,
                &x[i.saturating_sub(8)..(i + 8).min(x.len())],
                &y[i.saturating_sub(8)..(i + 8).min(y.len())]
            ))
        }
        _ if a != b => Some(format!("{path}: {a} vs {b}")),
        _ => None,
    }
}

#[test]
fn native_ego_load_matches_original() {
    let f = read("native-saidata-ego.json");
    let inputs = read("sai-shipped-egos.json");
    let egos = inputs["egos"].as_array().unwrap();
    assert_eq!(egos.len(), 75);
    let uninitialized: Vec<usize> = f["uninitialized"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as usize)
        .collect();
    // Cases with a full native decode first: their differences are readable.
    let mut cases: Vec<&Value> = f["cases"].as_array().unwrap().iter().collect();
    cases.sort_by_key(|row| row.get("decoded").is_none());
    let (mut faults, mut loaded, mut full) = (0, 0, 0);
    for row in cases {
        let name = row["name"].as_str().unwrap();
        let input = &egos[row["ego"].as_u64().unwrap() as usize];
        let mut answers = json!({});
        for k in [
            "nodes",
            "managers",
            "things",
            "properties",
            "counts",
            "areas",
        ] {
            answers[k] = input[k].clone();
        }
        apply_patches(&mut answers, &row["patches"]);
        let p: Precision = serde_json::from_value(row["precision"].clone()).unwrap();
        let mut host = AnswerHost::new(answers);
        let mut e = Ego::new(input["ids"].as_str().unwrap());
        let result = ego::load(&mut host, input["root"].as_u64().unwrap() as u32, &mut e, p);
        let events = Value::Array(std::mem::take(&mut host.events));
        assert_eq!(
            events.as_array().unwrap().len() as u64,
            row["events"].as_u64().unwrap(),
            "{name}: event count; last Rust events {:?}",
            &events.as_array().unwrap()[events.as_array().unwrap().len().saturating_sub(3)..]
        );
        assert_eq!(
            Value::String(digest(&events)),
            row["events_sha256"],
            "{name}: events differ"
        );
        if !row["fault"].is_null() {
            assert!(result.is_err(), "{name}: native faulted ({})", row["fault"]);
            faults += 1;
            continue;
        }
        result.unwrap_or_else(|err| panic!("{name}: {err}"));
        loaded += 1;
        let got = decode(&e, &uninitialized);
        if let Some(expected) = row.get("decoded") {
            if let Some(d) = first_difference(&got, expected, "ego") {
                panic!("{name}: {d}");
            }
            full += 1;
        }
        assert_eq!(
            Value::String(digest(&got)),
            row["ego_sha256"],
            "{name}: decoded ego"
        );
    }
    assert_eq!(full, 75);
    assert!(
        faults > 100 && loaded > 300,
        "faults {faults}, loaded {loaded}"
    );
}
