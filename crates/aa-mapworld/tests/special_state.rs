use aa_actors::ActorId;
use aa_mapworld::special_state::SpecialShell;
use serde::Deserialize;
use std::collections::BTreeMap;
#[derive(Deserialize)]
struct Fixture {
    cases: Vec<Case>,
}
#[derive(Deserialize)]
struct Case {
    kind: String,
    resource_count: u32,
    saved_id: u32,
    words: BTreeMap<u32, u32>,
    partial_bytes: BTreeMap<u32, u8>,
    strings: BTreeMap<u32, Vec<u16>>,
    vectors: BTreeMap<u32, Vector>,
    zero_filled_indexed: BTreeMap<u32, Vec<u32>>,
}
#[derive(Deserialize)]
struct Vector {
    count: u32,
    growth: u32,
    values: Vec<u32>,
}
#[test]
fn derived_team_kingdom_constructors_match_original() {
    let f: Fixture = serde_json::from_str(include_str!(
        "../../../research/phase7-restoration/native-special-constructors.json"
    ))
    .unwrap();
    assert_eq!(f.cases.len(), 21);
    for c in f.cases {
        let s = if c.kind == "Team" {
            SpecialShell::team().unwrap()
        } else {
            SpecialShell::kingdom(ActorId::from_raw(c.saved_id), c.resource_count).unwrap()
        };
        assert_eq!(
            s.scalar_projection(),
            c.words,
            "{} count{}",
            c.kind,
            c.resource_count
        );
        assert_eq!(s.bytes, c.partial_bytes);
        assert_eq!(s.strings_utf16, c.strings);
        assert_eq!(s.indexed_words, c.zero_filled_indexed);
        if c.kind == "Team" {
            assert_eq!(s.team_members.as_deref(), Some(&[][..]));
            assert!(!s.words.contains_key(&0x44) && !s.words.contains_key(&0x48));
        } else {
            assert!(s.team_members.is_none());
        }
        assert_eq!(s.vectors.len(), c.vectors.len());
        for (o, v) in &s.vectors {
            let expected = &c.vectors[o];
            assert_eq!(v.values, expected.values);
            assert_eq!(v.values.len(), expected.count as usize);
            assert_eq!(v.capacity_hint, expected.growth);
        }
    }
}

#[test]
fn resolved_kingdom_resource_prefix_matches_original() {
    use aa_mapworld::restore::{NamedDefinition, RestoreDefinitions, RuntimeDefinition};
    use aa_numeric::Precision;
    struct Bindings(u32);
    impl RestoreDefinitions for Bindings {
        fn resolve(&self, _: &[u16], _: Precision) -> Result<RuntimeDefinition, String> {
            Err("not part of prefix fixture".into())
        }
        fn localize(&self, _: &[u16]) -> Result<Vec<u16>, String> {
            Ok(vec![])
        }
        fn resolve_named_definition(
            &self,
            domain: &str,
            id: &[u16],
            _: Precision,
        ) -> Result<Option<NamedDefinition>, String> {
            let id = String::from_utf16(id).unwrap();
            let index = if domain == "Resource" {
                Some(id.strip_prefix('r').unwrap().parse::<u32>().unwrap())
            } else {
                assert_eq!(domain, "KingdomColor");
                None
            };
            Ok(Some(NamedDefinition { id, index }))
        }
        fn stockpile_resource_max_bits(&self, _: Precision) -> Result<u32, String> {
            Ok(self.0)
        }
    }
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/phase7-restoration/native-kingdom-prefix.json"
    ))
    .unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 60);
    for case in cases {
        let bytes: Vec<u8> = case["payload_hex"]
            .as_str()
            .unwrap()
            .as_bytes()
            .chunks_exact(2)
            .map(|v| u8::from_str_radix(std::str::from_utf8(v).unwrap(), 16).unwrap())
            .collect();
        let (source, n) = aa_mapworld::kingdoms::KingdomPayload::decode_prefix(&bytes).unwrap();
        assert_eq!(n, case["native"]["consumed"].as_u64().unwrap() as usize);
        let mut state = SpecialShell::kingdom(ActorId::from_raw(1), 3).unwrap();
        state
            .load_kingdom_prefix(
                &source,
                &Bindings(case["maximum_bits"].as_u64().unwrap() as u32),
                Precision::Double53,
            )
            .unwrap();
        assert_eq!(
            serde_json::to_value([
                state.words[&0x188],
                state.words[&0x18c],
                state.words[&0x190]
            ])
            .unwrap(),
            case["native"]["words"]
        );
        assert_eq!(
            serde_json::to_value(&state.vectors[&0x1d0].values).unwrap(),
            case["native"]["relation_values"]
        );
        assert_eq!(
            serde_json::to_value(&state.vectors[&0x1c4].values).unwrap(),
            case["native"]["resource_values"]
        );
        assert_eq!(state.references[&0x1e8].as_ref().unwrap().id, "blue");
    }
}
