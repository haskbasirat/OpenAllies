use aa_mind::{
    Arena, MindId, Result,
    ability_decision::Decision,
    decision_score::{self, Health, LocalWeights, Shared, World},
};
use aa_numeric::{Binary, Precision};
use serde_json::{Value, json};
use std::cmp::Ordering;
struct Host {
    input: Value,
    events: Vec<Value>,
    mutations: Vec<Value>,
}
impl Host {
    fn event(
        &mut self,
        name: &str,
        args: &[u32],
        decision: &mut Decision,
        arena: &mut Arena,
    ) -> Result<()> {
        let mut values = vec![json!(name)];
        values.extend(args.iter().map(|v| json!(v)));
        values.push(json!(decision));
        self.events.push(json!(values));
        if self.mutations.first().is_some_and(|m| m["event"] == name) {
            let change = self.mutations.remove(0);
            if let Some(fields) = change["decision"].as_object() {
                let mut d = json!(decision);
                for (k, v) in fields {
                    d[k] = v.clone();
                }
                *decision = serde_json::from_value(d).unwrap();
            }
            if let Some(actor) = change["mind_actor"].as_u64() {
                arena.mind_mut(MindId(1))?.actor = actor as u32;
            }
            if let Some(locals) = change["locals"].as_object() {
                for (k, fields) in locals {
                    for (name, v) in fields.as_object().unwrap() {
                        self.input["locals"][k][name] = v.clone();
                    }
                }
            }
        }
        Ok(())
    }
}
impl World for Host {
    fn local_weights(&self, ability: u32) -> Result<LocalWeights> {
        Ok(serde_json::from_value(self.input["locals"][ability.to_string()].clone()).unwrap())
    }
    fn shared(&self, definition: u32) -> Result<Shared> {
        Ok(serde_json::from_value(self.input["shared"][definition.to_string()].clone()).unwrap())
    }
    fn actor_bonus(&self, actor: u32, index: usize) -> Result<u32> {
        Ok(self.input["actors"][actor.to_string()]["bonuses"][index]
            .as_u64()
            .unwrap() as u32)
    }
    fn movement_wait_weight(
        &mut self,
        ability: u32,
        actor: u32,
        decision: &mut Decision,
        arena: &mut Arena,
    ) -> Result<bool> {
        self.event("movement", &[ability, actor], decision, arena)?;
        Ok(self.input["movement_wait"].as_bool().unwrap())
    }
    fn target_health(&self, target: u32) -> Result<Health> {
        Ok(
            serde_json::from_value(self.input["actors"][target.to_string()]["health"].clone())
                .unwrap(),
        )
    }
    fn effect_scalar(&self, _definition: u32, kind: u32) -> Result<Option<u32>> {
        Ok(self.input["effects"][kind.to_string()]
            .as_u64()
            .map(|v| v as u32))
    }
    fn health_bonus(&self) -> Result<u32> {
        Ok(self.input["health_bonus"].as_u64().unwrap() as u32)
    }
    fn actor_uses_player_weight(&self, actor: u32) -> Result<bool> {
        Ok(self.input["actors"][actor.to_string()]["player_weight"]
            .as_bool()
            .unwrap())
    }
    fn target_player(&self, target: Option<u32>) -> Result<Option<u32>> {
        Ok(target.and_then(|v| {
            self.input["actors"][v.to_string()]["player"]
                .as_u64()
                .map(|v| v as u32)
        }))
    }
    fn player_weight(&mut self, player: u32, scalar: u32) -> Result<Binary> {
        // No fixture mutation at this leaf: original has no subsequent decision
        // access. Its event captures the latest status after movement admission.
        self.events.push(json!([
            "player",
            player,
            scalar,
            self.input["live_decision"]
        ]));
        Ok(Binary::from_f64_bits(self.input["player_factor_bits"].as_u64().unwrap()).unwrap())
    }
}
fn extended(row: &Value) -> Binary {
    let sig = row["extended_significand"].as_u64().unwrap();
    let se = row["extended_sign_exponent"].as_u64().unwrap() as u16;
    let p = Precision::Extended64;
    let mut value = Binary::from_u32((sig >> 32) as u32)
        .mul(&Binary::from_f64_bits(0x41f0000000000000).unwrap(), p)
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
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase7-mind/native-score.json"
    ))
    .unwrap()
}
#[test]
fn native_utility_local_health_player_and_extended_return() {
    let fixture = fixture();
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 267);
    for case in fixture["cases"].as_array().unwrap() {
        let input = &case["input"];
        let mut host = Host {
            input: input.clone(),
            events: vec![],
            mutations: input["mutations"].as_array().unwrap().clone(),
        };
        host.input["live_decision"] = input["decision"].clone();
        let mut arena = serde_json::from_value(input["arena"].clone()).unwrap();
        let mut decision = serde_json::from_value(input["decision"].clone()).unwrap();
        let precision = serde_json::from_value(input["precision"].clone()).unwrap();
        let value =
            decision_score::score(MindId(1), &mut decision, &mut arena, &mut host, precision)
                .unwrap();
        assert_eq!(
            value.store_f32().unwrap() as u64,
            case["output"]["stored"].as_u64().unwrap(),
            "{}",
            input["name"]
        );
        assert_eq!(
            value.compare(&extended(&case["output"])),
            Ordering::Equal,
            "extended {}",
            input["name"]
        );
        assert_eq!(
            json!(host.events),
            case["output"]["events"],
            "{}",
            input["name"]
        );
        assert_eq!(json!(decision), case["output"]["decision"]);
        assert!(host.mutations.is_empty());
    }
}
#[test]
fn unsupported_numeric_domain_is_explicit() {
    let fixture = fixture();
    let input = &fixture["cases"][0]["input"];
    let mut host = Host {
        input: input.clone(),
        events: vec![],
        mutations: vec![],
    };
    host.input["locals"]["1"]["priority"] = json!(0x7f800000u32);
    let mut arena = serde_json::from_value(input["arena"].clone()).unwrap();
    let mut decision = serde_json::from_value(input["decision"].clone()).unwrap();
    assert!(
        decision_score::score(
            MindId(1),
            &mut decision,
            &mut arena,
            &mut host,
            Precision::Extended64
        )
        .unwrap_err()
        .to_string()
        .contains("nonfinite")
    );
}
