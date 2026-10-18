//! Replays research/phase8-movement/native-placement.json: original 554850
//! free-placement search (552F70, 552250, 553BA0, table rotations) on
//! native-built tile indexes, under PC24/53/64.
use super::*;
use crate::collision;
use crate::index::ActorKey;
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
fn native_free_placement_search_matches_original_x86() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-movement/native-placement.json");
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    for table in doc["tables"].as_object().unwrap().values() {
        assert_eq!(table["sin"].as_u64().unwrap() as u32, ROTATION_SIN_BITS);
        assert_eq!(table["cos"].as_u64().unwrap() as u32, ROTATION_COS_BITS);
    }
    let mut moved = 0;
    for case in doc["cases"].as_array().unwrap() {
        let p: Precision = serde_json::from_value(case["precision"].clone()).unwrap();
        let input = &case["input"];
        let mut index = setup(input, p);
        let geometry: QueryGeometry = serde_json::from_value(case["geometry"].clone()).unwrap();
        let mut policy = Policy { input: input.clone(), calls: vec![] };
        for s in case["searches"].as_array().unwrap() {
            let w: Vec<u32> = s["shape"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as u32).collect();
            let search = PlacementSearch {
                shape: crate::collision::Shape { kind: w[0], radius_bits: w[1], bounds: [w[2], w[3], w[4], w[5]] },
                radius_bits: s["radius"].as_u64().unwrap() as u32,
                rings: s["rings"].as_i64().unwrap() as i32,
                keep_direction: s["keep"].as_u64().unwrap() != 0,
                strict: s["strict"].as_u64().unwrap() != 0,
                large: s["large"].as_u64().unwrap() != 0,
            };
            let mut point: [u32; 2] = serde_json::from_value(s["point"].clone()).unwrap();
            let mut direction: [u32; 2] = serde_json::from_value(s["direction"].clone()).unwrap();
            policy.calls.clear();
            let found = find_free_placement(&mut index, &geometry, &mut point, &mut direction, &search, &mut policy).unwrap();
            let native = &s["native"];
            assert_eq!(found as u64, native["found"].as_u64().unwrap(), "{p:?} {s}");
            assert_eq!(json!(point), native["point"], "{p:?} point {s}");
            assert_eq!(json!(direction), native["direction"], "{p:?} direction {s}");
            assert_eq!(json!(policy.calls), native["calls"], "{p:?} calls");
            assert_eq!(links(&index), native["links"]);
            moved += usize::from(json!(point) != s["point"]);
        }
        for s in case["distance_searches"].as_array().unwrap() {
            let w: Vec<u32> = s["shape"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as u32).collect();
            let shape = crate::collision::Shape { kind: w[0], radius_bits: w[1], bounds: [w[2], w[3], w[4], w[5]] };
            let mut point: [u32; 2] = serde_json::from_value(s["point"].clone()).unwrap();
            let mut direction: [u32; 2] = serde_json::from_value(s["direction"].clone()).unwrap();
            policy.calls.clear();
            let found = find_placement_within(&mut index, &geometry, &mut point, &mut direction, shape,
                s["definition_radius"].as_u64().unwrap() as u32, s["distance"].as_u64().unwrap() as u32,
                s["strict"].as_u64().unwrap() != 0, s["definition_flags"].as_u64().unwrap() & (1 << 21) != 0,
                &mut policy).unwrap();
            let native = &s["native"];
            assert_eq!(found as u64, native["found"].as_u64().unwrap(), "{p:?} {s}");
            assert_eq!(json!(point), native["point"], "{p:?} within {s}");
            assert_eq!(json!(direction), native["direction"], "{p:?}");
            assert_eq!(json!(policy.calls), native["calls"], "{p:?}");
        }
    }
    assert!(moved > 50, "{moved}");
}

#[test]
fn wrapper_radius_floors_at_quarter_cell() {
    let p = Precision::Double53;
    assert_eq!(wrapper_radius(0.25f32.to_bits(), 2f32.to_bits(), p).unwrap(), 0.5f32.to_bits());
    assert_eq!(wrapper_radius(0.75f32.to_bits(), 2f32.to_bits(), p).unwrap(), 0.75f32.to_bits());
}

