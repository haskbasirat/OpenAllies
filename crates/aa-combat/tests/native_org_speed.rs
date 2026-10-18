//! 0x57A640 against research/phase8-combat/native-org-speed-fixtures.json.
use aa_body::{F32, damage::DamageRecord};
use aa_combat::{
    Result,
    morale::Organization,
    organization::{SpeedWorld, recompute_speed},
};
use aa_numeric::Precision;
use serde_json::Value;

struct Adapter {
    mover: bool,
    calls: Vec<[u32; 2]>,
}
impl SpeedWorld for Adapter {
    fn speed_changed(&mut self, _: &mut Organization, old: F32, new: F32) -> Result<()> {
        if self.mover {
            self.calls.push([old.0, new.0]);
        }
        Ok(())
    }
}

#[test]
fn speed_minimum_matches_native() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-org-speed-fixtures.json"
    ))
    .unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 450);
    let mut notified = 0;
    for case in cases {
        let p: Precision = serde_json::from_value(case["precision"].clone()).unwrap();
        let values: Vec<Option<F32>> = case["slots"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|s| s.as_str() != Some("empty"))
            .map(|s| s.as_u64().map(|b| F32(b as u32)))
            .collect();
        let mut o = Organization {
            owner: 1,
            morale: F32::ZERO,
            maximum: F32::ZERO,
            breakpoint: F32::ZERO,
            living: 0,
            engaged_latch: false,
            engage_latch: false,
            attack_efficiency: F32::ZERO,
            attack_efficiency_desired: F32::ZERO,
            byte8d: false,
            last_damage: DamageRecord {
                source_id: 0,
                allegiance_id: 0,
                damage_type: 0,
                amount: F32::ZERO,
                time: F32::ZERO,
                event_kind: 0,
                reason: 0,
            },
            leader: None,
            primary: None,
            regroup_started: F32::ZERO,
            entrench_base: F32::ZERO,
            flare_pending: false,
            flare_word: 0,
            marker_pending: false,
            watched_target: 0,
            current_combat_value: F32::ZERO,
            maximum_combat_value: F32::ZERO,
            field_d8: F32(case["d8"].as_u64().unwrap() as u32),
        };
        let mut world = Adapter {
            mover: case["mover"].as_bool().unwrap(),
            calls: vec![],
        };
        recompute_speed(&mut o, &values, &mut world, p).unwrap();
        assert_eq!(
            u64::from(o.field_d8.0),
            case["d8_after"].as_u64().unwrap(),
            "{case}"
        );
        let native: Vec<[u32; 2]> = case["calls"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| [c[0].as_u64().unwrap() as u32, c[1].as_u64().unwrap() as u32])
            .collect();
        assert_eq!(world.calls, native, "{case}");
        notified += native.len();
    }
    assert!(notified > 100, "{notified}");
}
