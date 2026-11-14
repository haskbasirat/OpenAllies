//! Replays research/phase8-combat/native-kill-rewards-fixtures.json: original
//! 45D640 (ST0 as the 80-bit register image), 4D3E20 (with the real 4F02B0,
//! 85D840 and 471A30) and 4D10F0 under Unicorn, all precisions.
use aa_body::{F32, Result, kill_rewards::*};
use aa_numeric::Precision;
use serde_json::{Value, json};

fn precision(v: &Value) -> Precision {
    match v.as_str().unwrap() {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        "Extended64" => Precision::Extended64,
        other => panic!("{other}"),
    }
}

/// `85D840`: `wide = state * 0x41C64E6D + 0x3039`, state = low 32 bits,
/// draw = low 32 bits of `wide >> 16`.
fn lcg(state: &mut u32) -> u32 {
    let wide = u64::from(*state) * 0x41c6_4e6d + 0x3039;
    *state = wide as u32;
    (wide >> 16) as u32
}

fn word(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}

fn f64_bits(v: F32) -> u64 {
    f64::from(f32::from_bits(v.0)).to_bits()
}

struct Lair<'a> {
    case: &'a Value,
    state: u32,
    events: Vec<Value>,
}

impl LairWorld for Lair<'_> {
    fn kingdom_object(&mut self, _: u32) -> Result<Option<u32>> {
        Ok(self.case["kingdom_object"].as_bool().unwrap().then_some(7))
    }
    fn kingdom(&mut self, _: u32) -> Result<u32> {
        self.events.push(json!(["kingdom"]));
        Ok(word(&self.case["kingdom"]))
    }
    fn can_see(&mut self, _: u32, kingdom: u32, flag: u32) -> Result<bool> {
        self.events.push(json!(["can_see", kingdom, flag]));
        Ok(word(&self.case["can_see"]) & 0xFF != 0)
    }
    fn ignored(&mut self, _: u32) -> Result<()> {
        self.events.push(json!(["ignored", self.case["lair_id"]]));
        Ok(())
    }
    fn reward_bounds(&mut self, _: u32) -> Result<[F32; 2]> {
        Ok([F32(word(&self.case["low"])), F32(word(&self.case["high"]))])
    }
    fn draw_random(&mut self) -> Result<u32> {
        Ok(lcg(&mut self.state))
    }
    fn resource_count(&mut self) -> Result<i32> {
        Ok(self.case["resource_count"].as_i64().unwrap() as i32)
    }
    fn resource_word18(&mut self, index: i32) -> Result<u32> {
        Ok(word(&self.case["resources"][index as usize]["word18"]))
    }
    fn add_resource(&mut self, object: u32, index: i32, amount: F32) -> Result<()> {
        assert_eq!(object, 7);
        self.events.push(json!(["grant", index, amount.0]));
        Ok(())
    }
    fn name(&mut self, _: u32) -> Result<u32> {
        self.events.push(json!(["name"]));
        Ok(0x5157)
    }
    fn resource_label(&mut self, index: i32) -> Result<u16> {
        Ok(self.case["resources"][index as usize]["label"].as_u64().unwrap() as u16)
    }
    fn post_message(&mut self, kingdom: u32, lair: u32, label: u16, amount: F32, name: u32) -> Result<()> {
        assert_eq!((lair, name), (3, 0x5157));
        self.events.push(json!(["post", 10, kingdom, label, f64_bits(amount)]));
        Ok(())
    }
}

struct Organization<'a> {
    case: &'a Value,
    events: Vec<Value>,
}

const KILLER: u32 = 1;
const ORGANIZATION: u32 = 2;

impl OrganizationRewardWorld for Organization<'_> {
    fn object154(&mut self, actor: u32) -> Result<Option<u32>> {
        Ok(match actor {
            KILLER => self.case["own"].as_bool().unwrap().then_some(KILLER),
            ORGANIZATION => self.case["organization154"].as_bool().unwrap().then_some(ORGANIZATION),
            _ => unreachable!(),
        })
    }
    fn membership_organization(&mut self, _: u32) -> Result<Option<u32>> {
        Ok((self.case["membership"] == "org").then_some(ORGANIZATION))
    }
    fn organization_reward(&mut self, object: u32, victim: u32) -> Result<()> {
        let name = if object == KILLER { "own" } else { "organization" };
        self.events.push(json!(["reward", name, victim == 9]));
        Ok(())
    }
}

#[test]
fn original_kill_rewards_match_every_fixture() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-kill-rewards-fixtures.json"
    ))
    .unwrap();
    assert_eq!(doc["passed"], true);
    let draws = doc["cases"].as_array().unwrap();
    assert_eq!(draws.len(), 480);
    for (i, case) in draws.iter().enumerate() {
        let mut state = word(&case["state"]);
        let draw = match case["hooked_draw"].as_u64() {
            Some(d) => d as u32,
            None => lcg(&mut state),
        };
        assert_eq!(state, word(&case["state_after"]), "draw {i} state");
        let value = reward_draw(F32(word(&case["low"])), F32(word(&case["high"])), draw, precision(&case["precision"]))
            .unwrap();
        let expected = (u128::from(case["sign_exponent"].as_u64().unwrap() as u16) << 64)
            | u128::from(u64::from_str_radix(case["mantissa"].as_str().unwrap(), 16).unwrap());
        assert_eq!(value.to_x87_extended_bits(), Some(expected), "draw {i} {case}");
    }
    let lairs = doc["lairs"].as_array().unwrap();
    assert_eq!(lairs.len(), 180);
    for (i, case) in lairs.iter().enumerate() {
        let mut host = Lair { case, state: word(&case["state"]), events: Vec::new() };
        lair_reward(&mut host, 1, 3, precision(&case["precision"])).unwrap();
        assert_eq!(Value::Array(host.events), case["events"], "lair {i}");
        assert_eq!(host.state, word(&case["state_after"]), "lair {i} state");
    }
    let organizations = doc["organizations"].as_array().unwrap();
    assert_eq!(organizations.len(), 40);
    for (i, case) in organizations.iter().enumerate() {
        let mut host = Organization { case, events: Vec::new() };
        organization_reward(&mut host, KILLER, 9).unwrap();
        assert_eq!(Value::Array(host.events), case["events"], "organization {i}");
    }
}
