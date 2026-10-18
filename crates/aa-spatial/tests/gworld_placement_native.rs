//! Replays research/phase8-movement/native-gworld-placement.json: original
//! x86 execution (Unicorn, PC24/53/64) of the GWorld placement test 0x4F3C40
//! and the structure-crossing test 0x4F22C0 with their pure leaves 0x42A530,
//! 0x54ECF0 and _ftol 0x96CC4C. Every return value, native fault and recorded
//! leaf call / world-data read (in native order) must match. The composed
//! group runs the real 0x553BA0 under the real 0x4F3C40 and is replayed
//! through aa_spatial::query::first_hit. native-gworld-placement-callers.json
//! covers the callers 0x53CE40 (destination acceptance) and 0x4F46E0
//! (nearest placeable position, real -pi/4 table lookups) with recorded
//! 0x552250 / 0x54F370 / 0x4F3C40 / 0x554350 seams.
use aa_numeric::Precision;
use aa_spatial::{
    collision::{self, Descriptor, Shape},
    geometry::Placement,
    gworld_placement::{
        CrossingWorld, DescriptorWorld, EndpointWorld, Error, GMapView, NearestPlacementWorld,
        PlacementWorld, Result, Verdict, endpoint_acceptable, nearest_placement, placement_result,
        structure_crossing,
    },
    placement::{ROTATION_COS_BITS, ROTATION_SIN_BITS},
    index::{ActorKey, Member, SpatialIndex},
    index_world::CollisionPolicy,
    query::{self, Hit, PreparedQuery, QueryGeometry, QueryPolicy},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn load(name: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../research/phase8-movement").join(name);
    serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).expect("json")
}
fn document() -> Value {
    load("native-gworld-placement.json")
}
fn u(v: &Value) -> u32 {
    v.as_u64().expect("u32") as u32
}
fn bits2(v: &Value) -> [u32; 2] {
    [u(&v[0]), u(&v[1])]
}
fn key(v: &Value) -> Option<ActorKey> {
    Some(u(v)).filter(|k| *k != 0)
}
fn al(raw: u32) -> bool {
    raw & 0xff != 0
}

/// The GMap view from the oracle's GMap words.
fn gmap(map: &Value, extent: [u32; 2]) -> GMapView {
    GMapView {
        width: u(&map["width"]) as i32,
        height: u(&map["height"]) as i32,
        coordinate_shift: u(&map["shift"]),
        cell_size_bits: u(&map["cell_bits"]),
        extent_bits: extent,
    }
}

/// GMap regions as the oracle laid them out: tile +0x10 node, node +8 word.
struct Regions {
    tile_nodes: BTreeMap<usize, u32>,
    node_regions: BTreeMap<u32, u32>,
}
impl Regions {
    fn new(case: &Value) -> Self {
        let pairs = |v: &Value| -> Vec<(u32, u32)> {
            v.as_array().unwrap().iter().map(|p| (u(&p[0]), u(&p[1]))).collect()
        };
        Self {
            tile_nodes: pairs(&case["tile_nodes"]).into_iter().map(|(t, n)| (t as usize, n)).collect(),
            node_regions: pairs(&case["node_regions"]).into_iter().collect(),
        }
    }
    fn node(&self, tile: usize, events: &mut Vec<Value>) -> Option<u32> {
        let node = self.tile_nodes.get(&tile).copied();
        events.push(json!(["tile_node", tile, node.unwrap_or(0)]));
        node
    }
    fn region(&self, node: u32, events: &mut Vec<Value>) -> Option<u32> {
        let region = *self.node_regions.get(&node).expect("region node laid out");
        events.push(json!(["node_region", node, region]));
        Some(region).filter(|r| *r != 0)
    }
}

/// The seamed group's world: every answer is the raw EAX the oracle seam
/// returned; booleans are the native AL test.
struct Seamed<'a> {
    case: &'a Value,
    regions: Regions,
    events: Vec<Value>,
}
impl Seamed<'_> {
    fn answer(&self, table: &str, keys: &[u32]) -> u32 {
        let rows = self.case["answers"][table].as_array().unwrap();
        let row = rows
            .iter()
            .find(|r| keys.iter().enumerate().all(|(i, k)| u(&r[i]) == *k))
            .unwrap_or_else(|| panic!("{} has no {table} answer for {keys:?}", self.case["name"]));
        u(&row[keys.len()])
    }
    fn flag(&mut self, table: &str, keys: &[u32]) -> bool {
        let r = al(self.answer(table, keys));
        let mut row = vec![json!(table)];
        row.extend(keys.iter().map(|k| json!(k)));
        row.push(json!(u32::from(r)));
        self.events.push(Value::Array(row));
        r
    }
    fn object(&self, actor: ActorKey) -> &Value {
        self.case["objects"]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| u(&o["key"]) == actor)
            .expect("object")
    }
}
impl CrossingWorld for Seamed<'_> {
    fn actor_virtual_b4(&mut self, actor: ActorKey) -> Result<bool> {
        Ok(self.flag("actor_b4", &[actor]))
    }
    fn structure_at(&mut self, point: [u32; 2]) -> Result<Option<ActorKey>> {
        let k = self.answer("structure_at", &point);
        self.events.push(json!(["structure_at", point[0], point[1], k]));
        Ok(Some(k).filter(|k| *k != 0))
    }
    fn actor_4d36f0(&mut self, actor: ActorKey, structure: ActorKey) -> Result<bool> {
        Ok(self.flag("actor_4d36f0", &[actor, structure]))
    }
    fn structure_virtual_168(&mut self, structure: ActorKey, actor: ActorKey) -> Result<bool> {
        Ok(self.flag("structure_168", &[structure, actor]))
    }
    fn footprint_585280(&mut self, structure: ActorKey, actor: ActorKey) -> Result<bool> {
        Ok(self.flag("footprint_585280", &[structure, actor]))
    }
    fn actor_word_90_present(&mut self, actor: ActorKey) -> Result<bool> {
        let present = u(&self.object(actor)["word90"]) != 0;
        self.events.push(json!(["actor_word_90", actor, u32::from(present)]));
        Ok(present)
    }
    fn actor_virtual_100(&mut self, actor: ActorKey) -> Result<Option<ActorKey>> {
        let k = self.answer("actor_100", &[actor]);
        self.events.push(json!(["actor_100", actor, k]));
        Ok(Some(k).filter(|k| *k != 0))
    }
    fn footprint_585160(&mut self, structure: ActorKey) -> Result<bool> {
        Ok(self.flag("footprint_585160", &[structure]))
    }
    fn actor_virtual_164(&mut self, actor: ActorKey, structure: ActorKey) -> Result<bool> {
        Ok(self.flag("actor_164", &[actor, structure]))
    }
}
impl PlacementWorld for Seamed<'_> {
    fn actor_position(&mut self, actor: ActorKey) -> Result<[u32; 2]> {
        self.events.push(json!(["actor_position", actor]));
        Ok(bits2(&self.object(actor)["position"]))
    }
    fn tile_region_node(&mut self, tile: usize) -> Result<Option<u32>> {
        Ok(self.regions.node(tile, &mut self.events))
    }
    fn region_node_region(&mut self, node: u32) -> Result<Option<u32>> {
        Ok(self.regions.region(node, &mut self.events))
    }
    fn first_hit(&mut self, query: &PreparedQuery) -> Result<Hit> {
        assert_eq!(query.placement.position, bits2(&self.case["descriptor"]["position"]));
        let h = &self.case["first_hit"];
        let hit = Hit {
            kind: u(&h["kind"]),
            reference: u(&h["reference"]),
            cell: [h["cell"][0].as_i64().unwrap() as i32, h["cell"][1].as_i64().unwrap() as i32],
        };
        self.events.push(json!(["first_hit", hit.kind, hit.reference, hit.cell[0], hit.cell[1]]));
        Ok(hit)
    }
}

/// Descriptor words the port reads (+0x1C, +0x20, +0x28, +0x3C, +0x4C/+0x50).
/// The oracle filled every other word with junk the native never reads.
fn seamed_query(case: &Value) -> PreparedQuery {
    let d = &case["descriptor"];
    PreparedQuery {
        placement: Placement {
            shape: Shape { kind: 0, radius_bits: 0, bounds: [0; 4] },
            facing: [0, 0],
            position: bits2(&d["position"]),
        },
        descriptor: Descriptor {
            source_actor: key(&case["source"]),
            ignore_actor: key(&d["ignore_actor"]),
            collision_owner: key(&d["collision_owner"]),
            mask: u(&d["mask"]),
            ..Descriptor::default()
        },
        required_actor_flags: 0,
    }
}

#[test]
fn seamed_group_matches_original_x86() {
    let doc = document();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len() as u64, doc["case_count"].as_u64().unwrap());
    let mut per_precision: BTreeMap<String, usize> = BTreeMap::new();
    let mut codes: BTreeMap<String, usize> = BTreeMap::new();
    let mut refused = 0;
    for case in cases {
        *per_precision.entry(case["precision"].as_str().unwrap().into()).or_default() += 1;
        let native = &case["output"];
        let query = seamed_query(case);
        let mut world = Seamed { case, regions: Regions::new(case), events: vec![] };
        let name = &case["name"];
        match case["entry"].as_str().unwrap() {
            "4f3c40" => {
                let map = gmap(&case["map"], bits2(&case["map"]["extent_bits"]));
                let result = placement_result(&map, key(&case["source"]), &query, &mut world);
                if native["fault"].as_str().is_some() {
                    assert_eq!(native["fault"], "0x20", "{name}");
                    assert!(matches!(result, Err(Error::NullSource)), "{name}: {result:?}");
                    *codes.entry("4f3c40:fault".into()).or_default() += 1;
                } else if case["rust_tile_domain"] == json!(false) && matches!(result, Err(Error::Tile(_))) {
                    // Native _ftol gives the low dword 0 for NaN, infinities and
                    // multiples of 2^32; the reused tile lookup refuses them, and
                    // only after the native would have read the source position.
                    let native_events = native["events"].as_array().unwrap();
                    assert_eq!(world.events, [json!(["actor_position", case["source"]])], "{name}");
                    assert!(native_events.starts_with(&world.events), "{name}");
                    refused += 1;
                } else {
                    let verdict: Verdict = result.unwrap_or_else(|e| panic!("{name}: {e}"));
                    assert_eq!(u64::from(verdict.code()), native["result"].as_u64().unwrap(), "{name}");
                    assert_eq!(json!(world.events), native["events"], "{name}");
                    *codes.entry(format!("4f3c40:{}", verdict.code())).or_default() += 1;
                }
            }
            "4f22c0" => {
                let c = &case["crossing"];
                let crossing = structure_crossing(
                    key(&case["source"]),
                    bits2(&c["from"]),
                    bits2(&c["to"]),
                    &query.descriptor,
                    &mut world,
                )
                .unwrap();
                let code = if crossing { 2 } else { 0 };
                assert_eq!(code, native["result"].as_u64().unwrap(), "{name}");
                assert_eq!(json!(world.events), native["events"], "{name}");
                *codes.entry(format!("4f22c0:{code}")).or_default() += 1;
            }
            other => panic!("entry {other}"),
        }
    }
    assert_eq!(per_precision.len(), 3, "{per_precision:?}");
    assert!(per_precision.values().all(|n| *n == cases.len() / 3), "{per_precision:?}");
    for code in ["4f3c40:0", "4f3c40:1", "4f3c40:2", "4f3c40:3", "4f3c40:4", "4f3c40:fault", "4f22c0:0", "4f22c0:2"] {
        assert!(codes.get(code).copied().unwrap_or(0) > 0, "no {code} rows: {codes:?}");
    }
    assert_eq!(refused, 12, "{codes:?}");
}

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
        Ok(u(&self.definition(key)["flags"]))
    }
    fn definition_flag_1a8(&self, key: u32) -> collision::Result<bool> {
        Ok(self.definition(key)["flag_1a8"].as_bool().unwrap())
    }
    fn custom_filter(&mut self, _: &mut SpatialIndex, actor: ActorKey, d: &mut Descriptor) -> collision::Result<bool> {
        self.calls.push(json!({"op": "custom_filter", "actor": actor, "mask": d.mask}));
        Ok(actor == 3)
    }
    fn source_excludes(
        &mut self,
        _: &mut SpatialIndex,
        source: ActorKey,
        actor: ActorKey,
        d: &mut Descriptor,
    ) -> collision::Result<bool> {
        self.calls.push(json!({"op": "source_excludes", "source": source, "actor": actor, "mask": d.mask}));
        Ok(source == actor || self.input["excluded"].as_array().unwrap().iter().any(|v| u(v) == actor))
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

/// The composed group's world: the spatial index answers actor data and
/// virtual +0xB4, 0x4F2220 is the oracle's null seam, and 0x553BA0 is the
/// ported `query::first_hit`.
struct Composed<'a> {
    index: &'a mut SpatialIndex,
    geometry: &'a QueryGeometry,
    policy: &'a mut Policy,
    regions: Regions,
    events: Vec<Value>,
}
impl CrossingWorld for Composed<'_> {
    fn actor_virtual_b4(&mut self, actor: ActorKey) -> Result<bool> {
        let category = self.index.member(actor)?.actor_category;
        self.events.push(json!(["actor_b4", actor, u32::from(category)]));
        Ok(category)
    }
    fn structure_at(&mut self, point: [u32; 2]) -> Result<Option<ActorKey>> {
        self.events.push(json!(["structure_at", point[0], point[1], 0]));
        Ok(None)
    }
    fn actor_4d36f0(&mut self, _: ActorKey, _: ActorKey) -> Result<bool> {
        panic!("no structure in the composed fixture")
    }
    fn structure_virtual_168(&mut self, _: ActorKey, _: ActorKey) -> Result<bool> {
        panic!("no structure in the composed fixture")
    }
    fn footprint_585280(&mut self, _: ActorKey, _: ActorKey) -> Result<bool> {
        panic!("no structure in the composed fixture")
    }
    fn actor_word_90_present(&mut self, _: ActorKey) -> Result<bool> {
        panic!("no structure in the composed fixture")
    }
    fn actor_virtual_100(&mut self, _: ActorKey) -> Result<Option<ActorKey>> {
        panic!("no structure in the composed fixture")
    }
    fn footprint_585160(&mut self, _: ActorKey) -> Result<bool> {
        panic!("no structure in the composed fixture")
    }
    fn actor_virtual_164(&mut self, _: ActorKey, _: ActorKey) -> Result<bool> {
        panic!("no structure in the composed fixture")
    }
}
impl PlacementWorld for Composed<'_> {
    fn actor_position(&mut self, actor: ActorKey) -> Result<[u32; 2]> {
        self.events.push(json!(["actor_position", actor]));
        Ok(self.index.member(actor)?.actor.position)
    }
    fn tile_region_node(&mut self, tile: usize) -> Result<Option<u32>> {
        Ok(self.regions.node(tile, &mut self.events))
    }
    fn region_node_region(&mut self, node: u32) -> Result<Option<u32>> {
        Ok(self.regions.region(node, &mut self.events))
    }
    fn first_hit(&mut self, query: &PreparedQuery) -> Result<Hit> {
        let hit = query::first_hit(self.index, self.geometry, query, self.policy)?;
        self.events.push(json!(["first_hit", hit.kind, hit.reference, hit.cell[0], hit.cell[1]]));
        Ok(hit)
    }
}

#[test]
fn composed_group_matches_original_x86_with_real_first_hit() {
    let doc = document();
    let mut codes: BTreeMap<u32, usize> = BTreeMap::new();
    let mut steps = 0;
    for case in doc["composed_cases"].as_array().unwrap() {
        let p: Precision = serde_json::from_value(case["precision"].clone()).unwrap();
        let input = &case["input"];
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
            index.insert(u(&value["key"])).unwrap();
        }
        for value in input["retired"].as_array().unwrap() {
            let reference = index.member(u(value)).unwrap().reference;
            index.retire_reference(reference);
        }
        let geometry: QueryGeometry = serde_json::from_value(case["geometry"].clone()).unwrap();
        // The runtime's view: SpatialIndex grid words and QueryGeometry extents.
        let grid = index.grid();
        let map = GMapView {
            width: grid.dimensions[0],
            height: grid.dimensions[1],
            coordinate_shift: grid.coordinate_shift,
            cell_size_bits: grid.cell_size_bits,
            extent_bits: geometry.extent_bits,
        };
        let mut policy = Policy { input: input.clone(), calls: vec![] };
        for step in case["steps"].as_array().unwrap() {
            let query: PreparedQuery = serde_json::from_value(step["descriptor"].clone()).unwrap();
            assert_eq!(query.placement.position.len(), 2);
            policy.calls.clear();
            let mut world = Composed {
                index: &mut index,
                geometry: &geometry,
                policy: &mut policy,
                regions: Regions::new(input),
                events: vec![],
            };
            let verdict = placement_result(&map, Some(1), &query, &mut world)
                .unwrap_or_else(|e| panic!("{p:?} {}: {e}", step["point"]));
            let events = std::mem::take(&mut world.events);
            assert_eq!(u64::from(verdict.code()), step["result"].as_u64().unwrap(), "{p:?} {}", step["point"]);
            assert_eq!(json!(events), step["events"], "{p:?} {}", step["point"]);
            assert_eq!(json!(policy.calls), step["calls"], "{p:?} {}", step["point"]);
            *codes.entry(verdict.code()).or_default() += 1;
            steps += 1;
        }
    }
    assert_eq!(steps, 16 * 11 * 3);
    for code in [0, 1, 3, 4] {
        assert!(codes.get(&code).copied().unwrap_or(0) > 0, "{codes:?}");
    }
}

/// 0x42A530 index outside the tile allocation: the original reads foreign
/// memory; the reused tile lookup refuses it.
#[test]
fn source_tile_outside_allocation_is_refused() {
    let case = json!({"name": "unit", "tile_nodes": [], "node_regions": [], "answers": {},
        "objects": [{"key": 1, "position": [(-1.5f32).to_bits(), (-3.0f32).to_bits()], "word90": 0}],
        "descriptor": {"position": [5f32.to_bits(), 5f32.to_bits()], "ignore_actor": 0,
                       "collision_owner": 0, "mask": 0x300}, "source": 1});
    let map = gmap(
        &json!({"width": 8, "height": 7, "shift": 1, "cell_bits": 2f32.to_bits()}),
        [16f32.to_bits(), 14f32.to_bits()],
    );
    let mut world = Seamed { case: &case, regions: Regions::new(&case), events: vec![] };
    let result = placement_result(&map, Some(1), &seamed_query(&case), &mut world);
    assert!(matches!(result, Err(Error::Tile(_))), "{result:?}");
    assert_eq!(json!(world.events), json!([["actor_position", 1]]));
}

/// 0x53CE40's seams: 0x552250 stores the case's "snapped" position, 0x54F370
/// may rewrite descriptor +0x1C, 0x4F3C40 returns the case's EAX.
struct Endpoint<'a> {
    case: &'a Value,
    events: Vec<Value>,
}
impl DescriptorWorld for Endpoint<'_> {
    fn set_descriptor_position(&mut self, descriptor: &mut PreparedQuery, point: [u32; 2]) -> Result<()> {
        self.events.push(json!(["set_position", point[0], point[1]]));
        let snap = &self.case["snap"];
        descriptor.placement.position = if snap.is_null() { point } else { bits2(snap) };
        Ok(())
    }
    fn placement_code(&mut self, source: Option<ActorKey>, descriptor: &PreparedQuery) -> Result<u32> {
        let v = u(&self.case["placement"]);
        let p = descriptor.placement.position;
        self.events.push(json!(["placement", source.unwrap_or(0), p[0], p[1], v]));
        Ok(v)
    }
}
impl EndpointWorld for Endpoint<'_> {
    fn tile_byte_20(&mut self, tile: usize) -> Result<u8> {
        // The oracle zero-fills the tile allocation before writing these bytes.
        let byte = self.case["tile_bytes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| u(&t[0]) as usize == tile)
            .map_or(0, |t| u(&t[1]) as u8);
        self.events.push(json!(["tile_byte_20", tile, byte]));
        Ok(byte)
    }
    fn tile_blocked(&mut self, tile: usize, descriptor: &mut PreparedQuery) -> Result<bool> {
        let blocked = al(u(&self.case["tile_blocked"]));
        let p = descriptor.placement.position;
        self.events.push(json!(["tile_blocked", tile, p[0], p[1], u32::from(blocked)]));
        if !self.case["reassign_source"].is_null() {
            descriptor.descriptor.source_actor = key(&self.case["reassign_source"]);
        }
        Ok(blocked)
    }
}

#[test]
fn endpoint_acceptance_matches_original_x86() {
    let doc = load("native-gworld-placement-callers.json");
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len() as u64, doc["case_count"].as_u64().unwrap());
    let mut results = [0usize; 2];
    for case in cases {
        let native = &case["output"];
        let mut descriptor = PreparedQuery {
            placement: Placement {
                shape: Shape { kind: 0, radius_bits: 0, bounds: [0; 4] },
                facing: [0, 0],
                position: [0xdead_beef; 2],
            },
            descriptor: Descriptor { source_actor: key(&case["source"]), ..Descriptor::default() },
            required_actor_flags: 0,
        };
        let map = gmap(&case["map"], bits2(&case["map"]["extent_bits"]));
        let mut world = Endpoint { case, events: vec![] };
        let accepted = endpoint_acceptable(&map, bits2(&case["point"]), &mut descriptor, &mut world)
            .unwrap_or_else(|e| panic!("{}: {e}", case["name"]));
        assert_eq!(u64::from(accepted), native["result"].as_u64().unwrap(), "{}", case["name"]);
        assert_eq!(json!(world.events), native["events"], "{} {}", case["name"], case["precision"]);
        results[usize::from(accepted)] += 1;
    }
    assert!(results[0] > 0 && results[1] > 0, "{results:?}");
}

/// 0x4F46E0's seams: actor data from the case, 0x554350 returns the case's
/// descriptor words, 0x552250 stores the position under a snap mask and
/// 0x4F3C40 returns the case's EAX sequence.
struct Nearest<'a> {
    case: &'a Value,
    events: Vec<Value>,
    calls: usize,
}
impl Nearest<'_> {
    fn actor(&self, k: ActorKey) -> &Value {
        self.case["actors"].as_array().unwrap().iter().find(|a| u(&a["key"]) == k).expect("actor")
    }
}
impl DescriptorWorld for Nearest<'_> {
    fn set_descriptor_position(&mut self, descriptor: &mut PreparedQuery, point: [u32; 2]) -> Result<()> {
        self.events.push(json!(["set_position", point[0], point[1]]));
        let mask = u(&self.case["snap_mask"]);
        descriptor.placement.position = [point[0] & mask, point[1] & mask];
        Ok(())
    }
    fn placement_code(&mut self, source: Option<ActorKey>, descriptor: &PreparedQuery) -> Result<u32> {
        let v = u(&self.case["results"][self.calls]);
        self.calls += 1;
        let p = descriptor.placement.position;
        self.events.push(json!([
            "placement",
            source.unwrap_or(0),
            p[0],
            p[1],
            descriptor.required_actor_flags,
            v
        ]));
        Ok(v)
    }
}
impl NearestPlacementWorld for Nearest<'_> {
    fn actor_virtual_b4(&mut self, actor: ActorKey) -> Result<bool> {
        let r = al(u(&self.actor(actor)["b4"]));
        self.events.push(json!(["actor_b4", actor, u32::from(r)]));
        Ok(r)
    }
    fn actor_word_7c(&mut self, actor: ActorKey) -> Result<Option<u32>> {
        let k = u(&self.actor(actor)["word7c"]);
        self.events.push(json!(["actor_word_7c", actor, k]));
        Ok(Some(k).filter(|k| *k != 0))
    }
    fn object_word_40(&mut self, object: u32) -> Result<Option<ActorKey>> {
        let owners = self.case["owners"].as_array().unwrap();
        let owner = owners.iter().find(|o| u(&o["key"]) == object).expect("owner");
        let k = u(&owner["word40"]);
        self.events.push(json!(["object_word_40", object, k]));
        Ok(Some(k).filter(|k| *k != 0))
    }
    fn prepare_descriptor(&mut self, actor: ActorKey) -> Result<PreparedQuery> {
        self.events.push(json!(["prepare_descriptor", actor]));
        let w: Vec<u32> = self.case["descriptor_words"].as_array().unwrap().iter().map(u).collect();
        Ok(PreparedQuery {
            placement: Placement {
                shape: Shape { kind: w[0], radius_bits: w[1], bounds: [w[2], w[3], w[4], w[5]] },
                facing: [w[17], w[18]],
                position: [w[19], w[20]],
            },
            descriptor: Descriptor { mask: w[15], ..Descriptor::default() },
            required_actor_flags: w[14],
        })
    }
    fn actor_facing(&mut self, actor: ActorKey) -> Result<[u32; 2]> {
        self.events.push(json!(["actor_facing", actor]));
        Ok(bits2(&self.actor(actor)["facing"]))
    }
}

#[test]
fn nearest_placement_matches_original_x86() {
    let doc = load("native-gworld-placement-callers.json");
    for table in doc["findings"]["tables"].as_object().unwrap().values() {
        assert_eq!(u(&table["sin"]), ROTATION_SIN_BITS);
        assert_eq!(u(&table["cos"]), ROTATION_COS_BITS);
    }
    let mut outcomes: BTreeMap<String, usize> = BTreeMap::new();
    let mut by_name: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for case in doc["nearest_cases"].as_array().unwrap() {
        let p: Precision = serde_json::from_value(case["precision"].clone()).unwrap();
        let native = &case["output"];
        let name = case["name"].as_str().unwrap();
        // Only the cell size (GMap +8) is read; the oracle's other GMap words are 0.
        let map = GMapView {
            width: 0,
            height: 0,
            coordinate_shift: 0,
            cell_size_bits: u(&case["cell_bits"]),
            extent_bits: [0, 0],
        };
        let mut point = bits2(&case["point"]);
        let mut world = Nearest { case, events: vec![], calls: 0 };
        let result =
            nearest_placement(&map, key(&case["actor"]), &mut point, u(&case["radius"]), p, &mut world);
        if native["fault"].is_string() {
            assert_eq!(native["fault"], "0x0", "{name}");
            assert!(matches!(result, Err(Error::NullActor)), "{name}: {result:?}");
            *outcomes.entry("fault".into()).or_default() += 1;
        } else {
            let found = result.unwrap_or_else(|e| panic!("{name} {p:?}: {e}"));
            assert_eq!(u64::from(found), native["result"].as_u64().unwrap(), "{name} {p:?}");
            *outcomes.entry(found.to_string()).or_default() += 1;
        }
        assert_eq!(json!(point), native["point"], "{name} {p:?}");
        assert_eq!(json!(world.events), native["events"], "{name} {p:?}");
        by_name.entry(name.into()).or_default().push(native.clone());
    }
    for outcome in ["true", "false", "fault"] {
        assert!(outcomes.get(outcome).copied().unwrap_or(0) > 0, "{outcomes:?}");
    }
    // The unstored step sum is compared: PC24 tries a third ring, PC53/PC64 do not.
    let split = &by_name["extended_step_compare_25"];
    assert_eq!(split.len(), 3);
    assert_eq!(split[0]["result"], 1);
    assert_eq!(split[1]["result"], 0);
    assert_eq!(split[2]["result"], 0);
    let dependent = by_name.values().filter(|rows| rows.iter().any(|r| r != &rows[0])).count();
    assert!(dependent > 100, "{dependent}");
}
