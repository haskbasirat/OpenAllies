use aa_body::F32;
use aa_combat::{
    Result,
    ability::Ability,
    constructors::{self, Factory, LocalAbility},
};
use aa_numeric::{Binary, Precision};
use serde_json::{Value, json};
struct World {
    input: Value,
    actions: Vec<Ability>,
    events: Vec<Value>,
    notified: bool,
}
impl Factory for World {
    fn local_ability_count(&self, _: u32) -> Result<usize> {
        Ok(self.input["abilities"].as_array().unwrap().len())
    }
    fn local_ability(&self, _: u32, index: usize) -> Result<LocalAbility> {
        Ok(LocalAbility {
            definition: 100 + index as u32,
            local_definition: 200 + index as u32,
            main_power: serde_json::from_value(
                self.input["abilities"][index]["main_power"].clone(),
            )
            .unwrap(),
        })
    }
    fn allocate_ability(&mut self, a: Ability) -> Result<u32> {
        if self.input["fail_allocation_after"].as_u64() == Some(self.actions.len() as u64) {
            return Err(aa_combat::Error::World("arena exhausted".into()));
        }
        self.events.push(json!({"name":"allocate","ability":a}));
        self.actions.push(a);
        Ok(self.actions.len() as u32)
    }
    fn notify_actor_abilities(&mut self, owner: u32) -> Result<()> {
        self.events.push(json!({"name":"notify","owner":owner}));
        self.notified = true;
        Ok(())
    }
    fn energy(&self, _: u32) -> Result<F32> {
        Ok(serde_json::from_value(
            self.input[if self.notified {
                "energy_after_notify"
            } else {
                "energy"
            }]
            .clone(),
        )
        .unwrap())
    }
    fn turret_count(&self, _: u32) -> Result<usize> {
        Ok(self.input["angles"].as_array().unwrap().len())
    }
    fn turret_center_angle(&self, _: u32, index: usize) -> Result<Binary> {
        Ok(Binary::from_f32_bits(
            self.input["angles"][index].as_u64().unwrap() as u32,
        )?)
    }
    fn startup_target(&self) -> Result<[F32; 2]> {
        Ok(serde_json::from_value(self.input["startup"].clone()).unwrap())
    }
    fn properties_component(&self, _: u32) -> Result<Option<u32>> {
        Ok(self.input["properties"].as_bool().unwrap().then_some(9))
    }
    fn refresh_properties(&mut self, properties: u32, immediate: bool) -> Result<()> {
        self.events
            .push(json!({"name":"refresh","properties":properties,"immediate":immediate}));
        Ok(())
    }
}
#[test]
fn native_performer_and_shared_action_constructors_match() {
    let f: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-performer/native-constructor-fixtures.json"
    ))
    .unwrap();
    for row in f["cases"].as_array().unwrap() {
        let input = row["input"].clone();
        let precision: Precision = serde_json::from_value(input["precision"].clone()).unwrap();
        let mut w = World {
            input,
            actions: vec![],
            events: vec![],
            notified: false,
        };
        let actual = if w.input["kind"] == "projectile" {
            json!(aa_combat::projectile::Projectile::new(
                1,
                serde_json::from_value(w.input["speed"].clone()).unwrap()
            ))
        } else if w.input["kind"] == "shared" {
            let a = Ability::new(
                100,
                None,
                serde_json::from_value(w.input["abilities"][0]["main_power"].clone()).unwrap(),
            );
            w.allocate_ability(a).unwrap();
            json!({"actions":w.actions,"events":w.events})
        } else {
            let owned = constructors::create(1, &mut w, precision).unwrap();
            let ok = constructors::init_create(1, &mut w).unwrap();
            json!({"owned":owned,"actions":w.actions,"events":w.events,"init_return":ok})
        };
        assert_eq!(actual, row["output"], "{}", w.input["name"]);
    }
}

#[test]
fn constructor_allocation_error_keeps_published_actions_and_skips_notification() {
    let f: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-performer/native-constructor-fixtures.json"
    ))
    .unwrap();
    let mut input = f["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["input"]["abilities"].as_array().unwrap().len() == 4)
        .unwrap()["input"]
        .clone();
    input["fail_allocation_after"] = json!(2);
    let mut w = World {
        input,
        actions: vec![],
        events: vec![],
        notified: false,
    };
    assert!(constructors::create(1, &mut w, Precision::Double53).is_err());
    assert_eq!(w.actions.len(), 2);
    assert_eq!(w.events.len(), 2);
    assert!(!w.notified);
}
