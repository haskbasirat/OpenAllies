//! Replays research/phase8-ai/native-uai-kill.json (original 5340C0 and
//! 535240 under Unicorn) against aa_mind::uai_kill.
use aa_mind::{
    Arena, Error, Mind, MindId, Result, State, StateId,
    uai_kill::{
        Ability, ChooseWorld, KillDecision, KillThinkWorld, PerformerState, TargetContext, choose,
        think,
    },
};
use aa_numeric::{Binary, Precision};
use serde_json::{Value, json};

const TARGET: u32 = 1000;

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-uai-kill.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn f64v(v: &Value) -> Binary {
    Binary::from_f64_bits(v.as_f64().unwrap().to_bits()).unwrap()
}
fn rec(r: &KillDecision) -> Value {
    let ability = if r.ability == 0 || r.ability == 0x11 {
        r.ability
    } else {
        r.ability - 1
    };
    json!([
        ability,
        r.kind,
        r.status,
        r.score,
        u32::from(r.target == TARGET),
        r.distance2,
        r.priority
    ])
}

struct Host {
    c: Value,
    events: Vec<Value>,
    retry: u32,
    rng: u32,
}
impl Host {
    fn ev(&mut self, e: Value) {
        self.events.push(e);
    }
    /// Ability handles are index + 1 (0 is the native null pointer).
    fn ability(&self, a: u32) -> &Value {
        &self.c["abilities"][a as usize - 1]
    }
}
impl ChooseWorld for Host {
    fn retry_time(&mut self) -> Result<u32> {
        Ok(self.retry)
    }
    fn set_retry_time(&mut self, bits: u32) -> Result<()> {
        self.retry = bits;
        Ok(())
    }
    fn world_time(&mut self) -> Result<u32> {
        Ok(u(&self.c["time"]))
    }
    fn primary_member(&mut self) -> Result<bool> {
        if !self.c["body"].as_bool().unwrap() || self.c["membership"].is_null() {
            return Ok(false);
        }
        let r = self.c["primary"].as_bool().unwrap();
        self.ev(json!({"event": "primary", "result": r}));
        Ok(r)
    }
    fn member_count(&mut self) -> Result<i32> {
        if self.c["membership"]["org"] == false {
            return Err(Error::World("null organization".into()));
        }
        let r = self.c["member_count"].as_i64().unwrap() as i32;
        self.ev(json!({"event": "member_count", "result": r}));
        Ok(r)
    }
    fn organization_anchor(&mut self) -> Result<[u32; 2]> {
        self.ev(json!({"event": "anchor"}));
        Ok([u(&self.c["anchor"][0]), u(&self.c["anchor"][1])])
    }
    fn actor_position(&mut self) -> Result<[u32; 2]> {
        Ok([u(&self.c["pos"][0]), u(&self.c["pos"][1])])
    }
    fn engaged(&mut self) -> Result<bool> {
        Ok(u(&self.c["engaged"]) != 0)
    }
    fn roe_rate(&mut self) -> Result<Binary> {
        let v = self.c["roe_rate"].clone();
        self.ev(json!({"event": "roe_rate", "result": v}));
        Ok(f64v(&v))
    }
    fn health(&mut self) -> Result<[u32; 2]> {
        Ok([u(&self.c["health"][0]), u(&self.c["health"][1])])
    }
    fn roe_ratio_threshold(&mut self) -> Result<u32> {
        Ok(u(&self.c["ratio_threshold"]))
    }
    fn roe_value_threshold(&mut self) -> Result<u32> {
        Ok(u(&self.c["value_threshold"]))
    }
    fn roe_value(&mut self, rate_bits: u32) -> Result<Binary> {
        let v = self.c["roe_value"].clone();
        self.ev(json!({"event": "roe_value", "rate": rate_bits, "result": v}));
        Ok(f64v(&v))
    }
    fn roe_retry_delay(&mut self) -> Result<u32> {
        Ok(u(&self.c["retry_delay"]))
    }
    fn target_attacks_us(&mut self, _t: u32) -> Result<bool> {
        let r = self.c["attacks_us"].as_bool().unwrap();
        self.ev(json!({"event": "target_attacks_us", "result": r}));
        Ok(r)
    }
    fn attacker_bonus(&mut self) -> Result<u32> {
        Ok(u(&self.c["attacker_bonus"]))
    }
    fn ability_kind(&mut self, a: u32, context: &TargetContext) -> Result<u32> {
        let r = u(&self.ability(a)["kind"]);
        let point = context.point.unwrap_or([0, 0]);
        self.ev(
            json!({"event": "ability_kind", "ability": a - 1, "context": {
            "kind": context.kind, "actor_ok": true, "target": u32::from(context.target == TARGET),
            "point": point, "word28": 0}, "result": r}),
        );
        Ok(r)
    }
    fn decision_status(&mut self, kind: u32) -> Result<u32> {
        let r = u(&self.c["status_of_kind"][kind.to_string()]);
        self.ev(json!({"event": "decision_status", "kind": kind, "result": r}));
        Ok(r)
    }
    fn organization_busy(&mut self) -> Result<bool> {
        if self.c["membership"].is_null() || self.c["membership"]["org"] == false {
            return Ok(false);
        }
        let r = self.c["org_busy"].as_bool().unwrap();
        self.ev(json!({"event": "organization_busy", "result": r}));
        Ok(r)
    }
    fn can_attack(&mut self, a: u32, _t: u32) -> Result<bool> {
        let r = self.ability(a)["can_attack"].as_bool().unwrap();
        self.ev(json!({"event": "can_attack", "ability": a - 1, "result": r}));
        Ok(r)
    }
    fn score(&mut self, record: &KillDecision) -> Result<Binary> {
        let v = self.ability(record.ability)["score"][record.status.to_string()].clone();
        self.ev(json!({"event": "score", "record": rec(record), "result": v}));
        Ok(f64v(&v))
    }
    fn point_score(&mut self, a: u32) -> Result<Binary> {
        let v = self.ability(a)["point_score"].clone();
        self.ev(json!({"event": "point_score", "ability": a - 1, "result": v}));
        Ok(f64v(&v))
    }
    fn player(&mut self) -> Result<u32> {
        self.ev(json!({"event": "player"}));
        Ok(7)
    }
    fn ability_range(&mut self, a: u32) -> Result<Binary> {
        let v = self.ability(a)["range"].clone();
        self.ev(json!({"event": "ability_range", "ability": a - 1, "result": v}));
        Ok(f64v(&v))
    }
    fn ability_value(&mut self, a: u32, _player: u32) -> Result<Binary> {
        let v = self.ability(a)["value"].clone();
        self.ev(json!({"event": "ability_value", "ability": a - 1, "result": v}));
        Ok(f64v(&v))
    }
    fn world_random(&mut self) -> Result<u32> {
        let wide = u64::from(self.rng) * 0x41c6_4e6d + 0x3039;
        self.rng = wide as u32;
        Ok((wide >> 16) as u32)
    }
    fn priority(&mut self, _t: u32) -> Result<i32> {
        let r = self.c["priority"].as_i64().unwrap() as i32;
        self.ev(json!({"event": "priority", "result": r}));
        Ok(r)
    }
    fn target_position(&mut self, _t: u32) -> Result<[u32; 2]> {
        let p = &self.c["target"]["pos"];
        Ok([u(&p[0]), u(&p[1])])
    }
}
impl KillThinkWorld for Host {
    fn performer(&mut self) -> Result<Option<PerformerState>> {
        if !self.c["performer"].as_bool().unwrap() {
            return Ok(None);
        }
        Ok(Some(PerformerState {
            current: self.c["performer_current"].as_bool().unwrap(),
            stage: u(&self.c["performer_stage"]),
        }))
    }
    fn resolve(&mut self, gtid: u32) -> Result<Option<u32>> {
        let t = &self.c["target"];
        Ok((u(&t["gtid"]) == gtid && t["registered"].as_bool() != Some(false)).then_some(TARGET))
    }
    fn relation(&mut self, _t: u32) -> Result<i32> {
        let r = self.c["relation"].as_i64().unwrap() as i32;
        self.ev(json!({"event": "relation", "result": r}));
        Ok(r)
    }
    fn choose(&mut self, _t: u32, record: &mut KillDecision) -> Result<u32> {
        self.ev(json!({"event": "choose", "before": rec(record)}));
        let w: Vec<u32> = self.c["choose_record"]
            .as_array()
            .unwrap()
            .iter()
            .map(u)
            .collect();
        *record = KillDecision {
            ability: w[0],
            kind: w[1],
            status: w[2],
            score: w[3],
            target: w[4],
            distance2: w[5],
            priority: w[6] as i32,
        };
        Ok(w[2])
    }
    fn apply(&mut self, record: &mut KillDecision) -> Result<()> {
        self.ev(json!({"event": "apply", "before": rec(record)}));
        if let Some(s) = self.c["apply_status"].as_u64() {
            record.status = s as u32;
        }
        Ok(())
    }
    fn in_range(&mut self, _t: u32) -> Result<bool> {
        let r = self.c["in_range"].as_bool().unwrap();
        self.ev(json!({"event": "in_range", "result": r}));
        Ok(r)
    }
    fn approach(&mut self, _t: u32) -> Result<()> {
        self.ev(json!({"event": "approach"}));
        Ok(())
    }
    fn stop(&mut self) -> Result<()> {
        self.ev(json!({"event": "stop"}));
        Ok(())
    }
    fn target_lost(&mut self, gtid: u32) -> Result<()> {
        self.ev(json!({"event": "target_lost", "gtid": gtid}));
        Ok(())
    }
}

#[test]
fn native_unit_kill_decision_and_think_match_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 840);
    let mut faults = 0;
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let name = &c["name"];
        let p: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let mut host = Host {
            c: c.clone(),
            events: vec![],
            retry: u(&c["retry"]),
            rng: u(&c["rng"]),
        };
        let abilities: Vec<Ability> = c["abilities"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, a)| Ability {
                handle: i as u32 + 1,
                enabled: a["enabled"].as_bool().unwrap(),
                flags54: u(&a["flags54"]),
                byte28: u(&a["byte28"]) as u8,
            })
            .collect();
        let result = if c["entry"] == "choose" {
            let target = (!c["target"].is_null()).then_some(TARGET);
            let point = (!c["point"].is_null()).then(|| [u(&c["point"][0]), u(&c["point"][1])]);
            let mut out = KillDecision {
                ability: 0x11,
                kind: 0x22,
                status: 0x33,
                score: 0x44,
                target: 0x55,
                distance2: 0x66,
                priority: 0x77,
            };
            let use_out = c["out"].as_bool().unwrap();
            let r = choose(
                target,
                point,
                use_out.then_some(&mut out),
                c["by_point"].as_bool().unwrap(),
                &abilities,
                &mut host,
                p,
            );
            r.map(|status| {
                // Native records with no candidate keep the 0x55 sentinel
                // target word only through the copy; normalize for compare.
                let mut v = rec(&out);
                if out.target == 0x55 {
                    v[4] = json!(0x55);
                }
                json!({"status": status, "out": if use_out { v } else { Value::Null }})
            })
        } else {
            let mut arena = Arena::default();
            arena.minds.insert(MindId(1), Mind::base(1));
            arena.states.insert(
                StateId(1),
                State {
                    parent: MindId(1),
                    discard_on_suspend: false,
                    status: u(&c["state_status"]),
                },
            );
            let r = think(StateId(1), u(&c["target_gtid"]), &mut arena, &mut host);
            r.map(|wrong| {
                let _ = wrong;
                json!({"state_status": arena.states[&StateId(1)].status})
            })
        };
        let native: Vec<Value> = o["events"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["event"] != "diagnostic")
            .cloned()
            .collect();
        assert_eq!(json!(host.events), json!(native), "{name}");
        if !o["fault"].is_null() {
            assert!(result.is_err(), "{name}");
            faults += 1;
            continue;
        }
        let result = result.unwrap();
        if c["entry"] == "choose" {
            assert_eq!(result["status"], o["status"], "{name}");
            assert_eq!(result["out"], o["out"], "{name}");
        } else {
            assert_eq!(result["state_status"], o["state_status"], "{name}");
        }
        assert_eq!(json!(host.retry), o["retry"], "{name}");
        assert_eq!(json!(host.rng), o["rng"], "{name}");
    }
    assert_eq!(faults, 96);
}
