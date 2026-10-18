DENIZEN ACTION CONTRIBUTOR / SAVED OWNER BINDING -- 2026-10-16

Scope
  Native Denizen569C70 dispatched by Actor4D1350. This is an action-mask
  contributor, not Denizen command execution, world publication, or a battle.
  Original game files, release files, saves and profiles are unchanged.

Native references
  569C70..569E3E contributor
  4CED40 Settlement-presence/self or virtual+100 parent
  4CED60 Structure+10, then Wall58CB30 (nonnull Wall reference reread)
  58CB30 Wall+10 getter
  517FA0 action bit setter
  Pseudocode and actual instructions checked; disassemblies retained here.

Production
  crates/aa-actors/src/denizen_actions.rs
    Explicit owned-storage World; contribute(component-holder ActorId).
    Native state30 clears23,24,25,26,27,28 and returns before parent/flags.
    Parent absence clears23/24; parent presence and flags38 bit3 enable
    byte20==0 / byte20!=0 writes. Bit7 gates byte21==0 / !=0 bits27/28.
    Bits25/26 always written, requiring both flags38 and flags3C bit4/6.
    Missing capability flags leave23/24 and27/28 unchanged where native does.
    Full byte truth is used, including7/128/255; high word is preserved.
  crates/aa-runtime/src/saved_denizen_actions.rs
    Reads the actual RestoredComponent::Denizen allocation/header/bytes/words.
    Settlement only needs presence here; no Settlement payload read is made.
    Structure parent uses actual object_handle_10. Present Wall remains an
    explicit unsupported boundary because no concrete saved Wall payload exists.
    No cloned/replacement Denizen, source defaults, new rules or dependencies.
  Shared saved_organization_actions.rs forwarder is owned by restore_checkpoint.

Proof
  native_denizen_actions.py executes unchanged original and v14 code with no
  gameplay seams. Page-zero guard from phase8-economy/harness.py is retained.
  native-denizen-actions.json:2,154 executions, original/v14 x PC24/53/64,
  all256 low-byte capability flags, flag3C combinations, raw boolean bytes,
  state30, varied64-bit masks/states, Settlement/Structure/Wall precedence.
  Function windows and actual executable hashes are pinned in each JSON.
  The observable low-word action-mask writes and helper call ordering are
  replayed in crates/aa-actors/tests/native_denizen_actions.rs (two tests).
  Native sometimes writes the unchanged high word before its low word; Rust
  updates the combined64-bit mask. There are no intervening callbacks. Raw
  individual memory-store ordering is not claimed beyond the observable mask.

  actual-saved-inputs.json captures two actual tutorial01 saved Denizen owners
  (rus_corps ID0x4C4, ger_corps ID0x1136), with actual action/state masks,
  bytes20/21, words38/3C and parent-component facts. This uses InspectPartial
  only as a diagnostic source; it does not prove the map can be published.
  native-saved-denizen-actions.json:12 further original/v14/three-PC executions.
  Runtime ignored test rechecks every captured input and every native result,
  preserving the actual Denizen allocation. Its unused Organization context
  fixture is explicit and never queried by Denizen; it is not a runtime owner.

Commands (workspace root; AA_GAME_ROOT points to parent game folder)
  python research/skirmish-20261016-denizen-actions/native_denizen_actions.py
  CARGO_TARGET_DIR=target/s-denizen-actions cargo test -p aa-actors
    --test native_denizen_actions --release --locked --offline -j4
  AA_GAME_ROOT=... CARGO_TARGET_DIR=target/s-denizen-actions cargo test
    -p aa-runtime --lib saved_denizen_actions::tests --release --locked
    --offline -j4 -- --include-ignored
  To recapture source facts only: set AA_DENIZEN_CAPTURE=1 for the runtime
  command, then unset it; run generator --saved; rerun the mandatory comparison.

Limits
  No command execution, Denizen refresh/tick, complete world startup, battle,
  multiplayer or release EXE was supplied/tested. No engine cap is encountered
  or removed in these integer flag contributors. Invalid native raw pointers,
  allocation faults and exact malformed-storage read/fault timing are unclaimed.
  All new errors remain errors after completed action-mask prefix effects.
  Strict Clippy initially stops on pre-existing recruit_actions:77 collapsible_if
  and actor_creation:716 doc_lazy_continuation; scoped Clippy allows only those
  two named lints and passes the aa-actors native_denizen_actions target.
  Runtime Clippy then stops in out-of-scope aa-rules/native_load.rs:878 on
  manual_map; its result is blocked, not passed (clippy-runtime.log).
  No out-of-scope source was edited.

Backup
  backups/before-denizen-actions-20261016-172026/ contains both module-export
  files before append. New files are pinned in findings.json after validation.
