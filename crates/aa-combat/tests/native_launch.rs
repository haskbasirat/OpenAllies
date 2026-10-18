use aa_body::F32;
use aa_combat::{
    Result,
    ability::Ability,
    launch::{self, Local, Source, SpawnRequest, World},
    performer::Context,
};
use aa_numeric::Precision;
use serde_json::{Value, json};
struct Host {
    input: Value,
    events: Vec<Value>,
}
impl World for Host {
    fn projectile_definition(&self, _: u32) -> Result<u32> {
        Ok(123)
    }
    fn local_definition(&self, _: u32) -> Result<Local> {
        Ok(serde_json::from_value(self.input["local_values"].clone()).unwrap())
    }
    fn is_kingdom(&mut self, source: u32) -> Result<bool> {
        self.events.push(json!({"name":"kingdom","actor":source}));
        Ok(self.input["kingdom"].as_bool().unwrap())
    }
    fn source(&self, actor: u32) -> Result<Source> {
        Ok(serde_json::from_value(self.input["sources"][actor.to_string()].clone()).unwrap())
    }
    fn missing_local_definition(&mut self, definition: u32) -> Result<()> {
        self.events
            .push(json!({"name":"missing_local","definition":definition}));
        Ok(())
    }
    fn create_actor(&mut self, request: &SpawnRequest) -> Result<Option<u32>> {
        self.events.push(json!({"name":"create","request":request}));
        if self.input["fail_at"] == "create" {
            return Err(aa_combat::Error::World("factory failure".into()));
        }
        if self.input["mutate_after_create"].as_bool().unwrap() {
            self.input["local_values"]["spawn_position"][2] = json!(7.17f32.to_bits());
            self.input["local_values"]["spawn_bone"] = json!(99);
            self.input["sources"]["2"]["position"] =
                json!([1.7f32.to_bits(), (-2.9f32).to_bits(), 5f32.to_bits()]);
        }
        Ok(self.input["factory_success"]
            .as_bool()
            .unwrap()
            .then_some(7))
    }
    fn target_hit_position(&mut self, actor: u32, direction: [F32; 3]) -> Result<[F32; 3]> {
        self.events
            .push(json!({"name":"surface","actor":actor,"direction":direction}));
        Ok(serde_json::from_value(self.input["surface"].clone()).unwrap())
    }
    fn start_projectile(
        &mut self,
        actor: u32,
        context: &Context,
        _: &Ability,
        bone: u32,
        _: Precision,
    ) -> Result<()> {
        self.events
            .push(json!({"name":"start","actor":actor,"context":context,"bone":bone}));
        if self.input["fail_at"] == "start" {
            return Err(aa_combat::Error::World("activation failure".into()));
        }
        Ok(())
    }
}
#[test]
fn native_projectile_creation_record_spawn_and_start_handoff_match() {
    let f: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-ability/native-launch-fixtures.json"
    ))
    .unwrap();
    for row in f["cases"].as_array().unwrap() {
        let i = &row["input"];
        let mut world = Host {
            input: i.clone(),
            events: vec![],
        };
        let p: Precision = serde_json::from_value(i["precision"].clone()).unwrap();
        let mut context: Context = serde_json::from_value(i["context"].clone()).unwrap();
        let a = Ability::new(10, i["local"].as_bool().unwrap().then_some(11), F32::ONE);
        let result = launch::launch(&a, &mut context, &mut world, p).unwrap();
        let actual = json!({"result":result,"context":context,"events":world.events});
        assert_eq!(actual, row["output"], "{}", i["name"]);
    }
}

#[test]
fn launch_errors_stop_at_factory_boundary_and_preserve_published_actor() {
    let f: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-ability/native-launch-fixtures.json"
    ))
    .unwrap();
    for boundary in ["create", "start"] {
        let mut input = f["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| {
                r["input"]["local"] == true
                    && r["input"]["context"]["target"] == 2
                    && r["input"]["factory_success"] == true
            })
            .unwrap()["input"]
            .clone();
        input["fail_at"] = json!(boundary);
        let mut context: Context = serde_json::from_value(input["context"].clone()).unwrap();
        let initial = context.clone();
        let mut w = Host {
            input,
            events: vec![],
        };
        let ability = Ability::new(10, Some(11), F32::ONE);
        assert!(launch::launch(&ability, &mut context, &mut w, Precision::Double53).is_err());
        let names: Vec<_> = w
            .events
            .iter()
            .map(|e| e["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"create"));
        if boundary == "create" {
            assert!(!names.contains(&"start"));
            assert_eq!(context, initial);
        } else {
            assert_eq!(names.last(), Some(&"start"));
            assert_ne!(context.target_position, initial.target_position);
        }
    }
}
