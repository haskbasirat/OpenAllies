Dynamic TerrainBlock Resource (723D30 -> 723960 -> 5ACEC0)
=========================================================

Native
------
723D30(map, x, y): name = 405D60 "Dynamic/Terrain x%d y%d", 0x150 object
723960 (5AA890 base Resource: flags+8 0x10, counts 0, own TaskLoad), then
723CE1 5AA270(n*n*0x22, n*n*0x22) with n = +C8 = TerrainMap+A4, 723CED
flags |= 8 (never cached by 5AC3D0); 5ACEC0 under manager+20: flags bit4 ->
5AC1C0 name-hash insert; always 59CB00 class accounting (manager+9C).
Class KKC_RTerrainBlockResource, descriptor B4040C -> Resource B38E24
(CRT 9E4130): not NiAVObj, so 5B1A10 takes no +20 and 5ACA50 skips it.
Vtable A88DB0: vf0C 5AA3F0 (generic Request), vf10 721180 (children
4B7060, then vf0C with mode 4 / B282F0), vf14 7211F0 (5AA550 first, then
71F250, children 4B7090, 721030, ...), vf18 7212D0 (Load/Block: children,
5AAE10, 720AD0 when +44 is NULL), vf3C 5AA6F0, vf40 5AA700, vf4C 4F0500
(`mov eax,1`), vf54 71B5F0.

Port API (aa-runtime)
---------------------
resource_objects::Kind::TerrainBlock; Entry::terrain_block (no source file).
Resources::register_terrain_block_5acec0(x, y, map_a4, backend) -> Handle
  (+20 guard via the backend's enter/leave_request; a duplicate name is an
  explicit error).
Resources::request_0c(handle, delta, mode, backend): 721180's vf0C after the
  caller's child requests; mode 4 runs the task at once (vf4C = 4F0500 is
  the ResourceManager's own leaf), setting bit0|4|6.
Resources::release_14(handle, delta, backend): 7211F0's 5AA550 part. At zero
  a loaded block unloads (bit3): "Unloading", +3C, clear bit0, vf40 5AA700,
  vf54 -> Backend::terrain_unload_71b5f0 (terrain owner), "Unloaded". The
  caller then runs 7211F0's remaining steps.
A nested vf18 from request_0c (count > 1 with mode 4) is an explicit error:
the terrain owner calls its 7212D0 Load itself.

Proof
-----
native_terrain_block.py: class chain, 4F0500, 5ACEC0 branches (flags 0x18
and 0x08), the 723CCE..723CF0 memory/flag slice (n = 17, 33, 0, 0x10000),
3 PCs x 2 images, plus static pins. Rust (resource_release_tests.rs):
terrain_block_constructor_class_and_memory_match_native_all_pc and the
AA_GAME_ROOT round trip terrain_block_registration_request_and_release_
round_trip (register, immediate request, release-unload).
