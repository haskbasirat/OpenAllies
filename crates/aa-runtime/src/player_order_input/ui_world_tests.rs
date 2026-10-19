//! Replays research/phase8-orders-input/selection/native-selection.json: the
//! ORIGINAL game UIWorldState code run on test objects whose virtual slots
//! are recorded seams. The mock below answers every leaf from the same case
//! description and records the same events; state, results and the complete
//! event sequence (every virtual call, request, sound and refresh) must
//! match for every operation at every precision.
use super::*;
use crate::player_order_input::requests::Requests;
use serde_json::{json, Value};

fn precision(name: &str) -> Precision {
    match name {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        "Extended64" => Precision::Extended64,
        other => panic!("precision {other}"),
    }
}
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn w3(v: &Value) -> [u32; 3] {
    let a = v.as_array().unwrap();
    [u(&a[0]), u(&a[1]), u(&a[2])]
}
fn idx(v: &Value) -> Option<usize> {
    v.as_u64().map(|x| x as usize)
}

struct Mock<'a> {
    case: &'a Value,
    p: Precision,
    flags: Vec<u32>,
    events: Vec<Value>,
    find: Vec<Value>,
    brush_8: bool,
    vworld_140: bool,
    requests: Rec,
}
#[derive(Default)]
struct Rec {
    events: Vec<Value>,
    actor_view: Vec<Option<usize>>,
}
impl Requests<usize> for Rec {
    fn add_436ad0(&mut self, view: usize, player: Option<u32>) -> Result<()> {
        assert!(player.is_none());
        self.events.push(json!(["request", "add", self.actor_view[view], true]));
        Ok(())
    }
    fn remove_436b90(&mut self, view: usize, player: Option<u32>) -> Result<()> {
        assert!(player.is_none());
        self.events.push(json!(["request", "remove", self.actor_view[view], true]));
        Ok(())
    }
    fn clear_436c50(&mut self, player: Option<u32>) -> Result<()> {
        assert!(player.is_none());
        self.events.push(json!(["request", "clear", true]));
        Ok(())
    }
    fn save_group_438180(&mut self, name: &[u16], player: Option<u32>) -> Result<()> {
        assert!(player.is_none());
        self.events.push(json!(["request", "save_group", String::from_utf16_lossy(name), true]));
        Ok(())
    }
}
impl Mock<'_> {
    fn v(&self, i: usize) -> &Value {
        &self.case["views"][i]
    }
    fn a(&self, i: usize) -> Option<&Value> {
        let a = &self.v(i)["actor"];
        (!a.is_null()).then_some(a)
    }
    fn ab(&self, i: usize, name: &str) -> bool {
        self.a(i).unwrap()[name].as_bool().unwrap()
    }
    fn vflag(&mut self, i: usize, name: &str) -> Result<bool> {
        self.flush();
        self.events.push(json!(["view", name, i]));
        Ok(self.v(i)[name].as_bool().unwrap())
    }
    fn aflag(&mut self, i: usize, name: &str) -> Result<bool> {
        self.flush();
        self.events.push(json!(["actor", name, i]));
        Ok(self.ab(i, name))
    }
    /// Requests record into their own buffer; keep one ordered stream.
    fn flush(&mut self) {
        self.events.append(&mut self.requests.events);
    }
    fn org(&self, i: usize) -> Option<usize> {
        self.a(i).and_then(|a| idx(&a["org"]))
    }
}
impl Leaves for Mock<'_> {
    type View = usize;
    fn precision(&self) -> Precision {
        self.p
    }
    fn flags_54(&mut self, v: usize) -> Result<u32> {
        Ok(self.flags[v])
    }
    fn set_flags_54(&mut self, v: usize, f: u32) -> Result<()> {
        self.flags[v] = f;
        Ok(())
    }
    fn position_38(&mut self, v: usize) -> Result<[u32; 3]> {
        Ok(w3(&self.v(v)["pos"]))
    }
    fn field_10_58cb30(&mut self, v: usize) -> Result<u64> {
        Ok(self.v(v)["field10"].as_u64().unwrap())
    }
    fn pickable_9c(&mut self, v: usize) -> Result<bool> {
        self.vflag(v, "pickable")
    }
    fn class_78(&mut self, v: usize) -> Result<bool> {
        self.vflag(v, "class_78")
    }
    fn class_84(&mut self, v: usize) -> Result<bool> {
        self.vflag(v, "class_84")
    }
    fn class_7c(&mut self, v: usize) -> Result<bool> {
        self.vflag(v, "class_7c")
    }
    fn class_80(&mut self, v: usize) -> Result<bool> {
        self.vflag(v, "class_80")
    }
    fn virtual_50(&mut self, v: usize) -> Result<bool> {
        self.vflag(v, "virtual_50")
    }
    fn virtual_54(&mut self, v: usize) -> Result<bool> {
        self.vflag(v, "virtual_54")
    }
    fn actor_present_14(&mut self, v: usize) -> Result<bool> {
        Ok(self.a(v).is_some())
    }
    fn actor_45b7a0(&mut self, v: usize) -> Result<bool> {
        if self.a(v).is_none() {
            return Ok(false);
        }
        self.aflag(v, "valid_b4")
    }
    fn actor_flag8_bit0(&mut self, v: usize) -> Result<bool> {
        Ok(u(&self.a(v).unwrap()["flags8"]) & 1 != 0)
    }
    fn actor_virtual_58(&mut self, v: usize) -> Result<bool> {
        self.aflag(v, "virtual_58")
    }
    fn actor_virtual_84(&mut self, v: usize) -> Result<bool> {
        self.aflag(v, "virtual_84")
    }
    fn actor_4d12c0(&mut self, v: usize) -> Result<bool> {
        let a = self.a(v).unwrap();
        let f100 = a.get("f100").map(u).unwrap_or(0);
        let f104 = a.get("f104").map(u).unwrap_or(0);
        if u(&a["flags8"]) >> 29 & 1 != 0 {
            return Ok(true);
        }
        let d10a0 = f104 & 4 == 0 && f100 & 0x0400_0000 == 0 && f104 & 0x10 == 0 && f100 & 0x0200_0000 == 0;
        Ok(!d10a0)
    }
    fn actor_virtual_20(&mut self, v: usize) -> Result<bool> {
        self.aflag(v, "virtual_20")
    }
    fn actor_70(&mut self, v: usize) -> Result<bool> {
        Ok(self.ab(v, "f70"))
    }
    fn actor_90(&mut self, v: usize) -> Result<bool> {
        Ok(self.ab(v, "f90"))
    }
    fn link_16c_view(&mut self, v: usize) -> Result<Option<usize>> {
        Ok(idx(&self.a(v).unwrap()["link16c"]))
    }
    fn link_94_view(&mut self, v: usize) -> Result<Option<usize>> {
        Ok(self.a(v).unwrap()["link94"].as_i64().filter(|x| *x >= 0).map(|x| x as usize))
    }
    fn link_94_live(&mut self, v: usize) -> Result<bool> {
        Ok(!self.a(v).unwrap()["link94"].is_null())
    }
    fn hidden_4549c0(&mut self, v: usize) -> Result<bool> {
        match self.org(v) {
            Some(org) => Ok(!self.aflag(org, "virtual_58")?),
            None => Ok(false),
        }
    }
    fn actor_view_0c(&mut self, v: usize) -> Result<Option<usize>> {
        let own = self.a(v).unwrap().get("own_view").and_then(Value::as_bool).unwrap_or(true);
        Ok(own.then_some(v))
    }
    fn root_view_4ce6c0(&mut self, v: usize) -> Result<Option<usize>> {
        let f7c = self.ab(v, "f7c");
        match self.org(v) {
            Some(org) if !f7c => self.actor_view_0c(org),
            _ => self.actor_view_0c(v),
        }
    }
    /// Test organization components carry no voiced record (+0xA4 = 0).
    fn voiced_436600(&mut self, _: usize) -> Result<Option<Option<usize>>> {
        Ok(None)
    }
    fn actor_virtual_150(&mut self, _: usize, _: &mut aa_orders::commands::CommandWire) -> Result<bool> {
        unreachable!("not in this oracle")
    }
    /// 0x436850 runs natively with the acknowledgement option on: its
    /// 0x45B7A0, then the 0x42A590 voice with the view's actor.
    fn selection_sound_436850(&mut self, v: usize) -> Result<()> {
        self.actor_45b7a0(v)?;
        self.flush();
        let actor = self.requests.actor_view[v];
        self.events.push(json!(["sound", actor]));
        Ok(())
    }
    fn command_masks_437eb0(&mut self, _: &[usize], _: StaticRef, _: &mut [u32; 4]) -> Result<()> {
        self.flush();
        self.events.push(json!(["masks"]));
        Ok(())
    }
    fn brush_changed_42d4d0(&mut self) -> Result<()> {
        self.brush_8 = true;
        Ok(())
    }
    fn vworld_list_changed_4325c0(&mut self) -> Result<()> {
        self.vworld_140 = true;
        Ok(())
    }
    fn vworld_ad(&mut self) -> Result<bool> {
        Ok(self.case.get("vworld_ad").map(u).unwrap_or(1) != 0)
    }
    fn floater_remove_42eb10(&mut self, _: usize) -> Result<()> {
        Ok(())
    }
    fn floater_update_453470(&mut self, v: usize) -> Result<()> {
        self.flush();
        self.events.push(json!(["floater", v]));
        Ok(())
    }
    fn requests(&mut self) -> &mut dyn Requests<usize> {
        self.flush();
        &mut self.requests
    }
}
impl Host for Mock<'_> {
    fn find_object_at_43d240(&mut self, _: [u32; 3], _: [u32; 3], hits: Option<&mut Vec<usize>>) -> Result<Option<usize>> {
        self.flush();
        let spec = if self.find.is_empty() { json!({"hits": [], "best": null}) } else { self.find.remove(0) };
        self.events.push(json!(["find", 0, hits.is_some()]));
        if let Some(list) = hits {
            list.clear();
            for h in spec.get("hits").and_then(Value::as_array).cloned().unwrap_or_default() {
                list.insert(0, h.as_u64().unwrap() as usize);
            }
        }
        Ok(idx(&spec["best"]))
    }
    fn box_frustum_436dd0(&mut self, _: &super::super::event::Event, _: &super::super::event::Event) -> Result<()> {
        Ok(())
    }
    fn box_frustum_4371a0(&mut self, _: [i32; 4]) -> Result<()> {
        unreachable!("not in this oracle")
    }
    fn box_inside_4367a0(&mut self, v: usize) -> Result<bool> {
        if self.v(v).get("proximity").map(u).unwrap_or(2) != 2 {
            return Ok(false);
        }
        if !self.pickable_9c(v)? {
            return Ok(false);
        }
        self.events.push(json!(["frustum", v]));
        Ok(self.v(v).get("inside").and_then(Value::as_bool).unwrap_or(false))
    }
    fn vworld_list_94(&mut self) -> Result<Vec<usize>> {
        let n = self.case["views"].as_array().unwrap().len();
        Ok(match self.case.get("grid") {
            Some(g) => g.as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as usize).collect(),
            None => (0..n).collect(),
        })
    }
    fn pick_under_cursor_43dc70(&mut self, _: i32, _: i32) -> Result<Option<usize>> {
        unreachable!("not in this oracle")
    }
    fn hover_enabled_5c1e70(&mut self) -> Result<bool> {
        Ok(true)
    }
    /// UI+0xC4 holds the real default state: vt+0x24 = 0x5E05D0.
    fn choose_5c1970(&mut self, a: Option<usize>, da: u32, b: Option<usize>, db: u32) -> Result<Option<usize>> {
        super::super::states::choose_5e05d0(self, a, da, b, db)
    }
    /// vt+0x18 = 0x60CA00 -> 0x45B470.
    fn state_pick_filter_18(&mut self, v: usize) -> Result<Option<bool>> {
        Ok(Some(State::<usize>::selectable_45b470(self, v)?))
    }
}

fn snapshot(state: &State<usize>, mock: &Mock<'_>) -> Value {
    let static_ref = match state.static_40() {
        StaticRef::None => json!("none"),
        StaticRef::Wildcard => json!("wildcard"),
        StaticRef::Group(i) => json!(i),
    };
    let groups: Vec<Value> = state
        .groups()
        .iter()
        .map(|g| json!({"name": String::from_utf16_lossy(&g.name), "members": g.members, "byte_18": g.byte_18}))
        .collect();
    let links: Vec<Value> = (0..mock.flags.len())
        .map(|v| match state.group_of_150(v) {
            None => Value::Null,
            Some(id) => match state.groups().iter().position(|g| g.id == id) {
                Some(i) => json!(i),
                None => json!("stale"),
            },
        })
        .collect();
    json!({
        "groups": groups,
        "links": links,
        "selected": state.selected(),
        "tail": state.selected().last(),
        "hovered": state.hovered_10(),
        "candidates": state.candidates_14(),
        "changed_1c": u8::from(state.changed_1c()),
        "static_40": static_ref,
        "class_50": state.candidate_class_50(),
        "class_54": state.selection_class_54(),
        "flags": mock.flags,
        "brush_8": u8::from(mock.brush_8),
        "vworld_140_40": u8::from(mock.vworld_140),
    })
}

fn event(spec: &Value) -> super::super::event::Event {
    super::super::event::Event { x: 0, y: 0, flags: 0, code: 0, delta: 0, world: w3(&spec["world"]), origin: [0; 3], direction: [0; 3] }
}

#[test]
fn selection_manager_matches_native_rows() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-orders-input/selection/native-selection.json");
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let mut operations = 0;
    for row in doc["rows"].as_array().unwrap() {
        let case = &row["case"];
        let p = precision(row["precision"].as_str().unwrap());
        let views = case["views"].as_array().unwrap();
        let mut mock = Mock {
            case,
            p,
            flags: views.iter().map(|v| u(&v["flags"])).collect(),
            events: Vec::new(),
            find: case["find"].as_array().cloned().unwrap_or_default(),
            brush_8: false,
            vworld_140: false,
            requests: Rec { events: Vec::new(), actor_view: views.iter().enumerate().map(|(i, v)| (!v["actor"].is_null()).then_some(i)).collect() },
        };
        let mut state = State::<usize>::from_constructor_640640();
        if let Some(groups) = case.get("groups") {
            for g in groups.as_array().unwrap() {
                let members: Vec<usize> = g["members"].as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as usize).collect();
                let id = state.groups().len() as u64 + 1000;
                let name: Vec<u16> = g["name"].as_str().unwrap().encode_utf16().collect();
                // The harness writes view+0x150 group by group: the last wins.
                for m in &members {
                    state.links_150.retain(|(v, _)| v != m);
                    state.links_150.push((*m, id));
                }
                let mut group = StaticGroup::new(id, name, members);
                group.byte_18 = g.get("byte_18").and_then(Value::as_bool).unwrap_or(false);
                state.groups_mut().push(group);
            }
        }
        for step in row["native"].as_array().unwrap() {
            let op = step["op"].as_array().unwrap();
            mock.events.clear();
            let name = op[0].as_str().unwrap();
            let b = |i: usize| u(&op[i]) != 0;
            let i = |k: usize| op[k].as_u64().unwrap() as usize;
            let result: Value = match name {
                "select" => state.select(&mut mock, i(1), b(2)).map(|_| Value::Null),
                "clear" => state.clear(&mut mock, b(1)).map(|_| Value::Null),
                "deselect" => state.deselect_438df0(&mut mock, i(1)).map(|_| Value::Null),
                "add" => state.add_438370(&mut mock, i(1), b(2)).map(|_| Value::Null),
                "click" => state.click_640cf0(&mut mock, w3(&op[1]), w3(&op[2]), b(3)).map(|_| Value::Null),
                "click_add" => state.click_add_640e10(&mut mock, w3(&op[1]), w3(&op[2]), b(3)).map(|_| Value::Null),
                "click_hits" => {
                    let hits: Vec<usize> = op[2].as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as usize).collect();
                    state.click_hits_437b20(&mut mock, w3(&op[1]), &hits, idx(&op[3])).map(|_| Value::Null)
                }
                "select_list" => {
                    let list: Vec<usize> = op[1].as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as usize).collect();
                    state.select_list_640fa0(&mut mock, &list).map(|_| Value::Null)
                }
                "box" => state.box_select_4377b0(&mut mock, &event(&op[1]), &event(&op[2])).map(|_| Value::Null),
                "compatible" => state.compatible_640b20(&mut mock, i(1)).map(|r| json!(u8::from(r))),
                "pick_target" => State::pick_target_6409a0(&mut mock, Some(i(1))).map(|r| json!(r)),
                "select_owner" => State::select_owner_640b40(&mut mock, Some(i(1))).map(|r| json!(r)),
                "selectable" => State::selectable_640660(&mut mock, Some(i(1))).map(|r| json!(u8::from(r))),
                "class" => State::class_6406c0(&mut mock, i(1), u(&op[2])).map(|r| json!(r)),
                "filter" => State::pick_filter_5c19e0(&mut mock, Some(i(1))).map(|r| json!(u8::from(r))),
                "choose" => mock.choose_5c1970(idx(&op[1]), u(&op[2]), idx(&op[3]), u(&op[4])).map(|r| json!(r)),
                "save_group" => {
                    let name: Vec<u16> = op[1].as_str().unwrap().encode_utf16().collect();
                    state.save_group_439e90(&mut mock, &name).map(|_| Value::Null)
                }
                "select_group" => {
                    let name: Vec<u16> = op[1].as_str().unwrap().encode_utf16().collect();
                    state.select_group_4376c0(&mut mock, &name).map(|_| Value::Null)
                }
                "hover" => {
                    state.hovered = idx(&op[1]);
                    Ok(Value::Null)
                }
                other => panic!("op {other}"),
            }
            .unwrap_or_else(|e| panic!("{} {:?}: {e}", case["name"], op));
            mock.flush();
            let ctx = format!("{} {} {:?}", case["name"], row["precision"], op);
            assert_eq!(result, step["result"], "result {ctx}");
            assert_eq!(Value::Array(mock.events.clone()), step["events"], "events {ctx}");
            assert_eq!(snapshot(&state, &mock), step["state"], "state {ctx}");
            operations += 1;
        }
    }
    assert!(operations >= 600, "{operations}");
}
