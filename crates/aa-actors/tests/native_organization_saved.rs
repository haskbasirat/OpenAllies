//! Replays research/phase8-saved-components/native-organization.json: the
//! original Organization constructor 579BD0 and saved Load 57D130 under
//! Unicorn. The world below answers memory facts from the case input and
//! records every ordered callback exactly like the harness seams; the whole
//! callback sequence, the outcome and the (partial) state must match.
use aa_actors::{
    ActorId,
    ground_states::{LayoutDefinition, OrganizationState},
    organization_state::{
        self as org, Arg, Definitions, Diagnostic, LayoutEntry, LayoutInfo, LoadError, MemberFacts,
        Message, OrganizationWorld, OwnerFacts, Result, StateWords, WorldFacts,
    },
};
use aa_body::F32;
use aa_numeric::{Binary, Precision};
use serde_json::{Value, json};
use std::collections::{BTreeMap, VecDeque};

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../research/phase8-saved-components/native-organization.json"
);
const OWNER: ActorId = ActorId::from_raw(0x0001_0100);
const KINGDOM: u32 = 0x4b00;
const MODIFIER: u32 = 0x7800;

struct Fixture {
    actor_flags: BTreeMap<String, (u32, u32)>,
    properties: Vec<String>,
    layouts: BTreeMap<String, Value>,
    formations: Vec<(u8, u32)>,
    damage: Vec<String>,
}

struct World<'a> {
    fx: &'a Fixture,
    spec: &'a Value,
    lists: BTreeMap<String, Vec<u32>>,
    labels: Vec<String>,
    pending: VecDeque<String>,
    state_group: VecDeque<Value>,
    modifier_results: VecDeque<u64>,
    latch: bool,
    registered: Option<i32>,
    calls: Vec<Value>,
}
impl World<'_> {
    fn member_id(&self, label: &str) -> ActorId {
        let i = self.labels.iter().position(|l| l == label).unwrap();
        ActorId::from_raw(0x0001_0000 + i as u32 + 1)
    }
    fn label(&self, id: ActorId) -> String {
        if id == OWNER {
            return "owner".into();
        }
        self.labels[(id.raw() - 0x0001_0001) as usize].clone()
    }
    fn member(&self, id: ActorId) -> &Value {
        &self.spec["members"][self.label(id)]
    }
    fn property_label(&self, handle: u32) -> String {
        format!(
            "property:{}",
            self.fx.properties[(handle - 0xe000) as usize]
        )
    }
    fn record(&mut self, v: Value) {
        self.calls.push(v);
    }
}
fn s(v: &Value) -> String {
    v.as_str().unwrap().to_owned()
}
fn u(v: &Value) -> u32 {
    v.as_i64().unwrap() as u32
}

impl Definitions<String> for World<'_> {
    fn layout_data(&mut self, ids: &[u16]) -> Result<Option<String>> {
        let name = String::from_utf16(ids).unwrap();
        let found = self.fx.layouts.contains_key(&name);
        self.record(json!({"seam": "463FA0", "ids": name, "found": found}));
        Ok(found.then_some(name))
    }
    fn actor_data(&mut self, ids: &[u16]) -> Result<Option<String>> {
        let name = String::from_utf16(ids).unwrap();
        let found = self.fx.actor_flags.contains_key(&name);
        self.record(json!({"seam": "4BDBA0", "ids": name, "found": found}));
        Ok(found.then_some(name))
    }
    fn layout_info(&self, layout: &String) -> Result<LayoutInfo> {
        let l = &self.fx.layouts[layout];
        Ok(LayoutInfo {
            name: layout.clone(),
            primary_slot: l["primary"].as_i64().unwrap() as i32,
            entries: l["entries"]
                .as_array()
                .unwrap()
                .iter()
                .map(|e| {
                    let property = s(&e[0]);
                    LayoutEntry {
                        property: 0xe000
                            + self
                                .fx
                                .properties
                                .iter()
                                .position(|p| *p == property)
                                .unwrap() as u32,
                        property_name: property,
                        required: u(&e[1]) != 0,
                        locked: u(&e[2]) != 0,
                        morale_line: u(&e[3]) != 0,
                    }
                })
                .collect(),
            slot_groups: l["groups"].as_array().unwrap().iter().map(u).collect(),
            formations: (0..u(&l["formations"])).map(|i| 0xf000 + i).collect(),
            formation_44: l["formation_44"].as_i64().unwrap() as i32,
            formation_48: l["formation_48"].as_i64().unwrap() as i32,
            property_50: u(&l["property_50"]),
        })
    }
    fn name(&self, definition: &String) -> Result<String> {
        Ok(definition.clone())
    }
}

impl OrganizationWorld<String> for World<'_> {
    fn updatable_table_word0(&self) -> Result<i32> {
        Ok(self.spec["table"]["max"].as_i64().unwrap() as i32)
    }
    fn updatable_slot_occupied(&self, id: i32) -> Result<bool> {
        Ok(self.spec["table"]["occupied"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v.as_i64() == Some(id as i64)))
    }
    fn register_updatable(&mut self, id: i32) -> Result<()> {
        self.registered = Some(id);
        Ok(())
    }
    fn actor_modifier(&mut self, actor: ActorId) -> Result<Option<u32>> {
        Ok((actor == OWNER && !self.spec["owner"]["modifier"].is_null()).then_some(MODIFIER))
    }
    fn modifier_apply(&mut self, modifier: u32, value_bits: u32) -> Result<Binary> {
        assert_eq!(modifier, MODIFIER);
        let r = self.modifier_results.pop_front().unwrap();
        self.record(json!({"seam": "501C90", "modifier": "modifier:owner", "value": value_bits, "result_f64": r}));
        Ok(Binary::from_f64_bits(r).unwrap())
    }
    fn morale_contribution(&self, actor_data: &String) -> Result<u32> {
        Ok(self.fx.actor_flags[actor_data].1)
    }
    fn has_property(&mut self, actor_data: &String, property: u32) -> Result<bool> {
        let key = format!("actor:{actor_data}/{}", self.property_label(property));
        let r = self.spec["has_property"]
            .get(&key)
            .is_none_or(|v| v.as_bool().unwrap());
        let p = self.property_label(property);
        self.record(json!({"seam": "45EEF0", "actor": format!("actor:{actor_data}"), "property": p, "result": r}));
        Ok(r)
    }
    fn log(&mut self, d: Diagnostic) -> Result<()> {
        let args: Vec<Value> = d
            .message
            .args
            .iter()
            .map(|a| match a {
                Arg::Text(t) => json!(t),
                Arg::Int(v) => json!(v),
            })
            .collect();
        let v = if d.function == 0x5a1040 {
            json!({"seam": "5A1040", "format": d.message.format, "args": args})
        } else {
            json!({"seam": format!("{:X}", d.function), "line": d.line, "format": d.message.format, "args": args})
        };
        self.record(v);
        Ok(())
    }
    fn load_member_record(&mut self, bytes: &[u8]) -> Result<(Option<ActorId>, usize)> {
        let label = self.pending.pop_front().unwrap();
        let n = u(&self.spec["members"][&label]["record_len"]) as usize;
        if bytes.len() < n {
            return Err(LoadError::Truncated { at: 0, wanted: n });
        }
        let member = (label != "null").then(|| self.member_id(&label));
        self.record(json!({"seam": "4FAE80", "member": if label == "null" { Value::Null } else { json!(label) }, "bytes": n}));
        Ok((member, n))
    }
    fn assert_latch(&self) -> Result<bool> {
        Ok(self.latch)
    }
    fn is_actor(&mut self, thing: ActorId) -> Result<bool> {
        let r = self.member(thing)["actor"].as_bool().unwrap();
        let l = self.label(thing);
        self.record(json!({"seam": "IsGActor", "member": l, "result": r}));
        Ok(r)
    }
    fn assert_dialog(&mut self) -> Result<bool> {
        self.record(json!({"seam": "855D30"}));
        Ok(false)
    }
    fn set_assert_latch(&mut self, value: bool) -> Result<()> {
        self.latch = value;
        Ok(())
    }
    fn member_facts(&self, member: ActorId) -> Result<MemberFacts<String>> {
        let m = self.member(member);
        let definition = s(&m["def"]);
        Ok(MemberFacts {
            definition_flags: self.fx.actor_flags[&definition].0,
            definition,
            gtid: u(&m["gtid"]),
            has_body: m["body"].as_bool().unwrap(),
            mover_speed: m["speed"].as_i64().map(|v| v as u32),
            has_experience: m["experience"].as_bool().unwrap(),
        })
    }
    fn element_set_organization(
        &mut self,
        _org: &mut OrganizationState<String>,
        member: ActorId,
        slot: i32,
    ) -> Result<()> {
        let l = self.label(member);
        self.record(json!({"seam": "5705A0", "this": format!("{l}.Element"), "owner": "owner", "slot": slot}));
        Ok(())
    }
    fn body_set_organization(&mut self, member: ActorId) -> Result<()> {
        let l = self.label(member);
        self.record(json!({"seam": "561760", "this": format!("{l}.Body"), "owner": "owner"}));
        Ok(())
    }
    fn owner_experience_token(&mut self) -> Result<u32> {
        let r = u(&self.spec["ced10"]);
        self.record(json!({"seam": "4CED10", "this": "owner", "result": r}));
        Ok(r)
    }
    fn set_experience_source(&mut self, member: ActorId, token: u32) -> Result<()> {
        let l = self.label(member);
        self.record(json!({"seam": "4CEBB0", "this": l, "zero": 0, "token": token}));
        Ok(())
    }
    fn combat_value(&mut self, _org: &mut OrganizationState<String>) -> Result<()> {
        self.record(json!({"seam": "5788B0", "this": "org"}));
        Ok(())
    }
    fn mind_speed_changed(&mut self, old_bits: u32, new_bits: u32) -> Result<()> {
        self.record(json!({"seam": "Mind+118", "old": old_bits, "new": new_bits}));
        Ok(())
    }
    fn member_flag_4d1080(&mut self, member: ActorId) -> Result<bool> {
        let r = self.member(member)["d1080"].as_bool().unwrap();
        let l = self.label(member);
        self.record(json!({"seam": "4D1080", "this": l, "result": r}));
        Ok(r)
    }
    fn set_owner_state(&mut self, state: u8, enabled: bool) -> Result<()> {
        self.record(json!({"seam": "4D64F0", "this": "owner", "state": state, "enabled": u8::from(enabled)}));
        Ok(())
    }
    fn set_leader(&mut self, o: &mut OrganizationState<String>, member: ActorId) -> Result<()> {
        let l = self.label(member);
        self.record(json!({"seam": "579A00", "this": "org", "member": l}));
        // The harness seam stores the leader (+40), as 579A00 does.
        o.words.insert(0x40, member.raw());
        Ok(())
    }
    fn mind_member_added(&mut self, member: ActorId) -> Result<()> {
        let l = self.label(member);
        self.record(json!({"seam": "524B80", "this": "owner.Mind", "member": l}));
        Ok(())
    }
    fn follow_leader(&mut self, _o: &mut OrganizationState<String>, member: ActorId) -> Result<()> {
        let l = self.label(member);
        self.record(json!({"seam": "57C800", "this": "org", "member": l}));
        Ok(())
    }
    fn owner_4d4cf0(&mut self, _org: &OrganizationState<String>) -> Result<()> {
        self.record(json!({"seam": "4D4CF0", "this": "owner"}));
        Ok(())
    }
    fn owner_4cf0f0(&mut self, _org: &mut OrganizationState<String>) -> Result<()> {
        self.record(json!({"seam": "4CF0F0", "this": "owner"}));
        Ok(())
    }
    fn owner_4d1350(&mut self, _org: &mut OrganizationState<String>) -> Result<()> {
        self.record(json!({"seam": "4D1350", "this": "owner"}));
        Ok(())
    }
    fn member_added_5784c0(
        &mut self,
        _o: &mut OrganizationState<String>,
        member: ActorId,
    ) -> Result<()> {
        let l = self.label(member);
        self.record(json!({"seam": "5784C0", "this": "org", "member": l}));
        Ok(())
    }
    fn definition_list_2d8(&self, actor_data: &String) -> Result<Vec<u32>> {
        Ok(self.lists[actor_data].clone())
    }
    fn apply_property(&mut self, property: u32, kind: u32, actor: ActorId) -> Result<()> {
        let l = self.label(actor);
        self.record(json!({"seam": "4CEF10", "this": "owner", "property": property, "kind": kind, "actor": l}));
        Ok(())
    }
    fn remove_property(&mut self, property: u32, actor: ActorId) -> Result<()> {
        let l = self.label(actor);
        self.record(json!({"seam": "4CEF70", "this": "owner", "property": property, "actor": l}));
        Ok(())
    }
    fn has_applied_property(&mut self, property: u32, actor: ActorId) -> Result<bool> {
        let r = self.spec["cef80"].as_bool().unwrap();
        let l = self.label(actor);
        self.record(json!({"seam": "4CEF80", "this": "owner", "property": property, "actor": l, "result": r}));
        Ok(r)
    }
    fn owner_object_notify(&mut self, object: u32) -> Result<()> {
        self.record(json!({"seam": "5147C0", "this": format!("{object:#x}")}));
        Ok(())
    }
    fn owner_kingdom(&mut self) -> Result<Option<u32>> {
        self.record(json!({"seam": "owner+108"}));
        Ok((!self.spec["owner"]["kingdom"].is_null()).then_some(KINGDOM))
    }
    fn recruit_single_count(&self, kingdom: u32) -> Result<i32> {
        assert_eq!(kingdom, KINGDOM);
        Ok(self.spec["owner"]["kingdom"]["singles"]
            .as_array()
            .unwrap()
            .len() as i32)
    }
    fn recruit_single_at(&self, _kingdom: u32, index: i32) -> Result<u32> {
        Ok(u(&self.spec["owner"]["kingdom"]["singles"][index as usize]))
    }
    fn find_recruit_single(&mut self, kingdom: u32, actor_data: &String) -> Result<u32> {
        assert_eq!(kingdom, KINGDOM);
        let r = self.spec["recruit_lookup"].get(actor_data).map_or(0, u);
        self.record(json!({"seam": "4E2A60", "actor": actor_data, "result": r}));
        Ok(r)
    }
    fn recruit_item_words(&self, item: u32) -> Result<[u32; 2]> {
        Ok(match self.spec["items"].get(item.to_string()) {
            Some(v) => [u(&v[0]), u(&v[1])],
            None => [0, 0],
        })
    }
    fn pair_recruit_item(&mut self, item: u32, member: ActorId) -> Result<()> {
        let l = self.label(member);
        self.record(json!({"seam": "4E1200", "this": format!("item:{item:#x}"), "member": l}));
        Ok(())
    }
    fn damage_type(&mut self, ids: &[u16]) -> Result<Option<u32>> {
        let name = String::from_utf16(ids).unwrap();
        let index = self.fx.damage.iter().position(|d| *d == name);
        self.record(json!({"seam": "48D500", "ids": name, "found": index.is_some()}));
        Ok(index.map(|i| 0xd000 + i as u32))
    }
    fn formation_facts(&self, formation: u32) -> Result<(u8, u32)> {
        Ok(self.fx.formations[(formation - 0xf000) as usize])
    }
    fn state_group4(&mut self) -> Result<Option<StateWords>> {
        let r = self.state_group.pop_front().unwrap_or(Value::Null);
        self.record(json!({"seam": "4D49B0", "group": 4, "result": r}));
        Ok((!r.is_null()).then(|| StateWords {
            word_20: u(&r[0]),
            word_24: u(&r[1]),
        }))
    }
    fn formation_refresh(&mut self, _o: &mut OrganizationState<String>) -> Result<()> {
        self.record(json!({"seam": "57BEF0", "this": "org"}));
        Ok(())
    }
    fn owner_object_164(&mut self) -> Result<Option<u32>> {
        let r = u(&self.spec["d1900"]);
        self.record(json!({"seam": "4D1900", "this": "owner", "result": r}));
        Ok((r != 0).then_some(r))
    }
    fn object_refresh(&mut self, object: u32) -> Result<()> {
        self.record(json!({"seam": "4B2830", "this": format!("{object:#x}")}));
        Ok(())
    }
}

fn precision(name: &str) -> Precision {
    match name {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        "Extended64" => Precision::Extended64,
        _ => panic!(),
    }
}
fn message(outcome: &Value) -> Message {
    let format: &'static str = Box::leak(s(&outcome["format"]).into_boxed_str());
    let args = outcome["args"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| match a {
            Value::String(t) => Arg::Text(t.clone()),
            v => Arg::Int(v.as_i64().unwrap() as i32),
        })
        .collect();
    Message { format, args }
}
fn expected_error(outcome: &Value) -> LoadError {
    match outcome["kind"].as_str().unwrap() {
        "Truncated" => LoadError::Truncated {
            at: outcome["at"].as_u64().unwrap() as usize,
            wanted: outcome["wanted"].as_u64().unwrap() as usize,
        },
        "ThrowException" => LoadError::Exception(message(outcome)),
        "Fatal" => LoadError::Fatal(message(outcome)),
        "Warning" => LoadError::Warning(message(outcome)),
        "Assertion" => LoadError::Assertion {
            line: u(&outcome["line"]),
            expression: Box::leak(s(&outcome["expression"]).into_boxed_str()),
        },
        "NullDereference" => LoadError::NullDereference(
            u32::from_str_radix(s(&outcome["pc"]).trim_start_matches("0x"), 16).unwrap(),
        ),
        k => panic!("unexpected outcome {k}"),
    }
}

fn compare(name: &str, native: &Value, o: &OrganizationState<String>, w: &World<'_>) {
    let ref_label = |raw: u32| {
        if raw == 0 {
            Value::Null
        } else {
            json!(w.label(ActorId::from_raw(raw)))
        }
    };
    let words = &native["words"];
    for (key, value) in words.as_object().unwrap() {
        let offset = u32::from_str_radix(key.trim_start_matches("0x"), 16).unwrap();
        let ours =
            org::native_word(o, offset).unwrap_or_else(|| panic!("{name}: word {key} missing"));
        let ours = match offset {
            0x40 | 0x44 => ref_label(ours),
            0x70 | 0x74 => {
                if ours == 0 {
                    Value::Null
                } else {
                    json!(format!("formation:{}", ours - 0xf000))
                }
            }
            0x98 => {
                if ours == 0 {
                    Value::Null
                } else {
                    json!(format!("damage:{}", w.fx.damage[(ours - 0xd000) as usize]))
                }
            }
            _ => json!(ours),
        };
        assert_eq!(&ours, value, "{name}: word {key}");
    }
    for (key, value) in native["bytes"].as_object().unwrap() {
        let offset = u32::from_str_radix(key.trim_start_matches("0x"), 16).unwrap();
        assert_eq!(json!(o.bytes[&offset]), *value, "{name}: byte {key}");
    }
    assert_eq!(native["scheduled_head"], 0);
    assert!(o.header.scheduled_head.is_none());
    if native["layout_definition"].is_null() {
        // 4C30C3 stores the null lookup result to Layout+8 before the 41DFB0
        // "Unknown IDS" throw; a LayoutDefinition cannot hold a null
        // definition, so the port keeps the previous one (recorded gap).
        assert!(
            name.starts_with("layout_unknown") || name.starts_with("truncated"),
            "{name}"
        );
    } else {
        assert_eq!(
            native["layout_definition"],
            json!(format!("layout:{}", o.layout.definition)),
            "{name}"
        );
    }
    let params: Vec<Value> = o
        .layout
        .parameters
        .iter()
        .map(|p| {
            p.as_ref()
                .map_or(Value::Null, |p| json!(format!("actor:{p}")))
        })
        .collect();
    assert_eq!(
        native["layout_parameters"],
        json!(params),
        "{name}: layout parameters"
    );
    let members: Vec<Value> = o.members.iter().map(|m| ref_label(m.raw())).collect();
    assert_eq!(native["members"], json!(members), "{name}: members");
    let items: Vec<Value> = o
        .words_34
        .iter()
        .map(|&h| {
            if h == 0 {
                Value::Null
            } else {
                json!(format!("item:{h:#x}"))
            }
        })
        .collect();
    assert_eq!(
        native["recruit_items"],
        json!(items),
        "{name}: recruit items"
    );
    assert_eq!(native["words_50"], json!(o.words_50), "{name}: +50");
    let offsets: Vec<[u32; 2]> = o.offsets_78.iter().map(|p| [p[0].0, p[1].0]).collect();
    assert_eq!(native["offsets_78"], json!(offsets), "{name}: +78");
}

fn fixture(doc: &Value) -> Fixture {
    let d = &doc["definitions"];
    Fixture {
        actor_flags: d["actor_data"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), (u(&v[0]), u(&v[1]))))
            .collect(),
        properties: serde_json::from_value(d["properties"].clone()).unwrap(),
        layouts: d["layouts"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
        formations: d["formations"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| (u(&f[0]) as u8, u(&f[1])))
            .collect(),
        damage: serde_json::from_value(d["damage_types"].clone()).unwrap(),
    }
}
fn world<'a>(fx: &'a Fixture, case: &'a Value) -> World<'a> {
    let spec = &case["input"];
    let mut labels: Vec<String> = spec["members"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    labels.sort();
    World {
        fx,
        spec,
        lists: case["native"]["definition_lists_2d8"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.as_array().unwrap().iter().map(u).collect()))
            .collect(),
        labels,
        pending: spec["slots"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e[0].as_i64().unwrap() >= 0)
            .map(|e| s(&e[1]))
            .collect(),
        state_group: spec["state_group"]
            .as_array()
            .unwrap()
            .iter()
            .cloned()
            .collect(),
        modifier_results: spec["modifier_results"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap())
            .collect(),
        latch: false,
        registered: None,
        calls: Vec::new(),
    }
}
/// Constructor 579BD0 (aa_actors::ground_states, proven by
/// research/phase7-actors/component_constructors.py) and the owner facts.
fn construct(spec: &Value, w: &World<'_>) -> (OrganizationState<String>, OwnerFacts<String>) {
    let owner = &spec["owner"];
    let default_name = s(&spec["default_layout"][0]);
    let default_params: Vec<Option<String>> = spec["default_layout"][1]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p.as_str().filter(|p| !p.is_empty()).map(str::to_owned))
        .collect();
    let default = LayoutDefinition {
        slots: w.layout_info(&default_name).unwrap().slot_count(),
        definition: default_name,
        parameters: default_params,
    };
    let position = [F32(u(&owner["position"][0])), F32(u(&owner["position"][1]))];
    let o = OrganizationState::new(
        OWNER,
        default.clone(),
        F32(u(&spec["time_e0"])),
        F32(u(&owner["breakpoint_308"])),
        position,
    )
    .unwrap();
    let facts = OwnerFacts {
        definition: s(&owner["def"]),
        gtid: u(&owner["gtid"]),
        has_mind: owner["mind"].as_bool().unwrap(),
        object_10: owner["objects_10"].as_i64().map(|v| v as u32),
        default_layout: default,
    };
    (o, facts)
}
fn payload(case: &Value) -> Vec<u8> {
    let h = s(&case["payload"]);
    (0..h.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&h[i..i + 2], 16).unwrap())
        .collect()
}
fn load_doc() -> Value {
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(FIXTURE).unwrap()).unwrap();
    assert_eq!(doc["passed"], true);
    doc
}

#[test]
fn organization_constructor_and_load_match_original() {
    let doc = load_doc();
    let fx = fixture(&doc);
    let mut counts = BTreeMap::<String, usize>::new();
    for case in doc["cases"].as_array().unwrap() {
        let name = s(&case["name"]);
        let spec = &case["input"];
        let native = &case["native"];
        let payload = payload(case);
        let mut world = world(&fx, case);
        let (mut o, owner_facts) = construct(spec, &world);
        compare(&format!("{name}/ctor"), &native["after_ctor"], &o, &world);
        let facts = WorldFacts {
            world_version: u(&spec["world_version"]),
            time_e4: u(&spec["time_e4"]),
        };
        let result = org::load(
            &mut o,
            &payload,
            &owner_facts,
            facts,
            &mut world,
            precision(&s(&spec["precision"])),
        );
        let kind = match (&native["outcome"], &result) {
            (Value::Null, Ok(consumed)) => {
                assert_eq!(
                    *consumed as u64,
                    native["consumed"].as_u64().unwrap(),
                    "{name}"
                );
                "ok".to_owned()
            }
            (outcome, Err(e)) if !outcome.is_null() => {
                assert_eq!(*e, expected_error(outcome), "{name}");
                s(&outcome["kind"])
            }
            (outcome, r) => panic!("{name}: native {outcome} rust {r:?}"),
        };
        *counts.entry(kind).or_default() += 1;
        assert_eq!(
            world.calls,
            native["calls"].as_array().unwrap().clone(),
            "{name}: seam sequence"
        );
        if native["state"].is_object() {
            compare(&name, &native["state"], &o, &world);
        }
        if let Some(slot) = native.get("table_slot") {
            let uid = spec["updatable"][1].as_i64().unwrap() as i32;
            assert_eq!(
                slot == "org+8",
                world.registered == Some(uid),
                "{name}: updatable table"
            );
        }
    }
    let total: usize = counts.values().sum();
    assert_eq!(total, doc["cases"].as_array().unwrap().len());
    assert!(
        counts["ok"] >= 90 && counts["Truncated"] >= 40,
        "{counts:?}"
    );
}

/// Not oracle parity: a saved slot index at or beyond the layout slot count
/// makes 57CBA0 store outside the native member array (heap overflow). The
/// port loads the nested record exactly as the original (before 57CBA0),
/// then refuses explicitly instead of inventing a result.
#[test]
fn organization_out_of_range_member_slot_is_refused() {
    let doc = load_doc();
    let fx = fixture(&doc);
    let case = doc["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "version10/Double53")
        .unwrap();
    let mut bytes = payload(case);
    // The slot word is the read just before the first nested record (9 bytes).
    let reads = case["native"]["reads"].as_array().unwrap();
    let nested = reads.iter().position(|r| r[1] == 9).unwrap();
    let at = reads[nested - 1][0].as_u64().unwrap() as usize;
    assert_eq!(u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()), 0);
    for slot in [4u32, 0x7fff_ffff] {
        bytes[at..at + 4].copy_from_slice(&slot.to_le_bytes());
        let mut w = world(&fx, case);
        let (mut o, owner) = construct(&case["input"], &w);
        let facts = WorldFacts {
            world_version: 27,
            time_e4: u(&case["input"]["time_e4"]),
        };
        let r = org::load(&mut o, &bytes, &owner, facts, &mut w, Precision::Double53);
        assert!(matches!(r, Err(LoadError::Unsupported(_))), "{r:?}");
        let last: Vec<&str> = w
            .calls
            .iter()
            .rev()
            .take(2)
            .map(|c| c["seam"].as_str().unwrap())
            .collect();
        assert_eq!(last, ["IsGActor", "4FAE80"]);
        assert!(o.members.iter().all(|m| m.is_null()));
    }
}
