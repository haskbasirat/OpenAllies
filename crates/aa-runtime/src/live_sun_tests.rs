//! Replays research/live-battle-ticks/native-sun.json: the original 4EDB10 and
//! 4EDDC0 executed under Unicorn at PC24/PC53/PC64 (seam: 50ACC0 links the new
//! event at Sun+4). Every case compares all Sun words +14..+6C, the number of
//! schedules with their delay/data, and the final scheduled state.
use super::*;
use aa_world::queues::{Clock, Kind, Queues};

fn precision(name: &str) -> Precision {
    match name {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        "Extended64" => Precision::Extended64,
        _ => panic!("{name}"),
    }
}
fn words(sun: &State) -> Vec<u32> {
    let mut out = Vec::new();
    for c in sun.target_colors.iter().flatten() {
        out.push(*c);
    }
    for c in sun.current_colors.iter().flatten() {
        out.push(*c);
    }
    out.extend([sun.color_duration, sun.current_dimmer, sun.target_dimmer, sun.dimmer_duration, sun.start_time]);
    out
}
fn load(words: &[u32]) -> State {
    let mut sun = State::new([0; 3]);
    let mut i = 0;
    for k in 0..3 {
        for j in 0..3 {
            sun.target_colors[k][j] = words[i];
            i += 1;
        }
    }
    for k in 0..3 {
        for j in 0..3 {
            sun.current_colors[k][j] = words[i];
            i += 1;
        }
    }
    sun.color_duration = words[18];
    sun.current_dimmer = words[19];
    sun.target_dimmer = words[20];
    sun.dimmer_duration = words[21];
    sun.start_time = words[22];
    sun
}

#[test]
fn native_sun_set_colors_and_update_replay() {
    let doc: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/live-battle-ticks/native-sun.json"
    ))
    .unwrap();
    let rows = doc["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 480);
    for (n, row) in rows.iter().enumerate() {
        let p = precision(row["precision"].as_str().unwrap());
        let input = &row["input"];
        let w: Vec<u32> = input["words"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as u32).collect();
        assert_eq!(w.len(), 23);
        let mut sun = load(&w);
        assert_eq!(words(&sun), w, "case {n} layout");
        let now = input["now"].as_u64().unwrap() as u32;
        let clock = Clock { begin_bits: now, end_bits: now, step_bits: 0x3d80_0000 };
        let mut queues = Queues::new(clock, p).unwrap();
        queues.initialize(3, Kind::Deferred, 64).unwrap();
        let owner = 7;
        if input["scheduled"].as_u64().unwrap() != 0 {
            queues.add_after(3, owner, 0x55, 0x3f80_0000).unwrap();
        }
        let before = queues.snapshot();
        let result = if input["op"] == "set_colors" {
            let c: Vec<Vec<u32>> = input["colors"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as u32).collect())
                .collect();
            let colors = [
                [c[0][0], c[0][1], c[0][2]],
                [c[1][0], c[1][1], c[1][2]],
                [c[2][0], c[2][1], c[2][2]],
            ];
            set_colors(&mut sun, colors, input["duration"].as_u64().unwrap() as u32, &mut queues, owner, now)
        } else {
            update(&mut sun, &mut queues, owner, now, p)
        };
        let out = &row["output"];
        let expected: Vec<u32> = out["words"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as u32).collect();
        match result {
            Ok(()) => assert_eq!(words(&sun), expected, "case {n} ({}) {input}", row["precision"]),
            Err(e) => panic!("case {n} {e} {input}"),
        }
        let calls = out["calls"].as_array().unwrap();
        let added: usize = queues
            .snapshot()
            .iter()
            .map(|q| q.buckets.iter().flatten().count())
            .sum::<usize>()
            - before.iter().map(|q| q.buckets.iter().flatten().count()).sum::<usize>();
        assert_eq!(added, calls.len(), "case {n} schedules");
        for call in calls {
            assert_eq!(call["delay"].as_u64().unwrap() as u32, SUN_DELAY_BITS);
            assert_eq!(call["data"].as_u64().unwrap(), 0);
        }
        assert_eq!(queues.owner_head(owner).is_some(), out["scheduled_after"].as_bool().unwrap(), "case {n}");
    }
}
