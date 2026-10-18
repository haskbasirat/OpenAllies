//! Source-owner reads, not a claim of a complete native Session constructor.
use aa_netcode::lobby::Session;
use aa_runtime::load_world_globals::SessionPhase;
use serde_json::Value;

#[test]
fn actual_session_fields_replay_native_phase_and_application_reads() {
    let proof: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-load-world-globals/native-instructions.json"
    ))
    .unwrap();
    let rows = proof["getter_cases"].as_array().unwrap();
    assert_eq!(rows.len(), 936);
    // Only the two reached native fields are seeded by the oracle. All other
    // fixture fields are unused; their defaults are not native startup proof.
    let mut owner = Session::default();
    for row in rows {
        owner.field_f8 = row["phase"].as_u64().unwrap() as u32;
        owner.state = row["application_state"].as_u64().unwrap() as u32;
        let live: &dyn SessionPhase = &owner;
        assert_eq!(
            live.current_phase().unwrap() as u64,
            row["native_phase_result"]
        );
        assert_eq!(live.application_state().unwrap(), owner.state);
        assert_eq!(
            live.application_state().unwrap() == 3,
            row["native_application_equals3"].as_bool().unwrap()
        );
    }
}
