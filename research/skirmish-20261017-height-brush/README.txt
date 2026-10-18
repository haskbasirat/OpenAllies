Bounded height brush checkpoint (2026-10-17)

The real 551140 same-endpoint mode0/1 path is ported over the retained base
HeightMap. Temporary pending samples are processed in native order before any
height word commits. Committed height/dirty-prefix writes survive notification
errors. Geometry, terrain slope, caller scratch and notification are reached
services; none has a stock default.

Proof: 75 rows /150 separate original+v14 runs, all3PC,2050 executed byte pins.
Metadata:36 source rows +24 notification rows /120 native runs,334 executed
pins +5 stopped-boundary pins. Direct exact production source Rust harness2/2
passes, including callback errors before and after the commit boundary.

Replays (immutable; do not use --write on frozen files):
  python research/skirmish-20261017-height-brush/native_height_brush.py
  python research/skirmish-20261017-height-brush/native_brush_metadata.py
  research/skirmish-20261017-height-brush/exact-kernel-tests.exe --nocapture

The native fullbrush fixture supplies phase6 explicitly. The separate actual
50C310 gate fixture covers phase0/6, map dirty0/1 and attached view absent/present.
Positive renderer is a named seam, never a completed rendering service.

The runtime bridge keeps the real BattleTerrain sample and dirty owners. The
MapEC field is the embedded helper+90 (Map+5C), initialized to0 by55AAF0.
Effects receives that owner byreference and can lazily read phase first.
RMG/editor cache-build and positive attached render children remain distinct.

The unwritten4EB500 descriptor+1C is not defaulted: an independent real caller
chain proves preceding42A5D3 left actualActorZ there for the documented same
Actor/Params gate. The caller owns that guarded receipt; the brush core demands
it only after raster acceptance.

See findings.json for precise seams, source pins, historical check qualification,
64-bit temporary index extension and open modes/domains. No whole-map startup,
renderer or playable-battle acceptance is claimed here.
