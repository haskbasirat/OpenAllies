use aa_body::F32;
use aa_combat::{
    Result,
    ability::Ability,
    fx::{self, Instance, Trigger, World},
    performer::Context,
};
use serde_json::{Value, json};
struct Host {
    input: Value,
    events: Vec<&'static str>,
    instances: Vec<Instance>,
}
impl World for Host {
    fn trigger(&self, _: u32, event: u32) -> Result<Trigger> {
        Ok(serde_json::from_value(self.input["triggers"][event as usize].clone()).unwrap())
    }
    fn suppressed(&self) -> Result<bool> {
        Ok(self.input["suppressed"].as_bool().unwrap())
    }
    fn is_kingdom(&mut self, _: u32) -> Result<bool> {
        self.events.push("kingdom");
        Ok(self.input["source_kind"] == "kingdom")
    }
    fn is_actor(&mut self, _: u32) -> Result<bool> {
        self.events.push("actor");
        Ok(self.input["source_kind"] == "actor")
    }
    fn allegiance(&mut self, _: u32) -> Result<Option<u32>> {
        self.events.push("allegiance");
        if self.input["mutate"] == true {
            self.input["positions"]["1"] =
                json!([9.17f32.to_bits(), (-3.7f32).to_bits(), 4.11f32.to_bits()]);
            self.input["positions"]["2"] =
                json!([(-3.3f32).to_bits(), 17.7f32.to_bits(), 1.919f32.to_bits()]);
        }
        Ok((self.input["owner"] == true).then_some(3))
    }
    fn actor_id(&self, id: u32) -> Result<u32> {
        Ok(0x10000 + id)
    }
    fn actor_position(&self, id: u32) -> Result<[F32; 3]> {
        Ok(serde_json::from_value(self.input["positions"][id.to_string()].clone()).unwrap())
    }
    fn current_is_render_thread(&mut self) -> Result<bool> {
        self.events.push("clock");
        Ok(self.input["render_thread"] == true)
    }
    fn render_time(&self) -> Result<F32> {
        Ok(serde_json::from_value(self.input["render_time"].clone()).unwrap())
    }
    fn world_time(&self) -> Result<F32> {
        Ok(serde_json::from_value(self.input["world_time"].clone()).unwrap())
    }
    fn submit(&mut self, instance: Instance) -> Result<()> {
        self.events.push("submit");
        self.instances.push(instance);
        Ok(())
    }
}
#[test]
fn original_fx_handoff_uses_actual_positions_identity_and_clock_branch() {
    let f: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-fx/native-fx-fixtures.json"
    ))
    .unwrap();
    for row in f["cases"].as_array().unwrap() {
        let i = &row["input"];
        let mut w = Host {
            input: i.clone(),
            events: vec![],
            instances: vec![],
        };
        let mut ability = Ability::new(10, (i["local"] == true).then_some(11), F32::ONE);
        ability.use_count = i["count"].as_u64().unwrap() as u32;
        let context: Context = serde_json::from_value(i["context"].clone()).unwrap();
        for event in 0..4 {
            fx::emit(&ability, event, &context, &mut w).unwrap();
        }
        let actual = json!({"events":w.events,"instances":w.instances});
        assert_eq!(actual, row["output"], "{}", i["name"]);
    }
}
