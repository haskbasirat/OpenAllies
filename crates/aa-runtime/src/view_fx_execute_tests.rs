//! Replays research/phase8-audio/native-fx-execute.json (native_fx_execute.py):
//! the original 4289A0, 42A770 (CreateVFX and PlaySFX vtables), 42A920 and
//! the 43EC60 FX segment under Unicorn at PC24/PC53/PC64.
use super::*;
use crate::view_fx_command_list::{Key, List, Message};
use serde_json::{json, Value};

fn bits(v: &Value) -> u32 {
    (v.as_f64().unwrap() as f32).to_bits()
}
fn pc(row: &Value) -> Precision {
    match row["precision"].as_str().unwrap() {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        _ => Precision::Extended64,
    }
}
fn fixture() -> Value {
    let path = std::env::var("AA_FX_FIXTURE")
        .unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-audio/native-fx-execute.json").into());
    serde_json::from_str(&std::fs::read_to_string(&path).expect("native-fx-execute.json")).unwrap()
}

#[test]
fn native_4289a0_timing_and_dispatch() {
    let fixture = fixture();
    let rows = fixture["timing"].as_array().unwrap();
    assert_eq!(rows.len(), 486);
    for row in rows {
        let c = &row["input"];
        let out = &row["output"];
        let mut calls = vec![];
        let (result, last) = if !c["ad"].as_bool().unwrap() {
            (false, bits(&c["last"]))
        } else {
            let (prev, elapsed) = times_4289a0(bits(&c["start"]), bits(&c["last"]), bits(&c["now"]), pc(row)).unwrap();
            let mut result = true;
            for (i, d) in c["delays"].as_array().unwrap().iter().enumerate() {
                match node_step(bits(d), prev, elapsed).unwrap() {
                    Step::NotReached => result = false,
                    Step::Passed => {}
                    Step::Fire => {
                        calls.push(json!(["vf1c", i]));
                        if c["filter"][i].as_bool().unwrap() {
                            calls.push(json!(["vf18", i]));
                        }
                    }
                }
            }
            (result, bits(&c["now"]))
        };
        assert_eq!(json!(result as u32), out["result"], "{c}");
        assert_eq!(json!(last), out["last"], "{c}");
        assert_eq!(Value::Array(calls), out["calls"], "{c}");
    }
}

struct Reads<'a> {
    c: &'a Value,
    calls: Vec<Value>,
}
impl Reads<'_> {
    fn thing_b4(&self, key: &str) -> bool {
        self.c[key].is_object() && self.c[key]["b4"].as_bool().unwrap()
    }
}
impl FilterReads for Reads<'_> {
    fn vf20(&mut self) -> Result<bool> {
        //428360 (CreateVFX): def+24 == 1 and node+64 == 0; 6A71F0 (PlaySFX).
        Ok(self.c["kind"] == "vfx" && self.c["def24"] == 1 && self.c["b64"] == 0)
    }
    fn target_alive(&mut self) -> Result<bool> {
        Ok(self.thing_b4("target"))
    }
    fn target_flags_4cf410(&mut self) -> Result<u32> {
        self.calls.push(json!(["4cf410"]));
        Ok(self.c["flags"].as_u64().unwrap() as u32)
    }
    fn terrain_group_at(&mut self, xy: [u32; 2]) -> Result<Option<String>> {
        self.calls.push(json!(["4282a0", xy[0], xy[1]]));
        if !self.c["valid"].as_bool().unwrap() {
            return Ok(None);
        }
        self.calls.push(json!(["42a530", xy[0], xy[1]]));
        self.calls.push(json!(["54ece0"]));
        Ok(Some(if self.c["tile_group"].as_bool().unwrap() { "group".into() } else { "other".into() }))
    }
    fn source_alive(&mut self) -> Result<bool> {
        Ok(self.thing_b4("source"))
    }
    fn source_owns_14(&mut self) -> Result<bool> {
        let s = &self.c["source"];
        Ok(!s["r100"].as_bool().unwrap() || !s["r98"].as_bool().unwrap() || s["owns"].as_bool().unwrap())
    }
}

#[test]
fn native_42a770_filter() {
    let fixture = fixture();
    let rows = fixture["filter"].as_array().unwrap();
    assert_eq!(rows.len(), 1800);
    for row in rows {
        let c = &row["input"];
        let vfx = c["kind"] == "vfx";
        let at = c["at"].as_u64().unwrap() as u32;
        let p = |k: &str, i: usize| bits(&c[k][i]);
        let input = FilterInput {
            byte58: c["byte58"] == 1,
            must_18: c["must"].as_u64().unwrap() as u32,
            must_not_1c: c["must_not"].as_u64().unwrap() as u32,
            group_20: c["group"].as_bool().unwrap().then(|| "group".into()),
            //42AF30 (CreateVFX node+40) / 42AE10 (PlaySFX node+3C): AT 1, 3, 5.
            vf30: matches!(at, 1 | 3 | 5),
            source_xy: [p("src_pos", 0), p("src_pos", 1)],
            target_xy: [p("dst_pos", 0), p("dst_pos", 1)],
            threshold_2c: if c["threshold"].is_null() { 0xbf80_0000 } else { bits(&c["threshold"]) },
            mode_30: c["mode"].as_u64().unwrap() as u32,
            before_40: bits(&c["before"]),
            now_44: bits(&c["now"]),
            state_34: c["state"].as_u64().unwrap() as u32,
            ctx_state_54: c["ctx_state"].as_u64().unwrap() as u32,
        };
        let _ = vfx;
        let mut reads = Reads { c, calls: vec![] };
        let result = filter_values_42a770(&input, &mut reads).unwrap();
        assert_eq!(json!(result as u32), row["output"]["result"], "{c}");
        assert_eq!(Value::Array(reads.calls), row["output"]["calls"], "{c}");
    }
}

#[test]
fn native_42a920_play_sfx_arguments() {
    let fixture = fixture();
    let rows = fixture["sfx"].as_array().unwrap();
    assert_eq!(rows.len(), 900);
    for row in rows {
        let c = &row["input"];
        let thing = |k: &str| -> Option<ThingState> {
            let t = &c[k];
            t.is_object().then(|| ThingState { alive_b4: t["b4"].as_bool().unwrap(), position: [bits(&t["pos"][0]), bits(&t["pos"][1]), bits(&t["pos"][2])] })
        };
        let v3 = |k: &str| [bits(&c[k][0]), bits(&c[k][1]), bits(&c[k][2])];
        let args = sfx_args_42a920(c["at"].as_u64().unwrap() as u32, c["loop"].as_u64().unwrap() as u8, v3("src_pos"), v3("dst_pos"),
            thing("source"), thing("target"), [bits(&c["map"][0]), bits(&c["map"][1])], pc(row)).unwrap();
        //The +8 child (ChainImmediate) never fires: 4289A0 sees prev == elapsed.
        let attached = args.attached.map(|s| if s == 0 { "source" } else { "target" });
        let owner = c["owner"].is_object().then_some("owner");
        let expected = json!([["7474f0", true, args.loop_38 as u32, attached, 0, owner, args.position, true, true]]);
        assert_eq!(row["output"]["calls"], expected, "{c}");
        assert_eq!(row["output"]["result"], 1);
    }
}

struct Cmd {
    name: usize,
    start: u32,
    last: u32,
    delays: Vec<u32>,
    next: Option<Key>,
}
impl Message for Cmd {
    fn set_next(&mut self, next: Option<Key>) {
        self.next = next;
    }
    fn next(&self) -> Option<&Key> {
        self.next.as_ref()
    }
}

#[test]
fn native_43ec60_fx_segment() {
    let fixture = fixture();
    let rows = fixture["loop"].as_array().unwrap();
    assert_eq!(rows.len(), 240);
    for row in rows {
        let c = &row["input"];
        let precision = pc(row);
        let now = bits(&c["now"]);
        let mut list = List::from_native_constructor();
        let mut keys = vec![];
        for (i, spec) in c["commands"].as_array().unwrap().iter().enumerate() {
            let mut m = Some(Cmd {
                name: i,
                start: bits(&spec["start"]),
                last: bits(&spec["last"]),
                delays: spec["delays"].as_array().unwrap().iter().map(bits).collect(),
                next: None,
            });
            keys.push(list.append(&mut m, |_| Ok(())).unwrap());
        }
        let mut calls: Vec<Value> = vec![];
        let mut lasts = vec![0u32; keys.len()];
        fx_loop_43ec60(
            &mut list,
            |cmd: &mut Cmd, calls: &mut Vec<Value>| {
                let (prev, elapsed) = times_4289a0(cmd.start, cmd.last, now, precision)?;
                cmd.last = now;
                let mut result = true;
                for (j, d) in cmd.delays.iter().enumerate() {
                    match node_step(*d, prev, elapsed)? {
                        Step::NotReached => result = false,
                        Step::Passed => {}
                        Step::Fire => {
                            calls.push(json!(["vf1c", format!("{}.{}", cmd.name, j)]));
                            calls.push(json!(["vf18", format!("{}.{}", cmd.name, j)]));
                        }
                    }
                }
                Ok(result)
            },
            |cmd: &mut Cmd, calls: &mut Vec<Value>| {
                calls.push(json!(["429050", cmd.name]));
                calls.push(json!(["45c7a0", cmd.name]));
                calls.push(json!(["free", cmd.name]));
                Ok(())
            },
            &mut calls,
        )
        .unwrap();
        for (i, k) in keys.iter().enumerate() {
            if let Some(cmd) = list.get(k) {
                lasts[i] = cmd.last;
            } else {
                lasts[i] = now;
            }
        }
        let out = &row["output"];
        assert_eq!(Value::Array(calls), out["calls"], "{c}");
        let mut order = vec![];
        let mut p = list.head().cloned();
        while let Some(k) = p {
            let cmd = list.get(&k).unwrap();
            order.push(cmd.name);
            p = cmd.next.clone();
        }
        assert_eq!(json!(order), out["order"], "{c}");
        assert_eq!(json!(list.native_count()), out["count"], "{c}");
        let tail = list.tail().and_then(|k| list.get(k)).map(|c| c.name);
        //A freed last command leaves the native tail dangling (still its pointer).
        if !out["tail"].is_null() && tail.is_some() {
            assert_eq!(json!(tail), out["tail"], "{c}");
        }
        assert_eq!(json!(lasts), out["last"], "{c}");
    }
}
