Orders view leaves over the current Kingdom, visibility, seen records and
links (battle_orders_input::ViewLeaves), 2026-10-19

Rust: crates/aa-client/src/view_order_world.rs (kernels over `Reads`, the
production `Live` reads and `OrderContext`), used by DirectoryViews in
view_order_leaves.rs. Replay test:
live_scene::view_directory::order_leaves::world::tests::native_view_order_world_replay
(values AND the seam call order of every row).

Oracle: native_view_order_world.py (default compares the fixture; --write
regenerates). 775 rows (5E05D0 at PC24/53/64, the rest PC53: no x87 work),
1038 pinned instructions, all byte-identical in the original and v14 images.
Things and Kingdoms carry copies of the real class vtables; the only seams
are Thing vt+70/+74 (the ported NativeVisibility visible_to / explored_by),
Kingdom vt+144 (the ported relations::borrowed::kingdom), 40EAA0 (the
ported can_process_40eaa0) and the GetCurrentThreadId import.

Bound leaves and their native code
  class_78/7c/80/84   view vt+78/7C/80/84 = 457B70/457BB0/457BF0/457C30:
                      k = 5C0E50 (40CEC0 E4==3 -> NULL; NULL Session or NULL
                      76AC70 -> UI+1E0; 76AC70: Node+8 player -> 765E00
                      player+28), owner = 4EAD70(view+14, k), then Kingdom
                      vt+134 4E5240 (k == owner) / +138 4E5250 (== or
                      relation 1) / +13C 4E5280 (relation 2; NULL k or Thing
                      -> true) / +140 4E52A0 (relation 3).
  4EAD70 / 4EB340     visible = vt+5C 4EEF70(k,1); seen = vt+60 4EAB20(k,1)
                      (4CE890 jumps there for 3/4/8); bit7 override; flare =
                      Data vt+38 && Data+174 bit26; visible, seen && 4EA350,
                      or flare -> vt+108 (GActor+E8; objects 4C9FA0 NULL) /
                      Thing+8; seen -> 4EA430 record +14 / +34; else [B3CC00]
                      / [B3CC24] (zero-filled, never stored).
  4EEF70              Thing+8 bit7 -> false; GetCurrentThreadId ==
                      [B364D4]+48 && (Session+E4 == 3 || 4F0930) -> true;
                      else vt+70. 4F0930 = CheatFog [[B39AB0]+1BC] || GWorld+275.
  4EAB20 / 4EA350 / 4EA430 / 4EA3D0
                      vt+5C true -> false; vt+74 false -> false; 4EA350
                      (Data vt+40, flare, Thing+8 bits 29/11/!23/!22,
                      ObjectData+16D, vt+A8: class4 only) or the k+18C slot
                      of Thing+5C when k non-NULL, k+190 == 0, Thing+58 > 0.
  virtual_50          view vt+50 45BF20(1): 45BBD0 (UVar
                      PerformanceDisableDoodads, Thing+8 bit10, 4BB3C0
                      category+0C 1/0x26, Data vt+40) -> false; 40CFA0 ->
                      true; 5C0E10 -> false; view+54 bit26/bit20 -> true; NULL
                      5C0E50 -> true; vt+5C(k,1).
  virtual_54          view vt+54 45BD40(1): 5C0E10, 45BBD0, 5C0E50, view+54
                      bit20, vt+60(k,1).
  actor_virtual_58    Thing vt+58 4EE3C0(5C0E50, 1) = vt+5C || vt+60.
  actor_virtual_84    Thing vt+84 4EB8B0(5C0E50): live k record -> 4EB340
                      bit14, else Thing+8 bit14.
  actor_virtual_20    Thing vt+20(1) 4CF120 (3/4/8; others 969B10 false):
                      Thing+8 bit2, Element parent tail call, 40EAA0 &&
                      76AC90 == 7, Mover NULL, MobileDeployment/Recruit/Denizen.
  actor_can_34        Thing vt+34 4D1F90: 96D710 (1 << low byte, >= 64 is 0)
                      & GActor+F0/+F4.
  link_16c_view       437650: +16C handle through B364C0 (generation, entry)
                      and that object's non-NULL +C.
  link_94_view        640B40: Structure(+94)+34 handle, object +C non-NULL.
  link_94_live        5E05D0's test: Structure+34 resolves to a live entry.
  hidden_4549c0       640B40: 4549C0 (+80 -> +10) non-NULL and its
                      vt+58(5C0E50, 1) false.
  voiced_436600       436600 per selected 45B7A0 actor: +7C Organization
                      +40 leader with Experience (+A4) -> leader+C.
  floaters            451860/452A90 and 453470 do nothing while view+9C (the
                      attached VFX list 455310 allocates from 43CD00) is
                      NULL; the constructor's NULL has no ported writer. The
                      453470 head is in this oracle; non-NULL is an error.

Static section of the fixture: every reference to B28158 (.data initial 2,
the orphan relation), B2815C (initial 3, the Team NULL relation), B3CC00 and
B3CC24 (zero-filled) is a load. 4E43D0 and 4E4B40 take the addresses of
B28158/B2815C and read through them only.

Production owners (view_order_world::Live)
  Thing class/flags/Data   ActorStore definition (native_kind -> Data vt+38/
                           +40: ThingData A20770 / ObjectData A13318 none,
                           ActorData A05540 vt+38, DynamicDoodadData A0F6E0
                           vt+40; other kinds are errors), placement flags,
                           component_mask, category registry_index
  Thing+58/+5C             the ported detection memory (ObjectFields.memory)
  vt+70/+74                live_battle_visibility NativeVisibility with the
                           World's FogOfWar SVar
  Kingdom +18C/+190, relations   the Kingdom/Team derived SpecialShell
  GActor fields            component table, +E8 (ordinary_owner), +16C, +F0
  Session E4/F8/Node+8/76AC70, 40EAA0, UI+1E0, UVar PerformanceDisableDoodads
                           OrderContext, read by
                           battle_view_startup::State::order_context before
                           the frame's AdminNode loan
  thread                   the orders input runs in the UI frame 5C92D0,
                           which App 410E70 runs from MainThread vt+3C
                           (A84530 -> 6FD290), the B364D4 object whose +48
                           85FAA9 captured on that thread: equal.
  CheatFog                 the World's SVar manager

GWorld+275 is PresentationParts::runtime_fields.fog_cheat_word() and
ObjectData+16D is PresentationParts::source_fields.byte(id, 0x16D), both of
the SAME World.

Explicit errors left in the bound leaves (reached only where native reads):
  A UVar PerformanceDisableDoodads record that 4B2E60 cannot find is the
  native NULL dereference. Data kinds other than Thing/Object/Actor/
  DynamicDoodad have unproven vt+38/+40. Thing-less views (45BFA5 position
  path) do not occur in the directory.
