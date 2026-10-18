Load-time zone owner and reached Object services
==============================================

NEW aa-runtime::load_zone_services constructs the one ZoneMap before the first
world record, then borrows that map for actual footprint reads. It consumes the
same BoundTerrain, SpatialIndex and MAP_HEADER as load placement. It is neither
a world reset nor a complete detection/memory/SAI implementation.

Initial state authority
-----------------------
Native556697..5566C3 allocates48 bytes per GMap tile and explicitly zero-fills
them before publishing map+28. Constructor54FDC0 then binds the actual variation
and static flags. Its admitted initial calls preserve tile+4 zone-set pointer,
tile+14 fog word and tile+18 supply word at zero.

The serialized TERRAIN explored mask is not unconditionally current fog.
557E6E..557E91 checks terrain version>=4 and59E520 before54EA50 replaces the
low16 of tile+14.59E520 follows reader+90 MAP_HEADER ->+4 WorldRules, then511860
indexes WorldRules.mode through the enum table B3B194/+4. This is not an SVar
and is independent of MapResource+A8 and Session+F8.

CRT9CCA10 calls4994D0 ->4981A0. The latter explicitly initializes the mode
payloads: playable0 and cutscene1 false, savegame2 true, film3 false, metagame
save4/5 true and campaign save6 true. The current MapHeader decoder admits only
mode0. Its saved masks remain retained source data, not applied fog state.

before_first_record validates both mode fields, header-derived layout, held
index dimensions/shift/cell size, every static flags20 word, and absence of any
existing node/primary membership. It constructs ZoneMap from that layout. The
world extents are the same native ceiled integer extents stored back to f32 by
556653..68, including fractional header inputs. Caller must invoke it at the
documented native stage and retain this exact owner; equal dimensions do not
authenticate an unrelated map or authorize resetting a live map.

native-load-order.log pins bounded caller windows of4FC600: TERRAIN selection
and map virtual Load at4FC867.., before Team/Kingdom factory calls and later
STATIC_THINGS/DYNAMIC_THINGS. This is static ordering evidence, not execution
of the entire world loader or auxiliary suffixes. Existing terrain decoding,
registry construction and world record ownership retain their separate proofs.

Services
--------
Context borrows load_world_globals::PointShape and the actual SessionPhase
trait owner. It neither constructs a POINT from PE zeros nor caches phase.
The point producer and actual Session startup transitions are certified by the
separate global-owner author. Session phase reads are lazy at native76AC90's
position, distinct from application E4 and world48.
At the reached point read, validate_catalog rechecks the current destination
source identities and numeric Settings through the point owner's private FP
context. Reusing an earlier valid owner with a foreign/edited Catalog fails.

footprint uses live_object_init::occupancy_indices with the actual private point
owner, current shell pose and source-bound shape. It then reads the CURRENT
ZoneMap tile sets and entries in order, following4EA150 /515000 /5158A0 and the
existing detection::refresh_from_tiles traversal. Null current sets naturally
have no callback. No empty-set query service is fabricated. The initial mode0
factory establishes the first record's empty sets; later mutations remain live.

For nonempty type6 sets, actual supported actor+108 owner is read from the held
or generation-checked registry shell (Object returns null, Actor/Kingdom reads
actual+E8). Actual Kingdom+18C/+190 supply the reached fog prefix. Native reveal
prefix writes are retained before a reached unsupported container/memory/first-
sighting child. Type1 Detection reveal masks and world kingdom-vector mapping
remain explicit required owners. Zone entry actor keys must use ActorId.raw()
or the world must provide an explicit mapping; SpatialIndex's internal ActorKey
is a distinct domain.

visibility reads actual signed Object+58 (8C4EA3D0), then the concrete existing
memory_eligible implementation with current shell/mask and actual phase owner.
The admitted first Kingdom is ineligible and has no memory slots. Positive
memory-slot/vector/create/delete/SAI paths return explicit unsupported errors.
ObjectData/ActorData+16D is not fabricated if the native predicate reaches it.
blocking rechecks current bit10; its positive554350/555620/4D67F0 path remains
an explicit required service. No positive missing child becomes success.

Proof and tests
---------------
native_initial_zones.py executes actual4981A0 payload stores,59E520/511860,
557E6E..91/54EA50 and the GMap zero-fill/54FDC0 prefix, separately in original
and v14 at PC24/53/64. Six fresh machines produce126 mode-gate rows and12 zero-
state rows;217 executed address pins and2 separately intercepted entry pins.
Only allocation and mode name/payload registration storage are seams. Generic
intern-table/CString implementation and whole GMap/GWorld construction are not
claimed. Native partial-version rows verify the version gate and high16 retention.

native_current_services.py runs actual4EA150/554DD0/554330/4CF210/0C getters,
4EB490/4EA350/76AC90 and4EBC70 using the stopped tutorial02 first Kingdom's
captured source shape/pose/flags at all3PC. Thirty rows use60 separate P/V
machines,364 executed address pins and no intercepted bodies. The two mask
values and five phases are explicit controlled probes, not claims about captured
source mask/startup phase. The zero point is an explicit fixture input pending
the global startup owner receipt. Native current empty-set state is explicit;
the independent factory proof above establishes initial-map emptiness. These
rows do not execute the complete Object Init call graph.

Rust tests replay all native mode/occupancy rows, check constructor-storage
refusal, and load the actual tutorial02 map with the real source catalog and
terrain at PC24/53/64. Every16,384 tile current set/fog/supply is checked, as
are saved-mask versus current-fog distinction, mode refusal, mismatched grid /
static flags, and fractional creator extent handling. The actual PointShape
constructor and complete wrapper acceptance remain separate dependency tests.
An own JSON macro typo failed one test compile; its corrected final run and
earlier truthful logs are retained. Final pins/results are in findings.json.

Reproduce from rust-rebuild
---------------------------
 python research/skirmish-20261016-load-zone-services/native_initial_zones.py
 python research/skirmish-20261016-load-zone-services/native_current_services.py
 $env:CARGO_TARGET_DIR='target/s-graphics-reference'
 $env:AA_GAME_ROOT='E:\Games\Axis and Alies'
 cargo test -p aa-runtime --lib load_zone_services --release --locked --offline -j4 -- --include-ignored --nocapture

No original binaries/assets/saves/profiles/releases or Cargo/lock were edited.
This slice establishes reached initialization owners; it does not certify a
playable battle, completed map Init, renderer parity or a completion percentage.
