//! Replays research/phase8-combat/native-org-tick-fixtures.json: original
//! 57C8E0 with 4D7DA0, 5788B0/57DC40, 579110, 4D7CE0, 57C800, 5147C0, 58B150.
use aa_body::{F32, damage::DamageRecord};
use aa_combat::{
    Result,
    morale::{Actor, Mind, Organization, StateDescriptor},
    organization::{self, MemberCombat, OwnerObjects, SlotDefinition, World},
};
use aa_numeric::{Binary, Precision};
use serde_json::{Value, json};

fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}

struct Host {
    c: Value,
    events: Vec<Value>,
}
impl Host {
    fn ev(&mut self, name: &str, value: Value) {
        self.events.push(json!({ name: value }));
    }
    fn mind_flag(&self, key: &str) -> bool {
        self.c["mind"][key] == true
    }
}
impl World for Host {
    fn session_mode(&self) -> Result<u32> {
        Ok(u(&self.c["mode"]))
    }
    fn actor(&self, actor: u32) -> Result<Actor> {
        let o = &self.c["owner"];
        let flags8 = match actor {
            1 => u(&o["flags8"]),
            2 => u(&self.c["leader"]["flags8"]),
            3 => u(&self.c["primary"]),
            _ => 0,
        };
        Ok(Actor {
            flags8,
            flags100: if actor == 1 { u(&o["flags100"]) } else { 0 },
            flags104: 0,
            engaged: actor == 1 && o["engaged"] == true,
            has_organization: actor == 1 && o["organization"] == true,
            membership: None,
        })
    }
    fn mind(&self, actor: u32) -> Result<Option<Mind>> {
        assert_eq!(actor, 1);
        Ok((!self.c["mind"].is_null()).then_some(Mind {
            word28: 0,
            word2c: 0,
        }))
    }
    fn position(&self, actor: u32) -> Result<[F32; 2]> {
        let xy = if actor == 1 {
            &self.c["owner"]["xy"]
        } else {
            &self.c["leader"]["xy"]
        };
        Ok([F32(u(&xy[0])), F32(u(&xy[1]))])
    }
    fn now(&self) -> Result<F32> {
        Ok(F32(u(&self.c["now"])))
    }
    fn set_state(&mut self, _: &mut Organization, state: u8, enabled: bool) -> Result<()> {
        self.ev("SetState", json!([state, enabled]));
        Ok(())
    }
    fn state_group(&mut self, _: &mut Organization, group: u32) -> Result<Option<StateDescriptor>> {
        self.ev("StateGroup", json!(group));
        let g = &self.c["group"];
        Ok((!g.is_null()).then(|| StateDescriptor {
            byte1c: g["byte1c"] != 0,
            byte1d: g["byte1d"] != 0,
            byte1e: g["byte1e"] != 0,
        }))
    }
    fn notify(&mut self, _: &mut Organization, actor: u32) -> Result<()> {
        self.ev("Notify", json!(actor));
        Ok(())
    }
    fn morale_tick(&mut self, _: &mut Organization, d: F32) -> Result<()> {
        self.ev("MoraleTick", json!(d.0));
        Ok(())
    }
    fn engagement_experience(&mut self, _: &mut Organization, d: F32) -> Result<()> {
        self.ev("EngagementExperience", json!(d.0));
        Ok(())
    }
    fn attack_efficiency(&mut self, _: &mut Organization, d: F32) -> Result<()> {
        self.ev("AttackEfficiency", json!(d.0));
        Ok(())
    }
    fn shortfall(&mut self, _: &mut Organization, d: F32) -> Result<()> {
        self.ev("Shortfall", json!(d.0));
        Ok(())
    }
    fn regenerate(&mut self, _: &mut Organization, d: F32) -> Result<()> {
        self.ev("Regenerate", json!(d.0));
        Ok(())
    }
    fn regroup_check(&mut self, _: &mut Organization) -> Result<bool> {
        let r = self.c["regroup_ok"] == true;
        self.ev("RegroupCheck", json!(r));
        Ok(r)
    }
    fn unable_to_regroup(&mut self, _: &mut Organization) -> Result<()> {
        self.ev("UnableToRegroup", Value::Null);
        Ok(())
    }
    fn mind_busy64(&mut self, _: &mut Organization) -> Result<bool> {
        let r = self.mind_flag("busy64");
        self.ev("Mind64", json!(r));
        Ok(r)
    }
    fn mind_busy7c(&mut self, _: &mut Organization) -> Result<bool> {
        let r = self.mind_flag("busy7c");
        self.ev("Mind7C", json!(r));
        Ok(r)
    }
    fn mind_regroup(&mut self, _: &mut Organization) -> Result<()> {
        self.ev("Mind150", json!(false));
        Ok(())
    }
    fn member_count(&self, _: &Organization) -> Result<i32> {
        Ok(self.c["members"].as_array().unwrap().len() as i32)
    }
    fn member(&self, _: &Organization, index: i32) -> Result<Option<u32>> {
        let m = &self.c["members"][index as usize];
        Ok((m["kind"] == "live").then_some(10 + index as u32))
    }
    fn member_combat(&self, member: u32) -> Result<Option<MemberCombat>> {
        let m = &self.c["members"][(member - 10) as usize];
        if m["combat"].is_null() {
            return Ok(None);
        }
        Ok(Some(MemberCombat {
            value: F32(u(&m["combat"])),
            health: (!m["body"].is_null()).then(|| [F32(u(&m["body"][0])), F32(u(&m["body"][1]))]),
        }))
    }
    fn slot_definition(&self, _: &Organization, index: i32) -> Result<Option<SlotDefinition>> {
        let m = &self.c["members"][index as usize];
        Ok((!m["slot"].is_null()).then(|| SlotDefinition {
            flags174: u(&m["slot"][0]),
            value1e4: F32(u(&m["slot"][1])),
        }))
    }
    fn definition_flag2f4(&self, _: u32) -> Result<bool> {
        Ok(self.c["def2f4"] == true)
    }
    fn in_supply(&mut self, _: &mut Organization) -> Result<bool> {
        let r = self.c["in_supply"] == true;
        self.ev("InSupply", json!(r));
        Ok(r)
    }
    fn allegiance_shortage(&mut self, _: &mut Organization) -> Result<Option<F32>> {
        let present = self.c["allegiance"] == true;
        self.ev(
            "Allegiance",
            if present {
                json!("allegiance")
            } else {
                json!(0)
            },
        );
        Ok(present.then(|| F32(u(&self.c["shortage"]))))
    }
    fn owner_objects(&self, _: u32) -> Result<OwnerObjects> {
        Ok(OwnerObjects {
            count: self.c["objects_count"].as_i64().unwrap() as i32,
            object10: self.c["object10"].as_u64().map(|w| (99, F32(w as u32))),
        })
    }
    fn flare(
        &mut self,
        _: &mut Organization,
        target: Option<u32>,
        p: [F32; 2],
        word: u32,
    ) -> Result<()> {
        let t = if target.is_some() {
            json!("object10")
        } else {
            json!(0)
        };
        self.ev("Flare", json!([t, p[0].0, p[1].0, word]));
        Ok(())
    }
    fn marker(&mut self, _: &mut Organization, object: u32) -> Result<()> {
        assert_eq!(object, 99);
        self.ev("Marker", json!("object10"));
        Ok(())
    }
    fn move_to_leader(&mut self, _: &mut Organization, p: [F32; 2]) -> Result<()> {
        self.ev("MoveToLeader", json!([p[0].0, p[1].0]));
        Ok(())
    }
    fn entrench_time(&mut self, _: &mut Organization) -> Result<Binary> {
        let tag = if self.c["owner"]["properties"] == true {
            json!("modifier")
        } else {
            json!(0)
        };
        self.ev("EntrenchTime", tag);
        Ok(Binary::from_f32_bits(u(&self.c["entrench_time"])).unwrap())
    }
    fn watched_target(&self, gtid: u32) -> Result<Option<(bool, bool)>> {
        assert_eq!(gtid, 0x40004);
        Ok(match self.c["d4"].as_str().unwrap() {
            "stale" => None,
            "destructed" => Some((true, false)),
            "complete" => Some((false, true)),
            _ => Some((false, false)),
        })
    }
}

fn organization(c: &Value) -> Organization {
    Organization {
        owner: 1,
        morale: F32::ZERO,
        maximum: F32::ZERO,
        breakpoint: F32::ZERO,
        living: 0,
        engaged_latch: false,
        engage_latch: false,
        attack_efficiency: F32::ZERO,
        attack_efficiency_desired: F32::ZERO,
        byte8d: false,
        last_damage: DamageRecord {
            source_id: 0,
            allegiance_id: 0,
            damage_type: 0,
            amount: F32::ZERO,
            time: F32::ZERO,
            event_kind: 0,
            reason: 10,
        },
        leader: (!c["leader"].is_null()).then_some(2),
        primary: (!c["primary"].is_null()).then_some(3),
        regroup_started: F32(u(&c["regroup_started"])),
        entrench_base: F32(u(&c["entrench_base"])),
        flare_pending: c["b0"] == true,
        flare_word: 0x5151,
        marker_pending: c["b8e"] == true,
        watched_target: if c["d4"].is_null() { 0 } else { 0x40004 },
        current_combat_value: F32(0x1234),
        maximum_combat_value: F32(0x5678),
        field_d8: F32::ZERO,
    }
}

#[test]
fn original_organization_tick_matches_every_fixture() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-org-tick-fixtures.json"
    ))
    .unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert!(cases.len() >= 180);
    for row in cases {
        let c = &row["input"];
        let out = &row["output"];
        let name = format!("{} {}", c["name"], c["precision"]);
        let p: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let mut o = organization(c);
        let mut host = Host {
            c: c.clone(),
            events: vec![],
        };
        organization::tick_maintenance(
            &mut o,
            F32(u(&c["delta"])),
            F32(u(&c["regroup_time"])),
            &mut host,
            p,
        )
        .unwrap();
        assert_eq!(json!(host.events), out["events"], "{name}");
        if c["leader"].is_null() {
            continue; // native returns before 5788B0; fields untouched
        }
        assert_eq!(o.current_combat_value.0, u(&out["current_value"]), "{name}");
        assert_eq!(o.maximum_combat_value.0, u(&out["maximum_value"]), "{name}");
        assert_eq!(o.flare_pending as u32, u(&out["b0"]), "{name}");
        assert_eq!(o.marker_pending as u32, u(&out["b8e"]), "{name}");
        assert_eq!(o.watched_target, u(&out["d4"]), "{name}");
    }
}
