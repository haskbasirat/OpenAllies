//! Replays research/phase8-net/native-speed.json (original 0x75D810 authority
//! test) and checks that the decree reaches Timeline::set_game_speed.
use aa_netcode::speed::{process_set_game_speed, set_game_speed_gate, SpeedAuthority};
use aa_orders::payload::Field;
use aa_runtime::simulation::{AdminNode, Driver, Gates, PowPath, Role, SpeedContext, SpeedSettings, Timeline};
use aa_simulation::FloatBits;
use serde_json::Value;

fn fixture() -> Value {
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-net/native-speed.json");
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}
fn flag(v: &Value) -> bool {
    v.as_bool().unwrap()
}

#[test]
fn native_set_game_speed_authority() {
    let v = fixture();
    let cases = v["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 48);
    let mut reached = 0;
    for c in cases {
        let authority = SpeedAuthority {
            session: flag(&c["session"]),
            ordering_player: flag(&c["ordering_player"]),
            ordering_client: flag(&c["ordering_client"]),
            client_is_host: flag(&c["client_is_host"]),
        };
        let cutscene = c["world_cutscene"].as_bool();
        let speed = c["speed_bits"].as_u64().unwrap() as u32;
        let got = set_game_speed_gate(&[Field::Uint { width: 32, value: speed }], &authority, cutscene).unwrap();
        let native: Vec<u32> = c["set_game_speed"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| {
                assert!(x["sim"].as_bool().unwrap(), "called on [0xB36554]");
                x["speed_bits"].as_u64().unwrap() as u32
            })
            .collect();
        assert_eq!(got.map(|f| f.0).into_iter().collect::<Vec<_>>(), native, "{c}");
        reached += native.len();
    }
    assert_eq!(reached, 2);
}

#[test]
fn decree_drives_timeline_set_game_speed() {
    // x87 precision of the simulation thread (lead policy: PC53).
    let precision = serde_json::from_str("\"Double53\"").unwrap();
    let ticks = FloatBits(0x4180_0000); // 16.0 admin ticks per second
    let timeline = Timeline::new(ticks.0, precision).unwrap();
    let mut driver = Driver::new(AdminNode::new(Role::Host), timeline, Gates::default(), ticks, precision);
    let settings = SpeedSettings {
        game_speed_min: FloatBits(0xc080_0000),
        game_speed_max: FloatBits(0x4080_0000),
        film_min: FloatBits(0xc080_0000),
        film_max: FloatBits(0x40c0_0000),
        admin_ticks_per_second: ticks,
    };
    let context = SpeedContext { session_mode: 2, session_f4: true, world_cutscene: Some(false) };
    let fields = [Field::Uint { width: 32, value: 0x3f80_0000 }];
    let allowed = SpeedAuthority { session: true, ordering_player: true, ordering_client: true, client_is_host: true };

    let mut direct = driver.timeline;
    let want = direct.set_game_speed(FloatBits(0x3f80_0000), &settings, &context, PowPath::Sse2, precision).unwrap();
    let got = process_set_game_speed(&mut driver, &fields, &allowed, &settings, &context, PowPath::Sse2).unwrap();
    assert_eq!(got, Some(want));
    assert_eq!(driver.timeline, direct);

    // A client that is not the host: 0x74B900 fails, the timeline is untouched.
    let before = driver.timeline;
    let refused = SpeedAuthority { client_is_host: false, ..allowed };
    let fields = [Field::Uint { width: 32, value: 0x4000_0000 }];
    assert_eq!(process_set_game_speed(&mut driver, &fields, &refused, &settings, &context, PowPath::Sse2).unwrap(), None);
    assert_eq!(driver.timeline, before);
    // Cutscene world: 0x76AD90 refuses.
    let cutscene = SpeedContext { world_cutscene: Some(true), ..context };
    assert_eq!(process_set_game_speed(&mut driver, &fields, &allowed, &settings, &cutscene, PowPath::Sse2).unwrap(), None);
    // Malformed payload is a typed error.
    assert!(process_set_game_speed(&mut driver, &[], &allowed, &settings, &context, PowPath::Sse2).is_err());
}
