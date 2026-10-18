pub use aa_actors::{ActorId,components};
#[path="E:/Games/Axis and Alies/rust-rebuild/crates/aa-actors/src/world_publication.rs"] pub mod world_publication;
mod tests{
//Caller oracle only: these effect seams do not certify the Staged bindings.
use crate::{
    ActorId,
    components::ComponentSlot as Slot,
    world_publication::{self, CreateWorld, Error, Result},
};
use serde_json::{Value, json};
struct Probe {
    input: Value,
    trace: Vec<Value>,
    flags: u32,
}
impl Probe {
    fn stop<T>(&self) -> Result<T> {
        Err(Error::Unsupported("required child"))
    }
}
impl CreateWorld for Probe {
    type Record = u32;
    fn has_component(&self, _: ActorId, s: Slot) -> Result<bool> {
        Ok(self.input["present"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v.as_u64() == Some(s.index() as u64)))
    }
    fn map_mode_a8(&self) -> Result<u32> {
        Ok(self.input["mode"].as_u64().unwrap() as u32)
    }
    fn object_ground_post_create(&mut self, _: ActorId, _: &u32) -> Result<()> {
        self.trace.push(json!(["ground"]));
        self.stop()
    }
    fn component_post_create(
        &mut self,
        _: ActorId,
        s: Slot,
        record: &u32,
        flag: u32,
    ) -> Result<()> {
        assert_eq!(u64::from(flag), self.input["flag"].as_u64().unwrap());
        if s == Slot::City && flag as u8 == 0 {
            self.trace.push(json!(["city", record, "component"]));
            return self.stop();
        }
        if s == Slot::Properties {
            for value in self.input["properties"].as_array().unwrap() {
                self.trace.push(json!(["property", value, 0, 0]));
                if self.input["stop_property"].as_u64() == Some(self.trace.len() as u64) {
                    return self.stop();
                }
                self.flags ^= 1;
            }
        }
        Ok(())
    }
    fn stored_owner(&self, _: ActorId) -> Result<Option<ActorId>> {
        Ok(self.input["owner"]
            .as_bool()
            .unwrap()
            .then(|| ActorId::from_raw(0x1234)))
    }
    fn owner_post_create_4e6180(&mut self, _: ActorId, _: ActorId) -> Result<()> {
        self.trace.push(json!(["owner", 0x1234]));
        if self.input["stop_owner"].as_bool().unwrap() {
            return self.stop();
        }
        self.flags |= 1 << 28;
        Ok(())
    }
    fn definition_mask(&self, _: ActorId) -> Result<u32> {
        Ok(self.input["mask"].as_u64().unwrap() as u32)
    }
    fn city_predicate_45d6a0(&mut self, _: ActorId) -> Result<bool> {
        Ok(f32::from_bits(self.input["radius"].as_u64().unwrap() as u32) > 0.0)
    }
    fn city_argument_34(&self, value: &u32) -> Result<u32> {
        Ok(*value)
    }
    fn city_post_create_5644f0(&mut self, _: ActorId, value: u32) -> Result<()> {
        self.trace.push(json!(["city", value, "postpass"]));
        self.stop()
    }
}
#[test]
fn native_fresh_nonnull_params_full_flag_and_partial_effects_all_precision() {
    let fixture: Value = serde_json::from_str(include_str!(
        "E:/Games/Axis and Alies/rust-rebuild/research/skirmish-20261017-fresh-post-create/native-fresh-post-create.json"
    ))
    .unwrap();
    let rows = fixture["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 384);
    for row in rows {
        let mut w = Probe {
            input: row["input"].clone(),
            trace: vec![],
            flags: 0x80000000,
        };
        let argument = w.input["arg34"].as_u64().unwrap() as u32;
        let flag = w.input["flag"].as_u64().unwrap() as u32;
        let result = world_publication::post_create(&mut w, ActorId::from_raw(9), &argument, flag);
        assert_eq!(result.is_ok(), row["output"]["status"] == "ok", "{row}");
        assert_eq!(json!(w.trace), row["output"]["trace"], "{row}");
        assert_eq!(
            u64::from(w.flags),
            row["output"]["flags"].as_u64().unwrap(),
            "{row}"
        );
    }
}

}
