//! 0x567960 / 0x567530 / 0x566D40 against
//! research/phase8-combat/native-defender-fixtures.json.
use aa_body::F32;
use aa_combat::{
    Result,
    defender::{Defender, DefenderWorld, tick},
    performer::Context,
};
use aa_numeric::Precision;
use serde_json::{Value, json};
use std::collections::HashMap;

const OWNER: u32 = 0x0F00;
const ABILITY: u32 = 0x100;

struct Adapter<'a> {
    case: &'a Value,
    /// actor id (index + 1) -> its +14 handle word.
    handles: Vec<u32>,
    /// ability index -> its +14 target handle.
    targets: Vec<u32>,
    events: Vec<Value>,
    calls: u32,
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
    fn words(&self, c: &Context) -> Value {
        let mut words = vec![json!(c.event_kind), json!(c.phase), self.label(c.source)];
        words.push(json!(c.allegiance.unwrap_or(0)));
        words.extend(c.source_position.iter().map(|f| json!(f.0)));
        words.push(self.label(c.target));
        words.extend(c.target_position.iter().map(|f| json!(f.0)));
        words.push(json!(c.word2c));
        Value::Array(words)
    }
    fn listed(&self, key: &str, actor: u32) -> bool {
        self.case[key]
            .as_array()
            .unwrap()
            .contains(&self.spec(actor)["id"])
    }
}
fn ability_index(ability: u32) -> usize {
    (ability - ABILITY) as usize
}
impl DefenderWorld for Adapter<'_> {
    fn resolve(&self, handle: u32) -> Result<Option<u32>> {
        if handle == 0 {
            return Ok(None);
        }
        Ok(self
            .handles
            .iter()
            .position(|&h| h == handle)
            .map(|k| k as u32 + 1)
            .filter(|&a| self.spec(a)["slot"] == "live"))
    }
    fn removed(&self, actor: u32) -> Result<bool> {
        Ok(self.spec(actor)["flags"].as_u64().unwrap() & 1 != 0)
    }
    fn handle_of(&self, actor: u32) -> Result<u32> {
        Ok(self.handles[(actor - 1) as usize])
    }
    fn position(&self, actor: u32) -> Result<[F32; 2]> {
        let p = if actor == OWNER {
            &self.case["owner_position"]
        } else {
            &self.spec(actor)["position"]
        };
        Ok([F32(p[0].as_u64().unwrap() as u32), F32(p[1].as_u64().unwrap() as u32)])
    }
    fn ability_target(&self, ability: u32) -> Result<u32> {
        Ok(self.targets[ability_index(ability)])
    }
    fn ability_kind_flag(&self, ability: u32) -> Result<bool> {
        Ok(self.case["abilities"][ability_index(ability)]["flags54"].as_u64().unwrap() & 8 != 0)
    }
    fn can_use(&mut self, ability: u32, context: &mut Context, check_range: bool) -> Result<u32> {
        let name = format!("a{}", ability_index(ability));
        let words = self.words(context);
        self.events.push(json!({"can_use": name, "check_range": u32::from(check_range),
            "context": words}));
        self.calls += 1;
        context.word2c = 0x100 + self.calls;
        let target = match &words[7] {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        Ok(self.case["admission"]
            .get(format!("{name}:{target}"))
            .map_or(0, |v| v.as_u64().unwrap() as u32))
    }
    fn set_target(&mut self, ability: u32, target: Option<u32>) -> Result<()> {
        self.events.push(json!({"set_target": format!("a{}", ability_index(ability)),
            "target": self.label(target)}));
        self.targets[ability_index(ability)] = target.map_or(0, |t| self.handles[(t - 1) as usize]);
        Ok(())
    }
    fn qualifies(&mut self, actor: u32) -> Result<bool> {
        self.events.push(json!({"qualifies": self.label(Some(actor))}));
        Ok(self.listed("qualifies", actor))
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
    fn aggressive(&mut self, actor: u32) -> Result<bool> {
        if !self.spec(actor)["component70"].as_bool().unwrap() {
            return Ok(false);
        }
        self.events.push(json!({"aggressive": self.label(Some(actor))}));
        Ok(self.listed("aggressive", actor))
    }
    fn harmless(&mut self, actor: u32) -> Result<bool> {
        self.events.push(json!({"harmless": self.label(Some(actor))}));
        Ok(self.listed("harmless", actor))
    }
    fn body_takeable(&self, actor: u32) -> Result<bool> {
        let s = self.spec(actor);
        Ok(s["field60"].as_u64().unwrap() != 0
            && s["data291"].as_u64().unwrap() != 0
            && s["flags"].as_u64().unwrap() & 0x800 == 0)
    }
    fn capturable(&mut self, actor: u32) -> Result<bool> {
        self.events.push(json!({"capturable": self.label(Some(actor))}));
        Ok(self.listed("capturable", actor))
    }
    fn ability_has_turret(&self, ability: u32) -> Result<bool> {
        Ok(self.ability_turret(ability)? != -1)
    }
    fn ability_turret(&self, ability: u32) -> Result<i32> {
        Ok(self.case["abilities"][ability_index(ability)]["turret"].as_i64().unwrap() as i32)
    }
    fn aim(&mut self, owner: u32, index: i32, point: [F32; 2], handle: u32) -> Result<bool> {
        assert_eq!(owner, OWNER);
        let target = self.handles.iter().position(|&h| h == handle).map(|k| k as u32 + 1);
        self.events.push(json!({"aim": index, "point": [point[0].0, point[1].0],
            "handle": self.label(target)}));
        Ok(self.case["aimed"].as_array().unwrap().contains(&self.label(target)))
    }
    fn use_ability(&mut self, owner: u32, ability: u32, context: &mut Context) -> Result<()> {
        assert_eq!(owner, OWNER);
        let words = self.words(context);
        self.events.push(json!({"use": format!("a{}", ability_index(ability)), "context": words}));
        Ok(())
    }
    fn refresh_radius(&mut self, owner: u32, position: [F32; 2], radius: F32) -> Result<()> {
        assert_eq!(owner, OWNER);
        self.events.push(json!({"radius": [position[0].0, position[1].0, radius.0],
            "zone": u32::from(self.case["zone"].as_bool().unwrap())}));
        Ok(())
    }
}

#[test]
fn defender_update_matches_native() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-defender-fixtures.json"
    ))
    .unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 450);
    for fixture in cases {
        let precision: Precision = serde_json::from_value(fixture["precision"].clone()).unwrap();
        let case = &fixture["case"];
        let actors = case["actors"].as_array().unwrap();
        let handles: Vec<u32> = actors
            .iter()
            .map(|a| fixture["handles"][a["id"].as_str().unwrap()].as_u64().unwrap() as u32)
            .collect();
        let by_label: HashMap<String, u32> = actors
            .iter()
            .zip(&handles)
            .map(|(a, &h)| (a["id"].as_str().unwrap().to_string(), h))
            .collect();
        let handle = |v: &Value| v.as_str().map_or(0, |s| by_label[s]);
        let abilities = case["abilities"].as_array().unwrap();
        let mut world = Adapter {
            case,
            handles: handles.clone(),
            targets: abilities.iter().map(|a| handle(&a["target"])).collect(),
            events: vec![],
            calls: 0,
        };
        let mut defender = Defender {
            owner: OWNER,
            abilities: (0..abilities.len() as u32).map(|k| ABILITY + k).collect(),
            intruders: case["intruders"].as_array().unwrap().iter().map(handle).collect(),
            radius: F32(case["radius"].as_u64().unwrap() as u32),
            radius_dirty: case["radius_dirty"].as_bool().unwrap(),
        };
        tick(&mut defender, &mut world, precision).unwrap();
        assert_eq!(Value::Array(world.events.clone()), fixture["events"], "{fixture}");
        let after: Vec<Value> = world
            .targets
            .iter()
            .map(|&t| match handles.iter().position(|&h| h == t && t != 0) {
                Some(k) => actors[k]["id"].clone(),
                None => json!(t),
            })
            .collect();
        assert_eq!(Value::Array(after), fixture["targets_after"], "{fixture}");
        assert_eq!(
            u64::from(defender.radius_dirty),
            fixture["radius_dirty_after"].as_u64().unwrap()
        );
    }
}
