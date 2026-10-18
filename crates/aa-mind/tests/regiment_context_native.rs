//! Full context application of original528C20 fixtures plus independent4C2D90
//! table/range cases. Runtime pointers become supplied portable handles.
use aa_mind::{
    Arena, MindId, StateId,
    construct::{self, Kind, World},
    regiment_load::{self, Context},
    saved_load::{self, CommandBindings, Saved},
};
use aa_numeric::Precision;
use serde_json::{Value, json};
use std::collections::BTreeMap;
fn read(name: &str) -> Value {
    serde_json::from_slice(
        &std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../research")
                .join(name),
        )
        .unwrap(),
    )
    .unwrap()
}
fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}
fn word(b: &[u8], i: u32) -> u32 {
    u32::from_le_bytes(b[i as usize..i as usize + 4].try_into().unwrap())
}
struct NoCalls;
impl World for NoCalls {
    fn random_word(&mut self) -> aa_mind::Result<u32> {
        panic!("Regiment RNG")
    }
    fn performer_ability_count(&self, _: u32) -> aa_mind::Result<usize> {
        panic!("Regiment abilities")
    }
    fn set_ability_jitter(&mut self, _: u32, _: usize, _: u32) -> aa_mind::Result<()> {
        panic!("Regiment jitter")
    }
}
fn construct(pc: Precision) -> (construct::Installed, Arena) {
    let made = construct::construct(Kind::Regiment, 7, &mut NoCalls, pc).unwrap();
    let bindings = made
        .states
        .iter()
        .enumerate()
        .map(|(i, s)| (s.offset, StateId(i as u32 + 1)))
        .collect();
    let mut arena = Arena::default();
    let installed = made.install(MindId(1), &bindings, &mut arena).unwrap();
    (installed, arena)
}
fn bindings(saved: &regiment_load::Regiment) -> CommandBindings {
    let base = saved.base.as_ref().unwrap();
    let count = base.commands.len() as u32;
    CommandBindings {
        pending: (100..100 + count).collect(),
        current: base.current_command.as_ref().map(|_| 100 + count),
    }
}

#[test]
fn context_loaded_fields_and_single_owned_containers_match_native_regiment() {
    let mut doc = read("skirmish-20261016-regiment-load/native-regiment.json");
    let source = read("skirmish-20261016-regiment-context/native-source-context.json");
    doc["cases"].as_array_mut().unwrap().extend(source["cases"].as_array().unwrap().iter().cloned());
    let mut total = 0;
    for case in doc["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["native"]["outcome"] == "loaded" && c["version"].as_u64().unwrap() >= 2)
    {
        total += 1;
        let native = &case["native"];
        let image = hex(native["image"].as_str().unwrap());
        let (Saved::Regiment(saved), _) =
            saved_load::decode(Kind::Regiment, &hex(case["payload"].as_str().unwrap())).unwrap()
        else {
            panic!("Regiment")
        };
        let commands = bindings(&saved);
        let reference = saved
            .construct_definition
            .as_ref()
            .unwrap()
            .as_ref()
            .map(|_| 0x4141);
        assert_eq!(
            json!(saved.construct_definition.as_ref().unwrap()),
            native["construct_definition"]
        );
        let pc = match case["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            _ => Precision::Extended64,
        };
        let (mut installed, mut arena) = construct(pc);
        let applied = regiment_load::apply_with_context(
            saved,
            &mut installed,
            &mut arena,
            &commands,
            Context {
                world_time_bits: case["world_time"].as_u64().unwrap() as u32,
                formations: Some(&[0x101, 0x202, 0x303]),
                construct_definition: reference,
            },
        )
        .unwrap();
        assert_eq!(applied.serialized_id, Some(7));
        let owned = applied.owned;
        assert_eq!(json!(owned.tracked_words), native["tracked"]);
        assert_eq!(
            json!(owned.path.nodes.iter().map(|n| n.words).collect::<Vec<_>>()),
            native["path"]
        );
        assert_eq!(owned.path.kind1_count as u32, word(&image, 0xa8));
        assert_eq!(owned.path.kind2_count as u32, word(&image, 0xac));
        assert_eq!(owned.path.word18, word(&image, 0xb4));
        assert_eq!(owned.path.dirty, image[0xb0] != 0);
        assert_eq!(json!(owned.engage_targets.nodes), native["engage"]);
        assert_eq!(json!(owned.join_words), native["join"]);
        assert_eq!(json!(owned.bombard_text), native["bombard_text"]);
        assert_eq!(
            owned.desired_formation,
            native["formation_index"]
                .as_u64()
                .map(|i| [0x101, 0x202, 0x303][i as usize])
        );
        assert_eq!(owned.construct_definition, reference);
        for (off, value) in &installed.words {
            assert_eq!(
                *value,
                word(&image, *off),
                "v{} field{off:x}",
                case["version"]
            );
        }
        for (off, value) in &installed.bytes {
            assert_eq!(*value, image[*off as usize]);
        }
        for (off, id) in &installed.state_bindings {
            assert_eq!(arena.state(*id).unwrap().status, word(&image, off + 12));
        }
        let mind = arena.mind(MindId(1)).unwrap();
        for (i, value) in mind.words_24_30.iter().enumerate() {
            assert_eq!(*value, word(&image, 0x24 + i as u32 * 4));
        }
        assert_eq!(mind.current_command, commands.current);
        assert_eq!(
            mind.pending_commands.iter().copied().collect::<Vec<_>>(),
            commands.pending
        );
        let reverse: BTreeMap<_, _> = installed
            .state_bindings
            .iter()
            .map(|(off, id)| (*id, *off))
            .collect();
        assert_eq!(
            json!(
                mind.stack
                    .iter()
                    .map(|id| reverse[&id.unwrap()])
                    .collect::<Vec<_>>()
            ),
            native["stack"]
        );
        for off in [
            0xa0, 0xa4, 0xa8, 0xac, 0xb4, 0xb8, 0x148, 0x1dc, 0x1e0, 0x268, 0x26c, 0x2bc, 0x2c0,
        ] {
            assert!(!installed.words.contains_key(&off));
        }
        for off in [0x9c, 0xe8, 0x264] {
            assert!(!installed.references.contains_key(&off));
        }
        assert!(!installed.bytes.contains_key(&0xb0));
    }
    assert_eq!(total, 243);
}

fn sparse_saved() -> regiment_load::Regiment {
    let doc = read("skirmish-20261016-regiment-load/native-regiment.json");
    let case = doc["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["version"] == 21 && c["native"]["tracked"].as_array().unwrap().is_empty())
        .unwrap();
    regiment_load::decode(&hex(case["payload"].as_str().unwrap()))
        .unwrap()
        .0
}
#[test]
fn native_formation_indices_need_real_layout_and_keep_null_lookup_semantics() {
    let doc = read("skirmish-20261016-regiment-context/native-formation-context.json");
    assert_eq!(doc["cases"].as_array().unwrap().len(), 120);
    for case in doc["cases"].as_array().unwrap() {
        let mut saved = sparse_saved();
        saved.desired_formation_index = Some(case["index"].as_i64().unwrap() as i32);
        let table: Option<Vec<u32>> = serde_json::from_value(case["formations"].clone()).unwrap();
        let commands = bindings(&saved);
        let (mut installed, mut arena) = construct(Precision::Double53);
        let before = (
            installed.words.clone(),
            installed.bytes.clone(),
            installed.references.clone(),
            arena.clone(),
        );
        let result = regiment_load::apply_with_context(
            saved,
            &mut installed,
            &mut arena,
            &commands,
            Context {
                world_time_bits: 0x41480000,
                formations: table.as_deref(),
                construct_definition: None,
            },
        );
        if case["native"]["outcome"] == "loaded" {
            let raw = case["native"]["result"].as_u64().unwrap() as u32;
            assert_eq!(
                result.unwrap().owned.desired_formation,
                (raw != 0).then_some(raw)
            );
        } else {
            assert!(result.is_err());
            assert_eq!(
                (
                    installed.words,
                    installed.bytes,
                    installed.references,
                    arena
                ),
                before
            );
        }
    }
}

#[test]
fn missing_references_and_repeat_application_never_create_empty_substitutes() {
    let original = sparse_saved();
    for invalid in 0..3 {
        let mut saved = original.clone();
        if invalid == 0 {
            saved.construct_definition = Some(Some(vec![65]));
        }
        if invalid == 1 {
            saved.bombard_text = None;
        }
        if invalid == 2 {
            saved.desired_formation_index = None;
        }
        let commands = bindings(&saved);
        let (mut installed, mut arena) = construct(Precision::Double53);
        let before = (
            installed.words.clone(),
            installed.bytes.clone(),
            installed.references.clone(),
            arena.clone(),
        );
        assert!(
            regiment_load::apply_with_context(
                saved,
                &mut installed,
                &mut arena,
                &commands,
                Context {
                    world_time_bits: 123,
                    formations: None,
                    construct_definition: None
                }
            )
            .is_err()
        );
        assert_eq!(
            (
                installed.words,
                installed.bytes,
                installed.references,
                arena
            ),
            before
        );
    }
    let commands = bindings(&original);
    let (mut installed, mut arena) = construct(Precision::Double53);
    let owned = regiment_load::apply_with_context(
        original.clone(),
        &mut installed,
        &mut arena,
        &commands,
        Context {
            world_time_bits: 123,
            formations: None,
            construct_definition: None,
        },
    )
    .unwrap()
    .owned;
    let before = (
        installed.words.clone(),
        installed.bytes.clone(),
        installed.references.clone(),
        arena.clone(),
    );
    assert!(
        regiment_load::apply_with_context(
            original,
            &mut installed,
            &mut arena,
            &commands,
            Context {
                world_time_bits: 456,
                formations: None,
                construct_definition: None
            }
        )
        .is_err()
    );
    assert_eq!(
        (
            installed.words,
            installed.bytes,
            installed.references,
            arena
        ),
        before
    );
    assert!(owned.tracked_words.is_empty());
}
