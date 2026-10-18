//! Compile-time ownership contract for a resource-worker result. Send permits
//! one owned move; it does not authorize simulation work on another thread.
fn require_send<T: Send + 'static>() {}

#[test]
fn stopped_load_owners_can_move_once_between_os_threads() {
    require_send::<aa_runtime::restoration::Attempt>();
    require_send::<aa_runtime::restoration::Staged>();
    require_send::<aa_runtime::restore_lifecycle::InactiveLoad>();
    require_send::<aa_runtime::restore_lifecycle::StartupOwner>();
}
