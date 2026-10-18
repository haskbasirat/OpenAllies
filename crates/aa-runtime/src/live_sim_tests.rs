use super::*;
fn pc(s: &str) -> Precision {
    match s {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        "Extended64" => Precision::Extended64,
        _ => panic!("PC"),
    }
}
fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../research/development10/live-sim/native-live-sim.json"
    ))
    .unwrap()
}
#[test]
fn real_constructor_keeps_native_uninitialized_time_words_and_reset_matches_all_pcs() {
    let fixture = fixture();
    assert_eq!(fixture["constructors"].as_array().unwrap().len(), 72);
    for row in fixture["constructors"].as_array().unwrap() {
        let mut state = Sim::from_native_constructor_fields();
        let n = &row["native"];
        state
            .complete_rate(
                row["input"]["rate"].as_u64().unwrap() as u32,
                pc(row["precision"].as_str().unwrap()),
            )
            .unwrap();
        assert_eq!(state.paused_byte(), n["paused"].as_u64().unwrap() as u8);
        assert_eq!(state.frozen_byte(), n["frozen"].as_u64().unwrap() as u8);
        assert!(state.world_is_null());
        assert_eq!(state.constructor_field_34(), 0);
        assert_eq!(
            state.game_per_admin_tick().unwrap().0,
            n["rate"].as_u64().unwrap() as u32
        );
        assert_eq!(
            state.time_words(),
            TimeWords {
                begin_game: None,
                begin_admin: None,
                end_game: None,
                end_admin: None
            }
        );
        assert!(state.completed_timeline_view().is_err());
        let fill = row["input"]["fill"].as_u64().unwrap() as u32;
        let word = fill * 0x01010101;
        for v in n["times"].as_array().unwrap() {
            assert_eq!(v.as_u64().unwrap(), word as u64)
        }
    }
    assert_eq!(fixture["resets"].as_array().unwrap().len(), 30);
    for row in fixture["resets"].as_array().unwrap() {
        let mut state = Sim::from_native_constructor_fields();
        let input = &row["input"];
        let admin = FloatBits(input["admin"].as_u64().unwrap() as u32);
        let world = FloatBits(input["world_begin"].as_u64().unwrap() as u32);
        let mut reads = 0;
        state
            .reset_admin_time_76dce0(admin, pc(row["precision"].as_str().unwrap()), || {
                reads += 1;
                Ok::<_, String>(world)
            })
            .unwrap();
        assert_eq!(reads, 2);
        let t = state.time_words();
        let actual = [t.begin_game, t.begin_admin, t.end_game, t.end_admin].map(|v| v.unwrap().0);
        let expected: Vec<_> = row["native"]["times"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u32)
            .collect();
        assert_eq!(actual.as_slice(), expected);
    }
}
#[test]
fn real_owner_preserves_partial_reset_and_sync_failure_without_fake_time_initialization() {
    let mut state = Sim::from_native_constructor_fields();
    let mut reads = 0;
    assert!(
        state
            .reset_admin_time_76dce0(FloatBits(0x3f000000), Precision::Double53, || {
                reads += 1;
                if reads == 1 {
                    Ok(FloatBits(0x40000000))
                } else {
                    Err("second world read")
                }
            })
            .is_err()
    );
    assert_eq!(
        state.time_words(),
        TimeWords {
            begin_game: Some(FloatBits(0x40000000)),
            begin_admin: Some(FloatBits(0xbf000000)),
            end_game: None,
            end_admin: None
        }
    );
    assert!(
        state
            .store_frozen_after_sync(0, || Err("sync stop"))
            .is_err()
    );
    assert_eq!(state.frozen_byte(), 1);
    state
        .store_frozen_after_sync(128, || Ok::<_, String>(()))
        .unwrap();
    assert_eq!(state.frozen_byte(), 128);
    assert!(state.frozen());
    assert!(simulation::borrowed::SimFields::frozen_for_log(&state).is_err());
    assert!(state.game_per_admin_tick().is_err());
}
#[test]
fn process_counter_zero_tail_and_same_failure_latch_survive_reborrows() {
    let fixture = fixture();
    assert_eq!(
        fixture["process_counter_zero_tail"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    for section in fixture["process_counter_zero_tail"].as_array().unwrap() {
        assert!(section["word_offset"].as_u64().unwrap() >= section["raw_size"].as_u64().unwrap());
        assert_eq!(section["initial_words"], serde_json::json!([0, 0]));
    }
    let mut owner = FrameControl::fresh_application();
    assert!(!owner.stopped());
    assert_eq!(
        *owner.counters(),
        simulation::SyncCounters { client: 0, host: 0 }
    );
    {
        let loan = owner.control_loan();
        loan.counters.host = 17;
        *loan.stopped = true;
    }
    assert!(owner.stopped());
    assert_eq!(owner.counters().host, 17);
    {
        let loan = owner.control_loan();
        assert!(*loan.stopped);
        assert_eq!(loan.counters.host, 17);
    }
}
