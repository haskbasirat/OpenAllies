//! Replays research/phase8-triggers/native-triggers.json: the ORIGINAL
//! TriggerManager (ctor 54CEE0, load 54D3D0 -> 541410 / 5468A0 / 544190 /
//! 54A640 / 49AD70) reading the TRIGGERS chunk of every original map after the
//! original MAPSTRINGS loader filled the KKC_TextDataManager. The Rust decoder
//! plus `MapStrings::bind` must reproduce every loaded native field.
mod sha;
use aa_triggers::{
    decode::{Element, Localization, SavedManager, Value},
    mapstrings::MapStrings,
};
use serde_json::Value as J;
use std::path::Path;

fn utf16(s: &J) -> Vec<u16> {
    s.as_str().unwrap().encode_utf16().collect()
}

fn element(map: &str, rust: &Element, native: &J) {
    assert_eq!(rust.opcode as u64, native["opcode"].as_u64().unwrap(), "{map} {}", rust.name);
    assert_eq!(rust.word_10 as u64, native["word_10"].as_u64().unwrap(), "{map} {}", rust.name);
    let params = native["parameters"].as_array().unwrap();
    assert_eq!(rust.parameters.len(), params.len(), "{map} {}", rust.name);
    for (p, n) in rust.parameters.iter().zip(params) {
        let at = format!("{map} {} {}", rust.name, p.argument);
        assert_eq!(p.storage as u64, n["storage"].as_u64().unwrap(), "{at}");
        assert_eq!(
            aa_triggers::argument_index(&p.argument).map(u64::from),
            n["argument_index"].as_u64(),
            "{at}"
        );
        assert_eq!(p.item_index as u64, n["item_index"].as_u64().unwrap(), "{at}");
        assert_eq!(p.flag as u64, n["flag"].as_u64().unwrap(), "{at}");
        match &p.value {
            Value::Empty => assert!(n.get("word").is_none() && n.get("text").is_none(), "{at}"),
            Value::Word(w) => assert_eq!(*w as u64, n["word"].as_u64().unwrap(), "{at}"),
            Value::Position(xyz) => {
                let native: Vec<u64> = n["position"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap()).collect();
                assert_eq!(xyz.iter().map(|&v| v as u64).collect::<Vec<_>>(), native, "{at}");
            }
            Value::Text(t) => assert_eq!(t, &utf16(&n["text"]), "{at}"),
            Value::Localized { key, inline } => {
                assert_eq!(key, &utf16(&n["key"]), "{at}");
                assert_eq!(inline.as_ref().unwrap(), &utf16(&n["localized"]), "{at}");
            }
        }
    }
}

#[test]
fn every_original_map_matches_the_native_trigger_loader() {
    let research = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/phase8-triggers");
    let doc: J = serde_json::from_slice(&std::fs::read(research.join("native-triggers.json")).unwrap()).unwrap();
    let chunks = research.join("chunks");
    let index: J = serde_json::from_slice(&std::fs::read(chunks.join("index.json")).unwrap()).unwrap();
    let (mut maps, mut v4, mut elements, mut localized) = (0, 0, 0, 0);
    for case in doc["cases"].as_array().unwrap() {
        let map = case["map"].as_str().unwrap();
        assert!(case.get("error").is_none(), "{map}: native load threw {}", case["error"]);
        let entry = index.as_array().unwrap().iter().find(|e| e["map"] == map).unwrap();
        let bytes = std::fs::read(chunks.join(entry["file"].as_str().unwrap())).unwrap();
        assert_eq!(sha::hex(&bytes), case["chunk_sha256"].as_str().unwrap());
        let text = std::fs::read(chunks.join(entry["mapstrings_file"].as_str().unwrap())).unwrap();
        assert_eq!(sha::hex(&text), case["mapstrings_sha256"].as_str().unwrap());
        assert_eq!(case["consumed"].as_u64().unwrap(), bytes.len() as u64, "{map}");
        let mut saved = SavedManager::decode(&bytes, Localization::External, bytes.len())
            .unwrap_or_else(|e| panic!("{map}: {e}"));
        let strings = MapStrings::parse(&text, "English").unwrap();
        localized += strings.bind(&mut saved).unwrap();
        assert_eq!(saved.next_id as u64, case["next_id"].as_u64().unwrap(), "{map}");
        let triggers = case["triggers"].as_array().unwrap();
        assert_eq!(saved.triggers.len(), triggers.len(), "{map}");
        for (t, n) in saved.triggers.iter().zip(triggers) {
            assert_eq!(t.id as u64, n["id"].as_u64().unwrap(), "{map}");
            assert_eq!(t.mode as u64, n["mode"].as_u64().unwrap(), "{map} {}", t.id);
            assert_eq!(t.enabled as u64, n["enabled"].as_u64().unwrap(), "{map} {}", t.id);
            assert_eq!(t.repeat as u64, n["repeat"].as_u64().unwrap(), "{map} {}", t.id);
            assert_eq!(t.activation_bits as u64, n["activation_bits"].as_u64().unwrap(), "{map} {}", t.id);
            assert_eq!(t.name, utf16(&n["name"]), "{map} {}", t.id);
            assert_eq!(t.description, utf16(&n["description"]), "{map} {}", t.id);
            assert_eq!(t.word_2c as u64, n["word_2c"].as_u64().unwrap(), "{map} {}", t.id);
            for (label, list) in [("events", &t.events), ("conditions", &t.conditions), ("actions", &t.actions)] {
                let native = n[label].as_array().unwrap();
                assert_eq!(list.len(), native.len(), "{map} {} {label}", t.id);
                for (e, ne) in list.iter().zip(native) {
                    element(map, e, ne);
                    elements += 1;
                }
            }
        }
        let mut vars: Vec<(Vec<u16>, u32)> = saved.variables.iter().map(|v| (v.name.clone(), v.bits)).collect();
        vars.sort();
        let mut native: Vec<(Vec<u16>, u32)> = case["variables"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| (utf16(&v["name"]), v["bits"].as_u64().unwrap() as u32))
            .collect();
        native.sort();
        assert_eq!(vars, native, "{map}");
        let native: Vec<Vec<u16>> = case["strings"].as_array().unwrap().iter().map(utf16).collect();
        assert_eq!(saved.strings, native, "{map}");
        // After loading, 54D3D0 broadcasts "TriggerListUpdate" (50D120); the
        // runtime owns that UI message.
        assert_eq!(case["broadcasts"], serde_json::json!(["TriggerListUpdate"]), "{map}");
        maps += 1;
        if entry["manager_version"] == 4 {
            v4 += 1;
        }
    }
    println!("{maps} maps ({v4} manager v4), {elements} elements, {localized} localized parameters bound");
    assert_eq!(maps, 80);
    assert_eq!(v4, 32);
}
