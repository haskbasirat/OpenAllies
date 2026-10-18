Player+50 VWorldState owning payload, 2026-10-17
==============================================

Production: aa-runtime::player_view_state::State; native4ED6D0,4ED160,
4E1020->4EFB10,4ED210,4E3ED0->4E28D0, Player replacement765E10.
The aa-netcode private VWorldState is now an alias for this SAME owner.
SelectionBorrow still lends State.selected (the existing non-Clone List), its
Kingdom, mask words, look-at words and typed constructor-null current group.
No selected-actor list, detached View or present-group boolean is introduced.

Exact constructor payload:
- Kingdom argument; selection head/tail null; masks10..1C zero; current group20
  null; wildcard24 initially null then actual constructed group; lookAt28/2C0.
- VWorld+C remains typed UNINITIALIZED (None), not allocator poison or zero.
- wildcard0x24-byte payload: empty name; caption from actual current
  Localization.display("static_group_wildcard_name"); CRT empty text;
  vector count0/capacity1 with one initialized null slot; byte18=0,word1C=0,
  byte20=0. Padding bytes are not declared initialized. Storage may grow;1
  is original constructor capacity metadata, not a game or selection cap.
- Strings retain owned UTF16 contents; native allocator/refcount identities
  are portability details and not claimed bit-for-bit allocations.

Destruction/replacement:
4ED210 destroys the wildcard first, then updates actual selection head/tail
before retiring each stable List node. There are NO actor deselect/Object4C
callbacks. 765E10 stores Player28 and invalidates binding_complete before old
view destruction. Player50 is cleared only after old destruction succeeds.
A constructor/text failure leaves raw28 committed, Player50 absent and the
complete-binding guard false. A destruction error retains that actual old
view and all partial writes. CurrentGroup positive production is still absent;
there is no public positive-group handle constructor.

Native evidence:
  python research/skirmish-20261017-player-view-state/native_player_view.py
is IMMUTABLE replay. --write is author-only fixture generation.
60 rows /120 separate original-v14 executions at PC24/PC53/PC64. Null/non-null
Kingdom; two caption strings including Unicode; ctor, empty/two-node destroy,
empty/two-node replacement. Full constructor fields, list values/head/tail,
raw28, group fields and native frees/order are captured. All rows match.
389 observed address-byte pins =384 executed +5 intercepted native entries
401B00,4B9F40,6F7430,6F7810,856370. Additional synthetic CString manager virtual
services are harness seams, not executed game instructions. The original and
v14 .text/.rdata mappings are selected separately; fixture data remains the
explicit initialized inherited bootstrap harness data. Unrelated inherited
bootstrap initialization pins are cleared before each focused case.
Fixture SHA256 a26f1d1b9faff390a7e9506a945c77189bfd4069865ab615706d3e5b76711b15.

Author validation completed:
- aa-netcode --lib release locked/offline check passed17.57s.
- aa-runtime --lib --profile test locked/offline source check passed1m15s,
  including new unit-test source. No new full runtime link was started.
- Fresh corrected author native run and separate immutable replay passed.
- Runtime test EXECUTION remains for parent's shared coherent cohort:
  player_view_state::tests::native_player_view_constructor_and_destructor_rows
  player_view_state::tests::failed_group_retirement_keeps_same_selection_and_unavailable_text_never_completes
- Netcode existing filters:
  local_bootstrap::tests::native_whole_fresh_reset_and_first_local_player_replay
  local_bootstrap::tests::actual_localization_is_required_and_player_handles_belong_to_the_live_owner
  local_bootstrap::tests::ordering_selection_borrows_actual_admin_and_disjoint_session_fields
- Existing external actual Windows/source test (requires AA_GAME_ROOT):
  aa-runtime --test local_bootstrap_source
  actual_source_and_windows_os_construct_the_retained_local_player --ignored.
  This uses actual installed Localization, reset, player ctor and explicit
  low-level765E10(NULL); it does NOT certify BeginPlaying or a human battle.

Open/qualified:
- Successful allocation domain only. Native allocation-null branch, SEH unwind
  and shared-string allocator failure behavior are not claimed.
- Group members stay truly constructor-empty until a native positive producer
  is implemented. Reached positive4E28D0 child effects fail explicitly.
- Native4F5290/4F1C20 already have pure kernels in aa-actors/world_load_tail.rs
  and same-World binding in runtime/load_tail_binding.rs; this change does not
  duplicate or silently replace them. Their prior proofs remain independent.
- Raw28 vs complete binding guards unchanged. Constructor-null is NOT proof
  that765E10 or BeginPlaying completed. Real initialized same-World player,
  VObject directory/controller/decree and frame-loop interaction acceptance
  belongs to the parent composition, not these constructor labs.
- No original assets, release, Cargo manifests/lock or gameplay rules changed.
