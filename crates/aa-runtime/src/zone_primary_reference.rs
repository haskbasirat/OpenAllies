//!45BC70's portable identity result; the caller resolves the SAME registry.
//! The logical GTID denotes a live allocation, never a fabricated pointer.
pub(crate) fn resolve(reference: u32, live: impl FnOnce(u32) -> bool) -> u32 {
    if live(reference) { reference } else { 0 }
}
