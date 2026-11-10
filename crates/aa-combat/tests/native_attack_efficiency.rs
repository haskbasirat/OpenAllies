//! 0x57DBF0 against research/phase8-combat/native-attack-efficiency-fixtures.json.
use aa_body::F32;
use aa_combat::{
    Result,
    attack_efficiency::{AttackEfficiencyWorld, attack_efficiency},
};
use aa_numeric::Fast;
use serde_json::Value;

const ACTOR: u32 = 0x10;
const ORGANIZATION: u32 = 0x20;

struct Adapter<'a>(&'a Value);
fn bits(v: &Value) -> F32 {
    F32(v.as_u64().unwrap() as u32)
}
impl AttackEfficiencyWorld for Adapter<'_> {
    fn organization_actor(&self, actor: u32) -> Result<Option<u32>> {
        assert_eq!(actor, ACTOR);
        Ok((self.0["membership"].as_bool().unwrap() && self.0["organization"].as_bool().unwrap())
            .then_some(ORGANIZATION))
    }
    fn organization_attack_efficiency(&self, organization: u32) -> Result<F32> {
        assert_eq!(organization, ORGANIZATION);
        Ok(bits(&self.0["efficiency"]))
    }
    fn offense_modifiers(&self, actor: u32) -> Result<Option<[F32; 2]>> {
        assert_eq!(actor, ACTOR);
        Ok(self.0["modifiers"]
            .as_bool()
            .unwrap()
            .then(|| [bits(&self.0["primary"]), bits(&self.0["secondary"])]))
    }
}

#[test]
fn attack_efficiency_matches_native() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-attack-efficiency-fixtures.json"
    ))
    .unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 600);
    for case in cases {
        let value = attack_efficiency(ACTOR, &Adapter(&case["case"])).unwrap();
        let native = u128::from(case["result"][0].as_u64().unwrap())
            | (u128::from(case["result"][1].as_u64().unwrap()) << 64);
        assert_eq!(
            Fast::from_f32_bits(value.0).unwrap().to_x87_extended_bits(),
            native,
            "{case}"
        );
    }
}
