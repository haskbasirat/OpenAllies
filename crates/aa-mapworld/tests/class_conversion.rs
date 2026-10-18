//! Replays the native class-changed conversion oracle
//! (research/phase8-conversion/native_conversion.py: the original AA.exe
//! 4FAE80 / 4EA050 / 4F9840 / 4FC340 / 4FC600 tail under Unicorn).
use aa_mapworld::class_conversion::*;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;

fn research() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../research/phase8-conversion")
}
fn load(name: &str) -> Value {
    let path = research().join(name);
    serde_json::from_slice(&std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display())))
        .unwrap()
}
fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}
fn u32s(v: &Value) -> Vec<u32> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_u64().unwrap() as u32)
        .collect()
}
fn u16s(v: &Value) -> Vec<u16> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_u64().unwrap() as u16)
        .collect()
}
fn text_units(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}
fn n(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}

/// Native vector element (native_machine.Machine.element).
fn assert_element(record: &ChangedClassRecord<String>, e: &Value) {
    assert_eq!(record.thing_id_utf16, u16s(&e["ids"]));
    assert_eq!(record.definition, e["definition"].as_str().unwrap());
    assert_eq!(record.saved_id, n(&e["saved_id"]));
    assert_eq!(record.name_utf16, u16s(&e["name"]));
    assert_eq!(record.word_10, n(&e["word_10"]));
    assert_eq!(record.position_bits.to_vec(), u32s(&e["position"]));
    assert_eq!(record.direction_bits.to_vec(), u32s(&e["direction"]));
    assert_eq!(record.variant, n(&e["variant"]));
}

/// The native warnings of a case: 4B9F40 key followed by its 5A1040 call.
fn native_warnings(calls: &Value) -> Vec<(String, Vec<u16>)> {
    calls
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["seam"] == "5A1040")
        .map(|c| {
            (
                c["text"].as_str().unwrap().to_owned(),
                text_units(c["ids"].as_str().unwrap()),
            )
        })
        .collect()
}

/// Compares a Rust result of on_class_mismatch with one native 4FAE80 run.
fn check_load(
    result: Result<MismatchOutcome, ConversionError>,
    queue: &ConversionQueue<String>,
    case: &Value,
    vector_before: usize,
) {
    let calls = case["calls"].as_array().unwrap();
    let consumed = case["consumed"].as_u64().unwrap() as usize;
    match &case["outcome"] {
        Value::Null => {
            assert_eq!(case["out"], 0, "4FAE80 yields null");
            match result.unwrap() {
                MismatchOutcome::Converted { index, consumed: c } => {
                    assert_eq!(index, vector_before);
                    assert_eq!(c, consumed);
                    assert!(!calls.iter().any(|c| c["seam"] == "5A1040"));
                }
                MismatchOutcome::Skipped { warning, consumed: c } => {
                    assert_eq!(c, consumed);
                    let w = native_warnings(&case["calls"]);
                    assert_eq!(w, vec![(warning.key.to_owned(), warning.ids_utf16.clone())]);
                    assert_eq!(warning.key, CHANGED_CLASS_WARNING);
                }
            }
        }
        outcome => {
            let reads = case["reads"].as_array().unwrap();
            match (outcome["kind"].as_str().unwrap(), result) {
                ("LoadWarningException", Err(ConversionError::BadDataVersion { offset, .. })) => {
                    assert_eq!(outcome["format"], "Bad data version");
                    // the version word is the last native read
                    let last = reads.last().unwrap();
                    assert_eq!(offset, last[0].as_u64().unwrap() as usize);
                    assert_eq!(last[1], 4);
                }
                ("ReadPastChunk", Err(ConversionError::ReadPastChunk { offset, wanted, .. })) => {
                    assert_eq!(offset, outcome["at"].as_u64().unwrap() as usize);
                    assert_eq!(wanted, outcome["wanted"].as_u64().unwrap() as usize);
                }
                (kind, other) => panic!("native {kind} vs Rust {other:?}"),
            }
        }
    }
    let vector = case["vector"].as_array().unwrap();
    assert_eq!(queue.len(), vector.len());
    for (r, e) in queue.records().iter().zip(vector) {
        assert_element(r, e);
    }
}

#[test]
fn class_decision_table_matches_native_4fae80() {
    let fixture = load("native-conversion.json");
    let mut checked = 0;
    for case in fixture["mismatch"].as_array().unwrap() {
        if case.get("label").is_some() {
            // Null results BEFORE the class compare (4FAF14 generation, 427BD0
            // unknown IDS) belong to the restoration loop; only the prefix
            // length is shared with this module.
            let p = read_prefix(&hex(case["payload"].as_str().unwrap())).unwrap();
            assert_eq!(p.length, case["consumed"].as_u64().unwrap() as usize);
            continue;
        }
        let saved = n(&case["saved_class"]);
        let def_class = n(&case["definition_class"]);
        let ctx = case["context"].as_bool().unwrap();
        let constructs = case["calls"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["seam"] == "4FB0A9");
        let check = check_class(saved, def_class, ctx);
        let payload = hex(case["payload"].as_str().unwrap());
        let mut queue = ConversionQueue::new();
        let result = on_class_mismatch(
            &payload,
            "tree".to_owned(),
            def_class,
            ctx.then_some(&mut queue),
        );
        if constructs {
            assert_eq!(check, ClassCheck::Matches);
            assert!(matches!(result, Err(ConversionError::NotAMismatch(c)) if c == saved));
            assert_eq!(case["out"], 0x0BAD_F00D_u32);
        } else {
            assert_ne!(check, ClassCheck::Matches);
            assert_eq!(check == ClassCheck::Convert, !case["vector"].as_array().unwrap().is_empty());
            check_load(result, &queue, case, 0);
        }
        checked += 1;
    }
    assert_eq!(checked, 13 * 10 * 2);
}

#[test]
fn saved_field_reads_match_native_4ea050() {
    let fixture = load("native-conversion.json");
    let sections = fixture["reads"].as_array().unwrap();
    assert_eq!(sections.len(), 3, "PC24, PC53 and PC64 runs");
    let mut errors = BTreeMap::<String, usize>::new();
    for section in sections {
        for case in section["rows"].as_array().unwrap() {
            let payload = hex(case["payload"].as_str().unwrap());
            let mut queue = ConversionQueue::new();
            let result = on_class_mismatch(&payload, "arctic_trees".to_owned(), 9, Some(&mut queue));
            if let Err(e) = &result {
                let key = format!("{e:?}").split(' ').next().unwrap().to_owned();
                *errors.entry(key).or_default() += 1;
            }
            check_load(result, &queue, case, 0);
            // The read sequence: total bytes equal; the Rust reader consumed
            // exactly the native reads in order (checked through `consumed`).
            if case["outcome"].is_null() {
                let total: u64 = case["reads"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|r| r[1].as_u64().unwrap())
                    .sum();
                assert_eq!(total, case["consumed"].as_u64().unwrap());
            }
        }
    }
    assert!(errors.keys().any(|k| k.starts_with("BadDataVersion")));
    assert!(errors.keys().any(|k| k.starts_with("ReadPastChunk")));
}

#[test]
fn append_sequence_matches_native_4f9840() {
    let fixture = load("native-conversion.json");
    let append = &fixture["append"];
    let defs: BTreeMap<&str, u32> = [("a_tree", 9), ("b_hedge", 9), ("c_bridge", 3)].into();
    let mut queue = ConversionQueue::new();
    for step in append["steps"].as_array().unwrap() {
        let payload = hex(step["payload"].as_str().unwrap());
        let prefix = read_prefix(&payload).unwrap();
        let ids = String::from_utf16(&prefix.thing_id_utf16).unwrap();
        let before = queue.len();
        let result = on_class_mismatch(&payload, ids.clone(), defs[ids.as_str()], Some(&mut queue));
        assert_eq!(step["out"], 0);
        assert_eq!(queue.len(), n(&step["count"]) as usize);
        match result.unwrap() {
            MismatchOutcome::Converted { index, consumed } => {
                assert_eq!(index, before);
                assert_eq!(consumed, step["consumed"].as_u64().unwrap() as usize);
            }
            MismatchOutcome::Skipped { warning, consumed } => {
                assert_eq!(consumed, step["consumed"].as_u64().unwrap() as usize);
                assert_eq!(
                    native_warnings(&step["calls"]),
                    vec![(warning.key.to_owned(), warning.ids_utf16)]
                );
            }
        }
    }
    let vector = append["vector"].as_array().unwrap();
    assert_eq!(queue.len(), vector.len());
    for (r, e) in queue.records().iter().zip(vector) {
        assert_element(r, e);
    }
}

/// Test-side model of the 4FC340 loop (generation check, Thing lookup,
/// class compare via the hook, construct seam consumption, 858F50 seek after
/// a null result) — the shape the restoration loop must have.
#[test]
fn section_loop_seeks_match_native_4fc340() {
    let fixture = load("native-conversion.json");
    for case in fixture["section"].as_array().unwrap() {
        let scenario = case["scenario"].as_str().unwrap();
        let defs: BTreeMap<String, u32> = case["definitions"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), n(v)))
            .collect();
        let generations: BTreeMap<u32, u32> = case["generations"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.parse().unwrap(), n(v)))
            .collect();
        let data = hex(case["data"].as_str().unwrap());
        let index = hex(case["index"].as_str().unwrap());
        let calls = case["calls"].as_array().unwrap();
        let native_seeks: Vec<u64> = calls
            .iter()
            .filter(|c| c["seam"] == "858F50")
            .map(|c| c["offset"].as_u64().unwrap())
            .collect();
        let word = |b: &[u8], at: usize| u32::from_le_bytes(b[at..at + 4].try_into().unwrap());
        let opened: Vec<&str> = calls
            .iter()
            .filter(|c| c["seam"] == "859030" && c["found"] == true)
            .map(|c| c["chunk"].as_str().unwrap())
            .collect();
        if opened != ["DYNAMIC_THINGS_INDEX", "DYNAMIC_THINGS"] {
            assert!(case["vector"].as_array().unwrap().is_empty() && native_seeks.is_empty());
            continue;
        }
        if word(&index, 0) != 0 || word(&data, 0) != 0 {
            // 4FC3A2/4FC409: "Bad data version" (858AD0) before any record.
            assert_eq!(case["outcome"]["kind"], "FileException");
            assert!(native_seeks.is_empty());
            continue;
        }
        let count = word(&index, 4) as usize;
        let lengths: Vec<usize> = (0..count).map(|i| word(&index, 8 + 4 * i) as usize).collect();
        let mut queue = ConversionQueue::new();
        let mut seeks = Vec::new();
        let mut pos = 8usize;
        let mut error = None;
        for length in lengths {
            let start = pos;
            let stream = &data[start..];
            let prefix = read_prefix(stream).unwrap();
            let slot = prefix.saved_id & 0xffff;
            let generation = prefix.saved_id >> 16;
            let def = String::from_utf16(&prefix.thing_id_utf16).unwrap();
            let null = if generations.get(&slot).copied().unwrap_or(0) != generation {
                true
            } else if let Some(&class) = defs.get(&def) {
                match on_class_mismatch(stream, def.clone(), class, Some(&mut queue)) {
                    Ok(_) => true,
                    Err(ConversionError::NotAMismatch(_)) => {
                        // 4FB0A9 construction seam: consumed the prefix plus
                        // the fixture count; no seek follows a non-null result.
                        pos = start + prefix.length + case["construct_consume"].as_u64().unwrap() as usize;
                        false
                    }
                    Err(e) => {
                        error = Some(e);
                        break;
                    }
                }
            } else {
                true // 427BD0 null: unknown IDS
            };
            if null {
                pos = start + length;
                seeks.push(pos as u64);
            }
        }
        assert_eq!(seeks, native_seeks, "{scenario}");
        match &case["outcome"] {
            Value::Null => {
                assert!(error.is_none());
                assert_eq!(pos as u64, case["final_position"].as_u64().unwrap(), "{scenario}");
            }
            o => {
                assert_eq!(o["kind"], "LoadWarningException");
                assert!(matches!(error, Some(ConversionError::BadDataVersion { .. })));
            }
        }
        let vector = case["vector"].as_array().unwrap();
        assert_eq!(queue.len(), vector.len(), "{scenario}");
        for (r, e) in queue.records().iter().zip(vector) {
            assert_element(r, e);
        }
    }
}

/// Definition inputs of the replacement loop (vf+30, +108, +110), keyed by label.
#[derive(Clone)]
struct DefInputs {
    object_data: bool,
    variations: bool,
    count: i32,
}
struct World {
    defs: BTreeMap<String, DefInputs>,
    results: Vec<bool>,
    created: Vec<ObjectCreationParams<String>>,
    reads: Vec<(&'static str, String)>,
}
impl VariationSource<String> for World {
    type Error = String;
    fn is_object_data(&mut self, d: &String) -> Result<bool, String> {
        self.reads.push(("vf30", d.clone()));
        Ok(self.defs[d].object_data)
    }
    fn variations_enabled(&mut self, d: &String) -> Result<bool, String> {
        self.reads.push(("+108", d.clone()));
        Ok(self.defs[d].variations)
    }
    fn aspect_count(&mut self, d: &String) -> Result<i32, String> {
        self.reads.push(("+110", d.clone()));
        Ok(self.defs[d].count)
    }
}
impl ReplacementWorld<String> for World {
    type Created = usize;
    fn create_thing(&mut self, p: &ObjectCreationParams<String>) -> Result<Option<usize>, String> {
        let i = self.created.len();
        self.created.push(p.clone());
        Ok(self.results[i].then_some(i))
    }
}
fn assert_block(p: &ObjectCreationParams<String>, b: &[Value]) {
    // [vtable, definition, position, direction, name, variant, requested_id]
    assert_eq!(n(&b[0]), ObjectCreationParams::<String>::VTABLE);
    assert_eq!(p.definition, b[1].as_str().unwrap());
    assert_eq!(p.position_bits.to_vec(), u32s(&b[2]));
    assert_eq!(p.direction_bits.to_vec(), u32s(&b[3]));
    assert_eq!(p.name_utf16, u16s(&b[4]));
    assert_eq!(p.variant, n(&b[5]));
    assert_eq!(p.requested_id, n(&b[6]));
}
fn block_row(b: &Value) -> Vec<Value> {
    ["vtable", "definition", "position", "direction", "name", "variant", "requested_id"]
        .iter()
        .map(|k| b[*k].clone())
        .collect()
}

#[test]
fn replacement_loop_matches_native_4fc600_tail() {
    let fixture = load("native-conversion.json");
    let vtables_object = ["ObjectData", "ActorData", "DynamicDoodadData"];
    for case in fixture["replace"].as_array().unwrap() {
        let mut defs = BTreeMap::new();
        let mut classes = BTreeMap::new();
        for d in case["definitions"].as_array().unwrap() {
            let label = d[0].as_str().unwrap().to_owned();
            classes.insert(label.clone(), n(&d[1]));
            defs.insert(
                label,
                DefInputs {
                    // vf+30 of the original vtables (4C9FA0 / 4F0500)
                    object_data: vtables_object.contains(&d[2].as_str().unwrap()),
                    variations: n(&d[3]) & 0xff != 0,
                    count: n(&d[4]) as i32,
                },
            );
        }
        let mut queue = ConversionQueue::new();
        for p in case["payloads"].as_array().unwrap() {
            let bytes = hex(p.as_str().unwrap());
            let ids = String::from_utf16(&read_prefix(&bytes).unwrap().thing_id_utf16).unwrap();
            let class = classes[&ids];
            assert!(matches!(
                on_class_mismatch(&bytes, ids, class, Some(&mut queue)).unwrap(),
                MismatchOutcome::Converted { .. }
            ));
        }
        for (r, e) in queue.records().iter().zip(case["vector"].as_array().unwrap()) {
            assert_element(r, e);
        }
        let results: Vec<bool> = case["results"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_bool().unwrap())
            .collect();
        let mut world = World {
            defs,
            results: results.clone(),
            created: Vec::new(),
            reads: Vec::new(),
        };
        let done = replace_converted(queue, &mut world).unwrap();
        let blocks = case["blocks"].as_array().unwrap();
        assert_eq!(world.created.len(), blocks.len());
        for (i, (p, b)) in world.created.iter().zip(blocks).enumerate() {
            assert_block(p, &block_row(b));
            assert_eq!(b["result"], results[i]);
        }
        let native = native_warnings(&case["calls"]);
        let rust: Vec<(String, Vec<u16>)> = done
            .iter()
            .map(|r| (r.warning.key.to_owned(), r.warning.ids_utf16.clone()))
            .collect();
        assert_eq!(rust, native, "{}", case["scenario"]);
        for (r, ok) in done.iter().zip(&results) {
            assert_eq!(r.created.is_some(), *ok);
        }
    }
}

#[test]
fn native_create_dispatch_table_is_pinned() {
    // 4F9C10 on the A25B1C block: what the restoration agent's fresh-creation
    // port must reproduce for each definition class (oracle section dispatch).
    let fixture = load("native-conversion.json");
    let mut table = BTreeMap::new();
    for case in fixture["dispatch"].as_array().unwrap() {
        let class = n(&case["definition_class"]);
        let b = &case["block"];
        assert_eq!(n(&b["vtable"]), 0xA25B1C);
        assert!(u16s(&b["name"]).is_empty());
        let outcome = match (&case["outcome"], &case["returned"]) {
            (Value::Null, r) => {
                let logs: Vec<_> = case["calls"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|c| c["seam"] == "404ED0")
                    .map(|c| c["format"].as_str().unwrap().to_owned())
                    .collect();
                assert_eq!(r, 0);
                format!("null: {}", logs.join("; "))
            }
            (o, _) if o["kind"] == "ReachedConstructor" => {
                format!("constructor {:#X}", o["address"].as_u64().unwrap())
            }
            (o, _) => format!("native fault at {}", o["pc"].as_str().unwrap()),
        };
        table.insert(class, outcome);
    }
    let expect: BTreeMap<u32, &str> = [
        (0, "native fault at 0x4f9f6e"),
        (1, "native fault at 0x4f9f6e"),
        (2, "constructor 0x4EA5E0"),
        (3, "null: Actor class requires a GACD: %s"),
        (4, "null: Kingdom class requires a GKCD: %s"),
        (5, "constructor 0x4E66D0"),
        (6, "constructor 0x4DB610"),
        (7, "constructor 0x4DBEE0"),
        (8, "null: Group class requires a GACD: %s"),
        (9, "constructor 0x4DCB30"),
        (10, "constructor 0x4DC450"),
        (11, "native fault at 0x4f9f6e"),
    ]
    .into();
    let got: BTreeMap<u32, &str> = table.iter().map(|(k, v)| (*k, v.as_str())).collect();
    assert_eq!(got, expect);
}

fn real_fixture() -> Option<Value> {
    let path = research().join("native-conversion-real.json");
    path.exists().then(|| load("native-conversion-real.json"))
}

#[test]
fn real_group1_records_match_native() {
    let Some(fixture) = real_fixture() else {
        panic!("research/phase8-conversion/native-conversion-real.json missing (see README)");
    };
    let mut total = 0usize;
    let mut maps = 0usize;
    let mut created_by_code = BTreeMap::<String, usize>::new();
    for map in fixture["real"].as_array().unwrap() {
        maps += 1;
        let mut defs = BTreeMap::new();
        let mut classes = BTreeMap::new();
        for (ids, d) in map["definitions"].as_object().unwrap() {
            classes.insert(ids.clone(), n(&d["runtime_class"]));
            defs.insert(
                ids.clone(),
                DefInputs {
                    object_data: native_kind_is_object_data(n(&d["native_kind"])).unwrap(),
                    variations: d["variations_enabled"].as_bool().unwrap(),
                    count: d["aspect_count"].as_i64().unwrap() as i32,
                },
            );
        }
        let mut queue = ConversionQueue::new();
        for row in map["records"].as_array().unwrap() {
            // [ordinal, saved_id, start, length, consumed, consumed bytes]
            let bytes = hex(row[5].as_str().unwrap());
            let prefix = read_prefix(&bytes).unwrap();
            assert_eq!(prefix.class_code, 2);
            assert_eq!(prefix.saved_id, n(&row[1]));
            let ids = String::from_utf16(&prefix.thing_id_utf16).unwrap();
            let outcome = on_class_mismatch(&bytes, ids.clone(), classes[&ids], Some(&mut queue)).unwrap();
            assert_eq!(
                outcome,
                MismatchOutcome::Converted {
                    index: queue.len() - 1,
                    consumed: row[4].as_u64().unwrap() as usize
                }
            );
            assert!(row[4].as_u64().unwrap() <= row[3].as_u64().unwrap());
            total += 1;
        }
        let vector = map["vector"].as_array().unwrap();
        assert_eq!(queue.len(), vector.len());
        for (r, e) in queue.records().iter().zip(vector) {
            // [ids, definition, saved_id, name, word_10, position, direction, variant]
            let e = serde_json::json!({"ids": e[0], "definition": e[1], "saved_id": e[2], "name": e[3],
                "word_10": e[4], "position": e[5], "direction": e[6], "variant": e[7]});
            assert_element(r, &e);
        }
        // 4F9C10 results of the native tail run: null exactly where the
        // original 4F9C10 refuses the A25B1C block before construction.
        let results: Vec<bool> = map["results"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_bool().unwrap())
            .collect();
        for ((code, ok), e) in map["create"].as_array().unwrap().iter().zip(&results).zip(vector) {
            let class = classes[e[1].as_str().unwrap()];
            let code = code.as_str().unwrap();
            match class {
                9 => assert_eq!(code, "ctor:4DCB30"),
                3 => assert_eq!(code, "null:Actor class requires a GACD: %s"),
                other => panic!("unexpected class {other}"),
            }
            assert_eq!(*ok, class == 9);
            *created_by_code.entry(code.to_owned()).or_default() += 1;
        }
        let mut world = World {
            defs,
            results,
            created: Vec::new(),
            reads: Vec::new(),
        };
        let done = replace_converted(queue, &mut world).unwrap();
        let blocks = map["blocks"].as_array().unwrap();
        assert_eq!(world.created.len(), blocks.len());
        for (p, b) in world.created.iter().zip(blocks) {
            assert_block(p, b.as_array().unwrap());
        }
        let native: Vec<(String, Vec<u16>)> = map["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|w| (w[0].as_str().unwrap().to_owned(), text_units(w[1].as_str().unwrap())))
            .collect();
        let rust: Vec<(String, Vec<u16>)> = done
            .iter()
            .map(|r| (r.warning.key.to_owned(), r.warning.ids_utf16.clone()))
            .collect();
        assert_eq!(rust, native);
    }
    assert_eq!((maps, total), (27, 7879));
    // Expected effect on inventory group 1 once wired: every record is
    // converted (null result, no failure); 7,878 reach the DynamicDoodad
    // constructor 4DCB30 of the fresh-creation path, 1 (london_bridge) is
    // refused by 4F9C10 and gets the "can't replace" warning.
    let expect: BTreeMap<String, usize> = [
        ("ctor:4DCB30".to_owned(), 7878),
        ("null:Actor class requires a GACD: %s".to_owned(), 1),
    ]
    .into();
    assert_eq!(created_by_code, expect);
}

#[test]
fn definition_virtual_30_matches_native() {
    let fixture = load("native-conversion.json");
    let rows = fixture["vf30"].as_array().unwrap();
    assert_eq!(rows.len(), 24);
    for row in rows {
        let kind = n(&row["native_kind"]);
        assert_eq!(
            native_kind_is_object_data(kind),
            Some(n(&row["eax"]) & 0xff != 0),
            "kind {kind}"
        );
    }
    assert_eq!(native_kind_is_object_data(24), None);
}

fn game_root() -> PathBuf {
    PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"))
}

/// Writes research/phase8-conversion/real-definitions.json: the native
/// definition inputs of every group-1 IDS (runtime class, ObjectData+108,
/// +110) from the definitions agent's native catalog. Input of the oracle's
/// `real` section.
#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 assets; writes the oracle input"]
fn dump_real_conversion_definitions() {
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
    };
    let game = game_root();
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let catalog = ActorCatalog::load_v14(&mut sources).unwrap();
    let inventory = load("../phase8-definitions/inventory-round2.raw.json");
    let mut ids = std::collections::BTreeSet::new();
    for map in inventory["maps"].as_array().unwrap() {
        for f in map["failures"].as_array().unwrap() {
            if f["error"].as_str().unwrap().starts_with("source class ") {
                ids.insert(f["definition"].as_str().unwrap().to_owned());
            }
        }
    }
    let mut out = serde_json::Map::new();
    for id in &ids {
        let def = catalog.find(id).unwrap_or_else(|| panic!("{id} resolves natively"));
        let category = catalog.native_category(&def.id).unwrap();
        let creation = catalog
            .native_creation_definition(&def.id, aa_numeric::Precision::Double53)
            .unwrap();
        out.insert(
            id.clone(),
            serde_json::json!({
                "resolved_id": def.id,
                "category": category.name,
                "native_kind": category.native_kind,
                "runtime_class": category.runtime_class,
                "variations_enabled": creation.variations_enabled,
                "aspect_count": creation.aspects.len(),
            }),
        );
    }
    let value = serde_json::json!({
        "source": "aa_rules ActorCatalog::load_v14 (native index load), native_category, native_creation_definition (49DC40 aspect list)",
        "definitions": out,
    });
    std::fs::write(
        research().join("real-definitions.json"),
        serde_json::to_string_pretty(&value).unwrap(),
    )
    .unwrap();
}

/// The inventory-placement path equals the byte path on every group-1 record
/// of the original maps, and the fixture bytes are the maps' bytes.
#[test]
#[ignore = "requires AA_GAME_ROOT original maps"]
fn real_maps_placement_path_matches_byte_path() {
    use aa_mapworld::{Inventory, MapArchive};
    let Some(fixture) = real_fixture() else {
        panic!("native-conversion-real.json missing");
    };
    let root = game_root().join("reconstruction/assets/original/Data/Maps");
    let mut total = 0;
    for map in fixture["real"].as_array().unwrap() {
        let archive =
            MapArchive::from_bytes(std::fs::read(root.join(map["map"].as_str().unwrap())).unwrap())
                .unwrap();
        let data = archive.decode_chunk("DYNAMIC_THINGS", u64::MAX).unwrap();
        let inventory = Inventory::decode(&archive, u64::MAX).unwrap();
        let by_ordinal: BTreeMap<usize, &aa_mapworld::Placement> = inventory
            .placements
            .iter()
            .map(|p| (p.record_ordinal, p))
            .collect();
        for row in map["records"].as_array().unwrap() {
            let ordinal = row[0].as_u64().unwrap() as usize;
            let start = row[2].as_u64().unwrap() as usize;
            let consumed = row[4].as_u64().unwrap() as usize;
            let stream = &data.bytes[start..];
            assert_eq!(&stream[..consumed], hex(row[5].as_str().unwrap()).as_slice());
            let from_bytes = read_changed_class_record(stream, ()).unwrap();
            let placement = by_ordinal[&ordinal];
            assert_eq!(placement.source_span.offset, start);
            let from_placement = read_placement_record(placement, ()).unwrap();
            assert_eq!(from_bytes, from_placement);
            total += 1;
        }
    }
    assert_eq!(total, 7879);
}
