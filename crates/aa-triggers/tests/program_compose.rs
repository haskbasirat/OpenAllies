//! Composition test (NOT an oracle): world_program::Program drives a decoded
//! tutorial-style trigger through Manager::update, dispatching core, phase8
//! elements/actions and phase7 world opcodes in one manager pass.  Each element
//! is proven separately by the native corpora; this checks the wiring only.
mod fixture;
use aa_body::credit::Object as BodyObject;
use aa_navigation::{control, startup::Object};
use aa_numeric::Precision;
use aa_triggers::{
    decode::{Element, Parameter, SavedManager, Trigger, Value as PV},
    manager::{Error, Manager, Result},
    objectives::{self, MissionServices},
    program::CoreProgram,
    selectors::{Checks, Registry},
    world_program::{Presentation, Program, Transition, World},
};
use fixture::{FixtureWorld, span, utf16};
use serde_json::json;

struct Composite {
    base: FixtureWorld,
    presentation: Presentation,
}
macro_rules! delegate {
    ($($t:tt)*) => {};
}
delegate!();
impl Registry for Composite {
    fn resolve_id(&self, raw: u32) -> Result<Option<Object>> {
        self.base.resolve_id(raw)
    }
    fn raw_id(&self, object: Object) -> Result<u32> {
        self.base.raw_id(object)
    }
    fn named_actor(&mut self, name: &[u16]) -> Result<Option<Object>> {
        self.base.named_actor(name)
    }
    fn named_kingdom(&mut self, name: &[u16]) -> Result<Option<Object>> {
        self.base.named_kingdom(name)
    }
}
impl Checks for Composite {
    fn is_area(&mut self, o: Object) -> Result<bool> {
        self.base.is_area(o)
    }
    fn is_camera(&mut self, o: Object) -> Result<bool> {
        self.base.is_camera(o)
    }
}
impl objectives::Services for Composite {
    fn objective_definitions(&self) -> Result<&[objectives::ObjectiveDef]> {
        self.base.objective_definitions()
    }
    fn kingdom_objectives_mut(&mut self, k: Object) -> Result<&mut Vec<objectives::Entry>> {
        self.base.kingdom_objectives_mut(k)
    }
    fn sort_objectives(&mut self) -> Result<bool> {
        objectives::Services::sort_objectives(&mut self.base)
    }
    fn post_message(&mut self, code: u32, k: Object, text: &[u16]) -> Result<()> {
        objectives::Services::post_message(&mut self.base, code, k, text)
    }
}
impl MissionServices for Composite {
    fn mission_result(&self, k: Object) -> Result<u32> {
        self.base.mission_result(k)
    }
    fn set_mission_result(&mut self, k: Object, r: u32) -> Result<()> {
        self.base.set_mission_result(k, r)
    }
    fn game_var_byte(&mut self, n: &[u16]) -> Result<Option<u8>> {
        self.base.game_var_byte(n)
    }
    fn team_member_count(&self, k: Object) -> Result<Option<usize>> {
        self.base.team_member_count(k)
    }
    fn world_48(&self) -> Result<u32> {
        self.base.world_48()
    }
    fn kingdom_flag_239(&self, k: Object) -> Result<u8> {
        self.base.kingdom_flag_239(k)
    }
    fn kingdom_result_text(&mut self, k: Object, f: u32) -> Result<Vec<u16>> {
        self.base.kingdom_result_text(k, f)
    }
    fn post_mission_message(&mut self, c: u32, k: Object, t: &[u16]) -> Result<()> {
        self.base.post_mission_message(c, k, t)
    }
    fn local_kingdom(&mut self) -> Result<Option<Object>> {
        self.base.local_kingdom()
    }
    fn ui_mission_ended(&mut self) -> Result<()> {
        self.base.ui_mission_ended()
    }
    fn world_mission_result(&mut self, k: Object) -> Result<()> {
        self.base.world_mission_result(k)
    }
}
include!("compose/delegate.rs");

impl control::World for Composite {
    fn flags8(&self, _: Object) -> control::Result<u32> {
        Ok(0)
    }
    fn set_flags8(&mut self, o: Object, v: u32) -> control::Result<()> {
        self.base.flags8_writes.insert(o.0, v);
        Ok(())
    }
    fn actor_sai_handle(&self, _: Object) -> control::Result<Option<control::Shared>> {
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
    fn notify_disabled(&mut self, _: Object, _: control::Shared, _: [u32; 4]) -> control::Result<()> {
        unreachable!()
    }
    fn kingdom_head284(&self, _: Object) -> control::Result<Option<control::Node>> {
        Ok(None)
    }
    fn node_actor(&self, _: control::Node) -> control::Result<Option<Object>> {
        unreachable!()
    }
    fn node_next(&self, _: control::Node) -> control::Result<Option<control::Node>> {
        unreachable!()
    }
    fn command_definition(&self, _: u32) -> control::Result<Option<Object>> {
        unreachable!()
    }
    fn definition_component18(&self, _: Object) -> control::Result<i32> {
        unreachable!()
    }
    fn actor_component(&self, _: Object, _: usize) -> control::Result<Option<Object>> {
        unreachable!()
    }
    fn can_receive(&mut self, _: Object, _: Object, _: &mut control::Command) -> control::Result<bool> {
        unreachable!()
    }
    fn process_component(&mut self, _: Object, _: &mut control::Command) -> control::Result<bool> {
        unreachable!()
    }
    fn actor_virtual158(&mut self, _: Object, _: &mut control::Command) -> control::Result<bool> {
        unreachable!()
    }
    fn actor_virtual15c(&mut self, _: Object, _: &mut control::Command) -> control::Result<Option<Object>> {
        unreachable!()
    }
    fn organization_parent(&self, _: Object) -> control::Result<Option<Object>> {
        unreachable!()
    }
}
impl aa_body::property_admission::World for Composite {
    fn property_component(&self, _: BodyObject) -> aa_body::Result<Option<BodyObject>> {
        unreachable!()
    }
    fn component_98_present(&self, _: BodyObject) -> aa_body::Result<bool> {
        unreachable!()
    }
    fn applicable(&mut self, _: BodyObject, _: BodyObject) -> aa_body::Result<bool> {
        unreachable!()
    }
    fn create_and_insert(&mut self, _: BodyObject, _: BodyObject, _: u32, _: Option<BodyObject>) -> aa_body::Result<()> {
        unreachable!()
    }
}
impl World for Composite {
    fn current_time_bits(&self) -> u32 {
        0x4120_0000
    }
    fn presentation_mut(&mut self) -> Result<&mut Presentation> {
        Ok(&mut self.presentation)
    }
    fn body_actor_identity(&self, _: Object) -> Result<BodyObject> {
        Err(Error::State("unused".into()))
    }
    fn named_property(&mut self, _: &[u16]) -> Result<Option<BodyObject>> {
        Err(Error::State("unused".into()))
    }
    fn set_default_victory(&mut self, _: Object, _: u8) -> Result<()> {
        Ok(())
    }
}

fn param(argument: &str, item: &str, item_index: u32, storage: u32, value: PV) -> Parameter {
    Parameter { source: span(), argument: argument.into(), item: item.into(), item_index, storage, flag: 1, value }
}
fn el(name: &str, opcode: u32, parameters: Vec<Parameter>) -> Element {
    Element { source: span(), name: name.into(), opcode, word_10: 0, parameters }
}

#[test]
fn one_manager_pass_dispatches_core_elements_actions_and_world_wrappers() {
    let kingdom = || param("ma_kingdom", "mi_kingdom_named", 18, 6, PV::Text(utf16("kingdom_a")));
    let trigger = Trigger {
        source: span(), id: 7, mode: 1, enabled: 1, repeat: 0, activation_bits: 0,
        name: vec![], description: vec![], word_2c: 0,
        events: vec![],
        conditions: vec![
            el("true", 0, vec![]),
            el("objective_succeeded", 97, vec![
                param("ma_objective", "mi_objective", 33, 6, PV::Text(utf16("obj_a"))), kingdom()]),
        ],
        actions: vec![
            el("kingdom_gains_objective", 50, vec![kingdom(), param("ma_objective", "mi_objective", 33, 6, PV::Text(utf16("obj_b")))]),
            el("global_toggle_letterbox", 43, vec![
                param("ma_boolean", "mi_boolean", 32, 1, PV::Word(1)), param("ma_value", "mi_int", 1, 1, PV::Word(2))]),
            el("kingdom_mission_won", 56, vec![kingdom()]),
        ],
    };
    let mut manager = Manager::from_saved(SavedManager { next_id: 8, triggers: vec![trigger], variables: vec![], strings: vec![] });
    let c = json!({
        "time_bits": 0x4120_0000u32,
        "objects": {"kingdom_a": {"kind": "kingdom", "slot": 2, "gen": 1}},
        "objective_defs": [{"name": "obj_a", "text": "A", "flag": 0}, {"name": "obj_b", "text": "B", "flag": 1}],
        "kingdoms": {"kingdom_a": {"objectives": [["obj_a", 1]], "result": 0, "team_members": null}},
        "returns": {"NamedKingdom": {"by_arg": {"kingdom_a": "@kingdom_a"}}, "SortObjectivesVar": 0,
                    "GameVar": {"by_arg": {"TeamVictory": 0}}, "LocalKingdom": "@kingdom_a"}
    });
    let off = Transition { enabled: 0, started_bits: 0, duration_bits: 0 };
    let mut world = Composite { base: FixtureWorld::build(&c), presentation: Presentation { letterbox: off.clone(), screenfade: off } };
    let mut program = Program { core: CoreProgram::new(0, 0, Precision::Double53), world: &mut world };
    let trace = manager.update(&mut program).unwrap().clone();
    assert_eq!(trace.fired, vec![7]);
    assert_eq!(manager.trigger(0).unwrap().enabled, 0);
    let k = world.base.index["kingdom_a"].0;
    assert_eq!(world.base.kingdom_objectives[&k].len(), 2);
    assert_eq!(world.base.results[&k], 1);
    assert_eq!(world.presentation.letterbox.enabled, 1);
    assert_eq!(world.presentation.letterbox.duration_bits, 0x4000_0000);
    // Every registered opcode is ported; an explicit element error (here a
    // non-finite value register) stops the manager.
    let mut stopped = Manager::from_saved(SavedManager {
        next_id: 1,
        triggers: vec![Trigger {
            source: span(), id: 1, mode: 1, enabled: 1, repeat: 0, activation_bits: 0, name: vec![], description: vec![],
            word_2c: 0, events: vec![], conditions: vec![], actions: vec![el("global_adjust_sun_dimmer", 45, vec![
                param("ma_value", "mi_fixed", 2, 2, PV::Word(0x7fc0_0000)),
                param("ma_value", "mi_fixed", 2, 2, PV::Word(0x3f80_0000))])],
        }],
        variables: vec![], strings: vec![],
    });
    let mut program = Program { core: CoreProgram::new(0, 0, Precision::Double53), world: &mut world };
    assert!(stopped.update(&mut program).is_err());
    assert!(stopped.stopped());
}
