//! Replays research/phase8-combat/native-combat-reaction-fixtures.json:
//! original 501770/4C9EB0, 4D3FA0 and 4D6190/4D3A70/4D1B40 under Unicorn.
use aa_body::{F32, credit::Object, experience::Request, experience_amounts};
use aa_combat::{
    Result,
    morale::{self, RoutWorld},
    performer::Context,
    reactions::{self, AttackWorld, AttackedBy, DefenderWorld, Membership},
};
use aa_numeric::{Binary, Precision};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}

fn context_words(c: &Context) -> Value {
    json!([
        c.event_kind,
        c.phase,
        c.source.unwrap_or(0),
        c.allegiance.unwrap_or(0),
        c.source_position[0].0,
        c.source_position[1].0,
        c.source_position[2].0,
        c.target.unwrap_or(0),
        c.target_position[0].0,
        c.target_position[1].0,
        c.target_position[2].0,
        c.word2c
    ])
}

struct Defender {
    abilities: Vec<Value>,
    now: F32,
    events: Vec<Value>,
}
impl DefenderWorld for Defender {
    fn ability_count(&self) -> Result<i32> {
        Ok(self.abilities.len() as i32)
    }
    fn ability(&self, index: i32) -> Result<u32> {
        Ok(100 + index as u32)
    }
    fn can_use(&mut self, ability: u32, c: &mut Context, check_range: bool) -> Result<u32> {
        assert!(check_range);
        self.events
            .push(json!({"CanUse": [ability, context_words(c)]}));
        let entry = &self.abilities[(ability - 100) as usize];
        if let Some(m) = entry["mutate"].as_object() {
            for (offset, value) in m {
                let v = u(value);
                match offset.as_str() {
                    "4" => c.phase = v,
                    "12" => c.allegiance = (v != 0).then_some(v),
                    "28" => c.target = (v != 0).then_some(v),
                    other => panic!("{other}"),
                }
            }
        }
        Ok(u(&entry["can_use"]))
    }
    fn use_ability(&mut self, ability: u32, c: &mut Context, time: F32) -> Result<u32> {
        self.events
            .push(json!({"Use": [ability, context_words(c), time.0]}));
        Ok(1)
    }
    fn now(&self) -> Result<F32> {
        Ok(self.now)
    }
}

struct Actors {
    input: Value,
    values: BTreeMap<String, Vec<u64>>,
    events: Vec<Value>,
    attacked: BTreeMap<u32, Vec<AttackedBy>>,
    engaged: BTreeMap<u32, Vec<u32>>,
}
impl Actors {
    fn new(input: &Value) -> Self {
        let mut attacked = BTreeMap::new();
        let mut engaged = BTreeMap::new();
        for tag in input["actors"].as_object().unwrap().keys() {
            let t: u32 = tag.parse().unwrap();
            attacked.insert(t, vec![]);
            engaged.insert(t, vec![]);
        }
        if let Some(list) = input["list"].as_array() {
            attacked.insert(
                1,
                list.iter()
                    .map(|e| AttackedBy {
                        attacker_id: u(&e[0]),
                        time: F32(u(&e[1])),
                        timeout: F32(u(&e[2])),
                    })
                    .collect(),
            );
        }
        let values = input["values"]
            .as_object()
            .map(|m| {
                m.iter()
                    .map(|(k, v)| {
                        (
                            k.clone(),
                            v.as_array()
                                .unwrap()
                                .iter()
                                .map(|x| x.as_u64().unwrap())
                                .collect(),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self {
            input: input.clone(),
            values,
            events: vec![],
            attacked,
            engaged,
        }
    }
    fn view(&self, actor: u32) -> &Value {
        &self.input["actors"][actor.to_string()]
    }
    fn message(&mut self, actor: u32, event: u32) {
        let allegiance = self.view(actor)["allegiance"].as_u64().unwrap_or(0);
        self.events.push(json!({"Name": actor}));
        self.events.push(json!({"Allegiance": actor}));
        self.events
            .push(json!({"Message": [event, allegiance, actor]}));
    }
}
impl experience_amounts::World for Actors {
    fn actor_value(&mut self, actor: Object, _: Precision) -> aa_body::Result<Binary> {
        self.events.push(json!({"ActorValue": actor.0}));
        let bits = self.values.get_mut(&actor.0.to_string()).unwrap().remove(0);
        Ok(Binary::from_f64_bits(bits).unwrap())
    }
    fn definition4f4(&self, _: Object) -> aa_body::Result<F32> {
        Ok(F32::ZERO)
    }
}
impl RoutWorld for Actors {
    fn rout_message(&mut self, routed: u32) -> Result<()> {
        self.message(routed, 0xc);
        Ok(())
    }
    fn engaged_by(&self, _: u32) -> Result<Vec<u32>> {
        Ok(self.input["engaged_by"]
            .as_array()
            .unwrap()
            .iter()
            .map(u)
            .collect())
    }
    fn resolve(&self, gtid: u32) -> Result<Option<u32>> {
        let tag = gtid & 0xffff;
        Ok((self.view(tag)["registered"] == true && gtid == 0x10000 * tag + tag).then_some(tag))
    }
    fn has_organization(&self, actor: u32) -> Result<bool> {
        Ok(self.view(actor)["organization"] == true)
    }
    fn breaker(&mut self, attacker: u32, routed: u32) -> Result<()> {
        self.events.push(json!({"Breaker": [attacker, routed]}));
        Ok(())
    }
    fn award(&mut self, actor: u32, r: Request) -> Result<()> {
        self.events
            .push(json!({"Award": [actor, r.amount.0, r.argument1 as u8, r.argument2 as u8]}));
        Ok(())
    }
}
impl AttackWorld for Actors {
    fn virtual_a8(&mut self, actor: u32) -> Result<bool> {
        self.events.push(json!({"VirtualA8": actor}));
        Ok(self.view(actor)["a8"] == true)
    }
    fn membership(&self, actor: u32) -> Result<Option<Membership>> {
        let m = &self.view(actor)["membership"];
        Ok((!m.is_null()).then(|| Membership {
            organization: (u(&m["organization"]) != 0).then(|| u(&m["organization"])),
            attached: m["attached"] == true,
        }))
    }
    fn outer(&mut self, actor: u32) -> Result<Option<u32>> {
        self.events.push(json!({"Outer": actor}));
        let o = self.view(actor)["outer"].as_u64().unwrap_or(0) as u32;
        Ok((o != 0).then_some(o))
    }
    fn actor_id(&self, actor: u32) -> Result<u32> {
        Ok(0x10000 * actor + actor)
    }
    fn flags8(&self, actor: u32) -> Result<u32> {
        Ok(self.view(actor)["flags8"].as_u64().unwrap_or(0) as u32)
    }
    fn attacked_by(&mut self, actor: u32) -> Result<&mut Vec<AttackedBy>> {
        Ok(self.attacked.get_mut(&actor).unwrap())
    }
    fn engaged(&mut self, actor: u32) -> Result<&mut Vec<u32>> {
        Ok(self.engaged.get_mut(&actor).unwrap())
    }
    fn has_mind(&self, actor: u32) -> Result<bool> {
        Ok(!self.view(actor)["mind"].is_null())
    }
    fn mind_accepts_weapon_bonus(&mut self, actor: u32) -> Result<bool> {
        self.events.push(json!({"Mind6C": actor}));
        Ok(self.view(actor)["mind"] == true)
    }
    fn now(&self) -> Result<F32> {
        Ok(F32(u(&self.input["now"])))
    }
    fn engagement_message(&mut self, attacker: u32, event: u32) -> Result<()> {
        self.message(attacker, event);
        Ok(())
    }
}

#[test]
fn original_combat_reactions_match_every_fixture() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-combat-reaction-fixtures.json"
    ))
    .unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert!(cases.len() >= 80);
    for row in cases {
        let i = &row["input"];
        let out = &row["output"];
        let name = format!("{} {}", i["name"], i["precision"]);
        let p: Precision = serde_json::from_value(i["precision"].clone()).unwrap();
        match i["operation"].as_str().unwrap() {
            "defender_abilities" => {
                let mut host = Defender {
                    abilities: i["abilities"].as_array().unwrap().clone(),
                    now: F32(u(&i["now"])),
                    events: vec![],
                };
                reactions::defender_abilities(1, 2, &mut host).unwrap();
                assert_eq!(json!(host.events), out["events"], "{name}");
            }
            "rout" => {
                let mut host = Actors::new(i);
                let settings = experience_amounts::Settings {
                    minimum: F32(0.25f32.to_bits()),
                    maximum: F32(6f32.to_bits()),
                    kill: F32::ZERO,
                    capture: F32::ZERO,
                    rout: F32(2.5f32.to_bits()),
                    engagement: F32::ZERO,
                    default_value: F32(1f32.to_bits()),
                };
                morale::rout(1, &settings, &mut host, p).unwrap();
                assert_eq!(json!(host.events), out["events"], "{name}");
            }
            "attacked_by" => {
                let mut host = Actors::new(i);
                let weapon = match &i["weapon"] {
                    Value::Null => None,
                    v if u(v) == 0 => Some(None),
                    v => Some(Some(F32(u(v)))),
                };
                reactions::attacked_by(
                    1,
                    u(&i["source"]),
                    weapon,
                    F32(u(&i["bombard_timeout"])),
                    &mut host,
                    p,
                )
                .unwrap();
                let attacked: BTreeMap<String, Value> = host
                    .attacked
                    .iter()
                    .map(|(k, v)| {
                        (
                            k.to_string(),
                            json!(
                                v.iter()
                                    .map(|e| json!([e.attacker_id, e.time.0, e.timeout.0]))
                                    .collect::<Vec<_>>()
                            ),
                        )
                    })
                    .collect();
                let tails: BTreeMap<String, bool> = host
                    .attacked
                    .iter()
                    .map(|(k, v)| (k.to_string(), !v.is_empty()))
                    .collect();
                let engaged: BTreeMap<String, Value> = host
                    .engaged
                    .iter()
                    .map(|(k, v)| {
                        (
                            k.to_string(),
                            json!(v.iter().map(|g| json!([g])).collect::<Vec<_>>()),
                        )
                    })
                    .collect();
                assert_eq!(json!(attacked), out["attacked_lists"], "{name}");
                assert_eq!(json!(tails), out["attacked_tail_present"], "{name}");
                assert_eq!(json!(engaged), out["engaged_lists"], "{name}");
                assert_eq!(json!(host.events), out["events"], "{name}");
            }
            other => panic!("{other}"),
        }
    }
}
