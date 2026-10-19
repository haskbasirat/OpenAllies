//! Replays research/phase8-orders-input/action-mask-publication/
//! native-action-mask-publication.json (original 0x4CFCC0, 0x76C7F0,
//! 0x4ECD70, 0x50C8E0 with 0x50C7D0 and 0x4F5290 under Unicorn, both images,
//! PC24/PC53/PC64) through the Rust kernels: every row's event sequence,
//! every VWorldState's final +10..+1C words (including partial stores before
//! a native fault) and the fault/success outcome.
use super::*;
use serde_json::{Value, json};
use std::cell::RefCell;
use std::collections::BTreeMap;

const GARBAGE: [u32; 4] = [0xA5A5A5A5, 0x5A5A5A5A, 0xDEADBEEF, 0x01234567];

fn fixture() -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/phase8-orders-input/action-mask-publication/native-action-mask-publication.json");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn al(v: &Value) -> bool {
    v.as_u64().unwrap() & 0xff != 0
}
fn utf16(v: &Value) -> Vec<u16> {
    v.as_str().unwrap().encode_utf16().collect()
}
/// Labels map to stable small integers (ActorId raw for kingdoms/actors).
struct Labels(RefCell<Vec<String>>);
impl Labels {
    fn id(&self, label: &str) -> u32 {
        let mut v = self.0.borrow_mut();
        if let Some(i) = v.iter().position(|l| l == label) {
            return i as u32 + 1;
        }
        v.push(label.to_owned());
        v.len() as u32
    }
    fn opt(&self, label: &Value) -> Option<u32> {
        label.as_str().map(|l| self.id(l))
    }
    fn name(&self, id: u32) -> String {
        self.0.borrow()[id as usize - 1].clone()
    }
    fn name_opt(&self, id: Option<u32>) -> Value {
        id.map_or(Value::Null, |i| json!(self.name(i)))
    }
}

struct Lab<'c> {
    c: &'c Value,
    labels: Labels,
    events: RefCell<Vec<Value>>,
    masks: BTreeMap<String, [u32; 4]>,
}
impl<'c> Lab<'c> {
    fn new(c: &'c Value) -> Self {
        let masks = c["states"].as_object().unwrap().keys().map(|k| (k.clone(), GARBAGE)).collect();
        Self { c, labels: Labels(RefCell::new(Vec::new())), events: RefCell::new(Vec::new()), masks }
    }
    fn event(&self, v: Value) {
        self.events.borrow_mut().push(v);
    }
    fn thing(&self, id: u32) -> &'c Value {
        &self.c["things"][self.labels.name(id)]
    }
    fn actor(&self, id: ActorId) -> &'c Value {
        self.thing(id.raw())
    }
    fn a(&self, label: &Value) -> Option<ActorId> {
        self.labels.opt(label).map(ActorId::from_raw)
    }
    fn an(&self, a: Option<ActorId>) -> Value {
        self.labels.name_opt(a.map(ActorId::raw))
    }
    fn player(&self, p: PlayerRef) -> &'c Value {
        &self.c["players"][p.0 as usize]
    }
    fn rebuild_state(&mut self, state: &str) -> Result<()> {
        self.event(json!(["4ecd70", state]));
        let mut masks = self.masks[state];
        let result = selection_masks_4ecd70(&mut State { lab: self, state, masks: &mut masks });
        self.masks.insert(state.to_owned(), masks);
        result
    }
}

/// One VWorldState of the case.
struct State<'l, 'c> {
    lab: &'l Lab<'c>,
    state: &'l str,
    masks: &'l mut [u32; 4],
}
impl State<'_, '_> {
    fn s(&self) -> &Value {
        &self.lab.c["states"][self.state]
    }
    fn nodes(&self) -> &Vec<Value> {
        self.s()["nodes"].as_array().unwrap()
    }
    fn kind(&self, thing: u32) -> &str {
        self.lab.thing(thing)["type"].as_str().unwrap()
    }
}
impl SelectionMaskLeaves for State<'_, '_> {
    type Node = usize;
    type Thing = u32;
    type Kingdom = u32;
    type Research = u32;
    type Group = bool;
    fn kingdom_0(&mut self) -> Result<Option<u32>> {
        Ok(self.lab.labels.opt(&self.s()["kingdom"]))
    }
    fn head_4(&mut self) -> Result<Option<usize>> {
        Ok((!self.nodes().is_empty()).then_some(0))
    }
    fn tail_8(&mut self) -> Result<Option<usize>> {
        Ok((!self.nodes().is_empty() && self.s()["tail_null"] != json!(true)).then(|| self.nodes().len() - 1))
    }
    fn thing(&mut self, node: usize) -> Result<Option<u32>> {
        Ok(self.lab.labels.opt(&self.nodes()[node]))
    }
    fn next(&mut self, node: usize) -> Result<Option<usize>> {
        Ok((node + 1 < self.nodes().len()).then_some(node + 1))
    }
    fn masks(&mut self) -> &mut [u32; 4] {
        self.masks
    }
    fn non_group_3c(&mut self, index: u32) -> Result<bool> {
        let defs = self.lab.c["defs"].as_array().ok_or("[B36530] NULL")?;
        Ok(defs[index as usize].as_u64().unwrap() != 0)
    }
    fn virtual_40(&mut self, thing: u32, kingdom: Option<u32>, index: u32) -> Result<bool> {
        self.lab.event(json!(["vt40", self.lab.labels.name(thing), self.lab.labels.name_opt(kingdom), index]));
        Ok(match self.kind(thing) {
            "synthetic" => al(&self.lab.thing(thing)["v40_eax"][index as usize]),
            _ => false, // 0x887B50
        })
    }
    fn research_b4(&mut self, thing: u32) -> Result<Option<u32>> {
        assert_eq!(self.kind(thing), "synthetic");
        Ok(self.lab.labels.opt(&self.lab.thing(thing)["research"]))
    }
    fn research_available_5845a0(&mut self, research: u32) -> Result<bool> {
        let name = self.lab.labels.name(research);
        self.lab.event(json!(["5845a0", name]));
        Ok(al(&self.lab.c["researches"][name]))
    }
    fn virtual_20(&mut self, thing: u32, argument: u8) -> Result<bool> {
        self.lab.event(json!(["vt20", self.lab.labels.name(thing), argument]));
        Ok(match self.kind(thing) {
            "object" => false, // 0x969B10
            _ => al(&self.lab.thing(thing)["v20_eax"]),
        })
    }
    fn virtual_b4(&mut self, thing: u32) -> Result<bool> {
        self.lab.event(json!(["vtb4", self.lab.labels.name(thing)]));
        Ok(match self.kind(thing) {
            "object" => false, // 0x6A71F0
            "synthetic" => al(&self.lab.thing(thing)["vb4_eax"]),
            _ => true, // 0x6A71D0
        })
    }
    fn virtual_134(&mut self, thing: u32, kingdom: u32) -> Result<bool> {
        let k = self.lab.labels.name(kingdom);
        self.lab.event(json!(["vt134", self.lab.labels.name(thing), k]));
        let t = self.lab.thing(thing);
        Ok(match self.kind(thing) {
            "gactor" | "ggroup" => self.lab.labels.opt(&t["e8"]) == Some(kingdom), // 0x4D1E90
            "gkingdom" => thing == kingdom,                                         // 0x4E5240
            "synthetic" => t["v134_eax"].get(&k).is_some_and(al),
            _ => false, // 0x969B10
        })
    }
    fn group_20(&mut self) -> Result<Option<bool>> {
        Ok(self.s()["group"].as_u64().map(|b| b != 0))
    }
    fn group_byte_18(&mut self, group: bool) -> Result<bool> {
        Ok(group)
    }
}

impl OwnerViews for Lab<'_> {
    type Player = PlayerRef;
    fn players_bc(&mut self) -> Result<Vec<PlayerRef>> {
        Ok((0..self.c["players"].as_array().unwrap().len() as u32).map(PlayerRef).collect())
    }
    fn kingdom_765e00(&mut self, p: PlayerRef) -> Result<Option<ActorId>> {
        LocalKingdomReads::player_kingdom_765e00(&*self, p)
    }
    fn has_view_state_50(&mut self, p: PlayerRef) -> Result<bool> {
        Ok(!self.player(p)["view_state"].is_null())
    }
    fn rebuild_4ecd70(&mut self, p: PlayerRef) -> Result<()> {
        let state = self.player(p)["view_state"].as_str().unwrap().to_owned();
        self.rebuild_state(&state)
    }
}

impl LocalKingdomReads for Lab<'_> {
    fn application_state_e4(&self) -> Result<u32> {
        Ok(self.c["app_state"].as_u64().unwrap() as u32)
    }
    fn world_word_48(&self) -> Result<u32> {
        Ok(self.c["world"]["w48"].as_u64().unwrap() as u32)
    }
    fn map_settings_mode_a8(&self) -> Result<u32> {
        // 0x4F52D6: a NULL [GWorld+28] jumps to 0x4F52E9 exactly like mode 0.
        Ok(self.c["world"]["settings"].as_u64().unwrap_or(0) as u32)
    }
    fn local_player_76ac60(&self) -> Result<Option<PlayerRef>> {
        let Some(label) = self.c["local_player"].as_str() else { return Ok(None) };
        let i = self.c["players"].as_array().unwrap().iter().position(|p| p["label"] == label).unwrap();
        Ok(Some(PlayerRef(i as u32)))
    }
    fn player_name_24(&self, p: PlayerRef) -> Result<Vec<u16>> {
        Ok(utf16(&self.player(p)["name"]))
    }
    fn player_kingdom_765e00(&self, p: PlayerRef) -> Result<Option<ActorId>> {
        self.event(json!(["765e00", self.player(p)["label"]]));
        if self.c["world"].is_null() {
            return Ok(None);
        }
        Ok(self.a(&self.player(p)["kingdom"]))
    }
    fn world_kingdom_slots(&self) -> Result<Vec<Option<ActorId>>> {
        Ok(self.c["world"]["kingdom_slots"].as_array().unwrap().iter().map(|k| self.a(k)).collect())
    }
}

impl SelectedView for Lab<'_> {
    type View = u32;
    fn world_phase_d8(&mut self) -> Result<u32> {
        Ok(self.c["world"]["phase"].as_u64().unwrap() as u32)
    }
    fn thing_view_c(&mut self, actor: ActorId) -> Result<Option<u32>> {
        Ok(self.labels.opt(&self.actor(actor)["view"]))
    }
    fn view_flags_54(&mut self, view: u32) -> Result<u32> {
        Ok(self.c["views"][self.labels.name(view)]["flags"].as_u64().unwrap() as u32)
    }
    fn owner_108(&mut self, actor: ActorId) -> Result<Option<ActorId>> {
        self.event(json!(["vf108", self.labels.name(actor.raw())]));
        Ok(self.a(&self.actor(actor)["e8"]))
    }
    fn local_kingdom_4f5290(&mut self) -> Result<Option<ActorId>> {
        let names = |k: ActorId| -> Result<Vec<u16>> { Ok(utf16(&self.c["kingdoms"][self.labels.name(k.raw())])) };
        let k = local_kingdom_4f5290(&*self, &names, aa_numeric::Precision::Double53)?;
        self.event(json!(["4f5290", self.an(k)]));
        Ok(k)
    }
    fn ui_selection_8(&mut self) -> Result<Vec<u32>> {
        let ui = self.c["vworld"]["ui"].as_array().ok_or("UIWorldState NULL (0x50C7D0 faults)")?;
        let list: Vec<u32> = ui.iter().map(|v| self.labels.opt(v).unwrap()).collect();
        // The kernel's 0x50C7D0 then searches for the re-read Thing+C.
        let view = self.labels.opt(&self.actor(ActorId::from_raw(self.labels.id(self.c["call"]["actor"].as_str().unwrap())))["view"]);
        self.event(json!(["50c7d0", self.labels.name_opt(view), contains_50c7d0(&list, view) as u8]));
        Ok(list)
    }
    fn command_masks_437eb0(&mut self) -> Result<()> {
        self.event(json!(["437eb0"]));
        Ok(())
    }
}

fn world_error(e: String) -> aa_actors::ability_flags::Error {
    aa_actors::ability_flags::Error::World(e)
}
impl aa_actors::action_publication::World for Lab<'_> {
    fn element_organization(&self, a: ActorId) -> aa_actors::ability_flags::Result<Option<ActorId>> {
        Ok(self.actor(a).get("element").and_then(|e| e.get("organization")).and_then(|o| self.a(o)))
    }
    fn organization_owner(&self, o: ActorId) -> aa_actors::ability_flags::Result<ActorId> {
        let e = &self.c["call"]["actor"];
        let x = &self.c["things"][e.as_str().unwrap()]["element"];
        assert_eq!(self.a(&x["organization"]), Some(o));
        Ok(self.a(&x["owner"]).unwrap())
    }
    fn rebuild_actions(&mut self, a: ActorId) -> aa_actors::ability_flags::Result<()> {
        self.event(json!(["4d1350", self.labels.name(a.raw())]));
        Ok(())
    }
    fn owner(&mut self, a: ActorId) -> aa_actors::ability_flags::Result<Option<ActorId>> {
        self.owner_108(a).map_err(world_error)
    }
    fn object_byte_4c(&self, a: ActorId) -> aa_actors::ability_flags::Result<u8> {
        Ok(self.actor(a)["byte4c"].as_u64().unwrap() as u8)
    }
    fn refresh_owner_views(&mut self, owner: Option<ActorId>) -> aa_actors::ability_flags::Result<()> {
        self.event(json!(["76c7f0", self.an(owner)]));
        owner_views_76c7f0(self, owner).map_err(world_error)
    }
    fn renderer_flags(&self, a: ActorId) -> aa_actors::ability_flags::Result<Option<u32>> {
        Ok(self.labels.opt(&self.actor(a)["view"])
            .map(|v| self.c["views"][self.labels.name(v)]["flags"].as_u64().unwrap() as u32))
    }
    fn refresh_selected_view(&mut self, a: ActorId) -> aa_actors::ability_flags::Result<()> {
        self.event(json!(["50c8e0", self.labels.name(a.raw())]));
        selected_view_50c8e0(self, a).map_err(world_error)
    }
}

fn replay(c: &Value) -> (bool, Vec<Value>, BTreeMap<String, [u32; 4]>) {
    let mut lab = Lab::new(c);
    // Allocate ids in the case's label order so GTIDs are stable.
    for k in c["things"].as_object().unwrap().keys() {
        lab.labels.id(k);
    }
    let call = &c["call"];
    let ok = match c["suite"].as_str().unwrap() {
        "masks" => lab.rebuild_state(call["state"].as_str().unwrap()).is_ok(),
        "owners" => {
            let owner = lab.a(&call["owner"]);
            lab.event(json!(["76c7f0", lab.an(owner)]));
            owner_views_76c7f0(&mut lab, owner).is_ok()
        }
        "selected" => {
            let a = lab.a(&call["actor"]).unwrap();
            lab.event(json!(["50c8e0", lab.an(Some(a))]));
            selected_view_50c8e0(&mut lab, a).is_ok()
        }
        _ => {
            let a = lab.a(&call["actor"]).unwrap();
            aa_actors::action_publication::changed(a, &mut lab).is_ok()
        }
    };
    let events = lab.events.take();
    (ok, events, lab.masks)
}

#[test]
fn native_action_mask_publication_rows_replay() {
    let f = fixture();
    assert_eq!(f["identical_across_runs"], json!(true));
    assert_eq!(f["runs"].as_array().unwrap().len(), 6);
    let cases = f["cases"].as_array().unwrap();
    let rows = f["rows"].as_array().unwrap();
    assert_eq!(cases.len(), rows.len());
    let mut by_suite = BTreeMap::<(String, bool), usize>::new();
    for (c, row) in cases.iter().zip(rows) {
        let name = c["name"].as_str().unwrap();
        let (ok, events, masks) = replay(c);
        let native_ok = row["fault"].is_null();
        assert_eq!(ok, native_ok, "{name}: outcome");
        assert_eq!(Value::Array(events), row["events"], "{name}: events");
        for (state, words) in &masks {
            let native: Vec<u32> = row["masks"][state].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as u32).collect();
            assert_eq!(words.to_vec(), native, "{name}: {state} masks");
        }
        *by_suite.entry((c["suite"].as_str().unwrap().to_owned(), native_ok)).or_default() += 1;
    }
    // Every suite has successful rows; every fault class is represented.
    for suite in ["masks", "owners", "selected", "publish"] {
        assert!(by_suite.get(&(suite.to_owned(), true)).copied().unwrap_or(0) > 0, "{suite}");
    }
    assert!(by_suite[&("masks".to_owned(), false)] >= 4);
    assert!(by_suite[&("selected".to_owned(), false)] >= 2);
}

/// The kernels' native instruction coverage recorded by the producer.
#[test]
fn native_action_mask_publication_coverage() {
    let f = fixture();
    for name in ["4CFCC0", "529D70", "76C7F0", "765E00", "4ECD70", "50C8E0", "76AC60", "76AC70", "4BEDF0", "96CB83",
        "4D1E80", "4D1E90", "4E5240", "887B50", "6A71D0", "969B10"] {
        assert_eq!(f["unexecuted"][name], json!([]), "{name}");
    }
    // Unexecuted: __allshl's >= 64 path, alignment padding, the CString
    // clone/free slow paths (the laboratory strings stay referenced) and
    // GObject vt+0xB4 0x6A71F0 (its vt+0x20 0x969B10 is false first).
    assert_eq!(f["unexecuted"]["96D710"], json!(["0x96d72a", "0x96d72c", "0x96d72e"]));
    assert_eq!(f["unexecuted"]["50C7D0"], json!(["0x50c7dd"]));
    assert_eq!(f["unexecuted"]["4F5290"], json!(["0x4f534b", "0x4f534d", "0x4f534f", "0x4f5350"]));
    assert_eq!(f["unexecuted"]["6A71F0"], json!(["0x6a71f0", "0x6a71f2"]));
}

/// 0x887B50 is vt+0x40 of every bound Thing primary vtable, so a real
/// selection's phase 1 always ends with zero words; with distinct things a
/// NULL group leaves all four words zero.
#[test]
fn real_selection_phase_one_is_zero() {
    let f = fixture();
    for (c, row) in f["cases"].as_array().unwrap().iter().zip(f["rows"].as_array().unwrap()) {
        if c["suite"] != "masks" || !row["fault"].is_null() {
            continue;
        }
        let state = &c["states"]["S1"];
        let real = state["nodes"].as_array().unwrap().iter()
            .all(|t| t.as_str().is_some_and(|t| c["things"][t]["type"] != "synthetic"));
        if real && !state["nodes"].as_array().unwrap().is_empty() {
            let words = &row["masks"]["S1"];
            assert_eq!(words[0], json!(0), "{}", c["name"]);
            assert_eq!(words[2], json!(0), "{}", c["name"]);
            let high = (words[1].as_u64().unwrap() as u32, words[3].as_u64().unwrap() as u32);
            assert!(matches!(high, (0, 0) | (0x20, 0x20) | (0x40, 0x40)), "{}", c["name"]);
        }
    }
}
