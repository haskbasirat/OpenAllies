use super::*;
use serde_json::{Value, json};
const FIXTURE: &str = include_str!(
    "../../../research/skirmish-20261019-data-release-ready/native-data-release-ready.json"
);
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
struct Trace(Vec<Value>);
impl Trace {
    fn call(&mut self, name: impl Into<String>, args: Vec<u32>) {
        self.0.push(json!({"call":name.into(),"args":args}));
    }
}
struct Fx<'a> {
    c: &'a Value,
    t: Trace,
}
impl FxNodes for Fx<'_> {
    type Node = usize;
    fn creates_vfx(&mut self, n: &usize) -> Result<bool> {
        Ok(matches!(self.c["nodes"][*n].as_str().unwrap(), "vfx" | "both"))
    }
    fn plays_sfx(&mut self, n: &usize) -> Result<bool> {
        Ok(matches!(self.c["nodes"][*n].as_str().unwrap(), "sfx" | "both"))
    }
    fn release_vfx(&mut self, n: &usize, d: u32) -> Result<()> {
        self.t.call(format!("node{n}_vfx"), vec![d]);
        Ok(())
    }
    fn vfx_ready(&mut self, n: &usize) -> Result<bool> {
        self.t.call(format!("node{n}_vfx"), vec![]);
        Ok(self.c["false_at"] != json!([n, "vfx"]))
    }
    fn release_sfx(&mut self, n: &usize, d: u32) -> Result<()> {
        self.t.call(format!("node{n}_sfx"), vec![d]);
        Ok(())
    }
    fn sfx_ready(&mut self, n: &usize) -> Result<bool> {
        self.t.call(format!("node{n}_sfx"), vec![]);
        Ok(self.c["false_at"] != json!([n, "sfx"]))
    }
}
struct Aspect<'a> {
    c: &'a Value,
    t: Trace,
    count: u32,
    extra: bool,
}
impl AspectRelease for Aspect<'_> {
    type Resource = String;
    fn count(&mut self) -> Result<u32> {
        Ok(self.count)
    }
    fn store_count(&mut self, v: u32) -> Result<()> {
        self.count = v;
        Ok(())
    }
    fn model(&mut self) -> Result<String> {
        Ok("model".into())
    }
    fn release(&mut self, r: &String, d: u32) -> Result<()> {
        self.t.call(r.clone(), vec![d]);
        Ok(())
    }
    fn loaded(&mut self, r: &String) -> Result<bool> {
        Ok(if r == "model" { self.c["model"] == "loaded" } else { self.c["unloaded"] != r.as_str() })
    }
    fn release_animations(&mut self, d: u32) -> Result<()> {
        let entries = self.entries();
        set_release(&entries, d, |r, d| self.release(r, d))
    }
    fn animations_ready(&mut self) -> Result<bool> {
        let entries = self.entries();
        let c = self.c;
        set_ready(&entries, |r| Ok(c["unloaded"] != r.as_str()))
    }
    fn skin(&mut self, i: usize) -> Result<Option<String>> {
        Ok((self.c["skins"][i] == true).then(|| format!("skin{i}")))
    }
    fn extra_requested(&mut self) -> Result<bool> {
        Ok(self.extra)
    }
    fn clear_extra_requested(&mut self) -> Result<()> {
        self.extra = false;
        Ok(())
    }
    fn extra_count(&mut self, _: &String) -> Result<i32> {
        Ok(self.c["extra_count"].as_i64().unwrap() as i32)
    }
    fn extra(&mut self, i: usize) -> Result<String> {
        Ok(format!("extra{i}"))
    }
}
impl Aspect<'_> {
    fn entries(&self) -> Vec<Option<String>> {
        match self.c["set"].as_array() {
            None => vec![],
            Some(v) => v.iter().enumerate().map(|(i, p)| (p == true).then(|| format!("set{i}"))).collect(),
        }
    }
}
struct Animation<'a> {
    c: &'a Value,
    t: Trace,
}
impl AnimationRelease for Animation<'_> {
    type Resource = String;
    type List = String;
    fn resource(&mut self) -> Result<Option<String>> {
        Ok((!self.c["resource"].is_null()).then(|| "sequence".into()))
    }
    fn release(&mut self, r: &String, d: u32) -> Result<()> {
        self.t.call(r.clone(), vec![d]);
        Ok(())
    }
    fn loaded(&mut self, _: &String) -> Result<bool> {
        Ok(self.c["resource"] == "loaded")
    }
    fn lists(&mut self) -> Result<Vec<String>> {
        let mut v: Vec<String> = (0..3).map(|i| format!("list{i}")).collect();
        v.extend((0..self.c["timed"].as_u64().unwrap()).map(|i| format!("timed{i}")));
        Ok(v)
    }
    fn release_list(&mut self, l: &String, d: u32) -> Result<()> {
        self.t.call(l.clone(), vec![d]);
        Ok(())
    }
    fn list_ready(&mut self, l: &String) -> Result<bool> {
        self.t.call(l.clone(), vec![]);
        Ok(self.c["false_at"] != l.as_str())
    }
}
fn run(c: &Value) -> Value {
    let ready = c["ready"] == true;
    let delta = u(&c["delta"]);
    let value = |r: Result<bool>| json!(u8::from(r.unwrap()));
    match c["kernel"].as_str().unwrap() {
        "fx_list" => {
            let mut w = Fx { c, t: Trace(vec![]) };
            let nodes: Vec<usize> = (0..c["nodes"].as_array().unwrap().len()).collect();
            let v = if ready { value(fx_list_ready(&mut w, &nodes)) } else { fx_list_release(&mut w, &nodes, delta).unwrap(); Value::Null };
            json!({"status":"ok","value":v,"trace":w.t.0})
        }
        "ability" => {
            let mut t = vec![];
            let v = if ready {
                value(ability_ready(|off| {
                    let i = (off - 0x70) / 8;
                    t.push(json!({"call":format!("list{i}"),"offset":off,"args":[]}));
                    Ok(c["false_at"] != json!(i))
                }))
            } else {
                ability_release(delta, |off, d| {
                    t.push(json!({"call":format!("list{}",(off-0x70)/8),"offset":off,"args":[d]}));
                    Ok(())
                })
                .unwrap();
                Value::Null
            };
            json!({"status":"ok","value":v,"trace":t})
        }
        "vfx" => {
            struct V<'a> { c: &'a Value, count: u32, t: Trace }
            impl VfxRelease for V<'_> {
                type Resource = String;
                fn release_base(&mut self, d: u32) -> Result<()> { self.count = self.count.wrapping_sub(d); Ok(()) }
                fn requested(&mut self) -> Result<bool> { Ok((self.count as i32) > 0) }
                fn resource40(&mut self) -> Result<Option<String>> { Ok((!self.c["resource"].is_null()).then(|| "resource40".into())) }
                fn release(&mut self, r: &String, d: u32) -> Result<()> { self.t.call(r.clone(), vec![d]); Ok(()) }
                fn loaded(&mut self, _: &String) -> Result<bool> { Ok(self.c["resource"] == "loaded") }
            }
            let entry = c["entry"].as_str().unwrap();
            let class = matches!(entry, "494450" | "494810");
            let mut w = V { c, count: u(&c["initial"]), t: Trace(vec![]) };
            let v = if matches!(entry, "4943a0" | "494810") {
                value(vfx_ready(&mut w, class))
            } else {
                vfx_release(&mut w, class, delta).unwrap();
                Value::Null
            };
            json!({"status":"ok","value":v,"trace":w.t.0,"count":w.count})
        }
        "audio" => {
            let lists: Vec<Vec<(usize, usize)>> = c["counts"].as_array().unwrap().iter().enumerate()
                .map(|(li, n)| (0..n.as_u64().unwrap() as usize).map(|j| (li, j)).collect()).collect();
            let lists = [&lists[0][..], &lists[1][..], &lists[2][..]];
            let mut t = Trace(vec![]);
            let v = if ready {
                value(audio_ready(lists, |(li, j)| Ok(c["unloaded"] != json!([li, j]))))
            } else {
                audio_release(lists, delta, |(li, j), d| { t.call(format!("sound{li}_{j}"), vec![d]); Ok(()) }).unwrap();
                Value::Null
            };
            json!({"status":"ok","value":v,"trace":t.0})
        }
        "aspect" => {
            let mut w = Aspect { c, t: Trace(vec![]), count: u(&c["count"]), extra: c["extra"] == true };
            let v = if ready { value(aspect_ready(&mut w)) } else { aspect_release(&mut w, delta).unwrap(); Value::Null };
            json!({"status":"ok","value":v,"trace":w.t.0,"count":w.count,"extra":u8::from(w.extra)})
        }
        "banner" => {
            let planes: Vec<Vec<Vec<String>>> = c["shape"].as_array().unwrap().iter().enumerate().map(|(pi, p)| {
                p.as_array().unwrap().iter().enumerate().map(|(di, n)| (0..n.as_u64().unwrap()).map(|t| format!("texture{pi}_{di}_{t}")).collect()).collect()
            }).collect();
            let mut t = Trace(vec![]);
            let v = if ready {
                value(banner_ready(&planes, |r| {
                    let [pi, di, ti]: [u64; 3] = r.trim_start_matches("texture").split('_').map(|x| x.parse().unwrap()).collect::<Vec<_>>().try_into().unwrap();
                    Ok(c["unloaded"] != json!([pi, di, ti]))
                }))
            } else {
                banner_release(&planes, delta, |r, d| { t.call(r.clone(), vec![d]); Ok(()) }).unwrap();
                Value::Null
            };
            json!({"status":"ok","value":v,"trace":t.0})
        }
        "animation" => {
            let mut w = Animation { c, t: Trace(vec![]) };
            let v = if ready { value(animation_ready(&mut w)) } else { animation_release(&mut w, delta).unwrap(); Value::Null };
            json!({"status":"ok","value":v,"trace":w.t.0})
        }
        k => panic!("{k}"),
    }
}
#[test]
fn native_release_ready_kernels_replay() {
    let fixture: Value = serde_json::from_str(FIXTURE).unwrap();
    let mut rows = 0;
    for row in fixture["rows"].as_array().unwrap() {
        let c = &row["input"];
        if c["kernel"].is_null() {
            continue;
        }
        assert_eq!(run(c), row["output"], "{} {c}", row["precision"]);
        rows += 1;
    }
    assert!(rows > 300, "{rows}");
}
