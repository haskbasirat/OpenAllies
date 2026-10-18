//! Replays research/phase8-ai/native-recover-select.json (original 52BB40 with
//! native 58AC70 and 52A930 under Unicorn) against
//! aa_mind::recover_select::select.
use aa_mind::{
    Result,
    recover_select::{Context, RecoverWorld, SupplyComponent, SupplyThing, select},
    rout_score::{self, MapGeometry, RoutWorld, Settings, SupplySource, Weights},
    x87::binary_from_x87,
};
use aa_numeric::{Binary, Precision};
use serde_json::{Value, json};

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-recover-select.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn pair(v: &Value) -> [u32; 2] {
    [u(&v[0]), u(&v[1])]
}
fn weights(v: &Value) -> Weights {
    let w: Vec<u32> = v.as_array().unwrap().iter().map(u).collect();
    Weights {
        base_value: w[0],
        factor_enemy_dist: w[1],
        factor_enemy_dist_cap: w[2],
        factor_enemy_angle: w[3],
        factor_moved_dist: w[4],
        factor_moved_dist_cap: w[5],
        factor_supply_dist: w[6],
        factor_supply_dist_cap: w[7],
        penalty_enemy_zoc: w[8],
        penalty_enemy_zog: w[9],
        bonus_friendly_zos: w[10],
        bonus_supply_in_range: w[11],
        fuzziness: w[12],
    }
}

struct Host {
    c: Value,
    native_angles: Vec<Value>,
    rng: u32,
    events: Vec<Value>,
    open_calls: usize,
}
impl RoutWorld for Host {
    fn actor_position(&mut self, gtid: u32) -> Result<Option<[u32; 2]>> {
        Ok(self.c["engaged"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| u(&e["gtid"]) == gtid && !e["position"].is_null())
            .map(|e| pair(&e["position"])))
    }
    fn angle(&mut self, from: [u32; 2], to: [u32; 2], p: Precision) -> Result<Binary> {
        let native = self.native_angles.remove(0);
        assert_eq!(json!(from), native["from"]);
        assert_eq!(json!(to), native["to"]);
        let value = binary_from_x87(
            native["extended_significand"].as_u64().unwrap(),
            native["extended_sign_exponent"].as_u64().unwrap() as u16,
        )?;
        assert_eq!(
            aa_trig::signed_angle(from, to, p, true).unwrap(),
            value.store_f32().unwrap()
        );
        self.events.push(native);
        Ok(value)
    }
    fn owner(&mut self) -> Result<u32> {
        self.events.push(json!({"event": "owner"}));
        Ok(0x0bad_0000)
    }
    fn zone(&mut self, tile: i32, _owner: u32, kind: u32) -> Result<u32> {
        let result = u(&self.c["zone_results"][kind.to_string()]);
        self.events
            .push(json!({"event": "zone", "tile": tile, "kind": kind, "result": result}));
        Ok(result)
    }
    fn friendly_zone(&mut self, tile: i32, _owner: u32, organization: bool) -> Result<bool> {
        let result = if organization {
            self.c["friendly_zone"].as_bool().unwrap()
        } else {
            self.c["recover_friendly"][(tile & 1).to_string()]
                .as_bool()
                .unwrap()
        };
        self.events
            .push(json!({"event": "friendly_zone", "tile": tile,
            "organization": organization, "result": result}));
        Ok(result)
    }
    fn kingdom_allied(&mut self, slot: usize, _owner: u32) -> Result<bool> {
        let result = self.c["kingdoms"][slot]["allied"].as_bool().unwrap();
        self.events
            .push(json!({"event": "allied", "kingdom": slot, "result": result}));
        Ok(result)
    }
    fn random(&mut self) -> Result<u32> {
        let wide = u64::from(self.rng) * 0x41c6_4e6d + 0x3039;
        self.rng = wide as u32;
        Ok((wide >> 16) as u32)
    }
}
impl RecoverWorld for Host {
    fn organization_virtual_4(&mut self) -> Result<()> {
        self.events.push(json!({"event": "organization_virtual_4"}));
        Ok(())
    }
    fn recover_kingdom_allied(&mut self, slot: usize, _owner: u32) -> Result<bool> {
        let result = self.c["recover_kingdoms"][slot]["allied"]
            .as_bool()
            .unwrap();
        self.events
            .push(json!({"event": "recover_allied", "kingdom": slot, "result": result}));
        Ok(result)
    }
    fn find_open_position(&mut self, position: &mut [u32; 2], radius: u32) -> Result<bool> {
        assert_eq!(radius, 0x4080_0000);
        let list = self.c["open"].as_array().unwrap();
        let rule = list[self.open_calls % list.len()].clone();
        self.open_calls += 1;
        let before = *position;
        if !rule["move"].is_null() {
            *position = pair(&rule["move"]);
        }
        let result = rule["result"].as_bool().unwrap();
        self.events.push(
            json!({"event": "open", "before": before, "after": *position,
            "result": result}),
        );
        Ok(result)
    }
}

fn component(v: &Value) -> Option<SupplyComponent> {
    if v.is_null() {
        return None;
    }
    let entries = v["entries"].as_array().unwrap();
    let count = v
        .get("count")
        .map(|c| c.as_i64().unwrap())
        .unwrap_or(entries.len() as i64);
    let first = if count > 0 {
        entries.first().and_then(|e| e.as_u64()).map(|w| w as u32)
    } else {
        None
    };
    Some(SupplyComponent {
        radius: u(&v["radius"]),
        first_entry_word8: first,
    })
}

#[test]
fn native_recover_destination_selection_matches_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 150);
    let mut selected = 0;
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let name = &c["name"];
        let p: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let context = Context {
            score: rout_score::Context {
                settings: Settings {
                    rout: weights(&c["svars"]["Rout"]),
                    recover: weights(&c["svars"]["Recover"]),
                },
                map: MapGeometry {
                    shift: u(&c["map"]["shift"]),
                    width: c["map"]["width"].as_i64().unwrap() as i32,
                },
                organization: pair(&c["org"]),
                engaged: c["engaged"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|e| u(&e["gtid"]))
                    .collect(),
                kingdoms: c["kingdoms"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|k| {
                        (!k.is_null()).then(|| {
                            k["supplies"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .map(|s| SupplySource {
                                    position: pair(&s["position"]),
                                    radius: s["radius"].as_u64().map(|r| r as u32),
                                })
                                .collect()
                        })
                    })
                    .collect(),
            },
            kingdoms: c["recover_kingdoms"]
                .as_array()
                .unwrap()
                .iter()
                .map(|k| {
                    (!k.is_null()).then(|| {
                        k["things"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|t| {
                                (!t.is_null()).then(|| SupplyThing {
                                    gtid: u(&t["gtid"]),
                                    position: pair(&t["position"]),
                                    component: component(&t["component"]),
                                })
                            })
                            .collect()
                    })
                })
                .collect(),
            candidates: u(&c["recover_candidates"]) as i32,
            directions: c["directions"]
                .as_array()
                .unwrap()
                .iter()
                .map(pair)
                .collect(),
            moved_from: (!c["moved_from"].is_null()).then(|| pair(&c["moved_from"])),
        };
        let native_events: Vec<Value> = o["events"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["event"] != "svar")
            .cloned()
            .collect();
        let mut host = Host {
            c: c.clone(),
            native_angles: native_events
                .iter()
                .filter(|e| e["event"] == "angle")
                .cloned()
                .collect(),
            rng: u(&c["rng_state"]),
            events: vec![],
            open_calls: 0,
        };
        let result = select(&context, &mut host, p).unwrap();
        assert!(host.native_angles.is_empty(), "{name}");
        assert_eq!(host.events, native_events, "{name}");
        assert_eq!(json!(host.rng), o["rng_state"], "{name}");
        match result {
            Some((position, gtid)) => {
                selected += 1;
                assert_eq!(o["selected"], json!(1), "{name}");
                assert_eq!(json!(position), o["out"], "{name}");
                assert_eq!(json!(gtid), o["gtid"], "{name}");
            }
            None => {
                assert_eq!(o["selected"], json!(0), "{name}");
                assert_eq!(
                    o["out"],
                    json!([0xdead_beef_u32, 0xfeed_face_u32]),
                    "{name}"
                );
                assert_eq!(o["gtid"], json!(0xcafe_f00d_u32), "{name}");
            }
        }
    }
    assert_eq!(selected, 60);
}
