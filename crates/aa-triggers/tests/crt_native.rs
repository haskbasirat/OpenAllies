//! Replays research/phase8-triggers/native-format.json: the ORIGINAL VC7.1
//! _vswprintf (96C7EC) output for %f/%.2f/%d.  Values >= 1e13 must be rejected.
use aa_triggers::crt;

#[test]
fn native_vswprintf_replay() {
    let doc: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/phase8-triggers/native-format.json"
    ))
    .unwrap();
    let mut checked = 0;
    let mut rejected = 0;
    let mut failures = vec![];
    for case in doc["cases"].as_array().unwrap() {
        let fmt = case["fmt"].as_str().unwrap();
        let bits = case["bits"].as_u64().unwrap();
        let text = case["text"].as_str().unwrap();
        let got = match fmt {
            "%d" => Ok(crt::format_d(bits as u32 as i32)),
            "%f" => crt::format_f(bits, 6),
            "%.2f" => crt::format_f(bits, 2),
            other => panic!("{other}"),
        };
        match got {
            Ok(got) => {
                checked += 1;
                if got != text {
                    failures.push(format!("{fmt} {:e}: rust {got} native {text}", f64::from_bits(bits)));
                }
            }
            Err(_) => {
                rejected += 1;
                assert!(f64::from_bits(bits & !(1 << 63)) >= 1e13, "{fmt} {bits:#x} rejected");
            }
        }
    }
    assert!(failures.is_empty(), "{} mismatches:\n{}", failures.len(), failures[..failures.len().min(20)].join("\n"));
    assert!(checked > 15000, "{checked} checked, {rejected} rejected");
}
