WORLD REVISION / CURRENT APPLICATION SAI OWNER — 2026-10-17

native_world_revision.py executes original4F8072..4F809C before the log call.
24 inputs /48 original-v14 executions cover all3 explicit PCs and u32 rollover.
Both verified PEs locate B3CC94 in the zero-filled virtual section tail:
section RVA721000, raw16000, virtual2B5C4, wordoffset1BC94. The initial zero is
therefore a PE-loader fact, not an inferred per-map default.
Immutable fixture SHA256:
863cdec91eb8e876938370df09785ead67afb11aeaa9672d2ced556e36e5c442
11 executed instruction addresses include the stopped log-call boundary.
This is a bounded field proof, not execution of the complete World ctor.

The non-Clone application Counter is retained by ScopedManagedLoad/InactiveLoad
and moves with the same SAI wrapper. Each constructed World retains a private
ConstructedWorldRevision. It is not recreated from a map ID or saved tick.
restore_lifecycle now owns the actual aa_mind::sai::manager::Sai created by the
existing proved7A0AA0 constructor kernel. The previous separate active byte,
placeholder lists and placeholder fibers were removed. with_startup_engine
retires the inactive load capability before its fallible callback; partial
mutations and the original synchronization allocation survive in StartupOwner.
StartupOwner certifies ownership only, including after an error, not readiness.

Accepted Rust log:
qa-output/development10/root-sai-revision-tests-02.log
5 lifecycle tests (including actual original/v14 UVars) +1 revision replay PASS.
The current strict map integration with retained SAI Map::load still reaches
112 fully completed lifecycles before2696 Detection After; no battle claim.

Reproduce from rust-rebuild:
  python research/development10/world-revision/native_world_revision.py
Set AA_GAME_ROOT and use the project's locked/offline targeted Cargo workflow.
Never use cargo test --workspace as a shortcut for these native port checks.
