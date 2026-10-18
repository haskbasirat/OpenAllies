//! Replays research/phase8-combat/native-member-removal-fixtures.json:
//! original 57CD70 (with 578540, 45D580, 578140, 4364C0, 57C800) under Unicorn.
use aa_body::{F32, damage::DamageRecord, properties::ModifierState};
use aa_combat::{
    Result,
    morale::{self, Actor, Mind, Organization, StateDescriptor},
    organization::{self, PropertyEntry, RemovalWorld},
};
use aa_numeric::{Binary, Precision};
use serde_json::{Value, json};

fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}

struct Host {
    c: Value,
    members: Vec<u32>,
    arr50: Vec<u32>,
    events: Vec<Value>,
}
impl Host {
    fn ev(&mut self, name: &str, value: Value) {
        self.events.push(json!({ name: value }));
    }
    fn slot(&self, i: i32) -> &Value {
        &self.c["slots"][i as usize]
    }
    fn mv(&self, member: u32) -> &Value {
        &self.c["slots"][(member - 10) as usize]["member"]
    }
    fn mind_flag(&self, k: &str) -> bool {
        self.c["mind"][k] == true
    }
    fn xy(&self, actor: u32) -> [u32; 2] {
        if actor == 1 {
            return [1f32.to_bits(), 2f32.to_bits()];
        }
        let v = &self.mv(actor)["xy"];
        if v.is_null() {
            [5f32.to_bits(), 6f32.to_bits()]
        } else {
            [u(&v[0]), u(&v[1])]
        }
    }
}
impl morale::World for Host {
    fn session_mode(&self) -> Result<u32> {
        Ok(u(&self.c["mode"]))
    }
    fn actor(&self, actor: u32) -> Result<Actor> {
        assert_eq!(actor, 1);
        Ok(Actor {
            flags8: u(&self.c["owner"]["flags8"]),
            flags100: 0,
            flags104: u(&self.c["owner"]["flags104"]),
            engaged: false,
            has_organization: true,
            membership: None,
        })
    }
    fn mind(&self, _: u32) -> Result<Option<Mind>> {
        let m = &self.c["mind"];
        Ok((!m.is_null()).then(|| Mind {
            word28: u(&m["word28"]),
            word2c: 0,
        }))
    }
    fn mind_declines(&mut self, _: &mut Organization) -> Result<bool> {
        let r = self.mind_flag("declines");
        self.ev("Mind5C", json!(r));
        Ok(r)
    }
    fn mind_exhausted(&mut self, _: &mut Organization) -> Result<()> {
        self.ev("MindExhausted", Value::Null);
        Ok(())
    }
    fn mind_wavering(&mut self, _: &mut Organization) -> Result<()> {
        self.ev("MindWavering", Value::Null);
        Ok(())
    }
    fn set_state(&mut self, _: &mut Organization, s: u8, e: bool) -> Result<()> {
        self.ev("SetState", json!([s, e]));
        Ok(())
    }
    fn modifier(&mut self, _: &mut Organization) -> Result<Option<ModifierState>> {
        Ok(None) // 4CF110 runs natively here and its result is unused
    }
    fn state_group(&mut self, _: &mut Organization, _: u32) -> Result<Option<StateDescriptor>> {
        unreachable!()
    }
    fn actor_value(&mut self, _: &mut Organization, _: u32, _: Precision) -> Result<Binary> {
        unreachable!()
    }
    fn definition4f4(&self, _: u32) -> Result<F32> {
        unreachable!()
    }
    fn organization_maximum(&self, _: u32) -> Result<F32> {
        unreachable!()
    }
    fn member_slots(&self, _: &Organization) -> Result<i32> {
        unreachable!()
    }
    fn engage_target(&mut self, _: &mut Organization) -> Result<Option<u32>> {
        unreachable!()
    }
    fn target_eligible(&mut self, _: u32) -> Result<bool> {
        unreachable!()
    }
    fn award(
        &mut self,
        _: &mut Organization,
        _: u32,
        _: aa_body::experience::Request,
    ) -> Result<()> {
        unreachable!()
    }
}
impl RemovalWorld for Host {
    fn slot_record_live(&self, _: &Organization, slot: i32) -> Result<bool> {
        Ok(self.slot(slot)["record"] == "live")
    }
    fn recruit_release(&mut self, _: &mut Organization, slot: i32) -> Result<()> {
        self.ev("RecruitRelease", json!([format!("record:{slot}")]));
        Ok(())
    }
    fn member_definition_properties(&self, member: u32) -> Result<Vec<u32>> {
        Ok(self.mv(member)["def_properties"]
            .as_array()
            .map(|a| a.iter().map(u).collect())
            .unwrap_or_default())
    }
    fn remove_property(
        &mut self,
        _: &mut Organization,
        actor: u32,
        property: u32,
        source: u32,
    ) -> Result<()> {
        self.ev("RemoveProperty", json!([actor, property, source]));
        Ok(())
    }
    fn owner_property_count(&self, _: u32) -> Result<i32> {
        Ok(self.c["owner"]["properties"].as_array().unwrap().len() as i32)
    }
    fn owner_property(&self, _: u32, i: i32) -> Result<PropertyEntry> {
        let e = &self.c["owner"]["properties"][i as usize];
        Ok(PropertyEntry {
            kind: u(&e[0]),
            property: u(&e[1]),
            disabled: u(&e[2]) != 0,
        })
    }
    fn member_has_properties(&self, member: u32) -> Result<bool> {
        Ok(self.mv(member)["properties"] != false)
    }
    fn member_a4(&self, member: u32) -> Result<bool> {
        Ok(self.mv(member)["a4"] == true)
    }
    fn detach(&mut self, _: &mut Organization, _: u32, slot: i32) -> Result<()> {
        self.ev("Detach", json!([1, slot]));
        Ok(())
    }
    fn clear_slot(&mut self, _: &mut Organization, slot: i32) -> Result<()> {
        self.members[slot as usize] = 0;
        self.arr50[slot as usize] = 0;
        Ok(())
    }
    fn morale_value(&self, member: u32) -> Result<F32> {
        Ok(F32(self.mv(member)["morale_value"]
            .as_u64()
            .map_or(4f32.to_bits(), |v| v as u32)))
    }
    fn living_slot(&self, _: &Organization, slot: i32) -> Result<bool> {
        Ok(self.slot(slot)["living"] == true)
    }
    fn member_value6c(&self, member: u32) -> Result<Option<F32>> {
        Ok(self.mv(member)["comp6c"].as_u64().map(|v| F32(v as u32)))
    }
    fn recompute_6c(&mut self, _: &mut Organization) -> Result<()> {
        self.ev("Recompute6C", Value::Null);
        Ok(())
    }
    fn member_flags(&self, member: u32) -> Result<[u32; 2]> {
        let m = self.mv(member);
        Ok([
            m["flags100"].as_u64().unwrap_or(0) as u32,
            m["flags104"].as_u64().unwrap_or(0) as u32,
        ])
    }
    fn member_flags8(&self, member: u32) -> Result<u32> {
        Ok(self.mv(member)["flags8"].as_u64().unwrap_or(0) as u32)
    }
    fn mind_primary_lost(&mut self, _: &mut Organization) -> Result<()> {
        self.ev("PrimaryLost", json!(false));
        Ok(())
    }
    fn mind_leader_changed(&mut self, _: &mut Organization) -> Result<()> {
        self.ev("LeaderChanged", json!(false));
        Ok(())
    }
    fn member_count(&self, _: &Organization) -> Result<i32> {
        Ok(self.members.len() as i32)
    }
    fn member(&self, _: &Organization, i: i32) -> Result<Option<u32>> {
        let m = self.members[i as usize];
        Ok((m != 0).then_some(m))
    }
    fn member_attached(&self, member: u32) -> Result<bool> {
        Ok(self.mv(member)["attached"] != false)
    }
    fn member_slot(&self, member: u32) -> Result<i32> {
        Ok((member - 10) as i32)
    }
    fn slot_refresh(&mut self, _: &mut Organization, slot: i32) -> Result<()> {
        self.ev("SlotRefresh", json!(slot));
        Ok(())
    }
    /// Event-level replica of the proven 57C800 (organization::follow_leader).
    fn follow_leader(&mut self, _: &mut Organization, leader: u32) -> Result<()> {
        if u(&self.c["mode"]) == 6 {
            return Ok(());
        }
        let both = u(&self.c["owner"]["flags8"]) & 0x4000_0000 != 0
            && self.mv(leader)["flags8"].as_u64().unwrap_or(0) as u32 & 0x4000_0000 != 0;
        if !both {
            if !self.c["mind"].is_null() {
                let b64 = self.mind_flag("busy64");
                self.ev("Mind64", json!(b64));
                let busy = b64 || {
                    let b7c = self.mind_flag("busy7c");
                    self.ev("Mind7C", json!(b7c));
                    b7c
                };
                if !busy {
                    return Ok(());
                }
            } else {
                return Ok(());
            }
            if self.xy(1) == self.xy(leader) {
                return Ok(());
            }
        }
        let xy = self.xy(leader);
        self.ev("MoveToLeader", json!(xy));
        Ok(())
    }
    fn update_4cf0(&mut self, _: &mut Organization) -> Result<()> {
        self.ev("Update4CF0", Value::Null);
        Ok(())
    }
    fn update_0f0(&mut self, _: &mut Organization) -> Result<()> {
        self.ev("Update0F0", Value::Null);
        Ok(())
    }
    fn notify_owner(&mut self, _: &mut Organization) -> Result<()> {
        self.ev("Notify", json!(1));
        Ok(())
    }
    fn definition18(&self, _: &Organization) -> Result<i32> {
        Ok(self.c["def18"].as_i64().unwrap() as i32)
    }
    fn destroy_owner(&mut self, _: &mut Organization) -> Result<()> {
        self.ev("Destroy", json!(1));
        Ok(())
    }
    fn begin_owner_death(&mut self, _: &mut Organization) -> Result<()> {
        self.ev("BeginDeath", json!(1));
        Ok(())
    }
    fn request_owner_removal(&mut self, _: &mut Organization) -> Result<()> {
        self.ev("RequestRemoval", json!([1, 0]));
        Ok(())
    }
}

#[test]
fn original_member_removal_matches_every_fixture() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-member-removal-fixtures.json"
    ))
    .unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert!(cases.len() >= 110);
    for row in cases {
        let c = &row["input"];
        let out = &row["output"];
        let name = format!("{} {}", c["name"], c["precision"]);
        let p: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let slots = c["slots"].as_array().unwrap();
        let members: Vec<u32> = slots
            .iter()
            .enumerate()
            .map(|(i, s)| {
                if s["member"].is_null() {
                    0
                } else {
                    10 + i as u32
                }
            })
            .collect();
        let o_in = &c["org"];
        let mut o = Organization {
            owner: 1,
            morale: F32(u(&o_in["morale"])),
            maximum: F32(u(&o_in["maximum"])),
            breakpoint: F32(u(&o_in["breakpoint"])),
            living: o_in["living"].as_i64().unwrap() as i32,
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
            leader: c["leader"].as_u64().map(|i| 10 + i as u32),
            primary: c["primary"].as_u64().map(|i| 10 + i as u32),
            regroup_started: F32::ZERO,
            entrench_base: F32::ZERO,
            flare_pending: false,
            flare_word: 0,
            marker_pending: false,
            watched_target: 0,
            current_combat_value: F32::ZERO,
            maximum_combat_value: F32::ZERO,
            field_d8: F32(u(&o_in["d8"])),
        };
        let mut host = Host {
            c: c.clone(),
            arr50: vec![0x5050; slots.len()],
            members,
            events: vec![],
        };
        let member = c["remove"].as_u64().map(|i| 10 + i as u32);
        organization::remove_member(
            &mut o,
            u(&c["slot"]) as i32,
            member,
            F32(u(&c["death_factor"])),
            &mut host,
            p,
        )
        .unwrap();
        assert_eq!(json!(host.events), out["events"], "{name}");
        assert_eq!(o.morale.0, u(&out["morale"]), "{name}");
        assert_eq!(o.living as u32, u(&out["living"]), "{name}");
        assert_eq!(json!(o.leader.unwrap_or(0)), out["leader"], "{name}");
        assert_eq!(json!(o.primary.unwrap_or(0)), out["primary"], "{name}");
        assert_eq!(json!(host.members), out["members"], "{name}");
        assert_eq!(json!(host.arr50), out["arr50"], "{name}");
    }
}
