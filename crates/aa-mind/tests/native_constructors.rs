use aa_mind::{
    Error, MindId, StateId,
    construct::{self, Kind, Reference, World},
};
use aa_numeric::Precision;
use serde_json::Value;
use std::collections::BTreeMap;

struct FixtureWorld {
    seed: u32,
    abilities: Vec<u32>,
    draws: usize,
    writes: usize,
    fail_at: Option<usize>,
}
impl World for FixtureWorld {
    fn random_word(&mut self) -> aa_mind::Result<u32> {
        let wide = u64::from(self.seed) * 0x41c64e6d + 0x3039;
        self.seed = wide as u32;
        self.draws += 1;
        Ok((wide >> 16) as u32)
    }
    fn performer_ability_count(&self, _: u32) -> aa_mind::Result<usize> {
        Ok(self.abilities.len())
    }
    fn set_ability_jitter(&mut self, _: u32, index: usize, bits: u32) -> aa_mind::Result<()> {
        if self.fail_at == Some(index) {
            return Err(Error::World("fixture write failure".into()));
        }
        self.abilities[index] = bits;
        self.writes += 1;
        Ok(())
    }
}
fn kind(s: &str) -> Kind {
    match s {
        "base" => Kind::Base,
        "element" => Kind::Element,
        "regiment" => Kind::Regiment,
        "aircraft" => Kind::Aircraft,
        _ => panic!("kind"),
    }
}
fn num(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn hex(s: &str) -> u32 {
    u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap()
}
fn put(bytes: &mut [u8], at: usize, word: u32) {
    bytes[at..at + 4].copy_from_slice(&word.to_le_bytes());
}
#[test]
fn every_initialized_native_constructor_field_and_rng_effect_matches() {
    let corpus: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-mind/native-constructors.json"
    ))
    .unwrap();
    let metadata: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-mind/native-metadata.json"
    ))
    .unwrap();
    let actor = hex(corpus["addresses"]["actor"].as_str().unwrap());
    let this = hex(corpus["addresses"]["this"].as_str().unwrap());
    let empty = hex(corpus["addresses"]["empty_string"].as_str().unwrap());
    for case in corpus["cases"].as_array().unwrap() {
        let input = &case["input"];
        let expected = &case["output"];
        let kind_name = input["kind"].as_str().unwrap();
        let kind = kind(kind_name);
        let pc = match input["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            _ => Precision::Extended64,
        };
        let mut world = FixtureWorld {
            seed: num(&input["seed"]),
            abilities: vec![0x12345678; num(&input["abilities"]) as usize],
            draws: 0,
            writes: 0,
            fail_at: None,
        };
        let result = construct::construct(kind, actor, &mut world, pc).unwrap();
        assert_eq!(world.seed, num(&expected["rng_state"]));
        assert_eq!(
            world.abilities,
            expected["ability_words"]
                .as_array()
                .unwrap()
                .iter()
                .map(num)
                .collect::<Vec<_>>()
        );
        assert_eq!(world.draws, usize::from(kind == Kind::Element));
        assert_eq!(
            world.writes,
            if kind == Kind::Element {
                world.abilities.len()
            } else {
                0
            }
        );
        assert_eq!(result.core.actor, actor);
        assert!(result.core.stack.is_empty());
        assert!(result.core.pending_commands.is_empty());
        assert!(result.core.current_command.is_none());
        assert_eq!(result.core.words_24_30, [0; 4]);
        assert!(result.update_head.is_none());
        let source = expected["initialized_hex"].as_str().unwrap();
        let bytes: Vec<u8> = (0..source.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&source[i..i + 2], 16).unwrap())
            .collect();
        let mut actual = vec![num(&input["fill"]) as u8; bytes.len()];
        // Native dispatch identity is intentionally represented by Kind; pointers here
        // are reconstructed only for fixture comparison, never used as runtime IDs.
        let (vt, updatable) = match kind {
            Kind::Base => (0xa34418, 0xa343f0),
            Kind::Element => (0xa35810, 0xa357e8),
            Kind::Regiment => (0xa335a0, 0xa33578),
            Kind::Aircraft => (0xa32710, 0xa326e4),
        };
        put(&mut actual, 0, vt);
        put(&mut actual, 4, actor);
        put(&mut actual, 8, updatable);
        for off in (12..0x34).step_by(4) {
            put(&mut actual, off, 0);
        }
        for (&off, &word) in &result.words {
            put(&mut actual, off as usize, word);
        }
        for (&off, &byte) in &result.bytes {
            actual[off as usize] = byte;
        }
        for (&off, reference) in &result.references {
            let value = match reference {
                Reference::EmptyUtf16 => empty,
                Reference::EmptyWordVector { initial_capacity } => {
                    assert_eq!(*initial_capacity, 1);
                    0x4400000
                }
                Reference::NativeSubobject { vtable } => *vtable,
            };
            put(&mut actual, off as usize, value);
        }
        let layout = metadata["state_layouts"][kind_name].as_array().unwrap();
        assert_eq!(result.states.len(), layout.len());
        let bindings: BTreeMap<_, _> = result
            .states
            .iter()
            .enumerate()
            .map(|(i, s)| (s.offset, StateId(0x800 + i as u32)))
            .collect();
        let bound = result.bind_states(MindId(0x999), &bindings).unwrap();
        for (state, record) in result.states.iter().zip(layout) {
            assert_eq!(state.offset, num(&record["offset"]));
            assert_eq!(
                state.native_vtable,
                hex(record["vtable_va"].as_str().unwrap())
            );
            assert_eq!(
                state.native_metadata,
                hex(record["metadata_va"].as_str().unwrap())
            );
            assert_eq!(state.initial_status, num(&record["status"]));
            let native = metadata["metadata"]
                .as_array()
                .unwrap()
                .iter()
                .find(|m| hex(m["metadata_va"].as_str().unwrap()) == state.native_metadata)
                .unwrap();
            assert_eq!(state.name, native["name"].as_str().unwrap());
            assert_eq!(
                state.discard_on_suspend,
                native["discard_on_suspend"].as_bool().unwrap()
            );
            let stored = &bound[&bindings[&state.offset]];
            assert_eq!(stored.parent, MindId(0x999));
            assert_eq!(stored.status, state.initial_status);
            assert_eq!(stored.discard_on_suspend, state.discard_on_suspend);
            put(&mut actual, state.offset as usize, state.native_vtable);
            put(&mut actual, state.offset as usize + 4, this);
            put(
                &mut actual,
                state.offset as usize + 8,
                state.native_metadata,
            );
            put(
                &mut actual,
                state.offset as usize + 12,
                state.initial_status,
            );
        }
        assert_eq!(actual, bytes, "{}", input);
    }
}
#[test]
fn bindings_and_partial_callback_failure_are_explicit() {
    let mut world = FixtureWorld {
        seed: 1,
        abilities: vec![0x12345678; 3],
        draws: 0,
        writes: 0,
        fail_at: Some(1),
    };
    assert!(construct::construct(Kind::Element, 7, &mut world, Precision::Extended64).is_err());
    assert_eq!(world.draws, 1);
    assert_eq!(world.writes, 1);
    assert_ne!(world.abilities[0], 0x12345678);
    assert_eq!(world.abilities[1..], [0x12345678; 2]);
    world.fail_at = None;
    let model = construct::construct(Kind::Regiment, 7, &mut world, Precision::Extended64).unwrap();
    assert!(model.bind_states(MindId(1), &BTreeMap::new()).is_err());
    let duplicates = model
        .states
        .iter()
        .map(|s| (s.offset, StateId(1)))
        .collect();
    assert!(model.bind_states(MindId(1), &duplicates).is_err());
    assert!(model.word(0x45).is_err());
}

#[test]
fn publication_moves_core_and_states_to_unique_arena_without_constructor_effects() {
    let mut world = FixtureWorld {
        seed: 7,
        abilities: vec![0x12345678],
        draws: 0,
        writes: 0,
        fail_at: None,
    };
    let mut model =
        construct::construct(Kind::Element, 55, &mut world, Precision::Double53).unwrap();
    model.core.pending_commands.push_back(777);
    let bindings = model
        .states
        .iter()
        .enumerate()
        .map(|(i, s)| (s.offset, StateId(100 + i as u32)))
        .collect();
    let expected_words = model.words.clone();
    let mut arena = aa_mind::Arena::default();
    let installed = model.install(MindId(50), &bindings, &mut arena).unwrap();
    assert_eq!(world.draws, 1);
    assert_eq!(world.writes, 1);
    assert_eq!(arena.minds[&MindId(50)].pending_commands, [777]);
    assert_eq!(installed.words, expected_words);
    assert_eq!(arena.states.len(), 11);
    assert_eq!(installed.state_bindings, bindings);
    let duplicate =
        construct::construct(Kind::Regiment, 55, &mut world, Precision::Double53).unwrap();
    let before = arena.clone();
    assert!(
        duplicate
            .install(MindId(50), &BTreeMap::new(), &mut arena)
            .is_err()
    );
    assert_eq!(arena, before);
}
