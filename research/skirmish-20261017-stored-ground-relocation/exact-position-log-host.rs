use aa_spatial::{Error,Result,position::Diagnostic};
fn fail(e:impl std::fmt::Display)->Error{Error::World(e.to_string())}
mod live_clocked{use std::cell::RefCell;use aa_orders::net::sync::Synchronizer;
/// The same process Synchronizer A/B3655C. The actual thread/application gate
/// is evaluated only after its enabled and suspend checks have passed.
pub struct SyncLog<'a> {
    pub owner: &'a mut Synchronizer,
    pub gate: &'a mut dyn FnMut() -> Result<bool, String>,
}
/// The direct API is retained for a standalone owner. A live frame can instead
/// loan the one retained SyncA cell to both NetworkSeams and world callbacks.
pub enum SyncBinding<'a> {
    Direct(SyncLog<'a>),
    Shared {
        owner: &'a RefCell<Synchronizer>,
        gate: &'a mut dyn FnMut() -> Result<bool, String>,
    },
}
}
pub fn log_position(
    sync: &mut crate::live_clocked::SyncBinding<'_>,
    value: Diagnostic,
) -> Result<()> {
    fn emit(
        owner: &mut aa_orders::net::sync::Synchronizer,
        gate: &mut dyn FnMut() -> std::result::Result<bool, String>,
        value: Diagnostic,
    ) -> Result<()> {
        let open = if owner.enabled && owner.suspend < 1 {
            gate().map_err(fail)?
        } else {
            false
        };
        match value {
            Diagnostic::Identity(v) => owner.log_int(open, "(int)GetID()", v),
            Diagnostic::X(v) => owner.log_float_bits(open, "GetPosition().x", v),
            Diagnostic::Y(v) => owner.log_float_bits(open, "GetPosition().y", v),
        };
        Ok(())
    }
    match sync {
        crate::live_clocked::SyncBinding::Direct(log) => emit(log.owner, log.gate, value),
        crate::live_clocked::SyncBinding::Shared { owner, gate } => {
            let mut same = owner.try_borrow_mut().map_err(fail)?;
            emit(&mut same, *gate, value)
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use aa_orders::net::sync::{EntryKind, Synchronizer};
    #[test]
    fn position_sync_keeps_same_a_and_native_labels_error_prefix() {
        let mut a = Synchronizer::new(64);
        a.reset(0, 17);
        let original = &a as *const _;
        let mut calls = 0;
        let mut gate = || {
            calls += 1;
            if calls == 2 {
                Err("required current thread/SAI gate".into())
            } else {
                Ok(true)
            }
        };
        let mut sync = crate::live_clocked::SyncBinding::Direct(crate::live_clocked::SyncLog {
            owner: &mut a,
            gate: &mut gate,
        });
        log_position(&mut sync, Diagnostic::Identity(0x20007)).unwrap();
        assert!(log_position(&mut sync, Diagnostic::X(0x80000000)).is_err());
        drop(sync);
        assert_eq!(original, &a as *const _);
        assert_eq!(calls, 2);
        assert_eq!(a.end(), 2);
        let entry = a.entry(1).unwrap();
        assert_eq!(entry.label, "(int)GetID()");
        assert_eq!(entry.kind, EntryKind::Int);
        assert_eq!(entry.value, 0x20007);
    }
    #[test]
    fn disabled_position_logs_do_not_read_gate_and_shared_a_keeps_float_bits() {
        let cell = std::cell::RefCell::new(Synchronizer::new(64));
        let before = cell.borrow().end();
        let mut gate =
            || -> std::result::Result<bool, String> { panic!("disabled native SyncA skips gate") };
        {
            let mut sync = crate::live_clocked::SyncBinding::Shared {
                owner: &cell,
                gate: &mut gate,
            };
            log_position(&mut sync, Diagnostic::X(0x80000000)).unwrap();
        }
        assert_eq!(cell.borrow().end(), before);
        cell.borrow_mut().reset(0, 19);
        let mut gate = || Ok(true);
        let mut sync = crate::live_clocked::SyncBinding::Shared {
            owner: &cell,
            gate: &mut gate,
        };
        log_position(&mut sync, Diagnostic::X(0x80000000)).unwrap();
        log_position(&mut sync, Diagnostic::Y(0x3f800000)).unwrap();
        drop(sync);
        let a = cell.borrow();
        assert_eq!(a.entry(1).unwrap().label, "GetPosition().x");
        assert_eq!(a.entry(1).unwrap().value, 0x80000000);
        assert_eq!(a.entry(2).unwrap().label, "GetPosition().y");
        assert_eq!(a.entry(2).unwrap().value, 0x3f800000);
    }
}
