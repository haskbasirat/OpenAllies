//! Generic world mirroring research/phase8-triggers/harness.py.  Seams record
//! the same JSON entries as the native harness; return values come from the
//! case's `returns` table.  Registry, name tables, objective lists and Kingdom
//! fields are plain fixture data.
#![allow(dead_code)]
use aa_navigation::startup::Object;
use aa_numeric::Precision;
use aa_triggers::{
    actions,
    decode::{Element, Parameter, SavedManager, Span, Trigger, Value as PV, Variable},
    elements::{self, Bank},
    extended,
    manager::{DeferredPayload, Error, Manager, Result},
    objectives::{self, Entry, ObjectiveDef},
    predicates::Event,
    program::CoreProgram,
    selectors::{Checks, Registry},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub fn span() -> Span {
    Span {
        offset: 0,
        length: 0,
    }
}
pub fn u(v: &Value) -> u32 {
    v.as_u64().unwrap_or_else(|| panic!("expected integer, got {v}")) as u32
}
pub fn utf16(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}
pub fn string(v: &[u16]) -> String {
    String::from_utf16(v).unwrap()
}

pub fn parameter(p: &Value) -> Parameter {
    let storage = u(&p["storage"]);
    Parameter {
        source: span(),
        argument: p["argument"].as_str().unwrap().into(),
        item: p["item"].as_str().unwrap().into(),
        item_index: u(&p["item_index"]),
        storage,
        flag: u(&p["flag"]) as u8,
        value: match storage {
            0 => PV::Empty,
            1 | 2 => PV::Word(u(&p["value"])),
            5 => {
                let v = p["value"].as_array().unwrap();
                PV::Position([u(&v[0]), u(&v[1]), u(&v[2])])
            }
            6 => PV::Text(utf16(p["value"].as_str().unwrap())),
            7 => PV::Localized {
                key: utf16(p["value"]["key"].as_str().unwrap()),
                inline: p["value"]["inline"].as_str().map(utf16),
            },
            other => panic!("storage {other}"),
        },
    }
}
pub fn element(e: &Value) -> Element {
    Element {
        source: span(),
        name: e["name"].as_str().unwrap().into(),
        opcode: u(&e["opcode"]),
        word_10: 0,
        parameters: e["parameters"].as_array().unwrap().iter().map(parameter).collect(),
    }
}

pub struct FixtureWorld {
    pub c: Value,
    pub names: Vec<String>,
    pub index: BTreeMap<String, Object>,
    pub registry: BTreeMap<u32, Object>,
    pub gtid: BTreeMap<u64, u32>,
    pub returns: BTreeMap<String, Value>,
    pub calls: Vec<Value>,
    pub defs: Vec<ObjectiveDef>,
    pub kingdom_objectives: BTreeMap<u64, Vec<Entry>>,
    pub results: BTreeMap<u64, u32>,
    pub teams: BTreeMap<u64, Option<usize>>,
    pub flag239: BTreeMap<u64, u8>,
    pub weather: Option<Object>,
    pub flags8_writes: BTreeMap<u64, u32>,
}

pub trait World: elements::Services + Sized {
    fn new(c: &Value) -> Self;
    fn base(&self) -> &FixtureWorld;
    fn base_mut(&mut self) -> &mut FixtureWorld;
}

impl FixtureWorld {
    pub fn build(c: &Value) -> Self {
        let mut names = vec![];
        let mut index = BTreeMap::new();
        let mut registry = BTreeMap::new();
        let mut gtid = BTreeMap::new();
        if let Some(objects) = c["objects"].as_object() {
            for (i, (name, spec)) in objects.iter().enumerate() {
                let object = Object(i as u64 + 1);
                names.push(name.clone());
                index.insert(name.clone(), object);
                if let (Some(slot), Some(generation)) = (spec["slot"].as_u64(), spec["gen"].as_u64()) {
                    let raw = ((generation as u32) << 16) | slot as u32;
                    registry.insert(raw, object);
                    gtid.insert(object.0, raw);
                }
            }
        }
        let defs: Vec<ObjectiveDef> = c["objective_defs"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|d| ObjectiveDef {
                        name: utf16(d["name"].as_str().unwrap()),
                        text: utf16(d["text"].as_str().unwrap()),
                        flag_10: u(&d["flag"]) as u8,
                    })
                    .collect()
            })
            .unwrap_or_default();
        let mut kingdom_objectives = BTreeMap::new();
        let mut results = BTreeMap::new();
        let mut teams = BTreeMap::new();
        let mut flag239 = BTreeMap::new();
        if let Some(kingdoms) = c["kingdoms"].as_object() {
            for (name, spec) in kingdoms {
                let k = index[name].0;
                let entries = spec["objectives"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|e| Entry {
                        definition: defs
                            .iter()
                            .position(|d| d.name == utf16(e[0].as_str().unwrap()))
                            .unwrap(),
                        state: u(&e[1]),
                    })
                    .collect();
                kingdom_objectives.insert(k, entries);
                results.insert(k, spec["result"].as_u64().unwrap_or(0) as u32);
                teams.insert(k, spec["team_members"].as_u64().map(|n| n as usize));
                flag239.insert(k, spec["flag_239"].as_u64().unwrap_or(0) as u8);
            }
        }
        let returns = c["returns"]
            .as_object()
            .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();
        Self {
            c: c.clone(),
            names,
            index,
            registry,
            gtid,
            returns,
            calls: vec![],
            defs,
            kingdom_objectives,
            results,
            teams,
            flag239,
            weather: None,
            flags8_writes: BTreeMap::new(),
        }
    }
    pub fn name(&self, o: Object) -> String {
        self.names[(o.0 - 1) as usize].clone()
    }
    pub fn describe(&self, o: Option<Object>) -> Value {
        match o {
            Some(o) => json!(format!("@{}", self.name(o))),
            None => json!(0),
        }
    }
    pub fn object_ref(&self, v: &Value) -> Option<Object> {
        match v {
            Value::String(s) if s.starts_with('@') => Some(self.index[&s[1..]]),
            Value::String(s) if s.starts_with("gtid:") => Some(self.index[&s[5..]]),
            Value::Number(n) if n.as_u64() == Some(0) => None,
            other => panic!("object reference {other}"),
        }
    }
    pub fn record(&mut self, call: &str, this: Option<Value>, args: Vec<Value>) {
        let mut e = json!({ "call": call });
        if let Some(this) = this {
            e["this"] = this;
        }
        if !args.is_empty() {
            e["args"] = Value::Array(args);
        }
        self.calls.push(e);
    }
    /// harness.Machine.give
    pub fn give(&mut self, call: &str, this: Option<Object>, arg: Option<&str>) -> Value {
        let this_name = this.map(|o| self.name(o));
        let Some(spec) = self.returns.get_mut(call) else {
            return json!(0);
        };
        let mut spec = if let Value::Array(list) = spec {
            if list.is_empty() { json!(0) } else { list.remove(0) }
        } else {
            spec.clone()
        };
        if let Some(by_arg) = spec.get("by_arg") {
            spec = by_arg.get(arg.unwrap_or("")).cloned().unwrap_or(json!(0));
        } else if let Value::Object(map) = &spec {
            spec = this_name
                .and_then(|n| map.get(&n).cloned())
                .or_else(|| map.get("*").cloned())
                .unwrap_or(json!(0));
        }
        spec
    }
    pub fn give_object(&mut self, call: &str, this: Option<Object>, arg: Option<&str>) -> Option<Object> {
        let v = self.give(call, this, arg);
        self.object_ref(&v)
    }
    pub fn give_word(&mut self, call: &str, this: Option<Object>, arg: Option<&str>) -> u32 {
        let v = self.give(call, this, arg);
        u(&v)
    }
    fn word_field(&self, o: Object, offset: &str) -> u32 {
        if offset == "0x8"
            && let Some(v) = self.flags8_writes.get(&o.0)
        {
            return *v;
        }
        let spec = &self.c["objects"][self.name(o)];
        match spec["words"].get(offset) {
            Some(Value::String(s)) if s.starts_with('@') => panic!("pointer field {offset} read as word"),
            Some(v) => u(v),
            None => 0,
        }
    }
    fn spawn_words(&self, r: &actions::SpawnRequest) -> Value {
        json!([
            10640168u32, self.describe(Some(r.definition)), r.position[0], r.position[1], r.position[2],
            r.facing[0], r.facing[1], format!("${}", string(&r.name)), 4294967295u32, 0,
            self.describe(Some(r.kingdom)), 0, 0, 4294967295u32, 0, 0, 0, 0, 0, 0, 0
        ])
    }
    pub fn list_field(&self, o: Object, offset: &str) -> Vec<Option<Object>> {
        self.c["objects"][self.name(o)]["lists"]
            .get(offset)
            .and_then(|l| l.as_array())
            .map(|l| l.iter().map(|v| self.object_ref(v)).collect())
            .unwrap_or_default()
    }
    fn ref_field(&self, o: Object, offset: &str) -> Option<Object> {
        let spec = &self.c["objects"][self.name(o)];
        match spec["words"].get(offset) {
            None => None,
            Some(v) => self.object_ref(v),
        }
    }
}

impl World for FixtureWorld {
    fn new(c: &Value) -> Self {
        Self::build(c)
    }
    fn base(&self) -> &FixtureWorld {
        self
    }
    fn base_mut(&mut self) -> &mut FixtureWorld {
        self
    }
}

impl Registry for FixtureWorld {
    fn resolve_id(&self, raw: u32) -> Result<Option<Object>> {
        Ok(self.registry.get(&raw).copied())
    }
    fn raw_id(&self, object: Object) -> Result<u32> {
        self.gtid
            .get(&object.0)
            .copied()
            .ok_or_else(|| Error::State("object without GTID".into()))
    }
    fn named_actor(&mut self, name: &[u16]) -> Result<Option<Object>> {
        let n = string(name);
        self.record("NamedActor", None, vec![json!(n)]);
        Ok(self.give_object("NamedActor", None, Some(&n)))
    }
    fn named_kingdom(&mut self, name: &[u16]) -> Result<Option<Object>> {
        let n = string(name);
        self.record("NamedKingdom", None, vec![json!(n)]);
        Ok(self.give_object("NamedKingdom", None, Some(&n)))
    }
}

impl Checks for FixtureWorld {
    fn is_area(&mut self, object: Object) -> Result<bool> {
        let this = self.describe(Some(object));
        self.record("IsArea", Some(this), vec![]);
        Ok(self.give_word("IsArea", Some(object), None) & 0xff != 0)
    }
    fn is_camera(&mut self, object: Object) -> Result<bool> {
        let this = self.describe(Some(object));
        self.record("IsCamera", Some(this), vec![]);
        Ok(self.give_word("IsCamera", Some(object), None) & 0xff != 0)
    }
}

impl objectives::Services for FixtureWorld {
    fn objective_definitions(&self) -> Result<&[ObjectiveDef]> {
        Ok(&self.defs)
    }
    fn kingdom_objectives_mut(&mut self, kingdom: Object) -> Result<&mut Vec<Entry>> {
        self.kingdom_objectives
            .get_mut(&kingdom.0)
            .ok_or_else(|| Error::State("Kingdom without objective list".into()))
    }
    fn sort_objectives(&mut self) -> Result<bool> {
        self.record("SortObjectivesVar", None, vec![]);
        Ok(self.give_word("SortObjectivesVar", None, None) & 0xff != 0)
    }
    fn post_message(&mut self, code: u32, kingdom: Object, text: &[u16]) -> Result<()> {
        let k = self.describe(Some(kingdom));
        self.record("PostMessage", None, vec![json!(code), k, json!(string(text))]);
        Ok(())
    }
}

impl objectives::MissionServices for FixtureWorld {
    fn mission_result(&self, kingdom: Object) -> Result<u32> {
        Ok(self.results[&kingdom.0])
    }
    fn set_mission_result(&mut self, kingdom: Object, result: u32) -> Result<()> {
        self.results.insert(kingdom.0, result);
        Ok(())
    }
    fn game_var_byte(&mut self, name: &[u16]) -> Result<Option<u8>> {
        let n = string(name);
        self.record("GameVar", None, vec![json!(n)]);
        let known = &self.returns.get("GameVar").cloned().unwrap_or(json!({}))["by_arg"];
        Ok(known.get(&n).and_then(|v| v.as_u64()).map(|v| v as u8))
    }
    fn team_member_count(&self, kingdom: Object) -> Result<Option<usize>> {
        Ok(self.teams.get(&kingdom.0).copied().flatten())
    }
    fn world_48(&self) -> Result<u32> {
        Ok(self.c["world_48"].as_u64().unwrap_or(0) as u32)
    }
    fn kingdom_flag_239(&self, kingdom: Object) -> Result<u8> {
        Ok(self.flag239.get(&kingdom.0).copied().unwrap_or(0))
    }
    fn kingdom_result_text(&mut self, kingdom: Object, flag: u32) -> Result<Vec<u16>> {
        let this = self.describe(Some(kingdom));
        self.record("KingdomResultText", Some(this), vec![json!(flag)]);
        Ok(utf16(&format!("result:{}", self.name(kingdom))))
    }
    fn post_mission_message(&mut self, code: u32, kingdom: Object, text: &[u16]) -> Result<()> {
        objectives::Services::post_message(self, code, kingdom, text)
    }
    fn local_kingdom(&mut self) -> Result<Option<Object>> {
        self.record("LocalKingdom", None, vec![]);
        Ok(self.give_object("LocalKingdom", None, None))
    }
    fn ui_mission_ended(&mut self) -> Result<()> {
        self.record("UiMissionEnded", None, vec![]);
        Ok(())
    }
    fn world_mission_result(&mut self, kingdom: Object) -> Result<()> {
        let k = self.describe(Some(kingdom));
        self.record("WorldMissionResult", None, vec![k]);
        Ok(())
    }
}

fn bank_name(bank: Bank) -> &'static str {
    match bank {
        Bank::Audio => "audio",
        Bank::ActorData => "actor_data",
        Bank::Property => "property",
        Bank::Command => "command",
        Bank::Weather => "weather",
        Bank::Sun => "sun",
        Bank::Formation => "formation",
        Bank::Sai => "sai",
        Bank::Ability => "ability",
    }
}

impl elements::Services for FixtureWorld {
    fn data_by_ids(&mut self, bank: Bank, name: &[u16]) -> Result<Option<Object>> {
        let call = format!("DataByIds:{}", bank_name(bank));
        let n = string(name);
        self.record(&call, None, vec![json!(n)]);
        Ok(self.give_object(&call, None, Some(&n)))
    }
    fn flags8(&self, object: Object) -> Result<u32> {
        Ok(self.word_field(object, "0x8"))
    }
    fn owner(&mut self, actor: Object) -> Result<Option<Object>> {
        let this = self.describe(Some(actor));
        self.record("Owner", Some(this), vec![]);
        Ok(self.give_object("Owner", Some(actor), None))
    }
    fn transfer_control(&mut self, actor: Object, kingdom: Object) -> Result<()> {
        let (this, k) = (self.describe(Some(actor)), self.describe(Some(kingdom)));
        self.record("TransferControl", Some(this), vec![k]);
        Ok(())
    }
    fn kingdom_default_lose(&mut self, kingdom: Object) -> Result<bool> {
        let this = self.describe(Some(kingdom));
        self.record("KingdomDefaultLose", Some(this), vec![]);
        Ok(self.give_word("KingdomDefaultLose", Some(kingdom), None) & 0xff != 0)
    }
    fn world_remove_object(&mut self, actor: Object) -> Result<()> {
        let a = self.describe(Some(actor));
        self.record("WorldRemoveObject", None, vec![a]);
        Ok(())
    }
    fn actor_destroy(&mut self, actor: Object) -> Result<()> {
        let this = self.describe(Some(actor));
        self.record("ActorDestroy", Some(this), vec![]);
        Ok(())
    }
    fn play_speech(&mut self, data: Option<Object>) -> Result<()> {
        let d = self.describe(data);
        self.record("PlaySpeech", None, vec![d, json!(0), json!(0), json!(0), json!(0)]);
        Ok(())
    }
    fn flash_ui_button(&mut self, name: &[u16]) -> Result<()> {
        self.record("FlashUiButton", None, vec![json!(string(name))]);
        Ok(())
    }
    fn difficulty_by_name(&mut self, name: &[u16]) -> Result<Option<i32>> {
        let n = string(name);
        self.record("DifficultyByName", None, vec![json!(n)]);
        let v = self.give("DifficultyByName", None, Some(&n));
        Ok(match v.as_u64() {
            Some(0) | None => None,
            Some(level) => Some(level as i32),
        })
    }
    fn is_actor_data(&mut self, data: Object) -> Result<bool> {
        let this = self.describe(Some(data));
        self.record("IsActorData", Some(this), vec![]);
        Ok(self.give_word("IsActorData", Some(data), None) != 0)
    }
    fn actor_virtual_158(&mut self, actor: Object) -> Result<bool> {
        let this = self.describe(Some(actor));
        self.record("Vt158", Some(this), vec![]);
        Ok(self.give_word("Vt158", Some(actor), None) & 0xff != 0)
    }
    fn actor_virtual_15c(&mut self, actor: Object) -> Result<Option<Object>> {
        let this = self.describe(Some(actor));
        self.record("Vt15C", Some(this), vec![]);
        Ok(self.give_object("Vt15C", Some(actor), None))
    }
    fn organization(&self, actor: Object) -> Result<Option<Object>> {
        Ok(self.ref_field(actor, "0x7c"))
    }
    fn organization_has_member(&mut self, organization: Object, id: u32) -> Result<bool> {
        let this = self.describe(Some(organization));
        self.record("OrganizationHasMember", Some(this), vec![json!(id)]);
        let members = self.c["objects"][self.name(organization)]["members"].clone();
        Ok(members
            .as_array()
            .map(|m| m.iter().any(|n| self.gtid.get(&self.index[n.as_str().unwrap()].0) == Some(&id)))
            .unwrap_or(false))
    }
    fn definition(&self, actor: Object) -> Result<Option<Object>> {
        Ok(self.ref_field(actor, "0x4"))
    }
    fn state_bits(&self, actor: Object) -> Result<u64> {
        Ok(u64::from(self.word_field(actor, "0x100")) | (u64::from(self.word_field(actor, "0x104")) << 32))
    }
    fn body(&self, actor: Object) -> Result<Option<(u32, u32)>> {
        Ok(self
            .ref_field(actor, "0x60")
            .map(|b| (self.word_field(b, "0x10"), self.word_field(b, "0x14"))))
    }
    fn area_first_actor(&mut self, area: Option<Object>) -> Result<Option<Object>> {
        let a = self.describe(area);
        self.record("AreaFirstActor", None, vec![a]);
        Ok(self.give_object("AreaFirstActor", None, None))
    }
    fn actor_inside_area(&mut self, actor: Option<Object>, area: Option<Object>) -> Result<bool> {
        let (a, r) = (self.describe(actor), self.describe(area));
        self.record("ActorInsideArea", None, vec![a, r]);
        Ok(self.give_word("ActorInsideArea", None, None) & 0xff != 0)
    }
    fn kingdom_player(&mut self, kingdom: Object) -> Result<Option<Object>> {
        let k = self.describe(Some(kingdom));
        self.record("KingdomPlayer", None, vec![k.clone()]);
        Ok(self.give_object("KingdomPlayer", None, k.as_str()))
    }
    fn player_selection(&self, player: Object) -> Result<Option<Object>> {
        Ok(self.ref_field(player, "0x50"))
    }
    fn selection_actors(&self, selection: Object) -> Result<Vec<Option<Object>>> {
        Ok(self.list_field(selection, "0x4"))
    }
    fn kingdom_has_property(&mut self, kingdom: Object, property: Object) -> Result<u32> {
        let (this, p) = (self.this(kingdom), self.this(property));
        self.record("KingdomHasProperty", Some(this), vec![p, json!(0)]);
        Ok(self.give_word("KingdomHasProperty", Some(kingdom), None))
    }
    fn actor_seen_by(&mut self, actor: Object, kingdom: Object, explored: bool) -> Result<u32> {
        let call = if explored { "IsExploredBy" } else { "IsVisibleTo" };
        let (this, k) = (self.this(actor), self.this(kingdom));
        self.record(call, Some(this), vec![k, json!(1)]);
        Ok(self.give_word(call, Some(actor), None))
    }
    fn map_difficulty(&self) -> Result<i32> {
        Ok(self.c["map_difficulty"]
            .as_u64()
            .ok_or_else(|| Error::State("map difficulty variable missing".into()))? as i32)
    }
}


fn bool_setting(c: &Value, key: &str) -> bool {
    c[key].as_u64().unwrap_or(0) & 0xff != 0
}

impl FixtureWorld {
    fn obj_text(&self, o: Object, offset: &str) -> Vec<u16> {
        match self.c["objects"][self.name(o)]["words"].get(offset) {
            Some(Value::String(s)) if s.starts_with('$') => utf16(&s[1..]),
            other => panic!("text field {offset} of {}: {other:?}", self.name(o)),
        }
    }
    fn this(&self, o: Object) -> Value {
        self.describe(Some(o))
    }
}

impl aa_triggers::predicates::CaptureObjects for FixtureWorld {
    fn capture_gtid(&self, object: u64) -> Result<u32> {
        Ok(self.gtid.get(&object).copied().unwrap_or(0))
    }
    fn capture_is_kingdom(&mut self, object: u64) -> Result<bool> {
        let this = self.describe(Some(Object(object)));
        self.record("IsKingdom", Some(this), vec![]);
        Ok(self.give_word("IsKingdom", Some(Object(object)), None) & 0xff != 0)
    }
    fn capture_owner(&mut self, object: u64) -> Result<Option<u64>> {
        let this = self.describe(Some(Object(object)));
        self.record("Owner", Some(this), vec![]);
        Ok(self.give_object("Owner", Some(Object(object)), None).map(|o| o.0))
    }
}

impl extended::Effects for FixtureWorld {
    fn view_adjust_sun_dimmer(&mut self, a: u32, b: u32) -> Result<()> {
        self.record("ViewAdjustSunDimmer", None, vec![json!(a), json!(b)]);
        Ok(())
    }
    fn sai_change_profile(&mut self, kingdom: Object, sai: Object) -> Result<()> {
        let (k, d) = (self.describe(Some(kingdom)), self.describe(Some(sai)));
        self.record("SaiChangeProfile", None, vec![k, d]);
        Ok(())
    }
    fn play_music(&mut self, data: Option<Object>) -> Result<()> {
        let d = self.describe(data);
        self.record("PlayMusic", None, vec![d]);
        Ok(())
    }
    fn ui_vfx_on_actor(&mut self, vfx: Object, actor: Object) -> Result<()> {
        let (v, a) = (self.describe(Some(vfx)), self.describe(Some(actor)));
        self.record("UiVfxOnActor", None, vec![v, a]);
        Ok(())
    }
    fn fidget_var_flags(&self) -> Result<u32> {
        Ok(self.c.get("fidget_flags").map(u).unwrap_or(0))
    }
    fn var_permission(&mut self, kind: u32) -> Result<bool> {
        self.record("VarPermission", None, vec![json!(kind)]);
        Ok(self.give_word("VarPermission", None, None) & 0xff != 0)
    }
    fn set_fidget_var(&mut self, value: u8) -> Result<()> {
        self.record("Store", Some(json!("@fidget_var")), vec![json!("0x0"), json!(value)]);
        Ok(())
    }
    fn set_game_speed(&mut self, bits: u32) -> Result<()> {
        self.record("SetGameSpeed", None, vec![json!(bits)]);
        Ok(())
    }
    fn kingdom_hero_entry(&mut self, kingdom: Object, data: Object) -> Result<Option<Object>> {
        let (k, d) = (self.describe(Some(kingdom)), self.describe(Some(data)));
        self.record("KingdomHeroEntry", Some(k), vec![d, json!(0)]);
        Ok(self.give_object("KingdomHeroEntry", Some(kingdom), None))
    }
    fn set_hero_entry_18(&mut self, entry: Object, bits: u32) -> Result<()> {
        let e = self.describe(Some(entry));
        self.record("Store", Some(e), vec![json!("0x18"), json!(bits)]);
        Ok(())
    }
    fn organization_modify_morale(&mut self, organization: Object, value: u32, mode: u32) -> Result<()> {
        let o = self.describe(Some(organization));
        self.record("OrganizationModifyMorale", Some(o), vec![json!(value), json!(mode)]);
        Ok(())
    }
    fn actor_display_name(&mut self, actor: Object) -> Result<Vec<u16>> {
        let a = self.describe(Some(actor));
        self.record("DisplayName", Some(a), vec![]);
        Ok(utf16(&format!("name:{}", self.name(actor))))
    }
    fn area_actor_ids(&self, area: Object) -> Result<Vec<u32>> {
        Ok(self
            .list_field(area, "0x84")
            .into_iter()
            .map(|r| self.word_field(r.expect("Area+84 node value"), "0x0"))
            .collect())
    }
    fn actor_4d12c0(&mut self, actor: Object) -> Result<bool> {
        let a = self.describe(Some(actor));
        self.record("Actor4D12C0", Some(a), vec![]);
        Ok(self.give_word("Actor4D12C0", Some(actor), None) & 0xff != 0)
    }
    fn world_4f6760(&mut self, actor: Object, flag: u32) -> Result<()> {
        let a = self.describe(Some(actor));
        self.record("World4F6760", None, vec![a, json!(flag)]);
        Ok(())
    }
    fn actor_teleport(&mut self, actor: Object, position: [u32; 3]) -> Result<()> {
        let a = self.describe(Some(actor));
        self.record("ActorTeleport", Some(a), vec![json!(position)]);
        Ok(())
    }
    fn member_teleport(&mut self, member: Object, position: [u32; 2]) -> Result<()> {
        let m = self.describe(Some(member));
        self.record("MemberTeleport", Some(m), vec![json!(position[0]), json!(position[1])]);
        Ok(())
    }
    fn store_flags8(&mut self, object: Object, value: u32) -> Result<()> {
        let o = self.describe(Some(object));
        self.record("Store", Some(o), vec![json!("0x8"), json!(value)]);
        self.flags8_writes.insert(object.0, value);
        Ok(())
    }
    fn world_4f03c0(&mut self) -> Result<bool> {
        self.record("World4F03C0", None, vec![]);
        Ok(self.give_word("World4F03C0", None, None) & 0xff != 0)
    }
    fn players_76ac60(&mut self) -> Result<Option<Object>> {
        self.record("Players76AC60", None, vec![]);
        Ok(self.give_object("Players76AC60", None, None))
    }
    fn player_765e00(&mut self, player: Option<Object>) -> Result<Option<Object>> {
        let p = self.describe(player);
        self.record("Player765E00", Some(p), vec![]);
        Ok(self.give_object("Player765E00", player, None))
    }
    fn audio_lookup(&mut self, name: &[u16]) -> Result<Option<Object>> {
        let n = string(name);
        self.record("AudioLookup", None, vec![json!(n)]);
        Ok(self.give_object("AudioLookup", None, Some(&n)))
    }
    fn new_hero_entry(&mut self, data: Object) -> Result<Option<Object>> {
        let object = Object(1001);
        self.names.resize(1001, String::new());
        self.names[1000] = "new_entry".into();
        self.index.insert("new_entry".into(), object);
        let d = self.describe(Some(data));
        self.record("HeroEntryInit", Some(json!("@new_entry")), vec![d]);
        Ok(Some(object))
    }
    fn kingdom_add_hero(&mut self, kingdom: Object, entry: Option<Object>) -> Result<()> {
        let (k, e) = (self.describe(Some(kingdom)), self.describe(entry));
        self.record("KingdomAddHero", Some(k), vec![e]);
        Ok(())
    }
    fn kingdom_remove_hero(&mut self, kingdom: Object, entry: Option<Object>) -> Result<()> {
        let (k, e) = (self.describe(Some(kingdom)), self.describe(entry));
        self.record("KingdomRemoveHero", Some(k), vec![e, json!(0)]);
        Ok(())
    }
    fn fog_restore_tiles(&mut self, area: Object, query: [u32; 4]) -> Result<()> {
        let a = self.describe(Some(area));
        self.record("FogRestoreTiles", None, vec![a, json!(query)]);
        Ok(())
    }
    fn actor_data_bank(&self) -> Result<Vec<Option<Object>>> {
        let Some(bank) = self.c["actor_data_bank"].as_str() else {
            return Err(Error::State("GameData+C ActorData bank is null: native faults".into()));
        };
        let bank = self.index[bank];
        let count = self.word_field(bank, "0x28");
        let array = self.ref_field(bank, "0x24");
        Ok((0..count)
            .map(|i| array.and_then(|a| self.ref_field(a, &format!("{:#x}", 4 * i))))
            .collect())
    }
    fn world_rng(&mut self) -> Result<Object> {
        self.record("WorldRng", None, vec![]);
        let object = Object(1002);
        self.names.resize(1002, String::new());
        self.names[1001] = "world_rng".into();
        self.index.insert("world_rng".into(), object);
        Ok(object)
    }
    fn random(&mut self, rng: Object) -> Result<u32> {
        let r = self.describe(Some(rng));
        self.record("Random", None, vec![r]);
        Ok(self.give_word("Random", None, None))
    }
    fn kingdom_hero_count(&self, kingdom: Object) -> Result<u32> {
        Ok(self.word_field(kingdom, "0x218"))
    }
    fn kingdom_hero_at(&mut self, kingdom: Object, index: u32) -> Result<Option<Object>> {
        let k = self.describe(Some(kingdom));
        self.record("KingdomHeroAt", Some(k), vec![json!(index)]);
        Ok(self.give_object("KingdomHeroAt", Some(kingdom), None))
    }
}

impl actions::Effects for FixtureWorld {
    fn warn_invalid_ids(&self) -> Result<bool> {
        Ok(bool_setting(&self.c, "warn_invalid_ids"))
    }
    fn camera_debug_log(&self) -> Result<bool> {
        Ok(bool_setting(&self.c, "camera_log"))
    }
    fn world_time_bits(&self) -> Result<u32> {
        Ok(u(&self.c["time_bits"]))
    }
    fn object_name(&self, object: Object) -> Result<Vec<u16>> {
        Ok(self.obj_text(object, "0x18"))
    }
    fn camera_fields(&self, camera: Object) -> Result<actions::CameraFields> {
        Ok(actions::CameraFields {
            look_at: [
                self.word_field(camera, "0x20"),
                self.word_field(camera, "0x24"),
                self.word_field(camera, "0x28"),
            ],
            zoom: self.word_field(camera, "0x60"),
        })
    }
    fn camera_pitch(&mut self, camera: Object) -> Result<u32> {
        let this = self.this(camera);
        self.record("CameraPitch", Some(this), vec![]);
        Ok(self.give_word("CameraPitch", Some(camera), None))
    }
    fn camera_yaw(&mut self, camera: Object) -> Result<u32> {
        let this = self.this(camera);
        self.record("CameraYaw", Some(this), vec![]);
        Ok(self.give_word("CameraYaw", Some(camera), None))
    }
    fn real_time_ms(&mut self) -> Result<u32> {
        self.record("RealTimeMs", None, vec![]);
        Ok(self.give_word("RealTimeMs", None, None))
    }
    fn camera_var(&self, offset: u32) -> Result<u32> {
        Ok(self.c["vars990"]
            .get(format!("{offset:#x}"))
            .map(u)
            .unwrap_or(0))
    }
    fn ui_script_command(&mut self, command: &[u16]) -> Result<()> {
        self.record("UiScriptCommand", None, vec![json!(string(command))]);
        Ok(())
    }
    fn world_set_cutscene_mode(&mut self, enabled: bool) -> Result<()> {
        self.record("WorldSetCutsceneMode", None, vec![json!(u32::from(enabled))]);
        Ok(())
    }
    fn camera_6fe710(&mut self) -> Result<()> {
        self.record("Camera6FE710", None, vec![]);
        Ok(())
    }
    fn camera_700b10(&mut self) -> Result<u32> {
        self.record("Camera700B10", None, vec![]);
        Ok(self.give_word("Camera700B10", None, None))
    }
    fn camera_700a90(&mut self, value: u32) -> Result<()> {
        self.record("Camera700A90", None, vec![json!(value)]);
        Ok(())
    }
    fn kingdom_add_property(&mut self, kingdom: Object, property: Object, kind: u32, a: u32, notify: u32) -> Result<()> {
        let (this, prop) = (self.this(kingdom), self.this(property));
        self.record("KingdomAddProperty", Some(this), vec![prop, json!(kind), json!(a), json!(notify)]);
        Ok(())
    }
    fn kingdom_remove_property(&mut self, kingdom: Object, property: Object, a: u32) -> Result<()> {
        let (this, prop) = (self.this(kingdom), self.this(property));
        self.record("KingdomRemoveProperty", Some(this), vec![prop, json!(a)]);
        Ok(())
    }
    fn actor_remove_property(&mut self, actor: Object, property: Object, a: u32) -> Result<()> {
        let (this, prop) = (self.this(actor), self.this(property));
        self.record("ActorRemoveProperty", Some(this), vec![prop, json!(a)]);
        Ok(())
    }
    fn world_kingdom_slots(&self) -> Result<Vec<Option<Object>>> {
        Ok(self.c["world_kingdoms"]
            .as_array()
            .map(|a| a.iter().map(|k| k.as_str().map(|n| self.index[n])).collect())
            .unwrap_or_default())
    }
    fn ui_display_banner(&mut self, text: &[u16], duration: u32) -> Result<()> {
        self.record("UiDisplayBanner", None, vec![json!(string(text)), json!(duration)]);
        Ok(())
    }
    fn sun_colors(&self, sun: Object) -> Result<[[u32; 3]; 3]> {
        let w = |i: u32| self.word_field(sun, &format!("{:#x}", 0x18 + 4 * i));
        Ok([[w(0), w(1), w(2)], [w(3), w(4), w(5)], [w(6), w(7), w(8)]])
    }
    fn view_set_sun_colors(&mut self, colors: &[[u32; 3]; 3], duration: u32) -> Result<()> {
        self.record(
            "ViewSetSunColors",
            None,
            vec![json!(colors[0]), json!(colors[1]), json!(colors[2]), json!(duration)],
        );
        Ok(())
    }
    fn resource_flags18(&self) -> Result<Vec<u32>> {
        Ok(self.c["resources"]
            .as_array()
            .map(|a| a.iter().map(u).collect())
            .unwrap_or_default())
    }
    fn kingdom_add_resource(&mut self, kingdom: Object, index: u32, amount: u32) -> Result<()> {
        let this = self.this(kingdom);
        self.record("KingdomAddResource", Some(this), vec![json!(index), json!(amount)]);
        Ok(())
    }
    fn world_show_counter(&mut self, start: u32, duration: u32) -> Result<()> {
        self.record("WorldShowCounter", None, vec![json!([start, duration])]);
        Ok(())
    }
    fn world_hide_counter(&mut self) -> Result<()> {
        self.record("WorldHideCounter", None, vec![]);
        Ok(())
    }
    fn set_weather(&mut self, weather: Option<Object>) -> Result<()> {
        self.weather = weather;
        Ok(())
    }
    fn play_sound(&mut self, data: Option<Object>, location: Option<[u32; 3]>) -> Result<()> {
        let d = self.describe(data);
        let l = location.map(|v| json!(v)).unwrap_or(json!(0));
        self.record("PlaySound", None, vec![d, json!(0), json!(0), json!(0), json!(0), l, json!(0), json!(0)]);
        Ok(())
    }
    fn ui_flash_actor(&mut self, actor: Object) -> Result<()> {
        let a = self.this(actor);
        self.record("UiFlashActor", None, vec![a]);
        Ok(())
    }
    fn ui_root(&self) -> Result<Option<Object>> {
        Ok(self.index.get("ui_root").copied())
    }
    fn ui_find_widget(&mut self, root: Object, name: &[u16]) -> Result<Option<Object>> {
        let this = self.this(root);
        let n = string(name);
        self.record("UiFindWidget", Some(this), vec![json!(n)]);
        Ok(self.give_object("UiFindWidget", Some(root), Some(&n)))
    }
    fn widget_name(&self, widget: Object) -> Result<Vec<u16>> {
        Ok(self.obj_text(widget, "0x4"))
    }
    fn world_script_listen(&mut self, name: &[u16]) -> Result<()> {
        self.record("WorldScriptListen", None, vec![json!(string(name))]);
        Ok(())
    }
    fn world_script_unlisten(&mut self, name: &[u16]) -> Result<bool> {
        self.record("WorldScriptUnlisten", None, vec![json!(string(name))]);
        Ok(self.give_word("WorldScriptUnlisten", None, None) & 0xff != 0)
    }
    fn actor_gain_experience(&mut self, actor: Object, amount: u32) -> Result<()> {
        let this = self.this(actor);
        self.record("ActorGainExperience", Some(this), vec![json!(amount), json!(0), json!(0)]);
        Ok(())
    }
    fn is_vfx(&mut self, data: Object) -> Result<bool> {
        let this = self.this(data);
        self.record("IsVfx", Some(this), vec![]);
        Ok(self.give_word("IsVfx", Some(data), None) != 0)
    }
    fn default_vfx_orientation(&self) -> Result<[u32; 2]> {
        let v = &self.c["vars990"]["0x64"];
        Ok([v.get(0).map(u).unwrap_or(0), v.get(1).map(u).unwrap_or(0)])
    }
    fn rotate_2d(&mut self, vector: [u32; 2], angle: u32) -> Result<[u32; 2]> {
        self.record("Rotate2D", None, vec![json!(vector), json!(angle)]);
        let r = self.returns.get("Rotate2D").cloned().unwrap_or(json!([0, 0]));
        Ok([u(&r[0]), u(&r[1])])
    }
    fn spawn_vfx(&mut self, vfx: Object, position: [u32; 3], orientation: [u32; 3]) -> Result<()> {
        let v = self.this(vfx);
        self.record("SpawnVfx", None, vec![v, json!(position), json!(orientation)]);
        Ok(())
    }
    fn chat_message_all(&mut self, text: &[u16]) -> Result<u32> {
        self.record("ChatMessageAll", None, vec![json!(15), json!(string(text))]);
        Ok(self.give_word("ChatMessageAll", None, None))
    }
    fn chat_message_kingdom(&mut self, kingdom: Object, text: &[u16]) -> Result<()> {
        let k = self.this(kingdom);
        self.record("ChatMessageKingdom", None, vec![json!(15), k, json!(0), json!(string(text))]);
        Ok(())
    }
    fn animation_set(&self, actor: Object) -> Result<Option<Object>> {
        let Some(def) = self.ref_field(actor, "0x4") else { return Ok(None) };
        let count = self.word_field(def, "0x1c0");
        if count == 0 {
            return Ok(None);
        }
        let Some(array) = self.ref_field(def, "0x1bc") else {
            return Err(Error::State("animation array missing".into()));
        };
        Ok(self.ref_field(array, &format!("{:#x}", (count - 1) * 4)))
    }
    fn animation_by_name(&mut self, set: Object, name: &[u16]) -> Result<Option<Object>> {
        let this = self.this(set);
        let n = string(name);
        self.record("AnimationByName", Some(this), vec![json!(n)]);
        Ok(self.give_object("AnimationByName", Some(set), Some(&n)))
    }
    fn actor_play_animation(&mut self, actor: Object, animation: Object) -> Result<bool> {
        let (this, a) = (self.this(actor), self.this(animation));
        self.record("ActorPlayAnimation", Some(this), vec![a]);
        Ok(self.give_word("ActorPlayAnimation", Some(actor), None) & 0xff != 0)
    }
    fn actor_4d4c60(&mut self, actor: Object, flag: u8) -> Result<()> {
        let this = self.this(actor);
        self.record("Actor4D4C60", Some(this), vec![json!(flag)]);
        Ok(())
    }
    fn process_command(&mut self, actor: Object, command: &mut actions::Command) -> Result<bool> {
        let this = json!({
            "definition": self.describe(command.definition),
            "flags": command.flags,
            "words": [command.target_id, self.describe(command.data), command.location[0], command.location[1],
                      command.move_words[0], command.move_words[1], command.move_words[2], 0],
            "byte30": 0, "shared34": 0, "word38": 0, "byte3c": 0,
        });
        let a = self.this(actor);
        self.record("ProcessCommand", Some(this), vec![a]);
        Ok(self.give_word("ProcessCommand", None, None) & 0xff != 0)
    }
    fn release_command_shared(&mut self, _shared: u64) -> Result<()> {
        Err(Error::State("fixture commands never install +34".into()))
    }
    fn world_slots_148(&self) -> Result<Vec<Option<Object>>> {
        Ok(self.c["world_148"]
            .as_array()
            .map(|a| a.iter().map(|k| k.as_str().map(|n| self.index[n])).collect())
            .unwrap_or_default())
    }
    /// Node handle = (kingdom object << 16) | position + 1.
    fn member_head(&self, kingdom: Object) -> Result<Option<u64>> {
        Ok((!self.list_field(kingdom, "0x284").is_empty()).then_some((kingdom.0 << 16) | 1))
    }
    fn member_value(&self, node: u64) -> Result<Option<Object>> {
        let list = self.list_field(Object(node >> 16), "0x284");
        Ok(list[(node & 0xffff) as usize - 1])
    }
    fn member_next(&self, node: u64) -> Result<Option<u64>> {
        let list = self.list_field(Object(node >> 16), "0x284");
        let next = (node & 0xffff) as usize + 1;
        Ok((next <= list.len()).then_some((node & !0xffff) | next as u64))
    }
    fn actor_80_ready(&self, actor: Object) -> Result<bool> {
        Ok(match self.ref_field(actor, "0x80") {
            Some(thing) => self.word_field(thing, "0x10") != 0,
            None => false,
        })
    }
    fn video_path(&mut self, name: &[u16]) -> Result<Vec<u16>> {
        let n = string(name);
        self.record("VideoPath", None, vec![json!(n)]);
        Ok(utf16(&format!("video:{n}")))
    }
    fn game_var_string(&mut self, name: &[u16]) -> Result<Vec<u16>> {
        let n = string(name);
        self.record("GameVarString", None, vec![json!(n)]);
        if let Some(by_arg) = self.returns.get("GameVarString").and_then(|v| v.get("by_arg")) {
            return Ok(utf16(by_arg.get(&n).and_then(|v| v.as_str()).unwrap_or("")));
        }
        Ok(utf16(&format!("var:{n}")))
    }
    fn command_definition(&self, ordinal: u32) -> Result<Option<Object>> {
        Ok(self.c["command_defs"]
            .get(ordinal.to_string())
            .and_then(|n| n.as_str())
            .map(|n| self.index[n]))
    }
    fn move_factor(&self, actor: Object) -> Result<Option<u32>> {
        if let Some(body) = self.ref_field(actor, "0x6c") {
            return Ok(Some(self.word_field(body, "0x28")));
        }
        if let Some(org) = self.ref_field(actor, "0x7c") {
            return Ok(Some(self.word_field(org, "0xd8")));
        }
        Ok(None)
    }
    fn find_placement(&mut self, position: &mut [u32; 2], facing: &mut [u32; 2], data: Object, radius: u32, mode: u32) -> Result<bool> {
        let d = self.this(data);
        self.record(
            "FindPlacement",
            None,
            vec![json!(*position), json!(*facing), d, json!(radius), json!(mode), json!(0)],
        );
        let spec = match self.returns.get_mut("FindPlacement") {
            Some(Value::Array(list)) if !list.is_empty() => list.remove(0),
            Some(Value::Array(_)) | None => json!({"ok": 0}),
            Some(v) => v.clone(),
        };
        if let Some(p) = spec["pos"].as_array() {
            *position = [u(&p[0]), u(&p[1])];
        }
        if let Some(f) = spec["facing"].as_array() {
            *facing = [u(&f[0]), u(&f[1])];
        }
        Ok(spec["ok"].as_u64().unwrap_or(0) & 0xff != 0)
    }
    fn data_word(&self, object: Object, offset: u32) -> Result<u32> {
        Ok(self.word_field(object, &format!("{offset:#x}")))
    }
    fn data_ref(&self, object: Object, offset: u32) -> Result<Option<Object>> {
        Ok(self.ref_field(object, &format!("{offset:#x}")))
    }
    fn transport_path(&mut self, from: [u32; 2], to: [u32; 2]) -> Result<[u32; 2]> {
        self.record("TransportPath", None, vec![json!(from[0]), json!(from[1]), json!(to[0]), json!(to[1])]);
        let r = self.returns.get("TransportPath").cloned().unwrap_or(json!([0, 0]));
        Ok([u(&r[0]), u(&r[1])])
    }
    fn transport_path_valid(&mut self, point: [u32; 2], to: [u32; 2]) -> Result<bool> {
        self.record("TransportPathValid", None, vec![json!(point[0]), json!(point[1]), json!(to[0]), json!(to[1])]);
        Ok(self.give_word("TransportPathValid", None, None) & 0xff != 0)
    }
    fn create_transport(&mut self, r: &actions::SpawnRequest) -> Result<Option<Object>> {
        let words = self.spawn_words(r);
        self.record("WorldCreateTransport", None, vec![words]);
        Ok(self.give_object("WorldCreateTransport", None, None))
    }
    fn or_flags8(&mut self, object: Object, bits: u32) -> Result<()> {
        let v = self.word_field(object, "0x8") | bits;
        self.flags8_writes.insert(object.0, v);
        Ok(())
    }
    fn create_group(&mut self, word_2f0: u32) -> Result<Option<Object>> {
        self.record("CreateGroup", None, vec![json!(word_2f0)]);
        let object = Object(1000);
        self.names.resize(1000, String::new());
        self.names[999] = "new_group".into();
        self.index.insert("new_group".into(), object);
        Ok(Some(object))
    }
    fn transport_load(&mut self, component: Object, cargo: Object, name: &[u16], group: Option<Object>, position: [u32; 2], destination: [u32; 2]) -> Result<()> {
        let (this, c, g) = (self.this(component), self.this(cargo), self.describe(group));
        self.record(
            "TransportLoad",
            Some(this),
            vec![c, json!(string(name)), g, json!(position[0]), json!(position[1]), json!(destination[0]), json!(destination[1])],
        );
        Ok(())
    }
    fn flare_ready(&mut self, _config: Object, on_actor: bool) -> Result<bool> {
        let call = if on_actor { "FlareReadyActor" } else { "FlareReadyPosition" };
        self.record(call, None, vec![json!(0)]);
        Ok(self.give_word(call, None, None) & 0xff != 0)
    }
    fn flare_var(&mut self, _on_actor: bool) -> Result<u32> {
        self.record("FlareVar", None, vec![json!("SAITriggerTeamCommandPriority")]);
        Ok(self.give_word("FlareVar", None, None))
    }
    fn create_flare(&mut self, r: &actions::SpawnRequest, kingdom: Object, actor: Option<Object>, duration: u32, variable: u32) -> Result<()> {
        let words = self.spawn_words(r);
        let (k, a) = (self.describe(Some(kingdom)), self.describe(actor));
        self.record("CreateFlare", None, vec![words, k, a, json!(duration), json!(variable)]);
        Ok(())
    }
    fn ability_user_init(&mut self, ability: Object) -> Result<()> {
        let a = self.describe(Some(ability));
        self.record("AbilityUserInit", None, vec![a]);
        Ok(())
    }
    fn ability_use(&mut self, r: &actions::AbilityRequest) -> Result<()> {
        let k = self.describe(Some(r.kingdom));
        self.record(
            "AbilityUse",
            None,
            vec![json!([5, 0, 0, k, r.from[0], r.from[1], r.from[2], 0, r.at[0], r.at[1], r.at[2], 0])],
        );
        Ok(())
    }
    fn ability_user_release(&mut self) -> Result<()> {
        self.record("AbilityUserRelease", None, vec![]);
        Ok(())
    }
    fn data_flags174(&self, data: Object) -> Result<u32> {
        Ok(self.word_field(data, "0x174"))
    }
    fn create_actor(&mut self, r: &actions::SpawnRequest, alternate: bool) -> Result<Option<Object>> {
        let call = if alternate { "WorldCreateActor150" } else { "WorldCreateActor" };
        let words = self.spawn_words(r);
        self.record(call, None, vec![words]);
        Ok(self.give_object(call, None, None))
    }
}

pub fn manager(c: &Value, world: &FixtureWorld) -> Manager {
    let mut manager = Manager::from_saved(SavedManager {
        next_id: 1,
        triggers: c["triggers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| Trigger {
                source: span(),
                id: u(&t["id"]),
                mode: 1,
                enabled: u(&t["enabled"]) as u8,
                repeat: 0,
                activation_bits: u(&t["activation_bits"]),
                name: vec![],
                description: vec![],
                word_2c: 0,
                events: vec![],
                conditions: vec![],
                actions: vec![],
            })
            .collect(),
        variables: c["variables"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(name, bits)| Variable {
                name: utf16(name),
                bits: u(bits),
            })
            .collect(),
        strings: c["strings"]
            .as_array()
            .map(|a| a.iter().map(|v| utf16(v.as_str().unwrap())).collect())
            .unwrap_or_default(),
    });
    let m = &c["manager"];
    if !m.is_null() {
        manager.set_context_word(m["context_word"].as_u64().unwrap_or(0) as u32);
        let subjects: Vec<u32> = m["subjects"]
            .as_array()
            .map(|a| a.iter().map(u).collect())
            .unwrap_or_default();
        for s in subjects.iter().rev() {
            manager.add_subject(*s);
        }
        manager.set_condition_kingdom(world.object_ref(&m["condition_kingdom"]).map(|o| o.0));
        manager.set_event_kingdom(world.object_ref(&m["event_kingdom"]).map(|o| o.0));
        if let Some(events) = m["events"].as_array() {
            for (i, ev) in events.iter().enumerate().rev() {
                manager.push_event(Some(DeferredPayload {
                    identity: i as u64 + 1,
                    data: event(ev, world),
                }));
            }
        }
    }
    manager
}

/// Fixture event description -> owned native payload (harness.Machine.event).
pub fn event(ev: &Value, world: &FixtureWorld) -> Event {
    let w = |i: usize| -> u64 {
        match &ev["words"][i] {
            Value::String(s) if s.starts_with("gtid:") => u64::from(world.gtid[&world.index[&s[5..]].0]),
            Value::String(s) if s.starts_with('@') => world.index[&s[1..]].0,
            other => other.as_u64().unwrap_or_else(|| panic!("event word {other}")),
        }
    };
    match u(&ev["kind"]) {
        0 => Event::Destroyed {
            victim_id: w(0) as u32,
            killer_id: w(1) as u32,
            word_c: w(2) as u32,
        },
        1 => Event::Captured {
            actor_id: w(0) as u32,
            capturer_id: w(1) as u32,
            kingdom_id: w(2) as u32,
        },
        2 => Event::AreaEntry {
            actor_id: w(0) as u32,
            area: w(1),
        },
        3 => Event::Construction {
            kingdom_id: w(1) as u32,
            definition: w(2),
            flag_10: w(3) as u8,
        },
        4 => Event::Recruitment {
            kingdom_id: w(1) as u32,
            definition: w(2),
            flag_10: w(3) as u8,
        },
        5 => Event::Script {
            text: utf16(ev["text"].as_str().unwrap()),
        },
        other => panic!("event kind {other}"),
    }
}

pub fn precision(name: &str) -> Precision {
    match name {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        "Extended64" => Precision::Extended64,
        other => panic!("{other}"),
    }
}

/// Run one native case through the Rust evaluator; Err describes a mismatch.
pub fn run_case<W: World>(case: &Value) -> std::result::Result<(), String> {
    let c = &case["input"];
    let out = &case["output"];
    let name = c["name"].as_str().unwrap().to_string();
    let mut world = W::new(c);
    let mut manager = manager(c, world.base());
    let mut core = CoreProgram::new(u(&c["time_bits"]), 0, precision(c["precision"].as_str().unwrap()));
    // Earlier elements share the manager+2C register file (native_extended prelude).
    if let Some(prelude) = c["prelude"].as_array() {
        for p in prelude {
            if let Err(err) = elements::evaluate(&mut core, &mut manager, &element(p), &mut world) {
                return Err(format!("{name}: prelude error {err}"));
            }
        }
    }
    let e = element(&c["element"]);
    let result = elements::evaluate(&mut core, &mut manager, &e, &mut world);
    if c.get("rust_unsupported").is_some() {
        return match result {
            Err(_) => Ok(()),
            Ok(v) => Err(format!("{name}: documented unsupported case returned {v:?}")),
        };
    }
    if out.get("fault").is_some() {
        return match result {
            Err(_) => Ok(()),
            Ok(v) => Err(format!("{name}: native faults, Rust returned {v:?}")),
        };
    }
    let result = match result {
        Ok(Some(v)) => v,
        Ok(None) => return Err(format!("{name}: opcode not handled")),
        Err(err) => return Err(format!("{name}: Rust error {err}")),
    };
    let b = world.base();
    let mut diffs = vec![];
    if result != out["result"].as_bool().unwrap() {
        diffs.push(format!("result {result} != {}", out["result"]));
    }
    let calls = Value::Array(b.calls.clone());
    if calls != out["calls"] {
        diffs.push(format!("calls\n  rust   {calls}\n  native {}", out["calls"]));
    }
    if manager.context_word() != u(&out["context_word"]) {
        diffs.push(format!("context {} != {}", manager.context_word(), out["context_word"]));
    }
    let ck = b.describe(manager.condition_kingdom().map(Object));
    if ck != out["condition_kingdom"] {
        diffs.push(format!("condition kingdom {ck} != {}", out["condition_kingdom"]));
    }
    let ek = b.describe(manager.event_kingdom().map(Object));
    if ek != out["event_kingdom"] {
        diffs.push(format!("event kingdom {ek} != {}", out["event_kingdom"]));
    }
    let snapshot = manager.snapshot();
    let subjects: Vec<u32> = snapshot.subjects.iter().copied().collect();
    let native_subjects: Vec<u32> = out["subjects"].as_array().unwrap().iter().map(u).collect();
    if subjects != native_subjects {
        diffs.push(format!("subjects {subjects:?} != {native_subjects:?}"));
    }
    let cursor = manager.current_subject();
    if cursor != out["cursor"].as_u64().map(|v| v as u32) {
        diffs.push(format!("cursor {cursor:?} != {}", out["cursor"]));
    }
    if let Some(kingdoms) = out["kingdoms"].as_object() {
        for (k, state) in kingdoms {
            let o = b.index[k].0;
            let got: Vec<Value> = b.kingdom_objectives[&o]
                .iter()
                .map(|e| json!([string(&b.defs[e.definition].name), e.state]))
                .collect();
            if Value::Array(got.clone()) != state["objectives"] {
                diffs.push(format!("{k} objectives {got:?} != {}", state["objectives"]));
            }
            if b.results[&o] != u(&state["result"]) {
                diffs.push(format!("{k} result {} != {}", b.results[&o], state["result"]));
            }
        }
    }
    if let Some(flags) = out.get("flags8").and_then(|f| f.as_object()) {
        for (name, v) in flags {
            // +8 of an array/table fixture holds a pointer, not a flag word.
            if matches!(&c["objects"][name]["words"]["0x8"], Value::String(r) if r.starts_with('@')) {
                continue;
            }
            let got = b.word_field(b.index[name], "0x8");
            if got != u(v) {
                diffs.push(format!("{name} flags8 {got:#x} != {v}"));
            }
        }
    }
    if let Some(w) = out.get("world_290") {
        let got = b.describe(b.weather);
        if &got != w {
            diffs.push(format!("weather {got} != {w}"));
        }
    }
    if core.diagnostics.len() as u64 != out["diagnostics"].as_u64().unwrap() {
        diffs.push(format!(
            "diagnostics {:?} != {}",
            core.diagnostics, out["diagnostics"]
        ));
    }
    if diffs.is_empty() {
        Ok(())
    } else {
        Err(format!("{name}: {}", diffs.join("; ")))
    }
}
