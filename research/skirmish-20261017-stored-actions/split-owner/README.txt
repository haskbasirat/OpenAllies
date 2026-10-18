Synchronous stored4D1350 split-owner binding, 2026-10-17

New additive entry:
  stored_actor_actions::rebuild_on_owners(
    catalog,&mut ActorAccess,&mut Arena,&mut Bank,key,
    &WorldInventory,&Spatial,Option<&World.variables>,livePC)

The caller lends paired current ActorAccess/Arena owners from one ActorStore.
This entry does not create an arena, clone components, remove a Mind, restore
saved shells, copy back a detached entity or run a second4D1350 coordinator.
The active Mind core must remain installed while the synchronous callback runs.

Existing rebuild/rebuild_with_variables wrappers split_minds once and forward.
The same Bound now holds those split loans; all Actor/component/source fields,
source/live precision guards, ordered contributor kernels, action publication
and exact required errors are unchanged. Mind contribution reborrows the same
Arena/ActorAccess into the existing mind_actions World.

The actual saved-prefix regression repeats all9 old native contexts through
both the Store wrapper and the already-split entry. It resets only the explicit
incoming action words between calls. Same Arena, ability-map, entity and
component pointers are checked, and both results/masks must match. The original
actual-inputs-v3/native-stored-actions-v3 fixtures remain unchanged. This is
direct4D1350 proof over saved-prefix allocations; it does not independently
prove a fresh Mind InitExisting/reset/enter callback or a complete mission.

Native immutable replay:9 contexts/18 original-v14 machines/924 executedpins,
no required cutoffs. This factoring changes storage loans, not native logic.
Production library check passed18.56s, test target typecheck passed17.81s.
Final current-source metadata check passed34.84s (check-final-source.log).
Single external actual-source runtime replay passed1/1 in14.22s (build2m42s),
replay-test.log. The18 Rust calls are9 wrapper plus9 already-split calls,
compared to the unchanged9 native contexts. No new native behavior is claimed.
No full aa-runtime unit-test link was started. All other unbound positive
services and prior findings scope remain in force.
