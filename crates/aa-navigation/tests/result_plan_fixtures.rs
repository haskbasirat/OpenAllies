use aa_navigation::{result_plan::*, solver};
use aa_numeric::Precision;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;
const TARGET: u32 = 0x2021000;
const REPLACEMENT: u32 = 0x2022000;
const HIT: u32 = 0x2023000;
#[derive(Clone, Deserialize)]
struct Case {
    name: String,
    precision: Precision,
    state: State,
    actor: Actor,
    trigger: Option<Object>,
    argument: u32,
    special_types: BTreeMap<String, bool>,
    special_components: BTreeMap<String, bool>,
    target_valid: bool,
    replacement: bool,
    regions: Vec<i32>,
    properties: Vec<u32>,
    property_active: bool,
    context_words: [u32; 22],
    hit: CollisionHit,
    solve_result: i32,
    mutations: BTreeMap<String, Value>,
}
struct Host {
    c: Case,
    events: Vec<Value>,
    region: usize,
    property: usize,
}
impl Host {
    fn event(&mut self, event: Value, state: &mut State, actor: &mut Actor) {
        let name = event["name"].as_str().unwrap().to_owned();
        self.events.push(event);
        if let Some(change) = self.c.mutations.get(&name) {
            if let Some(fields) = change.get("movement") {
                let mut val = serde_json::to_value(&state.movement).unwrap();
                for (k, v) in fields.as_object().unwrap() {
                    val[k] = v.clone();
                }
                state.movement = serde_json::from_value(val).unwrap();
            }
            if let Some(fields) = change.get("actor") {
                let mut val = serde_json::to_value(&*actor).unwrap();
                for (k, v) in fields.as_object().unwrap() {
                    val[k] = v.clone();
                }
                *actor = serde_json::from_value(val).unwrap();
            }
        }
    }
}
impl World for Host {
    fn special_type(
        &mut self,
        object: Object,
        state: &mut State,
        actor: &mut Actor,
    ) -> Result<bool> {
        let result = *self
            .c
            .special_types
            .get(&object.0.to_string())
            .unwrap_or(&false);
        self.event(
            json!({"name":"SpecialType","object":object,"result":result}),
            state,
            actor,
        );
        Ok(result)
    }
    fn has_special_component(&self, object: Object) -> bool {
        *self
            .c
            .special_components
            .get(&object.0.to_string())
            .unwrap_or(&false)
    }
    fn region_at(&mut self, point: [u32; 2], state: &mut State, actor: &mut Actor) -> Result<i32> {
        let result = self.c.regions[self.region.min(self.c.regions.len() - 1)];
        self.region += 1;
        self.event(
            json!({"name":"Region","point":point,"result":result}),
            state,
            actor,
        );
        Ok(result)
    }
    fn collision_context(
        &mut self,
        object: Object,
        state: &mut State,
        actor: &mut Actor,
    ) -> Result<CollisionContext> {
        self.event(json!({"name":"Context","object":object}), state, actor);
        Ok(CollisionContext {
            words: self.c.context_words,
        })
    }
    fn resolve_actor(&self, id: u32) -> Option<Object> {
        match id {
            0x10007 if self.c.target_valid => Some(Object(TARGET)),
            0x10008 => Some(Object(HIT)),
            _ => None,
        }
    }
    fn replacement_actor(&self, object: Object) -> Object {
        if object.0 == TARGET && self.c.replacement {
            Object(REPLACEMENT)
        } else {
            object
        }
    }
    fn object_definition(&self, object: Object) -> u32 {
        0x77770000 + object.0
    }
    fn sweep(
        &mut self,
        context: &mut CollisionContext,
        goal: [u32; 2],
        state: &mut State,
        actor: &mut Actor,
    ) -> Result<CollisionHit> {
        self.event(
            json!({"name":"Sweep","context":context.words,"goal":goal}),
            state,
            actor,
        );
        Ok(self.c.hit)
    }
    fn continue_turn(&mut self, turn: bool, state: &mut State, actor: &mut Actor) -> Result<()> {
        self.event(json!({"name":"ContinueTurn","turn":turn}), state, actor);
        Ok(())
    }
    fn continue_current(
        &mut self,
        argument: u32,
        state: &mut State,
        actor: &mut Actor,
    ) -> Result<()> {
        self.event(
            json!({"name":"ContinueCurrent","argument":argument}),
            state,
            actor,
        );
        Ok(())
    }
    fn property(&mut self, object: Object, state: &mut State, actor: &mut Actor) -> Result<u32> {
        let result = self.c.properties[self.property.min(self.c.properties.len() - 1)];
        self.property += 1;
        self.event(
            json!({"name":"Property","object":object,"result":result}),
            state,
            actor,
        );
        Ok(result)
    }
    fn property_active(
        &mut self,
        property: u32,
        state: &mut State,
        actor: &mut Actor,
    ) -> Result<bool> {
        let result = self.c.property_active;
        self.event(
            json!({"name":"PropertyActive","property":property,"result":result}),
            state,
            actor,
        );
        Ok(result)
    }
    fn solve(
        &mut self,
        context: &mut CollisionContext,
        request: solver::Request,
        state: &mut State,
        actor: &mut Actor,
    ) -> Result<i32> {
        let result = self.c.solve_result;
        self.event(
            json!({"name":"Solve","context":context.words,"request":request,"result":result}),
            state,
            actor,
        );
        Ok(result)
    }
    fn update_waypoint_offset(&mut self, state: &mut State, actor: &mut Actor) -> Result<()> {
        self.event(json!({"name":"Offset"}), state, actor);
        Ok(())
    }
    fn action_go(&mut self, speed_bits: u32, state: &mut State, actor: &mut Actor) -> Result<()> {
        self.event(
            json!({"name":"ActionGo","speed_bits":speed_bits}),
            state,
            actor,
        );
        Ok(())
    }
    fn failed(&mut self, state: &mut State, actor: &mut Actor) -> Result<()> {
        self.event(json!({"name":"Failed"}), state, actor);
        Ok(())
    }
}
#[test]
fn result_plan_native_attempts_turn_geometry_and_dispatch_match() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase6-navigation/result-plan-fixtures.json"
    ))
    .unwrap();
    for row in doc["fixtures"].as_array().unwrap() {
        let c: Case = serde_json::from_value(row["input"].clone()).unwrap();
        let mut state = c.state.clone();
        let mut actor = c.actor.clone();
        let mut host = Host {
            c: c.clone(),
            events: vec![],
            region: 0,
            property: 0,
        };
        result_plan(
            &mut state,
            &mut actor,
            c.trigger,
            c.argument,
            c.precision,
            &mut host,
        )
        .unwrap_or_else(|e| panic!("{}: {e}", c.name));
        assert_eq!(
            json!({"state":state,"actor":actor,"events":host.events,"abi_preserved":true}),
            row["output"],
            "{}",
            c.name
        );
    }
}
