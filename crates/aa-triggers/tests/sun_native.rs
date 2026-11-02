//! Replays research/live-battle-ticks/native-sun.json (the original 4EDB10
//! SetColors and 4EDDC0 update executed under Unicorn at PC24/PC53/PC64) on
//! `aa_triggers::sun::Sun`: every Sun word +14..+6C, the number of 50ACC0
//! schedules and the scheduled state after the call.
use aa_numeric::Precision;
use aa_triggers::sun::Sun;

fn precision(name: &str) -> Precision {
    match name {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        "Extended64" => Precision::Extended64,
        _ => panic!("{name}"),
    }
}
fn words(sun: &Sun) -> Vec<u32> {
    let mut out: Vec<u32> = sun.target_colors.iter().flatten().copied().collect();
    out.extend(sun.current_colors.iter().flatten().copied());
    out.extend([sun.color_duration, sun.current_dimmer, sun.target_dimmer, sun.dimmer_duration, sun.start_time]);
    out
}
fn load(w: &[u32]) -> Sun {
    let mut sun = Sun::new();
    for k in 0..3 {
        for j in 0..3 {
            sun.target_colors[k][j] = w[3 * k + j];
            sun.current_colors[k][j] = w[9 + 3 * k + j];
        }
    }
    sun.color_duration = w[18];
    sun.current_dimmer = w[19];
    sun.target_dimmer = w[20];
    sun.dimmer_duration = w[21];
    sun.start_time = w[22];
    sun
}

#[test]
fn native_sun_set_colors_and_update_replay() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/live-battle-ticks/native-sun.json");
    let doc: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let rows = doc["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 480);
    for (n, row) in rows.iter().enumerate() {
        let p = precision(row["precision"].as_str().unwrap());
        let input = &row["input"];
        let w: Vec<u32> = input["words"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as u32).collect();
        let mut sun = load(&w);
        assert_eq!(words(&sun), w, "case {n} layout");
        let now = input["now"].as_u64().unwrap() as u32;
        sun.scheduled = input["scheduled"].as_u64().unwrap() != 0;
        let before = sun.scheduled;
        // The delivery of an update unlinks the Sun's event first.
        let schedules = if input["op"] == "set_colors" {
            let c: Vec<Vec<u32>> = input["colors"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as u32).collect())
                .collect();
            let colors = [[c[0][0], c[0][1], c[0][2]], [c[1][0], c[1][1], c[1][2]], [c[2][0], c[2][1], c[2][2]]];
            sun.set_colors(colors, input["duration"].as_u64().unwrap() as u32, now);
            usize::from(sun.scheduled && !before)
        } else {
            // The oracle's update ran with the event still linked when
            // `scheduled` was set: 4EDDC0 then does not reschedule.
            sun.update(now, p).unwrap();
            usize::from(sun.scheduled && !before)
        };
        let out = &row["output"];
        let expected: Vec<u32> = out["words"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as u32).collect();
        assert_eq!(words(&sun), expected, "case {n} ({}) {input}", row["precision"]);
        let calls = out["calls"].as_array().unwrap();
        assert_eq!(schedules, calls.len(), "case {n} schedules");
        assert_eq!(sun.scheduled, out["scheduled_after"].as_bool().unwrap(), "case {n}");
    }
}

#[test]
fn sun_chunk_load_layout() {
    // 4EDBD0 field order on a synthetic SUN chunk (the installed maps' SUN
    // chunks load in aa-play's tests/triggers.rs).
    let mut sun = Sun::new();
    // Version 2 Sun, Updatable version 1 with id 7, then the fields.
    let mut bytes = vec![];
    for w in [2u32, 1, 7, 0, 0, 0x3f80_0000] {
        bytes.extend(w.to_le_bytes());
    }
    for _ in 0..18 {
        bytes.extend(0x3f00_0000u32.to_le_bytes());
    }
    for w in [0u32, 0, 0, 0, 0xbf80_0000] {
        bytes.extend(w.to_le_bytes());
    }
    bytes.push(1);
    bytes.extend([0u8; 8]);
    bytes.push(0);
    bytes.extend([0u8; 8]);
    assert_eq!(sun.load(&bytes).unwrap(), bytes.len());
    assert_eq!(sun.saved_updatable, Some(7));
    assert_eq!(sun.current_colors, [[0x3f00_0000; 3]; 3]);
    assert_eq!(sun.byte_70, 1);
}

#[test]
fn adjust_dimmer_follows_4ed820() {
    // 4ED820 has no oracle rows: this pins its branches as read from the
    // instructions (no parity claim beyond them).
    let (one, half, zero) = (0x3f80_0000u32, 0x3f00_0000u32, 0u32);
    let now = 10.0f32.to_bits();
    // a zero duration copies at once
    let mut sun = Sun::new();
    sun.adjust_dimmer(one, zero, now);
    assert_eq!((sun.current_dimmer, sun.dimmer_duration, sun.scheduled), (one, 0, false));
    // the target already reached copies at once
    let mut sun = Sun::new();
    sun.current_dimmer = one;
    sun.adjust_dimmer(one, half, now);
    assert_eq!((sun.dimmer_duration, sun.scheduled), (0, false));
    // otherwise an unscheduled Sun schedules itself and starts its clock
    let mut sun = Sun::new();
    sun.adjust_dimmer(one, half, now);
    assert_eq!((sun.target_dimmer, sun.dimmer_duration, sun.scheduled, sun.start_time), (one, half, true, now));
    // an already scheduled Sun keeps its clock
    let later = 11.0f32.to_bits();
    sun.adjust_dimmer(half, half, later);
    assert_eq!((sun.target_dimmer, sun.start_time), (half, now));
    // a delivery moves the dimmer towards the target by dt / duration
    let mut sun = Sun::new();
    sun.adjust_dimmer(one, one, now);
    sun.deliver(10.5f32.to_bits(), Precision::Double53).unwrap();
    assert_eq!(f32::from_bits(sun.current_dimmer), 0.5);
    assert!(sun.scheduled, "the transition remains");
}
