//! Replays research/phase8-definitions/native-definition-registry.json: each
//! case was produced by executing original AA.exe code under Unicorn
//! (native_definition_registry.py). The Rust ports must reproduce every case.
use aa_numeric::Precision;
use aa_rules::{
    actors::native_load::{
        NativeLoadEventKind, NativeLoadReport, NativeThingRegistry, NativeTranslateError,
        load_files,
    },
    native::index::{native_enumerate, native_resource_matches, native_wildcard_match},
    sources::{Origin, ResolvedFile, sha256},
    tgi::Limits,
};
use serde_json::Value;
use std::path::PathBuf;

fn fixture() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/phase8-definitions/native-definition-registry.json");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn section<'a>(f: &'a Value, name: &str) -> &'a Vec<Value> {
    f["sections"][name]["cases"].as_array().unwrap()
}
fn u16s(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}
fn file(path: &str, text: &str) -> ResolvedFile {
    ResolvedFile {
        logical_path: path.into(),
        origin: Origin::V14Loose,
        sha256: sha256(text.as_bytes()),
        bytes: text.as_bytes().to_vec(),
    }
}
fn thing(ids: &str) -> String {
    format!("[Thing]\r\n{{\r\n\tIDS = {ids}\r\n\tcategory = THING\r\n}}\r\n")
}

#[test]
fn wildcard_matcher_matches_859830() {
    let f = fixture();
    let cases = section(&f, "wildcard");
    assert!(cases.len() >= 400);
    for c in cases {
        let (p, s) = (c["pattern"].as_str().unwrap(), c["path"].as_str().unwrap());
        assert_eq!(
            native_wildcard_match(&u16s(p), &u16s(s)),
            c["match"].as_bool().unwrap(),
            "{p} vs {s}"
        );
    }
}

#[test]
fn resource_enumeration_matches_5aef60_segment_rule() {
    let f = fixture();
    for c in section(&f, "find") {
        let (p, s) = (c["pattern"].as_str().unwrap(), c["path"].as_str().unwrap());
        assert_eq!(
            native_resource_matches(&u16s(p), &u16s(s)),
            c["match"].as_bool().unwrap(),
            "{p} vs {s}"
        );
    }
}

#[test]
fn enumeration_order_matches_original_qsort() {
    let f = fixture();
    let cases = section(&f, "sort");
    assert!(cases.len() > 100);
    for c in cases {
        let pattern = c["pattern"].as_str().unwrap();
        let input: Vec<String> = c["input"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_owned())
            .collect();
        let native: Vec<String> = c["sorted"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_owned())
            .collect();
        let mut ordered = input.clone();
        ordered.sort_by(|a, b| u16s(a).cmp(&u16s(b)));
        assert_eq!(ordered, native, "{pattern}");
        let expected: Vec<String> = native
            .iter()
            .filter(|p| native_resource_matches(&u16s(pattern), &u16s(p)))
            .cloned()
            .collect();
        assert_eq!(native_enumerate(pattern, &input), expected, "{pattern}");
    }
}

#[test]
fn thing_find_matches_427bd0() {
    let f = fixture();
    for c in section(&f, "lookup") {
        let mut reg = NativeThingRegistry::default();
        let names: Vec<String> = c["registered"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_owned())
            .collect();
        for n in &names {
            reg.register(n).unwrap();
        }
        for t in c["translations"].as_array().unwrap() {
            // 478E10 inserts unconditionally in the fixture; translate()
            // also inserts before its target check.
            let _ = reg.translate(t[0].as_str().unwrap(), t[1].as_str().unwrap());
        }
        let mut generic = aa_rules::actors::native_load::NativeDataRegistry::new("Fixture", Some(0));
        for (i, n) in names.iter().enumerate() {
            let source = aa_rules::resources::Provenance {
                path: "lookup".into(),
                sha256: String::new(),
                span: aa_rules::tgi::Span { start: i, end: i },
            };
            generic.load(n, true, vec![i], source).unwrap();
        }
        for t in c["translations"].as_array().unwrap() {
            let _ = generic.translate(t[0].as_str().unwrap(), t[1].as_str().unwrap());
        }
        for q in c["results"].as_array().unwrap() {
            let query = q["query"].as_str().unwrap();
            let got = reg.find(query).map(|slot| names[slot].clone());
            assert_eq!(got.as_deref(), q["result"].as_str(), "{query}");
            let generic_got = generic.find(query).map(|e| e.ids.clone());
            assert_eq!(generic_got.as_deref(), q["result_48d500"].as_str(), "48D500 {query}");
        }
    }
}

#[test]
fn thing_registration_matches_4bca00() {
    let f = fixture();
    for c in section(&f, "thing") {
        let mut reg = NativeThingRegistry::default();
        for step in c["steps"].as_array().unwrap() {
            let ids = step["ids"].as_str().unwrap();
            let result = reg.register(ids);
            assert_eq!(result.is_err(), !step["thrown"].is_null(), "{ids}");
            if let Err(message) = result {
                assert_eq!(message, format!("ThingData name already used: {ids}"));
                assert_eq!(step["thrown"], "ThingData name already used: %s");
            }
            assert_eq!(reg.find(ids).is_some(), step["lookup_after"].as_bool().unwrap());
        }
    }
}

const TRANSLATE_TEMPLATE: &str =
    "[template Translate]\r\n{\r\n\tstring node\r\n\tstring ids_old\r\n\tstring ids_new\r\n}\r\n";

#[test]
fn translate_roots_match_48d2d0_and_48d660() {
    let f = fixture();
    for c in section(&f, "translate") {
        let mut translate = String::new();
        for t in c["input"].as_array().unwrap() {
            translate.push_str(&format!(
                "[Translate template=Translate]\r\n{{\r\n\tnode = {}\r\n\tids_old = {}\r\n\tids_new = {}\r\n}}\r\n",
                t[0].as_str().unwrap(),
                t[1].as_str().unwrap(),
                t[2].as_str().unwrap()
            ));
        }
        let things: String = c["registered"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| thing(v.as_str().unwrap()))
            .collect();
        let mut report = NativeLoadReport::default();
        let _ = load_files(
            vec![
                file("Templates/template_translate.tgi", TRANSLATE_TEMPLATE),
                file("things.tgi", &things),
                file("translate.tgi", &translate),
            ],
            Limits::default(),
            &mut report,
        );
        let steps = c["steps"].as_array().unwrap();
        let last = steps.last().unwrap();
        let native_map: Vec<(String, String)> = last["translations_after"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| (p[0].as_str().unwrap().into(), p[1].as_str().unwrap().into()))
            .collect();
        let mut rust_map = report.thing_translations.clone();
        rust_map.sort();
        assert_eq!(rust_map, native_map, "{c}");
        let exception = report
            .events
            .iter()
            .find(|e| e.kind == NativeLoadEventKind::RootException && e.file == "translate.tgi");
        match last["thrown"].as_str() {
            None => assert!(exception.is_none(), "{c}"),
            Some(format) => {
                let detail = &exception.expect("native threw").detail;
                for piece in format
                    .split("%s")
                    .map(|p| p.trim().trim_matches('\'').trim())
                    .filter(|p| p.len() > 3)
                {
                    assert!(detail.contains(piece), "{detail} vs {format}");
                }
            }
        }
        // Processing stops at the throwing node: later Translate roots are not read.
        assert_eq!(report.translations.len(), steps.len(), "{c}");
    }
}

/// 48D660 oracle cases mapped onto Thing roots: a handler exception is a
/// duplicate IDS (4BCA00), an unknown tag stays unknown.
#[test]
fn root_dispatch_abandons_rest_of_file_like_48d660() {
    let f = fixture();
    for c in section(&f, "dispatch") {
        let source = c["source"].as_str().unwrap();
        let throwing: Vec<&str> = c["throwing_handlers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        let mut text = String::new();
        let mut seen = std::collections::BTreeMap::<String, usize>::new();
        let mut expected = Vec::new();
        for header in source.split("\r\n").filter(|l| l.starts_with('[')) {
            let name = header.trim_matches(['[', ']']);
            if name.starts_with("Template ") {
                text.push_str(&format!("{header}\r\n{{\r\n}}\r\n"));
            } else if name == "Z" {
                text.push_str("[Z]\r\n{\r\n}\r\n");
            } else {
                let n = seen.entry(name.into()).or_default();
                let ids = if throwing.contains(&name) {
                    "taken".to_string()
                } else {
                    format!("{}{}", name.to_ascii_lowercase(), n)
                };
                *n += 1;
                expected.push(ids.clone());
                text.push_str(&thing(&ids));
            }
        }
        let mut report = NativeLoadReport::default();
        let _ = load_files(
            vec![file("first.tgi", &thing("taken")), file("fixture.tgi", &text)],
            Limits::default(),
            &mut report,
        );
        let registered: Vec<String> = report
            .things
            .iter()
            .filter(|(_, f)| f == "fixture.tgi")
            .map(|(id, _)| id.clone())
            .collect();
        let dispatched: Vec<String> = c["dispatched"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| d["tag"].as_str().unwrap().to_owned())
            .filter(|t| !throwing.contains(&t.as_str()))
            .collect();
        assert_eq!(registered.len(), dispatched.len(), "{c}");
        let kept = report
            .events
            .iter()
            .any(|e| e.kind == NativeLoadEventKind::Kept && e.file == "fixture.tgi");
        assert_eq!(kept, c["returned"].as_u64().unwrap() == 0, "{c}");
    }
}

#[test]
fn template_registry_matches_parser_and_unload() {
    let f = fixture();
    for c in section(&f, "templates") {
        let files: Vec<ResolvedFile> = c["files"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, v)| {
                // Fixture roots X/Y/A become a known root tag without a modeled registry
                // (Text) so the
                // 48D970 release decision is the native one (the oracle
                // released those files through 5BCDD0).
                let text = v
                    .as_str()
                    .unwrap()
                    .replace("[X ", "[Text ")
                    .replace("[Y ", "[Text ")
                    .replace("[A]", "[Text]");
                file(&format!("f{i}.tgi"), &text)
            })
            .collect();
        let mut report = NativeLoadReport::default();
        let _ = load_files(files, Limits::default(), &mut report);
        for (i, error) in c["errors"].as_array().unwrap().iter().enumerate() {
            let name = format!("f{i}.tgi");
            let kinds: Vec<&NativeLoadEventKind> = report
                .events
                .iter()
                .filter(|e| e.file == name)
                .map(|e| &e.kind)
                .collect();
            let not_found = kinds.contains(&&NativeLoadEventKind::TemplateNotFound);
            let already = kinds.contains(&&NativeLoadEventKind::TemplateAlreadyDefined);
            match error.as_str() {
                None => assert!(!not_found && !already, "{} {name}", c["name"]),
                Some("Template definition not found.") => assert!(not_found, "{} {name}", c["name"]),
                Some("Template was already defined.") => assert!(already, "{} {name}", c["name"]),
                Some(other) => panic!("unexpected native error {other}"),
            }
        }
        for released in c["released_after"].as_array().unwrap() {
            let name = format!("f{}.tgi", released.as_u64().unwrap());
            assert!(
                report
                    .events
                    .iter()
                    .any(|e| e.kind == NativeLoadEventKind::Released && e.file == name),
                "{}",
                c["name"]
            );
        }
    }
}

#[test]
fn kingdom_resource_lookup_is_null_for_unknown_ids_like_4cdbb0() {
    let f = fixture();
    let text = "[Template Resource]\r\n{\r\n\tstring IDS\r\n}\r\n\
        [Resource template=Resource]\r\n{\r\n\tIDS = money\r\n}\r\n\
        [Resource template=Resource]\r\n{\r\n\tIDS = ammo\r\n}\r\n\
        [Resource template=Resource]\r\n{\r\n\tIDS = oil\r\n}\r\n";
    let catalog = aa_rules::actors::ActorCatalog::from_files(
        vec![file("resources.tgi", text)],
        Limits::default(),
    );
    for c in section(&f, "kingdom") {
        let vector: Vec<f64> = c["vector"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        let mut rebuilt = vec![0.0f64; 3];
        let mut flag = 0;
        for r in c["records"].as_array().unwrap() {
            let ids = r[0].as_str().unwrap();
            let value = r[1].as_f64().unwrap();
            let found = if ids.is_empty() {
                None
            } else {
                catalog
                    .native_named_definition("Resource", ids, Precision::Double53)
                    .unwrap()
            };
            match found {
                Some(_) => {
                    let index = ["money", "ammo", "oil"].iter().position(|n| *n == ids).unwrap();
                    rebuilt[index] = value;
                }
                None if !ids.is_empty() => flag = 1,
                None => {}
            }
        }
        assert_eq!(rebuilt, vector, "{c}");
        assert_eq!(flag, c["returned_flag"].as_u64().unwrap(), "{c}");
    }
}

#[test]
fn translate_errors_are_typed() {
    let mut reg = NativeThingRegistry::default();
    reg.register("alpha").unwrap();
    assert_eq!(reg.translate("o", "missing"), Err(NativeTranslateError::MissingTarget));
    assert_eq!(reg.translate("O", "alpha"), Err(NativeTranslateError::DuplicateOld));
    assert_eq!(reg.find("o"), None);
}

#[test]
fn section_headers_match_5bd080() {
    let f = fixture();
    for c in section(&f, "headers") {
        let header = c["header"].as_str().unwrap();
        let text = format!(
            "[Template T]\r\n{{\r\n\tstring ids = x\r\n}}\r\n{header}\r\n{{\r\n\tids = y\r\n}}\r\n"
        );
        let catalog = aa_rules::actors::ActorCatalog::from_files(
            vec![file("headers.tgi", &text)],
            Limits::default(),
        );
        let roots = catalog.resolved_definitions("A");
        if c["error"].is_null() {
            let roots = roots.unwrap_or_else(|e| panic!("{header}: {e}"));
            assert_eq!(roots.len(), 1, "{header}");
            let based = roots[0].sources().len() == 2;
            assert_eq!(based, c["flag_40_template_base"].as_bool().unwrap(), "{header}");
        } else {
            assert!(roots.is_err(), "{header} must stay malformed");
        }
    }
}

/// Generic DataManager<T>::Load in modes 0/1/2 (oracle section `generic`).
#[test]
fn generic_data_manager_duplicates_match_vf1c_4bd270_47d960() {
    use aa_rules::actors::native_load::{LoadOutcome, NativeDataRegistry};
    let f = fixture();
    let cases = section(&f, "generic");
    assert_eq!(cases.len(), 15);
    for c in cases {
        let mode = c["mode"].as_u64().unwrap() as u32;
        let mut registry = NativeDataRegistry::new("Fixture", Some(mode));
        let mut slots = Vec::new();
        for (i, step) in c["steps"].as_array().unwrap().iter().enumerate() {
            let ids = step["ids"].as_str().unwrap();
            let raw = step["name"].as_str().unwrap();
            let named = aa_rules::tgi::native_localized_text(raw).is_some_and(|t| !t.is_empty());
            let source = aa_rules::resources::Provenance {
                path: "generic.tgi".into(),
                sha256: String::new(),
                span: aa_rules::tgi::Span { start: i, end: i },
            };
            let result = registry.load(ids, named, vec![i], source);
            match step["thrown"].as_str() {
                None => {
                    let (slot, outcome) = result.unwrap_or_else(|e| panic!("{c}: {e}"));
                    // Mode 2 creation is native-fatal; fixtures never reach it unthrown.
                    assert_ne!(outcome, LoadOutcome::PredefinedAssumed);
                    slots.push(slot);
                }
                Some(format) => {
                    // Mode 2 with no predefined entries: the Rust registry cannot
                    // know code-predefined IDS and reports the assumption.
                    if mode == 2 {
                        let (_, outcome) = result.unwrap();
                        assert_eq!(outcome, LoadOutcome::PredefinedAssumed, "{c}");
                        assert_eq!(format, "Unknown data cannot be added to this data manager: '%s'");
                        break;
                    }
                    let message = result.unwrap_err();
                    let head = format.split("%s").next().unwrap();
                    assert!(message.starts_with(head), "{message} vs {format}");
                    break;
                }
            }
        }
        let reused = slots.len() > 1 && slots.windows(2).any(|w| w[0] == w[1]);
        assert_eq!(reused, c["same_definition_reused"].as_bool().unwrap(), "{c}");
    }
}

/// Denizen constructor 56B720 slot list (oracle section `denizen`).
#[test]
fn denizen_slots_match_56b720() {
    use aa_rules::native::denizen::{native_denizen_slots, native_inventory_insert};
    let f = fixture();
    for c in section(&f, "denizen") {
        let inventory: Vec<(String, i32)> = c["inventory"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| (e[0].as_str().unwrap().to_owned(), e[1].as_i64().unwrap() as i32))
            .collect();
        let slots = native_denizen_slots(inventory.iter().map(|(t, n)| (t.as_str(), *n)));
        let native: Vec<&str> = c["slots_head_first"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["target"].as_str().unwrap())
            .collect();
        let rust: Vec<&str> = slots.iter().map(|s| s.target.as_str()).collect();
        assert_eq!(rust, native, "{c}");
        for s in c["slots_head_first"].as_array().unwrap() {
            assert_eq!(s["flag_c"], 0);
            assert_eq!(s["flag_d"], 0);
        }
        assert!(slots.iter().all(|s| !s.worker && !s.militia_or_air_defensive));
    }
    // 462E60 grouping: a repeated IDS goes after its equal run, nothing merges.
    let mut list: Vec<String> = Vec::new();
    for name in ["a", "b", "a", "c", "b", "A"] {
        native_inventory_insert(&mut list, name.to_string(), |s| s);
    }
    assert_eq!(list, ["a", "a", "b", "b", "c", "A"]);
}

/// Character view-set selection 4D0720/45E5E0/45E690 (section `character_view`).
#[test]
fn character_view_set_matches_4d0720() {
    use aa_rules::native::character_view::native_character_view_set;
    let f = fixture();
    let mut compared = 0;
    for c in section(&f, "character_view") {
        let present: Vec<bool> = c["sets_present"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() != 0)
            .collect();
        let stance_aspects: Vec<i32> = c["stance_aspects"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_i64().unwrap() as i32)
            .collect();
        let stance = c["stance"].as_i64().unwrap() as i32;
        let variant = c["variant"].as_i64().unwrap() as i32;
        let index = if stance != -1 { stance_aspects[stance as usize] } else { variant };
        // The original reads past ActorData+1BC for an out-of-range index;
        // such fixture rows only reflect fixture padding and are not compared.
        if !present.is_empty() && index >= present.len() as i32 {
            assert!(native_character_view_set(&present, &stance_aspects, stance, variant).is_err());
            continue;
        }
        let rust = native_character_view_set(&present, &stance_aspects, stance, variant).unwrap();
        let native = c["chosen_set"].as_u64().map(|v| v as usize);
        assert_eq!(rust, native, "{c}");
        compared += 1;
    }
    assert!(compared >= 20);
}
