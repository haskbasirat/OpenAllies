Water geometry recovery -- Phase 6

The client helper consumes the original TERRAIN auxiliary suffix, including
version-0 region graph record footprints, then decodes the saved water cache.
It retains authored XY vertices, triangle indices and cell ranges. A missing
cell follows the native regular-quad path; an existing empty range emits no
triangles. Runtime region graph construction is deliberately outside this API.

Native evidence:
  python research/phase6-water-geometry/native_water_geometry.py
Executes the original region reader loops and 55C540 cache reader for all 80
original maps, then queries the original cell hash lookup. Memory allocation,
vector reserve, version framing and byte I/O are explicit seams. Post-read
region-grid publication/linking is skipped. Real cache hash insertion runs.
The 9 saved nonempty caches and 71 empty caches are preserved. Crete's saved
cache is empty; any claim about later runtime regeneration is separate work.

Rust validation:
  Set AA_GAME_ROOT=E:/Games/Axis and Alies
  cargo test -p aa-client water_geometry::tests -- --include-ignored
The source-dependent test compares all 80 native reader offsets, source
identities, full vertex/index byte hashes and ordered cell-entry hashes.
Other tests cover missing versus cached-empty semantics, every truncation of a
synthetic valid source, excessive counts, invalid indices and trailing bytes.

Production owner: terrain_roads owns water_geometry.rs only.
Water selection, height sampling, depth shading and GPU integration are in
material_render's water_render.rs and have separate evidence/review.
