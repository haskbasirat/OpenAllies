//! Original575FB0/575C00/575BB0 fixtures; no gameplay answers are guessed.
use aa_actors::{
    ActorId,
    ability_flags::{ActorFields, Error, Result},
    mobile_deployment_actions::{self as mobile, BodyHealth, World},
};
use aa_numeric::Precision;
use serde_json::Value;
use std::cell::RefCell;

struct Fixture {
    input: Value,
    actions: u64,
    reads: RefCell<Vec<&'static str>>,
    missing: Option<&'static str>,
    writes: usize,
}
impl Fixture {
    fn get(&self, key: &'static str) -> Result<&Value> {
        self.reads.borrow_mut().push(key);
        if self.missing == Some(key) {
            return Err(Error::Missing(key));
        }
        Ok(&self.input[key])
    }
}
impl World for Fixture {
    fn fields(&self, _: ActorId) -> Result<ActorFields> {
        Ok(ActorFields {
            thing_flags: 0,
            state_bits: self.get("state")?.as_u64().unwrap(),
            action_bits: self.actions,
        })
    }
    fn set_action_bits(&mut self, _: ActorId, bits: u64) -> Result<()> {
        self.actions = bits;
        self.writes += 1;
        Ok(())
    }
    fn has_pack_reference(&self, _: ActorId) -> Result<bool> {
        Ok(self.get("deployment")?.as_bool().unwrap())
    }
    fn has_unpack_reference(&self, _: ActorId) -> Result<bool> {
        Ok(self.get("unpack")?.as_bool().unwrap())
    }
    fn pack_health_min_bits(&self, _: ActorId) -> Result<u32> {
        Ok(self.get("threshold")?.as_u64().unwrap() as u32)
    }
    fn properties_flags(&self, _: ActorId) -> Result<Option<u32>> {
        Ok(self.get("properties")?.as_u64().map(|v| v as u32))
    }
    fn body_health(&self, _: ActorId) -> Result<Option<BodyHealth>> {
        Ok(self.get("body")?.as_array().map(|v| BodyHealth {
            health_bits: v[0].as_u64().unwrap() as u32,
            maximum_bits: v[1].as_u64().unwrap() as u32,
        }))
    }
    fn has_mind(&self, _: ActorId) -> Result<bool> {
        Ok(self.get("mind")?.as_bool().unwrap())
    }
}
fn fixture(input: Value) -> Fixture {
    Fixture {
        actions: input["actions"].as_u64().unwrap(),
        input,
        reads: RefCell::new(Vec::new()),
        missing: None,
        writes: 0,
    }
}
fn precision(input: &Value) -> Precision {
    match input["precision"].as_str().unwrap() {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        "Extended64" => Precision::Extended64,
        v => panic!("unknown precision {v}"),
    }
}

#[test]
fn original_mobile_deployment_actions_match_all_branches() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-mobile-actions/native-mobile-actions.json"
    ))
    .unwrap();
    assert_eq!(
        doc["original_sha256"],
        "5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26"
    );
    assert_eq!(doc["cases"].as_array().unwrap().len(), 552);
    for case in doc["cases"].as_array().unwrap() {
        let input = &case["input"];
        let mut world = fixture(input.clone());
        let actor = ActorId::from_raw(0x10001);
        let pc = precision(input);
        let result = match input["entry"].as_str().unwrap() {
            "contribute" => {
                mobile::contribute(&mut world, actor, pc).unwrap();
                assert_eq!(world.writes, 1);
                None
            }
            "can_pack" => Some(
                mobile::can_pack(&world, actor, input["check"].as_bool().unwrap(), pc).unwrap(),
            ),
            "can_unpack" => {
                Some(mobile::can_unpack(&world, actor, input["check"].as_bool().unwrap()).unwrap())
            }
            "state_permits" => Some(mobile::state_permits(&world, actor).unwrap()),
            other => panic!("unknown entry {other}"),
        };
        assert_eq!(result, case["output"]["value"].as_bool(), "{case}");
        assert_eq!(
            world.actions,
            case["output"]["actions"].as_u64().unwrap(),
            "{case}"
        );
        assert_eq!(
            (world.actions ^ input["actions"].as_u64().unwrap()) & !(1 << 34),
            0
        );
    }
}

#[test]
fn required_facts_fail_before_action_write_and_unreached_reads_stay_unread() {
    let input = serde_json::json!({"state":0,"actions":0xfedcba9876543210u64,
        "deployment":true,"unpack":true,"threshold":0x3f000000u32,
        "properties":null,"body":[0x42960000u32,0x42c80000u32],"mind":false});
    let actor = ActorId::from_raw(1);
    for missing in [
        "state",
        "deployment",
        "threshold",
        "properties",
        "body",
        "mind",
    ] {
        let mut world = fixture(input.clone());
        world.missing = Some(missing);
        assert!(
            mobile::contribute(&mut world, actor, Precision::Double53).is_err(),
            "{missing}"
        );
        assert_eq!(world.writes, 0, "{missing}");
        assert_eq!(
            world.actions,
            input["actions"].as_u64().unwrap(),
            "{missing}"
        );
    }
    let mut world = fixture(input.clone());
    world.input["deployment"] = false.into();
    world.missing = Some("threshold");
    mobile::contribute(&mut world, actor, Precision::Double53).unwrap();
    assert_eq!(
        world.reads.borrow().as_slice(),
        &["state", "deployment", "state"]
    );
    let mut world = fixture(input);
    world.missing = Some("body");
    assert!(mobile::can_pack(&world, actor, false, Precision::Double53).unwrap());
    assert_eq!(
        world.reads.borrow().as_slice(),
        &["deployment", "threshold", "properties"]
    );
}
