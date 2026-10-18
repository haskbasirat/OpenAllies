//! Replays research/phase8-ai/native-rout-score.json (original 52A930 under
//! Unicorn) against aa_mind::rout_score::score.
use aa_mind::{
    Result,
    rout_score::{
        Candidate, Context, MapGeometry, RoutWorld, Settings, SupplySource, Weights, score,
    },
    x87::binary_from_x87,
};
use aa_numeric::{Binary, Precision};
use serde_json::{Value, json};
use std::cmp::Ordering;

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-rout-score.json"
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
fn extended(v: &Value) -> Binary {
    binary_from_x87(
        v["extended_significand"].as_u64().unwrap(),
        v["extended_sign_exponent"].as_u64().unwrap() as u16,
    )
    .unwrap()
}

struct Host {
    c: Value,
    native_angles: Vec<Value>,
    rng: u32,
    events: Vec<Value>,
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
        // Bound to the native 445400 ST0 captured for this call; its inputs
        // must equal the Rust-computed directions.
        let native = self.native_angles.remove(0);
        assert_eq!(json!(from), native["from"]);
        assert_eq!(json!(to), native["to"]);
        let value = extended(&native);
        // Independent checks: aa-trig's port of 445400 stores binary32, and
        // the real register value (native_angle, production binding) agrees
        // with Unicorn's binary64 FPATAN within two binary64 ulps.
        let stored = value.store_f32().unwrap();
        assert_eq!(aa_trig::signed_angle(from, to, p, true).unwrap(), stored);
        let hardware = aa_mind::rout_score::native_angle(from, to, p).unwrap();
        assert_eq!(hardware.store_f32().unwrap(), stored);
        let e = Precision::Extended64;
        let diff = hardware.sub(&value, e).unwrap();
        let diff = if diff.is_negative_nonzero() {
            diff.negated()
        } else {
            diff
        };
        let magnitude = if value.is_negative_nonzero() {
            value.clone().negated()
        } else {
            value.clone()
        };
        let bound = magnitude
            .mul(&Binary::from_f64_bits(0x3cc0_0000_0000_0000).unwrap(), e)
            .unwrap();
        assert_ne!(diff.compare(&bound), Ordering::Greater);
        self.events.push(native);
        Ok(value)
    }
    fn owner(&mut self) -> Result<u32> {
        self.events.push(json!({"event": "owner"}));
        Ok(0x0bad_0000)
    }
    fn zone(&mut self, tile: i32, owner: u32, kind: u32) -> Result<u32> {
        assert_eq!(owner, 0x0bad_0000);
        let result = u(&self.c["zone_results"][kind.to_string()]);
        self.events
            .push(json!({"event": "zone", "tile": tile, "kind": kind, "result": result}));
        Ok(result)
    }
    fn friendly_zone(&mut self, tile: i32, owner: u32, organization: bool) -> Result<bool> {
        assert!(organization);
        assert_eq!(owner, 0x0bad_0000);
        let result = self.c["friendly_zone"].as_bool().unwrap();
        self.events
            .push(json!({"event": "friendly_zone", "tile": tile, "result": result}));
        Ok(result)
    }
    fn kingdom_allied(&mut self, slot: usize, owner: u32) -> Result<bool> {
        assert_eq!(owner, 0x0bad_0000);
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

#[test]
fn native_rout_and_recover_scores_match_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 318);
    let mut angles = 0;
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let name = &c["name"];
        let p: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let context = Context {
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
        };
        let candidate = Candidate {
            position: pair(&c["candidate"]),
            moved_from: (!c["moved_from"].is_null()).then(|| pair(&c["moved_from"])),
            recover: c["recover"].as_bool().unwrap(),
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
        };
        angles += host.native_angles.len();
        let result = score(&candidate, &context, &mut host, p).unwrap();
        assert!(host.native_angles.is_empty(), "{name}");
        assert_eq!(host.events, native_events, "{name}");
        assert_eq!(json!(host.rng), o["rng_state"], "{name}");
        let native = extended(&o["result"]);
        assert_eq!(
            result.compare(&native),
            Ordering::Equal,
            "{name} {result:?} {native:?}"
        );
        assert_eq!(
            result.is_negative_nonzero(),
            native.is_negative_nonzero(),
            "{name}"
        );
        assert_eq!(
            result.store_f32().unwrap(),
            u(&o["result"]["f32_bits"]),
            "{name}"
        );
    }
    assert_eq!(angles, 522);
}
