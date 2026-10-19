use super::*;
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn f(v: &Value) -> u32 {
    (v.as_f64().unwrap() as f32).to_bits()
}
///The oracle's case as the same reads; every virtual appends its trace.
struct Case<'c> {
    c: &'c Value,
    names: Vec<String>,
    words: BTreeMap<(usize, u32), u32>,
    trace: Vec<Value>,
}
impl<'c> Case<'c> {
    fn new(c: &'c Value) -> Self {
        let names: Vec<String> = c["views"].as_object().unwrap().keys().cloned().collect();
        let mut words = BTreeMap::new();
        for (i, name) in names.iter().enumerate() {
            let v = &c["views"][name];
            for (axis, value) in v["pos"].as_array().unwrap().iter().enumerate() {
                words.insert((i, 0x38 + 4 * axis as u32), f(value));
            }
            words.insert((i, 0x74), f(v.get("radius").unwrap_or(&json!(0.0))));
            for (offset, key, default) in [(0x18, "f18", 0), (0x1c, "f1c", 0), (0x54, "f54", 0x0100_0000), (0x58, "f58", 0)] {
                words.insert((i, offset), v[key].as_u64().unwrap_or(default) as u32);
            }
        }
        Self { c, names, words, trace: Vec::new() }
    }
    fn id(&self, name: &str) -> usize {
        self.names.iter().position(|n| n == name).unwrap()
    }
    fn view(&self, v: usize) -> &'c Value {
        &self.c["views"][&self.names[v]]
    }
    fn event(&mut self, call: &str, v: usize) -> serde_json::Map<String, Value> {
        let mut e = serde_json::Map::new();
        e.insert("call".into(), json!(call));
        e.insert("view".into(), json!(self.names[v]));
        e
    }
}
impl Views for Case<'_> {
    type V = usize;
    fn word(&mut self, v: usize, offset: u32) -> Result<u32> {
        self.words.get(&(v, offset)).copied().ok_or_else(|| format!("+{offset:X} unwritten"))
    }
    fn store(&mut self, v: usize, offset: u32, value: u32) -> Result<()> {
        assert!(matches!(offset, 0x18 | 0x1c | 0x54 | 0x58), "{offset:#x}");
        self.words.insert((v, offset), value);
        Ok(())
    }
    fn kind(&mut self, v: usize) -> Result<ViewKind> {
        Ok(match self.view(v)["type"].as_str().unwrap() {
            "object" => ViewKind::Object,
            "actor" => ViewKind::Actor,
            _ => ViewKind::Organization,
        })
    }
    fn attached_vfx_9c(&mut self, _: usize) -> Result<bool> {
        Ok(false)
    }
    fn model_74_78(&mut self, v: usize) -> Result<[u32; 2]> {
        let m = self.view(v).get("model").cloned().unwrap_or(json!([0.0, 0.0]));
        Ok([f(&m[0]), f(&m[1])])
    }
    fn height_431550(&mut self, xy: [u32; 2]) -> Result<Binary> {
        self.trace.push(json!({"call": "height_431550", "xy": xy}));
        let views = self.c["views"].as_object().unwrap();
        let v = views.values().find(|v| [f(&v["pos"][0]), f(&v["pos"][1])] == xy).unwrap();
        Ok(Binary::from_f32_bits(f(&v["height"])).unwrap())
    }
    fn grid_word(&mut self, offset: u32) -> Result<u32> {
        Ok(match offset {
            0x1c => self.c["grid"]["columns"].as_u64().unwrap() as u32,
            0x24 => self.c["grid"]["shift"].as_u64().unwrap() as u32,
            _ => unreachable!(),
        })
    }
    fn cell_views(&mut self, index: usize) -> Result<Vec<usize>> {
        Ok(self.c["cells"][index.to_string()].as_array().map_or(vec![], |a| a.iter().map(|n| self.id(n.as_str().unwrap())).collect()))
    }
    fn update_0(&mut self, v: usize) -> Result<()> {
        let e = self.event("update_0", v);
        self.trace.push(Value::Object(e));
        Ok(())
    }
    fn model_request_d0(&mut self, v: usize, near: u32) -> Result<()> {
        let mut e = self.event("model_request_d0", v);
        e.insert("arg".into(), json!(near));
        self.trace.push(Value::Object(e));
        Ok(())
    }
    fn model_release_d4(&mut self, v: usize) -> Result<()> {
        let e = self.event("model_release_d4", v);
        self.trace.push(Value::Object(e));
        Ok(())
    }
    fn show_e0(&mut self, v: usize) -> Result<()> {
        let e = self.event("show_e0", v);
        self.trace.push(Value::Object(e));
        Ok(())
    }
    fn hide_e4(&mut self, v: usize) -> Result<()> {
        let e = self.event("hide_e4", v);
        self.trace.push(Value::Object(e));
        Ok(())
    }
    fn data_request_14(&mut self, v: usize, delta: u32, mode: u32) -> Result<()> {
        let mut e = self.event("data_request_14", v);
        e.insert("delta".into(), json!(delta));
        e.insert("mode".into(), json!(mode));
        self.trace.push(Value::Object(e));
        Ok(())
    }
    fn data_release_18(&mut self, v: usize, delta: u32) -> Result<()> {
        let mut e = self.event("data_release_18", v);
        e.insert("delta".into(), json!(delta));
        self.trace.push(Value::Object(e));
        Ok(())
    }
}

#[test]
fn native_view_streaming_replay() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../../research/skirmish-20261019-view-streaming/native-view-streaming.json")).unwrap();
    let mut rows = 0;
    for row in fixture["rows"].as_array().unwrap() {
        let (c, o) = (&row["input"], &row["output"]);
        let p = match row["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            _ => Precision::Extended64,
        };
        let mut w = Case::new(c);
        let mut lists = Lists::from_constructor();
        for (key, list) in [("on", List::On), ("near", List::Near)] {
            for name in c[key].as_array().map_or(vec![], |a| a.clone()) {
                lists.append_45b4c0(w.id(name.as_str().unwrap()), list);
            }
        }
        let mut stats = Stats::default();
        if let Some(initial) = c["stats"].as_array() {
            for (i, v) in initial.iter().enumerate() {
                stats.words[i] = v.as_u64().unwrap() as u32;
            }
        }
        let planes: [[u32; 4]; 6] = std::array::from_fn(|i| std::array::from_fn(|j| f(&c["planes"][i][j])));
        for frame in c["frames"].as_array().unwrap() {
            for b in frame["blocks"].as_array().unwrap() {
                let block = TerrainBlockCell {
                    column_ac: b["column"].as_i64().unwrap() as i32,
                    row_b0: b["row"].as_i64().unwrap() as i32,
                    state_b8: b["state"].as_u64().unwrap() as u32,
                    size_a0: c["block_size"].as_i64().unwrap() as i32,
                };
                sweep_42fad0(&mut lists, &mut w, block, &planes, p).unwrap();
            }
            if frame["traverse"] != false {
                traverse_42f670(&mut lists, &mut stats, &mut w).unwrap();
            }
        }
        for (i, name) in w.names.clone().iter().enumerate() {
            let expect = &o["views"][name];
            for (offset, key) in [(0x18, "f18"), (0x1c, "f1c"), (0x54, "f54"), (0x58, "f58")] {
                assert_eq!(json!(w.words[&(i, offset)]), expect[key], "{name} {key} {c}");
            }
            let list = match lists.list_of(i) {
                Some(List::On) => json!("on"),
                Some(List::Near) => json!("near"),
                None => Value::Null,
            };
            assert_eq!(list, expect["list"], "{name} {c}");
        }
        let names = |l: List| json!(lists.head_first(l).iter().map(|v| w.names[*v].clone()).collect::<Vec<_>>());
        assert_eq!(names(List::On), o["on"], "{c}");
        assert_eq!(names(List::Near), o["near"], "{c}");
        assert_eq!(json!(stats.words), o["stats"], "{c}");
        assert_eq!(json!(w.trace), o["trace"], "virtual order {c}");
        rows += 1;
    }
    assert_eq!(rows, 252);
}
