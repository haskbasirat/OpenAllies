//! Replays research/phase8-saved-components/native-load-tail.json: the
//! original 4FC600 tail 4FD3F3..4FD471 (world-start gate, 4F5290, 4F48E0,
//! 4F1C20, the GWorld +22C/+230 stores and the 404F60 log) against
//! aa_actors::world_load_tail.
use aa_actors::{
    post_load::{Error, Id, Result},
    world_load_tail::*,
};
use aa_numeric::Precision;
use serde_json::{Value, json};
use std::collections::BTreeMap;

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../research/phase8-saved-components/native-load-tail.json"
);

struct World<'a> {
    c: &'a Value,
    ids: BTreeMap<String, Id>,
    look_at: Option<[u32; 2]>,
    calls: Vec<Value>,
}
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
impl World<'_> {
    fn actor(&self, id: Id) -> &Value {
        self.c["actors"].as_array().unwrap().iter().find(|a| self.ids[a["label"].as_str().unwrap()] == id).unwrap()
    }
    fn kingdom(&self, id: Id) -> &Value {
        self.c["kingdoms"].as_array().unwrap().iter().find(|k| self.ids[k["label"].as_str().unwrap()] == id).unwrap()
    }
}
impl LoadTailWorld for World<'_> {
    type Player = ();
    fn map_settings_mode(&self) -> Result<u32> {
        Ok(u(&self.c["mode"]))
    }
    fn world_start_flag(&mut self) -> Result<bool> {
        let r = self.c["flag_59e520"].as_bool().unwrap();
        self.calls.push(json!({"seam": "59E520", "result": r}));
        Ok(r)
    }
    fn world_start(&mut self) -> Result<()> {
        self.calls.push(json!({"seam": "world_start"}));
        Ok(())
    }
    fn app_state(&self) -> Result<u32> {
        Ok(u(&self.c["app_state"]))
    }
    fn world_word_48(&self) -> Result<u32> {
        Ok(u(&self.c["world_48"]))
    }
    fn local_player(&self) -> Result<Option<()>> {
        Ok((!self.c["player"].is_null()).then_some(()))
    }
    fn player_name(&self, _: &()) -> Result<Vec<u16>> {
        Ok(self.c["player"]["name"].as_str().unwrap().encode_utf16().collect())
    }
    fn player_kingdom(&self, _: &()) -> Result<Option<Id>> {
        Ok(self.c["player"].get("kingdom").and_then(Value::as_str).map(|k| self.ids[k]))
    }
    fn world_kingdom_slots(&self) -> Result<Vec<Option<Id>>> {
        Ok(self.c["world_kingdoms"].as_array().unwrap().iter().map(|v| v.as_str().map(|k| self.ids[k])).collect())
    }
    fn object_name_18(&self, object: Id) -> Result<Vec<u16>> {
        Ok(self.kingdom(object)["name"].as_str().unwrap().encode_utf16().collect())
    }
    fn map_size_bits(&self) -> Result<[u32; 2]> {
        Ok([u(&self.c["map"][0]), u(&self.c["map"][1])])
    }
    fn kingdom_actors(&self, kingdom: Id) -> Result<Vec<Id>> {
        Ok(self.kingdom(kingdom)["actors"].as_array().unwrap().iter().map(|a| self.ids[a.as_str().unwrap()]).collect())
    }
    fn kingdom_word_1f8(&self, kingdom: Id) -> Result<Option<Id>> {
        Ok(self.kingdom(kingdom)["f8"].as_str().map(|a| self.ids[a]))
    }
    fn object_flags(&self, object: Id) -> Result<u32> {
        Ok(self.actor(object).get("flags").map(u).unwrap_or(0))
    }
    fn has_settlement(&self, actor: Id) -> Result<bool> {
        Ok(self.actor(actor).get("settlement").and_then(Value::as_bool).unwrap_or(false))
    }
    fn organization_of(&self, actor: Id) -> Result<Id> {
        let a = self.actor(actor);
        if a.get("organization_component").and_then(Value::as_bool).unwrap_or(false) {
            return Ok(actor);
        }
        Ok(a.get("element_org").and_then(Value::as_str).map_or(actor, |o| self.ids[o]))
    }
    fn definition_word_1b4(&self, object: Id) -> Result<i32> {
        Ok(self.actor(object).get("p").map_or(0, |v| v.as_i64().unwrap() as i32))
    }
    fn object_position(&self, object: Id) -> Result<[u32; 2]> {
        let a = self.actor(object);
        Ok([u(&a["xy"][0]), u(&a["xy"][1])])
    }
    fn set_look_at(&mut self, xy: [u32; 2]) -> Result<()> {
        self.look_at = Some(xy);
        Ok(())
    }
    fn log_look_at(&mut self, xy: [u32; 2]) -> Result<()> {
        let d = |b: u32| format!("{:016x}", f64::from(f32::from_bits(b)).to_bits());
        self.calls.push(json!({"seam": "404F60", "line": LOOK_AT_LINE, "function": LOOK_AT_FUNCTION,
            "format": LOOK_AT_FORMAT, "x": d(xy[0]), "y": d(xy[1])}));
        Ok(())
    }
    fn precision(&self) -> Precision {
        match self.c.get("precision").and_then(Value::as_str) {
            Some("Single24") => Precision::Single24,
            Some("Extended64") => Precision::Extended64,
            _ => Precision::Double53,
        }
    }
}

#[test]
fn load_tail_matches_original() {
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(FIXTURE).unwrap()).unwrap();
    assert_eq!(doc["passed"], true);
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 24);
    for case in cases {
        let c = &case["input"];
        let native = &case["native"];
        let name = c["name"].as_str().unwrap();
        let mut ids = BTreeMap::new();
        for (i, o) in c["kingdoms"].as_array().unwrap().iter().chain(c["actors"].as_array().unwrap()).enumerate() {
            ids.insert(o["label"].as_str().unwrap().to_owned(), 0x10001 + i as u32);
        }
        let mut world = World {
            c,
            ids,
            look_at: None,
            calls: Vec::new(),
        };
        match (after_post_load(&mut world), native["outcome"]["kind"].as_str()) {
            (Ok(()), None) => {
                let xy = world.look_at.unwrap();
                assert_eq!(json!(xy), native["look_at"], "{name}");
            }
            (Err(Error::NullDereference(_)), Some("NullDereference")) => assert!(world.look_at.is_none()),
            (r, k) => panic!("{name}: rust {r:?} native {k:?}"),
        }
        assert_eq!(Value::Array(world.calls), native["calls"], "{name}: calls");
    }
}
