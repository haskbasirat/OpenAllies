use aa_actors::{
    ActorId,
    element_position::{self, Actor, Error, Result, World},
    ground_states::{ElementState, LayoutDefinition, OrganizationState},
};
use aa_body::F32;
use serde_json::{Value, json};
use std::collections::BTreeMap;
struct Host {
    input: Value,
    organization: OrganizationState<u32>,
    events: Vec<Value>,
    mutations: Vec<Value>,
    calls: BTreeMap<u32, usize>,
    fail: Option<&'static str>,
}
fn id(raw: u32) -> ActorId {
    ActorId::from_raw(raw)
}
impl Host {
    fn new(input: &Value) -> Self {
        let position: [u32; 2] = serde_json::from_value(input["goal"].clone()).unwrap();
        let mut organization = OrganizationState::new(
            id(input["owner"].as_u64().unwrap() as u32),
            LayoutDefinition {
                definition: 0,
                parameters: vec![],
                slots: 0,
            },
            F32::ZERO,
            F32::ZERO,
            position.map(F32),
        )
        .unwrap();
        if let Some(speed) = input["local_speed"].as_u64() {
            organization.words.insert(0xd8, speed as u32);
        }
        Self {
            input: input.clone(),
            organization,
            events: vec![],
            mutations: input["mutations"].as_array().unwrap().clone(),
            calls: BTreeMap::new(),
            fail: None,
        }
    }
    fn event(&mut self, name: &str, args: &[Value]) -> Result<()> {
        let mut event = vec![json!(name)];
        event.extend_from_slice(args);
        event.push(json!(self.organization.position_xy.map(|v| v.0)));
        self.events.push(json!(event));
        if self.fail == Some(name) {
            return Err(Error::World(format!("fixture {name} failed")));
        }
        if self.mutations.first().is_some_and(|m| m["event"] == name) {
            let m = self.mutations.remove(0);
            if let Some(owner) = m["owner"].as_u64() {
                self.organization.header.owner = id(owner as u32);
            }
            if let Some(selected) = m.get("selected_member") {
                self.input["selected_member"] = selected.clone();
            }
            if let Some(goal) = m.get("goal") {
                let point: [u32; 2] = serde_json::from_value(goal.clone()).unwrap();
                self.organization.position_xy = point.map(F32);
            }
            if let Some(actors) = m["actors"].as_object() {
                for (k, fields) in actors {
                    for (name, value) in fields.as_object().unwrap() {
                        self.input["actors"][k][name] = value.clone();
                    }
                }
            }
        }
        Ok(())
    }
}
impl World for Host {
    type Definition = u32;
    fn world_mode(&self) -> Result<u32> {
        Ok(self.input["world_mode"].as_u64().unwrap() as u32)
    }
    fn actor(&self, actor: ActorId) -> Result<Actor> {
        let a = &self.input["actors"][actor.raw().to_string()];
        let position: [u32; 2] = serde_json::from_value(a["position"].clone()).unwrap();
        Ok(Actor {
            flags: a["flags"].as_u64().unwrap() as u32,
            position: position.map(F32),
            mind: a["mind"].as_u64().map(|v| v as u32),
        })
    }
    fn organization(&self, org: ActorId) -> Result<&OrganizationState<u32>> {
        assert_eq!(org, id(2));
        Ok(&self.organization)
    }
    fn organization_mut(&mut self, org: ActorId) -> Result<&mut OrganizationState<u32>> {
        assert_eq!(org, id(2));
        Ok(&mut self.organization)
    }
    fn selected_member(&self, _org: ActorId) -> Result<Option<ActorId>> {
        Ok(self.input["selected_member"].as_u64().map(|v| id(v as u32)))
    }
    fn mind_predicate(&mut self, mind: u32, slot: u32) -> Result<bool> {
        self.event("predicate", &[json!(mind), json!(slot)])?;
        let count = self.calls.entry(slot).or_default();
        let answers = self.input["predicates"][slot.to_string()]
            .as_array()
            .unwrap();
        let result = answers[(*count).min(answers.len() - 1)].as_bool().unwrap();
        *count += 1;
        Ok(result)
    }
    fn relocate_owner(&mut self, actor: ActorId, point: [F32; 2]) -> Result<()> {
        self.event("relocate", &[json!(actor.raw()), json!(point.map(|v| v.0))])
    }
    fn interpolate(&mut self, org: ActorId) -> Result<bool> {
        self.event("interpolate", &[json!(org.raw())])?;
        Ok(self.input["interpolate"].as_bool().unwrap())
    }
    fn has_scheduled(&mut self, org: ActorId, event: u32, data: u32) -> Result<bool> {
        self.event("queued", &[json!(org.raw()), json!(event), json!(data)])?;
        Ok(self.input["queued"].as_bool().unwrap())
    }
    fn schedule(&mut self, org: ActorId, data: u32) -> Result<()> {
        self.event("schedule", &[json!(org.raw()), json!(data)])
    }
}
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase7-element-position/native-position.json"
    ))
    .unwrap()
}
#[test]
fn original_element_and_organization_position_notification() {
    let fixture = fixture();
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 56);
    for case in fixture["cases"].as_array().unwrap() {
        let input = &case["input"];
        let mut host = Host::new(input);
        match input["operation"].as_str().unwrap() {
            "element" => {
                let mut element = ElementState::<u32>::new(id(1)).unwrap();
                element.organization = input["element_organization"].as_u64().map(|v| id(v as u32));
                element_position::element_moved(&element, &mut host).unwrap();
            }
            "notify" => element_position::organization_moved(
                id(2),
                id(input["member"].as_u64().unwrap() as u32),
                &mut host,
            )
            .unwrap(),
            "set" => {
                let point: [u32; 2] = serde_json::from_value(input["point"].clone()).unwrap();
                element_position::set_position(id(2), point.map(F32), &mut host).unwrap();
            }
            _ => unreachable!(),
        }
        assert_eq!(
            json!({"goal":host.organization.position_xy.map(|v|v.0),"events":host.events}),
            case["output"],
            "{}",
            input["name"]
        );
        assert!(host.mutations.is_empty());
    }
}
#[test]
fn failed_interpolation_keeps_the_new_goal_without_scheduling() {
    let fixture = fixture();
    let input = fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["input"]["name"] == "element_flags0_predFalse_True")
        .unwrap()["input"]
        .clone();
    let mut host = Host::new(&input);
    host.fail = Some("interpolate");
    let point = [F32(0x41200000), F32(0x42000000)];
    assert!(element_position::set_position(id(2), point, &mut host).is_err());
    assert_eq!(host.organization.position_xy, point);
    assert_eq!(host.events.last().unwrap()[0], "interpolate");
}

impl aa_actors::organization_interpolation::InterpolationWorld for Host {
    type Query = [u32; 2];
    fn speed_multiplier(&self) -> Result<F32> {
        Ok(F32(self.input["speed_multiplier"].as_u64().unwrap() as u32))
    }
    fn minimum_speed(&mut self) -> Result<aa_numeric::Binary> {
        self.event("minimum", &[])?;
        Ok(
            aa_numeric::Binary::from_f64_bits(self.input["minimum_speed_bits"].as_u64().unwrap())
                .unwrap(),
        )
    }
    fn timestep(&self) -> Result<F32> {
        Ok(F32(self.input["timestep"].as_u64().unwrap() as u32))
    }
    fn member_moving(&mut self, member: Option<ActorId>) -> Result<bool> {
        self.event("moving", &[json!(member)])?;
        Ok(self.input["moving"].as_bool().unwrap())
    }
    fn mind_point(&mut self, mind: u32) -> Result<[F32; 2]> {
        self.event("mind_point", &[json!(mind)])?;
        let point: [u32; 2] = serde_json::from_value(self.input["mind_point"].clone()).unwrap();
        Ok(point.map(F32))
    }
    fn prepare_probe(&mut self, member: Option<ActorId>, flags: u32) -> Result<Self::Query> {
        self.event("prepare", &[json!(member), json!(flags)])?;
        let values: [u32; 2] = serde_json::from_value(self.input["query_flags"].clone()).unwrap();
        Ok(values.map(|v| v | flags))
    }
    fn blocked(&mut self, query: &mut Self::Query, point: [F32; 2]) -> Result<bool> {
        self.event("probe", &[json!(point.map(|v| v.0)), json!(query)])?;
        let count = self.calls.entry(999).or_default();
        let answers = self.input["blocked"].as_array().unwrap();
        let result = answers[(*count).min(answers.len() - 1)].as_bool().unwrap();
        *count += 1;
        Ok(result)
    }
}
#[test]
fn original_organization_interpolation_coordinates_probes_and_callback_order() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-element-position/native-interpolation.json"
    ))
    .unwrap();
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 132);
    for case in fixture["cases"].as_array().unwrap() {
        let input = &case["input"];
        let mut host = Host::new(input);
        let precision = serde_json::from_value(input["precision"].clone()).unwrap();
        let complete =
            aa_actors::organization_interpolation::interpolate(id(2), &mut host, precision)
                .unwrap();
        assert_eq!(
            json!({"complete":complete,"goal":host.organization.position_xy.map(|v|v.0),"events":host.events}),
            case["output"],
            "{}",
            input["name"]
        );
        assert!(host.mutations.is_empty());
    }
}
#[test]
fn nonpositive_interpolation_step_is_explicit_without_fake_probe() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-element-position/native-interpolation.json"
    ))
    .unwrap();
    let mut host = Host::new(&fixture["cases"][16]["input"]);
    host.input["timestep"] = json!(0u32);
    assert!(
        aa_actors::organization_interpolation::interpolate(
            id(2),
            &mut host,
            aa_numeric::Precision::Extended64
        )
        .unwrap_err()
        .to_string()
        .contains("nonpositive")
    );
    assert!(!host.events.iter().any(|e| e[0] == "probe"));
}
