use super::*;
use crate::application_mode::ModeServices;
use serde_json::Value;

///Explicit negative manager-predicate test inputs for the already-proved
///651840 setter. Query fixtures edit only+C; neither query reads changed+8.
struct ExplicitNoManagers;
impl ModeServices for ExplicitNoManagers {
    fn manager_a_active(&mut self) -> Result<bool> {
        Ok(false)
    }
    fn post_manager_a_reset(&mut self) -> Result<()> {
        panic!("unreached explicit absent manager")
    }
    fn manager_b_active(&mut self) -> Result<bool> {
        Ok(false)
    }
    fn post_manager_b_reset(&mut self) -> Result<()> {
        panic!("unreached explicit absent manager")
    }
}

#[test]
fn fresh_owned_campaign_queries_match_native_empty_prefixes() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-campaign-manager/native-campaign-manager.json"
    ))
    .unwrap();
    assert_eq!(fixture["pe_loader_zero_words"].as_array().unwrap().len(), 3);
    for row in fixture["cases"].as_array().unwrap() {
        let input = &row["input"];
        let expected = &row["output"];
        let mut application = GameAppState::from_native_constructor();
        assert_eq!(application.mode(), 1);
        assert_eq!(application.campaigns().active_count(), 0);
        application
            .set_mode(
                input["mode"].as_u64().unwrap() as u32,
                &mut ExplicitNoManagers,
            )
            .unwrap();
        let owner = application.campaigns();
        let result = if input["kind"] == "capture_pair" {
            owner.current_4f1310(&application).and_then(|current| {
                assert!(current.is_none());
                let value = owner.lookup_411790(&application, current.is_some())?;
                for _ in 0..2 {
                    assert!(owner.lookup_411790(&application, false)?.is_none());
                }
                Ok(value)
            })
        } else {
            owner.lookup_411790(&application, true)
        };
        if expected["stopped"].is_null() {
            assert!(result.unwrap().is_none());
        } else {
            assert!(result.unwrap_err().contains("6F7810/470970"));
        }
        assert_eq!(owner.active_count(), 0);
        assert_eq!(expected["table_words"], serde_json::json!([0, 0, 0]));
    }
}

#[test]
fn campaign_origin_survives_moves_and_refuses_other_or_dead_application() {
    let application = GameAppState::from_native_constructor();
    let application = Box::new(application);
    let application = *application;
    assert!(
        application
            .campaigns()
            .current_4f1310(&application)
            .unwrap()
            .is_none()
    );
    let other = GameAppState::from_native_constructor();
    assert!(
        application
            .campaigns()
            .lookup_411790(&other, false)
            .unwrap_err()
            .contains("another or destroyed")
    );
    assert_eq!(application.campaigns().active_count(), 0);
    let detached = State::from_application_constructor(Weak::new());
    assert!(detached.current_4f1310(&application).is_err());
    assert!(
        application
            .campaigns()
            .lookup_411790(&application, true)
            .is_err()
    );
    assert!(
        application
            .campaigns()
            .lookup_411790(&application, false)
            .unwrap()
            .is_none()
    );
}
