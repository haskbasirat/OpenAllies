//! 0x576790 against research/phase8-combat/native-performer-interrupt-fixtures.json.
use aa_movement::{
    Result,
    mover_activation::{ActivationWorld, activate},
};
use serde_json::{Value, json};

const OWNER: u32 = 0x5000;

struct Adapter(Vec<Value>);
impl ActivationWorld for Adapter {
    fn cancel_events(&mut self) -> Result<()> {
        self.0.push(json!({"cancel": 8}));
        Ok(())
    }
    fn activation_triggers(&mut self, owner: u32) -> Result<()> {
        assert_eq!(owner, OWNER);
        self.0.push(json!({"triggers": "owner", "arg": 0}));
        Ok(())
    }
}

#[test]
fn mover_activation_matches_native() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-performer-interrupt-fixtures.json"
    ))
    .unwrap();
    let cases: Vec<&Value> = doc["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["function"] == "mover_activation")
        .collect();
    assert_eq!(cases.len(), 4);
    for case in cases {
        let flag = case["flag"].as_u64().unwrap() & 0xFF != 0;
        let mut world = Adapter(vec![]);
        activate(OWNER, flag, &mut world).unwrap();
        assert_eq!(Value::Array(world.0), case["events"], "{case}");
    }
}
