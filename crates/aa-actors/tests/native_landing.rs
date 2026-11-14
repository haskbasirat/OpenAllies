//! Replays research/phase8-combat/native-landing-fixtures.json: original
//! 4D64A0 (landing refresh) and 4D11D0 (siege site) under Unicorn with
//! recorded seams; call order and the returned word must match.
use aa_actors::{landing, siege_site};
use serde_json::{Value, json};

const ACTOR: u32 = 1;
const BODY: u32 = 2;
const CHARACTER: u32 = 3;
const MIND: u32 = 4;
const TARGET: u32 = 5;

struct Land<'a> {
    case: &'a Value,
    events: Vec<Value>,
}

impl landing::World for Land<'_> {
    fn actor_flags(&mut self, _: u32) -> landing::Result<u32> {
        Ok(self.case["flags"].as_u64().unwrap() as u32)
    }
    fn body(&mut self, _: u32) -> landing::Result<Option<u32>> {
        Ok(self.case["body"].as_bool().unwrap().then_some(BODY))
    }
    fn body_562380(&mut self, body: u32) -> landing::Result<()> {
        assert_eq!(body, BODY);
        self.events.push(json!(["562380", "body"]));
        Ok(())
    }
    fn world_4f6760(&mut self, actor: u32) -> landing::Result<()> {
        assert_eq!(actor, ACTOR);
        self.events.push(json!(["4F6760", "world", "actor", 0]));
        Ok(())
    }
    fn refresh_terrain(&mut self, _: u32) -> landing::Result<()> {
        self.events.push(json!(["4D4130", "actor"]));
        Ok(())
    }
    fn word_bc(&mut self, _: u32) -> landing::Result<u32> {
        Ok(self.case["word_bc"].as_u64().unwrap() as u32)
    }
    fn character(&mut self, _: u32) -> landing::Result<Option<u32>> {
        Ok(self.case["character"].as_bool().unwrap().then_some(CHARACTER))
    }
    fn character_563d50(&mut self, character: u32) -> landing::Result<()> {
        assert_eq!(character, CHARACTER);
        self.events.push(json!(["563D50", "character"]));
        Ok(())
    }
    fn notify_4d1350(&mut self, _: u32) -> landing::Result<()> {
        self.events.push(json!(["4D1350", "actor"]));
        Ok(())
    }
}

struct Siege<'a> {
    case: &'a Value,
    events: Vec<Value>,
}

impl siege_site::World for Siege<'_> {
    fn virtual100(&mut self, thing: u32) -> siege_site::Result<u32> {
        let (tag, key) = if thing == ACTOR { ("actor", "own") } else { ("target", "target_value") };
        self.events.push(json!(["virtual100", tag]));
        Ok(self.case[key].as_u64().unwrap() as u32)
    }
    fn mind(&mut self, _: u32) -> siege_site::Result<Option<u32>> {
        Ok((!self.case["mind"].is_null()).then_some(MIND))
    }
    fn mind_target_gtid(&mut self, _: u32) -> siege_site::Result<u32> {
        Ok(self.case["mind"]["gtid"].as_u64().unwrap() as u32)
    }
    fn registry_generation(&mut self, _: u32) -> siege_site::Result<u16> {
        Ok(self.case["mind"]["generation"].as_u64().unwrap() as u16)
    }
    fn registry_object(&mut self, _: u32) -> siege_site::Result<Option<u32>> {
        Ok(self.case["mind"]["live"].as_bool().unwrap().then_some(TARGET))
    }
    fn position(&mut self, _: u32) -> siege_site::Result<[u32; 2]> {
        let p = &self.case["position"];
        Ok([p[0].as_u64().unwrap() as u32, p[1].as_u64().unwrap() as u32])
    }
    fn world_4f2220(&mut self, point: [u32; 2]) -> siege_site::Result<u32> {
        self.events.push(json!(["4F2220", point[0], point[1]]));
        Ok(self.case["site"].as_u64().unwrap() as u32)
    }
}

#[test]
fn original_landing_and_siege_site_match_every_fixture() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-landing-fixtures.json"
    ))
    .unwrap();
    assert_eq!(doc["passed"], true);
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 120);
    for (i, case) in cases.iter().enumerate() {
        let mut host = Land { case, events: Vec::new() };
        landing::land(&mut host, ACTOR).unwrap();
        assert_eq!(Value::Array(host.events), case["events"], "landing {i}");
    }
    let sieges = doc["sieges"].as_array().unwrap();
    assert_eq!(sieges.len(), 120);
    for (i, case) in sieges.iter().enumerate() {
        let mut host = Siege { case, events: Vec::new() };
        let result = siege_site::siege_site(&mut host, ACTOR).unwrap();
        assert_eq!(json!(result), case["result"], "siege {i}");
        assert_eq!(Value::Array(host.events), case["events"], "siege {i} events");
    }
}
