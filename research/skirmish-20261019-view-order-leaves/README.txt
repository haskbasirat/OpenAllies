Orders input view/actor leaves (battle_orders_input::ViewLeaves), 2026-10-19

Type: crate::live_scene::view_directory::DirectoryViews (crates/aa-client/src/
view_order_leaves.rs, kernels in view_order_world.rs). Obtain it per orders
frame with battle_view_startup::State::order_views_with(world, &context),
where context = State::order_context(session, bridge, execution) is read
before session.parts(), and pass it as battle_orders_input::Live::views.
order_views(world) without a context still works; the Session/UI leaves then
return an explicit error.

Bound here (oracle native_view_order_leaves.py, 240 rows at PC24/53/64, 80
pinned instructions, original = v14; Rust replay
live_scene::view_directory::order_leaves::tests):
  flags_54 / set_flags_54   view+54 (Fields)
  position_38               view+38/+3C/+40 (Fields)
  field_10_58cb30           58CB30 = view+10, the Data allocation fixed at
                            construction: one identity per canonical
                            definition (append-only directory registry)
  pickable_9c               vt+9C of A02798/A02270/A01EF0 = 6A71D0 (true)
  actor_present_14          view+14 live Thing (a destroyed Thing is refused:
                            its VThing uninit is not ported)
  actor_45b7a0              45B7A0: Thing vt+B4 of the constructor's PRIMARY
                            vtable: 3 GActor A273F0, 4 Kingdom A299D8, 8 GROUP
                            A29490 -> 6A71D0 true; 2/5/6 (A28A00)/7/9/10 ->
                            6A71F0 false. (Correction: the first version read
                            class6's secondary vtable A289D8 at +60, whose
                            slot B4 is 4EA3D0, and had no class8.)
  actor_flag8_bit0          45B7D0 Thing+8 bit0 (placement flags = Thing+8)
  actor_4d12c0              4D12C0/4D10A0 over Thing+8 bit29 and +100/+104
  actor_70 / actor_90       GActor+70 Mind / +90 Wall (classes 3/4/8)
  actor_view_0c             Thing+C published view
  root_view_4ce6c0          4CE6C0 (+7C Organization NULL and +80 Element +10)
                            then that Thing's +C

Kingdom, visibility, seen-record and link leaves: see
../skirmish-20261019-view-order-world/README.txt.

Still explicit errors: on_list_100 / proximity_18 (VGrid traversal 42F670
and camera cell sweep 42FAD0 / 45B330 are not ported), flash_459600 (no
shared owner of the global B396E0 timer), and the model developer's
model_78, selection_radius_469ac0, update_model_86e8d0.
