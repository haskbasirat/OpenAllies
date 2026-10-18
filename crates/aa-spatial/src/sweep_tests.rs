use super::*;
use crate::index::Member;
use crate::{collision, index::ActorKey, index_world::CollisionPolicy};
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

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase7-spatial-sweep/native-sweep.json"
    ))
    .unwrap()
}
#[test]
fn original_sweep_endpoint_and_corridor_queries_match_complete_geometry() {
    let data = fixture();
    assert_eq!(data["cases"].as_array().unwrap().len(), 1024);
    let mut corridor_count = 0;
    for (i, c) in data["cases"].as_array().unwrap().iter().enumerate() {
        let input = &c["input"];
        let precision: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let mut index = setup(input, precision);
        let mut policy = Policy {
            input: input.clone(),
            calls: vec![],
            fail: false,
        };
        let geometry = QueryGeometry::from_native_extents(
            &index,
            serde_json::from_value(input["extent_bits"].clone()).unwrap(),
            precision,
        )
        .unwrap();
        let mut q = modified_query(&index, input, &policy);
        let start = q.placement.position;
        let goal: [u32; 2] = serde_json::from_value(c["goal"].clone()).unwrap();
        let result = sweep(&mut index, &geometry, &mut q, goal, &mut policy).unwrap();
        assert_eq!(json!(result), c["output"]["hit"], "hit case{i}");
        assert_eq!(
            prepared(&q),
            c["output"]["prepared"],
            "external descriptor case{i}"
        );
        assert_eq!(
            json!(policy.calls),
            c["output"]["calls"],
            "policies case{i}"
        );
        if c["output"]["queries"].as_array().unwrap().len() == 2 {
            let delta = [
                b(goal[0])
                    .unwrap()
                    .sub(&b(start[0]).unwrap(), precision)
                    .unwrap()
                    .store_f32()
                    .unwrap(),
                b(goal[1])
                    .unwrap()
                    .sub(&b(start[1]).unwrap(), precision)
                    .unwrap()
                    .store_f32()
                    .unwrap(),
            ];
            let inside = corridor(&q, start, goal, delta, precision).unwrap();
            let expected = &c["output"]["queries"][1];
            assert_eq!(
                prepared(&inside),
                json!({"placement":expected["placement"],"required_actor_flags":expected["required_actor_flags"],"mask":expected["mask"]}),
                "corridor case{i}"
            );
            assert!(inside.descriptor.source_definition.is_none());
            assert!(!inside.descriptor.ignore_all);
            assert!(!inside.descriptor.custom_filter);
            corridor_count += 1;
        }
        for m in index.members() {
            assert_eq!(index.query_link(m.key).unwrap(), None, "marks case{i}");
        }
    }
    assert!(
        corridor_count > 100,
        "coverage must reach real corridor tests"
    );
}
#[test]
fn sweep_error_preserves_endpoint_mutation_and_clears_temporary_marks() {
    let data = fixture();
    let c = &data["cases"][3];
    let input = &c["input"];
    let precision = Precision::Double53;
    let mut index = setup(input, precision);
    let mut policy = Policy {
        input: input.clone(),
        calls: vec![],
        fail: true,
    };
    let mut q = modified_query(&index, input, &policy);
    let geometry = QueryGeometry::from_native_extents(
        &index,
        serde_json::from_value(input["extent_bits"].clone()).unwrap(),
        precision,
    )
    .unwrap();
    let goal = [7.0f32.to_bits(), 5.0f32.to_bits()];
    assert!(sweep(&mut index, &geometry, &mut q, goal, &mut policy).is_err());
    assert_eq!(q.placement.position, goal);
    for m in index.members() {
        assert_eq!(index.query_link(m.key).unwrap(), None);
    }
}
