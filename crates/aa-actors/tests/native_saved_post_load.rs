//! Replays research/phase8-saved-components/native-post-load.json: the
//! original PostLoad pass 4FD3B3 (4EFBE0, 4EAA30, 4D3560, 4E4220, 4DDFA0,
//! virtual +12C and the component +30 virtuals) over synthetic GTID tables,
//! against aa_actors::post_load through a mock world that keeps the same
//! state and records the same seam calls (queue helpers, RNG, logs,
//! virtual +50, memory records, zones, clamps, CreateGThing).
use aa_actors::{
    actor_creation::ActorParams,
    components::{ALL_SLOTS, ComponentSlot},
    post_load::*,
};
use aa_numeric::Precision;
use serde_json::{Value, json};
use std::collections::BTreeMap;

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../research/phase8-saved-components/native-post-load.json"
);
const SLOT_NAMES: [&str; 31] = [
    "Body", "Projectile", "Character", "Mover", "Mind", "Performer", "Properties", "Organization", "Element",
    "Economy", "Denizen", "Supply", "Wall", "Structure", "Settlement", "Construction", "Recruit", "Experience",
    "Builder", "MobileDeployment", "Defender", "Research", "Detection", "Aircraft", "Transport", "Lair", "Flare",
    "MetaGame", "Portal", "City", "Hq",
];
fn slot_name(slot: ComponentSlot) -> &'static str {
    SLOT_NAMES[slot as usize]
}
fn slot_of(name: &str) -> ComponentSlot {
    ALL_SLOTS[SLOT_NAMES.iter().position(|n| *n == name).unwrap()]
}
const LIST_KEYS: [&str; 3] = ["0x118", "0x120", "0x128"];
fn f32_bits(v: f32) -> u32 {
    v.to_bits()
}

#[derive(Default)]
struct Comp {
    scheduled: Vec<[u32; 2]>,
    element_org: Option<usize>,
    mind_command: Option<String>,
    commands: Vec<Option<String>>,
    word_1c: u32,
    ability_targets: Vec<u32>,
    entries: Vec<[u32; 2]>,
    hq_index: i32,
    hq_items: usize,
    settlement_count: i32,
    settlement_things: Vec<u32>,
}
struct Obj {
    label: String,
    id: u32,
    class: PostLoadClass,
    report_flags: bool,
    flags8: u32,
    required: u32,
    name: String,
    lists: [Vec<u32>; 3],
    word_16c: u32,
    e8: Option<usize>,
    def_1c: u32,
    def_34: bool,
    def_2a8: u32,
    def_42c: bool,
    pos: [u32; 3],
    memory_count: i32,
    memory: BTreeMap<u32, String>,
    zones: i32,
    settlement_entries: Vec<[u32; 5]>,
    settlement_defs: Vec<Option<String>>,
    target: Option<GroupTarget>,
    radius_1b4: u32,
    comps: BTreeMap<ComponentSlot, Comp>,
    groups: Vec<Vec<u32>>,
    items: Vec<usize>,
    counter_224: u32,
    word_190: u32,
    index_18c: u32,
    hq_slots: Option<i32>,
    hq_taken: Vec<i32>,
}
struct Item {
    label: String,
    name: String,
    available: u8,
    organization: u32,
}
struct Command {
    refcount: u32,
    members: Vec<u32>,
}
struct World {
    objs: Vec<Obj>,
    table: BTreeMap<u32, (u16, Option<usize>)>,
    max_slot: i32,
    world_version: u32,
    unit_supply: bool,
    random: Vec<u32>,
    items: Vec<Item>,
    commands: BTreeMap<String, Command>,
    world_kingdoms: Vec<Option<usize>>,
    ui_agent: Option<u8>,
    created: Vec<Option<u32>>,
    clamp_max: u32,
    tile_size: u32,
    manager_74: u32,
    pc: Precision,
    calls: Vec<Value>,
}
fn err(s: &str) -> Error {
    Error::World(s.into())
}
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn units(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}
fn clamp(v: u32, top: u32) -> u32 {
    let (f, t) = (f32::from_bits(v), f32::from_bits(top));
    if f < 0.0 {
        0
    } else if f > t {
        top
    } else {
        v
    }
}

impl World {
    fn new(case: &Value) -> Self {
        let class = |c: &str| match c {
            "Thing" | "Team" => PostLoadClass::Inert,
            "Object" => PostLoadClass::Object,
            "Actor" => PostLoadClass::Actor,
            "Kingdom" => PostLoadClass::Kingdom,
            "Group" => PostLoadClass::Group,
            c => panic!("{c}"),
        };
        let specs = case["objects"].as_array().unwrap();
        let index_of = |label: &str| specs.iter().position(|o| o["label"] == label).unwrap();
        let mut table = BTreeMap::new();
        for s in case.get("stale").and_then(Value::as_array).into_iter().flatten() {
            table.insert(u(&s[0]), (u(&s[1]) as u16, None));
        }
        let mut items = Vec::new();
        let mut commands = BTreeMap::new();
        let mut objs = Vec::new();
        for (i, o) in specs.iter().enumerate() {
            let slot = u(&o["slot"]);
            let generation = u(&o["gen"]);
            table.insert(slot, (generation as u16, Some(i)));
            let get = |k: &str| o.get(k).filter(|v| !v.is_null());
            let lists = std::array::from_fn(|n| {
                get("lists")
                    .and_then(|l| l.get(LIST_KEYS[n]))
                    .map(|v| v.as_array().unwrap().iter().map(u).collect())
                    .unwrap_or_default()
            });
            let mut comps = BTreeMap::new();
            for (name, spec) in get("components").and_then(Value::as_object).into_iter().flatten() {
                let count = spec.get("count").map(|v| v.as_i64().unwrap() as i32).unwrap_or(0);
                let mut c = Comp {
                    scheduled: spec
                        .get("scheduled")
                        .map(|v| v.as_array().unwrap().iter().map(|e| [u(&e[0]), u(&e[1])]).collect())
                        .unwrap_or_default(),
                    element_org: spec.get("organization").and_then(Value::as_str).map(index_of),
                    word_1c: spec.get("word_1c").map(u).unwrap_or(0),
                    ability_targets: spec
                        .get("abilities")
                        .map(|v| v.as_array().unwrap().iter().map(u).collect())
                        .unwrap_or_default(),
                    entries: spec
                        .get("entries")
                        .map(|v| v.as_array().unwrap().iter().map(|e| [u(&e[0]), u(&e[1])]).collect())
                        .unwrap_or_default(),
                    hq_index: spec.get("index").map(|v| v.as_i64().unwrap() as i32).unwrap_or(0),
                    hq_items: spec.get("items").map(|v| u(v) as usize).unwrap_or(0),
                    settlement_count: count,
                    settlement_things: vec![0; count.max(0) as usize],
                    ..Default::default()
                };
                for cmd in spec.get("commands").and_then(Value::as_array).into_iter().flatten() {
                    if cmd.is_null() {
                        c.commands.push(None);
                        continue;
                    }
                    let label = cmd["label"].as_str().unwrap().to_owned();
                    commands.insert(
                        label.clone(),
                        Command {
                            refcount: u(&cmd["refcount"]),
                            members: cmd["members"].as_array().unwrap().iter().map(u).collect(),
                        },
                    );
                    c.commands.push(Some(label));
                }
                comps.insert(slot_of(name), c);
            }
            let mut item_indices = Vec::new();
            for it in get("recruit_items").and_then(Value::as_array).into_iter().flatten() {
                item_indices.push(items.len());
                items.push(Item {
                    label: it["label"].as_str().unwrap().into(),
                    name: it["name"].as_str().unwrap().into(),
                    available: it.get("byte_8").map(u).unwrap_or(0) as u8,
                    organization: it.get("organization").map(u).unwrap_or(0),
                });
            }
            let pos = get("pos")
                .map(|v| {
                    let a = v.as_array().unwrap();
                    [u(&a[0]), u(&a[1]), u(&a[2])]
                })
                .unwrap_or([f32_bits(100.0), f32_bits(200.0), f32_bits(5.0)]);
            objs.push(Obj {
                label: o["label"].as_str().unwrap().into(),
                id: (generation << 16) | slot,
                class: class(o["class"].as_str().unwrap()),
                report_flags: o.get("required").is_some() || o.get("flags8").is_some(),
                flags8: get("flags8").map(u).unwrap_or(0),
                required: get("required").map(u).unwrap_or(0),
                name: get("name").and_then(Value::as_str).unwrap_or(o["label"].as_str().unwrap()).into(),
                lists,
                word_16c: get("word_16c").map(u).unwrap_or(0),
                e8: get("e8").and_then(Value::as_str).map(index_of),
                def_1c: get("def_1c").map(u).unwrap_or(0),
                def_34: get("def_34").map(u).unwrap_or(0) != 0,
                def_2a8: get("def_2a8").map(u).unwrap_or(0),
                def_42c: get("def_42c").map(u).unwrap_or(0) != 0,
                pos,
                memory_count: get("memory_count").map(|v| v.as_i64().unwrap() as i32).unwrap_or(0),
                memory: get("memory")
                    .and_then(Value::as_object)
                    .into_iter()
                    .flatten()
                    .map(|(k, v)| (k.parse().unwrap(), v.as_str().unwrap().to_owned()))
                    .collect(),
                zones: get("zones").map(|v| v.as_i64().unwrap() as i32).unwrap_or(0),
                settlement_entries: get("settlement_entries")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .map(|e| std::array::from_fn(|j| u(&e[j])))
                    .collect(),
                settlement_defs: get("settlement_defs")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .map(|v| v.as_str().map(str::to_owned))
                    .collect(),
                target: get("target").map(|t| GroupTarget {
                    flags: u(&t["flags"]),
                    object: u(&t["object"]),
                    position_bits: [u(&t["xy"][0]), u(&t["xy"][1])],
                }),
                radius_1b4: get("radius_1b4").map(u).unwrap_or(f32_bits(48.0)),
                comps,
                groups: get("groups")
                    .map(|g| g.as_array().unwrap().iter().map(|m| m.as_array().unwrap().iter().map(u).collect()).collect())
                    .unwrap_or_default(),
                items: item_indices,
                counter_224: get("counter_224").map(u).unwrap_or(0),
                word_190: get("word_190").map(u).unwrap_or(0),
                index_18c: get("index_18c").map(u).unwrap_or(0),
                hq_slots: get("hq_slots").map(|v| v.as_i64().unwrap() as i32),
                hq_taken: get("hq_taken")
                    .map(|v| v.as_array().unwrap().iter().map(|x| x.as_i64().unwrap() as i32).collect())
                    .unwrap_or_default(),
            });
        }
        let pc = match case.get("precision").and_then(Value::as_str) {
            Some("Single24") => Precision::Single24,
            Some("Extended64") => Precision::Extended64,
            _ => Precision::Double53,
        };
        Self {
            objs,
            table,
            max_slot: case["max_slot"].as_i64().unwrap() as i32,
            world_version: case.get("world_version").map(u).unwrap_or(27),
            unit_supply: case.get("unit_supply").map(u).unwrap_or(1) != 0,
            random: case.get("random").map(|v| v.as_array().unwrap().iter().map(u).collect()).unwrap_or_default(),
            items,
            commands,
            world_kingdoms: case
                .get("world_kingdoms")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(|v| v.as_str().map(index_of))
                .collect(),
            ui_agent: case.get("ui_agent").and_then(Value::as_u64).map(|v| v as u8),
            created: case
                .get("created")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(|v| v.as_u64().map(|x| x as u32))
                .collect(),
            clamp_max: case.get("clamp_max").map(u).unwrap_or(f32_bits(1000.0)),
            tile_size: case.get("tile_size").map(u).unwrap_or(f32_bits(32.0)),
            manager_74: case.get("manager_74").map(u).unwrap_or(f32_bits(64.0)),
            pc,
            calls: Vec::new(),
        }
    }
    fn index(&self, id: Id) -> Result<usize> {
        match self.table.get(&(id & 0xffff)) {
            Some((generation, Some(i))) if u32::from(*generation) == id >> 16 => Ok(*i),
            _ => Err(err("not live")),
        }
    }
    fn obj(&self, id: Id) -> Result<&Obj> {
        Ok(&self.objs[self.index(id)?])
    }
    fn obj_mut(&mut self, id: Id) -> Result<&mut Obj> {
        let i = self.index(id)?;
        Ok(&mut self.objs[i])
    }
    fn comp(&self, at: Updatable) -> Result<&Comp> {
        self.obj(at.actor)?.comps.get(&at.slot).ok_or_else(|| err("component"))
    }
    fn comp_mut(&mut self, actor: Id, slot: ComponentSlot) -> Result<&mut Comp> {
        self.obj_mut(actor)?.comps.get_mut(&slot).ok_or_else(|| err("component"))
    }
    fn owner(&self, at: Updatable) -> Result<(String, &'static str)> {
        Ok((self.obj(at.actor)?.label.clone(), slot_name(at.slot)))
    }
    fn label_of_id(&self, id: Id) -> String {
        self.obj(id).map(|o| o.label.clone()).unwrap()
    }
    fn snapshot(&self) -> Value {
        let mut out = serde_json::Map::new();
        for o in &self.objs {
            let mut s = serde_json::Map::new();
            if o.report_flags {
                s.insert("flags8".into(), json!(o.flags8));
                if o.class != PostLoadClass::Inert {
                    s.insert("required".into(), json!(o.required));
                }
            }
            if let Some(t) = &o.target {
                s.insert("target".into(), json!({"flags": t.flags, "xy": t.position_bits}));
            }
            if o.class.is_actor() {
                s.insert(
                    "lists".into(),
                    json!({"0x118": o.lists[0], "0x120": o.lists[1], "0x128": o.lists[2]}),
                );
                let mut comps = serde_json::Map::new();
                for (slot, c) in &o.comps {
                    let mut cs = serde_json::Map::new();
                    cs.insert("scheduled".into(), json!(c.scheduled));
                    match slot {
                        ComponentSlot::Mind => {
                            cs.insert("command".into(), json!(c.mind_command));
                        }
                        ComponentSlot::Defender => {
                            cs.insert("ability_targets".into(), json!(c.ability_targets));
                            cs.insert("entries".into(), json!(c.entries));
                        }
                        ComponentSlot::Settlement => {
                            cs.insert("things".into(), json!(c.settlement_things));
                        }
                        ComponentSlot::Hq => {
                            cs.insert("index".into(), json!(c.hq_index));
                            cs.insert("items".into(), json!(c.hq_items));
                        }
                        _ => {}
                    }
                    comps.insert(slot_name(*slot).into(), Value::Object(cs));
                }
                s.insert("components".into(), Value::Object(comps));
            }
            if o.class == PostLoadClass::Kingdom {
                s.insert("groups".into(), json!(o.groups));
                s.insert(
                    "recruit_items".into(),
                    json!(o.items.iter().map(|i| json!({"label": self.items[*i].label,
                        "available": self.items[*i].available, "organization": self.items[*i].organization})).collect::<Vec<_>>()),
                );
                s.insert("counter_224".into(), json!(o.counter_224));
                s.insert("hq_taken".into(), json!(o.hq_taken));
            }
            out.insert(o.label.clone(), Value::Object(s));
        }
        out.insert(
            "commands".into(),
            Value::Object(self.commands.iter().map(|(k, c)| (k.clone(), json!({"refcount": c.refcount}))).collect()),
        );
        Value::Object(out)
    }
}

impl PostLoadWorld for World {
    type Command = String;
    type RecruitItem = usize;
    type Definition = String;
    type Layout = ();
    fn highest_slot(&self) -> Result<i32> {
        Ok(self.max_slot)
    }
    fn object_in_slot(&self, slot: u32) -> Result<Option<(Id, PostLoadClass)>> {
        assert!(slot <= 0xffff);
        Ok(match self.table.get(&slot) {
            Some((_, Some(i))) => Some((self.objs[*i].id, self.objs[*i].class)),
            _ => None,
        })
    }
    fn live(&self, id: Id) -> Result<bool> {
        Ok(self.index(id).is_ok())
    }
    fn object_class(&self, id: Id) -> Result<PostLoadClass> {
        Ok(self.obj(id)?.class)
    }
    fn object_flags(&self, id: Id) -> Result<u32> {
        Ok(self.obj(id)?.flags8)
    }
    fn required_object(&self, object: Id) -> Result<Id> {
        Ok(self.obj(object)?.required)
    }
    fn definition_name(&self, object: Id) -> Result<Vec<u16>> {
        Ok(units(&self.obj(object)?.name))
    }
    fn log_required_not_loaded(&mut self, e: RequiredObjectLog) -> Result<()> {
        self.calls.push(json!({"seam": "404ED0", "line": POST_LOAD_LINE, "function": POST_LOAD_FUNCTION,
            "format": REQUIRED_NOT_LOADED, "args": [e.required as i32, e.object as i32, String::from_utf16(&e.name).unwrap()]}));
        Ok(())
    }
    fn clear_required_object(&mut self, object: Id) -> Result<()> {
        self.obj_mut(object)?.required = 0;
        Ok(())
    }
    fn definition_flags_1c(&self, object: Id) -> Result<u32> {
        Ok(self.obj(object)?.def_1c)
    }
    fn set_blocking(&mut self, object: Id, blocking: bool) -> Result<()> {
        let label = self.label_of_id(object);
        self.calls.push(json!({"seam": "set_blocking", "object": label, "flag": u8::from(blocking)}));
        Ok(())
    }
    fn set_object_flags(&mut self, object: Id, flags: u32) -> Result<()> {
        self.obj_mut(object)?.flags8 = flags;
        Ok(())
    }
    fn world_kingdoms(&self) -> Result<Vec<Option<Id>>> {
        Ok(self.world_kingdoms.iter().map(|k| k.map(|i| self.objs[i].id)).collect())
    }
    fn kingdom_word_190(&self, kingdom: Id) -> Result<u32> {
        Ok(self.obj(kingdom)?.word_190)
    }
    fn kingdom_index(&self, kingdom: Id) -> Result<u32> {
        Ok(self.obj(kingdom)?.index_18c)
    }
    fn memory_count(&self, object: Id) -> Result<i32> {
        Ok(self.obj(object)?.memory_count)
    }
    fn memory_record(&self, object: Id, index: u32) -> Result<Option<u32>> {
        // Handle = object index << 8 | kingdom index (decoded for the log).
        let i = self.index(object)? as u32;
        Ok(self.obj(object)?.memory.get(&index).map(|_| (i << 8) | index))
    }
    fn ui_agent_active(&self) -> Result<bool> {
        Ok(self.ui_agent.is_some_and(|b| b != 0))
    }
    fn ui_memory_record(&mut self, record: Option<u32>, kingdom: Id, after: bool) -> Result<()> {
        let label = record.map(|r| self.record_label(r));
        let k = self.label_of_id(kingdom);
        self.calls.push(json!({"seam": "ui_memory", "after": after, "record": label, "kingdom": k}));
        Ok(())
    }
    fn refresh_memory_record(&mut self, object: Id, record: u32) -> Result<()> {
        let label = self.record_label(record);
        let ol = self.label_of_id(object);
        self.calls.push(json!({"seam": "refresh_memory_record", "object": ol, "record": label}));
        Ok(())
    }
    fn settlement_space(&self, object: Id) -> Result<bool> {
        Ok(self.obj(object)?.def_34)
    }
    fn tile_size_bits(&self) -> Result<u32> {
        Ok(self.tile_size)
    }
    fn thing_manager_74_bits(&self) -> Result<u32> {
        Ok(self.manager_74)
    }
    fn object_position(&self, object: Id) -> Result<[u32; 2]> {
        let p = self.obj(object)?.pos;
        Ok([p[0], p[1]])
    }
    fn zone_count(&self, actor: Id) -> Result<i32> {
        Ok(self.obj(actor)?.zones)
    }
    fn move_zone(&mut self, actor: Id, index: u32, xy: [u32; 2], radius_bits: u32) -> Result<()> {
        let label = self.label_of_id(actor);
        self.calls.push(json!({"seam": "515F40", "object": label, "zone": index, "x": xy[0], "y": xy[1], "radius": radius_bits}));
        Ok(())
    }
    fn precision(&self) -> Precision {
        self.pc
    }
    fn has_component(&self, actor: Id, slot: ComponentSlot) -> Result<bool> {
        Ok(self.obj(actor)?.comps.contains_key(&slot))
    }
    fn actor_list(&self, actor: Id, list: ActorList) -> Result<Vec<Id>> {
        Ok(self.obj(actor)?.lists[list as usize].clone())
    }
    fn remove_actor_list_node(&mut self, actor: Id, list: ActorList, index: usize) -> Result<()> {
        self.obj_mut(actor)?.lists[list as usize].remove(index);
        Ok(())
    }
    fn actor_kingdom(&self, actor: Id) -> Result<Option<Id>> {
        Ok(self.obj(actor)?.e8.map(|i| self.objs[i].id))
    }
    fn element_organization(&self, actor: Id) -> Result<Option<Id>> {
        Ok(self
            .obj(actor)?
            .comps
            .get(&ComponentSlot::Element)
            .and_then(|c| c.element_org)
            .map(|i| self.objs[i].id))
    }
    fn has_scheduled(&mut self, at: Updatable, queue: u8, data: u32) -> Result<bool> {
        let r = self.comp(at)?.scheduled.contains(&[u32::from(queue), data]);
        let (object, slot) = self.owner(at)?;
        self.calls.push(json!({"seam": "50B220", "object": object, "slot": slot, "queue": queue, "data": data, "result": r}));
        Ok(r)
    }
    fn has_any_scheduled(&self, at: Updatable) -> Result<bool> {
        Ok(!self.comp(at)?.scheduled.is_empty())
    }
    fn add_periodic(&mut self, at: Updatable, queue: u8, data: u32) -> Result<()> {
        self.comp_mut(at.actor, at.slot)?.scheduled.insert(0, [u32::from(queue), data]);
        let (object, slot) = self.owner(at)?;
        let seam = match queue {
            0 => "50AC00",
            1 => "50AC60",
            _ => panic!("queue"),
        };
        self.calls.push(json!({"seam": seam, "object": object, "slot": slot, "queue": queue, "data": data}));
        Ok(())
    }
    fn add_after(&mut self, at: Updatable, queue: u8, data: u32, delay_bits: u32) -> Result<()> {
        self.comp_mut(at.actor, at.slot)?.scheduled.insert(0, [u32::from(queue), data]);
        let (object, slot) = self.owner(at)?;
        self.calls.push(json!({"seam": "50ACC0", "object": object, "slot": slot, "queue": queue, "data": data, "delay": delay_bits}));
        Ok(())
    }
    fn remove_scheduled(&mut self, at: Updatable, queue: u8, data: u32) -> Result<u32> {
        let c = self.comp_mut(at.actor, at.slot)?;
        let before = c.scheduled.len();
        c.scheduled.retain(|e| *e != [u32::from(queue), data]);
        let n = (before - c.scheduled.len()) as u32;
        let (object, slot) = self.owner(at)?;
        self.calls.push(json!({"seam": "50B1C0", "object": object, "slot": slot, "queue": queue, "data": data, "removed": n}));
        Ok(n)
    }
    fn cancel_scheduled(&mut self, at: Updatable) -> Result<()> {
        self.comp_mut(at.actor, at.slot)?.scheduled.clear();
        let (object, slot) = self.owner(at)?;
        self.calls.push(json!({"seam": "50B250", "object": object, "slot": slot}));
        Ok(())
    }
    fn random(&mut self) -> Result<u32> {
        if self.random.is_empty() {
            return Err(err("random script exhausted"));
        }
        let v = self.random.remove(0);
        self.calls.push(json!({"seam": "random", "value": v}));
        Ok(v)
    }
    fn unit_supply_enabled(&self) -> Result<bool> {
        Ok(self.unit_supply)
    }
    fn body_shortfall_rate_bits(&self, actor: Id) -> Result<u32> {
        Ok(self.obj(actor)?.def_2a8)
    }
    fn word_16c(&self, actor: Id) -> Result<Id> {
        Ok(self.obj(actor)?.word_16c)
    }
    fn world_version(&self) -> Result<u32> {
        Ok(self.world_version)
    }
    fn wall_flag_42c(&self, actor: Id) -> Result<bool> {
        Ok(self.obj(actor)?.def_42c)
    }
    fn denizen_commands(&self, actor: Id) -> Result<Vec<Option<String>>> {
        Ok(self.comp(Updatable { actor, slot: ComponentSlot::Denizen })?.commands.clone())
    }
    fn command_members(&self, command: &String) -> Result<Vec<Id>> {
        Ok(self.commands[command].members.clone())
    }
    fn set_mind_command(&mut self, member: Id, command: &String) -> Result<()> {
        let old = self.comp(Updatable { actor: member, slot: ComponentSlot::Mind })?.mind_command.clone();
        if old.as_ref() == Some(command) {
            return Ok(());
        }
        self.commands.get_mut(command).unwrap().refcount += 1;
        if let Some(old) = old {
            let c = self.commands.get_mut(&old).unwrap();
            c.refcount -= 1;
            if c.refcount == 0 {
                self.calls.push(json!({"seam": "command_delete", "command": old}));
            }
        }
        self.comp_mut(member, ComponentSlot::Mind)?.mind_command = Some(command.clone());
        Ok(())
    }
    fn defender_entries(&self, actor: Id) -> Result<Vec<[Id; 2]>> {
        Ok(self.comp(Updatable { actor, slot: ComponentSlot::Defender })?.entries.clone())
    }
    fn set_defender_entry_value(&mut self, actor: Id, index: usize, value: Id) -> Result<()> {
        self.comp_mut(actor, ComponentSlot::Defender)?.entries[index][1] = value;
        Ok(())
    }
    fn remove_defender_entry(&mut self, actor: Id, index: usize) -> Result<()> {
        self.comp_mut(actor, ComponentSlot::Defender)?.entries.remove(index);
        Ok(())
    }
    fn defender_word_1c(&self, actor: Id) -> Result<u32> {
        Ok(self.comp(Updatable { actor, slot: ComponentSlot::Defender })?.word_1c)
    }
    fn defender_ability_count(&self, actor: Id) -> Result<usize> {
        Ok(self.comp(Updatable { actor, slot: ComponentSlot::Defender })?.ability_targets.len())
    }
    fn ability_target(&self, actor: Id, index: usize) -> Result<Id> {
        Ok(self.comp(Updatable { actor, slot: ComponentSlot::Defender })?.ability_targets[index])
    }
    fn set_ability_target(&mut self, actor: Id, index: usize, target: Id) -> Result<()> {
        self.comp_mut(actor, ComponentSlot::Defender)?.ability_targets[index] = target;
        Ok(())
    }
    fn hq_release_items(&mut self, actor: Id) -> Result<()> {
        self.comp_mut(actor, ComponentSlot::Hq)?.hq_items = 0;
        Ok(())
    }
    fn hq_index(&self, actor: Id) -> Result<i32> {
        Ok(self.comp(Updatable { actor, slot: ComponentSlot::Hq })?.hq_index)
    }
    fn set_hq_index(&mut self, actor: Id, index: i32) -> Result<()> {
        self.comp_mut(actor, ComponentSlot::Hq)?.hq_index = index;
        Ok(())
    }
    fn kingdom_hq_slots(&self, kingdom: Id) -> Result<Option<i32>> {
        Ok(self.obj(kingdom)?.hq_slots)
    }
    fn kingdom_hq_taken(&self, kingdom: Id) -> Result<Vec<i32>> {
        Ok(self.obj(kingdom)?.hq_taken.clone())
    }
    fn push_kingdom_hq_taken(&mut self, kingdom: Id, index: i32) -> Result<()> {
        self.obj_mut(kingdom)?.hq_taken.insert(0, index);
        Ok(())
    }
    fn kingdom_group_count(&self, kingdom: Id) -> Result<i32> {
        Ok(self.obj(kingdom)?.groups.len() as i32)
    }
    fn kingdom_group_members(&self, kingdom: Id, group: usize) -> Result<Vec<Id>> {
        Ok(self.obj(kingdom)?.groups[group].clone())
    }
    fn remove_kingdom_group_member(&mut self, kingdom: Id, group: usize, index: usize) -> Result<()> {
        self.obj_mut(kingdom)?.groups[group].remove(index);
        Ok(())
    }
    fn remove_kingdom_group(&mut self, kingdom: Id, group: usize) -> Result<()> {
        let g = self.obj_mut(kingdom)?.groups.remove(group);
        assert!(g.is_empty());
        Ok(())
    }
    fn recruit_item_count(&self, kingdom: Id) -> Result<i32> {
        Ok(self.obj(kingdom)?.items.len() as i32)
    }
    fn recruit_item_at(&self, kingdom: Id, index: usize) -> Result<Option<usize>> {
        Ok(self.obj(kingdom)?.items.get(index).copied())
    }
    fn recruit_item_byte_8(&self, item: &usize) -> Result<u8> {
        Ok(self.items[*item].available)
    }
    fn recruit_item_organization(&self, item: &usize) -> Result<Id> {
        Ok(self.items[*item].organization)
    }
    fn set_recruit_item_organization(&mut self, item: &usize, organization: Id) -> Result<()> {
        self.items[*item].organization = organization;
        Ok(())
    }
    fn recruit_item_name(&self, item: &usize) -> Result<Vec<u16>> {
        Ok(units(&self.items[*item].name))
    }
    fn set_recruit_item_available(&mut self, item: &usize) -> Result<()> {
        self.items[*item].available = 1;
        Ok(())
    }
    fn increment_kingdom_224(&mut self, kingdom: Id) -> Result<()> {
        self.obj_mut(kingdom)?.counter_224 += 1;
        Ok(())
    }
    fn map_warning(&mut self, format: &'static str, name: Vec<u16>) -> Result<()> {
        self.calls.push(json!({"seam": "5A1040", "format": format, "args": [String::from_utf16(&name).unwrap()]}));
        Ok(())
    }
    fn settlement_count(&self, actor: Id) -> Result<i32> {
        Ok(self.comp(Updatable { actor, slot: ComponentSlot::Settlement })?.settlement_count)
    }
    fn settlement_entry(&self, actor: Id, index: usize) -> Result<SettlementEntry> {
        let e = self.obj(actor)?.settlement_entries[index];
        Ok(SettlementEntry {
            kind: e[0],
            offset_bits: [e[1], e[2]],
            direction_bits: [e[3], e[4]],
        })
    }
    fn settlement_definition(&self, actor: Id, kind: u32) -> Result<Option<String>> {
        Ok(self.obj(actor)?.settlement_defs.get(kind as usize).cloned().flatten().filter(|_| kind <= 2))
    }
    fn object_z(&self, object: Id) -> Result<u32> {
        Ok(self.obj(object)?.pos[2])
    }
    fn create_gthing(&mut self, p: ActorParams<String, ()>) -> Result<Option<Id>> {
        let owner = (p.owner != 0).then(|| self.label_of_id(p.owner));
        let r = self.created.remove(0);
        self.calls.push(json!({"seam": "4F9C10", "block": {"vtable": 0xa25b28u32,
            "definition": format!("def:{}", p.definition), "position": p.position_bits,
            "direction": p.direction_bits, "name_length": p.name_utf16.len(), "variant": p.variant,
            "requested_id": p.requested_id, "owner": owner,
            "words_2c_4c": [u32::from(p.organization_layout.is_some()), p.word_30, p.word_34, p.mode,
                p.word_3c, p.word_40, p.word_44, p.word_48, p.word_4c],
            "bytes_50": [p.byte_50, p.byte_51, p.byte_52]}, "result": r}));
        Ok(r)
    }
    fn set_settlement_thing(&mut self, actor: Id, index: usize, id: Id) -> Result<()> {
        self.comp_mut(actor, ComponentSlot::Settlement)?.settlement_things[index] = id;
        Ok(())
    }
    fn group_target(&self, group: Id) -> Result<Option<GroupTarget>> {
        Ok(self.obj(group)?.target)
    }
    fn set_group_target(&mut self, group: Id, target: GroupTarget) -> Result<()> {
        self.obj_mut(group)?.target = Some(target);
        Ok(())
    }
    fn group_radius_bits(&self, group: Id) -> Result<u32> {
        Ok(self.obj(group)?.radius_1b4)
    }
    fn clamp_position(&mut self, xy: [u32; 2]) -> Result<[u32; 2]> {
        let out = [clamp(xy[0], self.clamp_max), clamp(xy[1], self.clamp_max)];
        self.calls.push(json!({"seam": "552640", "xy": xy, "out": out}));
        Ok(out)
    }
}

impl World {
    fn record_label(&self, record: u32) -> String {
        self.objs[(record >> 8) as usize].memory[&(record & 0xff)].clone()
    }
}

#[test]
fn post_load_pass_matches_original() {
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(FIXTURE).unwrap()).unwrap();
    assert_eq!(doc["passed"], true);
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 42);
    for case in cases {
        let name = case["input"]["name"].as_str().unwrap();
        let native = &case["native"];
        let mut world = World::new(&case["input"]);
        let result = post_load_pass(&mut world);
        match (&native["outcome"], &result) {
            (Value::Null, Ok(())) => {}
            (o, Err(Error::NullDereference(_))) if o["kind"] == "NullDereference" => {}
            (o, Err(Error::Undefined(_))) if o["kind"] == "Undefined" => {}
            (o, r) => panic!("{name}: native {o} rust {r:?}"),
        }
        assert_eq!(Value::Array(world.calls.clone()), native["calls"], "{name}: calls");
        assert_eq!(world.snapshot(), native["state"], "{name}: state");
    }
}

/// Native undefined behaviour stops with an explicit error before any
/// effect of its own.
#[test]
fn undefined_post_loads_are_explicit_errors() {
    // A Defender entry whose live value is not a GActor: 4D0CD0 would edit
    // +118 of a 0x60-byte GObject.
    let case = json!({"name": "y", "max_slot": 2, "objects": [
        {"label": "d", "class": "Actor", "slot": 1, "gen": 1,
         "components": {"Defender": {"word_1c": 1, "entries": [[9, 0x10002]]}}},
        {"label": "o", "class": "Object", "slot": 2, "gen": 1}]});
    let mut world = World::new(&case);
    assert!(matches!(post_load_pass(&mut world), Err(Error::Undefined(_))));
    // Virtual +12C on a GThing class.
    let case = json!({"name": "z", "max_slot": 1, "objects": [
        {"label": "t", "class": "Thing", "slot": 1, "gen": 1}]});
    let mut world = World::new(&case);
    assert!(matches!(required_object_lost(&mut world, 0x10001), Err(Error::Undefined(_))));
}

/// 50B220 run natively on hand-built entries: the walk follows the head
/// entry's queue-bucket successors (+8), across owners.
#[test]
fn updatable_has_walks_the_bucket_chain_from_the_head_entry() {
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(FIXTURE).unwrap()).unwrap();
    let rows = doc["updatable_has_50b220"].as_array().unwrap();
    assert_eq!(rows.len(), 8);
    for row in rows {
        let name = row["name"].as_str().unwrap();
        let who = row["updatable"].as_str().unwrap();
        let head = row["owners"][who].as_array().unwrap().first().map(|v| v.as_str().unwrap().to_owned());
        let mut chain = Vec::new();
        if let Some(head) = head {
            let bucket = row["buckets"]
                .as_array()
                .unwrap()
                .iter()
                .map(|b| b.as_array().unwrap().iter().map(|k| k.as_str().unwrap()).collect::<Vec<_>>())
                .find(|b| b.contains(&head.as_str()))
                .unwrap();
            let at = bucket.iter().position(|k| *k == head).unwrap();
            for key in &bucket[at..] {
                let e = &row["entries"][*key];
                chain.push((u(&e["queue"]) as u8, u(&e["data"])));
            }
        }
        let r = native_has_scheduled(chain, u(&row["queue"]) as u8, u(&row["data"]));
        assert_eq!(r, row["native_result"].as_bool().unwrap(), "{name}");
    }
}
