//! Original 4D9C80/4DACE0 and 52C580 executions, including owned queue/current
//! references. The registry-name to live-definition resolution belongs to the
//! runtime catalog and is not replaced by synthetic identities in production.
use aa_mind::{
    Arena, MindId, StateId,
    construct::{self, Kind, World},
    saved_command::{self, Command},
    saved_load::{self, CommandBindings, Error, Saved},
};
use aa_numeric::Precision;
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn fixture() -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/resume-20261016-mind-commands/native-commands.json");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn bytes(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&hex[at..at + 2], 16).unwrap())
        .collect()
}
fn fields(command: &Command) -> Value {
    json!({
        "definition":command.definition,
        "flags":command.flags,"word10":command.word10,
        "object_definition":command.object_definition,
        "position_bits":command.position_bits,"auxiliary_bits":command.auxiliary_bits,
        "scalar_bits":command.scalar_bits,"parameter_definition":command.parameter_definition,
        "extra_definition":command.extra_definition,"boolean":command.boolean,"trailing":command.trailing,
    })
}
fn same_command(command: &Command, expected: &Value) {
    for (key, value) in fields(command).as_object().unwrap() {
        assert_eq!(value, &expected[key], "{key}: {command:?}");
    }
    assert_eq!(expected["owned_parameters_null"], true);
}
struct Fixed;
impl World for Fixed {
    fn random_word(&mut self) -> aa_mind::Result<u32> {
        Ok(0x1234_5678)
    }
    fn performer_ability_count(&self, _: u32) -> aa_mind::Result<usize> {
        Ok(0)
    }
    fn set_ability_jitter(&mut self, _: u32, _: usize, _: u32) -> aa_mind::Result<()> {
        unreachable!()
    }
}
fn constructed(pc: Precision) -> (construct::Installed, Arena) {
    let value = construct::construct(Kind::Element, 7, &mut Fixed, pc).unwrap();
    let bindings: BTreeMap<_, _> = value
        .states
        .iter()
        .enumerate()
        .map(|(i, s)| (s.offset, StateId(i as u32 + 100)))
        .collect();
    let mut arena = Arena::default();
    let installed = value.install(MindId(1), &bindings, &mut arena).unwrap();
    (installed, arena)
}

#[test]
fn saved_command_fields_and_mind_ownership_match_original_x86() {
    let doc = fixture();
    let mut count = 0;
    for case in doc["cases"].as_array().unwrap() {
        count += 1;
        let payload = bytes(case["payload"].as_str().unwrap());
        let expected = &case["native"];
        if case["kind"] == "command" {
            let decoded = saved_command::decode(&payload);
            if expected["outcome"] == "bad_version" {
                match decoded {
                    Err(Error::BadVersion { offset, max, .. }) => {
                        assert_eq!(offset as u64, expected["consumed"].as_u64().unwrap());
                        assert_eq!(max, 3);
                    }
                    other => panic!("{other:?}"),
                }
            } else {
                let (command, used) = decoded.unwrap();
                assert_eq!(used as u64, expected["consumed"].as_u64().unwrap());
                same_command(&command, &expected["fields"]);
                assert_eq!(expected["fields"]["reference_count"], 0);
            }
            continue;
        }
        let pc = match case["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            _ => Precision::Extended64,
        };
        let (saved, used) = saved_load::decode(Kind::Element, &payload).unwrap();
        assert_eq!(used as u64, expected["consumed"].as_u64().unwrap());
        let Saved::Element(element) = &saved else {
            panic!("expected Element");
        };
        let base = element.base.as_ref().unwrap();
        let pending = expected["pending"].as_array().unwrap();
        assert_eq!(base.commands.len(), pending.len());
        for (command, value) in base.commands.iter().zip(pending) {
            same_command(command, value);
            assert_eq!(value["reference_count"], 1);
        }
        match &base.current_command {
            Some(command) => {
                same_command(command, &expected["current"]);
                assert_eq!(expected["current"]["reference_count"], 1);
            }
            None => assert!(expected["current"].is_null()),
        }
        assert_eq!(expected["distinct_allocations"], true);
        let bindings = CommandBindings {
            pending: (1..=base.commands.len() as u32).collect(),
            current: base
                .current_command
                .as_ref()
                .map(|_| base.commands.len() as u32 + 1),
        };
        let (mut installed, mut arena) = constructed(pc);
        let applied =
            saved_load::apply_with_commands(&saved, &mut installed, &mut arena, &bindings).unwrap();
        assert_eq!(applied.serialized_id, Some(7));
        let mind = arena.mind(MindId(1)).unwrap();
        assert_eq!(
            mind.pending_commands.iter().copied().collect::<Vec<_>>(),
            bindings.pending
        );
        assert_eq!(mind.current_command, bindings.current);
        for (i, key) in ["word24", "word28", "word2c"].iter().enumerate() {
            assert_eq!(mind.words_24_30[i] as u64, expected[key].as_u64().unwrap());
        }
        let reverse: BTreeMap<_, _> = installed
            .state_bindings
            .iter()
            .map(|(off, id)| (*id, *off))
            .collect();
        let stack: Vec<_> = mind.stack.iter().map(|id| reverse[&id.unwrap()]).collect();
        assert_eq!(json!(stack), expected["stack"]);
        assert!(installed.update_head.is_none());
    }
    assert_eq!(count, 369);
}

#[test]
fn missing_or_aliased_owned_bindings_never_drop_saved_commands() {
    let doc = fixture();
    let case = doc["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| {
            c["kind"] == "mind"
                && c["native"]["pending"].as_array().unwrap().len() == 3
                && !c["native"]["current"].is_null()
        })
        .unwrap();
    let (saved, _) =
        saved_load::decode(Kind::Element, &bytes(case["payload"].as_str().unwrap())).unwrap();
    for bindings in [
        None,
        Some(CommandBindings::default()),
        Some(CommandBindings {
            pending: vec![1, 2, 3],
            current: Some(3),
        }),
        Some(CommandBindings {
            pending: vec![0, 2, 3],
            current: Some(4),
        }),
    ] {
        let (mut installed, mut arena) = constructed(Precision::Double53);
        let before = (
            installed.words.clone(),
            installed.bytes.clone(),
            arena.clone(),
        );
        let result = match bindings {
            None => saved_load::apply(&saved, &mut installed, &mut arena),
            Some(value) => {
                saved_load::apply_with_commands(&saved, &mut installed, &mut arena, &value)
            }
        };
        assert!(result.is_err());
        assert_eq!((installed.words, installed.bytes, arena), before);
    }
}

#[test]
fn truncated_commands_and_unported_parameter_sets_are_explicit_errors() {
    let doc = fixture();
    for case in doc["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["kind"] == "command" && c["native"]["outcome"] == "loaded")
    {
        let payload = bytes(case["payload"].as_str().unwrap());
        for end in 0..payload.len() {
            assert!(
                matches!(
                    saved_command::decode(&payload[..end]),
                    Err(Error::ShortRead { .. })
                ),
                "prefix {end}"
            );
        }
    }
    // version0 + null definition + bit5 + scalar fields + null object + trailing.
    let mut payload = vec![0u8; 37];
    payload[6..10].copy_from_slice(&32u32.to_le_bytes());
    assert!(matches!(
        saved_command::decode(&payload),
        Err(Error::Unsupported(_))
    ));
}

#[test]
fn embedded_original_map_commands_match_native_loads() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/resume-20261016-mind-commands/native-map-commands.json");
    let doc: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert!(cases.len() >= 647, "embedded saved Minds: {}", cases.len());
    assert!(doc["maps"].as_u64().unwrap() >= 57);
    for case in cases {
        let payload = bytes(case["payload"].as_str().unwrap());
        let (saved, used) = saved_load::decode(Kind::Element, &payload).unwrap();
        assert_eq!(used, payload.len(), "{}", case["source"]);
        assert_eq!(used as u64, case["native"]["consumed"].as_u64().unwrap());
        let Saved::Element(element) = &saved else {
            panic!("expected Element");
        };
        let base = element.base.as_ref().unwrap();
        let native = &case["native"];
        let pending = native["pending"].as_array().unwrap();
        assert_eq!(base.commands.len(), pending.len());
        for (command, expected) in base.commands.iter().zip(pending) {
            same_command(command, expected);
            assert_eq!(expected["reference_count"], 1);
        }
        match &base.current_command {
            Some(command) => {
                same_command(command, &native["current"]);
                assert_eq!(native["current"]["reference_count"], 1);
            }
            None => assert!(native["current"].is_null()),
        }
        let bindings = CommandBindings {
            pending: (1..=base.commands.len() as u32).collect(),
            current: base
                .current_command
                .as_ref()
                .map(|_| base.commands.len() as u32 + 1),
        };
        let (mut installed, mut arena) = constructed(Precision::Double53);
        saved_load::apply_with_commands(&saved, &mut installed, &mut arena, &bindings)
            .unwrap_or_else(|e| panic!("{}: {e}", case["source"]));
        let mind = arena.mind(MindId(1)).unwrap();
        assert_eq!(
            mind.pending_commands.iter().copied().collect::<Vec<_>>(),
            bindings.pending
        );
        assert_eq!(mind.current_command, bindings.current);
        let reverse: BTreeMap<_, _> = installed
            .state_bindings
            .iter()
            .map(|(off, id)| (*id, *off))
            .collect();
        let stack: Vec<_> = mind.stack.iter().map(|id| reverse[&id.unwrap()]).collect();
        assert_eq!(json!(stack), native["stack"]);
    }
}
