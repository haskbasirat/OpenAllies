//! Original526260 ->528C20 fixture replay. All supported decoder versions and
//! container ordering are compared against x86 output, including later versions
//! whose application still requires real runtime owners.
use aa_mind::{
    Arena, MindId, StateId,
    construct::{self, Kind, World},
    regiment_load,
    saved_load::{self, CommandBindings, Error, Saved, Store},
};
use aa_numeric::Precision;
use serde_json::{Value, json};
use std::collections::BTreeMap;
fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}
fn word(image: &[u8], at: u32) -> u32 {
    u32::from_le_bytes(image[at as usize..at as usize + 4].try_into().unwrap())
}
struct NoCalls;
impl World for NoCalls {
    fn random_word(&mut self) -> aa_mind::Result<u32> {
        panic!("Regiment must not draw RNG")
    }
    fn performer_ability_count(&self, _: u32) -> aa_mind::Result<usize> {
        panic!("Regiment must not query abilities")
    }
    fn set_ability_jitter(&mut self, _: u32, _: usize, _: u32) -> aa_mind::Result<()> {
        panic!("Regiment must not change abilities")
    }
}
fn fixture() -> Value {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-regiment-load/native-regiment.json");
    serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap()
}
#[test]
fn all_regiment_saved_versions_match_original_x86() {
    let doc = fixture();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 266);
    for case in cases {
        let payload = hex(case["payload"].as_str().unwrap());
        let native = &case["native"];
        let got = saved_load::decode(Kind::Regiment, &payload);
        if native["outcome"] == "bad_version" {
            assert!(matches!(
                got,
                Err(Error::BadVersion {
                    max: 21,
                    offset: 4,
                    ..
                })
            ));
            continue;
        }
        let (saved, used) = got.unwrap();
        assert_eq!(used, payload.len());
        assert_eq!(used as u64, native["consumed"].as_u64().unwrap());
        let Saved::Regiment(r) = &saved else {
            panic!("expected Regiment")
        };
        let image = hex(native["image"].as_str().unwrap());
        let mut final_words = BTreeMap::new();
        let mut final_bytes = BTreeMap::new();
        for store in &r.stores {
            match *store {
                Store::Word { offset, value } => {
                    final_words.insert(offset, value);
                }
                Store::Byte { offset, value } => {
                    final_bytes.insert(offset, value);
                }
            }
        }
        for (off, val) in final_words {
            assert_eq!(val, word(&image, off), "version{} field{off:x}", r.version);
        }
        for (off, val) in final_bytes {
            assert_eq!(val, image[off as usize], "version{} byte{off:x}", r.version);
        }
        assert_eq!(
            json!(r.tracked_words.clone().unwrap_or_default()),
            native["tracked"]
        );
        assert_eq!(
            json!(r.join_words.clone().unwrap_or_default()),
            native["join"]
        );
        assert_eq!(
            json!(r.bombard_text.clone().unwrap_or_default()),
            native["bombard_text"]
        );
        if let Some(def) = &r.construct_definition {
            assert_eq!(json!(def), native["construct_definition"]);
        }
        if let Some(index) = r.desired_formation_index {
            assert_eq!(
                json!((index >= 0).then_some(index)),
                native["formation_index"]
            );
        }
        let nodes = r
            .path
            .as_ref()
            .map(|p| {
                p.loaded_nodes()
                    .into_iter()
                    .map(|n| n.words)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        assert_eq!(json!(nodes), native["path"]);
        if let Some(path) = &r.path {
            let (a, b) = path.kind_counts();
            assert_eq!(a as u32, word(&image, 0xa8));
            assert_eq!(b as u32, word(&image, 0xac));
            assert_eq!(path.word_18, word(&image, 0xb4));
            assert_eq!(image[0xb0], 1);
        }
        let targets = r.engage_targets().unwrap_or_default();
        assert_eq!(json!(targets.nodes), native["engage"]);
        if r.needs_world_time() {
            assert_eq!(
                word(&image, 0xbc) as u64,
                case["world_time"].as_u64().unwrap()
            );
        }
        if let Some(base) = saved.base() {
            assert_eq!(base.word_24, word(&image, 0x24));
            assert_eq!(base.word_28, word(&image, 0x28));
            if let Some(v) = base.word_2c {
                assert_eq!(v, word(&image, 0x2c));
            }
            assert_eq!(
                json!(base.stack.clone().unwrap_or_default()),
                native["stack"]
            );
            assert_eq!(
                base.commands.len(),
                native["pending"].as_array().unwrap().len()
            );
            assert_eq!(base.current_command.is_some(), !native["current"].is_null());
        }
        let pc = match case["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            _ => Precision::Extended64,
        };
        let constructor = construct::construct(Kind::Regiment, 7, &mut NoCalls, pc).unwrap();
        assert_eq!(
            constructor
                .states
                .iter()
                .map(|s| (s.offset, s.name))
                .collect::<Vec<_>>(),
            regiment_load::STATE_LIST
        );
        let bindings = constructor
            .states
            .iter()
            .enumerate()
            .map(|(i, s)| (s.offset, StateId(100 + i as u32)))
            .collect();
        let mut arena = Arena::default();
        let mut installed = constructor
            .install(MindId(1), &bindings, &mut arena)
            .unwrap();
        let base = saved.base();
        let count = base.map_or(0, |b| b.commands.len());
        let commands = CommandBindings {
            pending: (1..=count as u32).collect(),
            current: base
                .and_then(|b| b.current_command.as_ref())
                .map(|_| count as u32 + 1),
        };
        let before = (
            installed.words.clone(),
            installed.bytes.clone(),
            arena.clone(),
        );
        let applied =
            saved_load::apply_with_commands(&saved, &mut installed, &mut arena, &commands);
        if r.version >= 2 {
            assert!(matches!(applied, Err(Error::Unsupported(_))));
            assert_eq!((installed.words, installed.bytes, arena), before);
            continue;
        }
        let applied = applied.unwrap();
        assert!(applied.element_path.is_none());
        assert_eq!(
            applied.serialized_id,
            base.and_then(|b| b.updatable.serialized_id)
        );
        for (off, val) in &installed.words {
            assert_eq!(
                *val,
                word(&image, *off),
                "version{} installed{off:x}",
                r.version
            );
        }
        for (off, val) in &installed.bytes {
            assert_eq!(*val, image[*off as usize]);
        }
        let mind = arena.mind(MindId(1)).unwrap();
        assert_eq!(mind.current_command, commands.current);
        assert_eq!(
            mind.pending_commands.iter().copied().collect::<Vec<_>>(),
            commands.pending
        );
        for (i, val) in mind.words_24_30.iter().enumerate() {
            assert_eq!(*val, word(&image, 0x24 + i as u32 * 4));
        }
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
        for (off, id) in &installed.state_bindings {
            assert_eq!(arena.state(*id).unwrap().status, word(&image, off + 12));
        }
    }
}
#[test]
fn regiment_truncation_never_invents_fields_or_advances_to_unknown_suffix() {
    let doc = fixture();
    let case = doc["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["version"] == 21 && c["native"]["join"].as_array().unwrap().len() == 4)
        .unwrap();
    let payload = hex(case["payload"].as_str().unwrap());
    for end in 0..payload.len() {
        assert!(
            matches!(
                regiment_load::decode(&payload[..end]),
                Err(Error::ShortRead { .. })
            ),
            "prefix{end}"
        );
    }
}
