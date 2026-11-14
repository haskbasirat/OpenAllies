//! Replays research/phase8-ai/native-element-speed.json (original 52E610
//! with 4CE6C0 under Unicorn) against aa_mind::element_speed.
use aa_mind::{
    Result,
    element_speed::{ElementSpeedWorld, GroupHolder, set_speed},
};
use serde_json::{Value, json};

const ORGANIZATION: u32 = 40;

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-element-speed.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_i64().unwrap() as u32
}

struct Host {
    c: Value,
    events: Vec<Value>,
    requested: u32,
}
impl ElementSpeedWorld for Host {
    fn set_requested_speed(&mut self, bits: u32) -> Result<()> {
        self.requested = bits;
        Ok(())
    }
    fn actor_has_7c(&mut self) -> Result<bool> {
        Ok(self.c["has_7c"].as_bool().unwrap())
    }
    fn organization(&mut self) -> Result<Option<u32>> {
        Ok(self.c["membership"]
            .as_bool()
            .unwrap_or(false)
            .then_some(ORGANIZATION))
    }
    fn group_gtid(&mut self, holder: GroupHolder) -> Result<u32> {
        Ok(match holder {
            GroupHolder::Actor => u(&self.c["actor_group"]),
            GroupHolder::Organization(o) => {
                assert_eq!(o, ORGANIZATION);
                u(&self.c["org_group"])
            }
        })
    }
    fn resolve(&mut self, gtid: u32) -> Result<Option<u32>> {
        let registered = self.c["registered"].as_array().unwrap();
        Ok(registered.iter().any(|g| u(g) == gtid).then_some(1))
    }
    fn group_stop_speed(&mut self) -> Result<u32> {
        Ok(u(&self.c["svar"]))
    }
    fn stop(&mut self) -> Result<()> {
        self.events.push(json!({"event": "stop"}));
        Ok(())
    }
    fn locomotion_set_speed(&mut self, speed_bits: u32, factor_bits: u32) -> Result<()> {
        self.events.push(
            json!({"event": "locomotion_set_speed", "speed": speed_bits, "factor": factor_bits}),
        );
        Ok(())
    }
}

#[test]
fn native_element_speed_matches_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 600);
    assert_eq!(f["coverage"]["52E610"], json!([42, 42]));
    assert_eq!(f["coverage"]["4CE6C0"], json!([12, 12]));
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let name = &c["name"];
        let mut host = Host {
            c: c.clone(),
            events: vec![],
            requested: 0x5a5a_5a5a,
        };
        set_speed(u(&c["speed"]), &mut host).unwrap();
        assert_eq!(json!(host.events), o["events"], "{name}");
        assert_eq!(json!(host.requested), o["requested"], "{name}");
    }
}
