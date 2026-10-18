use super::*;
use serde_json::{json, Value};
struct Replay<'a> {
    c: &'a Value,
    events: Vec<Value>,
    data: bool,
    pixels: bool,
    refs: u32,
}
impl Replay<'_> {
    fn event(&mut self, value: Value) -> Result<()> {
        let fail = self.c["fail"].as_str() == value[0].as_str();
        self.events.push(value.clone());
        if fail {
            Err(value[0].as_str().unwrap().into())
        } else {
            Ok(())
        }
    }
    fn read(&mut self, name: &str, value: u32) -> Result<u32> {
        self.events.push(json!(["read", name, value]));
        Ok(value)
    }
}
impl Fields for Replay<'_> {
    type Source = ();
    type Renderer = ();
    type Pixel = ();
    fn renderer_b442c0(&mut self) -> Result<Option<()>> {
        Ok((self.read("renderer", u32::from(self.c["renderer"] == true))? != 0).then_some(()))
    }
    fn create_renderer_data_100(&mut self, _: (), _: ()) -> Result<bool> {
        self.event(json!(["create"]))?;
        if let Some(v) = self.c["install"].as_bool() {
            self.data = v;
        }
        Ok(self.c["result"] == true)
    }
    fn discard_policy_b43f20(&mut self) -> Result<u8> {
        self.read("policy", self.c["policy"].as_u64().unwrap() as u32)
            .map(|x| x as u8)
    }
    fn renderer_data_present_2c(&mut self, _: ()) -> Result<bool> {
        self.read("data", u32::from(self.data)).map(|x| x != 0)
    }
    fn discard_flag_44(&mut self, _: ()) -> Result<u8> {
        self.read("discard", self.c["discard"].as_u64().unwrap() as u32)
            .map(|x| x as u8)
    }
    fn pixels_40(&mut self, _: ()) -> Result<Option<()>> {
        Ok((self.read("pixels", u32::from(self.pixels))? != 0).then_some(()))
    }
    fn release_pixel(&mut self, _: ()) -> Result<()> {
        self.refs = self.refs.wrapping_sub(1);
        self.events.push(json!(["refs", self.refs]));
        if self.refs == 0 {
            self.event(json!(["destroy"]))?;
        }
        Ok(())
    }
    fn clear_pixels_40(&mut self, _: ()) -> Result<()> {
        self.events.push(json!(["clear", 0]));
        self.pixels = false;
        Ok(())
    }
}
#[test]
fn actual_source_renderer_gates_reread_real_data_and_preserve_pixel_release_error() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-texture-load-kernel/native-source-renderer.json"
    ))
    .unwrap();
    assert_eq!(fixture["rows"].as_array().unwrap().len(), 444);
    for row in fixture["rows"].as_array().unwrap() {
        let c = &row["input"];
        let mut fields = Replay {
            c,
            events: vec![],
            data: c["data"] == true,
            pixels: c["pixels"] == true,
            refs: c["refs"].as_u64().unwrap() as u32,
        };
        let result = create(&mut fields, ());
        let stopped = result.as_ref().err().cloned();
        let actual = json!({"events":fields.events,"result":result.ok().map(u8::from),"stopped":stopped,"pixel_refs":fields.refs,"pixels":fields.pixels,"data":fields.data});
        assert_eq!(actual, row["output"], "{} {c}", row["precision"]);
    }
}
