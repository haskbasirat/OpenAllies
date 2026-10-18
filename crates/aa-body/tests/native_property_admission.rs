use aa_body::{
    Error, F32, Result,
    credit::Object,
    property_admission::{self, World},
    property_effects::Instance,
};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Clone, Deserialize)]
struct Input {
    name: String,
    actor: bool,
    property: bool,
    component: bool,
    special: bool,
    applicable: bool,
    replace: bool,
    source: bool,
    source_id: u32,
    time_bits: u32,
    trigger: bool,
    kind: u32,
}
#[derive(Deserialize)]
struct Fixture {
    cases: Vec<Case>,
}
#[derive(Deserialize)]
struct Case {
    input: Input,
    output: Value,
}
fn fixture() -> Fixture {
    serde_json::from_str(include_str!(
        "../../../research/phase7-properties/native-admission.json"
    ))
    .unwrap()
}
struct Adapter {
    input: Input,
    component: Option<Object>,
    events: Vec<Value>,
    remove_component: bool,
    insertion_failure: bool,
}
impl Adapter {
    fn new(input: Input) -> Self {
        Self {
            component: input.component.then_some(Object(2)),
            input,
            events: vec![],
            remove_component: false,
            insertion_failure: false,
        }
    }
}
impl World for Adapter {
    fn property_component(&self, actor: Object) -> Result<Option<Object>> {
        assert_eq!(actor, Object(1));
        Ok(self.component)
    }
    fn component_98_present(&self, actor: Object) -> Result<bool> {
        assert_eq!(actor, Object(1));
        Ok(self.input.special)
    }
    fn applicable(&mut self, actor: Object, property: Object) -> Result<bool> {
        self.events.push(json!({"Applicable":[actor,property]}));
        if self.input.replace {
            self.component = Some(Object(3));
        }
        if self.remove_component {
            self.component = None;
        }
        Ok(self.input.applicable)
    }
    fn create_and_insert(
        &mut self,
        component: Object,
        property: Object,
        kind: u32,
        source: Option<Object>,
    ) -> Result<()> {
        let record = Instance::new(
            Object(9),
            property,
            kind,
            F32(self.input.time_bits),
            source.map(|id| {
                assert_eq!(id, Object(5));
                self.input.source_id
            }),
        );
        self.events
            .push(json!({"Insert":{"component":component,"record":record}}));
        if self.insertion_failure {
            Err(Error::Callback(
                "insertion failed after ownership taken".into(),
            ))
        } else {
            Ok(())
        }
    }
}
#[test]
fn native_actor_admission_and_trigger_wrapper() {
    let fixture = fixture();
    assert_eq!(fixture.cases.len(), 60);
    for case in fixture.cases {
        let mut world = Adapter::new(case.input.clone());
        let input = &case.input;
        let result = if input.trigger {
            property_admission::grant_technology(
                input.actor.then_some(Object(1)),
                input.property.then_some(Object(4)),
                &mut world,
            )
        } else {
            property_admission::add(
                Object(1),
                input.property.then_some(Object(4)),
                input.kind,
                input.source.then_some(Object(5)),
                &mut world,
            )
        }
        .unwrap();
        assert_eq!(
            json!({"result":result,"component":world.component.map_or(0,|o|o.0),"events":world.events,"abi_preserved":true}),
            case.output,
            "{}",
            input.name
        );
    }
}
#[test]
fn failed_mutating_admission_and_insert_do_not_reuse_or_rollback_state() {
    let mut input = fixture().cases.remove(0).input;
    input.applicable = true;
    let mut world = Adapter::new(input.clone());
    world.remove_component = true;
    assert!(property_admission::add(Object(1), Some(Object(4)), 2, None, &mut world).is_err());
    assert_eq!(world.component, None);
    assert_eq!(world.events.len(), 1);
    let mut world = Adapter::new(input);
    world.insertion_failure = true;
    assert!(
        property_admission::grant_technology(Some(Object(1)), Some(Object(4)), &mut world).is_err()
    );
    assert_eq!(world.events.len(), 2);
    assert!(world.events[1].get("Insert").is_some());
}
