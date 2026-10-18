use super::*;
use serde_json::{json, Value};
use std::collections::BTreeMap;
struct Source {
    refs: u32,
    pixels: u32,
    discard: u8,
    quality: u32,
}
struct Replay<'a> {
    c: &'a Value,
    events: Vec<Value>,
    flags: u32,
    current: u32,
    words: BTreeMap<String, u32>,
    memory: u32,
    warning: bool,
    sources: BTreeMap<u32, Source>,
    pixel_refs: BTreeMap<u32, u32>,
}
impl Replay<'_> {
    fn u(&self, k: &str) -> u32 {
        self.c[k].as_u64().unwrap() as u32
    }
    fn b(&self, k: &str) -> bool {
        self.c[k].as_bool().unwrap()
    }
    fn event(&mut self, value: Value) -> Result<()> {
        let fail = self.c["fail"].as_str() == value[0].as_str();
        self.events.push(value.clone());
        if fail {
            Err(value[0].as_str().unwrap().into())
        } else {
            Ok(())
        }
    }
    fn word_name(w: Word) -> &'static str {
        match w {
            Word::Width54 => "0x54",
            Word::Height58 => "0x58",
            Word::U4C => "0x4c",
            Word::V50 => "0x50",
        }
    }
    fn project(&self, stopped: Option<String>, result: Option<bool>) -> Value {
        json!({"events":self.events,"flags":self.flags,"current":self.current,"words":self.words,"memory":self.memory,"warning":u32::from(self.warning),
  "sources":self.sources.iter().map(|(k,v)|(k.to_string(),json!({"refs":v.refs,"pixels":v.pixels,"discard":v.discard,"quality":v.quality}))).collect::<BTreeMap<_,_>>(),"pixel_refs":self.pixel_refs,
  "stopped":stopped,"result":result.map(u32::from)})
    }
}
impl Fields for Replay<'_> {
    type Source = u32;
    type Pixel = u32;
    fn location_count(&mut self) -> Result<u32> {
        Ok(self.u("locations"))
    }
    fn last_location_writable(&mut self) -> Result<Option<bool>> {
        Ok(self.c["writable"].as_bool())
    }
    fn writable_child_present(&mut self) -> Result<bool> {
        Ok(self.b("child"))
    }
    fn filename_length(&mut self) -> Result<u32> {
        Ok(u32::from(self.b("filename")))
    }
    fn resource_no_ni_textures(&mut self) -> Result<u8> {
        Ok(self.u("no_ni") as u8)
    }
    fn flags8(&mut self) -> Result<u32> {
        Ok(self.flags)
    }
    fn store_flags8(&mut self, v: u32) -> Result<()> {
        self.events.push(json!(["entry", "0x8", v]));
        self.flags = v;
        Ok(())
    }
    fn loading_diagnostic(&mut self) -> Result<()> {
        self.event(json!(["loading"]))
    }
    fn default_source_5a4d80(&mut self) -> Result<Option<u32>> {
        self.event(json!(["default"]))?;
        Ok((self.u("default") != 0).then(|| self.u("default")))
    }
    fn current_source3c(&mut self) -> Result<Option<u32>> {
        Ok((self.current != 0).then_some(self.current))
    }
    fn same_source(&mut self, a: Option<u32>, b: Option<u32>) -> Result<bool> {
        Ok(a == b)
    }
    fn missing_direct_warning(&mut self) -> Result<bool> {
        Ok(self.warning)
    }
    fn assert_missing_direct_855d30(&mut self) -> Result<bool> {
        self.event(json!(["assert"]))?;
        Ok(self.b("assert_value"))
    }
    fn store_missing_direct_warning(&mut self, v: bool) -> Result<()> {
        self.events.push(json!(["warning", u32::from(v)]));
        self.warning = v;
        Ok(())
    }
    fn retain_source(&mut self, s: u32) -> Result<()> {
        let v = self.sources.get_mut(&s).unwrap();
        v.refs = v.refs.wrapping_add(1);
        self.events.push(json!(["source", s, "0x4", v.refs]));
        Ok(())
    }
    fn release_source(&mut self, s: u32) -> Result<()> {
        let v = self.sources.get_mut(&s).unwrap();
        v.refs = v.refs.wrapping_sub(1);
        self.events.push(json!(["source", s, "0x4", v.refs]));
        Ok(())
    }
    fn reload_direct_44(&mut self, _: u32) -> Result<()> {
        self.event(json!(["reload"]))
    }
    fn create_source_87be50(&mut self) -> Result<Option<u32>> {
        self.event(json!(["create"]))?;
        Ok((self.u("created") != 0).then(|| self.u("created")))
    }
    fn force_low_quality(&mut self) -> Result<u8> {
        self.event(json!(["low"]))?;
        Ok(self.u("low") as u8)
    }
    fn use_high_quality(&mut self) -> Result<u8> {
        self.event(json!(["high"]))?;
        Ok(self.u("high") as u8)
    }
    fn set_low_quality_3c(&mut self, s: u32, v: u32) -> Result<()> {
        self.sources.get_mut(&s).unwrap().quality = v;
        self.events.push(json!(["source", s, "0x48", v]));
        Ok(())
    }
    fn open_selected_stream_59a9b0(&mut self) -> Result<()> {
        self.event(json!(["open"]))
    }
    fn load_selected_stream_40(&mut self, s: u32) -> Result<()> {
        self.event(json!(["load"]))?;
        let p = u32::from(self.b("pixels"));
        self.sources.get_mut(&s).unwrap().pixels = p;
        Ok(())
    }
    fn close_selected_stream_8761a0(&mut self) -> Result<()> {
        self.event(json!(["close"]))
    }
    fn missing_source_diagnostic(&mut self) -> Result<()> {
        self.event(json!(["missing"]))
    }
    fn missing_retained_pixels_diagnostic(&mut self) -> Result<()> {
        self.event(json!(["keep-diagnostic"]))
    }
    fn source_pixels(&mut self, s: u32) -> Result<Option<u32>> {
        let p = self.sources[&s].pixels;
        Ok((p != 0).then_some(p))
    }
    fn set_source_discard44(&mut self, s: u32, v: u8) -> Result<()> {
        self.sources.get_mut(&s).unwrap().discard = v;
        self.events.push(json!(["source", s, "0x44", v]));
        Ok(())
    }
    fn source_width34(&mut self, s: u32) -> Result<u32> {
        self.event(json!(["width"]))?;
        Ok(if let Some(r) = self.c["renderer"].as_array() {
            r[0].as_u64().unwrap() as u32
        } else if self.sources[&s].pixels != 0 {
            self.u("width")
        } else {
            0
        })
    }
    fn source_height38(&mut self, s: u32) -> Result<u32> {
        self.event(json!(["height"]))?;
        Ok(if let Some(r) = self.c["renderer"].as_array() {
            r[1].as_u64().unwrap() as u32
        } else if self.sources[&s].pixels != 0 {
            self.u("height")
        } else {
            0
        })
    }
    fn word(&mut self, w: Word) -> Result<u32> {
        Ok(self.words[Self::word_name(w)])
    }
    fn store_word(&mut self, w: Word, v: u32) -> Result<()> {
        self.words.insert(Self::word_name(w).into(), v);
        self.events.push(json!(["entry", Self::word_name(w), v]));
        Ok(())
    }
    fn memory_counter(&mut self) -> Result<u32> {
        Ok(self.memory)
    }
    fn store_memory_counter(&mut self, v: u32) -> Result<()> {
        self.memory = v;
        self.events.push(json!(["memory", v]));
        Ok(())
    }
    fn filename_is_png(&mut self) -> Result<bool> {
        self.event(json!(["png"]))?;
        Ok(self.b("png"))
    }
    fn resize_diagnostic(&mut self, _: [u32; 2], _: [u32; 2]) -> Result<()> {
        self.event(json!(["resize-warning"]))
    }
    fn missing_resize_pixels_diagnostic(&mut self) -> Result<()> {
        self.event(json!(["resize-missing"]))
    }
    fn pixel_format_word(&mut self, _: u32, i: usize) -> Result<u32> {
        Ok(self.c["format"][i].as_u64().unwrap() as u32)
    }
    fn required_resize_format_word(&mut self, i: usize) -> Result<u32> {
        Ok(self.c["required_format"][i].as_u64().unwrap() as u32)
    }
    fn incompatible_resize_format_diagnostic(&mut self) -> Result<()> {
        self.event(json!(["resize-format"]))
    }
    fn resize_87c5c0(&mut self, _: u32, _: [u32; 2]) -> Result<Option<u32>> {
        self.event(json!(["resize"]))?;
        Ok((self.u("resize") != 0).then(|| self.u("resize")))
    }
    fn oversized_diagnostic(&mut self, _: [u32; 2]) -> Result<()> {
        self.event(json!(["oversized"]))
    }
    fn retain_pixel(&mut self, p: u32) -> Result<()> {
        let v = self.pixel_refs.get_mut(&p).unwrap();
        *v = v.wrapping_add(1);
        self.events.push(json!(["pixel", p, *v]));
        Ok(())
    }
    fn release_pixel(&mut self, p: u32) -> Result<()> {
        let v = self.pixel_refs.get_mut(&p).unwrap();
        *v = v.wrapping_sub(1);
        self.events.push(json!(["pixel", p, *v]));
        Ok(())
    }
    fn pixel_storage_bytes(&mut self, _: u32) -> Result<u32> {
        Ok(self.u("bytes"))
    }
    fn account_5aa270(&mut self, b: u32) -> Result<()> {
        self.event(json!(["account", b]))
    }
    fn render_format_count(&mut self) -> Result<i32> {
        Ok(self.c["render_count"]
            .as_i64()
            .map(|x| x as i32)
            .unwrap_or(self.c["render_formats"].as_array().unwrap().len() as i32))
    }
    fn render_format_word(&mut self, i: i32, w: usize) -> Result<u32> {
        Ok(self.c["render_formats"][i as usize][w].as_u64().unwrap() as u32)
    }
    fn incompatible_render_format_diagnostic(&mut self) -> Result<()> {
        self.event(json!(["render-format"]))
    }
    fn pixel_type(&mut self, _: u32) -> Result<i32> {
        Ok(self.u("pixel_type") as i32)
    }
    fn pixel_bits_per_pixel(&mut self, _: u32) -> Result<u8> {
        Ok(self.u("bpp") as u8)
    }
    fn unusual_pixel_format_diagnostic(&mut self) -> Result<()> {
        self.event(json!(["pixel-warning"]))
    }
    fn store_source3c(&mut self, s: Option<u32>) -> Result<()> {
        self.current = s.unwrap_or(0);
        self.events.push(json!(["entry", "0x3c", self.current]));
        Ok(())
    }
}
#[test]
fn original_v14_texture_load_all_pc_fields_refcounts_and_required_prefixes() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-texture-load-kernel/native-texture-load.json"
    ))
    .unwrap();
    assert_eq!(fixture["rows"].as_array().unwrap().len(), 351);
    for row in fixture["rows"].as_array().unwrap() {
        let c = &row["input"];
        let u = |k: &str| c[k].as_u64().unwrap() as u32;
        let sources = (1..=3)
            .map(|key| {
                let initial = (u("flags") & 0x50 != 0 || u("no_ni") == 1) && c["pixels"] == true;
                (
                    key,
                    Source {
                        refs: u("source_refs"),
                        pixels: if initial || key != 1 && c["pixels"] == true {
                            key
                        } else {
                            0
                        },
                        discard: 0,
                        quality: 0xeeeeeeee,
                    },
                )
            })
            .collect();
        let mut f = Replay {
            c,
            events: vec![],
            flags: u("flags"),
            current: u("old"),
            words: [
                ("0x54".into(), 0xcccccccc),
                ("0x58".into(), 0xdddddddd),
                ("0x4c".into(), 0x3f800000),
                ("0x50".into(), 0x3f800000),
            ]
            .into_iter()
            .collect(),
            memory: u("memory"),
            warning: c["warning"] == true,
            sources,
            pixel_refs: (1..=3).map(|key| (key, u("pixel_refs"))).collect(),
        };
        let pc = match row["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            "Extended64" => Precision::Extended64,
            _ => unreachable!(),
        };
        let mut state = State::new_call();
        let result = state.load(&mut f, pc);
        let error = result.as_ref().err().map(|e| {
            if e.contains("5A8E53") {
                "null-direct".into()
            } else if e.contains("5A8C30") {
                "null-quality".into()
            } else if e.contains("5A8C57") {
                "null-stream".into()
            } else if e.contains("5A8F8D") {
                "null-resized".into()
            } else {
                e.clone()
            }
        });
        assert_eq!(
            f.project(error, result.ok()),
            row["output"],
            "{} {}",
            row["precision"],
            c["name"]
        );
        assert!(state
            .load(&mut f, pc)
            .unwrap_err()
            .contains("prior attempt"));
    }
}
