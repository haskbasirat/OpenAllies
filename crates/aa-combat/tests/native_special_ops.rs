//! Special operations against
//! research/phase8-combat/native-special-ops-effects-fixtures.json.
use aa_body::{
    F32,
    credit::Object,
    property_api::{LookupWorld, find_actor_property, find_instance},
};
use aa_combat::{Result, ability::Ability, commander, performer::Context, special_ops::*};
use aa_numeric::Precision;
use serde_json::{Value, json};

fn doc() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-special-ops-effects-fixtures.json"
    ))
    .unwrap()
}
fn bits(v: &Value) -> F32 {
    F32(v.as_u64().unwrap() as u32)
}

const OWNER: u32 = 0x100;
const TARGET: u32 = 0x200;

struct Faction<'a> {
    case: &'a Value,
    events: Vec<Value>,
}
fn context_words(c: &Context, target: Option<u32>) -> Value {
    let mut w = vec![
        json!(c.event_kind),
        json!(c.phase),
        json!(u32::from(c.source == Some(OWNER))),
        json!(u32::from(c.allegiance == Some(OWNER))),
    ];
    w.extend(c.source_position.iter().map(|f| json!(f.0)));
    w.push(json!(u32::from(target.is_some() && c.target == target)));
    w.push(json!(c.target.is_none()));
    w.extend(c.target_position.iter().map(|f| json!(f.0)));
    w.push(json!(c.word2c));
    Value::Array(w)
}
impl FactionWorld for Faction<'_> {
    fn can_use(
        &mut self,
        _: &mut commander::State,
        index: usize,
        context: &mut Context,
    ) -> Result<u32> {
        let target = self.case["target"].as_bool().unwrap().then_some(TARGET);
        self.events.push(json!({"can_use": index, "check_range": 1,
            "context": context_words(context, target)}));
        Ok(self.case["can_use"].as_u64().unwrap() as u32)
    }
    fn use_ability(
        &mut self,
        _: &mut commander::State,
        index: usize,
        context: &mut Context,
        time: F32,
    ) -> Result<i32> {
        self.events.push(
            json!({"use": index, "time": time.0, "context_kind": context.event_kind,
            "target_position": context.target_position.iter().map(|f| f.0).collect::<Vec<_>>()}),
        );
        Ok(self.case["use_result"].as_i64().unwrap() as i32)
    }
    fn world_time(&self) -> Result<F32> {
        Ok(F32(123.5f32.to_bits()))
    }
    fn actor_position(&self, actor: u32) -> Result<[F32; 3]> {
        assert_eq!(actor, TARGET);
        let p = &self.case["target_position"];
        Ok([bits(&p[0]), bits(&p[1]), bits(&p[2])])
    }
    fn unable(&mut self, _: &commander::State, _: u32) -> Result<()> {
        self.events.push(json!({"log": true}));
        Ok(())
    }
}

#[test]
fn faction_use_ability_matches_native() {
    let doc = doc();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 40);
    for case in cases {
        let abilities = case["definitions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| Ability::new(1000 + d.as_u64().unwrap() as u32, None, F32::ZERO))
            .collect();
        let mut state = commander::State {
            definition: 1,
            kingdom: OWNER,
            experience: aa_economy::tick::CommanderExperience {
                balance: aa_economy::F32Bits::ZERO,
                maximum: aa_economy::F32Bits::ZERO,
            },
            abilities,
        };
        let mut world = Faction {
            case,
            events: vec![],
        };
        let target = case["target"].as_bool().unwrap().then_some(TARGET);
        let point = [bits(&case["point"][0]), bits(&case["point"][1])];
        let wanted = 1000 + case["wanted"].as_u64().unwrap() as u32;
        let got = use_ability(&mut state, wanted, target, point, &mut world).unwrap();
        assert_eq!(u64::from(got), case["returned"].as_u64().unwrap(), "{case}");
        assert_eq!(Value::Array(world.events), case["events"], "{case}");
    }
}

#[test]
fn heal_modifier_matches_native() {
    for case in doc()["heal_modifier"].as_array().unwrap() {
        let got = heal_modifier(
            bits(&case["amount"]),
            bits(&case["bonus"]),
            bits(&case["cap"]),
            Precision::Double53,
        )
        .unwrap();
        assert_eq!(u64::from(got.0), case["result"].as_u64().unwrap(), "{case}");
    }
}

struct Heal<'a> {
    case: &'a Value,
    events: Vec<Value>,
}
impl EffectTarget for Heal<'_> {
    fn effect_target(&mut self, _: &Context) -> Result<Option<u32>> {
        Ok(self.case["target"].as_bool().unwrap().then_some(TARGET))
    }
}
impl HealWorld for Heal<'_> {
    fn has_body(&self, _: u32) -> Result<bool> {
        Ok(self.case["body"].as_bool().unwrap())
    }
    fn source_modifier(&mut self, source: Option<u32>) -> Result<Option<u32>> {
        self.events
            .push(json!({"modifier_of_source": source == Some(OWNER)}));
        Ok(self.case["modifier"].as_bool().unwrap().then_some(7))
    }
    fn heal_bonus(&self, _: u32) -> Result<F32> {
        Ok(bits(&self.case["bonus"]))
    }
    fn heal_cap(&self) -> Result<F32> {
        Ok(F32(25.0f32.to_bits()))
    }
    fn heal(&mut self, target: u32, source: Option<u32>, amount: F32) -> Result<()> {
        assert_eq!(target, TARGET);
        self.events.push(json!({"heal": true,
            "record": [u32::from(source == Some(OWNER)), amount.0]}));
        Ok(())
    }
}

#[test]
fn heal_apply_matches_native() {
    for case in doc()["heal"].as_array().unwrap() {
        let mut context = Context::new(0);
        context.source = Some(OWNER);
        let mut world = Heal {
            case,
            events: vec![],
        };
        let got = apply_heal(
            bits(&case["amount"]),
            &context,
            bits(&case["scale"]),
            &mut world,
            Precision::Double53,
        )
        .unwrap();
        assert_eq!(u64::from(got), case["returned"].as_u64().unwrap(), "{case}");
        assert_eq!(Value::Array(world.events), case["events"], "{case}");
    }
}

const ALLEGIANCE: u32 = 0x300;
struct Enchant<'a> {
    case: &'a Value,
    events: Vec<Value>,
}
impl EffectTarget for Enchant<'_> {
    fn effect_target(&mut self, _: &Context) -> Result<Option<u32>> {
        Ok(self.case["target"].as_bool().unwrap().then_some(TARGET))
    }
}
impl EnchantmentWorld for Enchant<'_> {
    fn properties_component(&self, _: u32) -> Result<Option<u32>> {
        Ok(self.case["properties"].as_bool().unwrap().then_some(0x400))
    }
    fn has_property(&mut self, _: u32, _: u32) -> Result<bool> {
        Ok(self.case["existing"].as_str().unwrap().starts_with("same"))
    }
    fn allied(&mut self, owner: u32, allegiance: Option<u32>) -> Result<bool> {
        assert_eq!((owner, allegiance), (TARGET, Some(ALLEGIANCE)));
        self.events.push(json!({"allied_query": "allegiance"}));
        Ok(self.case["allied"].as_bool().unwrap())
    }
    fn enchantment_type(&self, _: u32) -> Result<u32> {
        Ok(0x6100)
    }
    fn immune(&self, _: u32, kind: u32) -> Result<bool> {
        assert_eq!(kind, 0x6100);
        Ok(matches!(
            self.case["immunity"].as_str().unwrap(),
            "match" | "wildcard"
        ))
    }
    fn world_time(&self) -> Result<F32> {
        Ok(F32(123.5f32.to_bits()))
    }
    fn insert_enchantment(
        &mut self,
        _: u32,
        _: u32,
        source: Option<u32>,
        allegiance: Option<u32>,
        time: F32,
    ) -> Result<()> {
        self.events.push(
            json!({"insert": true, "vtable": "0xa2ebe0", "time": time.0, "word8": 1,
            "property": true, "byte10": 0, "source_gtid": source == Some(OWNER),
            "allegiance_gtid": allegiance == Some(ALLEGIANCE)}),
        );
        Ok(())
    }
}

#[test]
fn enchantment_apply_matches_native() {
    let doc = doc();
    let cases = doc["enchant"].as_array().unwrap();
    assert_eq!(cases.len(), 40);
    for case in cases {
        let mut context = Context::new(0);
        context.source = Some(OWNER);
        context.allegiance = Some(ALLEGIANCE);
        let mut world = Enchant {
            case,
            events: vec![],
        };
        let got = apply_enchantment(0x99, &context, &mut world).unwrap();
        assert_eq!(u64::from(got), case["returned"].as_u64().unwrap(), "{case}");
        // The native recorded the queried allegiance pointer; compare kinds.
        let native: Vec<Value> = case["events"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| {
                if e.get("allied_query").is_some() {
                    json!({"allied_query": "allegiance"})
                } else {
                    e.clone()
                }
            })
            .collect();
        assert!(
            case["events"]
                .as_array()
                .unwrap()
                .iter()
                .all(|e| e.get("allied_query").is_none())
                || case["allied_query_allegiance"].as_bool().unwrap()
        );
        assert_eq!(Value::Array(world.events), Value::Array(native), "{case}");
    }
}

#[test]
fn capture_record_matches_native() {
    for case in doc()["capture"].as_array().unwrap() {
        for event in case["events"].as_array().unwrap() {
            assert_eq!(
                event["record"],
                json!(CAPTURE_EFFECT_RECORD.words()),
                "{case}"
            );
            assert_eq!(event["source"], true);
        }
        let reached = !case["events"].as_array().unwrap().is_empty();
        assert_eq!(reached, case["target"].as_bool().unwrap());
        let expected = if reached {
            case["attempt_result"].as_u64()
        } else {
            Some(0)
        };
        assert_eq!(case["returned"].as_u64(), expected);
    }
}

struct Describe<'a> {
    case: &'a Value,
    events: Vec<Value>,
}
impl EffectTarget for Describe<'_> {
    fn effect_target(&mut self, _: &Context) -> Result<Option<u32>> {
        Ok(self.case["target"].as_bool().unwrap().then_some(TARGET))
    }
}
impl DescribeWorld for Describe<'_> {
    fn kingdom_of(&mut self, _: u32) -> Result<Option<u32>> {
        Ok(self.case["kingdom"].as_bool().unwrap().then_some(0x500))
    }
    fn no_kingdom(&mut self) -> Result<()> {
        self.events.push(json!({"log": true}));
        Ok(())
    }
    fn post_economy_description(&mut self, kingdom: u32, allegiance: Option<u32>) -> Result<()> {
        assert_eq!(kingdom, 0x500);
        self.events.push(json!({"describe": true}));
        self.events.push(json!({"localize": true}));
        self.events
            .push(json!({"post": 15, "allegiance": allegiance == Some(ALLEGIANCE), "word": 0}));
        Ok(())
    }
}

#[test]
fn describe_economy_apply_matches_native() {
    for case in doc()["describe"].as_array().unwrap() {
        let mut context = Context::new(0);
        context.allegiance = Some(ALLEGIANCE);
        let mut world = Describe {
            case,
            events: vec![],
        };
        let got = apply_describe_economy(&context, &mut world).unwrap();
        assert_eq!(u64::from(got), case["returned"].as_u64().unwrap(), "{case}");
        assert_eq!(Value::Array(world.events), case["events"], "{case}");
    }
}

struct Lookup(bool);
impl LookupWorld for Lookup {
    fn property_component(&self, _: Object) -> aa_body::Result<Option<Object>> {
        Ok(self.0.then_some(Object(1)))
    }
    fn instances(&self, _: Object) -> aa_body::Result<Vec<Object>> {
        Ok(vec![Object(10)])
    }
    fn instance_facts(&self, _: Object) -> aa_body::Result<(Object, Option<Object>)> {
        Ok((Object(0xA1), None))
    }
}

#[test]
fn property_find_matches_native() {
    let doc = doc();
    for case in doc["find"].as_array().unwrap() {
        let definitions: Vec<u32> = case["definitions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u32)
            .collect();
        let sources: Vec<Option<u32>> = case["sources"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().map(|s| s as u32))
            .collect();
        let indices: Vec<usize> = (0..definitions.len()).collect();
        let got = find_instance(
            &indices,
            Object(case["definition"].as_u64().unwrap() as u32),
            case["source"].as_u64().map(|s| Object(s as u32)),
            case["exclude"].as_u64().map(|e| e as usize),
            |i| Ok((Object(definitions[i]), sources[i].map(Object))),
        )
        .unwrap();
        assert_eq!(got.map(|i| i as u64), case["result"].as_u64(), "{case}");
    }
    for case in doc["find_wrapper"].as_array().unwrap() {
        let world = Lookup(case["properties"].as_bool().unwrap());
        let got = find_actor_property(Object(5), Object(0xA1), None, &world).unwrap();
        assert_eq!(got.is_some(), case["found"].as_bool().unwrap());
    }
}

#[test]
fn enchantment_expiry_matches_native() {
    let doc = doc();
    let cases = doc["expiry"].as_array().unwrap();
    assert_eq!(cases.len(), 250);
    for case in cases {
        let got = enchantment_expired(
            bits(&case["now"]),
            bits(&case["created"]),
            bits(&case["duration"]),
            Precision::Double53,
        )
        .unwrap();
        assert_eq!(u64::from(got), case["expired"].as_u64().unwrap(), "{case}");
    }
}
