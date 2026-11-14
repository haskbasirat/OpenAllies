//! Replays research/phase8-combat/native-ability-choice.json (original
//! 57E920 with its 4C8300 / 517F60 / 516AD0 helpers under Unicorn) against
//! aa_combat::ability_choice: the returned code and every seam call in order.
use aa_combat::{
    Result,
    ability_choice::{AbilityChoiceWorld, Context, choose},
};
use aa_numeric::{Binary, Precision};
use serde_json::{Value, json};

const ACTOR: u32 = 0xa0;
const TARGET: u32 = 0xb0;
const KINGDOM: u32 = 0x4b1d;
const MODIFIER: u32 = 0x3d0f;

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-combat/native-ability-choice.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_i64().unwrap() as u32
}
fn pair(v: &Value) -> [u32; 2] {
    [u(&v[0]), u(&v[1])]
}
fn precision(name: &str) -> Precision {
    match name {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        "Extended64" => Precision::Extended64,
        other => panic!("precision {other}"),
    }
}

struct Host<'a> {
    c: &'a Value,
    events: Vec<Value>,
}
impl Host<'_> {
    fn ability(&self, index: i32) -> &Value {
        &self.c["abilities"][index as usize]
    }
}
impl AbilityChoiceWorld for Host<'_> {
    fn actor(&mut self) -> Result<u32> {
        Ok(ACTOR)
    }
    fn kingdom(&mut self, actor: u32) -> Result<u32> {
        assert_eq!(actor, ACTOR);
        self.events.push(json!(["kingdom"]));
        Ok(KINGDOM)
    }
    fn target_accepts(&mut self, target: u32, kingdom: u32) -> Result<bool> {
        assert_eq!(target, TARGET);
        self.events.push(json!(["accepts", kingdom]));
        Ok(self.c["target"]["accepts"].as_bool().unwrap())
    }
    fn target_bombardable(&mut self, target: u32, kingdom: u32, flag: u32) -> Result<bool> {
        assert_eq!(target, TARGET);
        self.events.push(json!(["bombardable", kingdom, flag]));
        Ok(self.c["target"]["bombardable"].as_bool().unwrap())
    }
    fn ability_count(&mut self) -> Result<i32> {
        Ok(self.c["abilities"].as_array().unwrap().len() as i32)
    }
    fn ability_enabled(&mut self, index: i32) -> Result<bool> {
        Ok(self.ability(index)["enabled"].as_bool().unwrap())
    }
    fn ability_flags54(&mut self, index: i32) -> Result<u32> {
        Ok(u(&self.ability(index)["flags54"]))
    }
    fn ability_applies(&mut self, index: i32, context: &Context, flag: u32) -> Result<bool> {
        self.events.push(json!([
            "applies",
            index,
            context.kind,
            u32::from(context.actor == ACTOR),
            u32::from(context.target == TARGET),
            flag
        ]));
        Ok(self.ability(index)["applies"].as_bool().unwrap())
    }
    fn target_position(&mut self, target: u32) -> Result<[u32; 2]> {
        assert_eq!(target, TARGET);
        Ok(pair(&self.c["target"]["position"]))
    }
    fn ability_reaches(&mut self, index: i32, actor: u32, point: [u32; 2]) -> Result<bool> {
        assert_eq!(actor, ACTOR);
        self.events.push(json!(["reaches", index, point[0], point[1]]));
        Ok(self.ability(index)["reaches"].as_bool().unwrap())
    }
    fn ability_ready(&mut self, index: i32, actor: u32, point: [u32; 2]) -> Result<bool> {
        assert_eq!(actor, ACTOR);
        self.events.push(json!(["ready", index, point[0], point[1]]));
        Ok(self.ability(index)["ready"].as_bool().unwrap())
    }
    fn actor_position(&mut self, actor: u32) -> Result<[u32; 2]> {
        assert_eq!(actor, ACTOR);
        Ok(pair(&self.c["actor_position"]))
    }
    fn modifier(&mut self, actor: u32) -> Result<u32> {
        assert_eq!(actor, ACTOR);
        self.events.push(json!(["modifier"]));
        Ok(MODIFIER)
    }
    fn ability_range(&mut self, index: i32, modifier: u32) -> Result<Binary> {
        assert_eq!(modifier, MODIFIER);
        self.events.push(json!(["range", index]));
        let range = self.ability(index)["range"].as_f64().unwrap();
        Ok(Binary::from_f64_bits(range.to_bits())?)
    }
}

#[test]
fn ability_choice_matches_native() {
    let d = fixture();
    let cases = d["cases"].as_array().unwrap();
    assert!(cases.len() >= 2700);
    let mut codes = [0usize; 7];
    for case in cases {
        let c = &case["input"];
        let want = &case["output"];
        let name = c["name"].as_str().unwrap();
        let mut host = Host { c, events: Vec::new() };
        let target = (!c["target"].is_null()).then_some(TARGET);
        let got = choose(
            target,
            u(&c["kind"]),
            c["strict"].as_bool().unwrap(),
            &mut host,
            precision(c["precision"].as_str().unwrap()),
        )
        .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(got, u(&want["result"]), "{name} code");
        assert_eq!(Value::Array(host.events), want["events"], "{name} events");
        codes[got as usize] += 1;
    }
    assert!(codes.iter().all(|&n| n > 100), "{codes:?}");
}
