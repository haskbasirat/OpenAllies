//! 0x57B5A0 / 0x57AEB0 against research/phase8-combat/native-regeneration-fixtures.json.
use aa_body::{F32, properties::ModifierState};
use aa_combat::{
    Result,
    morale::Organization,
    regeneration::{self, BodyHealth, SlotDefinition, World},
};
use aa_numeric::Precision;
use serde_json::{Value, json};

const SPAWNED: u32 = 1000;

struct Adapter {
    case: Value,
    members: Vec<Option<u32>>,
    progress: Vec<F32>,
    spawn_member: bool,
    events: Vec<Value>,
}
fn f(v: &Value) -> F32 {
    F32(v.as_u64().unwrap() as u32)
}
fn label(member: u32) -> Value {
    if member == SPAWNED {
        json!("spawned")
    } else {
        json!(member)
    }
}
impl World for Adapter {
    fn leader_slot(&self, _: &Organization) -> Result<i32> {
        Ok(self.case["leader_slot"].as_i64().unwrap() as i32)
    }
    fn slot_count(&self, _: &Organization) -> Result<i32> {
        Ok(self.members.len() as i32)
    }
    fn member(&self, _: &Organization, slot: i32) -> Result<Option<u32>> {
        Ok(self.members[slot as usize])
    }
    fn body(&self, member: u32) -> Result<Option<BodyHealth>> {
        if member == SPAWNED {
            return Ok(Some(BodyHealth {
                health: F32::ZERO,
                maximum: F32::ZERO,
            }));
        }
        let m = &self.case["members"][member as usize];
        Ok(m["body"].as_bool().unwrap().then(|| BodyHealth {
            health: f(&m["health"]),
            maximum: f(&m["maximum"]),
        }))
    }
    fn member_attached(&self, member: u32) -> Result<bool> {
        Ok(self.case["members"][member as usize]["attached"]
            .as_bool()
            .unwrap())
    }
    fn member_regeneration(&self, member: u32) -> Result<F32> {
        Ok(f(&self.case["members"][member as usize]["regeneration"]))
    }
    fn slot_definition(&self, _: &Organization, slot: i32) -> Result<Option<SlotDefinition>> {
        let d = &self.case["definitions"][slot as usize];
        Ok((!d.is_null()).then(|| SlotDefinition {
            flags_174: d["flags_174"].as_u64().unwrap() as u32,
            regeneration_2cc: f(&d["regeneration_2cc"]),
            health_284: f(&d["health_284"]),
        }))
    }
    fn allegiance_shortage(&mut self, _: &mut Organization) -> Result<F32> {
        Ok(f(&self.case["shortage"]))
    }
    fn modifier(&mut self, _: &mut Organization) -> Result<Option<ModifierState>> {
        Ok(self.case["modifier_3c"].as_u64().map(|bits| {
            let mut state = ModifierState::new(1);
            state.scalars[0x3c / 4 - 1] = F32(bits as u32);
            state
        }))
    }
    fn heal(&mut self, _: &mut Organization, member: u32, amount: F32) -> Result<()> {
        self.events
            .push(json!({"heal": label(member), "source": true, "amount": amount.0}));
        Ok(())
    }
    fn present_regeneration(&mut self, _: &mut Organization, amount: F32) -> Result<()> {
        self.events
            .push(json!({"present": amount.0, "owner": true, "source": true, "object": true}));
        Ok(())
    }
    fn progress(&self, _: &Organization, slot: i32) -> Result<F32> {
        Ok(self.progress[slot as usize])
    }
    fn set_progress(&mut self, _: &mut Organization, slot: i32, value: F32) -> Result<()> {
        self.progress[slot as usize] = value;
        Ok(())
    }
    fn spawn(&mut self, _: &mut Organization, slot: i32) -> Result<bool> {
        self.events.push(json!({"spawn": slot, "flag": 0}));
        let ok = self.case["spawn_result"].as_u64().unwrap() != 0;
        if ok && self.spawn_member {
            self.members[slot as usize] = Some(SPAWNED);
        }
        Ok(ok)
    }
    fn regroup_steal_ratio(&mut self) -> Result<F32> {
        Ok(f(&self.case["ratio"]))
    }
    fn set_health(&mut self, _: &mut Organization, member: u32, health: F32) -> Result<()> {
        self.events
            .push(json!({"set_health": label(member), "value": health.0, "rest": [0, 0, 0]}));
        Ok(())
    }
}

fn organization(primary: Option<u32>) -> Organization {
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
        last_damage: aa_body::damage::DamageRecord {
            source_id: 0,
            allegiance_id: 0,
            damage_type: 0,
            amount: F32::ZERO,
            time: F32::ZERO,
            event_kind: 0,
            reason: 0,
        },
        leader: None,
        primary,
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

#[test]
fn regeneration_and_regroup_match_native() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-regeneration-fixtures.json"
    ))
    .unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 480);
    for row in cases {
        let p: Precision = serde_json::from_value(row["precision"].clone()).unwrap();
        let case = row["case"].clone();
        let members: Vec<Option<u32>> = case["members"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, m)| (!m.is_null()).then_some(i as u32))
            .collect();
        let regroup = row["entry"] == "regroup";
        let mut world = Adapter {
            progress: case["progress"].as_array().unwrap().iter().map(f).collect(),
            members,
            spawn_member: regroup,
            case: case.clone(),
            events: vec![],
        };
        let mut o = organization(case["primary"].as_u64().map(|v| v as u32));
        if regroup {
            let got = regeneration::regroup(&mut o, &mut world, p).unwrap();
            assert_eq!(u64::from(got), row["result"].as_u64().unwrap(), "{row}");
        } else {
            regeneration::regenerate(&mut o, f(&case["delta"]), &mut world, p).unwrap();
        }
        assert_eq!(Value::Array(world.events.clone()), row["events"], "{row}");
        let after: Vec<u64> = world.progress.iter().map(|v| u64::from(v.0)).collect();
        let native: Vec<u64> = row["progress_after"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap())
            .collect();
        assert_eq!(after, native, "{row}");
    }
}
