Saved GActor::Init(NULL), 4D7230 — Development-09 component
=========================================================

Production entry: aa_actors::actor_init_null::initialize(world, actor, pc).
This is the saved/null coordinator, not fresh GACD initialization. Every
World operation is mandatory and uses the current real owner. Object/world
and component leaves remain explicit required services. There is no runtime
adapter or load-driver integration in this change, and it does not enable a
battle, finish restoration, or claim that saved actors initialize end to end.

Native sources
--------------
Both AA.exe original and the independently loaded v14 image execute 4D7230
with a real null incoming parameter. Full executable SHA256 pins are in the
fixture and findings.json. The trace includes actual native coordinator,
GTID generation lookup, definition virtual+24 (462E50), TrackInfo getters
45E700/45E710, floor96D5E0, ftol96CC4C, and local arithmetic. Object4EBE50,
required-object4D1FD0, component virtual methods, property and world leaves
are explicitly hooked, recorded, and subject to failure/mutation probes.
They are not claimed as implemented by this coordinator oracle.

The prior read-only plan and full disassembly are in:
../skirmish-20261016-saved-flows/lifecycle-proof/next-actor-init-null.json
../skirmish-20261016-saved-flows/lifecycle-proof/next-actor-init-null.asm.txt

Order and retained state
------------------------
4D7251 Object Init(NULL), then4D725C required-object preparation; either
false result stops. NULL jumps over4D7265..7281: actor+138 is never read or
written by this port. An earlier callback's change to it remains intact.
Current slots0..30 except Mind initialize in order, then current Mind last;
the separate current-slot vf+14 loop follows. No component list is cloned.
After that, reread +158 and validate its live GTID before virtual+128.
Call4D4CF0,4D1350,4D67F0. Current flag29 guards the global property read;
Properties presence and5035E0 guard5817A0. Then4D4130 and current present
Organization, Settlement, Performer, Denizen refreshes in that order.
Definition+24, current Object+50 liveness, create-zone success, radius and
current XY guard the zone2 move. Missing zone2 after successful creation
is an explicit null error, not success. Present SAI and current flag0 clear
guard7A0C20.

The track scan captures current ActorData once after SAI; later replacement
does not change the captured identity. Native repeated counts, repeated
track lookup, kind and length access order are preserved, including count
changes during traversal. Arithmetic happens for any initially positive
count, even if it finds no kind1 track. Divisor is not read until after
scale multiplication, binary64 store, floor and ftol. Allocation happens
only when kind1 was seen; actual allocation-null stores None into+144.
A constructor error keeps the previous+144. Final current state bit21 clear
permits42A590 using the current ActorData+E4 and arguments(actor,0).

Evidence and verification
-------------------------
972 rows:162 inputs x original/v14 x Single24/Double53/Extended64.
Rust replay checks full ordered callback trace, result/explicit error, all
remaining component slots, flags/state, required reference, current
definition, retained+138 and+144. Inputs include all31 individual slots,
all slots together, Mind-last and callback component mutations, low-byte
false values256, stale/missing required reference, property/zone/SAI/FX
gates, captured-versus-current definition mutations, dynamic counts,
allocator-null, negative/zero divisors and IDIV overflow, finite lengths, ignored NaN lengths,
and32 injected dependency failures. Mutation probes demonstrate coordinator
ordering; they do not assert that those are normal native gameplay effects.
An injected callback failure is a Rust service boundary: no native SEH
unwind or cleanup parity is claimed.

Commands (from rust-rebuild, PowerShell):
  $env:CARGO_TARGET_DIR='target/s-actor-init-null'
  cargo test -p aa-actors --test native_actor_init_null --release --offline --locked -j4
  $env:PYTHONPATH='E:\Games\Axis and Alies\reconstruction\tools\python-packages'
  python research/skirmish-20261016-actor-init-null/native_actor_init_null.py

Default oracle execution compares the existing fixture and never writes it.
Only explicit --write regenerates it. The initial948-row fixture was backed
up before adding three definition-mutation probes; that966-row fixture was
also backed up before adding the IDIV overflow probe. Each regeneration ran
both actual images. See native-run.log, native-compare.log, rust-test.log.

Limits and integration requirements
-----------------------------------
The private track numerator repeats the proved existing fresh-path numeric
sequence but splits before the divisor service to preserve lazy read/error
order. Finite product fitting the signed32 ftol result is the supported
numeric domain. Nonfinite scale/product and out-of-range conversion are
explicit arithmetic errors, not a claim of native indefinite-int behavior.
Division-by-zero is refused at native idiv; quotient overflow is explicit.
A separate9-case diagnostic test proves that unsupported NaN, infinity or
out-of-range ftol numerators stop before reading divisor and leave+138/+144
intact. This refusal test deliberately claims no native parity outside the
supported numeric domain.
No new fixed allocation budget or pool is introduced.

A runtime binder must borrow the actual restored actor/component state,
execute each reached world service and report unbound services explicitly.
It must not use fresh nonnull-GACD helpers, infer omitted source fields, clone
the actor for mutations, invent successful Object initialization, zero+138,
or mark the outer LoadThing lifecycle complete after this coordinator alone.
The caller still owns its bit7 Init gate, PostCreate, publication/registration
and failure destruction. No original game, release, fresh actor_creation.rs,
shared runtime driver, dependency or workspace manifest was changed here.
