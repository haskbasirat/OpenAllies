use super::*;
use serde_json::Value as Json;
#[test]
fn real_original_v14_constructor_cells_match_all_pc() {
    let f: Json = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-current-svars/native-constructors.json"
    ))
    .unwrap();
    assert_eq!(f["cases"].as_array().unwrap().len(), 6);
    for case in f["cases"].as_array().unwrap() {
        let s = Store::constructors();
        let rows = case["registrations"].as_array().unwrap();
        assert_eq!(s.entries.len(), rows.len());
        for (e, r) in s.entries.iter().zip(rows) {
            let name: String = r["name"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| char::from_u32(x.as_u64().unwrap() as u32).unwrap())
                .collect();
            assert_eq!(e.name, name);
            assert_eq!(e.flags, r["flags"].as_u64().unwrap() as u32);
            assert_eq!(e.value.kind(), r["kind"].as_u64().unwrap() as u32);
            let w = |i: usize| r["words"][i].as_u64().unwrap() as u32;
            match &e.value {
                Value::Bool(v) => assert_eq!(*v, w(0) as u8),
                Value::Fixed(v) | Value::Int(v) => {
                    assert_eq!(*v, w(0));
                    assert_eq!(e.bounds, [w(5), w(6)])
                }
                Value::Point3(v) => assert_eq!(*v, [w(1), w(2), w(3)]),
                Value::CString(_) => panic!("no String in135 ctor rows"),
            }
        }
    }
}
#[test]
fn actual_typed_setter_rows_and_nonfinite_explicit_boundary() {
    let f: Json = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-current-svars/native-setters.json"
    ))
    .unwrap();
    for r in f["rows"].as_array().unwrap() {
        let k = r["kind"].as_u64().unwrap() as u32;
        let flags = r["flags"].as_u64().unwrap() as u32;
        let start = 0x3f000000;
        let initial = match k {
            1 => Value::Bool(0),
            2 => Value::Fixed(start),
            3 => Value::Int(start),
            5 => Value::Point3([start; 3]),
            _ => panic!(),
        };
        let input = match k {
            1 => Value::Bool(r["input"].as_u64().unwrap() as u8),
            2 => Value::Fixed(r["input"].as_u64().unwrap() as u32),
            3 => Value::Int(r["input"].as_u64().unwrap() as u32),
            5 => Value::Point3(std::array::from_fn(|i| {
                r["input"][i].as_u64().unwrap() as u32
            })),
            _ => panic!(),
        };
        let mut s = Store {
            entries: vec![Entry {
                name: "lab".into(),
                value: initial,
                flags,
                bounds: if k == 2 {
                    [0xbf800000, 0x3f800000]
                } else {
                    [0xfffffffe, 2]
                },
            }],
        };
        let result = s.assign("lab", input);
        if k == 2 && flags == 32 && !f32::from_bits(r["input"].as_u64().unwrap() as u32).is_finite()
        {
            assert!(result.is_err());
            continue;
        }
        result.unwrap();
        let actual = match s.get("lab") {
            Value::Bool(v) => vec![u32::from(*v)],
            Value::Fixed(v) | Value::Int(v) => vec![*v],
            Value::Point3(v) => v.to_vec(),
            _ => panic!(),
        };
        let expected: Vec<_> = r["output"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u32)
            .collect();
        assert_eq!(actual, expected, "{r}");
    }
}
#[test]
#[ignore = "requires actual AA_GAME_ROOT and source-bound current SVar owners"]
fn actual_current_source_rules_and_positive_sync_use_same_cells() {
    use crate::{
        catalog::Catalog,
        combat_source_receipt::VerifiedStartupReceipt,
        load_view_variables::{NativeOverrides, State},
    };
    use aa_orders::net::sync::Synchronizer;
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings =
        Some(crate::settings::Settings::load_v14(&mut sources, Limits::default()).unwrap());
    let proof = VerifiedStartupReceipt::verify_embedded_reference(&catalog, &mut sources).unwrap();
    crate::combat_source_completion::install(
        &catalog,
        &mut sources,
        proof.context(),
        proof.csv(),
        proof.globals(),
    )
    .unwrap();
    crate::combat_source_completion::compute(&catalog).unwrap();
    catalog.native_load_mode = Some(1);
    let mut source_store = Store::constructors();
    apply_source_files(&catalog, proof.context().precision(), &mut source_store).unwrap();
    let source = snapshot(&source_store);
    let mut state =
        State::reset_from_fresh_source(&catalog, NativeOverrides::ControlledNone).unwrap();
    assert_eq!(state.registrations().count(), 231);
    assert!(state.supply_provide_under_siege());
    assert_eq!(state.fog_integer(), 1);
    let reference: Json = serde_json::from_str(include_str!(
        "../../../research/development10/world-ready-7757f0/svar-registration-reference.json"
    ))
    .unwrap();
    for (actual, expected) in state
        .registrations()
        .zip(reference["entries"].as_array().unwrap())
    {
        assert_eq!(actual.name(), expected["name"].as_str().unwrap());
        assert_eq!(
            actual.ordinal(),
            expected["ordinal"].as_u64().unwrap() as u32
        );
        assert_eq!(actual.kind(), expected["kind"].as_u64().unwrap() as u32);
    }
    let inventory = crate::inactive_svar_dump::Inventory::from_captured_source(&catalog).unwrap();
    let mut cases = Vec::new();
    for name in [
        "Campaign/tutorial02.RMP",
        "DeveloperMaps/(3v3)Firefight.RMP",
        "(2v2)Berlin.RMP",
    ] {
        let map = aa_mapworld::MapArchive::from_bytes(
            std::fs::read(
                game.join("reconstruction/assets/original/Data/Maps")
                    .join(name),
            )
            .unwrap(),
        )
        .unwrap();
        let map_inventory = aa_mapworld::WorldInventory::decode(&map, u64::MAX).unwrap();
        let rules = crate::load_world_rules::Owner::load(&catalog, &map_inventory).unwrap();
        state.publish_loaded_rules(&catalog, &rules).unwrap();
        let mut sync = Synchronizer::new(10000);
        sync.enabled = true;
        let calls = std::cell::Cell::new(0usize);
        let mut gate = || {
            calls.set(calls.get() + 1);
            Ok(true)
        };
        {
            let bank = catalog.definition_bank.borrow();
            let mut binding = crate::live_clocked::SyncLog {
                owner: &mut sync,
                gate: &mut gate,
            }
            .into();
            inventory
                .synchronize_current(&catalog, &bank, &state, &mut binding)
                .unwrap();
        }
        let first = sync.hash;
        assert!(calls.get() > 460);
        assert!(sync.end() > sync.begin());
        let index = state
            .registrations()
            .position(|r| r.name() == "CheatFog")
            .unwrap();
        let address = state.cheat_fog_word() as *const u8;
        state.assign_cheat_fog(128);
        assert_eq!(address, state.cheat_fog_word() as *const u8);
        assert!(
            matches!(state.current(index).unwrap(),crate::load_view_variables::CurrentValue::Bool(v)if *v==128)
        );
        let mut changed = Synchronizer::new(10000);
        changed.enabled = true;
        {
            let bank = catalog.definition_bank.borrow();
            let mut binding = crate::live_clocked::SyncLog {
                owner: &mut changed,
                gate: &mut gate,
            }
            .into();
            inventory
                .synchronize_current(&catalog, &bank, &state, &mut binding)
                .unwrap();
        }
        assert_ne!(changed.hash, first);
        state.assign_cheat_fog(0);
        cases.push(serde_json::json!({"map":name,"rules":rules.records().map(|r|serde_json::json!({"name":r.ids(),"kind":r.kind(),"default":r.default_value(),"value":r.value(),"enum_count":r.enum_values().len()})).collect::<Vec<_>>(),"current":snapshot(state.store_for_evidence()),"sync_hash":first,"changed_hash":changed.hash}));
    }
    //The SAME canonical source/current owner survives the real Bank move.
    let bank = std::mem::take(&mut *catalog.definition_bank.borrow_mut());
    state.validate_in_bank(&catalog, &bank).unwrap();
    assert!(state.validate_catalog(&catalog).is_err());
    let mut sync = Synchronizer::new(10000);
    sync.enabled = true;
    let mut gate = || Ok(true);
    let mut binding = crate::live_clocked::SyncLog {
        owner: &mut sync,
        gate: &mut gate,
    }
    .into();
    inventory
        .synchronize_current(&catalog, &bank, &state, &mut binding)
        .unwrap();
    let output = serde_json::json!({"source":source,"reset":snapshot(&from_fresh_source(&catalog,proof.context().precision()).unwrap()),"cases":cases});
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261017-current-svars/actual-current-inputs.json");
    if std::env::var_os("AA_CANONICAL_SVARS_CAPTURE").is_some() {
        use std::io::Write;
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap()
            .write_all(&(serde_json::to_string_pretty(&output).unwrap() + "\n").into_bytes())
            .unwrap();
    } else {
        let expected: Json = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(output, expected);
    }
}
fn snapshot(store: &Store) -> Json {
    serde_json::json!(store.entries.iter().enumerate().map(|(i,e)|{let words=match &e.value{Value::Bool(v)=>vec![u32::from(*v)],Value::Fixed(v)|Value::Int(v)=>vec![*v],Value::Point3(v)=>v.to_vec(),Value::CString(_)=>Vec::new()};serde_json::json!({"ordinal":i,"name":e.name,"kind":e.value.kind(),"flags":e.flags,"bounds":e.bounds,"words":words,"text":match &e.value{Value::CString(v)=>Some(v),_=>None}})}).collect::<Vec<_>>())
}
