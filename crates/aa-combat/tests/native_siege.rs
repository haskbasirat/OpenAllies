//! Replays research/phase8-combat/native-siege-fixtures.json: original
//! 4D6660 (under-siege refresh) and 4D0440 (recently damaged).
use aa_body::F32;
use aa_combat::{
    Result,
    siege::{self, World},
};
use aa_numeric::Precision;
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}

struct Host {
    c: Value,
    outers: BTreeMap<u32, Vec<u32>>,
    events: Vec<Value>,
}
impl Host {
    fn view(&self, actor: u32) -> &Value {
        &self.c["actors"][actor.to_string()]
    }
    fn wall_tag(&self, actor: u32) -> Value {
        let v = self.view(actor);
        if v["wall"] == true || v["damage"][0] == "wall" {
            json!(format!("wall:{actor}"))
        } else {
            json!(-1)
        }
    }
}
impl World for Host {
    fn flags8(&self, a: u32) -> Result<u32> {
        Ok(self.view(a)["flags8"].as_u64().unwrap_or(0) as u32)
    }
    fn flags100(&self, a: u32) -> Result<u32> {
        Ok(self.view(a)["flags100"].as_u64().unwrap_or(0) as u32)
    }
    fn last_damage_time(&self, a: u32) -> Result<Option<F32>> {
        let d = &self.view(a)["damage"];
        Ok((!d.is_null()).then(|| F32(u(&d[1]))))
    }
    fn has_wall(&self, a: u32) -> Result<bool> {
        let v = self.view(a);
        Ok(v["wall"] == true || v["damage"][0] == "wall")
    }
    fn has_wall_link(&self, a: u32) -> Result<bool> {
        Ok(self.view(a)["link90"] == true)
    }
    fn outer(&mut self, a: u32) -> Result<Option<u32>> {
        self.events.push(json!({"Outer": a}));
        let seq = self.outers.entry(a).or_default();
        let t = if seq.len() > 1 {
            seq.remove(0)
        } else {
            seq.first().copied().unwrap_or(0)
        };
        Ok((t != 0).then_some(t))
    }
    fn delegates(&mut self, a: u32) -> Result<bool> {
        self.events.push(json!({"Delegates": a}));
        Ok(self.view(a)["delegates"] == true)
    }
    fn engaged(&self, a: u32) -> Result<bool> {
        Ok(self.view(a)["engaged_list"] == true)
    }
    fn watched_flag(&self, a: u32) -> Result<bool> {
        Ok(self.view(a)["d4"].as_u64().is_some_and(|b| b != 0))
    }
    fn component88_gate(&self, a: u32) -> Result<bool> {
        let g = &self.view(a)["c88"];
        Ok(!g.is_null()
            && u(&g["count"]) as i32 > 5
            && g["object14"] == true
            && g["field24"] == true)
    }
    fn wall_predicate(&mut self, outer: u32) -> Result<bool> {
        let r = self.c["wall_predicate"] == true;
        let tag = self.wall_tag(outer);
        self.events.push(json!({"WallPredicate": [tag, null, r]}));
        Ok(r)
    }
    fn component88_predicate(&mut self, a: u32, outer: u32) -> Result<bool> {
        let r = self.c["c88_predicate"] == true;
        self.events
            .push(json!({"Component88Predicate": [format!("c88:{a}"), outer, r]}));
        Ok(r)
    }
    fn wall_update(&mut self, outer: u32) -> Result<()> {
        let tag = self.wall_tag(outer);
        self.events.push(json!({"WallUpdate": [tag, null, null]}));
        Ok(())
    }
    fn set_state(&mut self, _: u32, state: u8, enabled: bool) -> Result<()> {
        self.events.push(json!({"SetState": [state, enabled]}));
        Ok(())
    }
    fn notify(&mut self, a: u32) -> Result<()> {
        self.events.push(json!({ "Notify": a }));
        Ok(())
    }
    fn now(&self) -> Result<F32> {
        Ok(F32(u(&self.c["now"])))
    }
}

#[test]
fn original_under_siege_refresh_matches_every_fixture() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-siege-fixtures.json"
    ))
    .unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert!(cases.len() >= 200);
    for row in cases {
        let c = &row["input"];
        let name = format!("{} {}", c["name"], c["precision"]);
        let p: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let mut outers = BTreeMap::new();
        for (tag, v) in c["actors"].as_object().unwrap() {
            if let Some(seq) = v["outer"].as_array() {
                outers.insert(tag.parse().unwrap(), seq.iter().map(u).collect());
            }
        }
        let mut host = Host {
            c: c.clone(),
            outers,
            events: vec![],
        };
        let threshold = F32(u(&c["threshold"]));
        if c["operation"] == "recently_damaged" {
            let r = siege::recently_damaged(1, threshold, &host, p).unwrap();
            assert_eq!(json!(r as u8), row["output"]["result"], "{name}");
        } else {
            siege::refresh_under_siege(1, threshold, &mut host, p).unwrap();
        }
        assert_eq!(json!(host.events), row["output"]["events"], "{name}");
    }
}
