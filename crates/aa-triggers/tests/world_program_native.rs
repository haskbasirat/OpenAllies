use aa_body::{F32, credit::Object as BodyObject, property_admission, property_effects::Instance};
use aa_navigation::{control, startup::Object};
use aa_numeric::Precision;
use aa_triggers::{
    decode::{Element, Parameter, SavedManager, Span, Trigger, Value as ParameterValue, Variable},
    manager::{Elements, Error, Manager, Result},
    program::CoreProgram,
    selectors::Registry,
    world_program::{Presentation, World, WorldProgram},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn span() -> Span {
    Span {
        offset: 0,
        length: 0,
    }
}
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase7-triggers/native-world-program.json"
    ))
    .unwrap()
}
fn word(c: &Value, name: &str) -> u32 {
    c[name].as_u64().unwrap() as u32
}
fn flag(c: &Value, name: &str) -> bool {
    c[name].as_bool().unwrap()
}
fn element(c: &Value) -> Element {
    Element {
        source: span(),
        name: c["name"].as_str().unwrap().into(),
        opcode: word(c, "opcode"),
        word_10: 0,
        parameters: c["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| Parameter {
                source: span(),
                argument: p["argument"].as_str().unwrap().into(),
                item: p["item"].as_str().unwrap().into(),
                item_index: word(p, "item_index"),
                storage: word(p, "storage"),
                flag: word(p, "flag") as u8,
                value: match word(p, "storage") {
                    1 | 2 => ParameterValue::Word(word(p, "value")),
                    6 => {
                        ParameterValue::Text(p["value"].as_str().unwrap().encode_utf16().collect())
                    }
                    _ => panic!("unexpected fixture storage"),
                },
            })
            .collect(),
    }
}
fn manager(c: &Value) -> Manager {
    let mut manager = Manager::from_saved(SavedManager {
        next_id: 1,
        triggers: c["triggers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| Trigger {
                source: span(),
                id: word(t, "id"),
                mode: 1,
                enabled: word(t, "enabled") as u8,
                repeat: 0,
                activation_bits: word(t, "activation_bits"),
                name: vec![],
                description: vec![],
                word_2c: 0,
                events: vec![],
                conditions: vec![],
                actions: vec![],
            })
            .collect(),
        variables: c["variables"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(name, bits)| Variable {
                name: name.encode_utf16().collect(),
                bits: bits.as_u64().unwrap() as u32,
            })
            .collect(),
        strings: vec![],
    });
    manager.set_context_word(word(c, "condition_actor"));
    if let Some(id) = c["event_actor"].as_u64() {
        manager.add_subject(id as u32);
    }
    manager
}
struct Adapter {
    c: Value,
    presentation: Presentation,
    actor_flags: u32,
    kingdom_flags: u32,
    victory: u8,
    property_component: Option<BodyObject>,
    events: Vec<Value>,
    fail_process: bool,
}
impl Adapter {
    fn new(c: Value) -> Self {
        Self {
            presentation: serde_json::from_value(c["presentation"].clone()).unwrap(),
            actor_flags: word(&c, "actor_flags"),
            kingdom_flags: word(&c, "kingdom_flags"),
            victory: word(&c, "victory") as u8,
            property_component: flag(&c, "has_properties").then_some(BodyObject(5)),
            events: vec![],
            fail_process: false,
            c,
        }
    }
}
impl World for Adapter {
    fn current_time_bits(&self) -> u32 {
        word(&self.c, "time_bits")
    }
    fn presentation_mut(&mut self) -> Result<&mut Presentation> {
        Ok(&mut self.presentation)
    }
    fn body_actor_identity(&self, actor: Object) -> Result<BodyObject> {
        if actor == Object(3) {
            Ok(BodyObject(3))
        } else {
            Err(Error::State("missing live Body actor mapping".into()))
        }
    }
    fn named_property(&mut self, name: &[u16]) -> Result<Option<BodyObject>> {
        let name = String::from_utf16(name).unwrap();
        self.events.push(json!({"PropertyName":name}));
        Ok((name == "indispensable").then_some(BodyObject(4)))
    }
    fn set_default_victory(&mut self, kingdom: Object, value: u8) -> Result<()> {
        assert_eq!(kingdom, Object(2));
        self.victory = value;
        Ok(())
    }
}
impl Registry for Adapter {
    fn resolve_id(&self, raw: u32) -> Result<Option<Object>> {
        Ok(match raw {
            0x10002 => Some(Object(2)),
            0x10003 => Some(Object(3)),
            _ => None,
        })
    }
    fn raw_id(&self, object: Object) -> Result<u32> {
        Ok(match object.0 {
            2 => 0x10002,
            3 => 0x10003,
            _ => return Err(Error::State("object without GTID".into())),
        })
    }
    fn named_kingdom(&mut self, name: &[u16]) -> Result<Option<Object>> {
        let name = String::from_utf16(name).unwrap();
        self.events.push(json!({"KingdomName":name}));
        Ok((name == "kingdom_a").then_some(Object(2)))
    }
    fn named_actor(&mut self, name: &[u16]) -> Result<Option<Object>> {
        let name = String::from_utf16(name).unwrap();
        self.events.push(json!({"ActorName":name}));
        Ok((name == "actor_a").then_some(Object(3)))
    }
}
impl property_admission::World for Adapter {
    fn property_component(&self, actor: BodyObject) -> aa_body::Result<Option<BodyObject>> {
        assert_eq!(actor, BodyObject(3));
        Ok(self.property_component)
    }
    fn component_98_present(&self, _: BodyObject) -> aa_body::Result<bool> {
        unreachable!("technology kind2 does not use the bypass")
    }
    fn applicable(&mut self, actor: BodyObject, property: BodyObject) -> aa_body::Result<bool> {
        self.events.push(json!({"Applicable":[actor,property]}));
        if flag(&self.c, "replace_properties") {
            self.property_component = Some(BodyObject(6));
        }
        Ok(flag(&self.c, "applicable"))
    }
    fn create_and_insert(
        &mut self,
        component: BodyObject,
        property: BodyObject,
        kind: u32,
        source: Option<BodyObject>,
    ) -> aa_body::Result<()> {
        assert_eq!(source, None);
        let instance = Instance::new(
            BodyObject(9),
            property,
            kind,
            F32(self.current_time_bits()),
            None,
        );
        self.events.push(json!({"Insert":{"component":component,"kind":instance.kind,"created":instance.created,"definition":instance.definition,"source_id":instance.source_id,"disabled":instance.disabled}}));
        Ok(())
    }
}
impl control::World for Adapter {
    fn flags8(&self, o: Object) -> control::Result<u32> {
        Ok(match o.0 {
            2 => self.kingdom_flags,
            3 => self.actor_flags,
            _ => panic!("unknown object"),
        })
    }
    fn set_flags8(&mut self, o: Object, v: u32) -> control::Result<()> {
        match o.0 {
            2 => self.kingdom_flags = v,
            3 => self.actor_flags = v,
            _ => panic!("unknown object"),
        };
        Ok(())
    }
    fn actor_sai_handle(&self, o: Object) -> control::Result<Option<control::Shared>> {
        assert_eq!(o, Object(3));
        Ok(None)
    }
    fn references(&self, _: control::Shared) -> control::Result<u32> {
        unreachable!()
    }
    fn set_references(&mut self, _: control::Shared, _: u32) -> control::Result<()> {
        unreachable!()
    }
    fn destroy_shared(&mut self, _: control::Shared) -> control::Result<()> {
        unreachable!()
    }
    fn sai_target(&self, _: control::Shared) -> control::Result<Option<Object>> {
        unreachable!()
    }
    fn notify_disabled(
        &mut self,
        _: Object,
        _: control::Shared,
        _: [u32; 4],
    ) -> control::Result<()> {
        unreachable!()
    }
    fn kingdom_head284(&self, o: Object) -> control::Result<Option<control::Node>> {
        assert_eq!(o, Object(2));
        Ok(flag(&self.c, "kingdom_members").then_some(control::Node(1)))
    }
    fn node_actor(&self, n: control::Node) -> control::Result<Option<Object>> {
        assert_eq!(n, control::Node(1));
        Ok(Some(Object(3)))
    }
    fn node_next(&self, n: control::Node) -> control::Result<Option<control::Node>> {
        assert_eq!(n, control::Node(1));
        Ok(None)
    }
    fn command_definition(&self, ordinal: u32) -> control::Result<Option<Object>> {
        assert_eq!(ordinal, 6);
        Ok(Some(Object(8)))
    }
    fn definition_component18(&self, d: Object) -> control::Result<i32> {
        assert_eq!(d, Object(8));
        Ok(4)
    }
    fn actor_component(&self, o: Object, slot: usize) -> control::Result<Option<Object>> {
        assert_eq!((o, slot), (Object(3), 4));
        Ok(Some(Object(7)))
    }
    fn can_receive(
        &mut self,
        o: Object,
        d: Object,
        _: &mut control::Command,
    ) -> control::Result<bool> {
        assert_eq!((o, d), (Object(3), Object(8)));
        self.events.push(json!("Admission"));
        Ok(flag(&self.c, "admit"))
    }
    fn process_component(&mut self, o: Object, c: &mut control::Command) -> control::Result<bool> {
        assert_eq!(o, Object(7));
        assert_eq!(c, &control::Command::new(Some(Object(8))));
        self.events.push(json!("Process"));
        if self.fail_process {
            Err(control::Error("required command handler failed".into()))
        } else {
            Ok(flag(&self.c, "process_result"))
        }
    }
    fn actor_virtual158(&mut self, _: Object, _: &mut control::Command) -> control::Result<bool> {
        unreachable!()
    }
    fn actor_virtual15c(
        &mut self,
        _: Object,
        _: &mut control::Command,
    ) -> control::Result<Option<Object>> {
        unreachable!()
    }
    fn organization_parent(&self, _: Object) -> control::Result<Option<Object>> {
        unreachable!()
    }
}
#[test]
fn actual_world_action_wrappers_match_native_sequences_and_fields() {
    let fixture = fixture();
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 201);
    for case in cases {
        let input = &case["input"];
        let mut manager = manager(input);
        let element = element(&input["element"]);
        let mut world = Adapter::new(input.clone());
        let pc: Precision = serde_json::from_value(input["precision"].clone()).unwrap();
        let mut program = WorldProgram {
            core: CoreProgram::new(word(input, "time_bits"), 6, pc),
            world: &mut world,
        };
        let result = program.evaluate(&mut manager, 0, &element).unwrap();
        let diagnostics = program.core.diagnostics.len();
        let state = manager.snapshot();
        let output = json!({"result":result,"variables":state.variables.iter().map(|v|(String::from_utf16(&v.name).unwrap(),v.bits)).collect::<BTreeMap<_,_>>(),
            "triggers":state.triggers.iter().map(|t|json!({"id":t.id,"enabled":t.enabled,"activation_bits":t.activation_bits})).collect::<Vec<_>>(),
            "current_element":0,"diagnostics":diagnostics,"world_events":world.events,"actor_flags":world.actor_flags,"kingdom_flags":world.kingdom_flags,
            "victory":world.victory,"property_component":world.property_component.map_or(0,|o|o.0),"presentation":world.presentation});
        assert_eq!(output, case["output"], "{}", input["name"]);
    }
}
#[test]
fn required_command_failure_stops_manager_after_committed_control_flag() {
    let fixture = fixture();
    let input = fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| &c["input"])
        .find(|c| {
            word(&c["element"], "opcode") == 62
                && c["element"]["parameters"][0]["value"] == "kingdom_a"
        })
        .unwrap();
    let element = element(&input["element"]);
    let mut saved = SavedManager {
        next_id: 1,
        triggers: vec![Trigger {
            source: span(),
            id: 1,
            mode: 1,
            enabled: 1,
            repeat: 0,
            activation_bits: 0,
            name: vec![],
            description: vec![],
            word_2c: 0,
            events: vec![],
            conditions: vec![],
            actions: vec![element],
        }],
        variables: vec![],
        strings: vec![],
    };
    let mut manager = Manager::from_saved(saved.clone());
    let mut world = Adapter::new(input.clone());
    world.kingdom_flags &= !0x20;
    world.fail_process = true;
    let mut program = WorldProgram {
        core: CoreProgram::new(word(input, "time_bits"), 6, Precision::Extended64),
        world: &mut world,
    };
    assert!(manager.update(&mut program).is_err());
    assert!(manager.stopped());
    assert_ne!(program.world.kingdom_flags & 0x20, 0);
    assert!(manager.update(&mut program).is_err());
    assert_eq!(
        program
            .world
            .events
            .iter()
            .filter(|e| **e == "Process")
            .count(),
        1
    );
    // A registered but unimplemented world opcode must never silently succeed.
    saved.triggers[0].actions[0].opcode = 66;
    saved.triggers[0].actions[0].parameters.clear();
    let mut manager = Manager::from_saved(saved);
    assert!(manager.update(&mut program).is_err());
}
