use super::*;
use crate::collision::Descriptor;
use crate::index::Member;
use crate::{collision, index::ActorKey, index_world::CollisionPolicy};
use aa_numeric::Precision;
use serde_json::{Value, json};

struct Policy {
    input: Value,
    calls: Vec<Value>,
    fail: bool,
}
impl Policy {
    fn definition(&self, key: u32) -> &Value {
        &self.input["definitions"][key.to_string()]
    }
}
impl CollisionPolicy for Policy {
    fn definition_flags(&self, key: u32) -> collision::Result<u32> {
        Ok(self.definition(key)["flags"].as_u64().unwrap() as u32)
    }
    fn definition_flag_1a8(&self, key: u32) -> collision::Result<bool> {
        Ok(self.definition(key)["flag_1a8"].as_bool().unwrap())
    }
    fn custom_filter(
        &mut self,
        _: &mut SpatialIndex,
        actor: ActorKey,
        d: &mut Descriptor,
    ) -> collision::Result<bool> {
        self.calls
            .push(json!({"op":"custom_filter","actor":actor,"mask":d.mask}));
        Ok(actor == 3)
    }
    fn source_excludes(
        &mut self,
        _: &mut SpatialIndex,
        source: ActorKey,
        actor: ActorKey,
        d: &mut Descriptor,
    ) -> collision::Result<bool> {
        self.calls
            .push(json!({"op":"source_excludes","source":source,"actor":actor,"mask":d.mask}));
        if self.fail {
            return Err(collision::Error::World(
                "required exclusion policy failed".into(),
            ));
        }
        Ok(source == actor
            || self.input["excluded"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v.as_u64() == Some(u64::from(actor))))
    }
    fn allegiance(
        &mut self,
        _: &mut SpatialIndex,
        _: ActorKey,
        _: &mut Descriptor,
    ) -> collision::Result<Option<u32>> {
        panic!("fixture has no hostility context")
    }
    fn hostility_dispatch(&self, _: u32) -> collision::Result<u32> {
        panic!("fixture has no hostility context")
    }
    fn hostile(
        &mut self,
        _: &mut SpatialIndex,
        _: u32,
        _: Option<u32>,
        _: Option<u32>,
        _: &mut Descriptor,
    ) -> collision::Result<bool> {
        panic!("fixture has no hostility context")
    }
    fn moving_exclusion(
        &mut self,
        _: &mut SpatialIndex,
        _: ActorKey,
        _: &mut Descriptor,
    ) -> collision::Result<bool> {
        panic!("fixture does not ignore moving actors")
    }
    fn outer_actor(
        &mut self,
        _: &mut SpatialIndex,
        _: ActorKey,
        _: &mut Descriptor,
    ) -> collision::Result<Option<u32>> {
        panic!("fixture has no collision owner")
    }
}
impl QueryPolicy for Policy {
    fn definition_has_object_data(&self, key: u32) -> collision::Result<bool> {
        Ok(self.definition(key)["object_data"].as_bool().unwrap())
    }
}
fn setup(input: &Value, p: Precision) -> SpatialIndex {
    let mut index = SpatialIndex::new(
        serde_json::from_value(input["grid"].clone()).unwrap(),
        serde_json::from_value(input["flags"].clone()).unwrap(),
        p,
    )
    .unwrap();
    for value in input["members"].as_array().unwrap() {
        let m: Member = serde_json::from_value(value.clone()).unwrap();
        index.register(m).unwrap();
    }
    for value in input["members"].as_array().unwrap() {
        index.insert(value["key"].as_u64().unwrap() as u32).unwrap();
    }
    for value in input["retired"].as_array().unwrap() {
        let key = value.as_u64().unwrap() as u32;
        index.retire_reference(index.member(key).unwrap().reference);
    }
    for value in input["pre_marked"].as_array().unwrap() {
        let key = value.as_u64().unwrap() as u32;
        index.set_query_link(key, Some(key)).unwrap();
    }
    index
}

#[test]
fn original554350_context_and5522b0_facing_match_all_flags_and_precisions() {
    let f: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-spatial-context/native-context.json"
    ))
    .unwrap();
    assert_eq!(f["cases"].as_array().unwrap().len(), 384);
    for (i, c) in f["cases"].as_array().unwrap().iter().enumerate() {
        let input = &c["input"];
        let precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let index = setup(input, precision);
        let policy = Policy {
            input: input.clone(),
            calls: vec![],
            fail: false,
        };
        let mut q = prepare(&index, 1, input["strict"].as_bool().unwrap(), &policy).unwrap();
        assert_eq!(json!(q), c["output"]["context"], "context{i}");
        set_facing(&mut q, serde_json::from_value(c["facing"].clone()).unwrap());
        assert_eq!(json!(q), c["output"]["after_facing"], "facing{i}");
    }
}
#[test]
fn missing_context_actor_is_an_error_not_an_empty_collision_query() {
    let f: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-spatial-context/native-context.json"
    ))
    .unwrap();
    let input = &f["cases"][0]["input"];
    let index = setup(input, Precision::Double53);
    let policy = Policy {
        input: input.clone(),
        calls: vec![],
        fail: false,
    };
    assert!(prepare(&index, 0, false, &policy).is_err());
    assert!(prepare(&index, 999, false, &policy).is_err());
}

#[test]
fn original_forward_caller_goal_store_boundaries_match() {
    let f: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-spatial-context/native-context.json"
    ))
    .unwrap();
    assert_eq!(f["goal_cases"].as_array().unwrap().len(), 672);
    let mut distinct = 0;
    for (i, c) in f["goal_cases"].as_array().unwrap().iter().enumerate() {
        let pos = serde_json::from_value(c["position"].clone()).unwrap();
        let dir = serde_json::from_value(c["direction"].clone()).unwrap();
        let distance = c["distance"].as_u64().unwrap() as u32;
        let p = serde_json::from_value(c["precision"].clone()).unwrap();
        let current = current_actor_goal(pos, dir, distance, p).unwrap();
        let explicit = explicit_origin_goal(pos, dir, distance, p).unwrap();
        let actual = if c["variant"] == "current" {
            current
        } else {
            explicit
        };
        assert_eq!(json!(actual), c["result"], "caller goal{i}");
        distinct += usize::from(current != explicit);
    }
    assert!(
        distinct > 0,
        "corpus must distinguish the two original store patterns"
    );
}
