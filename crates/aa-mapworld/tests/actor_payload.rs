use aa_mapworld::actor_payload::{ActorPayload, ActorTail, PropertiesPayload, UpdatablePayload};
use serde_json::Value;

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase6-actor-payloads/native-actor-payload-fixtures.json"
    ))
    .unwrap()
}
fn hex(text: &str) -> Vec<u8> {
    text.as_bytes()
        .chunks_exact(2)
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}
#[test]
fn original_actor_tail_vectors() {
    let source = fixture();
    let cases = source["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 112);
    for case in cases {
        let raw = hex(case["tail_hex"].as_str().unwrap());
        let version = case["version"].as_u64().unwrap() as u32;
        let (tail, consumed) = ActorTail::decode_prefix(&raw, version, 0).unwrap();
        assert_eq!(
            consumed,
            case["native"]["consumed"].as_u64().unwrap() as usize
        );
        assert_eq!(consumed, raw.len());
        let actor_bytes = hex(case["actor_hex"].as_str().unwrap());
        let (actor, actor_consumed) = ActorPayload::decode_prefix(&actor_bytes, 0).unwrap();
        assert_eq!(actor.tail, tail);
        assert_eq!(
            actor_consumed,
            case["native_actor"]["consumed"].as_u64().unwrap() as usize
        );
        assert_eq!(actor_consumed, actor_bytes.len());
        assert_eq!(
            serde_json::to_value(actor.prefix.state_after_read([0, 0])).unwrap(),
            case["native_actor"]["state"]
        );
        let decoded = serde_json::to_value(&tail).unwrap();
        let expected = &case["expected"];
        for key in [
            "saved_components_mask",
            "default_components_mask",
            "current_component",
            "word_158",
            "legacy_word_15c",
            "word_160_source",
            "word_164",
            "localized_name_key_utf16",
            "word_168",
            "word_16c",
            "word_170",
            "lists",
            "word_174",
        ] {
            assert_eq!(
                decoded[key], expected[key],
                "version {version}, sample {}, field {key}",
                case["sample"]
            );
        }
        if let Some(properties) = &tail.properties {
            assert_eq!(
                properties.updatable.version,
                expected["updatable_version"].as_u64().unwrap() as u32
            );
            assert_eq!(
                properties.version,
                expected["properties_version"].as_u64().unwrap() as u32
            );
            assert_eq!(
                properties.property_classes.len(),
                expected["properties_count"].as_u64().unwrap() as usize
            );
            assert_eq!(
                serde_json::to_value(properties.updatable.serialized_id).unwrap(),
                expected["updatable_id"]
            );
            assert!(properties.property_classes.iter().all(|&v| v == u32::MAX));
        } else {
            assert!(expected["properties_count"].is_null());
        }
        for key in ["word_158", "word_168", "word_16c", "word_170", "word_174"] {
            if !decoded[key].is_null() {
                assert_eq!(decoded[key], case["native"]["fields"][key]);
            }
        }
        if let Some(lists) = &tail.lists {
            let native = &case["native"]["lists"];
            assert_eq!(
                serde_json::to_value(&lists.list_118.values).unwrap(),
                native[0]
            );
            assert_eq!(
                serde_json::to_value(&lists.list_120.values).unwrap(),
                native[1]
            );
            assert_eq!(
                serde_json::to_value(&lists.list_128.values).unwrap(),
                native[2]
            );
            assert_eq!(
                serde_json::to_value(&lists.list_130.values).unwrap(),
                native[3]
            );
        }
        // Every proper prefix truncation must fail, even where optional fields
        // or variable lists sit immediately before another record.
        for end in 0..raw.len() {
            assert!(ActorTail::decode_prefix(&raw[..end], version, 0).is_err());
        }
        let mut following = raw.clone();
        following.extend_from_slice(b"NEXT RECORD");
        assert_eq!(
            ActorTail::decode_prefix(&following, version, 0).unwrap().1,
            raw.len()
        );
    }
}

#[test]
fn original_crete_actor_boundary() {
    let source = fixture();
    let sample = &source["source_crete"];
    let mut bytes = hex(sample["actor_hex"].as_str().unwrap());
    bytes.extend_from_slice(&1u32.to_le_bytes());
    let (actor, consumed) = ActorPayload::decode_prefix(&bytes, 0).unwrap();
    assert_eq!(consumed, 84);
    assert_eq!(actor.prefix.version, 14);
    assert_eq!(actor.tail.saved_components_mask, 0x40);
    assert_eq!(actor.tail.current_component, -1);
    let properties = actor.tail.properties.unwrap();
    assert_eq!(
        properties.updatable,
        UpdatablePayload {
            version: 1,
            serialized_id: Some(0)
        }
    );
    assert_eq!(properties.version, 1);
    assert!(properties.property_classes.is_empty());
    assert_eq!(&bytes[consumed..], &1u32.to_le_bytes());
}

#[test]
fn original_dunkirk_uses_default_mask_without_saved_properties() {
    let source = fixture();
    let sample = &source["source_dunkirk"];
    let bytes = hex(sample["actor_hex"].as_str().unwrap());
    let (actor, consumed) = ActorPayload::decode_prefix(&bytes, 0).unwrap();
    assert_eq!(
        consumed,
        sample["native_actor"]["consumed"].as_u64().unwrap() as usize
    );
    assert_eq!(consumed, 68);
    assert_eq!(actor.tail.default_components_mask, 0x40);
    assert_eq!(actor.tail.saved_components_mask, 0);
    assert!(actor.tail.properties.is_none());
    assert_eq!(actor.tail.current_component, -1);
}

#[test]
fn explicit_unsupported_payloads_and_bounded_counts() {
    let source = fixture();
    let mut bytes = hex(source["cases"][0]["tail_hex"].as_str().unwrap());
    bytes[4..8].copy_from_slice(&1u32.to_le_bytes());
    assert!(
        ActorTail::decode_prefix(&bytes, 1, 0)
            .unwrap_err()
            .to_string()
            .contains("components")
    );
    assert!(ActorTail::decode_prefix(&bytes, 0, 0).is_err());
    assert!(ActorTail::decode_prefix(&bytes, 15, 0).is_err());
    assert!(ActorTail::decode_prefix(&bytes, 1, 1).is_err());
    let properties: Vec<u8> = [1u32, u32::MAX, 1, 1, 0]
        .iter()
        .flat_map(|x| x.to_le_bytes())
        .collect();
    assert!(
        PropertiesPayload::decode_prefix(&properties)
            .unwrap_err()
            .to_string()
            .contains("class")
    );
    let mut oversized = properties;
    oversized[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(
        PropertiesPayload::decode_prefix(&oversized)
            .unwrap_err()
            .to_string()
            .contains("count")
    );
    assert_eq!(
        UpdatablePayload::decode_prefix(&[0, 0, 0, 0]).unwrap(),
        (
            UpdatablePayload {
                version: 0,
                serialized_id: None
            },
            4
        )
    );
    assert!(UpdatablePayload::decode_prefix(&[1, 0, 0, 0]).is_err());
}
