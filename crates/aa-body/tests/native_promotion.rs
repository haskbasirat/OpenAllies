use aa_body::{
    F32, Result,
    credit::Object,
    promotion::{self, Effects, ExperienceState, State},
};
use aa_numeric::Precision;
use serde_json::{Value, json};

struct Host {
    case: Value,
    events: Vec<Value>,
    stats: Vec<F32>,
    allegiance_count: usize,
}
impl Host {
    fn new(case: &Value) -> Self {
        Self {
            case: case.clone(),
            events: vec![],
            stats: serde_json::from_value(case["stats"].clone()).unwrap(),
            allegiance_count: 0,
        }
    }
    fn event(&mut self, event: Value, state: &mut State) -> Result<()> {
        self.events.push(json!({"event":event,"state":state}));
        let name = event
            .as_str()
            .unwrap_or_else(|| event.as_object().unwrap().keys().next().unwrap());
        if self.case["mutations"]
            .as_array()
            .unwrap()
            .first()
            .is_some_and(|m| m["on"] == name)
        {
            let mutation = self.case["mutations"].as_array_mut().unwrap().remove(0);
            let mut value = serde_json::to_value(&*state).unwrap();
            for group in ["component", "actor", "ranks"] {
                if let Some(fields) = mutation[group].as_object() {
                    for (key, new) in fields {
                        value[group][key] = new.clone();
                    }
                }
            }
            *state = serde_json::from_value(value).unwrap();
            if let Some(fields) = mutation["world"].as_object() {
                for (key, value) in fields {
                    self.case[key] = value.clone();
                }
            }
        }
        if self.case["fail_on"] == name {
            return Err(aa_body::Error::Callback(format!("fixture {name} failure")));
        }
        Ok(())
    }
}
impl Effects for Host {
    fn remove_modifier(&mut self, modifier: Option<Object>, state: &mut State) -> Result<()> {
        self.event(json!({"RemoveModifier":{"modifier":modifier}}), state)
    }
    fn add_modifier(
        &mut self,
        modifier: Option<Object>,
        kind: u32,
        state: &mut State,
    ) -> Result<bool> {
        self.event(
            json!({"AddModifier":{"modifier":modifier,"kind":kind}}),
            state,
        )?;
        Ok(self.case["modifier_accepted"].as_bool().unwrap())
    }
    fn rank_changed(
        &mut self,
        old: Option<Object>,
        new: Option<Object>,
        state: &mut State,
    ) -> Result<()> {
        self.event(json!({"RankChanged":{"old":old,"new":new}}), state)
    }
    fn actor_label(&mut self, state: &mut State) -> Result<String> {
        self.event(json!("ActorLabel"), state)?;
        Ok("actor".into())
    }
    fn allegiance(&mut self, state: &mut State) -> Result<Option<Object>> {
        self.event(json!("Allegiance"), state)?;
        let items = self.case["allegiances"].as_array().unwrap();
        let value = &items[self.allegiance_count.min(items.len() - 1)];
        self.allegiance_count += 1;
        Ok(value.as_u64().map(|v| Object(v as u32)))
    }
    fn notification(
        &mut self,
        kind: u32,
        allegiance: Option<Object>,
        label: String,
        state: &mut State,
    ) -> Result<()> {
        self.event(
            json!({"Notification":{"kind":kind,"allegiance":allegiance,"label":label}}),
            state,
        )
    }
    fn propagate_experience(&mut self, amount: F32, state: &mut State) -> Result<()> {
        self.event(json!({"PropagateExperience":{"amount":amount}}), state)
    }
    fn statistics(&mut self, _: Object) -> Result<Option<&mut [F32]>> {
        Ok(self.case["has_stats"]
            .as_bool()
            .unwrap()
            .then_some(self.stats.as_mut_slice()))
    }
}
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase7-promotion/native-promotion-fixtures.json"
    ))
    .unwrap()
}

#[test]
fn native_owned_experience_rank_transitions_and_statistics() {
    let doc = fixture();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 141);
    for row in cases {
        let c = &row["input"];
        let mut state: State = serde_json::from_value(c["state"].clone()).unwrap();
        let mut host = Host::new(c);
        let p: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let amount = F32(c["amount"].as_u64().unwrap() as u32);
        let notify = c["notify"].as_bool().unwrap();
        match c["method"].as_str().unwrap() {
            "new" => state.component = ExperienceState::new(state.component.owner),
            "set" => promotion::set_rank(
                &mut state,
                c["requested"].as_u64().map(|v| Object(v as u32)),
                &mut host,
            )
            .unwrap(),
            "apply" => promotion::apply(&mut state, amount, notify, p, &mut host).unwrap(),
            "reset" => {
                promotion::reset_and_apply(&mut state, amount, notify, p, &mut host).unwrap()
            }
            _ => unreachable!(),
        }
        assert_eq!(
            json!({"state":state,"events":host.events,"stats":host.stats,"abi_preserved":true}),
            row["output"],
            "{}",
            c["name"]
        );
        assert!(host.case["mutations"].as_array().unwrap().is_empty());
    }
}

#[test]
fn missing_rank_and_callback_failure_preserve_prior_native_writes() {
    let doc = fixture();
    let case = &doc["cases"][0]["input"];
    let mut state: State = serde_json::from_value(case["state"].clone()).unwrap();
    state.component = ExperienceState::new(Object(1));
    let mut host = Host::new(case);
    assert!(
        promotion::apply(&mut state, F32::ONE, true, Precision::Double53, &mut host)
            .unwrap_err()
            .to_string()
            .contains("not initialized")
    );
    assert_eq!(state.component.experience, F32::ONE);
    assert!(host.events.is_empty());
    let mut state: State = serde_json::from_value(case["state"].clone()).unwrap();
    host.case["fail_on"] = json!("AddModifier");
    assert!(
        promotion::apply(
            &mut state,
            F32(11.0f32.to_bits()),
            true,
            Precision::Double53,
            &mut host
        )
        .is_err()
    );
    assert_eq!(state.component.current_rank, Some(Object(2)));
    assert_eq!(state.component.experience, F32(11.0f32.to_bits()));
    assert_eq!(host.events.len(), 2);
}
