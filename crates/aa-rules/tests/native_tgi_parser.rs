//! Replays research/phase8-restoration/native-tgi-parser.json: whole-source
//! runs of the original TGC_DataFileParser (5BD080) under Unicorn.
use aa_rules::tgi::{Document, Item, Limits, Node, native_localized_text};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../../research/phase8-restoration/native-tgi-parser.json");

fn hex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}
fn fields(doc: &Document, items: &[Item]) -> Vec<Value> {
    let mut out: Vec<Value> = items
        .iter()
        .filter_map(|item| match item {
            Item::Property(p) => {
                let name = doc.property_name(p).unwrap();
                let raw = doc.property_value(p);
                let localized = raw.and_then(|raw| {
                    // Only quoted values (and typed `localized ... = null`)
                    // become native localized fields (vtable A48770).
                    let declared = p.left.len() == 2
                        && doc.text(p.left[0].span).unwrap().eq_ignore_ascii_case("localized");
                    (raw.starts_with('"') || declared)
                        .then(|| native_localized_text(raw))
                        .flatten()
                });
                Some(json!({"name": name, "localized": localized.is_some(), "value": localized}))
            }
            Item::Node(_) => None,
        })
        .collect();
    out.sort_by_key(|f| f["name"].as_str().unwrap().to_ascii_lowercase());
    out
}
fn tree(doc: &Document, node: &Node) -> Value {
    let tokens: Vec<&str> = node
        .header
        .tokens
        .iter()
        .map(|t| doc.text(t.span).unwrap())
        .collect();
    let template = tokens[0].eq_ignore_ascii_case("template");
    let name = if template { tokens[1] } else { tokens[0] };
    json!({
        "name": name,
        "template": template,
        "fields": fields(doc, &node.items),
        "children": node.items.iter().filter_map(|i| match i {
            Item::Node(n) => Some(tree(doc, n)),
            _ => None,
        }).collect::<Vec<_>>(),
    })
}

#[test]
fn original_parser_snippet_fixtures_replay() {
    let doc: Value = serde_json::from_str(FIXTURE).unwrap();
    assert_eq!(doc["passed"], true);
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 44);
    for case in cases {
        let source = hex(case["source_hex"].as_str().unwrap());
        let native = &case["native"];
        let parsed = Document::parse_native(source, Limits::default());
        if !native["error"].is_null() {
            assert!(parsed.is_err(), "{} should fail: {}", case["name"], native["error"]);
            continue;
        }
        let parsed = parsed.unwrap_or_else(|e| panic!("{}: {e}", case["name"]));
        let mut roots = Vec::new();
        for item in &parsed.items {
            if let Item::Node(node) = item {
                roots.push(tree(&parsed, node));
            }
        }
        let expected: Vec<Value> = native["tree"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| {
                let mut n = n.clone();
                normalize(&mut n);
                n
            })
            .collect();
        assert_eq!(Value::Array(roots), Value::Array(expected), "{}", case["name"]);
        // File-level properties (outside any node) of the delayed top-level case.
        let top = fields(&parsed, &parsed.items);
        if case["name"] == "delayed_top_level" {
            assert_eq!(top.len(), 1);
            assert_eq!(top[0]["name"], "c");
        }
    }
}
/// List instances are renamed through 42BC40 (`?NNNNNNN_name`, a harness
/// seam in probe_lists.py); compare the authored node name.
fn normalize(node: &mut Value) {
    if let Some(name) = node["name"].as_str()
        && name.starts_with('?')
        && name.as_bytes().get(8) == Some(&b'_')
    {
        node["name"] = Value::String(name[9..].to_owned());
    }
    for f in node["fields"].as_array_mut().unwrap() {
        if f["localized"] == false {
            f["value"] = Value::Null;
        }
    }
    for c in node["children"].as_array_mut().unwrap() {
        normalize(c);
    }
}

#[test]
fn shipped_carrier_and_trigger_elements_parse_like_native() {
    let Some(root) = std::env::var_os("AA_GAME_ROOT") else {
        return;
    };
    let data = std::path::Path::new(&root).join("reconstruction/assets/original/Data");
    for file in ["Units/USA/usa_carrier.tgi", "Game/trigger_elements.tgi"] {
        let bytes = std::fs::read(data.join(file)).unwrap();
        let doc = Document::parse_native(bytes, Limits::default())
            .unwrap_or_else(|e| panic!("{file}: {e}"));
        assert!(!doc.items.is_empty());
    }
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 assets"]
fn localized_keys_are_exposed_without_resolution() {
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
    };
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let corps = actors.resolved_actor("rus_corps").unwrap();
    let name = corps.localized("name").unwrap().unwrap();
    // v14 loose override: the authored assignment text minus its quotes.
    let file = sources.read(&name.source.path).unwrap();
    let authored = std::str::from_utf8(&file.bytes[name.source.span.start..name.source.span.end]).unwrap();
    assert!(authored.ends_with(&format!("\"{}\"", name.text)), "{authored}");
    assert!(name.text.starts_with('#') && name.quoted);
    // A non-localized field is refused, not reinterpreted.
    assert!(corps.localized("IDS").is_err());
    let carrier = actors.resolved_actor("usa_carrier").unwrap();
    assert!(carrier.localized("name").unwrap().is_some());
}
