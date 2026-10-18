//! Replays research/phase8-movement/native-overlaps.json: original 555620
//! (and its tile iterator/marks/exclusion/overlap) plus 4D67F0 refreshes on
//! native-built tile indexes, under PC24/53/64.
use super::*;
use crate::collision::Descriptor;
use crate::index::Member;
use crate::index_world::CollisionPolicy;
use aa_numeric::Precision;
use serde_json::{Value, json};

struct Policy {
    input: Value,
    calls: Vec<Value>,
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
    fn custom_filter(&mut self, _: &mut SpatialIndex, actor: ActorKey, d: &mut Descriptor) -> collision::Result<bool> {
        self.calls.push(json!({"op":"custom_filter","actor":actor,"mask":d.mask}));
        Ok(actor == 3)
    }
    fn source_excludes(
        &mut self,
        _: &mut SpatialIndex,
        source: ActorKey,
        actor: ActorKey,
        d: &mut Descriptor,
    ) -> collision::Result<bool> {
        self.calls.push(json!({"op":"source_excludes","source":source,"actor":actor,"mask":d.mask}));
        Ok(source == actor
            || self.input["excluded"].as_array().unwrap().iter().any(|v| v.as_u64() == Some(u64::from(actor))))
    }
    fn allegiance(&mut self, _: &mut SpatialIndex, _: ActorKey, _: &mut Descriptor) -> collision::Result<Option<u32>> {
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
    fn moving_exclusion(&mut self, _: &mut SpatialIndex, _: ActorKey, _: &mut Descriptor) -> collision::Result<bool> {
        panic!("fixture does not ignore moving actors")
    }
    fn outer_actor(&mut self, _: &mut SpatialIndex, _: ActorKey, _: &mut Descriptor) -> collision::Result<Option<u32>> {
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
fn links(index: &SpatialIndex) -> Value {
    let mut out = serde_json::Map::new();
    for m in index.members() {
        out.insert(m.key.to_string(), json!(index.query_link(m.key).unwrap()));
    }
    Value::Object(out)
}

#[test]
fn native_all_overlaps_and_refresh_match_original_x86() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-movement/native-overlaps.json");
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let mut nonempty = 0;
    let mut refreshed = 0;
    for case in doc["cases"].as_array().unwrap() {
        let p: Precision = serde_json::from_value(case["precision"].clone()).unwrap();
        let input = &case["input"];
        let mut index = setup(input, p);
        let geometry: QueryGeometry = serde_json::from_value(case["geometry"].clone()).unwrap();
        let mut policy = Policy { input: input.clone(), calls: vec![] };
        let mut query = PreparedQuery::for_actor(&index, 1, input["strict"].as_bool().unwrap(),
            input["extra_flags"].as_u64().unwrap() as u32, &policy).unwrap();
        query.descriptor.ignore_actor = serde_json::from_value(input["ignore_actor"].clone()).unwrap();
        query.descriptor.ignore_definition = input["ignore_definition_actor"]
            .as_u64()
            .map(|key| index.member(key as u32).unwrap().actor.definition);
        query.descriptor.ignore_all = input["ignore_all"].as_bool().unwrap();
        query.descriptor.custom_filter = input["custom_filter"].as_bool().unwrap();
        for step in case["steps"].as_array().unwrap() {
            let point: [u32; 2] = serde_json::from_value(step["point"].clone()).unwrap();
            query.set_position(&index, point, &policy).unwrap();
            policy.calls.clear();
            let native = &step["native"];
            assert_eq!(serde_json::to_value(&query.placement).unwrap(), native["prepared"]["placement"]);
            let found = all_overlaps(&mut index, &geometry, &mut query, step["flag10_only"] == 1, &mut policy).unwrap();
            assert_eq!(json!(found), native["references"], "{p:?} {step}");
            assert_eq!(json!(policy.calls), native["calls"], "{p:?} calls {step}");
            assert_eq!(links(&index), native["links"], "{p:?} links");
            nonempty += usize::from(!found.is_empty());
        }
        for refresh in case["refreshes"].as_array().unwrap() {
            index.member_mut(1).unwrap().actor.flags = refresh["actor_flags"].as_u64().unwrap() as u32;
            let mut entries: Vec<OverlapEntry> = refresh["entries"]
                .as_array()
                .unwrap()
                .iter()
                .map(|e| OverlapEntry { reference: e[0].as_u64().unwrap() as u32, expiry_bits: e[1].as_u64().unwrap() as u32 })
                .collect();
            let eligible = refresh["eligible"].as_bool().unwrap();
            let mut events = vec![];
            policy.calls.clear();
            let ran = refresh_overlaps(&mut index, &geometry, 1, &mut entries,
                refresh["time_bits"].as_u64().unwrap() as u32,
                &mut |_, _| {
                    events.push(json!({"op":"eligible","actor":1}));
                    Ok(eligible)
                },
                &mut policy).unwrap();
            let native = &refresh["native"];
            let got: Vec<Value> = entries.iter().map(|e| json!([e.reference, e.expiry_bits])).collect();
            assert_eq!(Value::Array(got), native["entries"], "{p:?} refresh {refresh}");
            assert_eq!(Value::Array(events), native["events"]);
            assert_eq!(json!(policy.calls), native["calls"], "{p:?} refresh calls");
            assert_eq!(links(&index), native["links"]);
            refreshed += usize::from(ran);
        }
    }
    assert!(nonempty > 300, "{nonempty}");
    assert!(refreshed > 100, "{refreshed}");
}
