//! 0x5673B0 / 0x566F80 against
//! research/phase8-combat/native-defender-reach-fixtures.json.
use aa_body::F32;
use aa_combat::{
    Result,
    defender::{Defender, RadiusWorld, ReachWorld, reachable, set_radius},
    performer::Context,
};
use aa_numeric::{Fast, Precision};
use serde_json::{Value, json};

const OWNER: u32 = 0x0F00;
const ABILITY: u32 = 0x100;

struct Adapter<'a> {
    case: &'a Value,
    events: Vec<Value>,
}
impl Adapter<'_> {
    fn spec(&self, actor: u32) -> &Value {
        &self.case["actors"][(actor - 1) as usize]
    }
    fn label(&self, actor: Option<u32>) -> Value {
        match actor {
            None => json!(0),
            Some(OWNER) => json!("owner"),
            Some(a) => self.spec(a)["id"].clone(),
        }
    }
    fn id_of(&self, label: &Value) -> Option<u32> {
        self.case["actors"]
            .as_array()
            .unwrap()
            .iter()
            .position(|a| &a["id"] == label)
            .map(|k| k as u32 + 1)
    }
    fn refused(&self, name: &str, ability: u32, target: &Value) -> bool {
        let target = target.as_str().map_or_else(|| target.to_string(), str::to_string);
        let key = json!(format!("{name}:a{}:{target}", ability - ABILITY));
        self.case["refuse"].as_array().unwrap().contains(&key)
    }
    fn at(&self, point: [F32; 2]) -> Value {
        let xy = json!([point[0].0, point[1].0]);
        self.case["actors"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["position"] == xy)
            .unwrap()["id"]
            .clone()
    }
}
impl ReachWorld for Adapter<'_> {
    fn qualifies(&mut self, actor: u32) -> Result<bool> {
        let label = self.label(Some(actor));
        self.events.push(json!({"qualifies": label}));
        Ok(self.case["qualifies"].as_array().unwrap().contains(&label))
    }
    fn organization_members(&mut self, actor: u32) -> Result<Option<Vec<u32>>> {
        let spec = self.spec(actor);
        if spec["members"].is_null() {
            return Ok(None);
        }
        let members = spec["members"].as_array().unwrap();
        Ok(Some(
            spec["order"]
                .as_array()
                .unwrap()
                .iter()
                .map(|i| self.id_of(&members[i.as_u64().unwrap() as usize]).unwrap_or(0))
                .collect(),
        ))
    }
    fn flags(&self, actor: u32) -> Result<u32> {
        Ok(self.spec(actor)["flags"].as_u64().unwrap() as u32)
    }
    fn position(&self, actor: u32) -> Result<[F32; 2]> {
        let p = if actor == OWNER {
            &self.case["owner_position"]
        } else {
            &self.spec(actor)["position"]
        };
        Ok([F32(p[0].as_u64().unwrap() as u32), F32(p[1].as_u64().unwrap() as u32)])
    }
    fn admits(&mut self, ability: u32, context: &mut Context) -> Result<bool> {
        let mut words = vec![json!(context.event_kind), json!(context.phase), self.label(context.source)];
        words.push(json!(context.allegiance.unwrap_or(0)));
        words.extend(context.source_position.iter().map(|f| json!(f.0)));
        words.push(self.label(context.target));
        words.extend(context.target_position.iter().map(|f| json!(f.0)));
        words.push(json!(context.word2c));
        let target = self.label(context.target);
        self.events.push(json!({"admits": format!("a{}", ability - ABILITY),
            "context": words, "flag": 1}));
        Ok(!self.refused("admits", ability, &target))
    }
    fn within(&mut self, ability: u32, owner: u32, point: [F32; 2]) -> Result<bool> {
        assert_eq!(owner, OWNER);
        self.events.push(json!({"within": format!("a{}", ability - ABILITY),
            "point": [point[0].0, point[1].0]}));
        let target = self.at(point);
        Ok(!self.refused("within", ability, &target))
    }
    fn clear(&mut self, ability: u32, owner: u32, point: [F32; 2]) -> Result<bool> {
        assert_eq!(owner, OWNER);
        self.events.push(json!({"clear": format!("a{}", ability - ABILITY),
            "point": [point[0].0, point[1].0]}));
        let target = self.at(point);
        Ok(!self.refused("clear", ability, &target))
    }
    fn has_projectile(&self, ability: u32) -> Result<bool> {
        Ok(self.case["abilities"][(ability - ABILITY) as usize]["projectile"]
            .as_u64()
            .unwrap()
            != 0)
    }
    fn range(&mut self, ability: u32, owner: u32) -> Result<Fast> {
        assert_eq!(owner, OWNER);
        let name = format!("a{}", ability - ABILITY);
        self.events.push(json!({"modifiers": 1}));
        self.events.push(json!({"range": name}));
        Ok(Fast::from_f64_bits(self.case["ranges"][&name].as_u64().unwrap())?)
    }
}
impl RadiusWorld for Adapter<'_> {
    fn wide_radius(&self, owner: u32) -> Result<bool> {
        assert_eq!(owner, OWNER);
        Ok(self.case["data174"].as_u64().unwrap() & 0x80 != 0)
    }
    fn radius(&mut self, owner: u32, wide: bool, argument: u32) -> Result<Fast> {
        assert_eq!(owner, OWNER);
        self.events.push(json!({"radius": if wide { "wide" } else { "normal" }, "arg": argument}));
        Ok(Fast::from_f64_bits(self.case["radius_value"].as_u64().unwrap())?)
    }
}

#[test]
fn defender_reach_and_radius_match_native() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-defender-reach-fixtures.json"
    ))
    .unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 480);
    for fixture in cases {
        let precision: Precision = serde_json::from_value(fixture["precision"].clone()).unwrap();
        let case = &fixture["case"];
        let mut world = Adapter { case, events: vec![] };
        let count = case["abilities"].as_array().unwrap().len() as u32;
        let mut defender = Defender {
            owner: OWNER,
            abilities: (0..count).map(|k| ABILITY + k).collect(),
            intruders: vec![],
            radius: F32(0x1111_1111),
            radius_dirty: false,
        };
        let candidate = world.id_of(&case["candidate"]);
        let reached = reachable(&defender, candidate, &mut world, precision).unwrap();
        assert_eq!(u64::from(reached), fixture["result"].as_u64().unwrap(), "{fixture}");
        assert_eq!(Value::Array(std::mem::take(&mut world.events)), fixture["events"], "{fixture}");
        let argument = case["radius_arg"].as_u64().unwrap() as u32;
        set_radius(&mut defender, argument, &mut world).unwrap();
        assert_eq!(Value::Array(world.events), fixture["radius_events"], "{fixture}");
        assert_eq!(u64::from(defender.radius.0), fixture["radius"].as_u64().unwrap());
        assert_eq!(u64::from(defender.radius_dirty), fixture["radius_dirty"].as_u64().unwrap());
    }
}
