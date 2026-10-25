PHASE 8 - ORIGINAL AA.EXE ZONES, DETECTION AND FOG OF WAR (round 1)
====================================================================

Scope: port the original zone system (.\Source\Sim\Zones.cpp KKC_Zone and the
interned per-tile zone sets), the ZoneSolver (.\Source\Sim\Solver\
ZoneSolver.cpp, fast circle builder and flood-fill paths), the GMap tile fog
and supply words, the zone-type tile queries the AI uses (ZOC/ZOG/ZOS), and the
actor detection state (GObject.cpp: who sees whom, detection memory). Every
routine below was ported from the original machine code and replayed against
the ORIGINAL code executed under Unicorn 2.1.4 (every executed instruction is
byte-checked against the accepted v14 image). Nothing comes from aa-game.

Crate: crates/aa-zones (deps aa-numeric, serde, serde_json, thiserror; the
declared aa-trig/aa-spatial/aa-heightfield are not used yet). Modules: set
(interned zone sets), query (zone-set slots + AI tile queries), tile (fog/
supply words, reveal/hide/occupy, fog restore, kingdom vision masks), zone
(KKC_Zone, move/release, tile callbacks, actor-entered effects), solver (fast
circle builder, dispatch), flood (ZoneSolver Dijkstra / FIFO walks), detection
(actor detection word, who sees whom, detection memory), component (detection
component: ZOD/RZOD ranges and zones, CreateZone).

Files
-----
inventory.json         247 identified native functions (address, size, cluster,
                       role, status proven/ported/seam/partial/not-ported, Rust
                       path, fixtures that execute it, crate citations, notes),
                       plus the removed/kept caps. build_inventory.py rebuilds it.
tools.py               read-only helpers: disassembly, vtables, xrefs, callers,
                       vtable starts and constructors.
zones_machine.py       Unicorn harness (copied from phase8-ai, page-0 null
                       guard admitting only FS-prefixed fs:[0], allocator seams
                       6F7810/6F7430 with 0xCD fill, v14 byte check).
zones_world.py         synthetic GWorld/GMap/tiles/kingdoms/actors/zones with
                       the native layouts (documented in its header).
native_*.py / native-*.json   oracle producers and fixtures (table below).
record_findings.py     writes findings.json (hashes, counts, quirks, gaps).
rust-tests.log, clippy.log    last test / strict clippy runs.
requests-to-lead.md    changes needed outside this agent's files.

Native architecture (established from the code)
-----------------------------------------------
* Zone = KKC_Zone {vtable 0xA3229C, actor +4, radius +8 (binary32, -1.0 =
  inactive), tile x +0xC, y +0x10, flags u16 +0x14 = 1 << type, state +0x16}.
  An actor keeps up to 9 zones in actor+0x148 (slot = type; created by
  KKC_GActor::CreateZone 0x4D69A0). Types and creators:
    0 ZOS supply ............ SupplyComponent 0x58B0E0/0x58AE10
    1 RZOD reveal-to-kingdoms  detection component (actor+0xB8) 0x56EFC0
    2 discovery ............. GActor activation 0x4D7230, component 0x585940
                              (radius 0x4BB000 = 2*tile + settings+0x74 + 1e-4)
    3 (component +0x90) ..... 0x58CF50          4 ZOC .. 0x566DB0, Organization 0x579DD0
    5 ZOG guard ............. 0x568040          6 ZOD detection .. 0x56F140/0x56EB40
    7 ....................... 0x4DCDA0          8 (component +0xD4) .. 0x563E10
* Tile (GMap, 0x30 bytes): +4 interned zone set, +8 actor list, +0x14 fog
  word (bit k = explored by kingdom k (index +0x18C), bit k+16 = visible;
  0xA11740 = 16), +0x18 supply word, +0x1C blocker GTID, +0x20 flags (bit 0
  impassable, bit 8 terrain class), +0x24/+0x28/+0x2D solver scratch.
  Actors carry the same fog-word layout at actor+0x54 (seen / visible).
* Zone sets are hash-consed cons lists of (actor, flags) leaves shared by all
  tiles. add = OR into the actor's entry or prepend; remove = XOR (an entry
  whose flags equal the mask disappears). Two-element lists [a,b] and [b,a]
  are the same interned object (first alive wins), so list order depends on
  map-wide history; aa_zones::ZonePool reproduces the interning with
  reference counts (proven, including the live object set).
* Detection is event driven, not a per-tick pass: moving/creating/releasing a
  zone updates the tiles it covers; a type-6 zone (ZOD) marks its tiles
  explored+visible for the owner, a type-1 zone (RZOD) marks them visible for
  the kingdoms in the detection component's reveal mask; when the last
  detecting zone leaves a tile it becomes not-visible (explored stays until the
  trigger "fog restored in area", 0x542950 -> 0x54F930). Each change calls the
  actors standing on the tile (+0x64 reveal, +0x68 hide, +0x6C unexplore),
  which update the actor's own word, forward to their container, refresh the
  detection memory (ghost records per kingdom, 0x4EBC70) and announce first
  sightings. Shared vision: kingdom+0x228 masks (0x54EB40/0x54EB70), rebuilt
  from alliances by 0x4E0F90 (bits k and k+16 of every allied major kingdom).
* Per move (GActor tile change 0x4D6350): 0x4EC270 clears the actor's visible
  bits and replays the zone sets under it (0x4EA150, slot +0x58), every
  active zone of the actor moves with it (0x515F40), then 0x4CFEA0 ->
  0x54EF10 applies the zone effects of the new tile to the actor (slot +0x54:
  enemy ZOC/ZOG reactions, type 3/8 components, and the settlement-marker
  discovery: an actor with a detection component exploring the area of a
  type-2 marker zone, radius size + SVar MarkerRevealPadding (4)).
* Detection component (actor+0xB8) tick 0x56F140: ZOD (type 6) enabled while
  the ActorData+0x1CC range is > 0 and the actor is not attached/garrisoned
  (0x56EA30); RZOD (type 1) radius ActorData+0x1D0 with the reveal mask of
  kingdoms engaging the actor or flagged enemy kingdoms (0x56EFC0);
  actors under construction (actor +0x100 bit 21, 0x200000) keep a type-6
  zone of radius +0x1D0.
* ZoneSolver: types 1, 2, 6 use the fast circle builder when SVar
  ZoneBuilderFastDetectionZones is set (shipped Data/Game/SVars.tgi: true);
  the others flood: Dijkstra from the zone tile (cost tile, sqrt2*tile
  diagonally; reached within the radius, expanded while tile/2 + cost < radius,
  never across terrain class bit 8 for non-fast zones, ZOC blocked by tile
  blockers, diagonals need a passable corner); release floods breadth-first
  over the tiles holding the zone.
* AI queries: 0x54EFC0(tile, owner, 4|5) enemy ZOC/ZOG, 0x54F8B0(tile, owner,
  subject) friendly ZOS (supply word & kingdom+0x22C, or an allied supply zone
  of another actor), 0x54F3C0 fog/visibility, 0x54EF80/0x54EFF0/0x54F020/
  0x54F870/0x54F8F0 (aa_zones::query).

Ported and oracle-proven (Rust tests replay every fixture case)
----------------------------------------------------------------
fixture (cases / ops / native faults)   native code                       Rust
native-zone-sets.json (400/14055/368)    516360 5162D0 516410 516480        set::ZonePool
                                         516460 516560 516290 5161C0
                                         515FF0 5154E0 5160D0 516160
                                         515C90 515D20 514E90 514800
                                         514EF0 515DE0 515FA0 54FA10 54FA80
native-zone-queries.json (300/11921/100) leaf/union slots +14 +18 +1C +20    query::*
                                         +24 +28 +2C +30 +38 +3C +40 +44
                                         +58 +5C; tile wrappers 54EF40
                                         54EF80 54EFC0 54EFF0 54F020 54F870
                                         54F8B0 54F8F0 559530 5595D0
native-zone-tiles.json (400/8466/439)    516640 515AD0 515BA0 54F7F0 514DA0  tile::*, zone::*
                                         514E00 514E20 514E40 54F4F0 54F5E0
                                         54F6A0 54ED20 54F930 54F670 54F690
                                         54F3C0 54F3E0 + fog/supply bit ops
native-zone-moves.json (450/6606/489)    515F40 552640 515970 514CE0 540860  zone::*, solver::*
  PC24/PC53/PC64                         540590 5406D0 540020 5401A0 53FE90
                                         (+_CIfmod 96DB7A) 53F730 53FD30
                                         53FDE0 53FF00 53F650 53F6C0
native-zone-flood.json (360/3006/189)    540860/540590/5406D0 slow branches,  flood::*
  PC24/PC53/PC64                         5402E0 53F9F0 53F970 53C5C0 53C630
                                         53C590 552440 554ED0 4F2000 4F2760
                                         53D2A0 53D090 53D150 4F4050 53F890
                                         53F8D0 539B10 539DB0 539D20 53F5D0
                                         539DF0 53F880 53F610 53F600 540690
                                         5147C0 (scratch, stamp, busy byte too)
native-detection.json (750/19152/1755)   4EC010 4EC1B0 4EA0C0 4EA0D0 4EA1D0  detection::*
  PC24/PC53/PC64                         4D3B80 4D3C50 4EBC70 4EBFB0 4EBFD0
                                         4EC270 4EA150 (+ slot +0x58 visit)
native-detection-component.json          56F140 56EC90 56EB40 56EA30 56EC10  component::*
  (600/10830/51) PC24/PC53/PC64          56EFC0 56EA10 4D69A0 4849F0 4CF110
                                         4D6350 zone loop (+ fast solver)
native-zone-enter.json (750/11526/0)     54EF10 514FC0 515620 514DA0 514E40  zone::actor_entered_tile
  PC24/PC53/PC64                         (settlement-marker discovery)
native-kingdom-masks.json (500/2237)     4E0F90 54EBA0 559600               tile::kingdom_masks
"faults" are original-code null dereferences (null kingdom, missing detection
component, remove on an empty tile, ...); the Rust port returns
Error::NativeNull* at the same point with the same partial effects (tiles,
events) - the tests compare state after faults too. Native UB (removing an
absent actor from a union list, 0x516560) is detected by an observer and stops
a case; the port returns Error::NativeAbsentActor.

Mutation checks: disabling the two-element swap in ZonePool::cons, or
changing the open-list tie rule, makes the replays fail (both are needed).

Commands (bash, from rust-rebuild; PYTHONPATH=../reconstruction/tools/python-packages)
  python research/phase8-zones/native_zone_sets.py     (~1.3 min)
  python research/phase8-zones/native_zone_queries.py  (~1 min)
  python research/phase8-zones/native_zone_tiles.py    (~0.5 min)
  python research/phase8-zones/native_zone_moves.py    (~4 min)
  python research/phase8-zones/native_zone_flood.py    (~4 min)
  python research/phase8-zones/native_detection.py     (~1 min)
  python research/phase8-zones/native_detection_component.py (~1.5 min)
  python research/phase8-zones/native_zone_enter.py    (~1 min)
  python research/phase8-zones/native_kingdom_masks.py (~0.2 min)
  python research/phase8-zones/build_inventory.py && python research/phase8-zones/record_findings.py
  CARGO_TARGET_DIR=target/s-zones cargo test -p aa-zones --release --locked --offline -j 4
  CARGO_TARGET_DIR=target/s-zones cargo clippy -p aa-zones --release --locked --offline -j 4 --tests --no-deps -- -D warnings
Results 2026-10-15: 9 test binaries, 9 tests (one per fixture), all passed;
strict clippy clean (rust-tests.log, clippy.log).

Proven vs compiled vs untested
-------------------------------
* Proven: every function marked "proven" in inventory.json (189), under the
  seams listed there.
* Ported, not executed by a fixture: 514920 state bit0, 514960 (unreachable
  deep equality branch), 515290 (+0x34 same body as +0x18), 515940 state bit
  setter, 54EA50/54EA60 bulk word setters (callers not ported), 54EB40 alone,
  4F0590 (start push refused only with a zero limit), the FIFO-link-as-cost
  read in flood::scratch_value (see quirks).
* Partial: 0x4D6350 (only its zone loop is ours), 0x542950 (the iteration
  is aa-spatial's; per-tile body tile::restore_fog is proven).
* Bound to the battle: aa-play (crates/aa-play/src/zones.rs) runs the fog of
  war and the supply zones on aa-zones at PC53 (tile changes 0x4D6350,
  detection component 0x56F140, supply component 0x58AE10 tail). aa-runtime
  does not call aa-zones yet.

Threading
---------
* Zone moves, releases, reveal/hide and detection updates mutate shared tile
  state, the global intern pool and the solver scratch, and call back into
  actors/components/SAI in native order: they must run on the simulation
  thread in native order (they are event driven from component ticks).
* Pure reads may run on worker threads against an immutable ZoneMap snapshot
  taken at the native point, with results consumed in native order: all
  query::* functions (the AI's ZOC/ZOG/ZOS scoring over many candidate tiles),
  tile::{explored, visible, shared_*}, detection::{shared_explored,
  shared_visible} (and visible_to/explored_by when the world services they
  call are read-only). The services that record virtual calls must then be
  side-effect free (they are reads in the original).
* The fast circle builder's per-tile tests (in_circle) are pure and could be
  evaluated in parallel per span, but adds/removes must be applied in the
  native span order (callbacks); not done (no measured need).

Native quirks and bugs kept (details in findings.json)
------------------------------------------------------
1. Zone-set interning merges [a,b]/[b,a]: iteration order depends on which
   equal set is alive anywhere on the map.
2. 0x516560 (union remove) corrupts memory when the actor is absent; 0x516460
   (leaf remove) ignores the actor and yields (actor, flags ^ mask).
3. Fast span callbacks clip the end column to width-1 exclusively: the last
   map column never receives a fast-built zone.
4. 0x540020 computes the old r^2 from the register value but the new r^2 from
   the stored binary32, and the old raster radius as inv*r_reg vs new as
   r32*inv: at circle boundaries the remove pass can differ from the earlier
   add pass (leaked or absent entries); reproduced exactly.
5. Flood release (0x4F4050) applies the removal to the start tile even when
   the zone never covered it (zone centred on an impassable tile): null
   dereference (189 oracle cases) or XOR/UB on another set.
6. The ZoneSolver reads its start tile's +0x24 without resetting it; the FIFO
   solver stores tile POINTERS there. A link read as a float cost is address
   dependent; the port assumes the tile array lies below 0x3F800000 (value
   < 1.0, never passes the test) - not exercised by the oracle.
7. 0x5153B0 (union +0x3C) returns "list empty", the leaf returns "added".
8. 0x54F6A0 counts the SAI occupied counter down even when the bit was clear.
9. The ZoneSolver neighbour step has no bounds check (relies on an impassable
   map border); outside the tile array the port errors.
10. A dead actor (+8 bit 0) implicitly releases its zones except types 1/6.
11. 0x54F4F0 passes the actor callback's explored argument as a byte over
    the caller's stack word.

Removed / kept caps
-------------------
* Removed: ZoneSolver open list limit 0x1F9C (8092) entries (0x53CDF0 +0x10;
  pushes beyond it were silently dropped). aa_zones::flood::SolverState::
  queue_limit = None in production; tests use Some(0x1F9C).
* Not a cap: zone-set intern tables grow natively (rehash 514800/514EF0).
* Kept (data layout / rules): 16 kingdom bits in the fog/supply/reveal words
  (0xA11740, saved-game layout); 9 zone types per actor.

Gaps (next round; addresses in inventory.json)
----------------------------------------------
* Seams owned elsewhere: reveal sources 0x56EDC0 (engagement lists actor+0x118
  /+0x130, component +0xB0 +0x24, denizen threats 0x569300), the modifier
  context of 0x45D790 (0x501C30), shape/circle iteration 554DD0/554330 with
  descriptors 4EA0F0/4CF210/46E080 (aa-spatial), first-sighting tail (GTID,
  message 42D490, SAI 79EA90/7AD9A0), component effects 568D80/57AB70/
  567840/58CEA0/564300, SAI hooks 7A99A0..7A9A90.
* Zone creators in other components (+8 methods: 566DB0 ZOC, 568040 ZOG,
  563E10, 58CF50, 585940, 4D7230/4BB000, 58B0E0/58AE10 supply) - they call
  the ported CreateZone/move/release.
* GObject init/load/copy (0x4EBE50, 0x4EC7A0, 0x4EC360, 0x4EC290), memory
  record object 0x4EB9F0 and 0x4EBC30.
* Cheat fog 0x4F9970 / KKC_CheatFogOrder, reveal-all 0x551F90/0x551FD0
  (0x54F400/0x54F4B0), SAI white-fog timer 0x7B8E70.
