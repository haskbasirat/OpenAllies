//! Replays research/phase8-conversion/native-fresh-creation.json: the original
//! 4F9C10 path for GObject / DynamicDoodad (native down to the recorded
//! world-subsystem seams) and 4FA030 / 4FA150.
use aa_actors::creation::DirectionTable;
use aa_actors::registry::{ActorId, IdentityRegistry};
use aa_mapworld::class_conversion::{self, ObjectCreationParams};
use aa_mapworld::fresh_creation::*;
use aa_numeric::{Binary, Precision};
use aa_simulation::NativeRng;
use serde_json::{Value, json};
use std::path::PathBuf;

fn load() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/phase8-conversion/native-fresh-creation.json");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn n(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn u32s(v: &Value) -> Vec<u32> {
    v.as_array().unwrap().iter().map(n).collect()
}
fn pc(name: &str) -> Precision {
    match name {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        _ => Precision::Extended64,
    }
}
fn text(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

/// Fixture world: every service records a normalized call; identity uses
/// the aa-actors GTID registry port.
struct World {
    spec: Value,
    rng: NativeRng,
    registry: IdentityRegistry<()>,
    calls: Vec<Value>,
    events: u32,
    cities: Vec<u32>,
    world_list: Vec<u32>,
}
impl World {
    fn new(spec: &Value) -> Self {
        let idm = &spec["idmgr"];
        let highest = n(&idm["highest"]);
        let mut gens = vec![0u16; highest as usize];
        let mut live = Vec::new();
        let mut seeds = Vec::new();
        for s in idm["slots"].as_array().unwrap() {
            let (slot, generation) = (n(&s[0]), n(&s[1]) as u16);
            if (slot as usize) < gens.len() {
                gens[slot as usize] = generation;
            }
            let id = ActorId::from_raw(((generation as u32) << 16) | slot);
            if s[2] == 1 {
                live.push(id);
            } else {
                seeds.push(id);
            }
        }
        let mut registry = IdentityRegistry::from_load_metadata(n(&idm["cursor"]), highest, &gens).unwrap();
        for id in live {
            registry.seed_saved_generation(id).unwrap();
            registry.register_saved(id, false, ()).unwrap();
        }
        // Native table generations of free slots (also above `highest`).
        for id in seeds {
            registry.seed_saved_generation(id).unwrap();
        }
        // Slot generations above `highest` (requested IDs): 4EFB70 seeds them.
        let count = n(&idm["count"]) as usize;
        let mut filler = 20_000u32;
        while registry.len() < count {
            registry.register_saved(ActorId::from_raw(filler), false, ()).unwrap();
            filler += 1;
        }
        Self {
            spec: spec.clone(),
            rng: NativeRng::new(n(&spec["rng"])),
            registry,
            calls: Vec::new(),
            events: 0,
            cities: Vec::new(),
            world_list: Vec::new(),
        }
    }
}
fn desc(d: &Descriptor) -> Value {
    json!([match d.shape { ShapeRef::A => "A", ShapeRef::B => "B" },
           d.facing_bits[0], d.facing_bits[1], d.position_bits[0], d.position_bits[1], 0])
}
impl FreshWorld<String> for World {
    type Event = u32;
    type Error = String;
    fn definition(&mut self, key: &String) -> Result<ObjectDefinitionInputs<String>, String> {
        let s = &self.spec;
        let class = n(&s["class"]);
        let sa = u32s(&s["shape_a"]);
        let comps: Vec<Option<u32>> = s["components"].as_array().unwrap().iter().map(|v| v.as_u64().map(|x| x as u32)).collect();
        Ok(ObjectDefinitionInputs {
            key: key.clone(),
            ids_utf16: text(&format!("def_{class}")),
            display_utf16: text(&format!("display_{class}")),
            runtime_class: class,
            is_object_data: true,
            flags: n(&s["flags"]),
            radius_factor_bits: n(&s["radius_factor"]),
            scale_min_bits: n(&s["scale"][0]),
            scale_max_bits: n(&s["scale"][1]),
            city: s["city"] == 1,
            shape_a: Shape { kind: sa[0], radius_bits: sa[1], extent_10_bits: sa[4], extent_14_bits: sa[5] },
            variations_enabled: s["variations"] == 1,
            aspect_weights_bits: u32s(&s["aspects"]),
            total_weight_bits: n(&s["total_weight"]),
            component_periods_bits: [comps[0], comps[1]],
        })
    }
    fn rng(&mut self) -> &mut NativeRng {
        &mut self.rng
    }
    fn begin_time_bits(&self) -> u32 {
        n(&self.spec["begin"])
    }
    fn directions(&self) -> DirectionTable {
        DirectionTable::original(pc(self.spec["direction_table_pc"].as_str().unwrap()))
    }
    fn tile_size_bits(&self) -> u32 {
        n(&self.spec["tile_size"])
    }
    fn tile_shift(&self) -> u32 {
        n(&self.spec["tile_shift"])
    }
    fn map_mode(&self) -> u32 {
        n(&self.spec["map_mode"])
    }
    fn scene_present(&self) -> bool {
        n(&self.spec["world_d8"]) != 6 && self.spec["scene"] == true
    }
    fn instance_name_gtid(&mut self, _: &[u16]) -> Result<Option<u32>, String> {
        Err("instance names are not used by these cases".into())
    }
    fn gtid_live(&self, _: u32) -> Result<bool, String> {
        Err("unused".into())
    }
    fn register_requested_id(&mut self, id: u32) -> Result<(), String> {
        let id = ActorId::from_raw(id);
        self.registry.seed_saved_generation(id).map_err(|e| e.to_string())?;
        self.registry.register_saved(id, false, ()).map_err(|e| e.to_string())
    }
    fn allocate_id(&mut self) -> Result<Option<u32>, String> {
        match self.registry.allocate(false, ()) {
            Ok(id) => Ok(Some(id.raw())),
            Err(aa_actors::registry::RegistryError::Exhausted) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }
    fn index_move(&mut self, _: &mut FreshObject<String, u32>, _: [Descriptor; 4]) -> Result<(), String> {
        self.calls.push(json!(["index move"]));
        Ok(())
    }
    fn scene_transform(&mut self, _: &FreshObject<String, u32>) -> Result<(), String> {
        self.calls.push(json!(["scene transform"]));
        Ok(())
    }
    fn snap(&mut self, shape: &Shape, xy: [u32; 2], tile: u32) -> Result<[u32; 2], String> {
        assert_eq!(shape.kind, n(&self.spec["shape_a"][0]));
        self.calls.push(json!(["snap", "A", xy, tile]));
        Ok(match &self.spec["snap"] {
            Value::Null => xy,
            v => [n(&v[0]), n(&v[1])],
        })
    }
    fn placement_test(&mut self, _: &FreshObject<String, u32>) -> Result<u32, String> {
        self.calls.push(json!(["placement test"]));
        Ok(n(&self.spec["placement"]))
    }
    fn index_insert(&mut self, o: &mut FreshObject<String, u32>, b: Descriptor, a: Descriptor) -> Result<(), String> {
        self.calls.push(json!(["index insert", desc(&b), desc(&a)]));
        o.flags |= 1 << 26;
        Ok(())
    }
    fn index_remove(&mut self, o: &mut FreshObject<String, u32>, b: Descriptor, a: Descriptor) -> Result<(), String> {
        self.calls.push(json!(["index remove", desc(&b), desc(&a)]));
        o.flags &= !(1 << 26);
        Ok(())
    }
    fn terrain_height(&mut self, xy: [u32; 2], flags: u32) -> Result<Binary, String> {
        self.calls.push(json!(["terrain height", xy, flags]));
        Binary::from_f32_bits(n(&self.spec["height"])).map_err(|e| e.to_string())
    }
    fn blocking_notify(&mut self, _: &FreshObject<String, u32>) -> Result<(), String> {
        self.calls.push(json!(["blocking notify"]));
        Ok(())
    }
    fn tile_footprint(&mut self, _: &FreshObject<String, u32>) -> Result<(), String> {
        self.calls.push(json!(["tile footprint"]));
        Ok(())
    }
    fn world_list_insert(&mut self, o: &FreshObject<String, u32>) -> Result<(), String> {
        self.world_list.insert(0, o.gtid);
        Ok(())
    }
    fn visibility_refresh(&mut self, _: &FreshObject<String, u32>) -> Result<(), String> {
        self.calls.push(json!(["visibility refresh"]));
        Ok(())
    }
    fn ground_paint(&mut self, _: &FreshObject<String, u32>, _: &ObjectCreationParams<String>) -> Result<(), String> {
        self.calls.push(json!(["ground paint"]));
        Ok(())
    }
    fn city_assert_latch(&self) -> bool {
        false
    }
    fn available_cities_contains(&self, gtid: u32) -> Result<bool, String> {
        Ok(self.cities.contains(&gtid))
    }
    fn assertion_dialog(&mut self, expression: &'static str, line: u32) -> Result<bool, String> {
        self.calls.push(json!(["assert dialog", expression, line]));
        Ok(false)
    }
    fn set_city_assert_latch(&mut self, _: bool) {}
    fn add_available_city(&mut self, gtid: u32) -> Result<(), String> {
        self.cities.insert(0, gtid);
        Ok(())
    }
    fn scene_add(&mut self, _: &FreshObject<String, u32>) -> Result<(), String> {
        self.calls.push(json!(["scene add"]));
        Ok(())
    }
    fn creation_fx(&mut self, _: &FreshObject<String, u32>) -> Result<(), String> {
        self.calls.push(json!(["creation fx"]));
        Ok(())
    }
    fn schedule(&mut self, _gtid: u32, slot: u8, delay: u32) -> Result<u32, String> {
        let e = self.events;
        self.events += 1;
        self.calls.push(json!(["queue add", format!("component{slot}"), delay, 0, e]));
        Ok(e)
    }
    fn unlink(&mut self, e: u32) -> Result<(), String> {
        self.calls.push(json!(["queue unlink", e]));
        Ok(())
    }
    fn log(&mut self, entry: CreationLog) -> Result<(), String> {
        let v = match entry {
            CreationLog::NoIdsLeft => json!(["log", 0xab, "KKC_GTIDManager::Add() - no IDs left"]),
            CreationLog::NotThickEnough { .. } => json!(["log", 0x213, "Static object %s is not thick enough to block tiles"]),
            CreationLog::PlacementFailed { .. } => json!(["log", 299, "creating thing %s failed (reason: %s) at %f,%f"]),
            CreationLog::ThingDoesntFit { display_utf16, .. } => {
                json!(["map warning", "load_warning_thing_doesnt_fit|sff", String::from_utf16(&display_utf16).unwrap()])
            }
            CreationLog::Create { line, format, .. } => json!(["log", line, format]),
        };
        self.calls.push(v);
        Ok(())
    }
    fn destroy(&mut self, _: FreshObject<String, u32>) -> Result<(), String> {
        self.calls.push(json!(["destroy"]));
        Ok(())
    }
}

/// Native seam records in the same vocabulary.
fn native_calls(calls: &Value) -> Vec<Value> {
    let mut out = Vec::new();
    for c in calls.as_array().unwrap() {
        let s = c["seam"].as_str().unwrap();
        let v = match s {
            "scene transform" | "placement test" | "tile footprint" | "visibility refresh" | "ground paint"
            | "scene add" | "creation fx" | "destroy" | "index move" => json!([s]),
            "index insert" | "index remove" => json!([s, c["a"], c["b"]]),
            "terrain height" => json!([s, c["xy"], c["flags"]]),
            "snap" => json!([s, c["shape"], c["xy"], c["tile"]]),
            "queue add" => json!([s, c["updatable"], c["delay"], c["data"], c["node"]]),
            "queue unlink" => json!([s, c["node"]]),
            "blocking query shape" => continue,
            "blocking query" => json!(["blocking notify"]),
            "log 404EA0" | "log 404F60" => json!(["log", c["line"], c["format"]]),
            "4B9F40" => continue,
            "5A1040" => json!(["map warning", c["text"], c["ids"]]),
            "assert dialog" => json!([s, c["expression"], c["line"]]),
            other => panic!("unmapped native seam {other}"),
        };
        out.push(v);
    }
    out
}

#[test]
fn fresh_creation_matches_native_4f9c10() {
    let fixture = load();
    let mut checked = 0;
    for case in fixture["cases"].as_array().unwrap() {
        let label = case["label"].as_str().unwrap();
        let spec = &case["spec"];
        if spec.get("entry").is_some_and(|e| e != "create") {
            continue; // 4FA030 on an object: covered by the entry test
        }
        let r = &case["result"];
        let mut world = World::new(spec);
        let params = ObjectCreationParams {
            definition: "def".to_owned(),
            position_bits: u32s(&spec["position"]).try_into().unwrap(),
            direction_bits: u32s(&spec["direction"]).try_into().unwrap(),
            name_utf16: text(spec["name"].as_str().unwrap()),
            variant: n(&spec["variant"]),
            requested_id: n(&spec["requested_id"]),
        };
        let precision = pc(spec["pc"].as_str().unwrap());
        let created = create_thing(&params, &mut world, precision).unwrap_or_else(|e| panic!("{label}: {e}"));
        assert_eq!(world.calls, native_calls(&r["calls"]), "{label}");
        assert_eq!(world.rng.state, n(&r["rng_after"]), "{label} rng");
        let pa = u32s(&r["params_after"]);
        assert_eq!(created.params.position_bits.to_vec(), pa[2..5].to_vec(), "{label} params position");
        assert_eq!(created.params.direction_bits.to_vec(), pa[5..7].to_vec(), "{label} params direction");
        match (&r["returned"], created.object) {
            (Value::String(s), Some(o)) if s == "object" => {
                let w = u32s(&r["object"]);
                assert_eq!(o.flags, w[2], "{label} flags");
                assert_eq!([o.word_c, o.word_10], [w[3], w[4]]);
                assert_eq!(o.gtid, w[5], "{label} gtid");
                assert_eq!(o.created_at_bits, w[7]);
                assert_eq!(o.position_bits.to_vec(), w[8..11].to_vec(), "{label} position");
                assert_eq!(o.facing_bits.to_vec(), w[11..14].to_vec(), "{label} facing");
                assert_eq!(o.radius_bits, w[14], "{label} radius");
                assert_eq!(o.scale_bits, w[15], "{label} scale");
                assert_eq!(o.variant, w[16], "{label} variant");
                assert_eq!(o.tile, [w[17] as i32, w[18] as i32], "{label} tile");
                assert_eq!(o.byte_4c, (w[19] & 0xff) as u8);
                assert_eq!(o.words_50_to_5c.to_vec(), w[20..24].to_vec());
                assert!(o.name_utf16.is_empty() && r["object_name"].as_array().unwrap().is_empty());
                for (slot, native) in r["components"].as_array().unwrap().iter().enumerate() {
                    match (native, &o.components[slot]) {
                        (Value::Null, None) => {}
                        (nat, Some(c)) => {
                            let words = u32s(&nat["words"]);
                            assert!(nat["owner_is_object"] == true && nat["definition_is_slot_def"] == true);
                            assert_eq!(c.words, words[6..].to_vec(), "{label} component {slot}");
                            assert_eq!(nat["event_head_set"] == true, !c.scheduled.is_empty(), "{label} events");
                        }
                        other => panic!("{label}: component {slot} {other:?}"),
                    }
                }
                assert_eq!(world.world_list, if r["world_list_8"].as_array().unwrap().is_empty() { vec![] } else { vec![o.gtid] });
                assert_eq!(world.cities, u32s(&r["available_cities"]), "{label} cities");
                if label != "allocate_id_full" {
                    assert_eq!(world.registry.next_slot() as u32, n(&r["idmgr"]["cursor"]), "{label} cursor");
                    assert_eq!(world.registry.highest_slot() as u32, n(&r["idmgr"]["highest"]), "{label} highest");
                    assert_eq!(world.registry.len() as u32, n(&r["idmgr"]["count"]), "{label} count");
                }
            }
            (v, None) if v == 0 => {}
            (v, o) => panic!("{label}: native {v} vs Rust {o:?}"),
        }
        checked += 1;
    }
    assert!(checked >= 50, "{checked}");
}

struct Entry {
    spec: Value,
    latches: [bool; 2],
    calls: Vec<Value>,
    position: [u32; 3],
}
impl EntryWorld for Entry {
    type Object = u32;
    type Layout = &'static str;
    type Error = String;
    fn create(&mut self, position: [u32; 3]) -> Result<Option<u32>, String> {
        self.position = position;
        self.calls.push(json!(["4F9C10", position]));
        Ok((self.spec["create_result"] == true).then_some(0x42))
    }
    fn is_gactor(&mut self, _: &u32) -> Result<bool, String> {
        Ok(self.spec["is_gactor"] == true)
    }
    fn assert_latch(&self, address: u32) -> bool {
        self.latches[(address - 0xb3cc43) as usize]
    }
    fn set_assert_latch(&mut self, address: u32, value: bool) {
        self.latches[(address - 0xb3cc43) as usize] = value;
    }
    fn assertion_dialog(&mut self, line: u32) -> Result<bool, String> {
        self.calls.push(json!(["assert dialog", "thing->IsGActor()", line]));
        Ok(self.spec["assert_result"] == 1)
    }
    fn definition_is_actor_data(&mut self) -> Result<bool, String> {
        Ok(self.spec["def_vtable"] == "ActorData")
    }
    fn definition_flags_174(&mut self) -> Result<u32, String> {
        Ok(n(&self.spec["def_flags"]))
    }
    fn layout(&mut self) -> Result<&'static str, String> {
        Ok(if self.spec["params_layout"].is_null() { "definition+2F0" } else { "params+2C" })
    }
    fn layout_slot_count(&mut self, l: &&'static str) -> Result<i32, String> {
        let key = if *l == "params+2C" { "params_layout" } else { "def_layout" };
        Ok(n(&self.spec[key]) as i32)
    }
    fn first_slot(&mut self, l: &&'static str) -> Result<u32, String> {
        self.calls.push(json!(["4C2BC0", l]));
        Ok(n(&self.spec["first_slot"]))
    }
    fn slot_placement(&mut self, l: &&'static str, slot: u32, xy: &mut [u32; 2], facing: [u32; 2]) -> Result<bool, String> {
        self.calls.push(json!(["578290", l, slot, xy.to_vec(), facing.to_vec()]));
        if let Some(v) = self.spec["slot_xy"].as_array() {
            *xy = [n(&v[0]), n(&v[1])];
        }
        Ok(self.spec["slot_ok"] == 1)
    }
    fn organization(&mut self, _: &u32) -> Result<bool, String> {
        Ok(!self.spec["organization"].is_null())
    }
    fn leader_slot(&mut self, _: &u32) -> Result<i32, String> {
        Ok(n(&self.spec["organization"]["leader"]) as i32)
    }
    fn organization_slot_count(&mut self, _: &u32) -> Result<i32, String> {
        Ok(self.spec["organization"]["slot_groups"].as_array().unwrap().len() as i32)
    }
    fn slot_defined(&mut self, _: &u32, slot: i32) -> Result<bool, String> {
        let group = &self.spec["organization"]["slot_groups"][slot as usize];
        Ok(self.spec["organization"]["defined_groups"].as_array().unwrap().contains(group))
    }
    fn spawn_member(&mut self, _: &u32, slot: i32, flag: u8) -> Result<(), String> {
        self.calls.push(json!(["57AD30", slot, flag]));
        Ok(())
    }
    fn member_count(&mut self, _: &u32) -> Result<u32, String> {
        self.calls.push(json!(["57A730"]));
        Ok(n(&self.spec["member_count"]))
    }
    fn log_no_members(&mut self, _: &u32) -> Result<(), String> {
        self.calls.push(json!(["log", 0x8a0, "org %d failed to create any elements - destroying"]));
        Ok(())
    }
    fn destroy(&mut self, _: u32) -> Result<(), String> {
        self.calls.push(json!(["destroy"]));
        Ok(())
    }
}

#[test]
fn create_gactor_and_organization_match_native() {
    let fixture = load();
    for case in fixture["entries"].as_array().unwrap() {
        let label = case["label"].as_str().unwrap();
        let spec = &case["spec"];
        let r = &case["result"];
        let latch = spec["latch"] == 1;
        let mut world = Entry { spec: spec.clone(), latches: [latch, latch], calls: Vec::new(), position: [0; 3] };
        let pos: [u32; 3] = u32s(&spec["position"]).try_into().unwrap();
        let facing: [u32; 2] = u32s(&spec["direction"]).try_into().unwrap();
        let mut params = OrganizationParams { position_bits: pos, facing_bits: facing, byte_51: n(&spec["byte_51"]) as u8 };
        let result = if spec["entry"] == "actor" {
            create_gactor(&mut world, pos).unwrap()
        } else {
            create_organization(&mut world, &mut params).unwrap()
        };
        let native: Vec<Value> = r["calls"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| match c["seam"].as_str().unwrap() {
                "4F9C10" => json!(["4F9C10", c["position"]]),
                "assert dialog" => json!(["assert dialog", c["expression"], c["line"]]),
                "4C2BC0" => json!(["4C2BC0", c["layout"]]),
                "578290" => json!(["578290", c["layout"], c["slot"], c["xy"], c["facing"]]),
                "57AD30" => {
                    assert_eq!(c["org"], true);
                    json!(["57AD30", c["slot"], c["flag"]])
                }
                "57A730" => json!(["57A730"]),
                "log 404F60" => json!(["log", c["line"], c["format"]]),
                "destroy" => json!(["destroy"]),
                other => panic!("{other}"),
            })
            .collect();
        assert_eq!(world.calls, native, "{label}");
        assert_eq!(result.is_some(), r["returned"] == "object", "{label}");
        let latch_slot = if spec["entry"] == "actor" { 0 } else { 1 };
        let native_latch = if latch_slot == 0 { &r["latch_43"] } else { &r["latch_44"] };
        assert_eq!(world.latches[latch_slot], *native_latch == 1, "{label} latch");
        if spec["entry"] == "organization" {
            assert_eq!(params.position_bits.to_vec(), u32s(&r["params_after"])[2..5].to_vec(), "{label} params");
        }
    }
}

/// The class-conversion replacement on top of fresh creation: blocks from
/// `replace_converted` reach `create_thing` (classes 9 and 3: the latter is
/// refused by 4F9C10 before construction, "Actor class requires a GACD").
#[test]
fn replacement_creates_through_fresh_creation() {
    let fixture = load();
    let spec = fixture["cases"].as_array().unwrap().iter().find(|c| c["label"] == "doodad_components_Double53").unwrap()["spec"].clone();
    let mut world = World::new(&spec);
    let mut queue = class_conversion::ConversionQueue::new();
    let record = |id: u32| {
        let mut b = Vec::new();
        b.extend_from_slice(&3u16.to_le_bytes());
        for u in "def".encode_utf16() {
            b.extend_from_slice(&u.to_le_bytes());
        }
        // class 2, saved ID, Thing version 2, empty name.
        for w in [2u32, id, 2] {
            b.extend_from_slice(&w.to_le_bytes());
        }
        b.extend_from_slice(&0u16.to_le_bytes());
        // word, position, direction, flags; Object version 1, scale, variant.
        for w in [0u32, f32::to_bits(33.5), f32::to_bits(-12.75), 0, 0x3f19999a, 0xbf4ccccd, 0, 0, 1, 0x3f800000, 1] {
            b.extend_from_slice(&w.to_le_bytes());
        }
        b
    };
    for id in [(3 << 16) | 5, (1 << 16) | 6] {
        let bytes = record(id);
        class_conversion::on_class_mismatch(&bytes, "def".to_owned(), 9, Some(&mut queue)).unwrap();
    }
    let mut adapter = FreshReplacement { world: &mut world, pc: Precision::Double53, created: Vec::new() };
    let done = class_conversion::replace_converted(queue, &mut adapter).unwrap();
    assert_eq!(done.len(), 2);
    assert!(done.iter().all(|r| r.created.is_some() && r.warning.key == class_conversion::REPLACED_WARNING));
    assert_eq!(adapter.created.iter().map(|o| o.gtid).collect::<Vec<_>>(), vec![(3 << 16) | 5, (1 << 16) | 6]);
    assert!(adapter.created.iter().all(|o| o.components[0].is_some() && o.components[1].is_some()));
}

/// 4F9C10 class dispatch for an A25B1C block (conversion oracle section
/// `dispatch`): 3/8 "requires a GACD", 4 "requires a GKCD" -> logged null;
/// 0, 1 and >= 11 -> native null dereference; other non-object classes are
/// explicit unported errors here.
#[test]
fn create_thing_class_dispatch() {
    let fixture = load();
    let base = fixture["cases"].as_array().unwrap().iter().find(|c| c["label"] == "doodad_requested_Double53").unwrap()["spec"].clone();
    for class in 0..12u32 {
        let mut spec = base.clone();
        spec["class"] = json!(class);
        let mut world = World::new(&spec);
        let params = ObjectCreationParams {
            definition: "def".to_owned(),
            position_bits: [0x42060000, 0xc14c0000, 0],
            direction_bits: [0x3f19999a, 0xbf4ccccd],
            name_utf16: Vec::new(),
            variant: 1,
            requested_id: (3 << 16) | 5,
        };
        let r = create_thing(&params, &mut world, Precision::Double53);
        match class {
            3 | 4 | 8 => {
                let created = r.unwrap();
                assert!(created.object.is_none());
                let (line, format) = match class {
                    3 => (0x7f6, "Actor class requires a GACD: %s"),
                    4 => (0x821, "Kingdom class requires a GKCD: %s"),
                    _ => (0x82a, "Group class requires a GACD: %s"),
                };
                assert_eq!(world.calls, vec![json!(["log", line, format])]);
            }
            0 | 1 | 11 => assert!(matches!(r, Err(FreshError::NullDereference(_))), "{class}"),
            2 | 9 => assert!(r.unwrap().object.is_some()),
            _ => assert!(matches!(r, Err(FreshError::Unsupported(_))), "{class}"),
        }
    }
}
