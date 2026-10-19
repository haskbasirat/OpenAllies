//! Replays native-describe-61f210.json (original 61F210, both images,
//! PC24/PC53/PC64): every logged row's arguments, line, format and the
//! guess/log/delivery event order.
use super::*;

fn fixture() -> serde_json::Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/phase8-orders-input/recruit/native-describe-61f210.json");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn pc(name: &str) -> Precision {
    match name {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        "Extended64" => Precision::Extended64,
        p => panic!("{p}"),
    }
}
fn wide(v: &serde_json::Value) -> Vec<u16> {
    v.as_str().unwrap().encode_utf16().collect()
}

#[test]
fn player_order_describe_61f210_native_rows_replay() {
    let doc = fixture();
    let (mut logged, mut silent, mut outside) = (0, 0, 0);
    for row in doc["rows"].as_array().unwrap() {
        let input = &row["input"];
        let events = row["output"]["events"].as_array().unwrap();
        let kingdom = |key: &serde_json::Value| -> Option<i32> {
            key.as_str().map(|k| input["kingdoms"][k].as_u64().unwrap() as u32 as i32)
        };
        let flags = input["flags"].as_u64().unwrap() as u32;
        let explicit = input["definition"].as_str();
        let empty = input["actor"].is_null();
        if !empty && !input["actor"]["present"].as_bool().unwrap() {
            // NULL first actor: no description, straight to 4ED360 (the Rust
            // selection list cannot hold a NULL actor).
            assert_eq!(events.len(), 1, "{row}");
            assert_eq!(events[0][0], "deliver");
            silent += 1;
            continue;
        }
        if !empty && input["actor"]["b4"].as_u64().unwrap() & 0xff == 0 {
            // vt+B4 false: outside the Rust Actor/Kingdom dispatch domain
            // (object_with_bank refuses before any description is made).
            outside += 1;
            continue;
        }
        // The guess runs only for a default command (bit6 clear).
        let guessed = events.iter().any(|e| e[0] == "guess");
        assert_eq!(guessed, !empty && flags & 0x40 == 0, "{row}");
        let guess = if guessed { input["guess"].as_str() } else { None };
        let order = order_choice(flags, if flags & 0x40 != 0 { explicit } else { None }, guess);
        let player_kingdom = if input["world"].as_bool().unwrap() { kingdom(&input["player_kingdom"]) } else { None };
        let actor = (!empty).then(|| {
            let a = &input["actor"];
            (a["gtid"].as_u64().unwrap() as u32 as i32, wide(&a["ids"]), kingdom(&a["kingdom"]))
        });
        let d = assemble(order, player_kingdom, wide(&input["player"]), input["milliseconds"].as_u64().unwrap() as u32,
            input["view_bits"].as_u64().unwrap() as u32, actor, pc(row["precision"].as_str().unwrap()))
        .unwrap();
        let log = events.iter().find(|e| e[0] == "log").expect("logged row")[1].clone();
        assert_eq!(log["channel"], format!("{CHANNEL_B38B70:#x}"));
        assert_eq!(log["file"], FILE);
        assert_eq!(log["function"], FUNCTION);
        assert_eq!(log["line"].as_u64().unwrap() as u32, d.line(), "{row}");
        assert_eq!(log["format"], d.format(), "{row}");
        let args = &log["args"];
        match &d {
            Description::Empty { time_f64, player, player_kingdom, order } => {
                assert_eq!(args["time_f64"].as_u64().unwrap(), *time_f64, "{row}");
                assert_eq!(&wide(&args["player"]), player);
                assert_eq!(args["player_kingdom"].as_i64().unwrap(), i64::from(*player_kingdom), "{row}");
                assert_eq!(&wide(&args["order"]), order, "{row}");
                // Empty: description then reject (no 4ED360).
                assert!(!events.iter().any(|e| e[0] == "deliver"), "{row}");
            }
            Description::Selected { time_f64, view_f64, player, player_kingdom, actor, actor_ids, actor_kingdom, order } => {
                assert_eq!(args["time_f64"].as_u64().unwrap(), *time_f64, "{row}");
                assert_eq!(args["view_f64"].as_u64().unwrap(), *view_f64, "{row}");
                assert_eq!(&wide(&args["player"]), player);
                assert_eq!(args["player_kingdom"].as_i64().unwrap(), i64::from(*player_kingdom), "{row}");
                assert_eq!(args["actor"].as_i64().unwrap(), i64::from(*actor), "{row}");
                assert_eq!(&wide(&args["actor_ids"]), actor_ids);
                assert_eq!(args["actor_kingdom"].as_i64().unwrap(), i64::from(*actor_kingdom), "{row}");
                assert_eq!(&wide(&args["order"]), order, "{row}");
                // Native order: B4, 108, [guess], timer, log, deliver.
                let kinds: Vec<&str> = events.iter().map(|e| e[0].as_str().unwrap()).filter(|k| *k != "cstring").collect();
                let mut expected = vec!["b4", "108"];
                if guessed { expected.push("guess"); }
                expected.extend(["timer", "log", "deliver"]);
                assert_eq!(kinds, expected, "{row}");
            }
        }
        // The timer is always B396E0 (40B000's B39690+50 included).
        assert!(events.iter().filter(|e| e[0] == "timer").all(|e| e[1] == "0xb396e0"));
        logged += 1;
    }
    assert!(logged > 3000 && silent > 0 && outside > 0, "{logged} {silent} {outside}");
}

#[test]
fn player_order_describe_time_conversion_is_precision_exact() {
    // 0x80000001 ms: PC24 rounds the 2^32 adjustment to 2^31 before *0.001.
    let a = seconds_f64_bits(0x8000_0001, Precision::Single24).unwrap();
    let b = seconds_f64_bits(0x8000_0001, Precision::Double53).unwrap();
    assert_ne!(a, b);
    assert_eq!(view_f64_bits(0), 0);
    assert_eq!(order_choice(0x40, None, Some("move")), NULL_ORDER);
    assert_eq!(order_choice(0x02, Some("recruit"), Some("move")), "move");
}
