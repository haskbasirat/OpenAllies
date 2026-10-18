//!4D9A40's selected-command diagnostic on the retained A=[B3655C] owner.
//! The OS simulation-thread/application-flags gate is required and read lazily
//! inside each reached enabled/not-suspended logger. It has no default value.
use aa_orders::{Error, Position, Result, net::sync::Synchronizer};
use aa_runtime::mind_world::Commands;
use std::cell::RefCell;

fn emit(
    log: &RefCell<Synchronizer>,
    gate: &mut impl FnMut() -> Result<bool>,
    effect: impl FnOnce(&mut Synchronizer, bool),
) -> Result<()> {
    let mut owner = log
        .try_borrow_mut()
        .map_err(|_| Error::SchemaContext("same native A synchronizer borrow"))?;
    if !owner.enabled || owner.suspend >= 1 {
        return Ok(());
    }
    let open = gate()?;
    effect(&mut owner, open);
    Ok(())
}
fn command_error(e: impl std::fmt::Display) -> Error {
    Error::Handler {
        index: 86,
        position: Position::PACKET_START,
        message: e.to_string(),
    }
}
///77DC50 marker, then optional77D930 float bits and77DA70 integer flags.
/// Command fields are reread only after earlier effects, in native order.
/// The gate's supported domain is read-only OS/application inputs, not
/// arbitrary reentrant mutation of this borrowed synchronizer/command arena.
pub fn selected_command(
    log_a: &RefCell<Synchronizer>,
    commands: &Commands,
    command: Option<u32>,
    gate: &mut impl FnMut() -> Result<bool>,
) -> Result<()> {
    emit(log_a, gate, |log, open| log.log_marker(open, "_command"))?;
    if let Some(command) = command {
        let bits = commands.value(command).map_err(command_error)?.scalar_bits;
        emit(log_a, gate, |log, open| {
            log.log_float_bits(open, "_command->m_value", bits)
        })?;
        let flags = commands.value(command).map_err(command_error)?.flags;
        emit(log_a, gate, |log, open| {
            log.log_int(open, "_command->m_bits", flags)
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use aa_orders::net::sync::EntryKind;
    use aa_runtime::mind_world::PlainCommand;
    fn command(commands: &mut Commands) -> u32 {
        commands
            .allocate(PlainCommand {
                definition_id: 0,
                definition: None,
                flags: 0x5c,
                word10: 0,
                object_definition: None,
                position_bits: [0; 2],
                auxiliary_bits: [0; 2],
                scalar_bits: 0x7fc12345,
                parameter_definition: None,
                extra_definition: None,
                boolean: 0,
                trailing: 0,
            })
            .unwrap()
    }
    #[test]
    fn command_scope_marker_and_float_bits_preserve_the_same_a_owner_and_prefix() {
        let mut commands = Commands::default();
        let c = command(&mut commands);
        let a = RefCell::new(Synchronizer::new(65536));
        a.borrow_mut().reset(0, 17);
        let before = a.as_ptr();
        let mut calls = 0;
        let e = selected_command(&a, &commands, Some(c), &mut || {
            calls += 1;
            if calls == 3 {
                Err(Error::SchemaContext("required actual OS gate"))
            } else {
                Ok(true)
            }
        })
        .unwrap_err();
        assert!(e.to_string().contains("required actual OS gate"));
        assert_eq!(calls, 3);
        assert_eq!(a.as_ptr(), before);
        let owner = a.borrow();
        assert_eq!(owner.end(), 3);
        assert_eq!(owner.entry(1).unwrap().kind, EntryKind::Empty);
        assert_eq!(owner.entry(1).unwrap().label, "_command");
        assert_eq!(owner.entry(2).unwrap().kind, EntryKind::Float);
        assert_eq!(owner.entry(2).unwrap().value, 0x7fc12345);
        assert_eq!(
            owner.hash,
            aa_orders::net::sync::step(aa_orders::net::sync::step(17, 0), 0x7fc12345)
        );
        assert_eq!(commands.references(c).unwrap(), 1);
    }
    #[test]
    fn disabled_and_suspended_owners_skip_only_their_gate_reads() {
        let mut commands = Commands::default();
        let c = command(&mut commands);
        let a = RefCell::new(Synchronizer::new(65536));
        selected_command(&a, &commands, Some(c), &mut || {
            panic!("native disabled logger skips OS gate")
        })
        .unwrap();
        a.borrow_mut().enabled = true;
        a.borrow_mut().suspend = 1;
        selected_command(&a, &commands, Some(c), &mut || {
            panic!("native suspended logger skips OS gate")
        })
        .unwrap();
        assert_eq!(a.borrow().hash, -1);
        // The command field read still follows the disabled marker call.
        assert!(selected_command(&a, &commands, Some(u32::MAX), &mut || panic!()).is_err());
    }
    #[test]
    fn null_command_still_logs_its_marker_and_a_stale_command_fails_after_it() {
        let commands = Commands::default();
        let a = RefCell::new(Synchronizer::new(65536));
        a.borrow_mut().reset(0, 0);
        let mut calls = 0;
        selected_command(&a, &commands, None, &mut || {
            calls += 1;
            Ok(true)
        })
        .unwrap();
        assert_eq!(calls, 1);
        assert_eq!(a.borrow().end(), 2);
        assert!(selected_command(&a, &commands, Some(17), &mut || Ok(true)).is_err());
        assert_eq!(a.borrow().end(), 3);
    }
    #[test]
    fn all_native_command_sync_calls_and_required_prefixes_match() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../research/skirmish-20261016-player-orders/decree/native-command-sync.json");
        let doc: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        let mut checked = 0;
        for row in doc["rows"].as_array().unwrap() {
            let mut commands = Commands::default();
            let id = command(&mut commands);
            commands.value_mut(id).unwrap().flags = row["input"]["flags"].as_u64().unwrap() as u32;
            commands.value_mut(id).unwrap().scalar_bits =
                row["input"]["value"].as_u64().unwrap() as u32;
            let log = RefCell::new(Synchronizer::new(65536));
            log.borrow_mut().reset(0, 0);
            let selected = (row["input"]["command"] == true).then_some(id);
            let stages = ["marker", "value", "flags"];
            let mut calls = 0;
            let mut status = "complete";
            let result = selected_command(&log, &commands, selected, &mut || {
                let stage = stages[calls];
                calls += 1;
                if row["input"]["stop"].as_str() == Some(stage) {
                    status = stage;
                    Err(Error::SchemaContext(stage))
                } else {
                    Ok(true)
                }
            });
            assert_eq!(status, row["output"]["status"], "{row}");
            assert_eq!(result.is_ok(), status == "complete");
            assert_eq!(calls, row["output"]["events"].as_array().unwrap().len());
            let stored = if result.is_ok() { calls } else { calls - 1 };
            let log = log.borrow();
            assert_eq!(log.end(), 1 + stored as i32);
            for (i, expected) in row["output"]["events"]
                .as_array()
                .unwrap()
                .iter()
                .take(stored)
                .enumerate()
            {
                let actual = log.entry(i as i32 + 1).unwrap();
                assert_eq!(actual.label, expected["label"]);
                assert_eq!(u64::from(actual.value), expected["value"]);
                assert_eq!(
                    actual.kind,
                    match expected["kind"].as_str().unwrap() {
                        "marker" => EntryKind::Empty,
                        "value" => EntryKind::Float,
                        "flags" => EntryKind::Int,
                        _ => panic!(),
                    }
                );
            }
            checked += 1;
        }
        assert_eq!(checked, 432);
    }
}
