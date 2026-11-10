//! Replays research/phase7-triggers/native-compare-fixtures.json: the
//! original trigger comparison 541DE0 over special binary32 operands and the
//! activation comparison 5425C0 at every precision control, under Unicorn.
use aa_numeric::Precision;
use aa_triggers::{compare, program};
use serde_json::Value;

fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}

fn finite(bits: u32) -> bool {
    bits & 0x7f80_0000 != 0x7f80_0000
}

#[test]
fn original_compare_matches_every_fixture() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-triggers/native-compare-fixtures.json"
    ))
    .unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 4586);
    let (mut compares, mut activations, mut finite_agree) = (0, 0, 0);
    for row in cases {
        let i = &row["input"];
        let want = u(&row["output"]["result"]) != 0;
        let kind = u(&i["kind"]);
        let right = u(&i["right"]);
        match i["function"].as_str().unwrap() {
            "compare" => {
                compares += 1;
                let left = u(&i["left"]);
                assert_eq!(compare::compare_native(left, kind, right), want, "{i}");
                // The finite-only rule of the 546AF0 opcode 1 agrees wherever
                // it accepts the operands.
                if finite(left) && finite(right) {
                    assert_eq!(program::compare(left, kind, right).unwrap(), want, "{i}");
                    finite_agree += 1;
                }
            }
            "activation" => {
                activations += 1;
                let p: Precision = serde_json::from_value(i["precision"].clone()).unwrap();
                let got =
                    compare::activation_compare(u(&i["time"]), u(&i["activation"]), kind, right, p)
                        .unwrap();
                assert_eq!(got, want, "{i}");
            }
            other => panic!("{other}"),
        }
    }
    assert_eq!((compares, activations), (3200, 1386));
    assert_eq!(finite_agree, 13 * 13 * 8);
}
