use aa_numeric::{Binary, Precision};
use aa_runtime::server_clock::*;
use serde_json::{Value, json};
use std::collections::VecDeque;
fn number(bits: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}

struct Source {
    cached: VecDeque<u32>,
    ms: u32,
    fail: bool,
    cached_reads: u32,
    timer_calls: u32,
}
impl TimeSource for Source {
    fn cached_seconds_bits(&mut self) -> std::result::Result<u32, String> {
        self.cached_reads += 1;
        self.cached
            .pop_front()
            .ok_or("required cached Node+58".into())
    }
    fn milliseconds(&mut self) -> std::result::Result<u32, String> {
        self.timer_calls += 1;
        if self.fail {
            Err("timer_required".into())
        } else {
            Ok(self.ms)
        }
    }
}
#[test]
fn native_server_clock_all_images_precisions_rounding_stores_and_lazy_time_reads() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-server-clock/native-server-clock.json"
    ))
    .unwrap();
    let rows = doc["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 210);
    let mut steps = 0;
    for row in rows {
        let p = match row["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            "Extended64" => Precision::Extended64,
            _ => panic!(),
        };
        let mut clock = ServerClock::from_native_constructor();
        assert_eq!(clock.words(), [0, 0, 0]);
        for step in row["steps"].as_array().unwrap() {
            let input = &step["input"];
            let mut source = Source {
                cached: input["cached"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|v| v.as_u64().unwrap() as u32)
                    .collect(),
                ms: input["ms"].as_u64().unwrap_or(0) as u32,
                fail: input["fail_time"].as_bool().unwrap_or(false),
                cached_reads: 0,
                timer_calls: 0,
            };
            let cached = !source.cached.is_empty();
            let word = |key: &str| input[key].as_u64().unwrap() as u32;
            let output = match input["kind"].as_str().unwrap() {
                "restart" => clock
                    .restart(word("period"), word("start"), &mut source, p)
                    .map(|_| None),
                "rate" => clock
                    .host_rate(word("seconds"), &mut source, p)
                    .map(|_| None),
                "raw" => clock.raw_time(cached, &mut source, p).map(Some),
                "bounded" => clock
                    .bounded_time(cached, &mut source, p)
                    .and_then(|v| number(v.0))
                    .map(Some),
                "elapsed" => clock
                    .elapsed_since_limit(cached, &mut source, p)
                    .and_then(|v| number(v.0))
                    .map(Some),
                _ => panic!(),
            };
            let (outcome, extended) = match output {
                Ok(result) => (
                    "ok",
                    result.map(|v| format!("0x{:x}", v.to_x87_extended_bits().unwrap())),
                ),
                Err(Error::Source(e)) if e == "timer_required" => ("timer_required", None),
                Err(e) => panic!("{row}: {e}"),
            };
            assert_eq!(
                json!({"outcome":outcome,"words":clock.words(),"extended":extended,
                "cached_reads":source.cached_reads,"timer_calls":source.timer_calls}),
                step["output"],
                "{row}"
            );
            assert!(source.cached.is_empty());
            steps += 1;
        }
    }
    assert_eq!(steps, 1932);
}

#[test]
fn host_rate_second_read_failure_keeps_first_correction_and_never_asks_qpc() {
    let mut clock = ServerClock::from_native_constructor();
    let mut source = Source {
        cached: VecDeque::new(),
        ms: 0,
        fail: false,
        cached_reads: 0,
        timer_calls: 0,
    };
    clock
        .restart(0x3d800000, 0, &mut source, Precision::Double53)
        .unwrap();
    let old = clock.words();
    source.cached.push_back(10f32.to_bits());
    let result = clock.host_rate(0x3dcccccd, &mut source, Precision::Double53);
    assert!(matches!(result, Err(Error::Source(_))));
    assert_eq!(clock.words()[0], old[0]);
    assert_eq!(clock.words()[1], old[1]);
    assert_ne!(clock.words()[2], old[2]);
    assert_eq!(source.timer_calls, 1);
    assert_eq!(source.cached_reads, 2);
}
