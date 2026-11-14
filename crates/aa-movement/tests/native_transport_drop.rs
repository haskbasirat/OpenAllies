//! Replays research/phase8-movement/native-transport-drop.json: original x86
//! execution of the TransportComponent drop 58BCB0, the lead distance
//! 58B180, the loaded mark 58B160 and the exit 58B130 under PC24/53/64.
//! Every recorded callback with its argument bits, the creation blocks, the
//! Transport fields after the call and the x87 lead value must match.
use aa_movement::falling::MapBounds;
use aa_movement::transport_drop::*;
use aa_movement::{Error, Result};
use aa_numeric::{Binary, Precision};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn document() -> Value {
    let path =
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-movement/native-transport-drop.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).expect("json")
}
fn u(v: &Value) -> u32 {
    v.as_u64().expect("u32") as u32
}
fn f(v: &Value) -> f32 {
    f32::from_bits(u(v))
}
fn fs<const N: usize>(v: &Value) -> [f32; N] {
    let a = v.as_array().expect("array");
    std::array::from_fn(|i| f(&a[i]))
}
fn bits<const N: usize>(v: [f32; N]) -> Value {
    json!(v.map(f32::to_bits).to_vec())
}

struct World<'a> {
    c: &'a Value,
    keys: Vec<String>,
    events: Vec<Value>,
    members: BTreeMap<i32, Ref>,
}

impl<'a> World<'a> {
    fn new(c: &'a Value) -> Self {
        let mut keys = vec![String::new()];
        for k in ["plane", "thing", "org", "attachment", "layout", "name"] {
            keys.push(k.into());
        }
        for k in c["defs"].as_object().unwrap().keys() {
            keys.push(k.clone());
            keys.push(format!("name:{k}"));
        }
        Self { c, keys, events: Vec::new(), members: BTreeMap::new() }
    }
    fn id(&self, key: &str) -> Ref {
        self.keys.iter().position(|k| k == key).unwrap_or_else(|| panic!("key {key}")) as Ref
    }
    fn opt(&self, v: &Value) -> Ref {
        v.as_str().map_or(0, |k| self.id(k))
    }
    fn key(&self, r: Ref) -> Value {
        if r == 0 { json!(0) } else { json!(self.keys[r as usize]) }
    }
    fn name(&self, r: Ref) -> &str {
        &self.keys[r as usize]
    }
    fn resp(&self, k: &str) -> &'a Value {
        &self.c["responses"][k]
    }
    fn ext(v: &Value) -> Result<Binary> {
        Binary::from_f64_bits(v.as_u64().unwrap()).map_err(|e| Error::World(e.to_string()))
    }
    fn params(&self, p: &CreateParams) -> Value {
        let name = match p.name {
            None => json!("empty"),
            Some(n) => self.key(n),
        };
        json!([
            self.key(p.definition),
            p.position[0].to_bits(), p.position[1].to_bits(), p.position[2].to_bits(),
            p.facing[0].to_bits(), p.facing[1].to_bits(),
            name, p.variant, 0, p.kingdom, self.key(p.layout), 0, p.word_34, 0, 0,
            p.organization, 0, 0, 0, p.byte_50, 0, 0
        ])
    }
    fn write<const N: usize>(out: &Value, into: &mut [f32; N]) {
        if let Some(a) = out.as_array() {
            for (i, v) in a.iter().enumerate() {
                into[i] = f(v);
            }
        }
    }
}

impl TransportWorld for World<'_> {
    fn actor(&mut self, actor: Ref) -> Result<ActorView> {
        let v = &self.c[self.name(actor)];
        Ok(ActorView {
            definition: self.id(v["def"].as_str().unwrap()),
            flags: v.get("flags").map_or(0, u),
            position: fs(&v["position"]),
            facing: fs(&v["facing"]),
            word_88: v.get("word_88").map_or(0, u),
        })
    }
    fn definition(&mut self, definition: Ref) -> Result<DefinitionView> {
        let key = self.name(definition).to_string();
        let d = &self.c["defs"][&key];
        Ok(DefinitionView {
            name: self.id(&format!("name:{key}")),
            height_24: f(&d["height_24"]),
            field_1a4: f(&d["field_1a4"]),
            drop_offset: d.get("drop_offset").map_or([0.0; 2], fs),
            byte_3ec: d.get("byte_3ec").map_or(0, u) as u8,
        })
    }
    fn layout(&mut self, layout: Ref) -> Result<LayoutView> {
        assert_eq!(self.name(layout), "layout");
        let l = &self.c["layout"];
        Ok(LayoutView { leader_slot: u(&l["leader_slot"]) as i32, count: u(&l["count"]) as i32 })
    }
    fn layout_definition(&mut self, layout: Ref, slot: i32) -> Result<Ref> {
        assert_eq!(self.name(layout), "layout");
        let slots = self.c["layout"]["slots"].as_array().unwrap();
        Ok(self.opt(&slots[usize::try_from(slot).unwrap()]))
    }
    fn resolve(&mut self, handle: Handle) -> Result<Ref> {
        let org = &self.c["org"];
        if handle == u(&org["handle"]) && org["registered"].as_bool().unwrap() {
            return Ok(self.id("org"));
        }
        if handle == u(&self.c["attachment_handle"]) {
            return Ok(self.id("attachment"));
        }
        Ok(0)
    }
    fn handle_valid(&mut self, handle: Handle) -> Result<bool> {
        Ok(self.resolve(handle)? != 0)
    }
    fn handle_of(&mut self, object: Ref) -> Result<Handle> {
        assert_eq!(self.name(object), "org");
        Ok(u(&self.c["org"]["handle"]))
    }
    fn organization_member(&mut self, organization: Ref, slot: i32) -> Result<Ref> {
        assert_eq!(self.name(organization), "org");
        Ok(self.members.get(&slot).copied().unwrap_or(0))
    }
    fn organization_leader(&mut self, organization: Ref) -> Result<Ref> {
        assert_eq!(self.name(organization), "org");
        Ok(self.opt(&self.c["org"]["leader"]))
    }
}

impl DropWorld for World<'_> {
    fn kingdom(&mut self, _: Ref) -> Result<Ref> {
        self.events.push(json!(["kingdom"]));
        Ok(u(&self.c["plane"]["kingdom"]))
    }
    fn plane_virtual_0(&mut self, _: Ref) -> Result<Binary> {
        self.events.push(json!(["plane_virtual_0"]));
        Self::ext(&self.c["plane"]["v0"])
    }
    fn plane_virtual_18(&mut self, _: Ref) -> Result<Binary> {
        self.events.push(json!(["plane_virtual_18"]));
        Self::ext(&self.c["plane"]["v18"])
    }
    fn place_organization(
        &mut self,
        point: &mut [f32; 2],
        facing: &mut [f32; 2],
        leader: Ref,
        plane_xy: [f32; 2],
        radius: f32,
    ) -> Result<bool> {
        self.events.push(json!(["place_organization", bits(*point), bits(*facing), self.key(leader),
            bits(plane_xy), radius.to_bits()]));
        let r = self.resp("place_organization");
        Self::write(&r["point"], point);
        Self::write(&r["facing"], facing);
        Ok(r["ok"].as_bool().unwrap())
    }
    fn place_thing(
        &mut self,
        position: &mut [f32; 2],
        facing: &mut [f32; 2],
        definition: Ref,
        radius: f32,
        svar_3c: f32,
    ) -> Result<bool> {
        self.events.push(json!(["place_thing", bits(*position), bits(*facing), self.key(definition),
            radius.to_bits(), svar_3c.to_bits(), 0, 0]));
        let r = self.resp("place_thing");
        Self::write(&r["position"], position);
        Self::write(&r["facing"], facing);
        Ok(r["ok"].as_bool().unwrap())
    }
    fn landing_search(
        &mut self,
        target: &mut [f32; 2],
        direction: &mut [f32; 2],
        definition: Ref,
    ) -> Result<bool> {
        self.events.push(json!(["landing_search", bits(*target), bits(*direction), self.key(definition),
            [LANDING_SCALE.to_bits(), LANDING_COUNT, 0, 0, 0]]));
        let r = self.resp("landing");
        Self::write(&r["target"], target);
        Self::write(&r["direction"], direction);
        Ok(r["ok"].as_bool().unwrap())
    }
    fn report(&mut self, _: Failure, plane: Ref, kingdom: Ref, extra: Ref, name: Ref) -> Result<()> {
        self.events.push(json!(["report", self.key(plane), kingdom, self.key(extra), self.key(name)]));
        Ok(())
    }
    fn create(&mut self, params: &CreateParams) -> Result<Ref> {
        self.events.push(json!(["create", self.params(params)]));
        Ok(self.opt(self.resp("create")))
    }
    fn create_unloaded(&mut self, params: &CreateParams, plane: Ref) -> Result<Ref> {
        self.events.push(json!(["create_unloaded", self.params(params), self.key(plane)]));
        Ok(self.opt(self.resp("create_unloaded")))
    }
    fn attachment_query(&mut self, attachment: Ref, plane: Ref) -> Result<u32> {
        self.events.push(json!(["attachment_query", self.key(attachment), self.key(plane)]));
        Ok(u(self.resp("attachment_query")))
    }
    fn attachment_attach(&mut self, attachment: Ref, created: Ref, value: u32) -> Result<()> {
        self.events.push(json!(["attachment_attach", self.key(attachment), self.key(created), 0, value]));
        Ok(())
    }
    fn schedule(&mut self, updatable: Updatable) -> Result<()> {
        let at = match updatable {
            Updatable::At08 => 8,
            Updatable::At18 => 0x18,
        };
        self.events.push(json!(["schedule", at]));
        Ok(())
    }
    fn plane_loaded(&mut self, plane: Ref) -> Result<()> {
        self.events.push(json!(["plane_loaded", self.key(plane)]));
        Ok(())
    }
    fn organization_carrier(&mut self, organization: Ref, plane: Ref) -> Result<()> {
        self.events.push(json!(["organization_carrier", self.key(organization), self.key(plane)]));
        Ok(())
    }
    fn kingdom_record(&mut self, kingdom: Ref, definition: Ref) -> Result<Ref> {
        self.events.push(json!(["kingdom_record", kingdom, self.key(definition), ZERO_ARGUMENT]));
        Ok(u(self.resp("kingdom_record")))
    }
    fn record_recruit(&mut self, record: Ref, organization: Ref, slot: i32) -> Result<()> {
        self.events.push(json!(["record_recruit", record, self.key(organization), slot as u32]));
        Ok(())
    }
    fn organization_add(&mut self, organization: Ref, slot: i32, thing: Ref) -> Result<()> {
        self.events.push(json!(["organization_add", self.key(organization), slot as u32, self.key(thing)]));
        Ok(())
    }
    fn organization_place(
        &mut self,
        organization: Ref,
        slot: i32,
        position: [f32; 3],
        facing: [f32; 2],
        airborne: bool,
    ) -> Result<()> {
        self.events.push(json!(["organization_place", self.key(organization), slot as u32, bits(position),
            bits(facing), u32::from(airborne), ZERO_ARGUMENT]));
        let member = self.opt(self.resp("member"));
        self.members.insert(slot, member);
        Ok(())
    }
    fn organization_virtual_104(&mut self, organization: Ref) -> Result<Binary> {
        self.events.push(json!(["organization_virtual_104", self.key(organization)]));
        Self::ext(&self.c["org"]["v104"])
    }
    fn command(&mut self, organization: Ref, c: MoveCommand) -> Result<()> {
        let (p, v, z) = (c.point.map(f32::to_bits), c.value.to_bits(), c.vector.map(f32::to_bits));
        self.events.push(json!(["command_build", [p[0], p[1], v, z[0], z[1]]]));
        self.events.push(json!(["command", self.key(organization), [p[0], p[1], z[0], z[1], v], c.byte_3c]));
        self.events.push(json!(["command_destroyed"]));
        Ok(())
    }
    fn ground_height(&mut self, xy: [f32; 2]) -> Result<Binary> {
        self.events.push(json!(["ground_height", bits(xy), 0x122]));
        Self::ext(self.resp("ground"))
    }
    fn commit_pose(&mut self, thing: Ref, position: [f32; 3], facing: [f32; 3], speed: f32) -> Result<()> {
        self.events.push(json!(["commit_pose", self.key(thing), bits(position), bits(facing), speed.to_bits()]));
        Ok(())
    }
    fn begin_aimed_fall(&mut self, thing: Ref, drag: f32, target: [f32; 2]) -> Result<()> {
        self.events.push(json!(["begin_aimed_fall", self.key(thing), drag.to_bits(), bits(target), 0]));
        Ok(())
    }
    fn destroy(&mut self, thing: Ref) -> Result<()> {
        self.events.push(json!(["destroy", self.key(thing), 0]));
        Ok(())
    }
    fn remove_actor(&mut self, actor: Ref) -> Result<()> {
        self.events.push(json!(["remove_actor", self.key(actor), 1]));
        Ok(())
    }
}

fn transport(w: &World, c: &Value) -> Transport {
    let x = &c["transport"];
    Transport {
        plane: w.id("plane"),
        cargo: w.id(x["cargo"].as_str().unwrap()),
        layout: if c.get("layout").is_some_and(|l| !l.is_null()) { w.id("layout") } else { 0 },
        point: fs(&x["point"]),
        loaded: u(&x["loaded"]) as u8,
        count: u(&x["count"]) as i32,
        dropped: u(&x["dropped"]) as i32,
        organization: u(&x["organization"]),
        next_slot: u(&x["next_slot"]) as i32,
        name: w.id("name"),
        attachment: u(&x["attachment"]),
    }
}

fn v_is_nan(bits: u32) -> bool {
    f32::from_bits(bits).is_nan()
}

fn state(t: &Transport) -> Value {
    json!({"loaded": t.loaded, "count": t.count as u32, "dropped": t.dropped as u32,
        "organization": t.organization, "next_slot": t.next_slot as u32})
}

#[test]
fn transport_drop_matches_native() {
    let d = document();
    let cases = d["cases"].as_array().unwrap();
    assert!(cases.len() >= 1000);
    let mut counts = BTreeMap::<String, usize>::new();
    for case in cases {
        let (c, o) = (&case["input"], &case["output"]);
        let name = c["name"].as_str().unwrap();
        let precision: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let mut w = World::new(c);
        let mut t = transport(&w, c);
        let op = c["operation"].as_str().unwrap();
        *counts.entry(op.into()).or_default() += 1;
        match op {
            "drop" => {
                let context = DropContext {
                    precision,
                    map: MapBounds { cell_size: f(&c["map"]["cell"]), extent: fs(&c["map"]["extent"]) },
                    gravity: f(&c["gravity"]),
                    svar_3c: f(&c["svar_3c"]),
                    zero_vector: fs(&c["zero_vector"]),
                };
                drop_one(&mut t, &context, &mut w).unwrap_or_else(|e| panic!("{name}: {e}"));
                assert_eq!(state(&t), o["transport"], "{name} transport");
            }
            "lead" => {
                let a = &c["lead"];
                let v = lead_distance(&t, f(&a["distance"]), f(&a["factor"]), f(&c["gravity"]), precision, &mut w)
                    .unwrap_or_else(|e| panic!("{name}: {e}"));
                let (ext, single) = match v {
                    Some(v) => (v.to_x87_extended_bits().expect("extended"), v.store_f32().unwrap()),
                    None => (INDEFINITE_EXTENDED, INDEFINITE_F32),
                };
                let e = o["extended"].as_array().unwrap();
                let native = (u128::from(e[1].as_u64().unwrap() as u16) << 64) | u128::from(e[0].as_u64().unwrap());
                assert_eq!(ext, native, "{name} extended");
                assert_eq!(single, u(&o["f32_bits"]), "{name} f32");
                *counts.entry(if v_is_nan(single) { "lead_nan".into() } else { "lead_value".into() }).or_default() += 1;
            }
            "mark" => {
                mark_loaded(&mut t, &mut w).unwrap();
                assert_eq!(state(&t), o["transport"], "{name} transport");
            }
            "leave" => {
                leave(&t, &mut w).unwrap();
                assert_eq!(state(&t), o["transport"], "{name} transport");
            }
            other => panic!("{other}"),
        }
        assert_eq!(Value::Array(w.events.clone()), o["events"], "{name} events");
    }
    assert!(counts["drop"] >= 900 && counts["lead"] >= 200, "{counts:?}");
    assert!(counts.contains_key("lead_nan") && counts.contains_key("lead_value"), "{counts:?}");
}

#[test]
fn transport_drop_fixture_covers_every_branch() {
    let d = document();
    // Only the four string-free paths (a reference count reaching zero) of
    // 58BCB0's 846 instructions stay unvisited: 809 = 846 - 37.
    assert_eq!(d["coverage"]["58BCB0"], 809);
    let kinds: std::collections::BTreeSet<String> = d["cases"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|c| c["output"]["events"].as_array().unwrap().iter())
        .map(|e| e[0].as_str().unwrap().to_string())
        .collect();
    for k in ["place_organization", "report", "create", "create_unloaded", "attachment_attach", "schedule",
        "organization_carrier", "record_recruit", "organization_add", "place_thing", "organization_place",
        "command", "commit_pose", "landing_search", "begin_aimed_fall", "destroy", "remove_actor"]
    {
        assert!(kinds.contains(k), "{k}");
    }
}
