//! Replays research/phase8-ai/native-map-analysis.json (original 7AC0D0 with
//! 7AA5B0, 7A9C60/7A9C80, 7AE350, 7ABA70, 7AB940, 7AA620, 7BB780/7BB820 under
//! Unicorn) against aa_mind::sai::map_analysis.
use aa_mind::{
    Result,
    sai::{
        ego::Ego,
        map_analysis::{
            LearningRecord, MapRegion, MapSvars, MapWorld, RegionEntry, SaiMap, SaiPlayer,
            dynamic_map_analysis,
        },
        native_hash::NativeHash,
    },
    x87::binary_from_x87,
};
use aa_numeric::{Binary, Precision, crt_pow::CrtEnvironment};
use serde_json::{Value, json};

fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}

struct Host<'a> {
    c: &'a Value,
    events: Vec<Value>,
}

impl Host<'_> {
    fn object(&self, h: u32) -> &Value {
        &self.c["objects"][h.to_string()]
    }
    fn x87(v: &Value) -> Result<Binary> {
        binary_from_x87(
            v["significand"].as_u64().unwrap(),
            v["sign_exponent"].as_u64().unwrap() as u16,
        )
    }
}

impl MapWorld for Host<'_> {
    fn resolve(&mut self, gtid: u32) -> Result<Option<u32>> {
        for (h, o) in self.c["objects"].as_object().unwrap() {
            let g = u(&o["gtid"]);
            if g & 0xffff == gtid & 0xffff {
                return Ok((g >> 16 == gtid >> 16 && o["live"].as_bool().unwrap())
                    .then(|| h.parse().unwrap()));
            }
        }
        Ok(None)
    }
    fn deleting(&mut self, object: u32) -> Result<bool> {
        Ok(u(&self.object(object)["flags"]) & 1 != 0)
    }
    fn object_gtid(&mut self, object: u32) -> Result<u32> {
        Ok(u(&self.object(object)["gtid"]))
    }
    fn owner(&mut self, object: u32) -> Result<u32> {
        let k = u(&self.object(object)["owner"]);
        self.events
            .push(json!({"op": "owner", "object": object, "result": k}));
        Ok(k)
    }
    fn owner_index(&mut self, owner: u32) -> Result<u32> {
        Ok(owner)
    }
    fn combat_value(&mut self, object: u32) -> Result<Binary> {
        self.events.push(json!({"op": "cv", "object": object}));
        Self::x87(&self.object(object)["cv"])
    }
    fn has_organization(&mut self, object: u32) -> Result<bool> {
        Ok(!self.object(object)["org"].is_null())
    }
    fn organization_actor(&mut self, object: u32) -> Result<u32> {
        let r = u(&self.object(object)["org"]);
        self.events
            .push(json!({"op": "organization_actor", "object": object, "result": r}));
        Ok(r)
    }
    fn definition(&mut self, object: u32) -> Result<u32> {
        Ok(u32::from(
            self.object(object)["definition"].as_bool().unwrap(),
        ))
    }
    fn settlement_component_range(&mut self, object: u32) -> Result<Option<u32>> {
        Ok(self.object(object)["b0"].as_u64().map(|v| v as u32))
    }
    fn slot_count(&mut self, object: u32) -> Result<i32> {
        Ok(self.object(object)["slot_count"].as_i64().unwrap() as i32)
    }
    fn slot5_range(&mut self, object: u32) -> Result<Option<u32>> {
        Ok(self.object(object)["slot5"].as_u64().map(|v| v as u32))
    }
    fn speed(&mut self, object: u32) -> Result<Binary> {
        self.events.push(json!({"op": "speed", "object": object}));
        Self::x87(&self.object(object)["speed"])
    }
    fn region_terrain(&mut self, region: u32) -> Result<u32> {
        let t = u(&self.c["regions"][region as usize]["terrain"]);
        self.events
            .push(json!({"op": "terrain", "region": region, "result": t}));
        Ok(t)
    }
    fn game_time(&mut self) -> Result<u32> {
        Ok(u(&self.c["game_time"]))
    }
    fn yield_slice(&mut self) -> Result<()> {
        self.events.push(json!({"op": "yield"}));
        Ok(())
    }
    fn schedule(&mut self, delay: u32, immediate: bool) -> Result<()> {
        self.events
            .push(json!({"op": "schedule", "delay": delay, "immediate": u8::from(immediate)}));
        Ok(())
    }
}

fn hash<V>(t: &Value, value: impl Fn(&Value) -> V, count: u32) -> NativeHash<V> {
    NativeHash {
        buckets: t["buckets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|chain| {
                chain
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|kv| (u(&kv[0]), value(&kv[1])))
                    .collect()
            })
            .collect(),
        count,
        grow: true,
    }
}

fn words(v: &Value) -> Vec<u32> {
    v.as_array().unwrap().iter().map(u).collect()
}

fn records(v: &Value) -> Option<Vec<LearningRecord>> {
    v.as_array().map(|list| {
        list.iter()
            .map(|r| LearningRecord {
                value: u(&r[0]),
                time: u(&r[1]),
                word8: u(&r[2]),
            })
            .collect()
    })
}

#[test]
fn native_dynamic_map_analysis_matches_original() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-map-analysis.json"
    );
    let f: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 400);
    let (mut faults, mut spread) = (0, 0);
    for (i, row) in cases.iter().enumerate() {
        let c = &row["input"];
        let o = &row["output"];
        let p: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let control = match p {
            Precision::Single24 => 0x007f,
            Precision::Double53 => 0x027f,
            _ => 0x037f,
        };
        let crt = CrtEnvironment {
            sse2_available: true,
            mxcsr: 0x1f80,
            x87_control: control,
        };
        let mut map = SaiMap {
            analysed: c["analysed"].as_bool().unwrap(),
            regions: c["regions"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| {
                    let nb = &r["neighbors"];
                    let count = nb["buckets"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|ch| ch.as_array().unwrap().len() as u32)
                        .sum();
                    MapRegion {
                        index: u(&r["index"]),
                        neighbors: hash(nb, u, count),
                        entries: r["entries"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|e| {
                                (!e.is_null()).then(|| RegionEntry {
                                    player: e["player"].as_u64().unwrap() as usize,
                                    mobile: words(&e["stale"]),
                                    settlement: words(&e["stale"]),
                                    border: words(&e["stale"]),
                                })
                            })
                            .collect(),
                        static_lists: r["static"].as_array().unwrap().iter().map(words).collect(),
                        mobile_lists: r["mobile"].as_array().unwrap().iter().map(words).collect(),
                    }
                })
                .collect(),
        };
        let mut egos = Vec::new();
        let mut players = Vec::new();
        for (k, pl) in c["players"].as_array().unwrap().iter().enumerate() {
            let mut ego = Ego::new("map");
            let e = &pl["ego"];
            for (name, off) in [
                ("cv_influence_exponent", 0x28),
                ("cv_influence_max_travel_time", 0x2c),
                ("border_influence_range", 0x30),
                ("learning_factor", 0x93c),
                ("recruit_duration", 0x940),
                ("cv_exponent", 0x958),
                ("cv_ceiling", 0x95c),
            ] {
                ego.set_u32(off, u(&e[name]));
            }
            ego.set_u8(0x960, e["cv_scale_over_duration"].as_u64().unwrap() as u8);
            egos.push(ego);
            players.push(SaiPlayer {
                slot: u(&pl["slot"]),
                ego: k,
                learning: hash(&pl["learning"], records, u(&pl["learning"]["count"])),
            });
        }
        let sv = MapSvars {
            speed_cap: u(&c["svars"]["speed_cap"]),
            settlement_cv_range_fudge_factor: u(&c["svars"]["settlement_cv_range_fudge_factor"]),
            min_cv_propogation_factor: u(&c["svars"]["min_cv_propogation_factor"]),
            think_frequency: u(&c["svars"]["think_frequency"]),
        };
        let mut immediate = c["immediate"].as_bool().unwrap();
        let mut host = Host { c, events: vec![] };
        let result = dynamic_map_analysis(
            &mut map,
            &mut players,
            &egos,
            &mut host,
            &sv,
            &mut immediate,
            p,
            crt,
        );
        assert_eq!(json!(host.events), o["events"], "case {i}: events");
        if !o["fault"].is_null() {
            assert!(result.is_err(), "case {i}: native faulted");
            faults += 1;
            continue;
        }
        result.unwrap_or_else(|e| panic!("case {i}: {e}"));
        assert!(!immediate);
        let regions: Vec<Value> = map
            .regions
            .iter()
            .map(|r| {
                json!(
                    r.entries
                        .iter()
                        .map(|e| e.as_ref().map(|e| json!({
                            "mobile": e.mobile, "settlement": e.settlement, "border": e.border
                        })))
                        .collect::<Vec<_>>()
                )
            })
            .collect();
        assert_eq!(json!(regions), o["regions"], "case {i}: influence");
        let learned: Vec<Value> = players
            .iter()
            .map(|pl| {
                json!({
                    "count": pl.learning.count,
                    "buckets": pl.learning.buckets.iter().map(|chain| chain.iter().map(|(k, v)| {
                        json!([k, v.as_ref().map(|l| l.iter().map(|r| json!([r.value, r.time, r.word8])).collect::<Vec<_>>())])
                    }).collect::<Vec<_>>()).collect::<Vec<_>>(),
                })
            })
            .collect();
        assert_eq!(json!(learned), o["players"], "case {i}: learning tables");
        if regions
            .iter()
            .flat_map(|r| r.as_array().unwrap().iter())
            .any(|e| !e.is_null() && !e["border"].as_array().unwrap().is_empty())
        {
            spread += 1;
        }
    }
    assert!(
        faults > 0 && spread > 100,
        "faults {faults}, spread {spread}"
    );
}
