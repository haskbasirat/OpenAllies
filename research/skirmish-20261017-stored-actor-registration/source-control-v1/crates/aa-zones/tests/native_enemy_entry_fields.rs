//!Exact515620 slot4 ->514DA0/514E40 child/guard replay after field factoring.
use aa_zones::{
    Error, Result,
    tile::ComponentEffect,
    zone::{EnemyEnteredFields, enemy_entered_fields},
};
use serde_json::{Value, json};
struct Fields<'a> {
    input: &'a Value,
    events: Vec<Value>,
    defender: Option<&'static str>,
}
impl EnemyEnteredFields for Fields<'_> {
    type Component = &'static str;
    fn actor_flags(&mut self, _: u32) -> Result<u32> {
        Ok(self.input["flags"].as_u64().unwrap() as u32)
    }
    fn zone_owner(&mut self) -> Result<u32> {
        if !self.input["zone"].as_bool().unwrap() || self.input["count"].as_i64().unwrap() <= 4 {
            return Err(Error::NativeNull("zone4"));
        }
        self.events.push(json!(["owner"]));
        Ok(7)
    }
    fn enemy(&mut self, _: u32, _: u32) -> Result<bool> {
        self.events.push(json!(["enemy"]));
        Ok(self.input["enemy"].as_u64().unwrap() as u8 != 0)
    }
    fn actor_redirect(&mut self, _: u32) -> Result<Option<u32>> {
        Ok((self.input["element"].as_bool().unwrap()
            && self.input["redirect"].as_bool().unwrap()
            && self.input["enabled"].as_u64().unwrap() as u8 != 0)
            .then_some(99))
    }
    fn zone_flags(&mut self) -> Result<u16> {
        Ok(self.input["zone_flags"].as_u64().unwrap() as u16)
    }
    fn component(&mut self, offset: u32) -> Result<Option<Self::Component>> {
        Ok(match offset {
            0x88 => self.input["guard"].as_bool().unwrap().then_some("guard"),
            0x7c => self.input["org"].as_bool().unwrap().then_some("org"),
            0xb0 => self.defender,
            _ => panic!("unexpected component"),
        })
    }
    fn effect(&mut self, e: ComponentEffect, c: Self::Component, target: u32) -> Result<()> {
        let name = match e {
            ComponentEffect::Guard => "guard",
            ComponentEffect::ControlOrganization => "org",
            ComponentEffect::ControlB0 => "defender",
            _ => panic!("unexpected effect"),
        };
        self.events.push(json!([
            name,
            c,
            if target == 99 { "redirect" } else { "actor" }
        ]));
        if self.input["cut"].as_str().unwrap() == name {
            return Err(Error::World(format!("required-{name}")));
        }
        if name == "org" {
            match self.input["mutation"].as_str().unwrap() {
                "clear" => self.defender = None,
                "install" | "replace" => self.defender = Some("replacement"),
                _ => (),
            }
        }
        Ok(())
    }
}
#[test]
fn original_v14_control_entry_guards_redirect_children_and_failure_prefixes() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-stored-actor-registration/native-control-entry.json"
    ))
    .unwrap();
    let rows = fixture["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 72);
    for row in rows {
        let c = &row["input"];
        let mut fields = Fields {
            input: c,
            events: Vec::new(),
            defender: c["defender"].as_bool().unwrap().then_some("defender"),
        };
        let result = if c["leaf"].as_u64().unwrap() & 16 != 0 {
            enemy_entered_fields(&mut fields, 37)
        } else {
            Ok(())
        };
        let terminal = match result {
            Ok(()) => "returned".to_string(),
            Err(Error::NativeNull(_)) => "null-zone".into(),
            Err(Error::World(s)) => s,
            Err(e) => panic!("{e}"),
        };
        assert_eq!(
            json!({"events":fields.events,"terminal":terminal}),
            row["native"],
            "{row}"
        );
    }
}
