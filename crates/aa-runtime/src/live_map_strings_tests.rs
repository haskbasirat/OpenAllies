use super::*;
use crate::localization::Text;
use aa_rules::{resources::Provenance, tgi::Span};
fn table() -> Localization {
    let mut table = Localization::default();
    table.declarations.insert(
        "mapkey".into(),
        vec![Text::new(
            Provenance {
                path: "explicit-native-probe".into(),
                sha256: "probe-not-asset".into(),
                span: Span { start: 0, end: 1 },
            },
            Ok("old-global".encode_utf16().collect()),
        )],
    );
    table
}
fn units(v: &serde_json::Value) -> Vec<u16> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u16)
        .collect()
}
#[test]
fn native_pair_and_same_global_entry_writes_replay() {
    let proof:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261016-local-bootstrap/begin-playing/native-map-string-set.json")).unwrap();
    assert_eq!(proof["passed"], true);
    assert_eq!(proof["cases"].as_array().unwrap().len(), 72);
    for row in proof["cases"].as_array().unwrap() {
        let mut table = table();
        let mut pair = "old-pair".encode_utf16().collect();
        let key = units(&row["key_units"]);
        let supplied = units(&row["supplied_units"]);
        let before = table.declarations["mapkey"][0].source.clone();
        set_existing_pair(&mut table, &key, &mut pair, &supplied).unwrap();
        assert_eq!(pair, units(&row["pair_units"]));
        assert_eq!(
            table.declarations["mapkey"][0].plain_utf16,
            Ok(units(&row["global_units"]))
        );
        assert_eq!(table.declarations["mapkey"][0].source, before);
        if !key.is_empty() {
            assert_eq!(row["events"][0]["pair_text_before"], row["supplied_units"]);
        }
    }
}
#[test]
fn unbound_registry_creation_or_locale_preserves_first_actual_pair_write() {
    for key in [
        vec![937],
        vec![b'M' as u16, 0],
        "missing".encode_utf16().collect(),
    ] {
        let mut table = table();
        let mut pair = vec![1];
        assert!(set_existing_pair(&mut table, &key, &mut pair, &[2, 0, 3]).is_err());
        assert_eq!(pair, [2, 0, 3]);
        assert_eq!(
            table.declarations["mapkey"][0].plain_utf16,
            Ok("old-global".encode_utf16().collect())
        );
    }
    let mut table = table();
    let duplicate = table.declarations["mapkey"][0].clone();
    table
        .declarations
        .get_mut("mapkey")
        .unwrap()
        .push(duplicate);
    let mut pair = vec![];
    assert!(
        set_existing_pair(
            &mut table,
            &"MapKey".encode_utf16().collect::<Vec<_>>(),
            &mut pair,
            &[9]
        )
        .is_err()
    );
    assert_eq!(pair, [9]);
}
