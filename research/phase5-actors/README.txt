PHASE 5: BASE THING CREATION AND ACTOR DEFINITION PREREQUISITES
============================================================

Implemented boundary
--------------------
aa-actors::creation ports the complete successful finite numerical domain of
GThing constructor 4EF2A0 and the valid-record identity branch 4EE460. The latter
uses the previously verified GTID manager. This is a base Thing, not a fully
constructed GActor, deployed unit, initialized owner, or published world entity.

The explicit CreationRecord contains verified definition identity/flags, XYZ
position, XY facing, raw UTF-16 name and optional saved GTID. Those inputs must
come from native-equivalent record creation; renderer placement IDs and raw TGI
inspector strings are not accepted as substitutes. World+E0 supplies creation
time and world+D4 supplies the actual current LCG state. No seed is chosen.

Constructor flags are 80000000 | (definition+1C & 03FFFF00) |
(definition+1C & 20000000). Native+C/+10/+14 are zero initially. The native
CString assignment preserves length, embedded NUL and unpaired UTF-16 units;
Rust stores those units directly without copying native pointer/pool layout.

Facing selection preserves original precedence and effects:
- bit17 fixes XY to (0,-1), including when bit15 is also set, without RNG draw;
- otherwise bit15 draws once through the original LCG and uses modulo2048;
- otherwise abs(sqrt(x*x+y*y)-1) must be strictly below 0.0001, or the creation
  record itself is mutated to (0,-1) before copying to the new Thing.

GThing's 4EE9F0 post-selection normalization differs by bit12. With bit12 set,
4453A0 uses 3D length including zero Z and zeroes sufficiently tiny vectors.
Without it, normalization is skipped when abs(squared length-1)<=0.0001.
aa-numeric applies requested PC24/53/64 rounding at original operation boundaries
and stores f32 only where the original does. Direction nonfinite values are
explicitly unsupported when consumed; raw positions/time still copy bitwise.
Effects before a numeric error are not rolled back.

Direction startup
-----------------
Original startup9C8680 pushes0x800, loads ECX=B39A90, and calls408ED0. This
pins the2,048 count instead of assuming it from the fixture. 408ED0 ->408D50
produces all 2,048 XY direction entries, including its
original trig and normalization instructions. Each startup PC is executed and
stored separately in direction-tables.bin. PC53 and PC64 tables agree; PC24
differs in 433 vectors. DirectionTable::original requires startup precision
explicitly, because table creation and later actor operations need not use the
same FPU state. It is incorrect to substitute the separate 4,096-entry sin/cos
lookup tables used by the movement rotation helpers.

Identity initialization
-----------------------
4EE460 with a zero saved ID invokes normal registry Add. A nonzero saved ID
first seeds that slot's generation and then invokes saved-ID registration.
The returned registry Entry owns the ID; a second independent ID copy is not
kept in the Thing. is_actor is an explicit actual derived-class classification.
Invalid/occupied saved slots use registry safe errors instead of emulating native
assertion or partially poisoned table state. These are documented safe-domain
differences. Later initialization failure may occur after identity allocation.

Oracle / tests
--------------
original_creation.py runs untouched original x86 ctor, CString assignment,
normalization, startup table construction and identity initialization. String
pool/general allocation are declared leaves. The constructor's initial visual
notification is observed; visual link+C is initially null and spatial bit26
cannot arise from the copied flag mask, so unrelated spatial/visual lifecycle
paths are not entered. No original game process is modified or launched.

282 cases cover all three arithmetic modes, every facing branch, tolerance
neighborhoods, zero/nonunit/nearunit vectors, flag-copy boundaries, raw name
units, saved IDs and ordinary registry capacity. The table test compares all
6,144 vectors. A separate error test covers NaN rejection and axis precedence.
Nine total crate tests also preserve all Phase4 registry/component cases.

definition_prerequisites.py records a separate evidence inventory. It runs 441
actual native speed/health/defense getter calls and diagnostic binary32 stores.
It also pins original/v14 constructor and loader bytes plus mask instructions.
The mask inventory is static disassembly, not full loader execution or a Rust
implementation. Do not count those getter cases as implemented Rust behavior.

Definition prerequisites still missing
-------------------------------------
Actor definition constructor 4624B0 starts definition+174 at zero. Loader
464BD0 sets component mask bits from resolved child nodes and their values,
including later clearing predicates. Mask derivation therefore needs the actual
merged/inherited typed node tree and the full loader order, not a substring
search or a hand-written table of component names. Representative evidence:
  4662B8: query optional child "BodyComponent" through5B4410; if present set bit0;
  4662EB: typed float lookup "defense" through5B5130 ->definition+288;
  466321: typed float lookup "health" through5B5130 ->definition+284;
  465945..465973: Mover child branch sets bit3 and reads definition+228 speed;
  46507B: detection bit22 can be cleared after its child values were read;
  the later loader can also clear weapon bit5 after its earlier construction.
definition-mask-instructions.txt records every +174 access and nearby original
instructions. Whole functions are bound to both original and accepted v14 bytes.

Definition getters are not merely field reads when modifier context is nonnull:
  45D6F0: null ->+228; otherwise5014C0 multiplies base by modifier+38;
  45D730: null ->+284; otherwise5014F0 multiplies base by modifier+50;
  45D760: null ->+288; otherwise501B10 adds modifier+10, stores binary32, floors
  against min(base, **([B39AB0]+1A4)), and clamps that result to at least zero.
Component helper529C30 reads component+4 actor pointer, then actor+4 definition.
Body561960 and Mover576920 initially request the null-context getters. Runtime
recalculation with context requires the real modifier object, never guessed1/0.
The original returned x87 value may retain precision until its caller stores it;
the evidence's final diagnostic f32 store is stated explicitly.

Remaining full factory chain
----------------------------
GWorld4F9C10 dispatches category, constructs actual derived objects (including
4D6BB0 for actors), performs E4/E8 init, EC post-create, publication4F9510 and
queue50C220, or failure destruction4F5490. Base render/spatial object4EA5E0,
ownership, nested actors, component constructors, initial state/abilities,
world grids and publication/destruction are not supplied by this module.
The current client diagnostic runs source-pinned fixtures only. It does not
turn preview map objects into live simulation actors or issue game orders.

Reproduce
---------
python research/phase5-actors/original_creation.py
python research/phase5-actors/definition_prerequisites.py
cargo test -p aa-actors
cargo clippy -p aa-actors --all-targets -- -D warnings

No native installation, profile, save, launcher or previous release is changed.
