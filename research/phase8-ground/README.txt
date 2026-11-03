Ground under a created thing (2026-11-02)

What the original does when a building is unpacked
- GThing::PostCreate 0x4EBA40 -> 0x4EB500 runs for every created thing. A thing with Thing
  flags blocking and static (0x20000400), a radius of at least a quarter tile (0x4EE700)
  and flatten_terrain (0x2000) sets the BASE height map (GWorld+0x34) under its blocking
  shape to the ground height 0x4F0E50(pos, 0x122) at its position: the brush 0x551140,
  mode 0, slope ElevationSlopeMax (0.5), protect, smooth edge, spreading outward until the
  slope limit holds. It happens once, in the update the building is created, at the start
  of the unpacking. The building's own Z was read just before, so it stands level on it.
- The same blocking static thing clears the doodads under its shape: ClearDoodads 0x4F5560
  -> 0x4F5490, trees, rocks, grass, with their deferred updates and GTIDs.
- The base terrain paint at the end of 0x4EB500 runs only while GWorld+0x48 is 0, before
  the session starts. No tile changes its variation in a running battle. (Sampled: tiles
  of another variation between tick 1 and tick 2900 of two films: 0.)
- Static things nearby keep their Z (sampled: 0 of 4650 on Kursk, 0 of 1808 on Farmhouses
  changed Z in place). The drawn terrain is the height query at its vertices, marked for
  an update through 0x550430 -> 0x50C310 -> 0x43B240 -> 0x71D270.
- The tan patch around a finished building is not terrain: it is the definition's View
  WhenConstructed decal (ground_depot_dirt, _desert, _snow by terrain group), which the
  effects code already draws, on the ground as it now stands.
- Nothing puts the ground back. The brush has no other caller in a battle; the height map
  stays as it is when the thing dies or packs up.
- 0x4EB500 also runs on every position change of a thing (0x4D0EE0 -> 0x4EB820) and when a
  structure joins a settlement (0x585F60), but a thing that moves is not static, and none
  of those calls reached the brush on four films.

Settings +0x1C, the falloff distance
0x4EB500 never writes it: the word on the stack holds what the last call at that depth
left. Sampled at every brush of four films: the thing's own Z before the session starts
(the starting buildings, 28 brushes), 0 on an unpack in a running battle (31 brushes). A
building made by a construction order or a script in a running battle is not in those films:
aa-play gives them 0, not measured.

Where it is
crates/aa-play/src/ground.rs (the rule), crates/aa-play/src/mapgen/height.rs (the brush, one
code path with the generator's objects through HeightGrid), crates/aa-bevy/src/scene.rs
follow_ground (only the terrain layers holding a touched tile are remade) and clear_doodads.

Proof
crates/aa-play/tests/ground_native.rs: the FNV-1a hash of the whole base map after every
brush of a film of (2v2)Kursk, as a read-only sampler read it from the running original,
and the GTIDs of the doodads each brush cleared.
