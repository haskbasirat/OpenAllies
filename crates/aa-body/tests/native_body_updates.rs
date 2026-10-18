use aa_body::{
    Body, Environment, Event, F32, Host, properties::ModifierState, updates::MaximumHealthPair,
};
use aa_numeric::{Binary, Precision};
use serde_json::{Value, json};
#[derive(Default)]
struct Events(Vec<Value>);
impl Host for Events {
    fn event(
        &mut self,
        event: Event,
        body: &mut Body,
        _: &mut Environment,
    ) -> std::result::Result<(), String> {
        self.0.push(json!({"event":event,"body":body}));
        Ok(())
    }
}
#[test]
fn native_extended_body_refresh_and_maximum_health_step() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-properties/native-body-updates.json"
    ))
    .unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 87);
    for row in cases {
        let c = &row["input"];
        let mut body: Body = serde_json::from_value(c["body"].clone()).unwrap();
        let mut env: Environment = serde_json::from_value(c["environment"].clone()).unwrap();
        let p: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let mut host = Events::default();
        let f = |key: &str| F32(c[key].as_u64().unwrap() as u32);
        let mut modifier = ModifierState::new(1);
        modifier.scalars[0x50 / 4 - 1] = f("factor");
        modifier.scalars[0x10 / 4 - 1] = f("delta");
        let context = c["modifier"].as_bool().unwrap().then_some(&modifier);
        let mut pair = None;
        match c["method"].as_str().unwrap() {
            "maximum" => {
                let ratio = body.capture_maximum_ratio(p).unwrap();
                let maximum = context
                    .map_or_else(
                        || Binary::from_f32_bits(f("base").0).map_err(aa_body::Error::from),
                        |m| m.health_maximum(f("base"), p),
                    )
                    .unwrap();
                body.apply_extended_maximum(ratio, maximum, c["preserve"].as_bool().unwrap(), p)
                    .unwrap();
            }
            "defense" => body
                .refresh_defense(f("base"), context, f("minimum"), p)
                .unwrap(),
            "pair" => {
                pair = Some(
                    body.set_maximum_health_step(
                        MaximumHealthPair {
                            health: f("health"),
                            maximum: f("base"),
                        },
                        &mut env,
                        &mut host,
                        p,
                    )
                    .unwrap(),
                );
            }
            _ => unreachable!(),
        }
        let mut output = json!({"body":body,"environment":env,"events":host.0});
        if let Some(pair) = pair {
            output["pair"] = serde_json::to_value(pair).unwrap();
        }
        assert_eq!(output, row["output"], "{}", c["name"]);
    }
}
