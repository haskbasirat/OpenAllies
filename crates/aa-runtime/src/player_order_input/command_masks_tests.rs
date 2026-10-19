//! Replays research/phase8-orders-input/command-masks/native-command-masks.json
//! (original 0x437EB0, 0x5845A0 and vt+0x154 under Unicorn, both images,
//! PC24/PC53/PC64, identical) through the kernels of this module.
use super::*;
use serde_json::{Value, json};

fn fixture() -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/phase8-orders-input/command-masks/native-command-masks.json");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn al(v: &Value) -> bool {
    u(v) & 0xff != 0
}
fn names(v: &Value) -> Vec<String> {
    let mut out: Vec<String> = v.as_object().unwrap().keys().cloned().collect();
    out.sort();
    out
}

/// The 0x437EB0 laboratory memory of one case, answered through the trait.
struct Masks<'a> {
    case: &'a Value,
    things: Vec<String>,
    research: Vec<String>,
    events: Vec<Value>,
}
impl Masks<'_> {
    fn thing(&self, t: usize) -> &Value {
        &self.case["things"][&self.things[t]]
    }
    fn synthetic(&self, t: usize) -> bool {
        self.thing(t)["kind"] == "actor"
    }
    fn index(&self, name: &str) -> usize {
        self.things.iter().position(|n| n == name).unwrap()
    }
    fn kingdom(k: Option<u32>) -> u32 {
        k.unwrap_or(0)
    }
}
impl MaskLeaves for Masks<'_> {
    type View = usize;
    type Object = usize;
    type Organization = usize;
    type Kingdom = u32;
    type Research = usize;
    fn thing_14(&mut self, view: usize) -> Result<Option<usize>> {
        Ok(self.case["views"][view].as_str().map(|n| self.index(n)))
    }
    fn virtual_b4(&mut self, t: usize) -> Result<bool> {
        if !self.synthetic(t) {
            return Ok(false); // A2A2A0 +B4 = 6A71F0 (native)
        }
        self.events.push(json!(["b4", self.things[t]]));
        Ok(al(&self.thing(t)["b4"]))
    }
    fn organization_7c(&mut self, t: usize) -> Result<Option<usize>> {
        Ok((!self.thing(t)["org"].is_null()).then_some(t))
    }
    fn state_100(&mut self, t: usize) -> Result<u32> {
        Ok(u(&self.thing(t)["state"][0]))
    }
    fn organization_8c(&mut self, o: usize) -> Result<bool> {
        Ok(u(&self.thing(o)["org"]["b8c"]) & 0xff != 0)
    }
    fn non_group_3c(&mut self, index: u32) -> Result<Option<bool>> {
        Ok(self.case["definitions"][index as usize].as_u64().map(|b| b & 0xff != 0))
    }
    fn local_kingdom_5c0e30(&mut self) -> Result<Option<u32>> {
        self.events.push(json!(["5c0e30"]));
        let k = u(&self.case["kingdom_5c0e30"]);
        Ok((k != 0).then_some(k))
    }
    fn virtual_154(&mut self, t: usize, kingdom: Option<u32>, index: u32, selected: u8) -> Result<bool> {
        self.events.push(json!(["154", self.things[t], Self::kingdom(kingdom), index, selected]));
        let thing = self.thing(t);
        let can = thing["can154"].as_array().unwrap().iter().any(|i| u(i) == index);
        Ok(al(&thing["eax154"][usize::from(can)]))
    }
    fn research_b4(&mut self, t: usize) -> Result<Option<usize>> {
        if !self.synthetic(t) {
            return Ok(None); // the Object's +B4 word is 0
        }
        Ok(self.thing(t)["research"].as_str().map(|r| self.research.iter().position(|n| n == r).unwrap()))
    }
    fn research_available_5845a0(&mut self, r: usize) -> Result<bool> {
        self.events.push(json!(["5845a0", self.research[r]]));
        Ok(al(&self.case["research"][&self.research[r]]))
    }
    fn members_28(&mut self, o: usize) -> Result<Vec<Option<usize>>> {
        let members = self.thing(o)["org"]["members"].as_array().unwrap().clone();
        Ok(members.iter().map(|m| m.as_str().map(|n| self.index(n))).collect())
    }
    fn virtual_20(&mut self, t: usize, argument: u8) -> Result<bool> {
        if !self.synthetic(t) {
            return Ok(false); // A2A2A0 +20 = 969B10 (native)
        }
        self.events.push(json!(["20", self.things[t], argument]));
        Ok(al(&self.thing(t)["v20"]))
    }
    fn virtual_134(&mut self, t: usize, kingdom: u32) -> Result<bool> {
        self.events.push(json!(["134", self.things[t], kingdom]));
        Ok(al(&self.thing(t)["v134"][kingdom.to_string()]))
    }
}

#[test]
fn native_437eb0_rows_replay() {
    let f = fixture();
    assert_eq!(f["identical_across_runs"], true);
    assert_eq!(f["runs"].as_array().unwrap().len(), 6);
    let rows = f["masks"].as_array().unwrap();
    assert_eq!(rows.len(), 900);
    let (mut faults, mut retreat, mut research) = (0, 0, 0);
    for (n, row) in rows.iter().enumerate() {
        let case = &row["case"];
        let native = &row["native"];
        let mut lab = Masks { case, things: names(&case["things"]), research: names(&case["research"]), events: vec![] };
        let selected: Vec<usize> = case["selected"].as_array().unwrap().iter().map(|v| u(v) as usize).collect();
        let k = u(&case["kingdom_4c"]);
        let group = case["group18"].as_u64().map(|b| b & 0xff != 0);
        let mut masks = [0x5A5A_5A5A; 4];
        let result = command_masks_in_place_437eb0(&mut lab, &selected, (k != 0).then_some(k), group, &mut masks);
        assert_eq!(json!(lab.events), native["events"], "row {n} events");
        match &native["fault"] {
            Value::Null => {
                let words: Vec<u32> = native["masks"].as_array().unwrap().iter().map(u).collect();
                assert_eq!(result, Ok(()), "row {n}");
                assert_eq!(masks.to_vec(), words, "row {n} masks");
                assert_eq!(command_masks_437eb0(&mut Masks { events: vec![], ..lab }, &selected, (k != 0).then_some(k), group).unwrap(), masks);
                if all_set(&masks) & !(1 << RETREAT | 1 << ROUT | 3 << GROUP_BEHAVIOR_ENABLE) == 0 && all_set(&masks) & (1 << RETREAT | 1 << ROUT) != 0 {
                    retreat += 1;
                }
                if any_set(&masks) >> RESEARCH & 1 != 0 {
                    research += 1;
                }
            }
            fault => {
                faults += 1;
                let pc = fault["null"][0][0].as_str().unwrap().trim_start_matches("0x").to_ascii_uppercase();
                let error = result.unwrap_err();
                assert!(error.contains(&format!("0x{pc}")), "row {n}: {error} vs {pc}");
            }
        }
    }
    assert_eq!(faults, 48);
    assert!(retreat > 50 && research > 50, "{retreat} {research}");
}

/// 0x5845A0 laboratory: component +4 owner +4 ActorData +558 and +10 lists.
struct Research<'a> {
    case: &'a Value,
    items: Vec<String>,
    events: Vec<Value>,
}
impl Research<'_> {
    fn list(&self, key: &str) -> Vec<usize> {
        self.case[key].as_array().unwrap().iter().map(|n| self.items.iter().position(|i| i == n.as_str().unwrap()).unwrap()).collect()
    }
}
impl ResearchLeaves for Research<'_> {
    type Research = ();
    type Item = usize;
    fn items_558(&mut self, _: ()) -> Result<Vec<usize>> {
        Ok(self.list("items_558"))
    }
    fn blocked_5844a0(&mut self, _: (), item: usize, notify: u32) -> Result<u32> {
        self.events.push(json!(["5844a0", self.items[item], notify]));
        Ok(u(&self.case["blocked"][&self.items[item]]))
    }
    fn listed_10(&mut self, _: ()) -> Result<Vec<usize>> {
        Ok(self.list("listed_10"))
    }
}

#[test]
fn native_5845a0_rows_replay() {
    let f = fixture();
    let rows = f["research"].as_array().unwrap();
    assert_eq!(rows.len(), 240);
    for (n, row) in rows.iter().enumerate() {
        let case = &row["case"];
        assert!(row["native"]["fault"].is_null());
        let mut lab = Research { case, items: names(&case["blocked"]), events: vec![] };
        let available = research_available_5845a0(&mut lab, ()).unwrap();
        assert_eq!(json!(lab.events), row["native"]["events"], "row {n}");
        assert_eq!(u8::from(available), u(&row["native"]["al"]) as u8, "row {n}");
    }
}

/// One vt+0x154 row: Actor+0xE8 from memory, vt+0x20(1) / vt+0x34(index)
/// answered by the separately measured native calls on the same object.
struct Permission<'a> {
    row: &'a Value,
    kingdoms: Vec<u32>,
    events: Vec<Value>,
}
impl PermissionLeaves for Permission<'_> {
    type Actor = ();
    type Kingdom = u32;
    fn owner_e8(&mut self, _: ()) -> Result<Option<u32>> {
        let owner = u(&self.row["case"]["actors"]["a"]["owner"]);
        let raw = if (owner as usize) < self.kingdoms.len() { self.kingdoms[owner as usize] } else { owner };
        Ok((raw != 0).then_some(raw))
    }
    fn virtual_20(&mut self, _: (), argument: u8) -> Result<bool> {
        self.events.push(json!(["4cf120", "a", argument]));
        let measured = &self.row["native"]["vt20"];
        assert!(measured["fault"].is_null());
        Ok(u(&measured["al"]) != 0)
    }
    fn virtual_34(&mut self, _: (), index: u32) -> Result<bool> {
        self.events.push(json!(["4d1f90", "a", index]));
        Ok(u(&self.row["native"]["vt34"]["al"]) != 0)
    }
}

#[test]
fn native_vt154_rows_replay() {
    let f = fixture();
    for (vtable, slots) in f["vtable_pins"].as_object().unwrap() {
        if vtable != "Object" {
            assert_eq!(slots["0x154"], "0x4ce680", "{vtable}");
            assert_eq!(slots["0x20"], "0x4cf120", "{vtable}");
            assert_eq!(slots["0x34"], "0x4d1f90", "{vtable}");
            assert_eq!(slots["0xb4"], "0x6a71d0", "{vtable}");
        }
    }
    let kingdoms: Vec<u32> = f["kingdoms"].as_array().unwrap().iter().map(u).collect();
    let rows = f["v154"].as_array().unwrap();
    assert_eq!(rows.len(), 720);
    let mut granted = std::collections::BTreeMap::<String, u32>::new();
    for (n, row) in rows.iter().enumerate() {
        let args: Vec<u32> = row["case"]["args"].as_array().unwrap().iter().map(u).collect();
        let call = &row["native"]["call"];
        assert!(call["fault"].is_null(), "row {n}");
        let mut lab = Permission { row, kingdoms: kingdoms.clone(), events: vec![] };
        let kingdom = (args[0] != 0).then_some(args[0]);
        let result = virtual_154_4ce680(&mut lab, (), kingdom, args[1], args[2] as u8).unwrap();
        assert_eq!(u8::from(result), u(&call["al"]) as u8, "row {n}");
        let top: Vec<&Value> = call["events"].as_array().unwrap().iter()
            .filter(|e| (e[0] == "4cf120" || e[0] == "4d1f90") && e[1] == "a").collect();
        assert_eq!(json!(lab.events), json!(top), "row {n}");
        // vt+0x34 = 0x4D1F90 is the existing F0/F4 bit test.
        let bits = row["case"]["actors"]["a"]["bits"].as_u64().unwrap();
        assert_eq!(crate::player_order_world::native_command_bit(bits, args[1]), u(&row["native"]["vt34"]["al"]) != 0, "row {n}");
        if result {
            *granted.entry(row["case"]["actors"]["a"]["vtable"].as_str().unwrap().to_string()).or_default() += 1;
        }
    }
    assert_eq!(granted.len(), 3, "{granted:?}");
}

#[test]
fn coverage_is_complete_on_reachable_paths() {
    let f = fixture();
    let c = &f["coverage"];
    for (name, total) in [("45B7A0", 16), ("96D710", 15), ("5845A0", 38), ("4CE680", 26), ("4CF120", 45), ("4D1F90", 15), ("6A71F0", 2), ("969B10", 2)] {
        assert_eq!((u(&c[name]["executed"]), u(&c[name]["instructions"])), (total, total), "{name}");
    }
    // Unexecuted: dead `head == NULL` re-tests and the inconsistent tail
    // (+0C NULL with a non-NULL head) the UIWorldState list never holds.
    assert_eq!(c["437EB0"]["unexecuted"], json!(["0x437f3f", "0x437f4c", "0x4380dd", "0x4380ea"]));
    // Unexecuted: the filter-byte-1 Element test (437EB0 pushes byte 0).
    assert_eq!(c["4364C0"]["unexecuted"], json!(["0x4364e6", "0x4364ec", "0x4364ef", "0x4364f1"]));
}
