PHASE 6: NATIVE HEIGHT INPUTS, CELL TRAVERSAL AND FALLIBLE MOVEMENT

Height construction and input
-----------------------------
5519A0 calls original CRT ceil for binary32 world width/height, truncates the
integral result, and creates width=(extentX>>resolution)+1, likewise height.
Cell size is the signed integer 1<<resolution converted to float32; nearest
index bias is truncated half that size. The constructor fills samples with
the explicit default and clears byte flags. Supported safe domain: finite
nonnegative extents, representable ceil/count/allocation, resolution1..30.

Actual reader551E60 reads version, min/max/default, resolution/count witnesses,
then raw float32 samples and version1 byte flags. It checks both witnesses
against the preconstructed layout. Writer5500F0 has a similar but distinct
sequence and is NOT executed as the input oracle. Stream virtual read+0 is a
memory-backed leaf; virtual write+4 is a failing trap in the fixture.

Version0 instead calls5507C0 to reconstruct flags from world cells. Rust rejects
this explicitly. All80 recovered source maps use version1. The byte flags are
retained, not interpreted as new gameplay state. The native reader leaves any
remaining chunk bytes unread; Rust preserves these tails without assigning a
meaning. Every MOUNTAIN_HEIGHT_MAP currently has a nonempty tail (80 total).

Raw MAP_HEADER WorldCreator extent bits are read through aa-mapworld. Input
identity pins cover all80 RMP files, all160 height chunks and the original
Game/SVars.tgi with explicit HeightmapResolutionBits=1. Actual tests check that
no decoded map rule overrides that fixture input; runtime callers must still
supply their actual resolved SVar. Rounded renderer JSON supplies neither
dimensions nor height samples. The map index only locates source files.

28 original constructor executions and6 original v1 loads verify field bits,
allocation sizes, buffer fills and read order. Additional144 actual-map native
nearest/interpolated calls on Crete/tutorial01 cover PC24/53/64 and compare both
80-bit results and explicit float32 stores. A Rust integration test binds both
layers for all80 maps, checking source identities and exact corner sampling.
HeightLayers provides concrete base/overlay sampler dispatch for world hosts,
preserving Binary results until query_actor_height's native stores.

Tile traversal
--------------
TileWalk ports4DD030,4DD150,4DDDE0 and the sign helper4DCE20. Start/end grid
indices truncate before an arithmetic shift. The iterator yields the starting
cell, then exactly Manhattan-distance candidates; the native remaining count
is independent of whether floating-point steps ultimately reach the endpoint.
Absolute deltas below float32 0x38D1B717 take the original axis-only branches.
Otherwise the next Y boundary is stored to float32 while X stays extended.
The X boundary time stays extended for comparison with STORED Y time, although
the selected X step uses its separately stored time. Equal times choose Y.
Out-of-bounds endpoints remain unclamped; the checked wrapper skips invalid
candidate cells, preserving native order. No Bresenham or approximate DDA is
substituted. 360 complete original walks compare every candidate, point bit,
counter and filtered return across PC24/53/64, ties, negatives, boundaries,
near-axis deltas and seeded random inputs. Only diagnostic logging is hooked.

The caller supplies a candidate budget. Each skipped cell consumes work;
exhaustion retains cursor progress and may be resumed deliberately. There is
no fixed map or unit cap. The iterator is a geometry primitive, not proof of
passability, collision or line-of-sight ownership rules.

Movement errors
---------------
GroundWorld now returns Result for all eight boundary methods. Native callback
order and successful field writes are unchanged. A failure immediately stops
later callbacks; prior speed changes, position commits or notifications are not
rolled back. Regression tests cover errors before/after a ground commit, during
collision lookup and after cancellation. Existing native movement traces still
pass. Only the existing DiagnosticWorld implementation in client mechanics_check
was adapted; its synthetic heights remain clearly diagnostic fixtures.

Not yet implemented here
------------------------
Full shape intersection, actor collision filters, tile membership/ownership,
native terrain-change callbacks, live height editing and a coherent full-world
GroundWorld adapter remain required. No empty collision answer, default shape,
implicit FPU precision or direct selected-command-to-Mover path was introduced.
The code does not yet create a playable match. See findings.json for exact
source hashes, commands and reproduction logs.
