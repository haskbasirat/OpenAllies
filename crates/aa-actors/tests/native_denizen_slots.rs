//! Replays research/phase8-combat/native-denizen-slots.json (original
//! 568860, 568D40, 568550, 573AF0 and 573D10 under Unicorn) against
//! aa_actors::denizen_slots: the results and the 57A730 calls in order.
use aa_actors::denizen_slots::{
    OpenRegimentWorld, Record, Result, SlotEntry, any_active, count_present, free_slot_for,
    has_open_regiment, remove_kind2,
};
use serde_json::{Value, json};

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-combat/native-denizen-slots.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_i64().unwrap() as u32
}

struct Host<'a> {
    c: &'a Value,
    events: Vec<Value>,
}
impl Host<'_> {
    fn actor(&self, k: u32) -> &Value {
        &self.c["actors"][k as usize]
    }
}
impl OpenRegimentWorld for Host<'_> {
    fn resolve(&mut self, gtid: u32) -> Result<Option<u32>> {
        let actors = self.c["actors"].as_array().unwrap();
        let hit = actors.iter().position(|a| u(&a["gtid"]) & 0xffff == gtid & 0xffff);
        Ok(hit.and_then(|k| {
            let a = &actors[k];
            (u(&a["registered_generation"]) == gtid >> 16 && !a["null_entry"].as_bool().unwrap())
                .then_some(k as u32)
        }))
    }
    fn organization(&mut self, actor: u32) -> Result<Option<u32>> {
        Ok((!self.actor(actor)["org"].is_null()).then_some(actor))
    }
    fn capacity(&mut self, organization: u32) -> Result<i32> {
        Ok(u(&self.actor(organization)["org"]["capacity"]) as i32)
    }
    fn member_count(&mut self, organization: u32) -> Result<i32> {
        self.events.push(json!(["count", organization]));
        Ok(u(&self.actor(organization)["org"]["count"]) as i32)
    }
}

#[test]
fn denizen_slots_match_native() {
    let d = fixture();
    let cases = d["cases"].as_array().unwrap();
    assert!(cases.len() >= 380);
    let mut open = 0;
    for case in cases {
        let c = &case["input"];
        let want = &case["output"];
        let name = c["name"].as_str().unwrap();
        match c["entry"].as_str().unwrap() {
            "active" | "free" => {
                let entries: Vec<SlotEntry> = c["entries"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|e| SlotEntry {
                        word4: u(&e["word4"]),
                        timer_bits: u(&e["timer"]),
                        active: u(&e["active"]) as u8,
                    })
                    .collect();
                if c["entry"] == "active" {
                    assert_eq!(u32::from(any_active(&entries)), u(&want["result"]), "{name}");
                } else {
                    let got = free_slot_for(&entries, u(&c["object_word4"]));
                    assert_eq!(json!(got), want["result"], "{name}");
                }
            }
            "count" => {
                let words: Vec<u32> = c["words"].as_array().unwrap().iter().map(u).collect();
                assert_eq!(count_present(&words), u(&want["result"]) as i32, "{name}");
            }
            "remove" => {
                let mut records: Vec<Option<Record>> = c["records"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|r| (!r.is_null()).then(|| Record { kind: u(&r["kind"]), word8: u(&r["word8"]) }))
                    .collect();
                let got = remove_kind2(&mut records, u(&c["value"]));
                assert_eq!(i64::from(got), want["result"].as_i64().unwrap(), "{name}");
                let freed: Vec<Value> = if got >= 0 { vec![json!(["free", got])] } else { Vec::new() };
                assert_eq!(Value::Array(freed), want["events"], "{name} free");
                let slots: Vec<u32> = records.iter().map(|r| u32::from(r.is_some())).collect();
                assert_eq!(json!(slots), want["slots"], "{name} slots");
            }
            _ => {
                let gtids: Vec<u32> = c["list"].as_array().unwrap().iter().map(u).collect();
                let mut host = Host { c, events: Vec::new() };
                let got = has_open_regiment(&gtids, &mut host).unwrap();
                assert_eq!(u32::from(got), u(&want["result"]), "{name}");
                assert_eq!(Value::Array(host.events), want["events"], "{name} events");
                open += usize::from(got);
            }
        }
    }
    assert!(open > 30, "{open}");
}
