//! Replays research/phase8-combat/native-damage-response-fixtures.json:
//! original 4D1120, 4F1A90, 584E70, 4F65B0 (empty removal list), 584CD0,
//! 579490 and 578660/562D10 executed under Unicorn.
use aa_body::{
    F32,
    damage::{DamageRecord, DamageRequest},
};
use aa_combat::{
    Error, Result, capture,
    death::{self, DeathQueue},
    engagement,
    morale::Organization,
    shortfall::{self, MemberBody},
};
use aa_numeric::Precision;
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn words(v: &Value) -> DamageRecord {
    let w: Vec<u32> = v
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_u64().unwrap() as u32)
        .collect();
    DamageRecord {
        source_id: w[0],
        allegiance_id: w[1],
        damage_type: w[2],
        amount: F32(w[3]),
        time: F32(w[4]),
        event_kind: w[5],
        reason: w[6],
    }
}
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}

// ---------------------------------------------------------------- capture
struct CaptureHost(Vec<Value>);
impl capture::World for CaptureHost {
    fn capture_view(&self, actor: u32) -> Result<capture::CaptureView> {
        let v = &self.0[actor as usize - 1];
        Ok(capture::CaptureView {
            segment_owner: (v["component98"] == true).then(|| {
                let o = u(&v["component98_owner"]);
                (o != 0).then_some(o)
            }),
            flags100: u(&v["flags100"]),
            modifier_flags: v["properties"].as_u64().map(|x| x as u32),
            component94_target: (!v["component94"].is_null()).then_some(0x50000 + actor - 1),
            component_d4: v["d4"] == true,
            captureable: u(&v["captureable"]) as u8,
        })
    }
    fn resolve(&self, gtid: u32) -> Result<Option<u32>> {
        let v = &self.0[(gtid - 0x50000) as usize];
        Ok((v["component94"] == "live").then_some(99))
    }
}

// ---------------------------------------------------------------- queue
#[derive(Default)]
struct Queue(std::collections::VecDeque<u32>);
impl DeathQueue for Queue {
    fn push_front(&mut self, actor: u32) -> Result<()> {
        self.0.push_front(actor);
        Ok(())
    }
}

// ---------------------------------------------------------------- engagement
struct Engage {
    input: Value,
    events: Vec<Value>,
}
const OWNER: u32 = 1;
const ELEMENT: u32 = 2;
const ATTACKER: u32 = 3;
impl engagement::World for Engage {
    fn resolve(&self, gtid: u32) -> Result<Option<u32>> {
        Ok((gtid == 0x30003 && self.input["attacker_registered"] == true).then_some(ATTACKER))
    }
    fn has_mind(&self, actor: u32) -> Result<bool> {
        assert_eq!(actor, OWNER);
        Ok(!self.input["mind"].is_null())
    }
    fn can_engage(&mut self, owner: u32, attacker: u32) -> Result<bool> {
        assert_eq!(owner, OWNER);
        self.events.push(json!({"CanEngage": attacker}));
        Ok(self.input["can_engage"] == true)
    }
    fn mind_busy64(&mut self, _: u32) -> Result<bool> {
        let r = self.input["mind"]["busy64"] == true;
        self.events.push(json!({"busy64": r}));
        Ok(r)
    }
    fn mind_busy7c(&mut self, _: u32) -> Result<bool> {
        let r = self.input["mind"]["busy7c"] == true;
        self.events.push(json!({"busy7c": r}));
        Ok(r)
    }
    fn position(&self, actor: u32) -> Result<[F32; 2]> {
        let key = if actor == OWNER {
            "owner_xy"
        } else {
            "attacker_xy"
        };
        Ok([F32(u(&self.input[key][0])), F32(u(&self.input[key][1]))])
    }
    fn operating_radius_squared(&self, _: u32) -> Result<F32> {
        Ok(F32(u(&self.input["radius_squared"])))
    }
    fn element_attached(&self, element: u32) -> Result<bool> {
        assert_eq!(element, ELEMENT);
        Ok(self.input["element_membership"] == true)
    }
    fn flags100(&self, actor: u32) -> Result<u32> {
        assert_eq!(actor, OWNER);
        Ok(u(&self.input["owner_flags100"]))
    }
    fn add_engage_target(&mut self, _: u32, attacker: u32) -> Result<()> {
        self.events.push(json!({"AddEngageTarget": attacker}));
        Ok(())
    }
    fn outside_operating_radius(&mut self, _: u32, _: u32) -> Result<()> {
        self.events.push(json!("OutsideOperatingRadiusDiagnostic"));
        Ok(())
    }
}

fn organization(byte8d: bool) -> Organization {
    Organization {
        owner: OWNER,
        morale: F32::ZERO,
        maximum: F32::ONE,
        breakpoint: F32::ZERO,
        living: 0,
        engaged_latch: false,
        engage_latch: false,
        attack_efficiency: F32::ONE,
        attack_efficiency_desired: F32::ONE,
        byte8d,
        last_damage: words(&json!([
            0xdead0000u32,
            0xdead0001u32,
            0xdead0002u32,
            0xdead0003u32,
            0xdead0004u32,
            0xdead0005u32,
            0xdead0006u32
        ])),
        leader: None,
        primary: None,
        regroup_started: F32::ZERO,
        entrench_base: F32::ZERO,
        flare_pending: false,
        flare_word: 0,
        marker_pending: false,
        watched_target: 0,
        current_combat_value: F32::ZERO,
        maximum_combat_value: F32::ZERO,
        field_d8: F32::ZERO,
    }
}

// ---------------------------------------------------------------- shortfall
struct Short {
    members: Vec<Value>,
    count: i32,
    draws: Vec<u32>,
    removals: Vec<u32>,
    now: F32,
    events: Vec<Value>,
}
impl shortfall::World for Short {
    fn member_count(&self) -> Result<i32> {
        Ok(self.count)
    }
    fn member(&self, index: i32) -> Result<Option<u32>> {
        Ok((!self.members[index as usize].is_null()).then_some(10 + index as u32))
    }
    fn body(&self, member: u32) -> Result<Option<MemberBody>> {
        let v = &self.members[(member - 10) as usize];
        if v["body"].is_null() {
            return Ok(None);
        }
        if v["body_owner"] != true {
            return Err(Error::Invalid("shortfall Body without owner"));
        }
        Ok(Some(MemberBody {
            owner_id: 0x10000 + member,
            maximum: F32(u(&v["body"])),
        }))
    }
    fn draw_random(&mut self) -> Result<u32> {
        let value = self.draws.remove(0);
        self.events.push(json!("RandomSource"));
        self.events.push(json!({"Random": value}));
        Ok(value)
    }
    fn allegiance_id(&mut self, member: u32) -> Result<u32> {
        self.events.push(json!("Allegiance"));
        Ok(
            if self.members[(member - 10) as usize]["allegiance"] == true {
                0x7000a
            } else {
                0
            },
        )
    }
    fn now(&self) -> Result<F32> {
        Ok(self.now)
    }
    fn deal_damage(&mut self, member: u32, r: DamageRequest, _: Precision) -> Result<()> {
        self.events.push(json!({"DealDamage": {
            "body": member, "record": r.record.words(),
            "apply_type_modifier": r.apply_type_modifier as u8,
            "weapon": r.weapon_id.unwrap_or(0), "propagate": r.propagate_to_neighbors as u8}}));
        if self.removals.first() == Some(&member) {
            self.removals.remove(0);
            self.count -= 1;
        }
        Ok(())
    }
}

fn queue_from(input: &Value) -> Queue {
    let mut q = Queue::default();
    for tag in input["queue"].as_array().unwrap() {
        death::queue_death(u(tag), &mut q).unwrap();
    }
    q
}
fn queue_output(q: &Queue) -> Value {
    json!({"queue": q.0, "tail": q.0.back().copied().unwrap_or(0)})
}

#[test]
fn original_damage_responses_match_every_fixture() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-damage-response-fixtures.json"
    ))
    .unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert!(cases.len() >= 190);
    let mut seen = BTreeMap::new();
    for row in cases {
        let i = &row["input"];
        let out = &row["output"];
        let name = format!("{} {}", i["name"], i["precision"]);
        let p: Precision = serde_json::from_value(i["precision"].clone()).unwrap();
        let op = i["operation"].as_str().unwrap();
        *seen.entry(op.to_string()).or_insert(0) += 1;
        match op {
            "capture_eligible" => {
                let host = CaptureHost(i["chain"].as_array().unwrap().clone());
                let r = capture::eligible(1, &host).unwrap();
                assert_eq!(r, out["result"] != 0, "{name}");
            }
            "queue_death" => {
                let mut q = queue_from(i);
                death::queue_death(u(&i["actor"]), &mut q).unwrap();
                assert_eq!(queue_output(&q), *out, "{name}");
            }
            "wall_death" => {
                let mut q = queue_from(i);
                let segments: Vec<Option<u32>> = i["segments"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|s| (u(s) != 0).then_some(u(s)))
                    .collect();
                death::wall_member_killed(
                    u(&i["actor"]),
                    i["all_segments"] == true,
                    &segments,
                    &mut q,
                )
                .unwrap();
                assert_eq!(queue_output(&q), *out, "{name}");
            }
            // The +100 drain (4F65B0) belongs to aa_world::World::transition,
            // proven by research/phase8-tick; these cases stay as cross-evidence.
            "drain" => {}
            "outer_record" => {
                let mut last = words(&json!([1, 2, 3, 4, 5, 6, 7]));
                death::outer_damaged(&mut last, &words(&i["record"]));
                assert_eq!(json!(last.words()), out["record"], "{name}");
            }
            "element_damaged" => {
                let mut o = organization(i["byte8d"] == true);
                let mut host = Engage {
                    input: i.clone(),
                    events: vec![],
                };
                let r =
                    engagement::element_damaged(&mut o, ELEMENT, words(&i["record"]), &mut host, p)
                        .unwrap();
                assert_eq!(json!(r as u8), out["result"], "{name}");
                assert_eq!(json!(o.last_damage.words()), out["last_damage"], "{name}");
                assert_eq!(json!(host.events), out["events"], "{name}");
            }
            "shortfall" => {
                let members = i["members"].as_array().unwrap().clone();
                let mut host = Short {
                    count: members.len() as i32,
                    members,
                    draws: i["draws"].as_array().unwrap().iter().map(u).collect(),
                    removals: i["remove_after"]
                        .as_array()
                        .map(|a| a.iter().map(u).collect())
                        .unwrap_or_default(),
                    now: F32(u(&i["now"])),
                    events: vec![],
                };
                shortfall::tick(F32(u(&i["delta"])), F32(u(&i["rate"])), &mut host, p).unwrap();
                assert!(host.draws.is_empty() && host.removals.is_empty());
                assert_eq!(json!(host.events), out["events"], "{name}");
            }
            other => panic!("{other}"),
        }
    }
    assert_eq!(seen.len(), 7, "{seen:?}");
}

#[test]
fn capture_segment_cycle_is_an_explicit_error() {
    let chain = vec![
        json!({"component98": true, "component98_owner": 2, "flags100": 0, "properties": null,
               "component94": null, "d4": false, "captureable": 1}),
        json!({"component98": true, "component98_owner": 1, "flags100": 0, "properties": null,
               "component94": null, "d4": false, "captureable": 1}),
    ];
    assert!(matches!(
        capture::eligible(1, &CaptureHost(chain)),
        Err(Error::Invalid(_))
    ));
}
