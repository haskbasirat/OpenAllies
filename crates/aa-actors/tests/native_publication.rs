use aa_actors::{
    ActorId,
    components::ComponentSlot,
    world_publication::{self, CreateWorld, GroundPostCreate, Result},
};
use aa_body::credit::Object;
use serde_json::{Value, json};
use std::collections::BTreeSet;
struct World {
    input: Value,
    present: BTreeSet<u8>,
    mask: u32,
    events: Vec<Value>,
    fail: bool,
}
impl CreateWorld for World {
    type Record = ();
    fn has_component(&self, _: ActorId, s: ComponentSlot) -> Result<bool> {
        Ok(self.present.contains(&(s as u8)))
    }
    fn map_mode_a8(&self) -> Result<u32> {
        Ok(self.input["mapmode"].as_u64().unwrap() as u32)
    }
    fn object_ground_post_create(&mut self, _: ActorId, _: &()) -> Result<()> {
        self.events.push(json!({"kind":"ground"}));
        Ok(())
    }
    fn component_post_create(
        &mut self,
        a: ActorId,
        s: ComponentSlot,
        _: &(),
        flag: u32,
    ) -> Result<()> {
        if s == ComponentSlot::Properties {
            self.events.push(json!({"kind":"properties"}));
            return world_publication::ground_post_create(self, a, s);
        }
        self.events
            .push(json!({"kind":"post_create","slot":s as u8,"record":1,"flag":flag}));
        for m in self.input["mutations"].as_array().unwrap() {
            if m["slot"] == s as u8 {
                for x in m["add"].as_array().unwrap() {
                    self.present.insert(x.as_u64().unwrap() as u8);
                }
                for x in m["remove"].as_array().unwrap() {
                    self.present.remove(&(x.as_u64().unwrap() as u8));
                }
            }
        }
        Ok(())
    }
    fn stored_owner(&self, _: ActorId) -> Result<Option<ActorId>> {
        Ok(self.input["owner"]
            .as_bool()
            .unwrap()
            .then_some(ActorId::from_raw(1)))
    }
    fn owner_post_create_4e6180(&mut self, _: ActorId, _: ActorId) -> Result<()> {
        self.events.push(json!({"kind":"owner","owner":1}));
        if self.input["owner_changes_city"].as_bool().unwrap() {
            self.mask = 1 << 29;
        }
        Ok(())
    }
    fn definition_mask(&self, _: ActorId) -> Result<u32> {
        Ok(self.mask)
    }
    fn city_predicate_45d6a0(&mut self, _: ActorId) -> Result<bool> {
        Ok(self.input["city_value"].as_i64().unwrap() > 0)
    }
    fn city_argument_34(&self, _: &()) -> Result<u32> {
        Ok(123)
    }
    fn city_post_create_5644f0(&mut self, _: ActorId, argument: u32) -> Result<()> {
        self.events
            .push(json!({"kind":"city","argument":argument,"zero":0}));
        Ok(())
    }
}
impl GroundPostCreate for World {
    fn initial_properties(&self, _: ActorId) -> Result<Vec<Option<Object>>> {
        Ok(self.input["properties"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().map(|x| Object(x as u32)))
            .collect())
    }
    fn add_initial_property(&mut self, _: ActorId, p: Option<Object>) -> Result<()> {
        self.events
            .push(json!({"kind":"add_property","property":p,"property_kind":0,"source":0}));
        if self.fail {
            return Err(world_publication::Error::World(
                "fixture property failure".into(),
            ));
        }
        Ok(())
    }
}
fn load(c: &Value) -> World {
    World {
        input: c.clone(),
        present: c["present"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_u64().unwrap() as u8)
            .collect(),
        mask: c["mask"].as_u64().unwrap() as u32,
        events: vec![],
        fail: false,
    }
}
#[test]
fn original_post_create_world_owner_city_and_fixed_properties_order_matches() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-actor-live/native-publication.json"
    ))
    .unwrap();
    assert_eq!(doc["cases"].as_array().unwrap().len(), 36);
    for row in doc["cases"].as_array().unwrap() {
        let c = &row["input"];
        let mut world = load(c);
        world_publication::post_create(
            &mut world,
            ActorId::from_raw(3),
            &(),
            c["flag"].as_u64().unwrap() as u32,
        )
        .unwrap();
        assert_eq!(
            json!({"events":world.events,"present":world.present,"mask":world.mask}),
            row["output"]
        );
    }
}
#[test]
fn failed_initial_property_stops_later_publication_without_rollback() {
    let c = json!({"flag":0,"mapmode":0,"owner":true,"owner_changes_city":true,"mask":0,"city_value":0,"present":[6,7],"properties":[1,2],"mutations":[]});
    let mut world = load(&c);
    world.fail = true;
    assert!(world_publication::post_create(&mut world, ActorId::from_raw(3), &(), 0).is_err());
    assert_eq!(world.events.len(), 3);
    assert_eq!(world.events[0]["kind"], "ground");
    assert_eq!(world.events[2]["property"], 1);
    assert_eq!(world.mask, 0);
}
