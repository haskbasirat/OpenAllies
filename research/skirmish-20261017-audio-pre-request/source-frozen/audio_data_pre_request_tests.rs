use super::*;
use serde_json::{Value, json};
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-audio-pre-request/native-audio.json"
    ))
    .unwrap()
}
#[test]
fn original_v14_constructed_audio_category_registry_and_casefold() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-audio-pre-request/native-audio-category.json"
    ))
    .unwrap();
    for row in fixture["rows"].as_array().unwrap() {
        assert_eq!(
            row["native"]["table"],
            json!([{"value":0,"name":"SOUND"},{"value":1,"name":"MUSIC"},{"value":2,"name":"SPEECH"}])
        );
        for case in row["native"]["lookup"].as_array().unwrap() {
            assert_eq!(
                core::category(case["name"].as_str().unwrap()),
                case["value"].as_u64().map(|v| v as usize)
            );
        }
    }
}
struct Requests {
    lists: [Vec<u32>; 3],
    stop: usize,
    trace: Vec<Value>,
}
impl core::World for Requests {
    type Sound = u32;
    fn count(&mut self, list: usize) -> Result<usize> {
        Ok(self.lists[list].len())
    }
    fn sound(&mut self, list: usize, index: usize) -> Result<u32> {
        Ok(self.lists[list][index])
    }
    fn request(&mut self, sound: u32, delta: i32, mode: u32) -> Result<()> {
        //Actual5A6190 is executed by the native fixture. The shared Resource
        //request wrapper independently owns this same low-byte mapping.
        self.trace
            .push(json!({"sound":sound,"delta":delta as u32,"mode":if mode as u8!=0{4}else{1}}));
        if self.stop == self.trace.len() {
            Err("request required".into())
        } else {
            Ok(())
        }
    }
}
#[test]
fn original_v14_three_lists_duplicates_delta_mode_and_every_request_error_prefix() {
    let fixture = fixture();
    let mut count = 0;
    for row in fixture["rows"].as_array().unwrap() {
        for case in row["native"]["requests"].as_array().unwrap() {
            let mut world = Requests {
                lists: std::array::from_fn(|i| {
                    case["lists"][i]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_u64().unwrap() as u32)
                        .collect()
                }),
                stop: case["stop_at"].as_u64().unwrap() as usize,
                trace: vec![],
            };
            let out = core::request(
                &mut world,
                case["delta"].as_u64().unwrap() as u32 as i32,
                case["mode"].as_u64().unwrap() as u32,
            );
            assert_eq!(world.trace, case["trace"].as_array().unwrap().as_slice());
            assert_eq!(out.is_err(), !case["stop"].is_null());
            count += 1;
        }
    }
    assert_eq!(count, 720);
}
struct Sources {
    fields: [String; 3],
    folder: String,
    stop: usize,
    reads: Vec<String>,
    finds: Vec<String>,
    mapping: BTreeMap<String, usize>,
    bytes: [[u8; 3]; 3],
}
impl core::SourceWorld for Sources {
    type Sound = usize;
    fn text(&mut self, list: usize) -> Result<String> {
        self.reads.push(["leadin", "files", "fallout"][list].into());
        Ok(self.fields[list].clone())
    }
    fn resource(&mut self, token: &str, streaming: bool) -> Result<usize> {
        let path = crate::data_pre_request::path::relative(&self.folder, token)?;
        self.finds.push(path.clone());
        if self.stop == self.finds.len() {
            return Err("source required".into());
        }
        let index = self.mapping.len() % 3;
        let id = *self.mapping.entry(path).or_insert(index);
        if streaming {
            self.bytes[id][1] = 1
        }
        Ok(id)
    }
    fn standard_rate(&mut self, sound: &usize) -> Result<()> {
        self.bytes[*sound][0] = 1;
        Ok(())
    }
}
#[test]
fn original_v14_source_tokenizer_lookup_append_streaming_and_rate_partial_effects() {
    let fixture = fixture();
    let mut count = 0;
    for row in fixture["rows"].as_array().unwrap() {
        for case in row["native"]["source_projection"].as_array().unwrap() {
            let mut world = Sources {
                fields: std::array::from_fn(|i| case["fields"][i].as_str().unwrap().into()),
                folder: case["folder"].as_str().unwrap().into(),
                stop: case["stop_at"].as_u64().unwrap() as usize,
                reads: vec![],
                finds: vec![],
                mapping: BTreeMap::new(),
                bytes: [[0; 3]; 3],
            };
            let mut lists = std::array::from_fn(|_| vec![]);
            let category = case["category"].as_u64().unwrap();
            let out = core::load_lists(
                &mut world,
                &mut lists,
                category == 1 || category == 2,
                case["ignore"].as_u64().unwrap() != 0,
            );
            assert_eq!(out.is_err(), world.stop != 0);
            assert_eq!(json!(world.reads), case["reads"]);
            assert_eq!(json!(world.finds), case["finds"]);
            assert_eq!(json!(lists), case["lists"]);
            assert_eq!(json!(world.bytes), case["sound_bytes"]);
            count += 1;
        }
    }
    assert!(count > 500);
    assert!(core::tokens("quoted\".wav").is_err());
}
