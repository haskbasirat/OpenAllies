use aa_actors::ActorId;
use aa_mapworld::{
    actor_payload::CountedList,
    kingdoms::{KingdomPayload, ReferenceWord},
    restore::{NamedDefinition, RestoreDefinitions, RuntimeDefinition},
    special_state::SpecialShell,
};
use aa_numeric::Precision;
use serde::Deserialize;
use std::collections::BTreeMap;
struct Bindings;
impl RestoreDefinitions for Bindings {
    fn resolve(&self, _: &[u16], _: Precision) -> Result<RuntimeDefinition, String> {
        Err("unused".into())
    }
    fn actor_definition_token(&self, id: &[u16], _: Precision) -> Result<u32, String> {
        let id = String::from_utf16(id).unwrap();
        Ok(if id == "missing" {
            0
        } else {
            id[1..].parse::<u32>().unwrap() + 101
        })
    }
    fn nation_badge_count(&self, _: &NamedDefinition, _: Precision) -> Result<u32, String> {
        Ok(16)
    }
    fn resolve_named_definition(
        &self,
        domain: &str,
        id: &[u16],
        _: Precision,
    ) -> Result<Option<NamedDefinition>, String> {
        assert_eq!(domain, "Resource");
        let id = String::from_utf16(id).unwrap();
        Ok(Some(NamedDefinition {
            index: Some(id[1..].parse().unwrap()),
            id,
        }))
    }
}
#[derive(Deserialize)]
struct Fixture {
    cases: Vec<Vec<Step>>,
}
#[derive(Deserialize)]
struct Step {
    version: u32,
    refs: Vec<(String, u32)>,
    indices: Vec<u32>,
    source_bytes: [u8; 3],
    resources: Vec<(String, u32)>,
    empty_named_count: u32,
    native: Native,
}
#[derive(Deserialize)]
struct Native {
    buckets: u32,
    count: u32,
    lookup: BTreeMap<String, Option<u32>>,
    indices: Vec<u32>,
    byte238: u8,
    byte239: u8,
    byte314: u8,
    resources: Vec<u32>,
}
fn refs(values: Vec<(String, u32)>) -> CountedList<ReferenceWord> {
    CountedList {
        serialized_count: values.len() as u32,
        values: values
            .into_iter()
            .map(|(id, word)| ReferenceWord {
                reference_utf16: id.encode_utf16().collect(),
                word,
            })
            .collect(),
    }
}
#[test]
fn original_kingdom_suffix_versions_duplicate_rehash_and_saved_arrays() {
    let f: Fixture = serde_json::from_str(include_str!(
        "../../../research/phase7-restoration/native-kingdom-suffix.json"
    ))
    .unwrap();
    assert_eq!(f.cases.len(), 33);
    assert_eq!(f.cases.iter().map(Vec::len).sum::<usize>(), 69);
    for sequence in f.cases {
        let mut shell = SpecialShell::kingdom(ActorId::from_raw(1), 3).unwrap();
        shell.references.insert(
            0x234,
            Some(NamedDefinition {
                id: "synthetic-nation".into(),
                index: None,
            }),
        );
        for step in sequence {
            // A minimal version0 payload gives the typed empty prefix; suffix
            // fields below come directly from the native input fixture.
            let (mut source, _) = KingdomPayload::decode_prefix(&[0; 28]).unwrap();
            source.version = step.version;
            source.named_states_count = (step.version >= 12).then_some(step.empty_named_count);
            source.references_35c = (step.version >= 15).then(|| refs(step.refs));
            source.words_36c = (step.version >= 16).then_some(CountedList {
                serialized_count: step.indices.len() as u32,
                values: step.indices,
            });
            source.byte_238 = (step.version >= 17).then_some(step.source_bytes[0]);
            source.byte_239 = (step.version >= 18).then_some(step.source_bytes[1]);
            source.byte_314 = (step.version >= 19).then_some(step.source_bytes[2]);
            source.resources_258 = (step.version >= 20).then(|| refs(step.resources));
            shell
                .load_kingdom_definition_counts(&source, &Bindings, Precision::Double53)
                .unwrap();
            shell
                .load_kingdom_final_fields(&source, &Bindings, Precision::Double53)
                .unwrap();
            let table = shell.definition_counts.as_ref().unwrap();
            assert_eq!(table.bucket_count(), step.native.buckets);
            assert_eq!(table.len(), step.native.count);
            for (key, value) in step.native.lookup {
                assert_eq!(
                    table.get(key[1..].parse::<u32>().unwrap() + 101),
                    value,
                    "{key} version{}",
                    step.version
                );
            }
            assert_eq!(shell.nation_badge_indices, step.native.indices);
            let word = shell.words[&0x238];
            assert_eq!((word & 255) as u8, step.native.byte238);
            assert_eq!(((word >> 8) & 255) as u8, step.native.byte239);
            assert_eq!(shell.bytes[&0x314], step.native.byte314);
            assert_eq!(shell.vectors[&0x258].values, step.native.resources);
        }
    }
}

#[test]
fn failed_identity_lookup_keeps_previous_insert_and_later_fields_untouched() {
    struct Reject;
    impl RestoreDefinitions for Reject {
        fn resolve(&self, _: &[u16], _: Precision) -> Result<RuntimeDefinition, String> {
            Err("unused".into())
        }
        fn actor_definition_token(&self, id: &[u16], _: Precision) -> Result<u32, String> {
            if id == [65] {
                Ok(19)
            } else {
                Err("unsupported source evaluation".into())
            }
        }
    }
    let (mut source, _) = KingdomPayload::decode_prefix(&[0; 28]).unwrap();
    source.references_35c = Some(refs(vec![
        ("A".into(), 7),
        ("B".into(), 9),
        ("A".into(), 11),
    ]));
    source.byte_238 = Some(0);
    let mut state = SpecialShell::kingdom(ActorId::from_raw(1), 3).unwrap();
    assert!(
        state
            .load_kingdom_definition_counts(&source, &Reject, Precision::Double53)
            .is_err()
    );
    let table = state.definition_counts.as_ref().unwrap();
    assert_eq!(table.len(), 1);
    assert_eq!(table.get(19), Some(7));
    assert_eq!(state.words[&0x238], 0x01010001);
}
