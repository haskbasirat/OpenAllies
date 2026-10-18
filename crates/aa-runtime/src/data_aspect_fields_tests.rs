use super::*;
use serde_json::{Value, json};

struct Lab<'a> {
    c: &'a Value,
    count: u32,
    model: Option<usize>,
    biome: Option<usize>,
    flags: [u32; 3],
    extra: bool,
    extra_count: u32,
    trace: Vec<Value>,
}
impl Lab<'_> {
    fn call(&mut self, name: &str, args: Vec<u32>) -> Result<()> {
        self.trace
            .push(json!({"call":name,"args":args,"count":self.count}));
        if self.c["stop"] == name {
            return Err(format!("required_{name}"));
        }
        Ok(())
    }
}
impl World for Lab<'_> {
    type Model = usize;
    type Biome = usize;
    fn context_sensitive(&mut self) -> Result<bool> {
        Ok(self.c["sensitive"].as_bool().unwrap())
    }
    fn current_biome(&mut self) -> Result<usize> {
        Ok(1)
    }
    fn stored_biome(&mut self) -> Result<Option<usize>> {
        Ok(self.biome)
    }
    fn same_biome(&self, a: &usize, b: &usize) -> bool {
        a == b
    }
    fn store_biome(&mut self, v: usize) -> Result<()> {
        self.biome = Some(v);
        Ok(())
    }
    fn model(&mut self) -> Result<Option<usize>> {
        Ok(self.model)
    }
    fn same_model(&self, a: &usize, b: &usize) -> bool {
        a == b
    }
    fn make_model(&mut self) -> Result<usize> {
        self.call("factory", vec![])?;
        Ok(if self.c["factory_same"] == true { 0 } else { 1 })
    }
    fn store_model(&mut self, v: usize) -> Result<()> {
        self.model = Some(v);
        Ok(())
    }
    fn model_flags(&mut self, m: &usize) -> Result<u32> {
        Ok(self.flags[*m])
    }
    fn mark_model(&mut self, m: &usize) -> Result<()> {
        if self.c["unique"] == true {
            self.flags[*m] |= 0x80000
        } else if self.c["skin"] == true {
            self.flags[*m] |= 0x8000
        }
        Ok(())
    }
    fn extra_requested(&mut self) -> Result<bool> {
        Ok(self.extra)
    }
    fn assertion_suppressed(&mut self) -> Result<bool> {
        Ok(false)
    }
    fn assert_context_change(&mut self) -> Result<()> {
        self.call("assert", vec![])
    }
    fn clear_extra_count(&mut self) -> Result<()> {
        self.extra_count = 0;
        Ok(())
    }
    fn store_extra_requested(&mut self, v: bool) -> Result<()> {
        self.extra = v;
        Ok(())
    }
    fn request_skin(&mut self, index: usize, delta: u32, mode: u32) -> Result<()> {
        let name = if index == 0 { "skin" } else { "glow" };
        if self.c[name] == true {
            self.call(name, vec![delta, mode])?;
        }
        Ok(())
    }
    fn request_animations(&mut self, delta: u32) -> Result<()> {
        if self.c["animation"] == true {
            self.call("animation", vec![delta])?;
        }
        Ok(())
    }
    fn request_model(&mut self, _: &usize, delta: u32, mode: u32) -> Result<()> {
        self.call("model", vec![delta, mode])?;
        if self.c["model_switch"] == true {
            self.model = Some(2);
        }
        Ok(())
    }
    fn request_extra(&mut self, delta: u32, mode: u32) -> Result<()> {
        self.call("extra", vec![delta, mode])?;
        self.extra = true;
        if self.c["mutate_count"] == true {
            self.count = 123;
        }
        Ok(())
    }
    fn count(&mut self) -> Result<u32> {
        Ok(self.count)
    }
    fn store_count(&mut self, v: u32) -> Result<()> {
        self.count = v;
        Ok(())
    }
}

#[test]
fn original_v14_aspect469fe0_46aec0_current_fields_and_error_prefixes() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-data-pre-request/native-aspect-request.json"
    ))
    .unwrap();
    assert_eq!(fixture["rows"].as_array().unwrap().len(), 225);
    for row in fixture["rows"].as_array().unwrap() {
        let c = &row["input"];
        let loaded = u32::from(c["loaded"].as_bool().unwrap());
        let mut w = Lab {
            c,
            count: c["count"].as_u64().unwrap() as u32,
            model: (c["model_present"] == true).then_some(0),
            biome: Some(usize::from(c["same_context"] == true)),
            flags: [loaded, loaded, 1 - loaded],
            extra: c["extra"].as_bool().unwrap(),
            extra_count: 7,
            trace: vec![],
        };
        let r = request(
            &mut w,
            c["delta"].as_u64().unwrap() as u32,
            c["mode"].as_u64().unwrap() as u32,
        );
        assert_eq!(
            json!({"status":r.err().unwrap_or("ok".into()),"count":w.count,"context":w.biome,
            "model":w.model,"flags":w.flags,"extra_count":w.extra_count,"extra":w.extra,"trace":w.trace}),
            row["output"],
            "{} / {}",
            row["precision"],
            c
        );
    }
}
