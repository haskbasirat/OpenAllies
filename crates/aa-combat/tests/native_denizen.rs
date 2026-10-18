//! Replays research/phase8-combat/native-denizen-fixtures.json: original
//! 56AE70/569A80/568D80 (DenizenComponent intruders) under Unicorn.
use aa_body::{F32, damage::DamageRecord};
use aa_combat::{
    Result,
    denizen::{self, World},
    reactions::Membership,
};
use aa_numeric::Precision;
use serde_json::{Value, json};

fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}

struct Host {
    c: Value,
    events: Vec<Value>,
}
impl Host {
    fn view(&self, a: u32) -> &Value {
        &self.c["actors"][a.to_string()]
    }
}
impl World for Host {
    fn resolve(&self, gtid: u32) -> Result<Option<u32>> {
        let tag = gtid & 0xffff;
        let v = self.view(tag);
        Ok(
            (gtid != 0 && !v.is_null() && v["registered"] != false && gtid == 0x10000 * tag + tag)
                .then_some(tag),
        )
    }
    fn ranges(&self, _: u32) -> Result<[F32; 2]> {
        Ok([F32(u(&self.c["range364"])), F32(u(&self.c["range368"]))])
    }
    fn position(&self, a: u32) -> Result<[F32; 2]> {
        let xy = &self.view(a)["xy"];
        Ok([F32(u(&xy[0])), F32(u(&xy[1]))])
    }
    fn organization_member_count(&self, a: u32) -> Result<Option<i32>> {
        Ok(self.view(a)["members"].as_array().map(|m| m.len() as i32))
    }
    fn organization_member(&self, a: u32, index: i32) -> Result<Option<u32>> {
        let m = u(&self.view(a)["members"][index as usize]);
        Ok((m != 0).then_some(m))
    }
    fn membership(&self, a: u32) -> Result<Option<Membership>> {
        let m = &self.view(a)["membership"];
        Ok((!m.is_null()).then(|| Membership {
            organization: (u(&m["organization"]) != 0).then(|| u(&m["organization"])),
            attached: m["attached"] == true,
        }))
    }
    fn has_organization_or_body(&self, a: u32) -> Result<bool> {
        let v = self.view(a);
        Ok(!v["members"].is_null() || v["body"] == true)
    }
    fn flags8(&self, a: u32) -> Result<u32> {
        Ok(self.view(a)["flags8"].as_u64().unwrap_or(0) as u32)
    }
    fn actor_id(&self, a: u32) -> Result<u32> {
        Ok(0x10000 * a + a)
    }
    fn allegiance(&mut self, owner: u32) -> Result<Option<u32>> {
        self.events.push(json!({ "Allegiance": owner }));
        Ok(None) // fixture owner allegiance actor 9 is absent (null)
    }
    fn relation84(&mut self, a: u32, alleg: Option<u32>) -> Result<bool> {
        self.events
            .push(json!({"Relation84": [a, alleg.unwrap_or(0)]}));
        Ok(self.view(a)["relation"] != false)
    }
    fn hostile(&mut self, a: u32, alleg: Option<u32>) -> Result<bool> {
        self.events
            .push(json!({"Hostile140": [a, alleg.unwrap_or(0)]}));
        Ok(self.view(a)["hostile"] != false)
    }
    fn adding_intruder(&mut self, _: u32, _: u32) -> Result<()> {
        self.events.push(json!("AddingIntruder"));
        Ok(())
    }
}

#[test]
fn original_denizen_intruders_match_every_fixture() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-denizen-fixtures.json"
    ))
    .unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 57);
    for row in cases {
        let c = &row["input"];
        let name = format!("{} {}", c["name"], c["precision"]);
        let p: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let mut host = Host {
            c: c.clone(),
            events: vec![],
        };
        let mut intruders: Vec<u32> = c["intruders"].as_array().unwrap().iter().map(u).collect();
        let record = DamageRecord {
            source_id: u(&c["source"]),
            allegiance_id: 0,
            damage_type: 0,
            amount: F32::ZERO,
            time: F32::ZERO,
            event_kind: 0,
            reason: 0,
        };
        denizen::on_damaged(1, &mut intruders, &record, &mut host, p).unwrap();
        let out = &row["output"];
        assert_eq!(json!(intruders), out["intruders"], "{name}");
        assert_eq!(json!(intruders.last()), out["tail"], "{name}");
        assert_eq!(json!(host.events), out["events"], "{name}");
    }
}
