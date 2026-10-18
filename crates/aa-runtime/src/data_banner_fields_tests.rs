use super::*;
use serde_json::{Value, json};
struct Input<'a> {
    case: &'a Value,
    gets: Vec<String>,
    textures: Vec<String>,
}
impl Source<usize> for Input<'_> {
    type Node = Value;
    fn text(&mut self, n: &Value, key: &str) -> Result<String> {
        self.gets.push(key.into());
        Ok(n[key].as_str().unwrap().into())
    }
    fn flag(&mut self, n: &Value, key: &str) -> Result<bool> {
        self.gets.push(key.into());
        Ok(n[key].as_bool().unwrap())
    }
    fn children(&mut self, n: &Value, key: &str) -> Result<Vec<Value>> {
        self.gets.push(format!("children:{key}"));
        Ok(n[key].as_array().cloned().unwrap_or_default())
    }
    fn path(&mut self, format: &str, prefix: &str) -> Result<String> {
        Ok(format.replace("%s", prefix))
    }
    fn texture(&mut self, path: &str) -> Result<Option<usize>> {
        self.textures.push(path.into());
        if self.case["factory_stop"].as_i64() == Some(self.textures.len() as i64 - 1) {
            return Err("required:Texture".into());
        }
        Ok(Some(self.textures.len()))
    }
}
fn decal(d: &Decal<usize>) -> Value {
    json!({"function":d.function as u32,"flashing":u8::from(d.flashing),"textures":d.textures.len()})
}
fn plane(p: &Plane<usize>) -> Value {
    json!({"function":p.function as u32,"flashing":u8::from(p.flashing),"name":p.name,"decals":p.decals.iter().map(decal).collect::<Vec<_>>()})
}
#[test]
fn native_selected_banner_source_requests_and_partial_allocations_match() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-data-pre-request/native-banner-source-request.json"
    ))
    .unwrap();
    assert_eq!(doc["rows"].as_array().unwrap().len(), 120);
    for row in doc["rows"].as_array().unwrap() {
        let c = &row["input"];
        let mut input = Input {
            case: c,
            gets: vec![],
            textures: vec![],
        };
        let mut core = Core::constructed();
        let result = core.load(&c["source"], &mut input);
        let mut status = match result {
            Ok(()) => "ok".to_string(),
            Err(e) if e == "required:Texture" => e,
            Err(_) => "required:diagnostic".into(),
        };
        let mut requests = vec![];
        if status == "ok" {
            let delta = c["delta"].as_u64().unwrap_or(1) as u32;
            let mode = c["mode"].as_u64().unwrap_or(0) as u32;
            if let Err(e) = core.request(delta, mode, |resource, delta, mode| {
                requests.push(json!([resource, delta, mode]));
                if c["request_stop"].as_i64() == Some(requests.len() as i64 - 1) {
                    Err("required:request".into())
                } else {
                    Ok(())
                }
            }) {
                status = e;
            }
        }
        let allocated: Vec<_> = core.planes.iter().chain(core.pending.iter()).collect();
        let decals: Vec<_> = allocated
            .iter()
            .flat_map(|p| p.decals.iter().chain(p.pending.iter()))
            .map(decal)
            .collect();
        let actual = json!({"status":status,"model":core.model,"skin":core.skin.is_some(),"planes":core.planes.iter().map(plane).collect::<Vec<_>>(),
            "allocated_planes":allocated.iter().map(|p|plane(p)).collect::<Vec<_>>(),"allocated_decals":decals,"textures":input.textures,"gets":input.gets,"requests":requests});
        assert_eq!(actual, row["output"], "{row}");
    }
}

#[test]
fn actual_banner_function_constructor_registration_matches_lookup() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-data-pre-request/native-banner-aspect.json"
    ))
    .unwrap();
    for row in doc["rows"].as_array().unwrap() {
        for (index, entry) in row["output"]["enum"].as_array().unwrap().iter().enumerate() {
            assert_eq!(function(entry[1].as_str().unwrap()), index as i32);
            assert_eq!(entry[0], index);
            assert_eq!(entry[2], 0);
        }
    }
    assert_eq!(function("not_a_banner_function"), -1);
}
