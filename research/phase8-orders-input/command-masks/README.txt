UIWorldState command masks 0x437EB0, actor vt+0x154 (0x4CE680) and 0x5845A0

What it is
----------
The game UIWorldState (vtable 0xA61E40 over base 0x9FD648) keeps four DWORDs
at +0x20..+0x2C: two 64-bit sets over the 0x27 ActorCommand indices that the
HUD action buttons read.
  +0x20/+0x24 ANY: at least one selected actor may issue the command.
  +0x28/+0x2C ALL: every selected node may issue it.
0x437EB0 (thiscall, no arguments) rebuilds them. Callers (xrefs, rel32):
0x4383FB in 0x438370 (add), 0x43848E in 0x438430 (remove), 0x43858D in
0x438560 (sets the current group's +0x18 byte, then rebuilds), 0x438B14 in
0x438AD0 (deselect), 0x43A19C in 0x439E90 (SaveGroup), 0x43A22D tail jump
of 0x43A1C0 (the per-frame UIWorldState update from 0x440159: it first
deselects every node whose vt+0x10 fails, then caches 0x5C0E10 -> 0x765E00
player+0x28 in +0x4C), and 0x50C95B in 0x50C8E0 (from 0x4CFCC0: an actor of
the local kingdom whose view is selected changed).

Native algorithm (0x437EB0, 238 of 242 instructions executed)
* +0x20..+0x2C = 0. If the +0x08 head is NULL, return.
* [B36530] is read once.
* retreat_only = 1. For each node {view, next}: actor = 0x45B7A0(view)
  (view+0x14 when its vt+0xB4 holds). The scan sets the flag to 0 and stops
  at the first node with any of these: no actor, a NULL Actor+0x7C
  (Organization), Actor+0x100 & 0x10 clear, or a zero Organization+0x8C
  byte. The 64-bit test `and eax,0x10; xor ecx,ecx; or eax,ecx` reads only
  +0x100.
* For index 0..0x26:
  - head view != tail view ([+08]->view vs [+0C]->view): skip the index if
    the byte [[[B36530]+0x298][index]+0x3C] (non_group_command) is set. A
    NULL definition faults at 0x437F5C.
  - retreat_only: skip every index except 7 (retreat) and 8 (rout).
  - bit = 0x96D710(1, index); ALL |= bit.
  - For each node: actor = 0x45B7A0(view). With no actor, ALL &= ~bit.
    Otherwise it calls actor vt+0x154(0x5C0E30(), index, 1); 0x5C0E30 is
    called for every node. If that is false, ALL &= ~bit. If it is true and
    index != 0x23, ANY |= bit. For index 0x23 (research):
      - Actor+0x7C NULL: the actor's own +0xB4 Research must be non-NULL
        and 0x5845A0 must hold.
      - Otherwise, walk the Organization members with 0x4364C0 (iterator
        {org, -1, filter byte 0}; it stops at each non-NULL member of
        +0x28 with count +0x2C). The first member whose +0xB4 is non-NULL
        and whose 0x5845A0 holds sets ANY. When the walk ends without one,
        ALL &= ~bit.
* If +0x4C != NULL and head view != tail view: walk the nodes while
  count < 2. For each node, thing = view+0x14 (directly, not 0x45B7A0).
  count += 1 if the thing is non-NULL, its vt+0x20(1) holds, and either
  its vt+0xB4 is false or its vt+0x134(+0x4C) holds.
  When count == 2: bit = [+0x40]+0x18 ? 0x40 : 0x20 in the high DWORD
  (0x26 group_behavior_disable / 0x25 group_behavior_enable), ORed into
  both +0x24 and +0x2C. A NULL +0x40 faults at 0x438142 before any store.
* Unexecuted: 0x437F3F and 0x4380DD (dead re-tests of a NULL head),
  0x437F4C and 0x4380EA (a NULL tail with a non-NULL head; the list writers
  0x438370 and 0x438430 never produce one).

actor vt+0x154 per vtable (pins in the fixture, identical in both images)
  GActor   A273F0 (ctor 4D6BB0, class 3): +154 4CE680, +20 4CF120, +34 4D1F90,
           +B4 6A71D0, +134 4D1E90, +150 4CF430
  GGroup   A29490 (ctor 4DE350, class 8): the same slots as GActor
  GKingdom A299D8 (ctor 4E4DC0, class 4): the same, but +134 4E5240
  Object   A2A2A0 (class 2): +B4 6A71F0 (false), +20 969B10 (false, ret 4).
  The primary vtables of classes 5/6/7/9/10 (A29780, A28A00, A28B88, A28E90,
  A28D18) also have +B4 6A71F0 and +20 969B10. Only classes 3/4/8 reach
  vt+0x154 in 0x437EB0.
0x4CE680(kingdom, index, selected): if the selected byte is set, it
returns false unless Actor+0xE8 is non-NULL, equals kingdom, and vt+0x20(1)
holds. It then returns vt+0x34(index) (the whole DWORD; 0x4D1F90 uses CL).
0x4CF120 = vt+0x20:
  - Actor+8 bit2 set: false.
  - Element+0x80 with a +0x10 parent: tail-jump to the parent's vt+0x20,
    keeping the argument.
  - argument set, 0x40EAA0(B39690) true and 0x76AC90([B36550]) == 7: true.
  - Otherwise true when Mover +0x6C is NULL, or any of +0xAC, +0xA0 or
    +0x88 is non-NULL.
0x5845A0 (Research component, 38/38): walk the head-first list at
[[[this+4]+4]+0x558]. Return true at the first item whose
0x5844A0(item, 0) == 0 and which is not on this+0x10. Otherwise false.

Oracle
------
native_command_masks.py runs on research/phase8-ai/native_machine.py (page-0
guard, verify_v14). It uses the original AA.exe and the pinned v14 image at
PC24, PC53 and PC64. All 6 runs are identical, and the 421 executed
instruction bytes are equal in both images. Fixture: native-command-masks.json,
sha256 cbfc50ed629eb3474d47043b1d403d6230c063460e4966550d9f4000b53aaf91.
native-generate01.log holds the generation run, native-replay01.log the
byte-identical replay.
* masks: 900 cases, 48 of them native faults (25 at 0x438142 with NULL
  +0x40, 23 at 0x437F5C with a NULL definition). There are 36 empty
  selections, 95 retreat/rout-only rows, 60 rows with research in ANY, and
  rows with bit 0x25 (125) or 0x26 (127). The run makes 26531 vt+154
  calls, 187 0x5845A0 calls, 709 vt+20 calls and 442 vt+134 calls.
  Definitions are either the actual 39 non_group_command bytes
  (research/skirmish-20261016-player-orders/command-ui) or random bytes.
  Seams: 5C0E30 (constant per case, recorded per call) and 5845A0 (scripted,
  recorded). Synthetic actor vtable slots +B4/+154/+20/+134 return raw EAX
  (0x100 / FFFFFF00 = false via AL; 0x80 / FFFFFF01 = true). Every other slot
  traps. Class-2 Objects use the real A2A2A0.
* research: 240 cases (92 true). 0x5844A0 is a seam returning raw EAX
  (0, 1, 2, 4, 0x100, 0xFFFFFFFF, 0x80000000), recorded with (item, 0).
* v154: 720 cases, 240 per vtable (96 true in total). They dispatch through
  the REAL vtables, with Element parent chains of 0-2 real actors. The
  arguments are a kingdom (NULL / owner / other), an index in 0..0x26 or one
  of 0x3F/0x40/0x41/0x105/0x1FF/0xFFFFFFFF (__allshl), and selected as
  0, 1, 0x100, 0xFFFFFF01 or 0x80 (byte test). The same object's vt+20(1)
  and vt+34(index) are measured by separate native calls. Seams: 40EAA0
  (asserts this == B39690) and 76AC90 (asserts this == [B36550]).
Coverage (executed/instructions): 437EB0 238/242, 45B7A0 16/16,
4364C0 24/28, 96D710 15/15, 5845A0 38/38, 4CE680 26/26, 4CF120 45/45,
4D1F90 15/15, 6A71F0 2/2, 969B10 2/2. The four 4364C0 instructions are the
filter-byte-1 Element test, which 0x437EB0 never requests.

  python native_command_masks.py --write   (refuses to overwrite)
  python native_command_masks.py           (replay; byte-identical)

Rust
----
crates/aa-runtime/src/player_order_input/command_masks.rs (kernels):
  pub trait MaskLeaves { View, Object, Organization, Kingdom, Research;
    thing_14, virtual_b4, organization_7c, state_100, organization_8c,
    non_group_3c (None = NULL definition), local_kingdom_5c0e30,
    virtual_154, research_b4, research_available_5845a0, members_28,
    virtual_20, virtual_134 }
  pub fn command_masks_437eb0(leaves, selected: &[View],
    kingdom_4c: Option<Kingdom>, group_40_byte_18: Option<bool>)
    -> Result<[u32; 4], String>
  pub fn command_masks_in_place_437eb0(.., masks: &mut [u32; 4])
    (native partial stores on a failing leaf)
  pub fn actor_45b7a0, any_set, all_set, constants
  pub trait PermissionLeaves { owner_e8, virtual_20, virtual_34 }
  pub fn virtual_154_4ce680(leaves, actor, kingdom, index, selected: u8)
  pub trait ResearchLeaves { items_558, blocked_5844a0, listed_10 }
  pub fn research_available_5845a0(leaves, research)
crates/aa-runtime/src/player_order_input/command_mask_world.rs (live World):
  Live<V, A: ActorRead, ..> implements MaskLeaves over the same owners as
  the order decrees: Catalog, the Bank (DefinitionBankLoan in a frame,
  ReadBank between frames), ActorAccess or ActorStore, Spatial and Globals.
  The client supplies the view directory's VThing+0x14 as a closure and
  0x5C0E30 as a value.
  * class 3/4: guess::Live for +7C, +28/+2C, +100 and def+3C.
  * Organization+0x8C byte.
  * vt+154: virtual_154_4ce680 over actor_word_with_bank(E8),
    eligible_with_bank and allows_with_bank.
  * vt+134: 4D1E90 / 4E5240.
  * +B4: the Research slot. 5845A0 runs over definition.ground.
    research_items and ResearchState.items.
  * classes 2/5/6/7/9/10: vt+B4 and vt+20 are false.
  Entry points: command_masks_between_frames(catalog, &PresentationParts,
  ..), command_masks_in_frame(catalog, GameplayOrderLoans, ..),
  virtual_154_between_frames.
Tests (command_masks_tests.rs, command_mask_world.rs tests):
  * native_437eb0_rows_replay: all 900 rows. It checks masks, the full seam
    event sequence (b4/5c0e30/154/5845a0/20/134), and that fault rows error
    with the faulting address.
  * native_5845a0_rows_replay: 240 rows, events and result.
  * native_vt154_rows_replay: 720 rows. It checks the result and the top-level
    vt+20/vt+34 calls of the 4CE680 composition, that vt+34 equals the
    existing native_command_bit, and the vtable pins.
  * coverage_is_complete_on_reachable_paths.
  * live_command_mask_leaves_bind_actual_definitions (AA_GAME_ROOT): all 39
    live def+3C equal the native controls, an empty selection gives zeros,
    and an unknown Thing is refused.

Open gaps (explicit errors at the reached leaf)
* GGroup (class 8) selected things: vt+0xB4 is true, but every later read
  errors because object_with_bank admits only classes 3/4. Owner: the
  group/formation port.
* 0x5844A0 for a non-empty ActorData+0x558 list: the research admission
  owners (aa_production::research::research_blocked's Research world: vt+108
  kingdom, Structure+94 queue, Kingdom+324, 4CED40/581280/4E0FB0, 5842F0)
  have no live binding. Owner: production/research. An empty list returns
  false without it. The native order of the ActorData+0x558 list against
  the catalog's file order is not proven; it only matters once 0x5844A0 is
  bound.
* UIWorldState+0x4C (kingdom cached by 0x43A1C0) and the 0x43A1C0
  per-frame caller are not in ui_world.rs yet. Owner: ui_world.
* 0x5C0E30 (session [B36550] / 0x76AC70) is supplied by the caller.
