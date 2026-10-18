PHASE 8: CLASS-CHANGED SAVED-RECORD CONVERSION (2026-10-15)

Inventory group 1 ("source class # differs from definition class #",
7,879 records on 27 maps, inventory-round2) is NOT a load failure in the
original game. AA.exe converts these records. This folder ports that
conversion from the original machine code and checks it against the original
code running under Unicorn.

Rust: crates/aa-mapworld/src/class_conversion.rs (new module)
Tests: crates/aa-mapworld/tests/class_conversion.rs (replays the oracle
fixtures)
Oracle: native_conversion.py + native_machine.py (+ rmp.py, the map-archive
reader) -> native-conversion.json (synthetic) and native-conversion-real.json
(real maps)

---------------------------------------------------------------------------
NATIVE BEHAVIOUR (addresses in AA.exe; executed windows are identical in the
v14 runtime-expanded AA.exe)
---------------------------------------------------------------------------
Record load 4FAE80(this=world, ctx, &out, owner, conversion vector):
  *out = 0; 41D290 IDS; class word; saved ID word.
  4FAF14 generation check -> silent null (not this module).
  427BD0 Thing lookup -> null: load_warning_unknown_thing_ids (not this module).
  4FAFA8: definition class [[def+18]+1C] equal to the saved class -> normal
    construction (4FB0A9 jump table).
  4FAFB9..4FAFC3: saved class == 2 AND vector != null -> CONVERT:
    4F26F0 builds a 0x30 record; 405B80 copies the IDS (+00); +04 = definition;
    +08 = saved ID; 4EA050(ctx, &rec):
      4EF0C0: 8589B0 Thing version (0..2, else "Bad data version"), 41D290
        name -> +0C, word -> +10, 12 bytes position -> +14, 12 bytes
        direction -> +20, flags word (dead stack slot, discarded), version 1
        only: one more word (discarded);
      8589B0 Object version (0..5, else "Bad data version"); 59E300(0.0f)
        reads the scale word and multiplies it by +0.0 into a dead slot;
        version >= 1: variant word -> +2C, else +2C = -1.
    4F9840 appends a copy (4F5F20; growth 4F60C0/4F4340). out stays null.
  Any other mismatch: map warning load_warning_changed_gclass|s (A2CAB8, 5A1040
    at 4FB077) and null.
Section loader 4FC340(ctx, data chunk, index chunk, vector): after a null
  result it seeks to (position before the record) + DYNAMIC_THINGS_INDEX length
  (4FC45E..4FC466 -> 858F50), whatever 4FAE80 read (less, or MORE: a seek
  back). 4FC600 calls it for USED_REQUIRED_OBJECTS (GWORLD version >= 25),
  STATIC_THINGS and DYNAMIC_THINGS, always with its own vector (4FCF05..
  4FCF45). TEAMS/KINGDOMS (4FCACA, 4FCD52) and embedded records (4FC200)
  pass a null vector: their mismatches are skipped with the warning.
Replacement, the LAST step of 4FC600 (4FD47C..4FD781), after every section,
  the PostLoad loop (4EFBE0 -> vf DC), 4F1E10, 4F5290, 4F1C20 and the
  "[4610] InitForLoadedMap" log; per queued record in order:
    creation block, vtable A25B1C (0x28 bytes): +04 definition, +08..+10
    position, +14/+18 direction x,y (constructor default (0,-1.0f)), +1C name
    = EMPTY (fresh string; the saved name +0C is NOT passed), +20 variant,
    +24 requested ID = saved ID (+08). Not passed: +0C name, +10 word,
    direction z (+28).
    variant: def vf+30 false -> -1; ObjectData+108 (aspects_as_variations)
    false -> -1; +110 (aspect count) <= 0 -> -1; saved variant (signed) >=
    count -> 0; else the saved variant (negative values pass through).
    4F9C10(world, &block): non-null -> load_warning_replaced_changed_gclass|s
    (A2CE40, 5A1040 at 4FD721); null -> load_warning_cant_replace_changed_
    gclass|s (A2CE90, at 4FD674). Then the vector is destroyed (96BE33).
  A25B1C block virtuals: +0/+4/+8 6A71F0 (false), +C 6A71D0 (true).
  4F9C10 on such a block (dispatch section): class 3 "Actor class requires a
  GACD" and 8 "Group class requires a GACD" (vf+0 false), 4 "Kingdom class
  requires a GKCD" (vf+4 false) -> 404ED0 log, null; 2/5/6/7/9/10 reach the
  class constructor (4EA5E0/4E66D0/4DB610/4DBEE0/4DCB30/4DC450); 0, 1 and
  >= 11 dereference null at 4F9F6E (a native access violation; a saved class
  2 converted to a class-1 ThingData definition would crash the original).
Partial failure: "Bad data version" (8589B0 -> 41DFB0) is a
  TGC_LoadWarningException. The conversion runs in 4FAE80 EH states 0..3,
  outside its only try block (states 13..15, FuncInfo AE6F98); 4FC340
  (AE6520) and 4FC600 (AE7120) have no try block; the map resource loader
  5A2B90 catches it (try states 0..8, catch 5A312F on TGC_FileException, a
  base class of the thrown type): the map load fails, the record is not
  appended, the vector is destroyed by unwinding and nothing is replaced.
Precision: no floating-point value survives (59E300's product is dead; the
  tail copies words). The reads section runs at PC24, PC53 and PC64 and the
  oracle asserts identical results.

CORRECTIONS to the R3 description in research/phase8-definitions:
  record +2C is the Object variant word (version >= 1), not an owner; the
  saved name is not copied into the creation block (the block name is empty);
  only direction x,y are passed.

---------------------------------------------------------------------------
RUST PORT (crates/aa-mapworld/src/class_conversion.rs)
---------------------------------------------------------------------------
check_class (4FAFA8..4FAFC3), read_prefix, read_changed_class_record (byte
path: 4FAE80 prefix + 4EA050/4EF0C0), read_placement_record (same result from
a decoded inventory Placement), ConversionQueue (the 4FC600 vector; a Vec, no
cap), on_class_mismatch / on_placement_class_mismatch (the hook),
ObjectCreationParams (A25B1C block), native_kind_is_object_data (def vf+30 by
4BCA00 data kind), VariationSource + creation_variant (4FD535..4FD56E),
ReplacementWorld (4F9C10 seam: fresh creation, the restoration agent's port)
and replace_converted (4FD47C..4FD781). Errors: BadDataVersion (map load
fails, queue discarded), ReadPastChunk (native chunk-stream error path
858C90/858BA0 not ported), NotAMismatch, PlacementWithoutObject. A Rust Err
from the world stops replace_converted with the completed entries.
Hook instructions for the restoration loop: requests-to-lead.md (C1).

---------------------------------------------------------------------------
ORACLE (native_conversion.py; seams in native_machine.SEAM_DOC)
---------------------------------------------------------------------------
Original code executed: 4FAE80 (to its common exit), 4F26F0, 405B80,
4012C0, 4047A0, 4EA050, 4EF0C0, 8589B0, 59E300, 4F9840, 4F60C0, 4F5F20,
4F4340, 4F2740, CRT eh-vector iterators 96D18D/96BE33, 4FC340, the 4FC600
tail 4FD471..4FD7B1, 4F9C10 up to its class constructor, the definition
vf+30 leaves of all 24 data classes.
Seams: allocator 6F7810/6F7430/856370 (bump heap, 0xA5 fill), 401B00
literal ctor, 41D290 string read and the stream read (chunk payload; chunk
descriptor +58 size / +68 position kept current), 859030 chunk open, 858F50
seek, 427BD0 Thing lookup (fixture IDS -> definition object with the ORIGINAL
data-class vtable, +18 category with +1C class, +108, +110), 4B9F40 text
lookup (returns the key), 5A1040/404ED0 logs, throws 41DFB0/858AD0/4F0110,
4FB0A9 class-equal construction path (consumes a fixture count, out =
marker, resumes at 4FB539), 4F9C10 in the tail (records the block, returns a
fixture result), the class constructors in the dispatch section (stop).
Emulator caveats (docs/NATIVE-PORT-RULES.md): page-0 null guard installed
(phase8-restoration/null_guard.py): the only page-0 accesses are the genuine
4F9F6E null dereferences of the dispatch section (classes 0, 1, 11); every
allocation goes through the 6F7810 seam; no x87 transcendental and no x87
result is compared (59E300's product is dead); no code is written into
emulated memory.

Sections (native-conversion.json):
  mismatch  260 cases: saved class {0..11, FFFFFFFF} x definition class 1..10
            x vector null/non-null, + generation mismatch x2, unknown IDS.
            Converted exactly when saved == 2, definition != 2, vector set.
  reads     3 x 186 rows: Thing versions {0,1,2,3,-1,80000000} x Object
            versions {0..6,-1}, names (empty, 300 units, lone surrogate),
            variant x scale words (NaN, denormal, -inf, -0), a version-1
            legacy word, every truncation point of three records; PC24/53/64.
  append    59 records (converted and skipped) into one vector (growth).
  section   4FC340: mixed records (convert, skip, unknown, generation,
            construct seam with 0 / full consumption), an index length
            shorter than the reads (seek back), a bad Object version in the
            2nd record, empty section, index/data version 1, missing chunks.
  replace   tail over 99 / 12 / 0 / 2 converted records: 11 definitions
            (ThingData, ObjectData, ActorData, DynamicDoodadData; +108 0/1/2;
            +110 -1, 0, 1, 3, 4, 5, 80000000) x 9 saved variants, mixed
            4F9C10 results.
  dispatch  original 4F9C10 on the tail's block for classes 0..11.
  vf30      definition vf+30 of the 24 data classes of 4BCA00.
Real (native-conversion-real.json): every group-1 record of the 27 maps
(from research/phase8-definitions/inventory-round2.raw.json), read from the
decoded RMP chunk at its index offset with the map's IDMGR generations, run
through native 4FAE80 into one vector per map in ordinal order; then the
native tail; then native 4F9C10 on every block. Definition inputs (class,
+108, +110) come from real-definitions.json, written by the Rust test
dump_real_conversion_definitions from the definitions agent's native catalog.
Result: 7,879/7,879 converted (null result, only the 427BD0 seam reached,
84..108 bytes read, always exactly 10 bytes fewer than the index length:
all are Thing version 2 / Object version 5 and the Object version>=2/3/4
words (4+2+4) are skipped by the seek); 7,878 blocks reach the DynamicDoodad constructor 4DCB30;
london_bridge (TrailerMaps/axis08_sealion.RMP, ordinal 4694, saved ID 8089,
definition class 3) is refused by 4F9C10 ("Actor class requires a GACD") ->
load_warning_cant_replace_changed_gclass. Creation variants: 7,182 explicit,
697 -1 (696 definitions without aspects_as_variations + 1 saved -1); every
saved name is empty and every saved direction z is 0.

---------------------------------------------------------------------------
COMMANDS
---------------------------------------------------------------------------
PYTHONPATH="E:\Games\Axis and Alies\reconstruction\tools\python-packages"
  python research/phase8-conversion/native_conversion.py        (~5 s)
  python research/phase8-conversion/native_conversion.py --real (~45 s;
    needs real-definitions.json)
AA_GAME_ROOT='E:\Games\Axis and Alies' CARGO_TARGET_DIR=target/s-conversion
  cargo test -p aa-mapworld --release --locked --offline -j 4
  --test class_conversion -- --include-ignored
  -> 10 passed (8 default + dump_real_conversion_definitions and
     real_maps_placement_path_matches_byte_path, which need AA_GAME_ROOT).
Full aa-mapworld default suite with this module: all test binaries pass.

---------------------------------------------------------------------------
PROVEN / COMPILED / UNTESTED
---------------------------------------------------------------------------
Proven by oracle: class decision table; every read, its order and size, the
version limits and their throw; discarded fields; append order and record
copies; 4FC340 seek targets after null results (incl. seek back) and no seek
after a non-null result; the replacement block, variant rule, warning
selection and order; 4F9C10 dispatch on the block per class; def vf+30 per
data kind; all 7,879 real records and their blocks/warnings.
Rust replays every fixture row; the placement path equals the byte path on
all 7,879 real records (ignored test, real maps).
Static only: the EH propagation of "Bad data version" to 5A2B90/5A312F
(FuncInfo try maps, throw type hierarchy TGC_LoadWarningException ->
LoadingException -> FileException); the position of the replacement loop
after PostLoad in 4FC600; the 4FC600 sections passing the vector.
Not ported here (other owners): 4F9C10 fresh creation and the class
constructors (restoration agent); the record loop itself; the map-level
failure handling of 5A312F; the native chunk-stream read-past-end error.
Not tested: STATIC_THINGS / USED_REQUIRED_OBJECTS conversions on real data
(no shipped map has STATIC_THINGS; USED_REQUIRED_OBJECTS is empty in all 27).

REMOVED / NOTED ENGINE LIMITS
- 4FC600 conversion vector (new[1], 4F9840/4F60C0 doubling) -> Vec, no cap.
- 4FC340 reads the section index into a fixed 0x40004-byte block (6F7810 at
  4FC35C): more than 65,536 index entries overflow it natively. Not in this
  module (the restoration loop/inventory already uses growable storage).

===========================================================================
ROUND 2 (2026-10-15): FRESH CREATION (4F9C10 classes 2/9, 4DCB30, 4FA030,
4FA150)
===========================================================================
The restoration agent had already ported 4F9C10's own control flow
(aa-actors world_creation.rs: instance-name check, class switch, the
"Actor/Group class requires a GACD" / "Kingdom class requires a GKCD"
refusals, InitCreate / InitExisting / PostCreate / registration order,
destroy on failure; and 4FBF40's member-spawn control flow). This round adds
the per-class work below it for GObject (class 2) and DynamicDoodad
(class 9), and the CreateGActor / CreateOrganization entry points.

Rust: crates/aa-mapworld/src/fresh_creation.rs (new)
  construct (4DCB30 / 4EA5E0 over 4EF2A0 = aa_actors::creation::construct),
  init_create (4DCBA0 / 4E9FC0 / 4EE460), init_existing (4DC730 / 4EBE50 /
  4EE700 / 4EFA40 / 4EF6A0 / 4EE4C0), post_create (4EBA40), register
  (4F9510 / 4F4C40), create_thing (= world_creation::create with this
  module's CreateWorld adapter), FreshReplacement (class_conversion
  ReplacementWorld on create_thing), create_gactor (4FA030),
  create_organization (4FA150). World subsystems are FreshWorld /
  EntryWorld methods named after the original functions.
Tests: crates/aa-mapworld/tests/fresh_creation.rs (4 tests).
Oracle: native_fresh_creation.py -> native-fresh-creation.json (55 creation
  cases at PC24/53/64, 17 entry cases).

NATIVE ORDER (class 9; class 2 is the same without components)
  4F9C10 -> 6F7810(0x68) -> 4DCB30:
    4EA5E0 -> 4EF2A0: +4 def, flags = 0x80000000 | def+1C & 0x03FFFF00 |
      def+1C & 0x20000000; position; name; facing: bit 17 (0,-1), bit 15
      direction table [B39AF8] (one draw), else the direction check that
      rewrites params +14/+18 to (0,-1) when |len-1| >= 1e-4; 4EE9F0
      normalization (bit 12: 3D); 4EE9F0 then 50C270 -> 43B7D0 scene
      transform (bit 26 index move is unreachable: never copied);
      +1C = GWorld+E0.
    4EA5E0 fields: +38..+5C zero, +40 = -1, +4C byte 0; def+108 ->
      +40 = params+20, -1 -> 49B900 weighted draw.
    vtable A28E90, +60/+64 = 0, 4DC920(0/1): def+174 Disturbable (0x1C,
      590160), def+178 Breakable (0x30, 58FE10: +18..+2C = 0,0,0,
      [B279C0],[B279C4] (zero), 1.0f).
  vf E4 = 4DCBA0: 4E9FC0 (4EE460: params+24 != 0 -> 4EFB70 + 4F0190, else
    4EFDB0 which logs "KKC_GTIDManager::Add() - no IDs left" and fails
    when 0x8000 IDs are live; then 49B440 scale draw -> +3C), 4DC920 x2,
    component vf+4 x2 (5904A0: one draw, delay = fraction x compdef+10,
    50ACC0 -> GWorld+1E8 vf C; Breakable 58F960 then 50B250 unlinks its
    events head-first while +18 is 0/1).
  vf E8 = 4DC730: 4EBE50 (+4C = 0; +38 = f32(+3C x def+24); quarter =
    ftol(tile size) x 0.25f; 4EE700 (flags & 0x20000400 and ftol(tile) <=
    shapeA radius x 4) and shapeA kind 2/3 and min(+10,+14) < quarter ->
    404EA0 "Static object %s is not thick enough to block tiles"; 4EFA40:
    bit 16 snap 46EE00, params +8..+18 <- position/facing, 4EF6A0 (4F4590
    reason code; nonzero -> 404F60 "creating thing %s failed (reason: %s)
    at %f,%f" and failure, except reasons 1/2 in map mode 2: map warning
    load_warning_thing_doesnt_fit|sff and success), 4EE4C0 (B0 true ->
    5536B0 insert, sets flag bit 26), height 4F0E50 (bit 21: 0x123, else
    bit 12 clear: 0x122) -> +28; 4EB6E0 (bit 10 blocking), 4EA150 tile
    footprint, +44/+48 = ftol(x/y) sar [GMap+4], 4F1F70 (bit 8 -> GWorld+8
    list head), vf 8C (+58 > 0, false when fresh), 4EBC70 visibility),
    then component vf+4 x2 again.
  vf EC = 4EBA40(params, 0): 4D1E10 RET 8; map mode 0 -> 4EB500 ground
    paint.
  4F9510: flag bit 31 cleared; empty name; vf BC/C0/AC/B4 false;
    4F4C40(obj,1): def+34 (vf 24) -> available-cities head insert at
    GWorld+260 (assert latch B3CC40); 50C220 -> 43D160 scene add (GWorld+D8
    != 6 and [GWorld+130]+4); vf B0 true -> 42A590(def+BC) -> 429080 FX.
  Failure (no ID, placement): 4F5490 destroy, null.
RNG (GWorld+D4, 85D840) order: facing, variant, scale, component draws
  (InitCreate), component draws (InitExisting). The four basic cases run at
  PC24, PC53 and PC64; the Rust replay uses each case's precision.
4FA030: 4F9C10; null -> null; latch B3CC43 clear and vf B4 false ->
  855D30("thing->IsGActor()", 0x85E) whose result sets the latch.
4FA150: ActorData (def vf 38) with +174 bit 7: layout = params+2C or
  def+2F0; [layout+8]+2C > 0: 4C2BC0, 578290 slot test (false -> null,
  nothing created), 4BE740 params position = (x, y, 0); 4F9C10; IsGActor
  assert (B3CC44, 0x880); Organization +7C: leader (layout +18) spawned
  first with flag 0 when its group is defined, then every other defined
  slot with params+51 (57AD30 = member spawn 4FBF40); 57A730 == 0 ->
  404F60 "org %d failed to create any elements - destroying" (0x8A0),
  4F5490, null.

ORACLE SEAMS (other owners' subsystems): placement test 4F4590, index
insert/remove 5536B0/553830 (with their flag bit 26 write), snap 46EE00,
height 4F0E50, blocking query 554350/555620, tile footprint 4EA150,
visibility 4EBC70, ground paint 4EB500, scene 43B7D0/43D160, FX 429080,
deferred queue add (GWorld+1E8 vf C) / unlink 4F1A70, destroy 4F5490,
logs, assertion dialog 855D30, and in the entry cases 4F9C10 itself,
4C2BC0, 578290, 57AD30, 57A730. Everything else in the listed chain runs
natively; any other function entry stops a case. Fixture objects record
every read offset (`reads`). Null guard installed: no case reports a page-0
access; every allocation goes through the 6F7810 seam.

RUST REPLAY: every case's seam call sequence (normalized), RNG state after,
object fields (+8..+5C), components, parameter write-back, world lists and
the GTID manager (cursor, highest, count) against aa-actors
IdentityRegistry; entry cases' call sequences, results and latches.
create_thing_class_dispatch pins the 4F9C10 class table for an A25B1C block.

NOT DONE (round 2)
- Class 3 (actor) fresh creation: constructor 4D6BB0 (partial:
  aa_actors::creation::construct_actor), InitCreate 4D6F10 (aa_actors
  init_create.rs orchestration), InitExisting 4D7230 (unported), PostCreate
  4CF300 (world_publication.rs), actor registration in 4F9510 (vf B4 ->
  4D62F0), the 31 component factories 4CF480. No CreateWorld composes them
  yet, so 4FA030 / 4FA150 / 4FBF40 run their own control flow but their
  4F9C10 call for an actor block is not satisfied by this module.
- Classes 4/5/6/7/8/10 constructors.
- Instance-name registration in 4F9510 (nonempty names: explicit error).
- Real-data creation of the 7,878 converted records: needs definition
  inputs not yet exposed (requests C5).
