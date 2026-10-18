use super::*;
use serde_json::{Value, json};
use std::collections::BTreeMap;
struct Fields {
    handle: u32,
    words: BTreeMap<u32, u32>,
}
impl ViewFields<u32> for Fields {
    fn handle(&self) -> u32 {
        self.handle
    }
    fn word(&mut self, offset: u32) -> Result<u32> {
        self.words
            .get(&offset)
            .copied()
            .ok_or_else(|| "missing lab field".into())
    }
    fn store(&mut self, offset: u32, value: u32) -> Result<()> {
        *self.words.get_mut(&offset).ok_or("missing lab store")? = value;
        Ok(())
    }
    fn residual_occupancy_warning(&mut self, _: i32) -> Result<()> {
        Ok(())
    } //Declared native logger transport, not an actual game diagnostic binding.
}
struct Probe<'a> {
    bound: Bound<'a, u32, Fields>,
    stop: Option<&'a str>,
    events: Vec<String>,
    counts: BTreeMap<String, u32>,
}
impl Probe<'_> {
    fn event(&mut self, label: String) -> Result<()> {
        let n = self.counts.entry(label.clone()).or_default();
        let key = format!("{label}:{n}");
        *n += 1;
        if self.stop == Some(key.as_str()) {
            return Err(key);
        }
        self.events.push(key);
        Ok(())
    }
}
impl World for Probe<'_> {
    type View = u32;
    type Node = NodeId;
    fn view(&self) -> u32 {
        self.bound.view()
    }
    fn view_word(&mut self, o: u32) -> Result<u32> {
        self.event(format!("vread{o:x}"))?;
        self.bound.view_word(o)
    }
    fn view_store(&mut self, o: u32, v: u32) -> Result<()> {
        self.event(format!("vstore{o:x}"))?;
        self.bound.view_store(o, v)
    }
    fn grid_word(&mut self, o: u32) -> Result<u32> {
        self.bound.grid_word(o)
    }
    fn head(&mut self, c: usize) -> Result<Option<NodeId>> {
        self.bound.head(c)
    }
    fn tail(&mut self, c: usize) -> Result<Option<NodeId>> {
        self.bound.tail(c)
    }
    fn head_store(&mut self, c: usize, n: Option<NodeId>) -> Result<()> {
        self.event("head".into())?;
        self.bound.head_store(c, n)
    }
    fn tail_store(&mut self, c: usize, n: Option<NodeId>) -> Result<()> {
        self.event("tail".into())?;
        self.bound.tail_store(c, n)
    }
    fn allocate(&mut self) -> Result<NodeId> {
        self.event("allocate".into())?;
        self.bound.allocate()
    }
    fn node_view(&mut self, n: NodeId) -> Result<u32> {
        self.bound.node_view(n)
    }
    fn node_next(&mut self, n: NodeId) -> Result<Option<NodeId>> {
        self.bound.node_next(n)
    }
    fn node_view_store(&mut self, n: NodeId, v: u32) -> Result<()> {
        self.event("node_view".into())?;
        self.bound.node_view_store(n, v)
    }
    fn node_next_store(&mut self, n: NodeId, v: Option<NodeId>) -> Result<()> {
        self.event("node_next".into())?;
        self.bound.node_next_store(n, v)
    }
    fn free(&mut self, n: NodeId) -> Result<()> {
        self.event("free".into())?;
        self.bound.free(n)
    }
    fn residual_warning(&mut self, c: i32) -> Result<()> {
        self.event("warning".into())?;
        self.bound.residual_warning(c)
    }
}
fn fields(v: &Value, handle: u32) -> Fields {
    Fields {
        handle,
        words: [
            (0x38, "x"),
            (0x3c, "y"),
            (0x74, "r"),
            (0x54, "flags"),
            (0x68, "count"),
        ]
        .into_iter()
        .map(|(o, k)| (o, v[k].as_u64().unwrap() as u32))
        .collect(),
    }
}
fn adjust(op: &Value, fields: &mut Fields) {
    for (offset, key) in [(0x38, "x"), (0x3c, "y"), (0x74, "r")] {
        if let Some(bits) = op[key].as_u64() {
            fields.words.insert(offset, bits as u32);
        }
    }
}
fn call(state: &mut State<u32>, fields: &mut [Fields], op: &Value, pc: Precision) -> Result<()> {
    let index = op["view"].as_u64().unwrap() as usize;
    let view = &mut fields[index];
    adjust(op, view);
    if op["op"] == "occupy" {
        state.occupy(view, pc)
    } else {
        state.unoccupy(view, pc)
    }
}
#[test]
fn original_v14_shared_grid_lists_pose_flags_counts_and_all_store_prefixes() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-view-grid/native-grid-occupancy.json"
    ))
    .unwrap();
    struct LabGlobals;
    impl Globals for LabGlobals {
        fn word(&mut self, _: u32) -> Result<u32> {
            Ok(0)
        }
    }
    let rows = fixture["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 693);
    for (ordinal, row) in rows.iter().enumerate() {
        let c = &row["input"];
        let pc = match row["pc"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            _ => Precision::Extended64,
        };
        let mut state = State::<u32>::from_constructor(&mut LabGlobals).unwrap();
        state
            .initialize_bits([0x42000000, 0x41c00000], 2, pc)
            .unwrap();
        let mut views: Vec<_> = c["views"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, v)| fields(v, i as u32 + 1))
            .collect();
        for op in c["before"].as_array().unwrap() {
            call(&mut state, &mut views, op, pc).unwrap();
        }
        let op = &c["call"];
        let index = op["view"].as_u64().unwrap() as usize;
        adjust(op, &mut views[index]);
        let mut probe = Probe {
            bound: Bound {
                state: &mut state,
                view: &mut views[index],
            },
            stop: row["stop"].as_str(),
            events: Vec::new(),
            counts: BTreeMap::new(),
        };
        let result = if op["op"] == "occupy" {
            occupy(&mut probe, pc)
        } else {
            unoccupy(&mut probe, pc)
        };
        let events = std::mem::take(&mut probe.events);
        drop(probe);
        let cells: Vec<_> = state
            .cells
            .as_ref()
            .unwrap()
            .iter()
            .map(|c| json!([c.head.map(|n| n.0 + 1), c.tail.map(|n| n.0 + 1)]))
            .collect();
        let nodes:Vec<_>=state.nodes.iter().map(|n|match n {
            None=>Value::Null,Some(n)=>json!({"view":n.view,"next":match n.next {None=>json!("unknown"),Some(next)=>json!(next.map(|id|id.0+1))}})
        }).collect();
        let free: Vec<_> = state.free_nodes.iter().map(|n| n.0 + 1).collect();
        let actual = json!({"events":events,"stopped":result.err(),"cells":cells,"nodes":nodes,"free":free,
            "views":views.iter().map(|f|json!({"flags":f.words[&0x54],"count":f.words[&0x68]})).collect::<Vec<_>>()});
        assert_eq!(actual, row["output"], "native row {ordinal} {op}");
    }
}

#[test]
fn required_residual_diagnostic_retains_same_grid_and_view_prefix() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-view-grid/native-grid-occupancy.json"
    ))
    .unwrap();
    let row = fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| {
            r["stop"] == "warning:0"
                && r["pc"] == "Double53"
                && r["input"]["before"].as_array().is_some_and(|ops| {
                    ops.len() == 2 && ops.iter().all(|o| o["op"] == "occupy" && o["view"] == 0)
                })
        })
        .unwrap();
    struct NoWarning(Fields);
    impl ViewFields<u32> for NoWarning {
        fn handle(&self) -> u32 {
            self.0.handle()
        }
        fn word(&mut self, o: u32) -> Result<u32> {
            self.0.word(o)
        }
        fn store(&mut self, o: u32, v: u32) -> Result<()> {
            self.0.store(o, v)
        }
    }
    struct LabGlobals;
    impl Globals for LabGlobals {
        fn word(&mut self, _: u32) -> Result<u32> {
            Ok(0)
        }
    }
    let c = &row["input"];
    let expected = &row["output"];
    let mut state = State::from_constructor(&mut LabGlobals).unwrap();
    state
        .initialize_bits([0x42000000, 0x41c00000], 2, Precision::Double53)
        .unwrap();
    let mut views: Vec<_> = c["views"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(i, v)| fields(v, i as u32 + 1))
        .collect();
    for op in c["before"].as_array().unwrap() {
        call(&mut state, &mut views, op, Precision::Double53).unwrap();
    }
    let mut actual = NoWarning(views.remove(0));
    assert!(
        state
            .unoccupy(&mut actual, Precision::Double53)
            .unwrap_err()
            .contains("diagnostic")
    );
    views.insert(0, actual.0);
    let cells: Vec<_> = state
        .cells
        .as_ref()
        .unwrap()
        .iter()
        .map(|c| json!([c.head.map(|n| n.0 + 1), c.tail.map(|n| n.0 + 1)]))
        .collect();
    let nodes:Vec<_>=state.nodes.iter().map(|n|match n {None=>Value::Null,Some(n)=>json!({"view":n.view,"next":match n.next{None=>json!("unknown"),Some(n)=>json!(n.map(|v|v.0+1))}})}).collect();
    assert_eq!(json!(cells), expected["cells"]);
    assert_eq!(json!(nodes), expected["nodes"]);
    assert_eq!(
        json!(state.free_nodes.iter().map(|n| n.0 + 1).collect::<Vec<_>>()),
        expected["free"]
    );
    assert_eq!(
        json!(
            views
                .iter()
                .map(|v| json!({"flags":v.words[&0x54],"count":v.words[&0x68]}))
                .collect::<Vec<_>>()
        ),
        expected["views"]
    );
}
