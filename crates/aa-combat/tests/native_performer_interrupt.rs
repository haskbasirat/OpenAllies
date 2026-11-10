//! 0x57DCB0 against research/phase8-combat/native-performer-interrupt-fixtures.json.
use aa_body::F32;
use aa_combat::{
    Result,
    constructors::OwnedPerformer,
    performer::Performer,
    performer_interrupt::{InterruptWorld, left_current},
};
use serde_json::{Value, json};

const OWNER: u32 = 0x5000;

struct Adapter(Vec<Value>);
impl InterruptWorld for Adapter {
    fn cancel_events(&mut self) -> Result<()> {
        self.0.push(json!({"cancel": 8}));
        Ok(())
    }
    fn mind_interrupted(&mut self, owner: u32) -> Result<()> {
        assert_eq!(owner, OWNER);
        self.0.push(json!({"mind_interrupted": 1}));
        Ok(())
    }
    fn activate_updates(&mut self) -> Result<()> {
        self.0.push(json!({"activate": 8, "arg": 1}));
        Ok(())
    }
}

#[test]
fn performer_left_matches_native() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-performer-interrupt-fixtures.json"
    ))
    .unwrap();
    let cases: Vec<&Value> = doc["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["function"] == "performer_left")
        .collect();
    assert_eq!(cases.len(), 21);
    for case in cases {
        let performer = OwnedPerformer {
            state: Performer {
                owner: OWNER,
                stage: case["stage"].as_i64().unwrap() as i32,
                abilities: vec![],
                mana: F32::ZERO,
                target_id: 0,
                target_position: [F32::ZERO; 2],
                active_ability: None,
                active_index: -1,
                started_at: F32::ZERO,
                animation_length: F32::ZERO,
                event_kind: 0,
                animation: None,
                turret_index: -1,
            },
            maximum_mana: F32::ZERO,
            turrets: vec![],
            turret_update_pending: case["latch"].as_u64().unwrap() != 0,
            word60: 0,
        };
        let mut world = Adapter(vec![]);
        left_current(&performer, &mut world).unwrap();
        assert_eq!(Value::Array(world.0), case["events"], "{case}");
    }
}
