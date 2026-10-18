//! Replays research/phase8-actor-creation/native-actor-creation.json: the
//! ORIGINAL 4F9C10 / 4FA030 / 4FA150 on A25B28 actor blocks (class 3) under
//! Unicorn. The Rust composition (aa_actors::actor_creation) must make the
//! same seam calls in the same order with the same arguments, leave the same
//! RNG state, GTID manager, actor fields, parameter write-backs, world lists
//! and latches.
use aa_actors::actor_creation::{
    self, ActorDefinition, ActorParams, Descriptor, FxList, Log, NameList, Shape, ShapeRef, Shell, TrackInfo,
    UiAgent, World,
};
use aa_actors::components::ComponentSlot;
use aa_actors::creation::DirectionTable;
use aa_actors::init_create::FactoryKind;
use aa_actors::registry::{ActorId, IdentityRegistry, RegistryError};
use aa_numeric::{Binary, Precision};
use aa_simulation::NativeRng;
use serde_json::{Value, json};
use std::cell::RefCell;
use std::collections::BTreeMap;

const FIXTURE: &str = include_str!("../../../research/phase8-actor-creation/native-actor-creation.json");
const KINGDOM: u32 = 0x0007_0abc;

fn n(v: &Value) -> u32 {
    v.as_u64().unwrap_or_else(|| panic!("number expected: {v}")) as u32
}
fn u32s(v: &Value) -> Vec<u32> {
    v.as_array().map(|a| a.iter().map(n).collect()).unwrap_or_default()
}
fn text(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}
fn pc(name: &str) -> Precision {
    match name {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        _ => Precision::Extended64,
    }
}
fn kind_name(slot: ComponentSlot, kind: FactoryKind) -> String {
    match kind {
        FactoryKind::OrganizationMind => "4org".into(),
        FactoryKind::AircraftMind => "4air".into(),
        FactoryKind::ElementMind => "4elem".into(),
        FactoryKind::Standard(s) => {
            assert_eq!(s, slot);
            s.index().to_string()
        }
    }
}
/// Native requirement virtual (+20) of each constructed class.
fn requirement(kind: &str) -> Option<ComponentSlot> {
    let r = match kind {
        "4org" | "18" => 4,
        "4air" => 0x17,
        "4elem" => 8,
        "15" => 0,
        "17" => 6,
        "20" | "23" => 5,
        "24" => 3,
        _ => return None,
    };
    ComponentSlot::from_index(r)
}

#[derive(Clone, Debug)]
struct Comp {
    kind: String,
}

struct Fixture {
    spec: Value,
    res: Value,
    rng: NativeRng,
    registry: IdentityRegistry<usize>,
    calls: RefCell<Vec<Value>>,
    shells: Vec<Shell<Fixture>>,
    gtids: Vec<u32>,
    world_list: Vec<usize>,
    available: Vec<u32>,
    names: Vec<(String, i32)>,
    latches: BTreeMap<u32, bool>,
    filled: bool,
    extra: u32,
}

const OBJECT: usize = usize::MAX;

impl Fixture {
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
            if s[2] == 1 { live.push(id) } else { seeds.push(id) }
        }
        let mut registry = IdentityRegistry::from_load_metadata(n(&idm["cursor"]), highest, &gens).unwrap();
        for id in live {
            registry.seed_saved_generation(id).unwrap();
            registry.register_saved(id, false, OBJECT).unwrap();
        }
        for id in seeds {
            registry.seed_saved_generation(id).unwrap();
        }
        let mut extra = 0;
        if let Some(objects) = spec["objects"].as_object() {
            for (gtid, o) in objects {
                let id = u32::from_str_radix(gtid.trim_start_matches("0x"), 16).unwrap();
                let live = o.get("live").is_none_or(|v| v == true);
                if live {
                    registry.seed_saved_generation(ActorId::from_raw(id)).unwrap();
                    registry.register_saved(ActorId::from_raw(id), false, OBJECT).unwrap();
                    extra += 1;
                } else {
                    let generation = o.get("generation").map_or((id >> 16) + 1, n);
                    registry.seed_saved_generation(ActorId::from_raw((generation << 16) | (id & 0xffff))).unwrap();
                }
            }
        }
        let count = n(&idm["count"]) as usize;
        let mut filler = 20_000u32;
        let filled = registry.len() - extra as usize > count;
        let filled = filled || registry.len() - (extra as usize) < count;
        while registry.len() - (extra as usize) < count {
            registry.register_saved(ActorId::from_raw(filler), false, OBJECT).unwrap();
            filler += 1;
        }
        let mut latches = BTreeMap::new();
        for a in [0xb3cc40u32, 0xb3cc42, 0xb3cc43, 0xb3cc44] {
            latches.insert(a, spec["latches"][format!("{a:#x}")].as_u64().unwrap_or(0) != 0);
        }
        let names = spec["name_vector"]
            .as_array()
            .map(|v| v.iter().map(|e| (format!("names:{}", e[0].as_str().unwrap()), n(&e[1]) as i32)).collect())
            .unwrap_or_default();
        Self {
            spec: spec.clone(),
            res: spec["results"].clone(),
            rng: NativeRng::new(n(&spec["rng"])),
            registry,
            calls: RefCell::new(Vec::new()),
            shells: Vec::new(),
            gtids: Vec::new(),
            world_list: Vec::new(),
            available: Vec::new(),
            names,
            latches,
            filled,
            extra,
        }
    }
    fn push(&self, v: Value) {
        self.calls.borrow_mut().push(v);
    }
    fn label(&self, a: usize) -> String {
        format!("actor{a}")
    }
    fn result(&self, name: &str, actor: usize, kind: &str) -> u32 {
        let r = &self.res;
        r.get(format!("{name}:{actor}:{kind}"))
            .or_else(|| r.get(format!("{name}:{kind}")))
            .map_or(1, n)
    }
    fn kind(&self, a: usize, slot: ComponentSlot) -> String {
        self.shells[a].actor.as_ref().unwrap().components.get(slot).unwrap().kind.clone()
    }
    fn owner_label(&self, gtid: u32) -> Value {
        if gtid == KINGDOM { json!("kingdom") } else { json!(gtid) }
    }
    fn org(&self) -> &Value {
        &self.spec["organization"]
    }
}

fn desc(d: &Descriptor) -> Value {
    json!([match d.shape { ShapeRef::A => "A", ShapeRef::B => "B" },
           d.facing_bits[0], d.facing_bits[1], d.position_bits[0], d.position_bits[1]])
}
type R<T> = actor_creation::Result<T>;

impl World for Fixture {
    type Definition = String;
    type Layout = String;
    type Component = Comp;
    type Actor = usize;

    fn definition(&self, key: &String) -> R<ActorDefinition<String, String>> {
        let d = &self.spec["definitions"][key.as_str()];
        let sa = d.get("shape_a").map(u32s).unwrap_or(vec![1, 0x3f00_0000, 0, 0, 0, 0]);
        let names = d.get("names").map(|v| NameList {
            id: format!("names:{key}"),
            names: v.as_array().unwrap().iter().map(|s| text(s.as_str().unwrap())).collect(),
        });
        let tracks = d
            .get("tracks")
            .map(|v| v.as_array().unwrap().iter().map(|t| TrackInfo { kind: n(&t[0]), length_bits: n(&t[1]) }).collect())
            .unwrap_or_default();
        let node = d.get("node_offset").map(u32s).unwrap_or(vec![0, 0, 0]);
        Ok(ActorDefinition {
            key: key.clone(),
            ids_utf16: text(key),
            display_utf16: text(&format!("display_{key}")),
            runtime_class: d.get("class").map_or(3, n),
            flags: n(&d["flags"]),
            height_bits: d.get("height").map_or(0x3fa0_0000, n),
            scale_min_bits: n(&d["scale"][0]),
            scale_max_bits: n(&d["scale"][1]),
            settlement_space: d.get("settlement").is_some_and(|v| n(v) != 0),
            shape_a: Shape { kind: sa[0], radius_bits: sa[1], extent_10_bits: sa[4], extent_14_bits: sa[5] },
            variations_enabled: d.get("variations").is_some_and(|v| n(v) != 0),
            aspect_weights_bits: d.get("aspects").map(u32s).unwrap_or_default(),
            total_weight_bits: d.get("total_weight").map_or(0, n),
            component_mask: n(&d["mask"]),
            names,
            numbered: d.get("numbered").is_some_and(|v| n(v) != 0),
            tracks,
            trailer: d.get("trailer").and_then(|v| v.as_str()).map(str::to_owned),
            node_offset_bits: [node[0], node[1], node[2]],
            layout: d.get("layout").filter(|v| !v.is_null()).map(|_| format!("layout:{key}")),
            required_object: d.get("required_object").is_some_and(|v| n(v) != 0),
            zone_radius_bits: d.get("zone_radius").map_or(0, n),
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
        self.spec.get("world_d8").map_or(0, n) != 6 && self.spec.get("scene").is_none_or(|v| v == true)
    }
    fn ui_agent(&self) -> UiAgent {
        if self.spec.get("sai").is_some_and(|v| n(v) != 0) {
            UiAgent::Present { watching: self.spec.get("sai_watching").is_some_and(|v| n(v) != 0) }
        } else {
            UiAgent::Absent
        }
    }
    fn registered_state_count(&self) -> u32 {
        self.spec.get("registered_states").map_or(0, n)
    }
    fn global_property(&self) -> Option<u32> {
        self.spec.get("global_property").map(n).filter(|v| *v != 0)
    }
    fn track_settings(&self) -> R<(u32, i32)> {
        Ok((self.spec.get("track_scale").map_or(0x4120_0000, n), self.spec.get("track_divisor").map_or(4, n) as i32))
    }
    fn thing_manager_74_bits(&self) -> R<u32> {
        Ok(self.spec.get("thing_manager_74").map_or(0x3fc0_0000, n))
    }
    fn latch(&self, address: u32) -> bool {
        self.latches[&address]
    }
    fn set_latch(&mut self, address: u32, value: bool) {
        self.latches.insert(address, value);
    }
    fn assertion_dialog(&mut self, expression: &'static str, line: u32) -> R<bool> {
        self.push(json!(["assert", expression, line]));
        Ok(self.res.get("assert").is_some_and(|v| n(v) != 0))
    }
    fn log(&mut self, entry: Log) -> R<()> {
        let v = match entry {
            Log::Create { line, format, .. } => json!(["log", line, format]),
            Log::NoIdsLeft => json!(["log", 0xab, "KKC_GTIDManager::Add() - no IDs left"]),
            Log::MissingComponent { .. } => {
                json!(["log", 0x5e8, "Actor %s component %d requires component %d which is missing"])
            }
            Log::NotThickEnough { .. } => json!(["log", 0x213, "Static object %s is not thick enough to block tiles"]),
            Log::PlacementFailed { .. } => json!(["log", 299, "creating thing %s failed (reason: %s) at %f,%f"]),
            Log::ThingDoesntFit { .. } => json!(["map warning", "load_warning_thing_doesnt_fit|sff"]),
            Log::RequiredObjectDead { .. } => json!(["log", 0x151, "%s: failed placement due to dead required_object"]),
            Log::RequiredObjectIgnored { .. } => {
                json!(["log", 0x15a, "%s: ignoring failed placement due to 'required object' while loading"])
            }
            Log::RequiredObjectPlaced { .. } => json!(["log", 0x161, "%s: successful placement on required_object %d"]),
            Log::NoMembers { .. } => json!(["log", 0x8a0, "org %d failed to create any elements - destroying"]),
        };
        self.push(v);
        Ok(())
    }
    fn allocate(&mut self, shell: Shell<Self>) -> R<usize> {
        self.shells.push(shell);
        self.gtids.push(0);
        let a = self.shells.len() - 1;
        self.push(json!(["alloc", a]));
        Ok(a)
    }
    fn shell(&self, a: usize) -> R<&Shell<Self>> {
        Ok(&self.shells[a])
    }
    fn shell_mut(&mut self, a: usize) -> R<&mut Shell<Self>> {
        Ok(&mut self.shells[a])
    }
    fn gtid(&self, a: usize) -> R<u32> {
        Ok(self.gtids[a])
    }
    fn register_requested_id(&mut self, a: usize, id: u32) -> R<()> {
        let id = ActorId::from_raw(id);
        self.registry.seed_saved_generation(id).map_err(|e| actor_creation::Error::World(e.to_string()))?;
        self.registry.register_saved(id, true, a).map_err(|e| actor_creation::Error::World(e.to_string()))?;
        self.gtids[a] = id.raw();
        Ok(())
    }
    fn allocate_id(&mut self, a: usize) -> R<Option<u32>> {
        match self.registry.allocate(true, a) {
            Ok(id) => {
                self.gtids[a] = id.raw();
                Ok(Some(id.raw()))
            }
            Err(RegistryError::Exhausted) => Ok(None),
            Err(e) => Err(actor_creation::Error::World(e.to_string())),
        }
    }
    fn gtid_live(&self, id: u32) -> R<bool> {
        Ok(self.registry.get(ActorId::from_raw(id)).is_some())
    }
    fn resolve_actor(&self, id: u32) -> R<Option<usize>> {
        Ok(self.registry.get(ActorId::from_raw(id)).map(|e| e.value).filter(|v| *v != OBJECT))
    }
    fn destroy(&mut self, a: usize) -> R<()> {
        self.push(json!(["destroy", self.label(a)]));
        Ok(())
    }
    fn scene_transform(&mut self, a: usize) -> R<()> {
        self.push(json!(["scene transform", self.label(a)]));
        Ok(())
    }
    fn scene_add(&mut self, a: usize) -> R<()> {
        self.push(json!(["scene add", self.label(a)]));
        Ok(())
    }
    fn creation_fx(&mut self, a: usize, list: FxList) -> R<()> {
        let p = self.shells[a].thing.position_bits;
        self.push(json!(["creation fx", self.label(a), match list { FxList::Created => "BC", FxList::Existing => "E4" }, p]));
        Ok(())
    }
    fn instance_name_gtid(&mut self, name: &[u16]) -> R<Option<u32>> {
        self.push(json!(["name find", String::from_utf16_lossy(name)]));
        Ok(self.res.get("name_entry").and_then(|v| v.as_str()).map(|s| u32::from_str_radix(&s[2..], 16).unwrap()))
    }
    fn organization_set_layout(&mut self, a: usize, layout: &String) -> R<()> {
        self.push(json!(["organization layout", self.label(a), layout]));
        Ok(())
    }
    fn state_group(&self, a: usize, group: u32) -> R<Option<u32>> {
        self.push(json!(["state group", self.label(a), group]));
        Ok(self.res.get("group13").map(n).filter(|v| *v != 0))
    }
    fn set_state(&mut self, a: usize, state: u32, enabled: bool) -> R<()> {
        self.push(json!(["set state", self.label(a), state, u32::from(enabled)]));
        let s = &mut self.shells[a].actor.as_mut().unwrap().components.state;
        let word = if state < 32 { &mut s.low } else { &mut s.high };
        if enabled { *word |= 1 << (state & 31) } else { *word &= !(1 << (state & 31)) }
        Ok(())
    }
    fn construct_component(&mut self, a: usize, slot: ComponentSlot, kind: FactoryKind) -> R<()> {
        let k = kind_name(slot, kind);
        self.push(json!(["ctor", self.label(a), k]));
        self.shells[a].actor.as_mut().unwrap().components.insert(slot, Comp { kind: k });
        Ok(())
    }
    fn component_requirement(&mut self, a: usize, slot: ComponentSlot) -> R<Option<ComponentSlot>> {
        let k = self.kind(a, slot);
        if k == "13" {
            // 589090 calls 4D1050 on the owner, then returns -1.
            self.push(json!(["4D1050", self.label(a)]));
        }
        Ok(requirement(&k))
    }
    fn component_init_create(&mut self, a: usize, slot: ComponentSlot, _: &ActorParams<String, String>) -> R<bool> {
        let k = self.kind(a, slot);
        self.push(json!(["InitCreate", self.label(a), k]));
        Ok(self.result("InitCreate", a, &k) != 0)
    }
    fn component_init_existing(&mut self, a: usize, slot: ComponentSlot) -> R<bool> {
        let k = self.kind(a, slot);
        self.push(json!(["InitExisting", self.label(a), k]));
        Ok(self.result("InitExisting", a, &k) != 0)
    }
    fn component_vf14(&mut self, a: usize, slot: ComponentSlot) -> R<()> {
        let k = self.kind(a, slot);
        self.push(json!(["vf14", self.label(a), k]));
        Ok(())
    }
    fn component_post_create(&mut self, a: usize, slot: ComponentSlot, _: &ActorParams<String, String>, flag: u32) -> R<()> {
        let k = self.kind(a, slot);
        self.push(json!(["PostCreate", self.label(a), k, flag]));
        Ok(())
    }
    fn ui_components_created(&mut self, _: usize, owner: u32) -> R<()> {
        self.push(json!(["ui components created", self.owner_label(owner)]));
        Ok(())
    }
    fn character_view(&self, a: usize) -> R<(bool, bool)> {
        self.push(json!(["character view", self.label(a)]));
        Ok((self.res.get("view_count").map_or(1, n) != 0, self.res.get("view_first").map_or(1, n) != 0))
    }
    fn character_start(&mut self, a: usize) -> R<()> {
        self.push(json!(["character start", self.label(a)]));
        Ok(())
    }
    fn character_default_stance(&mut self, a: usize) -> R<()> {
        self.push(json!(["default stance", self.label(a)]));
        Ok(())
    }
    fn name_assignments(&self) -> R<Vec<(String, i32)>> {
        Ok(self.names.clone())
    }
    fn push_name_assignment(&mut self, list: &str, index: i32) -> R<()> {
        self.names.push((list.to_owned(), index));
        Ok(())
    }
    fn set_name(&mut self, a: usize, name: &[u16]) -> R<()> {
        self.push(json!(["set name", self.label(a), String::from_utf16_lossy(name)]));
        Ok(())
    }
    fn assign_number(&mut self, owner: u32, a: usize) -> R<()> {
        self.push(json!(["assign number", self.owner_label(owner), self.label(a)]));
        let number = self.res.get("number").map_or(7, n);
        self.shells[a].actor.as_mut().unwrap().words.insert(0x170, number);
        Ok(())
    }
    fn body_health(&self, _: usize) -> R<[u32; 2]> {
        Ok([self.res.get("body_health").map_or(0x42a0_0000, n), self.res.get("body_maximum").map_or(0x42c8_0000, n)])
    }
    fn copy_health(&mut self, child: usize, health: [u32; 2]) -> R<()> {
        self.push(json!(["copy health", self.label(child), health[0], health[1]]));
        Ok(())
    }
    fn register_construction(&mut self, owner: u32, definition: &String) -> R<()> {
        self.push(json!(["register construction", self.owner_label(owner), definition]));
        Ok(())
    }
    fn snap(&mut self, _: &Shape, xy: [u32; 2], tile: u32) -> R<[u32; 2]> {
        self.push(json!(["snap", xy, tile]));
        Ok(match self.res.get("snap") {
            Some(v) if !v.is_null() => [n(&v[0]), n(&v[1])],
            _ => xy,
        })
    }
    fn placement_test(&mut self, a: usize, _: &ActorParams<String, String>) -> R<u32> {
        self.push(json!(["placement test", self.label(a)]));
        Ok(self.res.get("placement").map_or(0, n))
    }
    fn index_insert(&mut self, a: usize, b: Descriptor, d: Descriptor) -> R<()> {
        self.push(json!(["index insert", self.label(a), desc(&b), desc(&d)]));
        self.shells[a].thing.flags |= 1 << 26;
        Ok(())
    }
    fn index_remove(&mut self, a: usize, b: Descriptor, d: Descriptor) -> R<()> {
        self.push(json!(["index remove", self.label(a), desc(&b), desc(&d)]));
        self.shells[a].thing.flags &= !(1 << 26);
        Ok(())
    }
    fn terrain_height(&mut self, xy: [u32; 2], flags: u32) -> R<Binary> {
        self.push(json!(["terrain height", xy, flags]));
        Ok(Binary::from_f32_bits(self.res.get("height").map_or(0x40e8_0000, n)).unwrap())
    }
    fn blocking_notify(&mut self, a: usize) -> R<()> {
        self.push(json!(["blocking notify", self.label(a)]));
        Ok(())
    }
    fn tile_footprint(&mut self, a: usize) -> R<()> {
        self.push(json!(["tile footprint", self.label(a)]));
        Ok(())
    }
    fn world_list_insert(&mut self, a: usize) -> R<()> {
        self.world_list.insert(0, a);
        Ok(())
    }
    fn visibility_refresh(&mut self, a: usize) -> R<()> {
        self.push(json!(["visibility refresh", self.label(a)]));
        Ok(())
    }
    fn find_required_object(&mut self, _: &String, xy: [u32; 2]) -> R<Option<u32>> {
        self.push(json!(["find required", xy]));
        Ok(self.res.get("required_found").and_then(|v| v.as_str()).map(|s| u32::from_str_radix(&s[2..], 16).unwrap()))
    }
    fn required_placement_blocked(&mut self, _: u32, _: usize) -> R<bool> {
        self.push(json!(["required blocked"]));
        Ok(self.res.get("required_blocked").is_some_and(|v| n(v) != 0))
    }
    fn required_object_attach(&mut self, required: u32, a: usize) -> R<()> {
        self.push(json!(["object vf128", format!("{required:#x}"), self.label(a)]));
        Ok(())
    }
    fn refresh_4d4cf0(&mut self, a: usize) -> R<()> {
        self.push(json!(["4D4CF0", self.label(a)]));
        Ok(())
    }
    fn refresh_4d1350(&mut self, a: usize) -> R<()> {
        self.push(json!(["4D1350", self.label(a)]));
        Ok(())
    }
    fn refresh_4d67f0(&mut self, a: usize) -> R<()> {
        self.push(json!(["4D67F0", self.label(a)]));
        Ok(())
    }
    fn refresh_4d4130(&mut self, a: usize) -> R<()> {
        self.push(json!(["4D4130", self.label(a)]));
        Ok(())
    }
    fn property_applies(&mut self, a: usize, property: u32) -> R<bool> {
        self.push(json!(["property applies", self.label(a), property]));
        Ok(self.res.get("5035E0").map_or(1, n) != 0)
    }
    fn properties_add(&mut self, a: usize, property: u32) -> R<()> {
        self.push(json!(["properties add", self.label(a), property]));
        Ok(())
    }
    fn organization_refresh(&mut self, a: usize) -> R<()> {
        self.push(json!(["organization refresh", self.label(a)]));
        Ok(())
    }
    fn settlement_refresh(&mut self, a: usize) -> R<()> {
        self.push(json!(["settlement refresh", self.label(a)]));
        Ok(())
    }
    fn performer_refresh(&mut self, a: usize) -> R<()> {
        self.push(json!(["performer refresh", self.label(a)]));
        Ok(())
    }
    fn denizen_refresh(&mut self, a: usize) -> R<()> {
        self.push(json!(["denizen refresh", self.label(a)]));
        Ok(())
    }
    fn create_zone(&mut self, a: usize, index: u32) -> R<bool> {
        // 4D69A0 (aa_zones::component::create_zone): nine empty slots on
        // first use (+14C = 9, capacity +150 = 9), then 516640 into an empty
        // slot (the dead-actor log needs flag bit 0, never copied here).
        if self.shells[a].actor.as_ref().unwrap().zones_148.is_empty() {
            self.shells[a].actor.as_mut().unwrap().zones_148.zones_mut().slots = vec![None; 9];
        }
        if self.shells[a].actor.as_ref().unwrap().zones_148.zones().slots[index as usize].is_none() {
            self.push(json!(["zone ctor", self.label(a), 1u32 << index]));
            // This oracle deliberately intercepts the Zone constructor/place
            // bodies. Keep its logical seam identity in the same typed owner.
            self.shells[a].actor.as_mut().unwrap().zones_148.zones_mut().slots[index as usize] =
                Some(aa_zones::zone::Zone::new(index+1, self.gtids[a], (1u32 << index) as u16));
        }
        Ok(true)
    }
    fn move_zone(&mut self, _: usize, _: u32, xy: [u32; 2], radius: u32) -> R<()> {
        self.push(json!(["zone place", xy[0], xy[1], radius]));
        Ok(())
    }
    fn zone_count(&self, a: usize) -> R<i32> {
        Ok(self.shells[a].actor.as_ref().unwrap().zones_148.len() as i32)
    }
    fn sai_notify(&mut self, a: usize) -> R<()> {
        self.push(json!(["SAI notify", self.label(a)]));
        Ok(())
    }
    fn track_updatable(&mut self, a: usize, period: i32) -> R<()> {
        self.push(json!(["track updatable", self.label(a), period]));
        Ok(())
    }
    fn ground_paint(&mut self, a: usize, _: &ActorParams<String, String>) -> R<()> {
        self.push(json!(["ground paint", self.label(a)]));
        Ok(())
    }
    fn kingdom_add_actor(&mut self, owner: u32, a: usize) -> R<()> {
        self.push(json!(["kingdom add", self.owner_label(owner), self.label(a)]));
        Ok(())
    }
    fn city_create(&mut self, a: usize, word_34: u32) -> R<()> {
        self.push(json!(["city create", self.label(a), word_34]));
        Ok(())
    }
    fn instance_name_remove(&mut self, _: &[u16]) -> R<()> {
        self.push(json!(["name remove"]));
        Ok(())
    }
    fn instance_name_insert(&mut self, _: &[u16], _: u32) -> R<()> {
        self.push(json!(["name insert"]));
        Ok(())
    }
    fn state_reenter(&mut self, a: usize, state: u32) -> R<()> {
        self.push(json!(["state reenter", self.label(a), state]));
        Ok(())
    }
    fn grid_register(&mut self, a: usize) -> R<()> {
        self.push(json!(["grid register", self.label(a)]));
        Ok(())
    }
    fn object_settlement_space(&self, gtid: u32) -> R<bool> {
        Ok(self.spec["objects"][format!("{gtid:#x}")].get("settlement").is_some_and(|v| n(v) != 0))
    }
    fn available_contains(&self, gtid: u32) -> R<bool> {
        Ok(self.available.contains(&gtid))
    }
    fn add_available(&mut self, gtid: u32) -> R<()> {
        self.available.insert(0, gtid);
        Ok(())
    }
    fn layout_slot_count(&mut self, layout: &String) -> R<i32> {
        let def = layout.strip_prefix("layout:").unwrap();
        Ok(n(&self.spec["definitions"][def]["layout"]) as i32)
    }
    fn layout_first_slot(&mut self, _: &String) -> R<u32> {
        self.push(json!(["first slot"]));
        Ok(self.res.get("first_slot").map_or(0, n))
    }
    fn layout_slot_placement(&mut self, _: &String, slot: u32, xy: &mut [u32; 2], facing: [u32; 2]) -> R<bool> {
        self.push(json!(["slot placement", slot, *xy, facing]));
        if let Some(v) = self.res.get("slot_xy").filter(|v| !v.is_null()) {
            *xy = [n(&v[0]), n(&v[1])];
        }
        Ok(self.res.get("slot_ok").map_or(1, n) != 0)
    }
    fn organization_leader(&mut self, _: usize) -> R<i32> {
        Ok(n(&self.org()["leader"]) as i32)
    }
    fn organization_slot_count(&mut self, _: usize) -> R<i32> {
        Ok(self.org()["slot_groups"].as_array().unwrap().len() as i32)
    }
    fn organization_slot_defined(&mut self, _: usize, slot: i32) -> R<bool> {
        let group = &self.org()["slot_groups"][slot as usize];
        Ok(self.org()["defined_groups"].as_array().unwrap().contains(group))
    }
    fn spawn_member(&mut self, a: usize, slot: i32, flag: u8) -> R<()> {
        self.push(json!(["spawn member", self.label(a), slot, flag]));
        Ok(())
    }
    fn member_count(&mut self, _: usize) -> R<u32> {
        self.push(json!(["member count"]));
        Ok(self.res.get("member_count").map_or(1, n))
    }
}

/// The oracle's recorded calls in the fixture's normalized form.
fn normalize(calls: &[Value]) -> Vec<Value> {
    let actor_of = |label: &Value| -> Value {
        // "comp0:2" -> "actor0"
        let s = label.as_str().unwrap();
        let idx = s.trim_start_matches("comp").split(':').next().unwrap();
        json!(format!("actor{idx}"))
    };
    let mut out = Vec::new();
    for c in calls {
        let seam = c["seam"].as_str().unwrap();
        let v = match seam {
            "alloc actor" => json!(["alloc", c["index"]]),
            "scene transform" => json!(["scene transform", c["args"][0]]),
            "scene add" => json!(["scene add", c["args"][0]]),
            "state group" => json!(["state group", c["actor"], c["group"]]),
            "set state" => json!(["set state", c["actor"], c["state"], c["enabled"]]),
            "ctor" => json!(["ctor", c["actor"], c["kind"]]),
            "InitCreate" | "InitExisting" | "vf14" => json!([seam, c["actor"], c["kind"]]),
            "PostCreate" => json!(["PostCreate", c["actor"], c["kind"], c["args"][1]]),
            "default stance" | "character start" => json!([seam, actor_of(&c["ecx"])]),
            "character view" => json!(["character view", c["actor"]]),
            "set name" => json!(["set name", c["actor"], c["text"]]),
            "assign number" => json!(["assign number", c["kingdom"], c["actor"]]),
            "copy health" => json!(["copy health", actor_of(&c["ecx"]), c["args"][0], c["args"][1]]),
            "register construction" => {
                let d = c["args"][1].as_str().unwrap().trim_start_matches("def:");
                json!(["register construction", c["args"][0], d])
            }
            "organization layout" => json!(["organization layout", actor_of(&c["component"]), c["layout"]]),
            "assert dialog" => json!(["assert", c["expression"], c["line"]]),
            "4154D0" => json!(["name find", c["text"]]),
            "5C9180" => json!(["name remove"]),
            "5043F0" => continue,
            "4F7B70" => json!(["name insert"]),
            "actor placement test" => json!(["placement test", c["args"][1]]),
            "index insert" | "index remove" => json!([seam, c["object"], c["a"], c["b"]]),
            "terrain height" => json!(["terrain height", c["xy"], c["flags"]]),
            "snap" => json!(["snap", c["xy"], c["tile"]]),
            "blocking query shape" => json!(["blocking notify", c["args"][0]]),
            "blocking query" => continue,
            "tile footprint" | "visibility refresh" => json!([seam, c["ecx"]]),
            "find required object" => json!(["find required", c["xy"]]),
            "required placement blocked" => json!(["required blocked"]),
            "object vf128" => json!(["object vf128", c["object"], c["actor"]]),
            "4D4CF0" | "4D1350" | "4D67F0" | "4D4130" => json!([seam, c["ecx"]]),
            "5035E0" => json!(["property applies", c["args"][0], c["args"][1]]),
            "properties add" => json!(["properties add", c["args"][2], c["args"][0]]),
            "organization refresh" | "settlement refresh" | "performer refresh" | "denizen refresh" => {
                json!([seam, actor_of(&c["ecx"])])
            }
            "zone ctor" => json!(["zone ctor", c["actor"], c["mask"]]),
            "zone place" => json!(["zone place", c["xyz"][0], c["xyz"][1], c["xyz"][2]]),
            "SAI notify" => json!(["SAI notify", c["args"][0]]),
            "SAI 79EA90" => json!(["ui components created", c["args"][0]]),
            "track updatable" => json!(["track updatable", c["actor"], c["period"]]),
            "creation fx" => {
                let list = c["list"].as_str().unwrap();
                json!(["creation fx", c["object"], &list[list.len() - 2..], c["pos"]])
            }
            "ground paint" => json!(["ground paint", c["ecx"]]),
            "kingdom add actor" => json!(["kingdom add", c["ecx"], c["args"][0]]),
            "city create" => json!(["city create", c["args"][0], c["args"][1]]),
            "state reenter" => json!(["state reenter", c["ecx"], n(&c["args"][0]) - 0x5e00_0000]),
            "grid register" => json!(["grid register", c["ecx"]]),
            "destroy" => json!(["destroy", c["args"][0]]),
            "404ED0" | "log 404F60" | "log 404EA0" => json!(["log", c["line"], c["format"]]),
            "4B9F40" => continue,
            "5A1040" => json!(["map warning", c["text"]]),
            "4D1050" => json!(["4D1050", c["ecx"]]),
            "57AD30" => json!(["spawn member", c["actor"], c["slot"], c["flag"]]),
            "57A730" => json!(["member count"]),
            "4C2BC0" => json!(["first slot"]),
            "578290" => json!(["slot placement", c["slot"], c["xy"], c["facing"]]),
            other => panic!("unnormalized oracle seam {other}: {c}"),
        };
        out.push(v);
    }
    out
}

fn params_from(spec: &Value) -> ActorParams<String, String> {
    let p = &spec["params"];
    let mut params = ActorParams::new(spec["definition"].as_str().unwrap().to_owned());
    let pos = u32s(&p["position"]);
    let dir = u32s(&p["direction"]);
    params.position_bits = [pos[0], pos[1], pos[2]];
    params.direction_bits = [dir[0], dir[1]];
    params.name_utf16 = text(p.get("name").and_then(|v| v.as_str()).unwrap_or(""));
    params.variant = p.get("variant").map_or(u32::MAX, n);
    params.requested_id = p.get("requested_id").map_or(0, n);
    params.owner = if p.get("owner").is_some_and(|v| v == true) { KINGDOM } else { 0 };
    params.organization_layout = p.get("layout").filter(|v| !v.is_null()).map(|_| "params layout".to_owned());
    params.word_30 = p.get("word_30").map_or(0, n);
    params.word_34 = p.get("word_34").map_or(u32::MAX, n);
    params.mode = p.get("mode").map_or(0, n);
    params.word_3c = p.get("word_3c").map_or(0, n);
    params.word_40 = p.get("word_40").map_or(0, n);
    params.word_44 = p.get("word_44").map_or(0, n);
    params.word_48 = p.get("word_48").map_or(0, n);
    params.byte_50 = p.get("byte_50").map_or(0, n) as u8;
    params.byte_51 = p.get("byte_51").map_or(0, n) as u8;
    params.byte_52 = p.get("byte_52").map_or(0, n) as u8;
    params
}

/// Compare one oracle actor image with the Rust shell (pointer words by
/// their labels; uninitialized 0xA5 words must be absent from the shell).
fn compare_actor(label: &str, index: usize, oracle: &Value, world: &Fixture) {
    let s = &world.shells[index];
    let w = oracle["words"].as_array().unwrap();
    let num = |off: usize| n(&w[off / 4]);
    let o = s.object.as_ref().unwrap();
    let a = s.actor.as_ref().unwrap();
    assert_eq!(num(0x8), s.thing.flags, "{label} actor{index} flags");
    assert_eq!(num(0xc), s.thing.word_c, "{label} +C");
    assert_eq!(num(0x10), s.thing.word_10, "{label} +10");
    assert_eq!(num(0x14), world.gtids[index], "{label} gtid");
    let name: Vec<u16> = w[6]["string"].as_array().unwrap().iter().map(|v| n(v) as u16).collect();
    assert_eq!(name, s.thing.name_utf16, "{label} name");
    assert_eq!(Some(num(0x1c)), s.thing.word_1c, "{label} +1C");
    for i in 0..3 {
        assert_eq!(num(0x20 + 4 * i), s.thing.position_bits[i], "{label} actor{index} position {i}");
        assert_eq!(num(0x2c + 4 * i), s.thing.facing_bits[i], "{label} actor{index} facing {i}");
    }
    assert_eq!(num(0x38), o.associated_handle, "{label} +38");
    assert_eq!(num(0x3c), o.scale_bits, "{label} scale");
    assert_eq!(num(0x40), o.variant as u32, "{label} variant");
    assert_eq!(num(0x44), o.word_44, "{label} +44");
    assert_eq!(num(0x48), o.word_48, "{label} +48");
    assert_eq!(num(0x4c) & 0xff, u32::from(o.byte_4c), "{label} +4C");
    assert_eq!(num(0x4c) >> 8, 0xa5a5a5, "{label} +4D..+4F uninitialized");
    for i in 0..2 {
        assert_eq!(num(0x50 + 4 * i), o.words_50_to_54[i], "{label} +5{i}");
    }
    assert_eq!(num(0x58),o.memory.count() as u32,"{label} +58");
    assert_eq!(num(0x5c),u32::from(o.memory.allocation_present()),"{label} +5C constructor-null");
    assert_eq!(num(0x100), a.components.state.low, "{label} +100");
    assert_eq!(num(0x104), a.components.state.high, "{label} +104");
    for off in (0xdc..0x180).step_by(4) {
        if off == 0x100 || off == 0x104 {
            continue;
        }
        let ov = &w[off / 4];
        let rust = a.words.get(&(off as u32)).copied();
        match off {
            0xdc | 0xe0 => {
                assert!(ov["string"].as_array().unwrap().is_empty(), "{label} string {off:#x}");
                let text = if off == 0xdc { &a.string_dc_utf16 } else { &a.string_e0_utf16 };
                assert!(text.is_empty());
            }
            0xe8 => assert_eq!(
                if ov == "kingdom" { KINGDOM } else { n(ov) },
                rust.unwrap(),
                "{label} +E8"
            ),
            0x138 => {
                let expect = match ov.as_str() {
                    Some(s) => u32::from_str_radix(s.trim_start_matches("object 0x"), 16).unwrap(),
                    None => n(ov),
                };
                assert_eq!(expect, rust.unwrap(), "{label} +138");
            }
            0x108 | 0x10c => {
                assert!(rust.is_none());
                assert!(a.actor_list_108.head().is_none() && a.actor_list_108.tail().is_none());
                assert_eq!(n(ov),0,"{label} constructor-empty actor list");
            }
            0x110 | 0x114 => {
                assert!(rust.is_none());
                assert!(a.overlap_list_110.head().is_none() && a.overlap_list_110.tail().is_none());
                assert_eq!(n(ov),0,"{label} constructor-empty overlap list");
            }
            // +144: the TrackInfo updatable block (pointer, the seam's); +148:
            // the zone vector data pointer (aa-zones storage).
            0x144 => assert!(rust.is_some(), "{label} {off:#x} field present"),
            0x148 => assert!(rust.is_none(), "{label} zone data has sole typed owner"),
            0x14c => { assert!(rust.is_none()); assert_eq!(n(ov),a.zones_148.len() as u32,"{label} zone size"); },
            0x150 => { assert!(rust.is_none()); assert_eq!(n(ov),a.zones_148.native_capacity(),"{label} zone capacity"); },
            _ => match ov.as_u64() {
                Some(0xa5a5_a5a5) => assert_eq!(rust, None, "{label} {off:#x} must stay uninitialized"),
                Some(v) => assert_eq!(Some(v as u32), rust, "{label} actor{index} word {off:#x}"),
                None => panic!("{label} unexpected label at {off:#x}: {ov}"),
            },
        }
    }
    let comps: Vec<Value> = ALL_COMPONENTS
        .iter()
        .filter_map(|s| a.components.get(*s).map(|c| json!([s.index(), c.kind])))
        .collect();
    assert_eq!(json!(comps), oracle["components"], "{label} components");
}
use aa_actors::components::ALL_SLOTS as ALL_COMPONENTS;

fn run_case(row: &Value) {
    let label = row["label"].as_str().unwrap();
    let spec = &row["spec"];
    let r = &row["result"];
    assert!(r["outcome"].is_null(), "{label}: native outcome {}", r["outcome"]);
    assert!(r["unexpected"].as_array().unwrap().is_empty(), "{label}");
    let mut world = Fixture::new(spec);
    let mut params = params_from(spec);
    let p = pc(spec["pc"].as_str().unwrap());
    let entry = spec.get("entry").and_then(|v| v.as_str()).unwrap_or("create");
    let created = match entry {
        "create" => actor_creation::create(&mut params, &mut world, p),
        "actor" => actor_creation::create_gactor(&mut params, &mut world, p),
        "organization" => actor_creation::create_organization(&mut params, &mut world, p),
        _ => unreachable!(),
    }
    .unwrap_or_else(|e| panic!("{label}: {e}"));
    let returned = match created {
        Some(a) => json!(format!("actor{a}")),
        None => json!(0),
    };
    let expected = normalize(r["calls"].as_array().unwrap());
    let actual = world.calls.borrow().clone();
    for (i, (e, a)) in expected.iter().zip(actual.iter()).enumerate() {
        assert_eq!(e, a, "{label}: call {i}\n expected {expected:#?}\n actual {actual:#?}");
    }
    assert_eq!(expected.len(), actual.len(), "{label}: call count\n expected {expected:#?}\n actual {actual:#?}");
    assert_eq!(returned, r["returned"], "{label}: result");
    assert_eq!(world.rng.state, n(&r["rng_after"]), "{label}: rng");
    let idm = &r["idmgr"];
    // Fixture objects are written into the native table without its count.
    assert_eq!(world.registry.len() as u32 - world.extra, n(&idm["count"]), "{label} count");
    if !world.filled {
        // The native count alone (0x8000 live, no objects) is represented by
        // filler registrations above the fixture's slots.
        assert_eq!(world.registry.next_slot() as u32, n(&idm["cursor"]), "{label} cursor");
        assert_eq!(world.registry.highest_slot() as u32, n(&idm["highest"]), "{label} highest");
    }
    let actors = r["actors"].as_array().unwrap();
    assert_eq!(actors.len(), world.shells.len(), "{label}: actors");
    for (i, a) in actors.iter().enumerate() {
        compare_actor(label, i, a, &world);
    }
    let pa = u32s(&r["params_after"]);
    assert_eq!(&pa[2..7], &[params.position_bits[0], params.position_bits[1], params.position_bits[2],
                            params.direction_bits[0], params.direction_bits[1]], "{label}: params +8..+18");
    assert_eq!(pa[10] != 0, params.owner != 0, "{label}: params +28");
    let list: Vec<Value> = world.world_list.iter().map(|a| json!(format!("actor{a}"))).collect();
    assert_eq!(json!(list), r["world_list_8"], "{label}: world list");
    assert_eq!(json!(world.available), r["available_cities"], "{label}: available list");
    let names: Vec<Value> = world.names.iter().map(|(l, i)| json!([l, i])).collect();
    assert_eq!(json!(names), r["name_assignments"], "{label}: name assignments");
    for (a, v) in &world.latches {
        assert_eq!(u64::from(*v), r["latches"][format!("{a:#x}")].as_u64().unwrap(), "{label}: latch {a:#x}");
    }
}

#[test]
fn native_actor_creation_cases_replay() {
    let doc: Value = serde_json::from_str(FIXTURE).unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert!(cases.len() >= 70);
    for row in cases {
        run_case(row);
    }
}
