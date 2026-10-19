Action-mask publication 0x4CFCC0 branches 2/3 (package 10)
============================================================

Native chain
------------
0x4D1350 rebuilds Actor+F0/F4 and tail-jumps (0x4D147E) to 0x4CFCC0 only when
the mask changed. 0x4CFCC0(this = actor):

 1. Element (+80) with an Organization (Element+10): 0x529D70 loads
    [Organization+7C]+4 (the Organization component's owner) and jumps to
    0x4D1350 (already ported: aa_actors::action_publication + the stored
    Bound's rebuild_actions).
 2. vf108 (0x4D1E80 = Actor+E8) non-NULL and (signed char)Object+4C > 0:
    vf108 again, then 0x76C7F0(ecx = [B36550] Session, owner).
    0x76C7F0 walks Session+BC nodes {player, next}: if 0x765E00(player)
    (NULL when [B36534] GWorld is NULL, else player+28) == owner, or a second
    0x765E00 call returns NULL, and player+50 (VWorldState) is non-NULL:
    0x4ECD70(player+50).
 3. Thing+C (view) non-NULL and view+54 bit 17 (selected): 0x50C8E0(actor)
    (ecx = [B36534]+130, unused). 0x50C8E0: return when [B36534]+D8 == 6, when
    Thing+C is NULL or bit 17 clear, when vf108 is NULL or 0x4F5290 is NULL;
    vf108 again, 0x4F5290 again, return unless equal; Thing+C re-read;
    0x50C7D0(UIWorldState = [[B36538]+13C], view) searches its +08 nodes
    {view, next}; found -> 0x437EB0(UIWorldState) (HUD masks +20..+2C).

0x4ECD70(this = VWorldState: +0 kingdom, +4 head / +8 tail {thing, next},
+10..+1C mask words, +20 group, group +18 byte): see the module doc of
crates/aa-runtime/src/action_mask_publication.rs. Every Thing primary vtable
(GActor A273F0, GKingdom A299D8, GGroup A29490, A2A2A0, A29780, A28A00,
A28B88, A28E90, A28D18) has vt+40 = 0x887B50 (`xor al,al; ret 8`), so with
real things phase 1 always ends with all four words zero; only phase 2 (two
distinct eligible own things and a non-NULL group) sets bit 0x25 or 0x26 in
+14 and +1C. Callers of 0x4ECD70: 0x4ED073 (4ECFF0 select), 0x4ED0BA
(4ED080 deselect), 0x4ED154 (4ED0D0), 0x76C81F.

0x4F5290 is the existing aa_actors::world_load_tail::local_kingdom kernel
(research/phase8-saved-components/native-load-tail.json). 0x4F52D6: a NULL
[GWorld+28] jumps to 0x4F52E9 exactly like map mode 0.

Oracle
------
native_action_mask_publication.py (research/phase8-ai/native_machine.py:
original AA.exe and pinned v14 image, PC24/PC53/PC64, page-0 guard, every
executed instruction byte-checked against both images). 320 cases, 6 runs,
rows identical across runs. Suites:
  masks    106 rows (101 ok, 5 native faults): 0x4ECD70 directly.
  owners    48 rows: 0x76C7F0 with 0x765E00/0x4ECD70 native, incl. NULL GWorld.
  selected 102 rows (97 ok, 5 faults): 0x50C8E0 with 4D1E80, 4F5290 (76AC60,
           76AC70, 765E00, 4F48E0, 4BEDF0, 4012C0, wcscmp 96CB83), 50C7D0.
  publish   64 rows (62 ok, 2 faults): 0x4CFCC0 end to end.
Seams (recorded): 0x4CF120 (vt+20 of the real vtables, raw EAX per case),
0x5845A0 (a NULL research falls through to the original prologue, which
faults at 0x5845A3), 0x4D1350 (529D70's jump target), 0x437EB0 (proved in
../command-masks), synthetic-vtable +20/+40/+B4/+134 (to cover the vt+40-true
and research branches), the CString manager (+10 clone returns itself, +4
free recorded; never reached).
Coverage: every instruction of 4CFCC0, 529D70, 76C7F0, 765E00, 4ECD70,
50C8E0, 76AC60, 76AC70, 4BEDF0, 96CB83, 4D1E80, 4D1E90, 4E5240, 887B50,
6A71D0, 969B10. Unexecuted (listed in the JSON): __allshl's >= 64 path,
alignment padding 0x50C7DD / 0x4F48FD, the CString clone/free slow paths
(4012C0, 4F5290, 4F48E0) and 6A71F0 (GObject vt+B4; its vt+20 is false first).
Native faults kept as rows (Rust must error): VWorldState tail NULL under a
head (0x4ECDAC), [B36530] NULL with distinct things (0x4ECDB4), NULL thing in
a node (0x4ECDF7), research NULL at index 0x23 (0x5845A3), UIWorldState NULL
(0x50C7D0), no local player on 4F5290's by-name path (0x4F5304).

  python native_action_mask_publication.py --write   (refuses to overwrite)
  python native_action_mask_publication.py           (replay: byte-identical)

Rust
----
crates/aa-runtime/src/view_flags_service.rs   ActionMaskPublication service
    trait (PlayerRef, HudLoans with command_masks_437eb0 over the existing
    player_order_input::command_masks port).
crates/aa-runtime/src/action_mask_publication.rs (view_flags_service::native)
    kernels selection_masks_4ecd70, owner_views_76c7f0, selected_view_50c8e0,
    contains_50c7d0, local_kingdom_4f5290 (LoadTailWorld adapter), and the
    live 4ECD70 binding rebuild_view_state_4ecd70 over player_view_state::State
    (vt+20/B4/134, +3C, B4/5845A0 from command_mask_world::Live; vt+40 =
    887B50 after the class check).
crates/aa-runtime/src/action_mask_publication_tests.rs   replays every row:
    events, outcome, every VWorldState's final words (also after faults).
crates/aa-runtime/src/stored_actor_actions.rs   Bound.publication; new
    rebuild_on_owners_publishing; refresh_owner_views (76C7F0),
    renderer_flags (Thing+C view+54 through the service) and
    refresh_selected_view (50C8E0) use the service, and keep their previous
    explicit errors when it is not lent.
crates/aa-client/src/view_flags_service.rs   ClientPublication composing
    DirectoryFlags (ViewDirectory view+54), SessionPublication and
    UiPublication into ActionMaskPublication.

Open (explicit errors until the owners lend the services)
---------------------------------------------------------
* The mind/order callbacks (stored_mind_init_world::Bound::rebuild_actions,
  reached from player_order_frame and live_element_mind) still call
  rebuild_on_owners without a publication: they must hold an
  Option<&mut dyn ActionMaskPublication> and call
  rebuild_on_owners_publishing when it is lent.
* ViewDirectory needs `impl DirectoryFlags` (view_directory.rs owner).
* SessionPublication (netcode Session players/VWorldStates, GWorld fields)
  and UiPublication (battle orders UIWorldState) implementations.
* VWorldState +20 group: CurrentGroup has no producer; a non-NULL group is an
  explicit error in 4ECD70.
* actor_world.rs ConstructionActions (saved/fresh construction): Thing+C is
  NULL and Object+4C is 0 at construction, so branches 2/3 are not reached
  there natively; the existing errors only fire on an impossible state.
