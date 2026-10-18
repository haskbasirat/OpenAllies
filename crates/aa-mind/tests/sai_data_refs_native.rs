//! Replays research/phase8-sai/native-data-refs.json (original pending
//! DataRef pass 475230 over SAI ego references under Unicorn) against
//! aa_mind::sai::data_refs.
use aa_mind::{
    Error, Result,
    sai::data_refs::{EgoLinks, RefLookups, resolve_pending},
    sai::ego::{DataRef, Ego, RefField},
};
use serde_json::{Value, json};

struct Host {
    c: Value,
    events: Vec<Value>,
}

fn u(v: &Value) -> u32 {
    v.as_i64().unwrap() as u32
}

impl Host {
    fn index(&mut self, op: &str, table: &str, ids: &str) -> Option<i32> {
        let r = self.c[table][ids.to_lowercase()].as_i64().map(|v| v as i32);
        self.events.push(json!({"op": op, "ids": ids, "result": r}));
        r
    }
}

impl RefLookups for Host {
    fn statistic(&mut self, ids: &str) -> Result<Option<i32>> {
        Ok(self.index("statistic", "statistic", ids))
    }
    fn company_template(&mut self, ids: &str) -> Result<Option<i32>> {
        Ok(self.index("company", "company", ids))
    }
    fn settlement_template(&mut self, ids: &str) -> Result<Option<i32>> {
        Ok(self.index("settlement", "settlement", ids))
    }
    fn property(&mut self, ids: &str) -> Result<Option<u32>> {
        let r = self.c["property"][ids.to_lowercase()].as_u64().map(|v| v as u32);
        self.events.push(json!({"op": "property", "ids": ids, "result": r}));
        Ok(r)
    }
    fn actor_thing(&mut self, ids: &str) -> Result<Option<u32>> {
        let t = self.c["thing"][ids.to_lowercase()].clone();
        if t.is_null() {
            self.events.push(json!({"op": "thing", "ids": ids, "result": null}));
            return Ok(None);
        }
        self.events.push(json!({"op": "thing", "ids": ids, "result": t[0]}));
        let actor = t[1].as_bool().unwrap();
        self.events.push(json!({"op": "thing_actor", "result": actor}));
        Ok(actor.then(|| u(&t[0])))
    }
    fn data_error(&mut self, format: &str, args: &[String]) -> Error {
        self.events
            .push(json!({"op": "data_error", "format": format, "args": args}));
        Error::World(format!("data error {format}"))
    }
}

fn data_ref(q: &Value) -> DataRef {
    let w = |i: usize| RefField::Word(u(&q["words"][i]));
    let vtable = u(&q["vtable"]);
    let off = u(&q["offset"]);
    let fields = match vtable {
        0xa1_7038 | 0xa1_7048 => vec![RefField::Ego(off), w(0), w(1)],
        0xa1_703c => vec![RefField::Ego(0), w(0), w(1)],
        0xa1_7040 => vec![RefField::Ego(0), w(0)],
        0xa1_7044 => vec![RefField::Ego(0)],
        0xa1_704c => vec![
            RefField::Ego(0),
            w(0),
            w(1),
            w(2),
            w(3),
            w(4),
            RefField::Byte(u(&q["flag"]) as u8),
            w(6),
        ],
        other => panic!("{other:#x}"),
    };
    DataRef {
        vtable,
        ids: q["ids"].as_str().unwrap().to_owned(),
        fields,
    }
}

#[test]
fn native_data_ref_pass_matches_original() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-sai/native-data-refs.json"
    );
    let f: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let cases = f["cases"].as_array().unwrap();
    assert!(cases.len() >= 600);
    let (mut ok, mut errors) = (0, 0);
    for (i, row) in cases.iter().enumerate() {
        let c = &row["input"];
        let o = &row["output"];
        let mut egos: Vec<Ego> = c["egos"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| {
                let mut ego = Ego::new(e["ids"].as_str().unwrap());
                ego.table_830 = aa_mind::sai::native_hash::NativeHash::new(7);
                ego
            })
            .collect();
        let mut queue = vec![];
        for q in c["queue"].as_array().unwrap() {
            let e = q["ego"].as_u64().unwrap() as usize;
            egos[e].refs.push(data_ref(q));
            queue.push((e, egos[e].refs.len() - 1));
        }
        let mut links = vec![EgoLinks::default(); egos.len()];
        let mut host = Host {
            c: c.clone(),
            events: vec![],
        };
        let result = resolve_pending(&queue, &mut egos, &mut links, &mut host);
        assert_eq!(json!(host.events), o["events"], "case {i}");
        match o["outcome"].as_str().unwrap() {
            "ok" => {
                ok += 1;
                result.unwrap();
            }
            "data_error" => {
                errors += 1;
                assert!(result.is_err(), "case {i}");
            }
            other => panic!("case {i}: {other}"),
        }
        for (k, (ego, link)) in egos.iter().zip(&links).enumerate() {
            let e = &o["egos"][k];
            let filters: Vec<Value> = link
                .filters
                .iter()
                .map(|f| json!([f.statistic, f.min, f.max]))
                .collect();
            assert_eq!(json!(filters), e["filters"], "case {i}: filters");
            assert_eq!(json!(link.company_templates), e["company"], "case {i}");
            assert_eq!(json!(link.settlement_templates), e["settlement"], "case {i}");
            let recruits: Vec<Value> = link
                .recruit_requests
                .iter()
                .map(|r| {
                    json!([r.thing, r.property, r.number, r.base, r.economic, r.repeat_penalty, r.one_time_bonus, r.hq_bonus])
                })
                .collect();
            assert_eq!(json!(recruits), e["recruits"], "case {i}: recruits");
            for (owner, list) in e["combined"].as_object().unwrap() {
                let got: Vec<Value> = link
                    .combined_arms
                    .get(&owner.parse::<u32>().unwrap())
                    .map(|l| {
                        l.iter()
                            .map(|x| json!([x.property, x.max_percent, x.max_multiplier]))
                            .collect()
                    })
                    .unwrap_or_default();
                assert_eq!(json!(got), *list, "case {i}: combined {owner}");
            }
            let tech: Vec<Value> = ego
                .table_830
                .buckets
                .iter()
                .map(|chain| json!(chain.iter().map(|(k, v)| json!([k, v])).collect::<Vec<_>>()))
                .collect();
            assert_eq!(json!(tech), e["tech"], "case {i}: tech hash");
        }
    }
    assert!(ok > 100 && errors > 100, "{ok} {errors}");
}
