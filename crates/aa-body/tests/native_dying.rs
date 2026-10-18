use aa_body::{Body, Environment, F32, dying::*};
use aa_numeric::{Binary, Precision};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Clone, Deserialize)]
struct Input {
    name: String,
    precision: Precision,
    body: Body,
    environment: Environment,
    retained: bool,
    definition: Definition,
    actor_flags: u32,
    modifier_present: bool,
    owner_present: bool,
    heights: Vec<[F32; 2]>,
    dimensions: Vec<[F32; 2]>,
    mutations: Vec<Value>,
}
struct Host {
    input: Input,
    events: Vec<Value>,
    height_index: usize,
    dimension_index: usize,
    fail_at: Option<&'static str>,
}
impl Host {
    fn event(
        &mut self,
        event: Value,
        body: &mut Body,
        env: &mut Environment,
    ) -> Result<(), String> {
        let name = event
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| event.as_object().unwrap().keys().next().unwrap().clone());
        self.events.push(json!({"event":event,"body":body}));
        if self
            .input
            .mutations
            .first()
            .is_some_and(|m| m["on"] == name)
        {
            let mutation = self.input.mutations.remove(0);
            if let Some(patch) = mutation.get("body") {
                let mut value = serde_json::to_value(&*body).unwrap();
                value
                    .as_object_mut()
                    .unwrap()
                    .extend(patch.as_object().unwrap().clone());
                *body = serde_json::from_value(value).unwrap();
            }
            if let Some(patch) = mutation.get("environment") {
                let mut value = serde_json::to_value(&*env).unwrap();
                value
                    .as_object_mut()
                    .unwrap()
                    .extend(patch.as_object().unwrap().clone());
                *env = serde_json::from_value(value).unwrap();
            }
            if let Some(patch) = mutation.get("definition") {
                let mut value = serde_json::to_value(self.input.definition).unwrap();
                value
                    .as_object_mut()
                    .unwrap()
                    .extend(patch.as_object().unwrap().clone());
                self.input.definition = serde_json::from_value(value).unwrap();
            }
            if let Some(patch) = mutation.get("settings") {
                for (key, value) in patch.as_object().unwrap() {
                    match key.as_str() {
                        "owner_present" => self.input.owner_present = value.as_bool().unwrap(),
                        "retained" => self.input.retained = value.as_bool().unwrap(),
                        _ => panic!("unknown fixture setting"),
                    }
                }
            }
        }
        if self.fail_at == Some(name.as_str())
            || (self.fail_at == Some("InvokeAbilityAndRelease")
                && matches!(name.as_str(), "InvokeAbility" | "ReleaseAbility"))
        {
            Err(format!("injected {name} failure"))
        } else {
            Ok(())
        }
    }
    fn value(&self, pair: [F32; 2]) -> Result<Binary, String> {
        Binary::from_f32_bits(pair[0].0)
            .and_then(|x| x.add(&Binary::from_f32_bits(pair[1].0)?, self.input.precision))
            .map_err(|e| e.to_string())
    }
}
impl World for Host {
    fn body_virtual38(&mut self, body: &mut Body, env: &mut Environment) -> Result<bool, String> {
        self.event(json!("BodyVirtual38"), body, env)?;
        Ok(self.input.retained)
    }
    fn definition(&self, _: u32) -> Result<Definition, String> {
        Ok(self.input.definition)
    }
    fn actor_flags(&self, _: u32) -> Result<u32, String> {
        Ok(self.input.actor_flags)
    }
    fn begin_ability(
        &mut self,
        definition: u32,
        body: &mut Body,
        env: &mut Environment,
    ) -> Result<u32, String> {
        self.event(json!({"BeginAbility":{"definition":definition}}), body, env)?;
        Ok(1)
    }
    fn allegiance(
        &mut self,
        body: &mut Body,
        env: &mut Environment,
    ) -> Result<Option<u32>, String> {
        self.event(json!("Allegiance"), body, env)?;
        Ok(self.input.owner_present.then_some(1))
    }
    fn actor_modifier(&self, _: u32) -> Result<Option<u32>, String> {
        Ok(self.input.modifier_present.then_some(1))
    }
    fn invoke_ability(
        &mut self,
        ability: u32,
        modifier: u32,
        request: AbilityInvocation,
        body: &mut Body,
        env: &mut Environment,
    ) -> Result<(), String> {
        self.event(
            json!({"InvokeAbility":{"ability":ability,"modifier":modifier,"request":request}}),
            body,
            env,
        )
    }
    fn release_ability(
        &mut self,
        ability: u32,
        body: &mut Body,
        env: &mut Environment,
    ) -> Result<(), String> {
        self.event(json!({"ReleaseAbility":{"ability":ability}}), body, env)
    }
    fn terrain_height(
        &mut self,
        flags: u32,
        body: &mut Body,
        env: &mut Environment,
    ) -> Result<Binary, String> {
        self.event(
            json!({"TerrainHeight":{"flags":flags,"position":env.owner_position}}),
            body,
            env,
        )?;
        let pair = self.input.heights[self.height_index];
        self.height_index += 1;
        self.value(pair)
    }
    fn actor_dimension(
        &mut self,
        body: &mut Body,
        env: &mut Environment,
    ) -> Result<Binary, String> {
        self.event(
            json!({"Dimension":{"index":self.dimension_index}}),
            body,
            env,
        )?;
        let pair = self.input.dimensions[self.dimension_index];
        self.dimension_index += 1;
        self.value(pair)
    }
    fn schedule_decay(
        &mut self,
        delay: F32,
        argument: u32,
        body: &mut Body,
        env: &mut Environment,
    ) -> Result<(), String> {
        self.event(
            json!({"ScheduleDecay":{"delay":delay,"argument":argument}}),
            body,
            env,
        )
    }
    fn remove_from_allegiance(
        &mut self,
        allegiance: u32,
        body: &mut Body,
        env: &mut Environment,
    ) -> Result<(), String> {
        self.event(
            json!({"RemoveFromAllegiance":{"allegiance":allegiance}}),
            body,
            env,
        )
    }
    fn remove_actor(
        &mut self,
        argument: u32,
        body: &mut Body,
        env: &mut Environment,
    ) -> Result<(), String> {
        self.event(json!({"RemoveActor":{"argument":argument}}), body, env)
    }
}
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase6-combat/dying-native-oracle.json"
    ))
    .unwrap()
}
#[test]
fn original_corpse_initialization_height_adjustment_and_callbacks_match() {
    let fixture = fixture();
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 84);
    for row in fixture["cases"].as_array().unwrap() {
        let input: Input = serde_json::from_value(row["input"].clone()).unwrap();
        let mut body = input.body.clone();
        let mut env = input.environment.clone();
        let mut host = Host {
            input: input.clone(),
            events: vec![],
            height_index: 0,
            dimension_index: 0,
            fail_at: None,
        };
        let outcome = start_dying(&mut body, &mut env, &mut host, input.precision).unwrap();
        assert!(host.input.mutations.is_empty());
        assert_eq!(
            outcome,
            if input.retained {
                Outcome::ScheduledDecay
            } else {
                Outcome::RemovedImmediately
            }
        );
        assert_eq!(
            json!({"body":body,"environment":env,"events":host.events}),
            row["output"],
            "{} {:?}",
            input.name,
            input.precision
        );
    }
}
#[test]
fn failed_schedule_keeps_prior_corpse_fields_and_does_not_remove_owner() {
    let input: Input = serde_json::from_value(fixture()["cases"][0]["input"].clone()).unwrap();
    let mut body = input.body.clone();
    let mut env = input.environment.clone();
    let mut host = Host {
        input: input.clone(),
        events: vec![],
        height_index: 0,
        dimension_index: 0,
        fail_at: Some("ScheduleDecay"),
    };
    assert!(matches!(
        start_dying(&mut body, &mut env, &mut host, input.precision),
        Err(aa_body::Error::Callback(_))
    ));
    assert_eq!(body.dying_started, input.environment.world_time);
    assert_eq!(body.sink_duration, input.definition.sink_time);
    assert_eq!(host.events.len(), 2);
    assert!(
        host.events.last().unwrap()["event"]
            .get("ScheduleDecay")
            .is_some()
    );
}

#[test]
fn every_post_acquisition_result_failure_attempts_ability_release_once() {
    let mut input: Input = serde_json::from_value(fixture()["cases"][0]["input"].clone()).unwrap();
    input.definition.death_ability = Some(1);
    for failure in [
        "Allegiance",
        "InvokeAbility",
        "ReleaseAbility",
        "InvokeAbilityAndRelease",
        "InvalidState",
    ] {
        let mut body = input.body.clone();
        let mut env = input.environment.clone();
        let mut host = Host {
            input: input.clone(),
            events: vec![],
            height_index: 0,
            dimension_index: 0,
            fail_at: if failure == "InvalidState" {
                None
            } else {
                Some(failure)
            },
        };
        if failure == "InvalidState" {
            host.input
                .mutations
                .push(json!({"on":"BeginAbility","body":{"maximum":0}}));
        }
        let error = start_dying(&mut body, &mut env, &mut host, input.precision).unwrap_err();
        let release_count = host
            .events
            .iter()
            .filter(|e| e["event"].get("ReleaseAbility").is_some())
            .count();
        assert_eq!(release_count, 1, "{failure}");
        assert!(
            host.events.last().unwrap()["event"]
                .get("ReleaseAbility")
                .is_some()
        );
        assert_eq!(body.dying_started, input.body.dying_started);
        if failure == "ReleaseAbility" {
            assert!(
                error
                    .to_string()
                    .contains("ability 1 release failed; world retains ownership")
            );
        }
        if failure == "InvokeAbilityAndRelease" {
            let message = error.to_string();
            assert!(
                message.contains("InvokeAbility failure")
                    && message.contains("cleanup also failed")
                    && message.contains("ability 1 release failed")
            );
        }
    }
}
