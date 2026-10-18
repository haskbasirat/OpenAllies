use aa_rules::tgi::Limits;
use aa_runtime::{film_resource::Resource, localization::Localization};
use serde_json::{Value, json};

fn table() -> Localization {
    //Explicit parsed native TextData fixture source, not a gameplay default.
    let bytes=b"[Text language=Default]\n{\nids=map_strings_table\nname=\"Map\"\na=\"Alpha\"\nb=\"Beta\"\n}\n".to_vec();
    let file = aa_rules::sources::ResolvedFile {
        logical_path: "explicit/native-text.tgi".into(),
        origin: aa_rules::sources::Origin::V14Loose,
        sha256: aa_rules::sources::sha256(&bytes),
        bytes,
    };
    Localization::from_files([file], Limits::default()).unwrap()
}
#[test]
fn original_existing_entries_nil_duplicates_and_missing_prefix() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-film/native-film-references.json"
    ))
    .unwrap();
    let rows = fixture["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 21);
    for row in rows {
        let text = table();
        let mut resource = Resource::open_memory("explicit-fixture.RFM");
        let mut steps = Vec::new();
        let mut stopped = None;
        let snapshot = |resource: &Resource| {
            let references = resource.map_string_references().unwrap();
            let head: Vec<_> = references
                .iter()
                .map(|r| r.key().unwrap_or("").to_owned())
                .collect();
            let deltas = ["a", "b", ""]
                .into_iter()
                .map(|k| {
                    (
                        k.to_owned(),
                        json!(head.iter().filter(|v| v.as_str() == k).count()),
                    )
                })
                .collect::<serde_json::Map<_, _>>();
            json!({"head_first":head,"reference_deltas":deltas})
        };
        for key in row["input"]["keys"].as_array().unwrap() {
            let key = key.as_str().unwrap().encode_utf16().collect::<Vec<_>>();
            if resource
                .retain_existing_localized_pair(&text, &key, Some(&[65, 66, 67]))
                .is_err()
            {
                stopped = Some("required_warning_create");
                break;
            }
            resource.validate_map_string_references(&text).unwrap();
            steps.push(snapshot(&resource));
        }
        assert_eq!(
            json!({"steps":steps,"stopped":stopped,"final":snapshot(&resource)}),
            row["output"]
        );
    }
}

#[test]
fn same_current_entry_mutation_and_replaced_or_cloned_owner_refusal() {
    let mut text = table();
    let key = "a".encode_utf16().collect::<Vec<_>>();
    let retained = text.reference_existing(&key).unwrap().unwrap();
    assert!(retained.same_entry(&text.reference_existing(&key).unwrap().unwrap()));
    text.declarations.get_mut("a").unwrap()[0].plain_utf16 = Ok(vec![90]);
    text.validate_reference(&retained).unwrap();
    assert!(text.clone().validate_reference(&retained).is_err());
    let replacement = text.declarations["a"][0].clone();
    text.declarations.get_mut("a").unwrap()[0] = replacement;
    assert!(text.validate_reference(&retained).is_err());
    let current = text.reference_existing(&key).unwrap().unwrap();
    text.declarations.get_mut("a").unwrap()[0]
        .source
        .sha256
        .push('x');
    assert!(text.validate_reference(&current).is_err());
    drop(text);
    assert!(table().validate_reference(&current).is_err());
}
