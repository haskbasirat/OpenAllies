use aa_actors::state::{self, Descriptor, Effects, Fields};
use serde_json::{Value, json};
#[derive(Default)]
struct Observe(Vec<Value>);
impl Effects for Observe {
    fn enter(
        &mut self,
        _: &mut Fields,
        new: &'static Descriptor,
        old: Option<&'static Descriptor>,
    ) -> state::Result<()> {
        self.0
            .push(json!(["enter", new.index, old.map(|s| s.index)]));
        Ok(())
    }
    fn leave(&mut self, _: &mut Fields, old: &'static Descriptor) -> state::Result<()> {
        self.0.push(json!(["leave", old.index]));
        Ok(())
    }
    fn rebuild_abilities(&mut self, _: &mut Fields) -> state::Result<()> {
        self.0.push(json!(["rebuild"]));
        Ok(())
    }
}
#[test]
fn original_registered_descriptors_masks_and_transitions_match() {
    let native: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-actors/native-actor-states.json"
    ))
    .unwrap();
    for (actual, record) in state::DESCRIPTORS
        .iter()
        .zip(native["registration"]["records"].as_array().unwrap())
    {
        assert_eq!(u64::from(actual.index), record["index"].as_u64().unwrap());
        assert_eq!(actual.name, record["name"].as_str().unwrap());
        assert_eq!(
            u64::from(actual.group.map(u32::from).unwrap_or(u32::MAX)),
            record["group"].as_u64().unwrap()
        );
        assert_eq!(actual.exclusive, record["exclusive"].as_bool().unwrap());
    }
    assert_eq!(
        state::GROUP_MASKS.to_vec(),
        native["registration"]["masks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        state::LOAD_CLEARED_MIND_MASK,
        state::DESCRIPTORS
            .iter()
            .filter(|s| s.group == Some(4))
            .fold(0, |bits, s| bits | (1 << s.index))
    );
    for row in native["group_cases"].as_array().unwrap() {
        assert_eq!(
            state::group(
                row["bits"].as_u64().unwrap(),
                row["group"].as_u64().unwrap() as u8
            )
            .unwrap()
            .map(|s| u64::from(s.index)),
            row["selected"].as_u64()
        );
    }
    for row in native["state_cases"].as_array().unwrap() {
        let input = &row["input"];
        let out = &row["output"];
        let mut fields = Fields {
            bits: input["bits"].as_u64().unwrap(),
            thing_flags: input["flags"].as_u64().unwrap() as u32,
        };
        let mut effects = Observe::default();
        state::set(
            &mut fields,
            input["index"].as_u64().unwrap() as u8,
            input["enable"].as_bool().unwrap(),
            &mut effects,
        )
        .unwrap();
        assert_eq!(fields.bits, out["bits"].as_u64().unwrap());
        assert_eq!(
            u64::from(fields.thing_flags),
            out["flags"].as_u64().unwrap()
        );
        assert_eq!(json!(effects.0), out["events"], "{input}");
    }
}
#[test]
fn invalid_inputs_and_callback_errors_do_not_fabricate_success() {
    struct Fail;
    impl Effects for Fail {
        fn enter(
            &mut self,
            _: &mut Fields,
            _: &'static Descriptor,
            _: Option<&'static Descriptor>,
        ) -> state::Result<()> {
            Err(state::Error::Callback("failure".into()))
        }
        fn leave(&mut self, _: &mut Fields, _: &'static Descriptor) -> state::Result<()> {
            Err(state::Error::Callback("failure".into()))
        }
        fn rebuild_abilities(&mut self, _: &mut Fields) -> state::Result<()> {
            Err(state::Error::Callback("failure".into()))
        }
    }
    let mut fields = Fields {
        bits: 0,
        thing_flags: 0,
    };
    assert!(state::set(&mut fields, 24, true, &mut Fail).is_err());
    assert_eq!(fields.bits, 1 << 24);
    assert_eq!(fields.thing_flags, 0);
    let saved = fields;
    assert!(state::set(&mut fields, 42, true, &mut Fail).is_err());
    assert_eq!(fields, saved);
    assert!(state::group(0, 31).is_err());
}
