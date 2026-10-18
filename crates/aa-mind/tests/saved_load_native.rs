//! Replays research/phase8-restoration/native-mind-load.json: original Element
//! constructor 535BA0 then saved Load 536340 over every distinct saved Mind of
//! the original maps plus synthetic branches. Rust constructs the same Mind,
//! decodes and applies, and every constructed field must equal the native
//! object image afterwards.
use aa_mind::{
    Arena, MindId, StateId,
    construct::{self, Kind, World},
    saved_load::{self, ELEMENT_STATE_LIST, Error, Saved},
};
use aa_numeric::Precision;
use serde_json::Value;
use std::collections::BTreeMap;

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
fn word(image: &[u8], offset: u32) -> u32 {
    let at = offset as usize;
    u32::from_le_bytes(image[at..at + 4].try_into().unwrap())
}
fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

#[test]
fn element_saved_load_matches_native_object_for_every_case() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/phase8-restoration/native-mind-load.json");
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(doc["passed"], true);
    let cases = doc["cases"].as_array().unwrap();
    let mut seen = BTreeMap::<String, usize>::new();
    let mut corpus = 0;
    for case in cases {
        let input = &case["input"];
        let native = &case["native"];
        let source = input["source"].as_str().unwrap();
        let payload = hex(input["payload"].as_str().unwrap());
        let pc = match input["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            _ => Precision::Extended64,
        };
        let consumed = native["consumed"].as_u64().unwrap() as usize;
        let outcome = native["outcome"].as_str().unwrap();
        *seen.entry(outcome.to_owned()).or_default() += 1;
        corpus += usize::from(input["corpus"].as_bool().unwrap());
        let decoded = saved_load::decode(Kind::Element, &payload);
        match outcome {
            "bad_version" => {
                match decoded {
                    Err(Error::BadVersion { offset, .. }) => {
                        assert_eq!(offset, consumed, "{source}")
                    }
                    other => panic!("{source}: {other:?}"),
                }
                continue;
            }
            "canceled" => {
                match decoded {
                    Err(Error::Canceled { offset, .. }) => assert_eq!(offset, consumed, "{source}"),
                    other => panic!("{source}: {other:?}"),
                }
                continue;
            }
            "short_read" => {
                assert!(matches!(decoded, Err(Error::ShortRead { .. })), "{source}");
                continue;
            }
            "loaded" => {}
            other => panic!("{source}: outcome {other}"),
        }
        let (saved, used) = decoded.unwrap_or_else(|e| panic!("{source}: {e}"));
        assert_eq!(used, consumed, "{source}");
        let Saved::Element(element) = &saved else {
            panic!("expected Element");
        };
        // Decoded path in native list order and native text.
        let path = element.path.as_ref();
        let nodes: Vec<Vec<u32>> = path
            .map(|p| p.loaded_nodes().iter().map(|n| n.words.to_vec()).collect())
            .unwrap_or_default();
        let expected_nodes: Vec<Vec<u32>> = native["path_head_first"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| {
                n.as_array()
                    .unwrap()
                    .iter()
                    .map(|w| w.as_u64().unwrap() as u32)
                    .collect()
            })
            .collect();
        assert_eq!(nodes, expected_nodes, "{source}");
        let text: Vec<u16> = native["bombard_text_units"]
            .as_array()
            .unwrap()
            .iter()
            .map(|u| u.as_u64().unwrap() as u16)
            .collect();
        assert_eq!(
            element.bombard_text.clone().unwrap_or_default(),
            text,
            "{source}"
        );

        let constructed = construct::construct(Kind::Element, 7, &mut Fixed, pc).unwrap();
        let bindings: BTreeMap<u32, StateId> = constructed
            .states
            .iter()
            .enumerate()
            .map(|(i, s)| (s.offset, StateId(100 + i as u32)))
            .collect();
        let names: BTreeMap<u32, &str> = constructed
            .states
            .iter()
            .map(|s| (s.offset, s.name))
            .collect();
        for (offset, name) in ELEMENT_STATE_LIST {
            assert_eq!(names[&offset], name, "state list name at {offset:#x}");
        }
        let mut arena = Arena::default();
        let mut installed = constructed
            .install(MindId(3), &bindings, &mut arena)
            .unwrap();
        let image = hex(native["image"].as_str().unwrap());
        let mut skipped = vec![0x1b8]; // jitter: 530E50 is seamed natively (constructor fixture)
        let applied = if !text.is_empty() {
            // Unported text owner: rejected before any mutation. The other
            // stores are still proven by applying the payload without it.
            let before = (
                installed.words.clone(),
                installed.bytes.clone(),
                arena.clone(),
            );
            let result = saved_load::apply(&saved, &mut installed, &mut arena);
            assert!(
                matches!(result, Err(Error::Unsupported(_))),
                "{source}: {result:?}"
            );
            assert_eq!(
                (
                    installed.words.clone(),
                    installed.bytes.clone(),
                    arena.clone()
                ),
                before
            );
            assert!(
                !input["corpus"].as_bool().unwrap(),
                "{source}: corpus needs this branch"
            );
            let mut stripped = element.clone();
            stripped.bombard_text = Some(Vec::new());
            Saved::Element(stripped)
        } else {
            saved.clone()
        };
        if !expected_nodes.is_empty() {
            // Native +D8/+DC hold the node pointers; Rust hands the nodes to
            // the typed Move owner instead (Applied::element_path).
            skipped.extend([0xd8, 0xdc]);
        }
        let result = saved_load::apply(&applied, &mut installed, &mut arena)
            .unwrap_or_else(|e| panic!("{source}: {e}"));
        let returned: Vec<Vec<u32>> = result
            .element_path
            .clone()
            .unwrap_or_default()
            .iter()
            .map(|n| n.words.to_vec())
            .collect();
        assert_eq!(returned, expected_nodes, "{source}");
        let id = result.serialized_id;
        for (offset, value) in &installed.words {
            if skipped.contains(offset) {
                continue;
            }
            assert_eq!(word(&image, *offset), *value, "{source}: word {offset:#x}");
        }
        for (offset, value) in &installed.bytes {
            assert_eq!(
                image[*offset as usize], *value,
                "{source}: byte {offset:#x}"
            );
        }
        for (offset, state) in &installed.state_bindings {
            let status = arena.state(*state).unwrap().status;
            assert_eq!(
                word(&image, offset + 0x0c),
                status,
                "{source}: status of {offset:#x}"
            );
        }
        let mind = arena.mind(MindId(3)).unwrap();
        for (i, value) in mind.words_24_30.iter().enumerate() {
            assert_eq!(
                word(&image, 0x24 + 4 * i as u32),
                *value,
                "{source}: mind word {i}"
            );
        }
        let reverse: BTreeMap<StateId, u32> = bindings.iter().map(|(o, s)| (*s, *o)).collect();
        let stack: Vec<u64> = mind
            .stack
            .iter()
            .map(|s| u64::from(reverse[&s.unwrap()]))
            .collect();
        let expected_stack: Vec<u64> = native["stack_offsets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap())
            .collect();
        assert_eq!(stack, expected_stack, "{source}");
        assert!(native["commands_empty"].as_bool().unwrap());
        assert!(mind.pending_commands.is_empty() && mind.current_command.is_none());
        let expected_id = element
            .base
            .as_ref()
            .and_then(|b| b.updatable.serialized_id)
            .filter(|id| *id >= 0);
        assert_eq!(id, expected_id, "{source}");
        assert_eq!(installed.update_head, None);
    }
    assert!(corpus >= 2000, "{corpus}");
    assert_eq!(seen.get("canceled"), Some(&5));
    assert_eq!(seen.get("bad_version"), Some(&5));
    println!("{seen:?} corpus {corpus}");
}
