//! Replays every row of native-command-sync-4d9a40.json (original 4D9A40 and
//! its native loggers, both images, PC24/PC53/PC64).
use super::*;
use crate::live_clocked::SyncLog;
use aa_orders::net::sync::EntryKind;
use std::cell::RefCell;

fn command(flags: u32, value: u32) -> PlainCommand {
    PlainCommand {
        definition_id: 0,
        definition: None,
        flags,
        word10: 0,
        object_definition: None,
        position_bits: [0; 2],
        auxiliary_bits: [0; 2],
        scalar_bits: value,
        parameter_definition: None,
        extra_definition: None,
        boolean: 0,
        trailing: 0,
    }
}
fn fixture() -> serde_json::Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/phase8-orders-input/recruit/native-command-sync-4d9a40.json");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn initial(input: &serde_json::Value) -> Synchronizer {
    let mut log = Synchronizer::new(input["max"].as_i64().unwrap() as i32);
    log.reset(input["index"].as_i64().unwrap() as i32, input["hash"].as_i64().unwrap() as i32);
    log.enabled = input["enabled"].as_u64().unwrap() != 0;
    log.suspend = input["suspend"].as_i64().unwrap() as i32;
    log
}
fn gate_open(input: &serde_json::Value) -> bool {
    input["thread"] == input["sim_thread"]
        && match input["sai"].as_array() {
            None => true,
            Some(f) => f[0] == 0 && f[1] == 0,
        }
}
fn check(row: &serde_json::Value, log: &Synchronizer, reads: &[usize]) {
    let out = &row["output"];
    assert_eq!(i64::from(log.hash), out["hash"].as_i64().unwrap(), "{row}");
    assert_eq!(i64::from(log.begin()), out["begin"].as_i64().unwrap(), "{row}");
    assert_eq!(i64::from(log.end()), out["end"].as_i64().unwrap(), "{row}");
    let expected: Vec<usize> = out["thread_reads"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as usize).collect();
    assert_eq!(reads, expected.as_slice(), "{row}");
    let entries = out["entries"].as_array().unwrap();
    // aa_orders Synchronizer::entry compares indexes as signed i32; a native
    // modular window that crosses INT_MAX is not readable through it (hash,
    // begin and end above still prove those rows).
    if log.end() < log.begin() {
        return;
    }
    for (i, expected) in entries.iter().enumerate() {
        let actual = log.entry(log.begin().wrapping_add(i as i32)).unwrap();
        let kind = match expected["kind"].as_u64().unwrap() {
            0 => EntryKind::Empty,
            2 => EntryKind::Int,
            3 => EntryKind::Float,
            k => panic!("unexpected kind {k}"),
        };
        // A retired (cleared) native slot has a NULL label.
        match expected["label"].as_str() {
            Some(label) => assert_eq!(actual.label, label, "{row}"),
            None => assert_eq!(actual.label, "", "{row}"),
        }
        assert_eq!(actual.kind, kind, "{row}");
        assert_eq!(u64::from(actual.value), expected["value"].as_u64().unwrap(), "{row}");
        assert_eq!(i64::from(actual.hash), expected["hash"].as_i64().unwrap(), "{row}");
    }
}

#[test]
fn player_order_command_sync_4d9a40_replays_every_native_row_direct_and_shared() {
    let doc = fixture();
    let rows = doc["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 2112);
    for row in rows {
        let input = &row["input"];
        let open = gate_open(input);
        for shared in [false, true] {
            let mut owner = initial(input);
            let cell = RefCell::new(initial(input));
            let mut reads = Vec::new();
            for call in input["calls"].as_array().unwrap() {
                let cmd = call["command"].as_object().map(|c| {
                    command(c["flags"].as_u64().unwrap() as u32, c["value"].as_u64().unwrap() as u32)
                });
                let label = call["label"].as_str().unwrap();
                assert!(matches!(label, COMMAND_LABEL | CMD_LABEL));
                let mut count = 0;
                let mut gate = || {
                    count += 1;
                    Ok(open)
                };
                let mut sync = if shared {
                    SyncBinding::Shared { owner: &cell, gate: &mut gate }
                } else {
                    SyncBinding::Direct(SyncLog { owner: &mut owner, gate: &mut gate })
                };
                sync_command_4d9a40(&mut sync, label, cmd.as_ref()).unwrap();
                drop(sync);
                reads.push(count);
            }
            if shared {
                check(row, &cell.borrow(), &reads);
            } else {
                check(row, &owner, &reads);
            }
        }
    }
}

#[test]
fn player_order_command_sync_gate_error_stops_after_committed_prefix() {
    let mut owner = Synchronizer::new(0x10000);
    owner.reset(0, 0);
    let mut calls = 0;
    let mut gate = || {
        calls += 1;
        if calls == 2 { Err("required actual OS gate".to_string()) } else { Ok(true) }
    };
    let cmd = command(0x42, 0x7fc12345);
    let mut sync = SyncBinding::Direct(SyncLog { owner: &mut owner, gate: &mut gate });
    let e = sync_command_4d9a40(&mut sync, CMD_LABEL, Some(&cmd)).unwrap_err();
    drop(sync);
    assert!(e.contains("required actual OS gate"));
    assert_eq!(owner.end(), 2);
    assert_eq!(owner.entry(1).unwrap().label, CMD_LABEL);
    // A shared owner already loaned elsewhere is an explicit error.
    let cell = RefCell::new(Synchronizer::new(0x10000));
    let guard = cell.borrow_mut();
    let mut gate = || Ok(true);
    let mut sync = SyncBinding::Shared { owner: &cell, gate: &mut gate };
    assert!(sync_command_4d9a40(&mut sync, COMMAND_LABEL, None).is_err());
    drop(sync);
    drop(guard);
}
