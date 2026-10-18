use super::*;
use crate::index::Member;
use serde_json::{Value, json};
use std::collections::BTreeMap;

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
fn prepared(q: &PreparedQuery) -> Value {
    json!({"placement":q.placement,"required_actor_flags":q.required_actor_flags,"mask":q.descriptor.mask})
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
fn modified_query(index: &SpatialIndex, input: &Value, policy: &Policy) -> PreparedQuery {
    let mut q = PreparedQuery::for_actor(
        index,
        1,
        input["strict"].as_bool().unwrap(),
        input["extra_flags"].as_u64().unwrap() as u32,
        policy,
    )
    .unwrap();
    q.descriptor.ignore_actor = serde_json::from_value(input["ignore_actor"].clone()).unwrap();
    q.descriptor.ignore_definition = input["ignore_definition_actor"]
        .as_u64()
        .map(|key| index.member(key as u32).unwrap().actor.definition);
    q.descriptor.ignore_all = input["ignore_all"].as_bool().unwrap();
    q.descriptor.custom_filter = input["custom_filter"].as_bool().unwrap();
    q
}
#[test]
fn full_native_first_hit_matches_preparation_scan_filter_marks_and_shapes() {
    let f: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-spatial/native-query.json"
    ))
    .unwrap();
    assert_eq!(f["cases"].as_array().unwrap().len(), 384);
    let mut count = 0;
    for (case_id, case) in f["cases"].as_array().unwrap().iter().enumerate() {
        let input = &case["input"];
        let p: Precision = serde_json::from_value(case["precision"].clone()).unwrap();
        let mut index = setup(input, p);
        let mut policy = Policy {
            input: input.clone(),
            calls: Vec::new(),
            fail: false,
        };
        let geometry = QueryGeometry::from_native_extents(
            &index,
            serde_json::from_value(input["extent_bits"].clone()).unwrap(),
            p,
        )
        .unwrap();
        assert_eq!(json!(geometry), case["geometry"], "case{case_id} cellshape");
        let initial = PreparedQuery::for_actor(
            &index,
            1,
            input["strict"].as_bool().unwrap(),
            input["extra_flags"].as_u64().unwrap() as u32,
            &policy,
        )
        .unwrap();
        assert_eq!(prepared(&initial), case["initial"], "case{case_id} initial");
        let mut q = modified_query(&index, input, &policy);
        for (step_id, step) in case["steps"].as_array().unwrap().iter().enumerate() {
            q.set_position(
                &index,
                serde_json::from_value(step["point"].clone()).unwrap(),
                &policy,
            )
            .unwrap();
            let before = prepared(&q);
            let hit = first_hit(&mut index, &geometry, &q, &mut policy).unwrap();
            let links: BTreeMap<String, Option<ActorKey>> = index
                .members()
                .map(|m| (m.key.to_string(), index.query_link(m.key).unwrap()))
                .collect();
            let actual = json!({"prepared":before,"hit":hit,"calls":std::mem::take(&mut policy.calls),"links":links});
            assert_eq!(actual, step["native"], "case{case_id} step{step_id}");
            assert_eq!(
                prepared(&q),
                before,
                "temporary iterator must not mutate external descriptor"
            );
            count += 1;
        }
    }
    assert_eq!(count, 2304);
}
#[test]
fn query_policy_failure_releases_own_marks_and_keeps_outer_marks() {
    let f: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-spatial/native-query.json"
    ))
    .unwrap();
    let input = &f["cases"][2]["input"];
    let p = Precision::Double53;
    let mut index = setup(input, p);
    let mut policy = Policy {
        input: input.clone(),
        calls: Vec::new(),
        fail: true,
    };
    let geometry = QueryGeometry::from_native_extents(
        &index,
        serde_json::from_value(input["extent_bits"].clone()).unwrap(),
        p,
    )
    .unwrap();
    let q = modified_query(&index, input, &policy);
    assert!(first_hit(&mut index, &geometry, &q, &mut policy).is_err());
    assert!(!policy.calls.is_empty());
    for key in 1..=6 {
        assert_eq!(
            index.query_link(key).unwrap(),
            if key == 3 { Some(3) } else { None }
        );
    }
    assert!(
        index.unregister(3).is_err(),
        "active outer query owns this actor"
    );
}
