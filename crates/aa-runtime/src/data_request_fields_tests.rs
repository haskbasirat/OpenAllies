use super::*;
use serde_json::{Value, json};
struct Lab<'a> {
    c: &'a Value,
    count: u32,
    flags: [u32; 3],
    models: [usize; 2],
    aspects: isize,
    bits: u32,
    unique: bool,
    mask: u32,
    inner: usize,
    outer: usize,
    primary: isize,
    secondary: isize,
    trace: Vec<Value>,
}
impl Lab<'_> {
    fn yes(&self, k: &str) -> bool {
        self.c[k].as_bool().unwrap()
    }
    fn call(&mut self, k: String, args: Vec<u32>) -> Result<()> {
        self.trace
            .push(json!({"call":k,"args":args,"count":self.count}));
        if self.c["stop"] == k {
            return Err(format!("required_{k}"));
        }
        let change = self.c["mutation"].as_str().unwrap_or("");
        if k == "aspect0" {
            match change {
                "aspect_count" => self.aspects = 1,
                "aspect_model" => self.models[0] = 2,
                "aspect_fields" => {
                    self.bits = 0;
                    self.unique = false
                }
                _ => {}
            }
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
        Ok(())
    }
}
impl ObjectWorld for Lab<'_> {
    type Data = String;
    type Aspect = usize;
    type Model = usize;
    fn base_request(&mut self, d: u32, _: u32) -> Result<()> {
        self.count = self.count.wrapping_add(d);
        Ok(())
    }
    fn portrait(&mut self) -> Result<Option<String>> {
        Ok(self.yes("portrait").then(|| "portrait".into()))
    }
    fn request_data(&mut self, d: String, a: u32, b: u32) -> Result<()> {
        self.call(d, vec![a, b])
    }
    fn aspect_count(&mut self) -> Result<isize> {
        Ok(self.aspects)
    }
    fn aspect(&mut self, i: usize) -> Result<usize> {
        Ok(i)
    }
    fn request_aspect(&mut self, i: usize, a: u32, b: u32) -> Result<()> {
        self.call(format!("aspect{i}"), vec![a, b])
    }
    fn flags_1c(&mut self) -> Result<u32> {
        Ok(self.bits)
    }
    fn unique_fc(&mut self) -> Result<bool> {
        Ok(self.unique)
    }
    fn current_aspect_model(&mut self, i: usize) -> Result<usize> {
        Ok(self.models[i])
    }
    fn or_model_flags(&mut self, i: usize, v: u32) -> Result<()> {
        self.flags[i] |= v;
        Ok(())
    }
    fn request_texture(&mut self, o: u32, a: u32, b: u32) -> Result<()> {
        if self.yes("textures") {
            self.call(format!("texture{o}"), vec![a, b])?
        }
        Ok(())
    }
    fn request_banner(&mut self, a: u32, b: u32) -> Result<()> {
        if self.yes("banner") {
            self.call("banner".into(), vec![a, b])?
        }
        Ok(())
    }
    fn request_view_fx(&mut self, i: usize, a: u32, b: u32) -> Result<()> {
        self.call(format!("viewfx{i}"), vec![a, b])
    }
}
impl ActorWorld for Lab<'_> {
    type AnimationSet = usize;
    type Animation = usize;
    type Ability = String;
    fn mask_174(&mut self) -> Result<u32> {
        Ok(self.mask)
    }
    fn mark_all_aspect_models(&mut self, v: u32) -> Result<()> {
        for i in 0..self.aspects.max(0) as usize {
            self.flags[self.models[i]] |= v
        }
        Ok(())
    }
    fn animation_set_count(&mut self) -> Result<isize> {
        Ok(self.outer as isize)
    }
    fn animation_set(&mut self, i: usize) -> Result<Option<usize>> {
        Ok((i != 0).then_some(i))
    }
    fn animation_count(&mut self, s: &usize) -> Result<isize> {
        Ok(if *s == 1 { self.inner } else { 1 } as isize)
    }
    fn animation(&mut self, s: &usize, i: usize) -> Result<usize> {
        Ok((s - 1) * 2 + i)
    }
    fn request_animation(&mut self, i: usize, d: u32) -> Result<()> {
        self.call(format!("animation{i}"), vec![d])
    }
    fn request_actor_fx(&mut self, o: u32, a: u32, b: u32) -> Result<()> {
        self.call(format!("actorfx{o}"), vec![a, b])
    }
    fn ability_count(&mut self, s: bool) -> Result<isize> {
        Ok(if s { self.secondary } else { self.primary })
    }
    fn ability(&mut self, s: bool, i: usize) -> Result<String> {
        Ok(format!("ability{}", i + if s { 2 } else { 0 }))
    }
    fn request_ability(&mut self, s: String, a: u32, b: u32) -> Result<()> {
        self.call(s, vec![a, b])
    }
    fn extra_ability_2a4(&mut self) -> Result<Option<String>> {
        Ok(self.yes("extra").then(|| "extra".into()))
    }
    fn settlement_data_534(&mut self) -> Result<Option<String>> {
        Ok(self.yes("settlement").then(|| "settlement".into()))
    }
}
#[test]
fn full_native_object_actor_dispatch_and_mutable_prefix() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-data-pre-request/native-object-actor-request.json"
    ))
    .unwrap();
    for row in fixture["rows"].as_array().unwrap() {
        let c = &row["input"];
        let mut w = Lab {
            c,
            count: c["initial"].as_u64().unwrap() as u32,
            flags: [0x400, 0x401, 0x402],
            models: [0, 1],
            aspects: c["count"].as_i64().unwrap() as isize,
            bits: c["flags"].as_u64().unwrap() as u32,
            unique: c["unique"].as_bool().unwrap(),
            mask: c["mask"].as_u64().unwrap() as u32,
            inner: 2,
            outer: if c["animations"] == true { 3 } else { 0 },
            primary: if c["abilities"] == true { 2 } else { 0 },
            secondary: if c["abilities"] == true { 2 } else { 0 },
            trace: vec![],
        };
        let a = c["delta"].as_u64().unwrap() as u32;
        let b = c["mode"].as_u64().unwrap() as u32;
        let result = if c["actor"] == true {
            actor(&mut w, a, b)
        } else {
            object(&mut w, a, b)
        };
        let status = result.err().unwrap_or_else(|| "ok".into());
        assert_eq!(
            json!({"status":status,"count":w.count,"flags":w.flags,"trace":w.trace}),
            row["output"],
            "{} {c}",
            row["precision"]
        );
    }
}
