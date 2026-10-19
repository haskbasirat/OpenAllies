use super::*;
use std::{cell::Cell, sync::Arc};

fn native() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../research/development10/resource-model-unload/native-model-unload.json"
    ))
    .unwrap()
}
fn kind(name: &str) -> Option<Kind> {
    match name {
        "Texture" => Some(Kind::Texture),
        "Model" => Some(Kind::Model),
        "Sequence" => Some(Kind::Sequence),
        "Sound" => Some(Kind::Sound),
        "Map" | "Film" => None,
        other => panic!("unknown native class {other}"),
    }
}
fn hex(v: &serde_json::Value) -> u32 {
    u32::from_str_radix(v.as_str().unwrap().trim_start_matches("0x"), 16).unwrap()
}
fn units(v: &serde_json::Value) -> Vec<u16> {
    serde_json::from_value(v.clone()).unwrap()
}
///Genuine 5A60E0/5A7DB0/5A5FB0/59B400 constructor projections, then the
///row's flags/count stores; this is an explicit field transport, not a load.
fn entry(origin: &Arc<()>, priority: &crate::resource_task::DefaultPriority, kind: Kind, index: usize, flags: u32, count: u32) -> Entry {
    let name = format!("Native/Row{index}.{}", match kind {
        Kind::Texture => "dds",
        Kind::Model => "nif",
        Kind::Sequence => "kf",
        Kind::Sound => "wav",
    });
    let source = crate::resource_objects::SourceFile::from_catalog(&aa_rules::sources::CatalogEntry {
        logical_path: name.clone(),
        size: 0,
        origin: aa_rules::sources::Origin::OriginalArchive { member_index: 0 },
    });
    let ids: Vec<u16> = name.encode_utf16().collect();
    let mut entry = match kind {
        Kind::Texture => Entry::texture(origin, ids, source, priority),
        Kind::Model => Entry::model(origin, ids, source, priority),
        Kind::Sequence => Entry::sequence(origin, ids, source, priority),
        Kind::Sound => Entry::sound(origin, ids, source, priority),
    }
    .unwrap();
    entry.flags8 = flags;
    entry.requests4 = count;
    entry
}

#[test]
fn native_class_descriptor_chains_and_constructor_vtables() {
    let native = native();
    for (name, class) in native["classes"].as_object().unwrap() {
        let chain: Vec<u32> = class["chain"].as_array().unwrap().iter().map(hex).collect();
        assert_eq!(chain.contains(&NI_AVOBJ_RESOURCE_B3D990), class["ni_avobj"].as_bool().unwrap());
        match kind(name) {
            Some(kind) => assert_eq!(native_class_chain(kind), chain.as_slice(), "{name}"),
            //Map/Film are typed MapResource/FilmResource owners outside Entry.
            None => assert!(!class["ni_avobj"].as_bool().unwrap(), "{name}"),
        }
    }
    assert_eq!(native["classes"]["Model"]["vtable"], "0xa45c98");
    assert_eq!(native["classes"]["Model"]["vf40"], "0x5a6560");
    assert_eq!(native["classes"]["Model"]["vf54"], "0x5a7cf0");
}

#[test]
fn every_native_5aca50_row_selects_the_same_entries_all_pc() {
    let native = native();
    let origin = Arc::new(());
    let priority = crate::resource_task::DefaultPriority::from_fresh_application(&origin);
    let mut rows = 0;
    let mut eligible_rows = 0;
    for row in native["cases"].as_array().unwrap() {
        let input = &row["input"];
        let output = &row["output"];
        let events = output["events"].as_array().unwrap();
        //'select' stops at the 5ABFF0 entry; 'unload' executes its body, whose
        //unconditional bit5 branch names the same resource at 59BFA0.
        let mut native_selected: Vec<usize> = events
            .iter()
            .filter_map(|e| match e[0].as_str().unwrap() {
                "5ABFF0" => Some(e[1].as_u64().unwrap() as usize),
                "cache_remove_59bfa0" => Some(e[2].as_u64().unwrap() as usize),
                _ => None,
            })
            .collect();
        native_selected.sort();
        let resources = input["resources"].as_array().unwrap();
        let mut entries = Vec::new();
        let mut selected = Vec::new();
        for (index, resource) in resources.iter().enumerate() {
            let flags = resource["flags"].as_u64().unwrap() as u32;
            let count = resource["count"].as_u64().unwrap() as u32;
            let (flags_read, count_read) = (Cell::new(false), Cell::new(false));
            let class = resource["class"].as_str().unwrap();
            let eligible = match kind(class) {
                Some(kind) => eligible_5aca50(kind, || { flags_read.set(true); flags }, || { count_read.set(true); count }),
                None => false,
            };
            //Native read guard: only the NiAVObj class reads +8, and +4 only
            //after bit5 matched (`shr 5; test cl,1; je` precedes `mov eax,[edi+4]`).
            assert_eq!(flags_read.get(), class == "Model", "{row}");
            assert_eq!(count_read.get(), class == "Model" && (flags >> 5) & 1 != 0, "{row}");
            if eligible {
                selected.push(index);
            }
            if let Some(kind) = kind(class) {
                entries.push(entry(&origin, &priority, kind, index, flags, count));
            }
        }
        let changed = events.iter().any(|e| e[0] == "assign_counted_string");
        let mut state = crate::vworld_resource_context::State::constructed();
        let current = units(&input["current"]);
        if !current.is_empty() {
            state.assign_and_invalidate(&current, || Ok((0, 0))).unwrap();
        }
        let mut reached = false;
        let result = state.assign_and_invalidate(&units(&input["next"]), || {
            reached = true;
            invalidate_5aca50(entries.iter())?;
            Ok((0, 0))
        });
        assert_eq!(reached, changed, "{row}");
        assert_eq!(state.current(), units(&output["current"]).as_slice(), "{row}");
        if changed {
            assert_eq!(selected, native_selected, "{row}");
            assert_eq!(result.is_err(), !selected.is_empty(), "{row}");
            let classified = output["classified"].as_array().unwrap();
            assert_eq!(classified.len(), resources.len(), "every Manager+60 node is classified");
        } else {
            assert!(output["classified"].as_array().unwrap().is_empty());
            assert!(!result.as_ref().unwrap().changed);
        }
        if let Err(error) = &result {
            assert!(error.contains("5ABFF0") && error.contains("new+80 retained"), "{error}");
            eligible_rows += 1;
        }
        //The entries are immutably borrowed: neither path changes +4/+8.
        for (entry, resource) in entries.iter().zip(resources.iter().filter(|r| kind(r["class"].as_str().unwrap()).is_some())) {
            assert_eq!(entry.flags8, resource["flags"].as_u64().unwrap() as u32);
            assert_eq!(entry.requests4, resource["count"].as_u64().unwrap() as u32);
        }
        rows += 1;
    }
    assert_eq!(rows, 276);
    assert!(eligible_rows > 0);
}

#[test]
fn port_reachable_states_take_the_native_skip_and_unload_body_is_recorded() {
    let native = native();
    //Every flag value the current Rust port can store on a Resource before
    //43F0F1 (constructors, attach, Aspect/Data/VFX/load-kernel option bits,
    //loaded 1|4|6) has bit5 clear: 59B9F0 is the sole bit5 writer.
    for row in native["cases"].as_array().unwrap().iter().take(36 * 3) {
        assert!(row["output"]["events"].as_array().unwrap().iter().all(|e| e[0] != "5ABFF0"), "{row}");
    }
    let unload: Vec<_> = native["cases"].as_array().unwrap().iter().filter(|r| r["input"]["stage"] == "unload").collect();
    assert_eq!(unload.len(), 4 * 3);
    for row in unload {
        let events = row["output"]["events"].as_array().unwrap();
        let names: Vec<&str> = events.iter().map(|e| e[0].as_str().unwrap()).collect();
        let flags = row["input"]["resources"][0]["flags"].as_u64().unwrap() as u32;
        let mut expected = vec!["assign_counted_string", "lock_857e10", "lock_857e10"];
        if flags & 1 != 0 {
            expected.extend(["vf40_5A6560", "vf54_5A7CF0", "log_404f90"]);
            //5AC014 clears bit0 BEFORE vf40 is called.
            assert_eq!(events[3][2].as_u64().unwrap() as u32, flags & !1);
            assert_eq!(events[5][5], "Unloaded %s");
            assert_eq!(events[5][3], 0x523);
        }
        expected.extend(["cache_remove_59bfa0", "unlock_857de0", "unlock_857de0"]);
        assert_eq!(names, expected, "{row}");
        assert_eq!(events[1][1], "0x20");
        assert_eq!(events[2][1], "0x3c");
        assert_eq!(events[events.len() - 2][1], "0x3c");
        assert_eq!(events[events.len() - 1][1], "0x20");
        let cache = events.iter().find(|e| e[0] == "cache_remove_59bfa0").unwrap();
        assert_eq!(cache[1], "0x9c");
    }
}
