use super::*;
use serde_json::{Value, json};
const FIXTURE: &str = include_str!(
    "../../../research/skirmish-20261019-data-release-ready/native-data-release-ready.json"
);
struct Lab<'a> {
    c: &'a Value,
    count: u32,
    aspects: isize,
    mask: u32,
    outer: isize,
    inner: isize,
    primary: isize,
    secondary: isize,
    trace: Vec<Value>,
}
impl Lab<'_> {
    fn yes(&self, k: &str) -> bool {
        self.c[k].as_bool().unwrap()
    }
    fn call(&mut self, k: String, args: Vec<u32>) -> Result<()> {
        self.trace.push(json!({"call":k,"args":args}));
        let change = self.c["mutation"].as_str().unwrap_or("");
        if k == "aspect0" && change == "aspect_count" {
            self.aspects = 1
        }
        if k == "animation0" {
            match change {
                "animation_inner" => self.inner = 1,
                "animation_outer" => self.outer = 2,
                _ => {}
            }
        }
        if k == "ability0" && change == "ability_count" {
            self.primary = 0;
            self.secondary = 1
        }
        if k == "actorfx404" {
            match change {
                "mask_clear" => self.mask = 0,
                "mask_set" => self.mask = 0x80000,
                _ => {}
            }
        }
        if self.c["stop"] == k.as_str() {
            return Err(format!("stop_{k}"));
        }
        Ok(())
    }
    fn ready(&mut self, k: String) -> Result<bool> {
        let answer = self.c["false_at"] != k.as_str();
        self.call(k, vec![])?;
        Ok(answer)
    }
}
impl ObjectRelease for Lab<'_> {
    type Data = String;
    type Aspect = usize;
    fn release_base(&mut self, d: u32) -> Result<()> {
        self.count = self.count.wrapping_sub(d);
        Ok(())
    }
    fn requested_base(&mut self) -> Result<bool> {
        Ok((self.count as i32) > 0)
    }
    fn portrait(&mut self) -> Result<Option<String>> {
        Ok(self.yes("portrait").then(|| "portrait".into()))
    }
    fn release_data(&mut self, d: String, delta: u32) -> Result<()> {
        self.call(d, vec![delta])
    }
    fn data_ready(&mut self, d: String) -> Result<bool> {
        self.ready(d)
    }
    fn release_banner(&mut self, d: u32) -> Result<()> {
        if self.yes("banner") {
            self.call("banner".into(), vec![d])?
        }
        Ok(())
    }
    fn banner_ready(&mut self) -> Result<bool> {
        if self.yes("banner") { self.ready("banner".into()) } else { Ok(true) }
    }
    fn aspect_count(&mut self) -> Result<isize> {
        Ok(self.aspects)
    }
    fn aspect(&mut self, i: usize) -> Result<usize> {
        Ok(i)
    }
    fn release_aspect(&mut self, i: usize, d: u32) -> Result<()> {
        self.call(format!("aspect{i}"), vec![d])
    }
    fn aspect_ready(&mut self, i: usize) -> Result<bool> {
        self.ready(format!("aspect{i}"))
    }
    fn release_texture(&mut self, o: u32, d: u32) -> Result<()> {
        if self.yes("textures") {
            self.call(format!("texture_{o:x}"), vec![d])?
        }
        Ok(())
    }
    fn texture_ready(&mut self, o: u32) -> Result<bool> {
        Ok(!self.yes("textures") || self.c["unloaded"] != format!("texture_{o:x}").as_str())
    }
    fn release_view_fx(&mut self, i: usize, d: u32) -> Result<()> {
        self.call(format!("viewfx{i}"), vec![d])
    }
    fn view_fx_ready(&mut self, i: usize) -> Result<bool> {
        self.ready(format!("viewfx{i}"))
    }
}
impl ActorRelease for Lab<'_> {
    type AnimationSet = usize;
    type Animation = usize;
    type Ability = String;
    fn mask_174(&mut self) -> Result<u32> {
        Ok(self.mask)
    }
    fn animation_set_count(&mut self) -> Result<isize> {
        Ok(self.outer)
    }
    fn animation_set(&mut self, i: usize) -> Result<Option<usize>> {
        Ok((i != 0).then_some(i))
    }
    fn animation_count(&mut self, s: &usize) -> Result<isize> {
        Ok(if *s == 1 { self.inner } else { 1 })
    }
    fn animation(&mut self, s: &usize, i: usize) -> Result<usize> {
        Ok((s - 1) * 2 + i)
    }
    fn release_animation(&mut self, i: usize, d: u32) -> Result<()> {
        self.call(format!("animation{i}"), vec![d])
    }
    fn animation_ready(&mut self, i: usize) -> Result<bool> {
        self.ready(format!("animation{i}"))
    }
    fn release_actor_fx(&mut self, o: u32, d: u32) -> Result<()> {
        self.call(format!("actorfx{o}"), vec![d])
    }
    fn actor_fx_ready(&mut self, o: u32) -> Result<bool> {
        self.ready(format!("actorfx{o}"))
    }
    fn ability_count(&mut self, s: bool) -> Result<isize> {
        Ok(if s { self.secondary } else { self.primary })
    }
    fn ability(&mut self, s: bool, i: usize) -> Result<String> {
        Ok(format!("ability{}", i + if s { 2 } else { 0 }))
    }
    fn release_ability(&mut self, a: String, d: u32) -> Result<()> {
        self.call(a, vec![d])
    }
    fn ability_ready(&mut self, a: String) -> Result<bool> {
        self.ready(a)
    }
    fn extra_ability_2a4(&mut self) -> Result<Option<String>> {
        Ok(self.yes("extra").then(|| "extra".into()))
    }
    fn deployment_data_534(&mut self) -> Result<Option<String>> {
        Ok(self.yes("deployment").then(|| "deployment".into()))
    }
}
#[test]
fn native_release_ready_coordinators_replay() {
    let fixture: Value = serde_json::from_str(FIXTURE).unwrap();
    let mut rows = 0;
    for row in fixture["rows"].as_array().unwrap() {
        let c = &row["input"];
        if !c["kernel"].is_null() {
            continue;
        }
        let abilities = c["abilities"] == true;
        let mut w = Lab {
            c,
            count: c["initial"].as_u64().unwrap() as u32,
            aspects: c["aspects"].as_i64().unwrap() as isize,
            mask: c["mask"].as_u64().unwrap() as u32,
            outer: if c["animations"] == true { 3 } else { 0 },
            inner: 2,
            primary: if abilities { 2 } else { 0 },
            secondary: if abilities { 2 } else { 0 },
            trace: vec![],
        };
        let delta = c["delta"].as_u64().unwrap() as u32;
        let (status, value) = match (c["actor"] == true, c["ready"] == true) {
            (false, false) => (object_release(&mut w, delta).err(), Value::Null),
            (true, false) => (actor_release(&mut w, delta).err(), Value::Null),
            (false, true) => match object_ready(&mut w) {
                Ok(v) => (None, json!(u8::from(v))),
                Err(e) => (Some(e), Value::Null),
            },
            (true, true) => match actor_ready(&mut w) {
                Ok(v) => (None, json!(u8::from(v))),
                Err(e) => (Some(e), Value::Null),
            },
        };
        let status = status.unwrap_or_else(|| "ok".into());
        assert_eq!(
            json!({"status":status,"value":value,"count":w.count,"trace":w.trace}),
            row["output"],
            "{} {c}",
            row["precision"]
        );
        rows += 1;
    }
    assert!(rows > 300, "{rows}");
}
