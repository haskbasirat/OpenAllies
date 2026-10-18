use aa_numeric::Precision;
use aa_runtime::camera_projection::{self, Error, Frustum, Result, World};
use serde_json::{Value, json};
use std::collections::BTreeMap;

struct Storage {
    case: Value,
    words: BTreeMap<u32, u32>,
    trace: Vec<Value>,
}
impl Storage {
    fn new(case: &Value) -> Self {
        let offsets = [0x34c, 0x350, 0x3fc, 0x40c]
            .into_iter()
            .chain((0x6e0..0x720).step_by(4));
        Self {
            case: case.clone(),
            words: offsets
                .map(|offset| (offset, 0xa5a5_0000 + offset))
                .collect(),
            trace: vec![],
        }
    }
    fn event(&mut self, kind: &str, offset: Option<u32>) -> Result<()> {
        self.trace
            .push(offset.map_or_else(|| json!([kind]), |offset| json!([kind, offset])));
        let n = self.trace.len() as u64;
        if let Some(changes) = self.case["mutations"].as_array() {
            for change in changes {
                if change["event"] == n {
                    self.words
                        .insert(word(&change["offset"]), word(&change["bits"]));
                }
            }
        }
        if self.case["stop"] == n {
            return Err(Error::Service("missing".into()));
        }
        Ok(())
    }
    fn output(&self, outcome: &str) -> Value {
        json!({"outcome":outcome,"trace":self.trace,
            "words":self.words.iter().map(|(offset,bits)|json!([offset,bits])).collect::<Vec<_>>()})
    }
}
impl World for Storage {
    fn renderer_flag_54(&mut self) -> Result<bool> {
        self.event("flag54", None)?;
        Ok(self.case["flag54"].as_bool().unwrap())
    }
    fn store_word(&mut self, offset: u32, bits: u32) -> Result<()> {
        self.event("store", Some(offset))?;
        *self.words.get_mut(&offset).unwrap() = bits;
        Ok(())
    }
    fn load_word(&mut self, offset: u32) -> Result<u32> {
        self.event("load", Some(offset))?;
        Ok(self.words[&offset])
    }
}
fn word(value: &Value) -> u32 {
    value.as_u64().unwrap().try_into().unwrap()
}
fn frustum(case: &Value) -> Frustum {
    let bounds = &case["frustum"];
    Frustum {
        left: word(&bounds[0]),
        right: word(&bounds[1]),
        top: word(&bounds[2]),
        bottom: word(&bounds[3]),
        near: word(&bounds[4]),
        far: word(&bounds[5]),
        orthographic: case["orthographic"].as_bool().unwrap(),
    }
}

#[test]
fn aa_projection_native_stores_flags_and_partial_failures() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/kohan-leads-20261016/projection/native-projection.json"
    ))
    .unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 1536);
    for case in cases {
        let mut storage = Storage::new(case);
        let pc: Precision = serde_json::from_value(case["precision"].clone()).unwrap();
        let result = camera_projection::build(&frustum(case), pc, &mut storage);
        let outcome = match result {
            Ok(()) => "ok",
            Err(Error::Service(ref message)) if message == "missing" => "missing",
            Err(error) => panic!("{}: unexpected {error}", case["name"]),
        };
        assert_eq!(
            storage.output(outcome),
            case["output"],
            "{} {} {}",
            case["name"],
            case["image"],
            case["precision"]
        );
    }
}

#[test]
fn unsupported_numeric_inputs_refuse_lazily_without_rollback() {
    let case = json!({"flag54":false});
    let mut storage = Storage::new(&case);
    let f = Frustum {
        left: 0,
        right: 1.0f32.to_bits(),
        bottom: 0,
        top: 1.0f32.to_bits(),
        near: 0x7f80_0000,
        far: 2.0f32.to_bits(),
        orthographic: false,
    };
    assert!(matches!(
        camera_projection::build(&f, Precision::Double53, &mut storage),
        Err(Error::Numeric(aa_numeric::Error::NonFiniteInput))
    ));
    assert_eq!(storage.trace, [json!(["store", 0x34c])]);
    assert_eq!(storage.words[&0x34c], f.near);
    // This is a portable finite-domain refusal, not native infinity/exception parity.
    let mut storage = Storage::new(&case);
    let f = Frustum {
        near: 0,
        left: 0,
        right: 0,
        ..f
    };
    assert!(matches!(
        camera_projection::build(&f, Precision::Single24, &mut storage),
        Err(Error::Numeric(aa_numeric::Error::DivisionByZero))
    ));
    assert_eq!(
        storage.trace,
        [
            json!(["store", 0x34c]),
            json!(["store", 0x350]),
            json!(["flag54"])
        ]
    );
    assert_eq!(storage.words[&0x350], 2.0f32.to_bits());
}
