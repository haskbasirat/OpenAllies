//! Replays research/phase8-combat/native-terrain-refresh-fixtures.json:
//! original 4D4130 under Unicorn with recorded seams; call order and the
//! final +F8 terrain must match.
use aa_actors::terrain_refresh::*;
use serde_json::{Value, json};

const ACTOR: u32 = 1;
const PROPERTIES: u32 = 2;
const ORGANIZATION: u32 = 3;
const MEMBERSHIP: u32 = 4;
/// Terrain t is handle 10 + t; property k is handle 20 + k.
fn terrain_handle(v: &Value) -> Option<u32> {
    v.as_u64().map(|t| 10 + t as u32)
}

struct Host<'a> {
    case: &'a Value,
    terrain: Option<u32>,
    events: Vec<Value>,
}

impl Host<'_> {
    fn flag(&self, key: &str) -> bool {
        self.case[key].as_bool().unwrap()
    }
}

impl World for Host<'_> {
    fn actor_flags(&mut self, _: u32) -> Result<u32> {
        Ok(self.case["flags"].as_u64().unwrap() as u32)
    }
    fn check_4d10a0(&mut self, _: u32) -> Result<bool> {
        self.events.push(json!(["check"]));
        Ok(self.case["check"].as_u64().unwrap() & 0xFF != 0)
    }
    fn word100(&mut self, _: u32) -> Result<u32> {
        Ok(self.case["word100"].as_u64().unwrap() as u32)
    }
    fn terrain(&mut self, _: u32) -> Result<Option<u32>> {
        Ok(self.terrain)
    }
    fn set_terrain(&mut self, _: u32, terrain: Option<u32>) -> Result<()> {
        self.terrain = terrain;
        Ok(())
    }
    fn membership(&mut self, _: u32) -> Result<Option<u32>> {
        Ok(self.flag("membership").then_some(MEMBERSHIP))
    }
    fn membership_terrain(&mut self, m: u32) -> Result<Option<u32>> {
        assert_eq!(m, MEMBERSHIP);
        self.events.push(json!(["membership_terrain"]));
        Ok(terrain_handle(&self.case["new"]))
    }
    fn tile(&mut self, _: u32) -> Result<[u32; 2]> {
        let t = &self.case["tile"];
        Ok([t[0].as_u64().unwrap() as u32, t[1].as_u64().unwrap() as u32])
    }
    fn tile_terrain(&mut self, tile: [u32; 2]) -> Result<Option<u32>> {
        self.events.push(json!(["tile_terrain", tile[0], tile[1]]));
        Ok(terrain_handle(&self.case["new"]))
    }
    fn organization(&mut self, _: u32) -> Result<Option<u32>> {
        Ok(self.flag("organization").then_some(ORGANIZATION))
    }
    fn organization_refresh(&mut self, o: u32) -> Result<()> {
        assert_eq!(o, ORGANIZATION);
        self.events.push(json!(["organization_refresh"]));
        Ok(())
    }
    fn properties(&mut self, _: u32) -> Result<Option<u32>> {
        Ok(self.flag("properties").then_some(PROPERTIES))
    }
    fn terrain_properties(&mut self, terrain: u32) -> Result<Vec<Option<u32>>> {
        Ok(self.case["terrains"][(terrain - 10) as usize]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().map(|k| 20 + k as u32))
            .collect())
    }
    fn remove(&mut self, properties: u32, property: Option<u32>, actor: u32) -> Result<()> {
        assert_eq!((properties, actor), (PROPERTIES, ACTOR));
        self.events.push(json!(["remove", property.map(|p| p - 20)]));
        Ok(())
    }
    fn applicable(&mut self, actor: u32, property: u32) -> Result<bool> {
        assert_eq!(actor, ACTOR);
        self.events.push(json!(["applicable", property - 20]));
        Ok(self.case["applicable"][(property - 20) as usize].as_u64().unwrap() & 0xFF != 0)
    }
    fn add(&mut self, properties: u32, property: u32, kind: u32, actor: u32) -> Result<()> {
        assert_eq!((properties, actor), (PROPERTIES, ACTOR));
        self.events.push(json!(["add", property - 20, kind]));
        Ok(())
    }
}

#[test]
fn original_terrain_refresh_matches_every_fixture() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-terrain-refresh-fixtures.json"
    ))
    .unwrap();
    assert_eq!(doc["passed"], true);
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 300);
    for (i, case) in cases.iter().enumerate() {
        let mut host = Host { case, terrain: terrain_handle(&case["old"]), events: Vec::new() };
        refresh_terrain(&mut host, ACTOR).unwrap();
        assert_eq!(json!(host.terrain.map(|t| t - 10)), case["final"], "case {i} final");
        assert_eq!(Value::Array(host.events), case["events"], "case {i} events");
    }
}
