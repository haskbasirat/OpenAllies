//! Isolated portable metadata check using the ACTUAL unmodified source module.
//! No full World/native constructor or frame-completion claim is made here.
//! Private crate constructor access belongs only to this research harness;
//! the production crate exports no scalar/raw World-token constructor.
#[path = "../../../crates/aa-runtime/src/world_revision.rs"]
mod world_revision;

fn main() {
    let mut first = world_revision::Counter::fresh_application();
    let actual = first.construct();
    let witness = actual.witness();
    let shared_metadata = witness.clone();
    assert!(witness.is_for(&actual));
    assert!(shared_metadata.is_for(&actual));
    let mut foreign = world_revision::Counter::fresh_application();
    let same_scalar = foreign.construct();
    assert_eq!(actual.value(), same_scalar.value());
    assert!(!witness.is_for(&same_scalar));
    let replacement = first.construct();
    assert!(!witness.is_for(&replacement));
    drop(actual);
    assert!(witness.owner_identity().upgrade().is_none());
    assert!(shared_metadata.owner_identity().upgrade().is_none());
    assert!(!witness.is_for(&replacement));
    assert!(!witness.is_for(&same_scalar));
    println!("actual-source Witness::is_for: same/metadata-clone/foreign-same-value/replaced/destroyed checks passed");
}
