use aa_actors::{
    ActorId,
    components::ComponentSlot,
    loading::{LoadKind, construct_shell_with_components},
};
use aa_mapworld::{
    actor_payload::ActorPayload,
    restore::{RestoreDefinitions, RestoredComponent, RuntimeDefinition, finish_kingdom_actor},
};
use serde::Deserialize;
use std::collections::BTreeMap;
struct LiteralEmpty;
impl RestoreDefinitions for LiteralEmpty {
    fn resolve(&self, _: &[u16], _: aa_numeric::Precision) -> Result<RuntimeDefinition, String> {
        Err("not used by tail-only contract".into())
    }
    fn localize(&self, key: &[u16]) -> Result<Vec<u16>, String> {
        if key.is_empty() {
            Ok(vec![])
        } else {
            Err("fixture accepts only empty key".into())
        }
    }
}
#[derive(Deserialize)]
struct Fixture {
    cases: Vec<Case>,
}
#[derive(Deserialize)]
struct Case {
    serialized_id: u32,
    actor_hex: String,
    native: Native,
}
#[derive(Deserialize)]
struct Native {
    consumed: usize,
    fields: BTreeMap<String, u32>,
}
#[test]
fn source_kingdom_actor_tail_matches_whole_native_actor_return() {
    let f: Fixture = serde_json::from_str(include_str!(
        "../../../research/phase7-restoration/native-kingdom-actor-load.json"
    ))
    .unwrap();
    assert_eq!(f.cases.len(), 656);
    for case in f.cases {
        let bytes: Vec<u8> = case
            .actor_hex
            .as_bytes()
            .chunks_exact(2)
            .map(|v| u8::from_str_radix(std::str::from_utf8(v).unwrap(), 16).unwrap())
            .collect();
        let (input, n) = ActorPayload::decode_prefix(&bytes, 0).unwrap();
        assert_eq!(n, case.native.consumed);
        let mut base = construct_shell_with_components(LoadKind::Actor, ());
        base.thing.flags = 1 << 29;
        base.actor.as_mut().unwrap().components.insert(
            ComponentSlot::Properties,
            RestoredComponent::Properties(Box::new(
                aa_body::properties::PropertiesState::new(
                    aa_body::credit::Object(ActorId::from_raw(case.serialized_id).raw()),
                    2,
                )
                .unwrap(),
            )),
        );
        finish_kingdom_actor(&mut base, &input.tail, &LiteralEmpty).unwrap();
        let actor = base.actor.unwrap();
        for (field, expected) in case.native.fields {
            let offset = u32::from_str_radix(field.strip_prefix("word_").unwrap(), 16).unwrap();
            assert_eq!(actor.words[&offset], expected, "{field}");
        }
        assert!(actor.string_dc_utf16.is_empty());
        assert!(actor.string_e0_utf16.is_empty());
    }
}
