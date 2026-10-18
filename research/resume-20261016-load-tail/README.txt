POSTLOAD CAMERA ADAPTER — 2026-10-16

This is a bounded integration step, not a running game or a publishable world.
No native game files, saves, old release files, or shared restoration source
were changed.

CURRENT AUDIT

aa-runtime restoration::Attempt::into_staged already moves the restored
registry, actual component boxes, Mind arena, Ability arena, saved queue
identities, clock and RNG into Staged. Staged::post_load loads saved queues,
calls aa_actors::post_load::post_load_pass in GTID slot order, and marks the
visited saved records PostLoaded. The source README records 18 maps finishing
this phase in round4. That is historical evidence, not a fresh test result.
The older unsupported-inventory.json still records round3 (13 clean maps).

Staged::init_for_loaded_map still returns Unported. No runtime code sets
runtime_publication_ready to true. SUN and MISC_DATA loading, live membership
and host/session state, full world start, conversion creation, spatial/world
publication and battle execution remain separate required work.

EXISTING NATIVE PORTS REUSED

aa_actors::world_load_tail::after_post_load:
  4FD3F3..4FD47C, before conversion replacement.
  Gate on [GWorld+28]+A8 and 59E520 -> 4F1E10.
  Local kingdom 4F5290 / 4F48E0 / 76AC60 / 76AC70 / 765E00.
  Camera selection 4F1C20 (4CE6C0 organization lookup).
  GWorld +22C/+230 writes, then 404F60 log arguments.
Oracle: research/phase8-saved-components/native-load-tail.json (24 cases).
The existing aa-actors test covers the ported control flow with fixture seams.

aa_economy::world_start::world_start:
  4F1E10 outer traversal, statistics promotion, actor +13C, snapshot timing.
Its oracle EXPLICITLY SEAMS 58F010 statistics tracking, 58F530 snapshot
construction, and 45F240 list insertion. The round5 saved-components note
saying the port covers all of 4F1E10 must not be read as full effect parity.
Those effects have no runtime binding here. The new adapter refuses this
branch before attempting world start; that refusal is not native fault parity.

NEW CODE

crates/aa-runtime/src/load_tail_world.rs
crates/aa-runtime/tests/load_tail_world.rs
One pub mod line appended in aa-runtime/src/lib.rs.

The adapter borrows ActorStore and reads its current names, placement flags,
position, component presence, Element organization identity, canonical
definition identity, and SpecialShell capital identity. It does not retain
another mutable pose or component snapshot. Capital +1F8 must be a typed
identity reference, or the proven constructor zero word; a nonzero unresolved
native word is refused instead of being reinterpreted as a portable GTID.

The wrapper requires every staged record to have completed PostLoad and no
saved failure. PostLoad-created fresh objects remain unsupported until their
creation/publication path is bound. This is a Rust stage precondition, not an
extra native gameplay rule. The existing native function controls read order,
mode gates, strict priority comparisons, and camera-before-log ordering.

INTEGRATION CONTRACT (LEAD OWNS restoration.rs)

Call load_tail_world::World::after_post_load only after successful
Staged::post_load. Supply actual values for:

  SessionFacts.application_state: [B36550]+E4.
  SessionFacts.local_player: [B36558]+8, with name +24 and Kingdom +28.
  WorldFacts.map_settings_mode: [GWorld+28]+A8. This is NOT MAP_HEADER
    WorldRules.mode; the adapter deliberately does not infer one from the other.
  WorldFacts.world_start_flag: exact 59E520 result, if reached by mode 1/4.
  WorldFacts.world_word_48: GWorld +48 (WorldInventory.world.word_48 for
    saves that contain it; older versions need the actual constructor value).
  WorldFacts.map_size_bits: actual terrain +C/+10, not display dimensions.
  WorldFacts.kingdom_slots: GWorld +148/+14C, preserving null slots/order.
    This is NOT the different +154/+158 major-Kingdom list.
  WorldFacts.kingdom_actors: Kingdom +284 membership in native head order.
    Do not reconstruct this order from sorted GTIDs or owner scans.
  WorldFacts.definition_priorities: proven ThingData +1B4 per canonical
    definition identity. Its source producer is not supplied by this module.
  CameraState: the caller-owned camera words and log argument list.

Missing optional facts or table entries are errors when the native path reads
them. They are not replaced with empty/zero values. Native app_state=3 can
return the map centre without reading the session/Kingdom fields; this mode
must be explicitly chosen by the real host, never used as a missing-data fix.

A success means ONLY that the bounded camera interval finished for the inputs.
It does not set publication readiness. ConversionQueue::replace_converted
(4FD47C..4FD781) is still the next stage; never silently drop that queue.
World start, conversion creation and later publication must each remain
explicit blockers until their real implementations are wired and verified.

VALIDATION SCOPE

The adapter test constructs synthetic ActorStore entries from the original
load-tail oracle's inputs and moves them through ActorStore::from_restored.
It uses actual owned Organization/Element components, names and staged poses.
No dummy component is installed to impersonate a Settlement.

Verified by the targeted test command (6 passed, 0 failed, 0 ignored;
validation.log):
  20 original cases: native result/error, camera words and exact log double
    words compared, including PC24/PC53/PC64 and odd/subnormal map dimensions.
   2 world-start cases: explicit Unsupported before camera mutation.
   2 Settlement-choice cases: not replayed by this adapter because there is
     no owned Settlement variant. The existing aa-actors algorithm fixtures
     cover their selection logic, not a runtime Settlement binding.
Extra tests cover actual-store changes, first-wins membership order, live
visibility flags, stale identity rejection, missing host facts, phase/failure
preconditions, unresolved capital words and signed-zero log arguments. The
test result, command and input/source hashes are recorded in findings.json.

No source TGI/map definitions are guessed by these tests. They prove the
adapter over the declared supplied facts, not the still-missing producers,
nor a complete real-map session, a world tick, combat or multiplayer.
