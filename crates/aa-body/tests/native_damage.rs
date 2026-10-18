use aa_body::{Body, Environment, Event, F32, HealthChange, Host, damage::*};
use aa_numeric::Precision;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Clone, Deserialize)]
struct View {
    wall_link: bool,
    organization: bool,
    organization_leader: bool,
    mind: bool,
    component88: bool,
}
#[derive(Clone, Deserialize)]
struct Input {
    name: String,
    precision: Precision,
    body: Body,
    environment: Environment,
    request: DamageRequest,
    view: View,
    source_present: bool,
    source_position: [F32; 3],
    wall_present: bool,
    segments_present: bool,
    transfer_factor: F32,
    wall_index: i32,
    neighbors: [bool; 2],
    hostile: bool,
    modifier_present: bool,
    factors: Vec<(u32, F32)>,
    outer_present: bool,
    ancestor_present: bool,
    ancestor_component: bool,
    capture_eligible: bool,
    capture_success: bool,
    capture_factor: F32,
    normalization_factor: F32,
    mutations: Vec<Value>,
}
struct World {
    input: Input,
    events: Vec<Value>,
}
impl World {
    fn event(
        &mut self,
        value: Value,
        body: &mut Body,
        env: &mut Environment,
        record: Option<&mut DamageRecord>,
    ) {
        let mut name = value
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| value.as_object().unwrap().keys().next().unwrap().clone());
        if name == "Action" {
            let action = &value["Action"]["kind"];
            name = action
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| action.as_object().unwrap().keys().next().unwrap().clone());
        }
        self.events.push(json!({"event":value,"body":body}));
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
            if let Some(patch) = mutation.get("view") {
                for (key, value) in patch.as_object().unwrap() {
                    let value = value.as_bool().unwrap();
                    match key.as_str() {
                        "wall_link" => self.input.view.wall_link = value,
                        "organization" => self.input.view.organization = value,
                        "organization_leader" => self.input.view.organization_leader = value,
                        "mind" => self.input.view.mind = value,
                        "component88" => self.input.view.component88 = value,
                        _ => panic!("unknown fixture view"),
                    }
                }
            }
            if let Some(patch) = mutation.get("record") {
                let record = record.expect("record mutations require a native record callback");
                let mut value = serde_json::to_value(*record).unwrap();
                value
                    .as_object_mut()
                    .unwrap()
                    .extend(patch.as_object().unwrap().clone());
                *record = serde_json::from_value(value).unwrap();
            }
            if let Some(patch) = mutation.get("settings") {
                for (key, value) in patch.as_object().unwrap() {
                    match key.as_str() {
                        "neighbors" => {
                            self.input.neighbors = serde_json::from_value(value.clone()).unwrap()
                        }
                        "hostile" => self.input.hostile = value.as_bool().unwrap(),
                        "capture_eligible" => {
                            self.input.capture_eligible = value.as_bool().unwrap()
                        }
                        "capture_success" => self.input.capture_success = value.as_bool().unwrap(),
                        _ => panic!("unknown fixture setting"),
                    }
                }
            }
        }
    }
}
impl Host for World {
    fn event(
        &mut self,
        event: Event,
        body: &mut Body,
        env: &mut Environment,
    ) -> Result<(), String> {
        self.event(serde_json::to_value(event).unwrap(), body, env, None);
        Ok(())
    }
    fn linked_health(
        &mut self,
        change: HealthChange,
        body: &mut Body,
        env: &mut Environment,
    ) -> Result<(), String> {
        assert_eq!(change.source_id, None);
        self.event(json!({"LinkedHealth":{"requested":change.requested,"token":change.token,"notify":change.notify,"source":0}}),body,env,None);
        Ok(())
    }
    fn health_source_position(&mut self, _: u32) -> Result<Option<[F32; 3]>, String> {
        Ok(self
            .input
            .source_present
            .then_some(self.input.source_position))
    }
    fn health_normalization_maximum(
        &mut self,
        _: &Body,
        env: &Environment,
        pc: Precision,
    ) -> Result<aa_numeric::Binary, String> {
        let value = aa_numeric::Binary::from_f32_bits(env.derived_normalization_maximum.0)
            .map_err(|e| e.to_string())?;
        if self.input.modifier_present {
            value
                .mul(
                    &aa_numeric::Binary::from_f32_bits(self.input.normalization_factor.0)
                        .map_err(|e| e.to_string())?,
                    pc,
                )
                .map_err(|e| e.to_string())
        } else {
            Ok(value)
        }
    }
}
impl aa_body::damage::World for World {
    fn propagate_damage(
        &mut self,
        target: PropagationTarget,
        request: DamageRequest,
        body: &mut Body,
        env: &mut Environment,
        _: Precision,
    ) -> Result<(), String> {
        self.event(
            json!({"PropagateDamage":{"target":target,"record":request.record,
            "apply_type_modifier":request.apply_type_modifier,"weapon":request.weapon_id.is_some(),
            "propagate_to_neighbors":request.propagate_to_neighbors}}),
            body,
            env,
            None,
        );
        Ok(())
    }
    fn actor_view(&self, _: u32) -> Result<ActorView, String> {
        let v = &self.input.view;
        Ok(ActorView {
            wall_link: v.wall_link.then_some(1),
            organization: v.organization.then_some(1),
            organization_leader: (v.organization && v.organization_leader).then_some(1),
            mind: v.mind.then_some(1),
            component88: v.component88.then_some(1),
        })
    }
    fn wall(&self, _: u32) -> Result<Option<Wall>, String> {
        Ok(self.input.wall_present.then_some(Wall {
            actor: 1,
            segments: self.input.segments_present.then_some(1),
            transfer_factor: self.input.transfer_factor,
        }))
    }
    fn wall_index(&self, _: u32, _: u32) -> Result<i32, String> {
        Ok(self.input.wall_index)
    }
    fn wall_neighbor(&self, _: u32, _: i32, forward: bool) -> Result<Option<u32>, String> {
        let i = usize::from(!forward);
        Ok(self.input.neighbors[i].then_some(i as u32 + 1))
    }
    fn hostile(&mut self, _: u32, body: &mut Body, env: &mut Environment) -> Result<bool, String> {
        self.event(json!("Hostile"), body, env, None);
        Ok(self.input.hostile)
    }
    fn resolve_source(&self, _: u32) -> Result<Option<u32>, String> {
        Ok(self.input.source_present.then_some(1))
    }
    fn modifier(&self, _: u32) -> Result<Option<u32>, String> {
        Ok(self.input.modifier_present.then_some(1))
    }
    fn damage_factor(&self, _: u32, kind: u32) -> Result<F32, String> {
        Ok(first_damage_factor(&self.input.factors, kind))
    }
    fn outer_actor(
        &mut self,
        body: &mut Body,
        env: &mut Environment,
    ) -> Result<Option<OuterActor>, String> {
        self.event(json!("OuterActor"), body, env, None);
        Ok(self.input.outer_present.then_some(OuterActor {
            actor: 1,
            component98: 1,
        }))
    }
    fn ancestor_component88(&self, _: u32) -> Result<Option<u32>, String> {
        Ok((self.input.ancestor_present && self.input.ancestor_component).then_some(2))
    }
    fn capture_eligible(&mut self, body: &mut Body, env: &mut Environment) -> Result<bool, String> {
        self.event(json!("CaptureEligible"), body, env, None);
        Ok(self.input.capture_eligible)
    }
    fn attempt_capture(
        &mut self,
        _: Option<u32>,
        _: &mut DamageRecord,
        body: &mut Body,
        env: &mut Environment,
    ) -> Result<bool, String> {
        self.event(json!("AttemptCapture"), body, env, None);
        Ok(self.input.capture_success)
    }
    fn capture_health_factor(&self) -> Result<F32, String> {
        Ok(self.input.capture_factor)
    }
    fn action(
        &mut self,
        action: Action,
        record: &mut DamageRecord,
        body: &mut Body,
        env: &mut Environment,
    ) -> Result<(), String> {
        self.event(
            json!({"Action":{"kind":action,"record":record}}),
            body,
            env,
            Some(record),
        );
        Ok(())
    }
}
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase6-combat/damage-native-oracle.json"
    ))
    .unwrap()
}
#[test]
fn original_damage_health_effects_and_world_order_match() {
    let fixtures = fixture();
    assert_eq!(fixtures["cases"].as_array().unwrap().len(), 219);
    for case in fixtures["cases"].as_array().unwrap() {
        let input: Input = serde_json::from_value(case["input"].clone()).unwrap();
        let mut body = input.body.clone();
        let mut env = input.environment.clone();
        let mut world = World {
            input: input.clone(),
            events: vec![],
        };
        deal_damage(
            &mut body,
            input.request,
            &mut env,
            &mut world,
            input.precision,
        )
        .unwrap();
        assert!(world.input.mutations.is_empty());
        let actual = json!({"body":body,"environment":env,"events":world.events});
        assert_eq!(
            actual, case["output"],
            "{} {:?}",
            input.name, input.precision
        );
    }
}

#[test]
fn unresolved_graph_or_source_is_not_silently_discarded() {
    struct Missing;
    impl Host for Missing {
        fn event(&mut self, _: Event, _: &mut Body, _: &mut Environment) -> Result<(), String> {
            Ok(())
        }
    }
    let input: Input = serde_json::from_value(fixture()["cases"][0]["input"].clone()).unwrap();
    let mut body = input.body;
    let mut env = input.environment;
    env.has_linked_body = true;
    let before = body.clone();
    assert!(matches!(
        body.set_health_from_source(
            HealthChange {
                requested: F32::ZERO,
                token: 0,
                notify: true,
                source_id: None
            },
            &mut env,
            &mut Missing,
            input.precision
        ),
        Err(aa_body::Error::Callback(_))
    ));
    assert_eq!(body, before);
    env.has_linked_body = false;
    env.health_rules = vec![aa_body::HealthRule {
        kind: 0,
        threshold: F32(0x3f00_0000),
    }];
    assert!(matches!(
        body.set_health_from_source(
            HealthChange {
                requested: F32::ZERO,
                token: 0,
                notify: true,
                source_id: Some(42)
            },
            &mut env,
            &mut Missing,
            input.precision
        ),
        Err(aa_body::Error::Callback(_))
    ));
    assert_eq!(body.health, F32::ZERO); // prior native setter effects are not rolled back
}
