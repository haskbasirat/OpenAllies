Orders input view/actor leaves (battle_orders_input::ViewLeaves), 2026-10-19

Type: crate::live_scene::view_directory::DirectoryViews (crates/aa-client/src/
view_order_leaves.rs). Obtain it per orders frame with
battle_view_startup::State::order_views(world) (or VWorldOwner::order_views)
and pass it as battle_orders_input::Live::views.

Bound (native evidence; oracle native_view_order_leaves.py, 237 rows at
PC24/53/64, 85 pinned instructions, original = v14; Rust replay
live_scene::view_directory::order_leaves::tests):
  flags_54 / set_flags_54   view+54 (Fields)
  position_38               view+38/+3C/+40 (Fields)
  field_10_58cb30           58CB30 = view+10, the Data allocation fixed at
                            construction: one identity per canonical
                            definition (append-only directory registry)
  pickable_9c               vt+9C of A02798/A02270/A01EF0 = 6A71D0 (true)
  actor_present_14          view+14 live Thing (a destroyed Thing is refused:
                            its VThing uninit is not ported)
  actor_45b7a0              45B7A0: Thing vt+B4 by 4F9C10 runtime class:
                            3 GActor, 4 its subclass -> true; 2/5/7/9/10 -> false;
                            6 (4EA3D0, Thing+58 > 0) and 8 (4EB920) are errors
  actor_flag8_bit0          45B7D0 Thing+8 bit0 (placement flags = Thing+8)
  actor_4d12c0              4D12C0/4D10A0 over Thing+8 bit29 and +100/+104
  actor_70 / actor_90       GActor+70 Mind / +90 Wall component slots
                            (classes 3/4 only)
  actor_view_0c             Thing+C published view
  root_view_4ce6c0          4CE6C0 (+7C Organization NULL and +80 Element +10)
                            then that Thing's +C
  actor_slot_14/handle_14   Thing+14 GTID low word / whole word

Explicit errors (owner not ported): class_78/84/7C/80 (457B70.. over the
5C0E50 Kingdom vt+134..140 of 4EAD70), virtual_50/54 (45BF20/45BD40, Thing
vt+5C/+60 visibility), actor_virtual_58/84/20, actor_can_34, links 16C/94
(437650, 640B40, 5E05D0), hidden_4549c0, voiced_436600, on_list_100 and
proximity_18 (VGrid On-list / proximity writers), model_78, 469AC0 radius,
86E8D0 (model owner), floaters 42EB10/453470, flash 459600.
