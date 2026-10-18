//! Replays research/phase8-combat/native-roe-fixtures.json: original
//! 570F70/570A70/570E30/570EA0 executed under Unicorn.
use aa_body::{F32, damage::DamageRecord};
use aa_combat::{
    Error, Result,
    roe::{self, World},
};
use aa_numeric::{Binary, Precision};
use serde_json::Value;
use std::cmp::Ordering;

fn record(words: &Value) -> DamageRecord {
    let w: Vec<u32> = words
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u32)
        .collect();
    DamageRecord {
        source_id: w[0],
        allegiance_id: w[1],
        damage_type: w[2],
        amount: F32(w[3]),
        time: F32(w[4]),
        event_kind: w[5],
        reason: w[6],
    }
}

fn extended(row: &Value) -> Binary {
    let sig = row["mantissa"].as_u64().unwrap();
    let se = row["sign_exponent"].as_u64().unwrap() as u16;
    let p = Precision::Extended64;
    let mut value = Binary::from_u32((sig >> 32) as u32)
        .mul(&Binary::from_f64_bits(0x41f0_0000_0000_0000).unwrap(), p)
        .unwrap()
        .add(&Binary::from_u32(sig as u32), p)
        .unwrap();
    if sig != 0 {
        let exponent = i32::from(se & 0x7fff) - 16383 - 63;
        value = value
            .mul(
                &Binary::from_f64_bits(((exponent + 1023) as u64) << 52).unwrap(),
                p,
            )
            .unwrap();
    }
    if se & 0x8000 != 0 {
        value = value.negated();
    }
    value
}

struct Host {
    organization_actor: bool,
    primary: &'static str,
}
const MEMBER: u32 = 1;
const OTHER: u32 = 2;
const ORGANIZATION: u32 = 3;
impl World for Host {
    fn membership_organization(&self, member: u32) -> Result<Option<u32>> {
        assert_eq!(member, MEMBER);
        Ok(self.organization_actor.then_some(ORGANIZATION))
    }
    fn primary_element(&self, organization: u32) -> Result<Option<u32>> {
        assert_eq!(organization, ORGANIZATION);
        Ok(match self.primary {
            "owner" => Some(MEMBER),
            "other" => Some(OTHER),
            "none" => None,
            _ => return Err(Error::Invalid("primary")),
        })
    }
}

#[test]
fn original_roe_history_matches_every_fixture() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-roe-fixtures.json"
    ))
    .unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert!(cases.len() >= 1000);
    for row in cases {
        let i = &row["input"];
        let p: Precision = serde_json::from_value(i["precision"].clone()).unwrap();
        let mut history: Vec<DamageRecord> = i["records"]
            .as_array()
            .unwrap()
            .iter()
            .map(record)
            .collect();
        let now = F32(i["now"].as_u64().unwrap() as u32);
        let length = F32(i["history_length"].as_u64().unwrap() as u32);
        let name = format!("{} {}", i["name"], i["precision"]);
        match i["operation"].as_str().unwrap() {
            "prune" => roe::prune(&mut history, now, length, p).unwrap(),
            "rate" => {
                let value = roe::rate(&mut history, now, length, p).unwrap();
                let expected = extended(&row["output"]["result"]);
                assert_eq!(value.compare(&expected), Ordering::Equal, "{name}");
            }
            "record" => {
                let host = Host {
                    organization_actor: i["organization_actor"] == true,
                    primary: match i["primary"].as_str().unwrap() {
                        "owner" => "owner",
                        "other" => "other",
                        _ => "none",
                    },
                };
                roe::record(
                    MEMBER,
                    &mut history,
                    record(&i["record"]),
                    now,
                    length,
                    &host,
                    p,
                )
                .unwrap();
            }
            other => panic!("{other}"),
        }
        let actual: Vec<Value> = history
            .iter()
            .map(|r| serde_json::json!(r.words()))
            .collect();
        assert_eq!(Value::Array(actual), row["output"]["records"], "{name}");
    }
}
